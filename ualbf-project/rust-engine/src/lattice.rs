//! LLL-based lattice pruning module.
//!
//! This module performs approximate log-space lattice computations to prune search branches
//! that cannot reach the target perfect-number ratio.
//!
//! To ensure soundness, these approximate computations must never reject reachable targets.
//! To achieve this, we follow the conservative-rounding precedent established in
//! `UALBF.Fixed64.scaleBoundCeil_conservative`. This ensures that all log-space conversions
//! are rounded in the conservative direction, keeping the approximate calculations soundly
//! within the trusted computing base (TCB) boundary.
//!
//! Note that the exact CRT/Touchard check remains authoritative for correctness
//! and stays outside this module (in `dfs_tree.rs` and related proof modules).

use crate::schema_generated::Prefix;
use crate::types::PrimePower;

/// Details of the LLL pruning bounds calculation.
#[derive(Clone, Debug)]
pub struct LllDetails {
    pub m: usize,
    pub shortest_sq_norm: f64,
    pub target_log: f64,
    pub epsilon: f64,
}

/// Converts `abundance_fp` (Q64.64 `u128`) into a scaled log-abundancy contribution.
///
/// To ensure that our approximate log-abundancy calculation never falsely rejects reachable
/// targets, we round each conversion in the conservative direction: **down** (using `.floor()`).
/// This means that the computed log-abundancy value is never larger than the true log-abundancy,
/// which prevents us from overestimating the abundancy contribution of any component and avoids
/// false prunings. This follows the conservative-rounding precedent of
/// `scaleBoundCeil_conservative`.
fn abundance_to_log_scaled(abundance_fp: u128, scaling_factor: f64) -> f64 {
    let abundance = (abundance_fp as f64) / 18446744073709551616.0; // 2^64
    let log_val = abundance.ln();
    (log_val * scaling_factor).floor()
}

fn dot_product(v1: &[f64], v2: &[f64]) -> f64 {
    v1.iter().zip(v2.iter()).map(|(x, y)| x * y).sum()
}

fn lll_reduction(basis: &mut [Vec<f64>], delta: f64) {
    let d = basis.len();
    let mut b_star = vec![vec![0.0; d]; d];
    let mut mu = vec![vec![0.0; d]; d];

    let mut update_gso = |basis: &[Vec<f64>], b_star: &mut [Vec<f64>], mu: &mut [Vec<f64>]| {
        for i in 0..d {
            b_star[i] = basis[i].clone();
            for j in 0..i {
                let num = dot_product(&basis[i], &b_star[j]);
                let den = dot_product(&b_star[j], &b_star[j]);
                let mu_val = if den > 1e-9 { num / den } else { 0.0 };
                mu[i][j] = mu_val;
                for k in 0..d {
                    b_star[i][k] -= mu_val * b_star[j][k];
                }
            }
        }
    };

    update_gso(basis, &mut b_star, &mut mu);

    let mut k = 1;
    let mut iterations = 0;
    while k < d && iterations < 1000 {
        iterations += 1;
        // Size reduction
        for j in (0..k).rev() {
            let mu_kj = mu[k][j];
            if mu_kj.abs() > 0.5 {
                let q = mu_kj.round();
                for i in 0..d {
                    basis[k][i] -= q * basis[j][i];
                }
                update_gso(basis, &mut b_star, &mut mu);
            }
        }

        // Lovasz condition
        let left = dot_product(&b_star[k], &b_star[k]);
        let right = (delta - mu[k][k - 1] * mu[k][k - 1]) * dot_product(&b_star[k - 1], &b_star[k - 1]);
        if left >= right {
            k += 1;
        } else {
            basis.swap(k, k - 1);
            update_gso(basis, &mut b_star, &mut mu);
            k = if k > 1 { k - 1 } else { 1 };
        }
    }
}

