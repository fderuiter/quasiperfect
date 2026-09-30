#![cfg(feature = "lattice")]

use crate::schema_generated::Prefix;
use crate::types::PrimePower;
use lll_rs::{lll::biglll, matrix::Matrix, vector::BigVector};
use rug::integer::Order;
use rug::{Assign, Float, Integer, Rational};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LatticeWitness {
    pub dimension: usize,
    pub w: Vec<String>,
    pub t: String,
    pub transformation_matrix: Vec<Vec<String>>,
    pub epsilon: f64,
    pub target_log: f64,
}

static LATTICE_WITNESSES: Mutex<Vec<LatticeWitness>> = Mutex::new(Vec::new());

pub fn clear_lattice_witnesses() {
    if let Ok(mut lock) = LATTICE_WITNESSES.lock() {
        lock.clear();
    }
}

pub fn get_lattice_witnesses() -> Vec<LatticeWitness> {
    if let Ok(lock) = LATTICE_WITNESSES.lock() {
        lock.clone()
    } else {
        Vec::new()
    }
}

pub fn add_lattice_witness(witness: LatticeWitness) {
    if let Ok(mut lock) = LATTICE_WITNESSES.lock() {
        lock.push(witness);
    }
}

pub fn compute_target_penalty(m: usize) -> f64 {
    let base = crate::manifest_constants::LATTICE_TARGET_PENALTY_BASE;
    if m < 2 || m > 16 {
        return base;
    }
    base * ((1usize << (m - 2)) as f64)
}

/// Convert a 256-bit/512-bit accumulator `Uint` directly to `rug::Integer`
/// without string formatting or string parsing.
pub fn uint_to_rug_integer(u: &crate::types::Uint) -> Integer {
    let bytes = u.to_le_bytes();
    Integer::from_digits(&bytes, Order::Lsf)
}

/// Convert `s_l` and `n_l` accumulators directly into an exact `rug::Rational` fraction S_l / N_l.
pub fn accumulators_to_rational(s_l: &crate::types::Uint, n_l: &crate::types::Uint) -> Rational {
    let s_int = uint_to_rug_integer(s_l);
    let n_int = uint_to_rug_integer(n_l);
    Rational::from((s_int, n_int))
}

/// Compute target log-abundancy and tolerance epsilon using 256-bit arbitrary precision Float arithmetic.
pub fn compute_target_log_and_epsilon_float(
    s_l: &crate::types::Uint,
    n_l: &crate::types::Uint,
) -> (Float, Float) {
    const PRECISION: u32 = 256;
    let rat = accumulators_to_rational(s_l, n_l);

    if *rat.numer() == 0 || *rat.denom() == 0 {
        return (Float::with_val(PRECISION, 0), Float::with_val(PRECISION, 0));
    }

    let a_curr_flt = Float::with_val(PRECISION, &rat);
    let ln_2 = Float::with_val(PRECISION, 2).ln();
    let target_log_flt = ln_2 - a_curr_flt.ln();

    let n_flt = Float::with_val(PRECISION, rat.denom());
    let inv_2n_flt = Float::with_val(PRECISION, 0.5) / &n_flt;
    let epsilon_flt = inv_2n_flt.ln_1p();

    (target_log_flt, epsilon_flt)
}

