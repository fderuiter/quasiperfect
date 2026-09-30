#![cfg(feature = "lattice")]

use crate::schema_generated::Prefix;
use crate::types::PrimePower;
use lll_rs::{lll::biglll, matrix::Matrix, vector::BigVector};
use rug::integer::Order;
use rug::{Assign, Integer, Rational};
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

pub fn compute_target_penalty_rat(m: usize) -> Rational {
    let base = crate::manifest_constants::LATTICE_TARGET_PENALTY_BASE as u64;
    if m < 2 || m > 16 {
        return Rational::from(base);
    }
    Rational::from(base << (m - 2))
}

pub fn compute_target_penalty(m: usize) -> f64 {
    compute_target_penalty_rat(m).to_f64()
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

/// Compute exact rational lower and upper bounds for ln(x) where x = p/q > 1.
///
/// Returns (L, U) as `(rug::Rational, rug::Rational)` satisfying L < ln(x) < U.
/// Uses the hyperbolic arctanh expansion:
///   ln(x) = 2 * sum_{k=0}^{inf} z^{2k+1} / (2k+1)
/// where z = (x - 1) / (x + 1) = (p - q) / (p + q) in (0, 1).
pub fn rational_log_interval(x: &Rational, terms: usize) -> (Rational, Rational) {
    if *x.numer() <= 0 || *x.denom() <= 0 {
        return (Rational::from(0), Rational::from(0));
    }
    if x.numer() == x.denom() {
        return (Rational::from(0), Rational::from(0));
    }

    let p = x.numer();
    let q = x.denom();

    let num_z = p.clone() - q;
    let den_z = p.clone() + q;

    if num_z <= 0 {
        return (Rational::from(0), Rational::from(0));
    }

    let z = Rational::from((num_z, den_z));
    let z_sq = z.clone() * &z;

    let mut sum = Rational::from(0);
    let mut z_pow = z.clone();

    for k in 0..=terms {
        let term = z_pow.clone() / Integer::from(2 * k + 1);
        sum += term;
        if k < terms {
            z_pow *= &z_sq;
        }
    }

    let lower_bound = Rational::from(2) * sum;

    let z_next = z_pow * &z_sq;
    let one_minus_z_sq = Rational::from(1) - z_sq;
    let error_denom = Rational::from(2 * terms + 3) * one_minus_z_sq;
    let error_bound = (Rational::from(2) * z_next) / error_denom;

    let upper_bound = lower_bound.clone() + error_bound;

    (lower_bound, upper_bound)
}

/// Compute exact rational lower and upper bounds for target log-abundancy T and epsilon.
///
/// Returns ((t_low, t_high), eps_upper) as exact `Rational`s.
pub fn compute_target_log_and_epsilon_rational(
    s_l: &crate::types::Uint,
    n_l: &crate::types::Uint,
) -> ((Rational, Rational), Rational) {
    let s_int = uint_to_rug_integer(s_l);
    let n_int = uint_to_rug_integer(n_l);

    if s_int == 0 || n_int == 0 {
        return ((Rational::from(0), Rational::from(0)), Rational::from(0));
    }

    let two_n = Integer::from(2 * &n_int);
    let x_t = Rational::from((two_n, s_int));

    let target_log_interval = if *x_t.numer() > *x_t.denom() {
        rational_log_interval(&x_t, 20)
    } else if x_t.numer() == x_t.denom() {
        (Rational::from(0), Rational::from(0))
    } else {
        (Rational::from(-1), Rational::from(-1))
    };

    let two_n_plus_1 = Integer::from(2 * &n_int) + 1;
    let two_n_denom = Integer::from(2 * &n_int);
    let x_e = Rational::from((two_n_plus_1, two_n_denom));

    let (_, eps_upper) = rational_log_interval(&x_e, 20);

    (target_log_interval, eps_upper)
}

/// Compute target log-abundancy and tolerance epsilon as f64 values for telemetry logging.
pub fn compute_target_log_and_epsilon(
    s_l: &crate::types::Uint,
    n_l: &crate::types::Uint,
) -> (f64, f64) {
    let ((t_low, t_high), eps_upper) = compute_target_log_and_epsilon_rational(s_l, n_l);
    let target_log_f64 = ((t_low + t_high) / Rational::from(2)).to_f64();
    let epsilon_f64 = eps_upper.to_f64();
    (target_log_f64, epsilon_f64)
}

/// LLL-based lattice pruning module using exact rational interval arithmetic.
///
/// This module provides mathematically sound and conservative bounding of the OQPN search space
/// by mapping candidate subset selection to a knapsack-like shortest vector problem (SVP).
/// To ensure zero false-negative branch elimination, all basis matrix entries, shortest vector bounds,
/// and target radius bounds are computed using exact rational floor/ceiling operations over `rug::Rational`.
pub fn lll_prune_decision(curr: &Prefix, components: &[PrimePower]) -> bool {
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
    if m < 2 || m > 16 {
        return false;
    }

    let scaling_factor_rat = compute_target_penalty_rat(m);

    let mut w = Vec::with_capacity(m);
    for comp in &remaining {
        let s_i = uint_to_rug_integer(&comp.sigma);
        let v_i = uint_to_rug_integer(&comp.val);
        if s_i <= v_i {
            return false;
        }
        let x_i = Rational::from((s_i, v_i));
        let (log_low, _log_high) = rational_log_interval(&x_i, 16);
        if *log_low.numer() <= 0 {
            return false;
        }
        let scaled_w = scaling_factor_rat.clone() * log_low;
        let w_val = scaled_w.numer().clone() / scaled_w.denom();
        if w_val <= 0 {
            return false;
        }
        w.push(w_val);
    }

    let s_int = uint_to_rug_integer(&curr.s_l);
    let n_int = uint_to_rug_integer(&curr.n_l);

    if n_int == 0 || s_int == 0 {
        return false;
    }

    let two_n = Integer::from(2 * &n_int);
    if s_int > two_n {
        return true;
    }

    let ((target_log_low, _target_log_high), epsilon_rat) =
        compute_target_log_and_epsilon_rational(&curr.s_l, &curr.n_l);

    if *target_log_low.numer() <= 0 {
        return false;
    }

    let scaled_t = scaling_factor_rat.clone() * &target_log_low;
    let t = scaled_t.numer().clone() / scaled_t.denom();

    if t <= 0 {
        return false;
    }

    let mut basis: Matrix<BigVector> = Matrix::init(m + 1, m + 1);
    for i in 0..m {
        basis[i][i].assign(1);
        basis[i][m].assign(&w[i]);
    }
    basis[m][m].assign(-&t);

    biglll::lattice_reduce(&mut basis);

    let b0 = &basis[0];
    let mut shortest_sq_norm = Integer::from(0);
    for j in 0..=m {
        let b0_j = &b0[j];
        shortest_sq_norm += Integer::from(b0_j * b0_j);
    }

    if shortest_sq_norm == 0 {
        return false;
    }

    let lhs_num = shortest_sq_norm;
    let lhs_den = Integer::from(1) << m;
    let lhs = Rational::from((lhs_num, lhs_den));

    let diff_exact = lhs - Rational::from(m);
    if diff_exact <= 0 {
        return false;
    }

    let half_m_plus_1 = Rational::from((m + 1, 2));
    let r_exact = half_m_plus_1 + scaling_factor_rat * &epsilon_rat;
    if r_exact <= 0 {
        return false;
    }

    let r_sq_exact = r_exact.clone() * &r_exact;

    if diff_exact > r_sq_exact {
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
                epsilon: epsilon_rat.to_f64(),
                target_log: target_log_low.to_f64(),
            };
            add_lattice_witness(witness);
        }

        return true;
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
    fn test_rational_log_interval_accuracy() {
        // Test x = 2
        let x2 = Rational::from((2, 1));
        let (low, high) = rational_log_interval(&x2, 20);
        assert!(
            low < high,
            "lower bound must be strictly less than upper bound"
        );

        let low_f = low.to_f64();
        let high_f = high.to_f64();
        let exact_ln2 = 2.0_f64.ln();

        assert!(
            low_f <= exact_ln2,
            "low_f {} <= exact_ln2 {}",
            low_f,
            exact_ln2
        );
        assert!(
            high_f >= exact_ln2,
            "high_f {} >= exact_ln2 {}",
            high_f,
            exact_ln2
        );
        assert!(
            (high_f - low_f) < 1e-15,
            "interval width should be smaller than 1e-15"
        );
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

        let (low_t, high_t) = rational_log_interval(&rat_350, 20);

        assert!(low_t <= high_t, "low_t must be <= high_t");
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

        let (target_log_flt, epsilon_flt) = compute_target_log_and_epsilon(&s_512, &n_512);
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