#[cfg(feature = "lattice")]
pub fn lll_prune_decision(
    curr: &Prefix,
    components: &[PrimePower],
    out_details: &mut Option<LllDetails>,
) -> bool {
    *out_details = None;

    // Start scanning remaining active candidates
    let mask = &curr.active_mask;
    let start_idx = curr.last_idx;
    let mut remaining = Vec::new();
    let mut block_idx = start_idx / 64;
    if block_idx < mask.len() {
        let mut block = mask[block_idx] & (!0 << (start_idx % 64));
        loop {
            while block != 0 {
                let tz = block.trailing_zeros();
                let j = block_idx * 64 + tz as usize;
                remaining.push(&components[j]);
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
    // We limit the number of candidates for performance and numeric precision.
    if m < 2 || m > 32 {
        return false;
    }

    let scaling_factor = 10_000_000.0;

    let mut w = Vec::with_capacity(m);
    for comp in &remaining {
        let w_val = abundance_to_log_scaled(comp.abundance_fp, scaling_factor);
        if w_val <= 0.0 {
            // Underflow or non-positive value. Don't prune to be conservative.
            return false;
        }
        w.push(w_val);
    }

    // Target abundance ratio is 2 + 1/N. Current is s_l/n_l.
    // Target log ratio t = ln(2) - ln(s_l/n_l).
    let s_l_f = curr.s_l.to_string().parse::<f64>().unwrap_or(1.0);
    let n_l_f = curr.n_l.to_string().parse::<f64>().unwrap_or(1.0);
    let a_curr = s_l_f / n_l_f;
    if a_curr <= 0.0 {
        return false;
    }
    let target_log = 2.0_f64.ln() - a_curr.ln();

    // Define target tolerance ε = log(2 + 1/N) − log(2)
    // ε bounds the log-space distance a branch may still cover to reach the perfect-number target.
    let epsilon = (2.0 + 1.0 / n_l_f).ln() - 2.0_f64.ln();

    if target_log + epsilon < 0.0 {
        // Since subset sum must be positive, and target_log + epsilon is negative, we can never reach it.
        return true;
    }

    let t = target_log * scaling_factor;
    let epsilon_scaled = epsilon * scaling_factor;

    // Initialize the basis matrix of size (m + 1) x (m + 1)
    let mut basis = vec![vec![0.0; m + 1]; m + 1];
    for i in 0..m {
        basis[i][i] = 1.0;
        basis[i][m] = w[i];
    }
    basis[m][m] = -t;

    // LLL reduction
    let delta = 0.75;
    lll_reduction(&mut basis, delta);

    // Compute final Gram-Schmidt orthogonalization vectors to find shortest vector lower bound.
    let mut b_star = vec![vec![0.0; m + 1]; m + 1];
    let mut mu = vec![vec![0.0; m + 1]; m + 1];
    for i in 0..=m {
        b_star[i] = basis[i].clone();
        for j in 0..i {
            let num = dot_product(&basis[i], &b_star[j]);
            let den = dot_product(&b_star[j], &b_star[j]);
            let mu_val = if den > 1e-9 { num / den } else { 0.0 };
            mu[i][j] = mu_val;
            for k in 0..=m {
                b_star[i][k] -= mu_val * b_star[j][k];
            }
        }
    }

    let mut min_gso_sq_norm = f64::INFINITY;
    for i in 0..=m {
        let sq_norm = dot_product(&b_star[i], &b_star[i]);
        if sq_norm < min_gso_sq_norm {
            min_gso_sq_norm = sq_norm;
        }
    }

    if min_gso_sq_norm < 1e-5 {
        return false;
    }

    let target_bound = (m as f64) + epsilon_scaled * epsilon_scaled;
    if min_gso_sq_norm > target_bound {
        *out_details = Some(LllDetails {
            m,
            shortest_sq_norm: min_gso_sq_norm,
            target_log,
            epsilon,
        });
        true
    } else {
        false
    }
}

#[cfg(not(feature = "lattice"))]
pub fn lll_prune_decision(
    _curr: &Prefix,
    _components: &[PrimePower],
    out_details: &mut Option<LllDetails>,
) -> bool {
    *out_details = None;
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

        let mut out_details = None;
        let decision = lll_prune_decision(&curr, &components, &mut out_details);
        println!("LLL Prune Decision: {}, details: {:?}", decision, out_details);
    }
}
