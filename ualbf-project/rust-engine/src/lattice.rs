#![cfg(feature = "lattice")]

use crate::schema_generated::Prefix;
use crate::types::PrimePower;
use lll_rs::{matrix::Matrix, vector::BigVector, lll::biglll};
use rug::{Integer, Assign, Rational};

/// LLL-based lattice pruning module.
///
/// This module provides approximate yet mathematically sound and conservative bounding
/// of the OQPN search space by mapping subset selection to a knapsack-like shortest vector problem (SVP).
/// To ensure soundness, approximate computations must never reject reachable targets.
pub fn lll_prune_decision(curr: &Prefix, components: &[PrimePower]) -> bool {
    // Collect remaining compatible candidates using the same active_mask and last_idx logic.
    let mut remaining = Vec::new();
    let mask = &curr.active_mask;
    let start_idx = curr.last_idx;
    let mut block_idx = start_idx / 64;
    if block_idx < mask.len() {
        let mut block = mask[block_idx] & (!0 << (start_idx % 64));
        loop {
            while block != 0 {
                let tz = block.trailing_zeros();
                let j = block_idx * 64 + tz as usize;
                remaining.push(components[j].clone());
                block &= block - 1;
            }
            block_idx += 1;
            if block_idx >= mask.len() {
                break;
            }
            block = mask[block_idx];
        }
    }

    let m = remaining.len();
    // Optimization: run LLL only for a reasonable number of candidates to balance performance.
    if m < 2 || m > 16 {
        return false;
    }

    // Convert abundance_fp values from Q64.64 into scaled log-abundancy lattice contributions.
    let ln_2 = 2.0_f64.ln();
    let scaling_factor = 1_000_000_000.0; // M = 10^9

    let mut w = Vec::with_capacity(m);
    for comp in &remaining {
        let fp_f64 = comp.abundance_fp as f64;
        let log_contribution = fp_f64.ln() - 64.0 * ln_2;
        if log_contribution <= 0.0 {
            // Log-abundancy of prime powers must be positive. If not, don't prune to be conservative.
            return false;
        }
        let w_val = (log_contribution * scaling_factor).round() as i64;
        if w_val <= 0 {
            return false;
        }
        w.push(Integer::from(w_val));
    }

    // Define target log-abundancy T = ln(2) - ln(A_curr).
    let s_l_f64 = curr.s_l.to_string().parse::<f64>().unwrap_or(1.0);
    let n_l_f64 = curr.n_l.to_string().parse::<f64>().unwrap_or(1.0);
    let a_curr = s_l_f64 / n_l_f64;
    if a_curr <= 0.0 {
        return false;
    }
    let target_log = ln_2 - a_curr.ln();

    // Define target tolerance epsilon = ln(2 + 1/N) - ln(2).
    let n_f64 = curr.n_l.to_string().parse::<f64>().unwrap_or(1.0);
    let epsilon = (2.0 + 1.0 / n_f64).ln() - ln_2;

    if target_log + epsilon < 0.0 {
        // Since subset sum must be positive, and target_log + epsilon is negative, we can never reach it.
        return true;
    }

    let t_val = (target_log * scaling_factor).round() as i64;
    let t = Integer::from(t_val);

    // Formulate the lattice basis.
    // Matrix of size (m + 1) columns x (m + 1) rows.
    let mut basis: Matrix<BigVector> = Matrix::init(m + 1, m + 1);
    for i in 0..m {
        basis[i][i].assign(1);
        basis[i][m].assign(&w[i]);
    }
    basis[m][m].assign(-&t);

    // Run LLL reduction in-place.
    biglll::lattice_reduce(&mut basis);

    // Compute the squared norm of the shortest vector b'_0.
    let b0 = &basis[0];
    let mut shortest_sq_norm = Integer::from(0);
    for j in 0..=m {
        let b0_j = &b0[j];
        shortest_sq_norm += Integer::from(b0_j * b0_j);
    }

    if shortest_sq_norm == 0 {
        return false;
    }

    // babai / LLL lower bound: any non-zero lattice vector y satisfies:
    // ||y||^2 >= 2^-m * shortest_sq_norm.
    // Since ||y||^2 = sum(x_j^2) + (sum(x_j * w_j) - t)^2 <= m + (sum(x_j * w_j) - t)^2,
    // we have (sum(x_j * w_j) - t)^2 >= 2^-m * shortest_sq_norm - m.
    let lhs_num = shortest_sq_norm;
    let lhs_den = Integer::from(1) << m;
    let lhs = Rational::from((lhs_num, lhs_den));

    let diff = lhs - Rational::from(m);
    if diff <= 0 {
        return false;
    }

    // Check if diff > (0.5 * (m + 1) + M * epsilon)^2
    let r = 0.5 * ((m + 1) as f64) + scaling_factor * epsilon;
    if r > 0.0 {
        let r_sq = r * r;
        if let Some(r_sq_rat) = Rational::from_f64(r_sq) {
            if diff > r_sq_rat {
                // Computed bound rigorously proves the branch cannot reach the target.
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UintExt;

    #[test]
    fn test_lll_prune_decision_basic() {
        crate::lean_ffi::initialize_lean_runtime();

        let mut curr = Prefix {
            n_l: crate::types::Uint::from_u32(100),
            s_l: crate::types::Uint::from_u32(150),
            last_idx: 0,
            factors: vec![3, 5],
            sigma_factors: vec![],
            sigma_factors_u64: vec![],
            active_mask: vec![0b111], // Indices 0, 1, 2 are active
            sigma_mod24: 1,
        };

        let components = vec![
            PrimePower {
                p: 7,
                two_e: 2,
                val: crate::types::Uint::from_u32(49),
                sigma: crate::types::Uint::from_u32(57),
                sigma_factors: vec![],
                needs_rho: vec![],
                abundance_fp: (57u128 << 64) / 49,
            },
            PrimePower {
                p: 11,
                two_e: 2,
                val: crate::types::Uint::from_u32(121),
                sigma: crate::types::Uint::from_u32(133),
                sigma_factors: vec![],
                needs_rho: vec![],
                abundance_fp: (133u128 << 64) / 121,
            },
            PrimePower {
                p: 13,
                two_e: 2,
                val: crate::types::Uint::from_u32(169),
                sigma: crate::types::Uint::from_u32(183),
                sigma_factors: vec![],
                needs_rho: vec![],
                abundance_fp: (183u128 << 64) / 169,
            },
        ];

        let decision = lll_prune_decision(&curr, &components);
        println!("LLL Prune Decision: {}", decision);
    }
}