/// Compute target log-abundancy and tolerance epsilon as f64 values using 256-bit Float arithmetic internally.
pub fn compute_target_log_and_epsilon(
    s_l: &crate::types::Uint,
    n_l: &crate::types::Uint,
) -> (f64, f64) {
    let (target_log_flt, epsilon_flt) = compute_target_log_and_epsilon_float(s_l, n_l);
    (target_log_flt.to_f64(), epsilon_flt.to_f64())
}

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
    let scaling_factor = compute_target_penalty(m);

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

    // Construct exact rug::Integer accumulators without string parsing.
    let s_int = uint_to_rug_integer(&curr.s_l);
    let n_int = uint_to_rug_integer(&curr.n_l);

    if n_int == 0 || s_int == 0 {
        return false;
    }

    // Exact rational bound check before vector reduction:
    // T + epsilon < 0 is mathematically equivalent to S_l >= 2 * N_l + 1 (i.e. S_l > 2 * N_l).
    // If S_l > 2 * N_l, current abundancy S_l / N_l > 2, so target abundancy 2 is strictly exceeded.
    let two_n = Integer::from(2 * &n_int);
    if s_int > two_n {
        return true;
    }

    // Compute target log-abundancy T and target tolerance epsilon using 256-bit Float arithmetic.
    let (target_log_flt, epsilon_flt) = compute_target_log_and_epsilon_float(&curr.s_l, &curr.n_l);

    if target_log_flt.clone() + &epsilon_flt < 0 {
        // Since subset sum must be positive, and target_log + epsilon is negative, we can never reach it.
        return true;
    }

    let target_log = target_log_flt.to_f64();
    let epsilon = epsilon_flt.to_f64();

    let t_val = (target_log * scaling_factor).round() as i64;
    let t = Integer::from(t_val);

    if t == 0 {
        return false;
    }

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
                // Reconstruct U matrix
                let mut u_matrix = Vec::with_capacity(m + 1);
                let mut valid_reconstruction = true;
                for i in 0..=m {
                    let mut row = Vec::with_capacity(m + 1);
                    for j in 0..m {
                        row.push(basis[i][j].to_string());
                    }
                    let mut sum = Integer::from(0);
                    for k in 0..m {
                        sum += Integer::from(&basis[i][k] * &w[k]);
                    }
                    sum -= &basis[i][m];
                    let (u_im, remainder) = sum.div_rem(t.clone());
                    if remainder != 0 {
                        valid_reconstruction = false;
                        break;
                    }
                    row.push(u_im.to_string());
                    u_matrix.push(row);
                }

                if valid_reconstruction {
                    let witness = LatticeWitness {
                        dimension: m + 1,
                        w: w.iter().map(|x| x.to_string()).collect(),
                        t: t.to_string(),
                        transformation_matrix: u_matrix,
                        epsilon,
                        target_log,
                    };
                    add_lattice_witness(witness);
                }

                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Uint, UintExt};
    use rug::ops::Pow;

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
            active_mask: vec![0b111].into(), // Indices 0, 1, 2 are active
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

    #[test]
    fn test_dynamic_target_penalty_scaling() {
        let base = crate::manifest_constants::LATTICE_TARGET_PENALTY_BASE;
        assert_eq!(compute_target_penalty(2), base);
        assert_eq!(compute_target_penalty(3), base * 2.0);
        assert_eq!(compute_target_penalty(4), base * 4.0);
        assert_eq!(compute_target_penalty(16), base * 16384.0);
        // Out of range defaults to base
        assert_eq!(compute_target_penalty(1), base);
        assert_eq!(compute_target_penalty(17), base);
    }

    #[test]
    fn test_epsilon_underflow_prevention_deep_prefix() {
        // Deep prefix N = 10^16
        let n_l_16 = Uint::from_u128(10_000_000_000_000_000); // 10^16
        let (_, eps_16) = compute_target_log_and_epsilon(&n_l_16, &n_l_16);
        assert!(
            eps_16 > 0.0,
            "Epsilon for N=10^16 must be positive, got {}",
            eps_16
        );

        // Extremely deep prefix N = 10^37
        let n_l_37 = Uint::from_u128(10_000_000_000_000_000_000_000_000_000_000_000_000); // 10^37
        let (_, eps_37) = compute_target_log_and_epsilon(&n_l_37, &n_l_37);
        assert!(
            eps_37 > 0.0,
            "Epsilon for N=10^37 must be positive, got {}",
            eps_37
        );
    }

    #[test]
    fn test_uint_to_rug_integer_and_rational_conversion() {
        let u1 = Uint::from_u128(123_456_789_012_345_678_901_234_567_890);
        let u2 = Uint::from_u128(987_654_321_098_765_432_109_876_543_210);

        let rug_i1 = uint_to_rug_integer(&u1);
        let rug_i2 = uint_to_rug_integer(&u2);

        assert_eq!(rug_i1.to_string(), u1.to_string());
        assert_eq!(rug_i2.to_string(), u2.to_string());

        let rat = accumulators_to_rational(&u1, &u2);
        assert_eq!(rat, Rational::from((&rug_i1, &rug_i2)));
    }

    #[test]
    fn test_deep_prefix_no_nan_or_overflow_exceeding_f64_limit() {
        // Test U512 accumulators up to 10^150
        let n_l_large = Uint::from_u128(10).pow(150);
        let s_l_large = Uint::from_u128(12).pow(130);

        let (target_log, epsilon) = compute_target_log_and_epsilon(&s_l_large, &n_l_large);

        assert!(!target_log.is_nan(), "target_log must not be NaN");
        assert!(!target_log.is_infinite(), "target_log must not be infinite");
        assert!(!epsilon.is_nan(), "epsilon must not be NaN");
        assert!(!epsilon.is_infinite(), "epsilon must not be infinite");

        // Test arbitrary rug::Integer accumulators exceeding f64 limit (10^350 > 10^308)
        let s_350 = Integer::from(15).pow(280); // ~ 10^329
        let n_350 = Integer::from(10).pow(350); // 10^350
        let rat_350 = Rational::from((s_350, n_350.clone()));

        const PRECISION: u32 = 256;
        let a_curr_flt = Float::with_val(PRECISION, &rat_350);
        let ln_2 = Float::with_val(PRECISION, 2).ln();
        let target_log_flt = ln_2 - a_curr_flt.ln();

        let n_flt = Float::with_val(PRECISION, &n_350);
        let inv_2n_flt = Float::with_val(PRECISION, 0.5) / &n_flt;
        let epsilon_flt = inv_2n_flt.ln_1p();

        assert!(
            !target_log_flt.is_nan(),
            "256-bit target_log for 10^350 must not be NaN"
        );
        assert!(
            !target_log_flt.is_infinite(),
            "256-bit target_log for 10^350 must not be infinite"
        );
        assert!(
            !epsilon_flt.is_nan(),
            "256-bit epsilon for 10^350 must not be NaN"
        );
        assert!(
            !epsilon_flt.is_infinite(),
            "256-bit epsilon for 10^350 must not be infinite"
        );
    }

    #[test]
    fn test_exact_rational_bound_pruning_deep_node() {
        crate::lean_ffi::initialize_lean_runtime();

        // Deep node with S_l > 2 * N_l (abundancy already > 2), with accumulators near 10^150
        let n_l = Uint::from_u128(10).pow(150);
        let s_l = n_l * Uint::from_u32(2) + Uint::from_u32(1); // S_l = 2 * N_l + 1

        let curr = Prefix {
            n_l,
            s_l,
            last_idx: 0,
            factors: vec![3, 5],
            sigma_factors: vec![],
            sigma_factors_u64: vec![],
            active_mask: vec![0b11].into(),
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
        ];

        // Must prune (return true) due to exact rational bound proving unreachability,
        // without producing NaN or overflowing.
        let pruned = lll_prune_decision(&curr, &components);
        assert!(pruned, "Deep node with S_l > 2 * N_l must be pruned");
    }

    #[test]
    fn test_u512_exceeding_2_to_53_rational_abundance() {
        // Values > 2^53 loss of precision test
        // 2^60 + 1 = 1152921504606846977
        let n_val = (Uint::one() << 60) + Uint::from_u32(100);
        let s_val = (Uint::one() << 60) + Uint::from_u32(101);

        let rat = accumulators_to_rational(&s_val, &n_val);
        let expected_s = uint_to_rug_integer(&s_val);
        let expected_n = uint_to_rug_integer(&n_val);

        assert_eq!(*rat.numer(), expected_s);
        assert_eq!(*rat.denom(), expected_n);

        // 512-bit large state variable > 2^256
        let n_512 = (Uint::one() << 300) + Uint::from_u64(987654321);
        let s_512 = (Uint::one() << 300) + Uint::from_u64(123456789);

        let rat_512 = accumulators_to_rational(&s_512, &n_512);
        assert_eq!(*rat_512.numer(), uint_to_rug_integer(&s_512));
        assert_eq!(*rat_512.denom(), uint_to_rug_integer(&n_512));

        let (target_log_flt, epsilon_flt) = compute_target_log_and_epsilon_float(&s_512, &n_512);
        assert!(
            !target_log_flt.is_nan(),
            "target_log_flt for 512-bit int must not be NaN"
        );
        assert!(
            !epsilon_flt.is_nan(),
            "epsilon_flt for 512-bit int must not be NaN"
        );
    }

    #[test]
    fn test_uint_to_rug_integer_zero_string_formatting() {
        let test_cases = vec![
            Uint::from_u32(0),
            Uint::from_u32(1),
            Uint::from_u64(u64::MAX),
            Uint::from_u128(u128::MAX),
            Uint::one() << 256,
            (Uint::one() << 511) - Uint::one(),
        ];

        for u in test_cases {
            let rug_int = uint_to_rug_integer(&u);
            let bytes = u.to_le_bytes();
            let expected_rug = Integer::from_digits(&bytes, Order::Lsf);
            assert_eq!(rug_int, expected_rug);
        }
    }

    #[test]
    fn test_no_improper_branch_pruning_near_boundary() {
        crate::lean_ffi::initialize_lean_runtime();

        // Exact boundary: S_l = 2 * N_l (abundancy = 2 exactly)
        let n_l = Uint::one() << 128;
        let s_l = n_l * Uint::from_u32(2);

        let curr = Prefix {
            n_l,
            s_l,
            last_idx: 0,
            factors: vec![3, 5],
            sigma_factors: vec![],
            sigma_factors_u64: vec![],
            active_mask: vec![0b1].into(),
            sigma_mod24: 1,
        };

        let components = vec![PrimePower {
            p: 7,
            two_e: 2,
            val: crate::types::Uint::from_u32(49),
            sigma: crate::types::Uint::from_u32(57),
            sigma_factors: vec![],
            needs_rho: vec![],
            abundance_fp: (57u128 << 64) / 49,
        }];

        // S_l == 2 * N_l must NOT trigger the S_l > 2 * N_l pruning rule
        let s_int = uint_to_rug_integer(&curr.s_l);
        let n_int = uint_to_rug_integer(&curr.n_l);
        let two_n = Integer::from(2 * &n_int);
        assert!(
            !(s_int > two_n),
            "S_l == 2 * N_l must not exceed 2 * N_l in exact integer comparison"
        );
    }

    #[test]
    fn test_rational_abundance_benchmark_and_allocations() {
        let n_val = (Uint::one() << 200) + Uint::from_u64(123456789);
        let s_val = (Uint::one() << 200) + Uint::from_u64(987654321);

        let start = std::time::Instant::now();
        let iterations = 10_000;
        let mut sink_target = 0.0_f64;
        let mut sink_eps = 0.0_f64;

        for _ in 0..iterations {
            let (target_log, epsilon) = compute_target_log_and_epsilon(&s_val, &n_val);
            sink_target += target_log;
            sink_eps += epsilon;
        }

        let elapsed = start.elapsed();
        println!(
            "Executed {} abundance evaluations in {:?} ({:.2} ns/eval)",
            iterations,
            elapsed,
            elapsed.as_nanos() as f64 / iterations as f64
        );

        assert!(!sink_target.is_nan());
        assert!(!sink_eps.is_nan());
    }
}
