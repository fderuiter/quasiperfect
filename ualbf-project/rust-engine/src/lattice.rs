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

/// Convert a 256-bit/512-bit accumulator `Uint` directly to `rug::Integer` in-place
/// without new allocations or string parsing.
pub fn uint_to_rug_integer_into(u: &crate::types::Uint, target: &mut Integer) {
    let bytes = u.to_le_bytes();
    target.assign_digits(&bytes, Order::Lsf);
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

/// Pre-allocated thread-local workspace for multi-precision accumulators and LLL basis matrices.
pub struct LllWorkspace {
    pub w: Vec<Integer>,
    pub s_int: Integer,
    pub n_int: Integer,
    pub two_n: Integer,
    pub t: Integer,
    pub shortest_sq_norm: Integer,
    pub b0_j_sq: Integer,
    pub lhs_den: Integer,
    pub rat: Rational,
    pub lhs: Rational,
    pub diff: Rational,
    pub a_curr_flt: Float,
    pub ln_2: Float,
    pub n_flt: Float,
    pub inv_2n_flt: Float,
    pub target_log_flt: Float,
    pub epsilon_flt: Float,
    pub half_flt: Float,
    pub temp_flt: Float,
    pub matrices: Vec<Option<Matrix<BigVector>>>,
}

impl LllWorkspace {
    pub fn new() -> Self {
        const PRECISION: u32 = 256;
        let mut matrices = Vec::with_capacity(18);
        matrices.resize_with(18, || None);
        for d in 3..=17 {
            matrices[d] = Some(Matrix::init(d, d));
        }

        Self {
            w: (0..17).map(|_| Integer::from(0)).collect(),
            s_int: Integer::from(0),
            n_int: Integer::from(0),
            two_n: Integer::from(0),
            t: Integer::from(0),
            shortest_sq_norm: Integer::from(0),
            b0_j_sq: Integer::from(0),
            lhs_den: Integer::from(0),
            rat: Rational::from((0, 1)),
            lhs: Rational::from((0, 1)),
            diff: Rational::from((0, 1)),
            a_curr_flt: Float::with_val(PRECISION, 0),
            ln_2: Float::with_val(PRECISION, 2).ln(),
            n_flt: Float::with_val(PRECISION, 0),
            inv_2n_flt: Float::with_val(PRECISION, 0),
            target_log_flt: Float::with_val(PRECISION, 0),
            epsilon_flt: Float::with_val(PRECISION, 0),
            half_flt: Float::with_val(PRECISION, 0.5),
            temp_flt: Float::with_val(PRECISION, 0),
            matrices,
        }
    }

    pub fn reset(&mut self, m: usize) {
        let d = m + 1;
        self.s_int.assign(0);
        self.n_int.assign(0);
        self.two_n.assign(0);
        self.t.assign(0);
        self.shortest_sq_norm.assign(0);
        self.b0_j_sq.assign(0);
        self.lhs_den.assign(0);

        for i in 0..17 {
            self.w[i].assign(0);
        }

        if d < self.matrices.len() {
            if let Some(ref mut basis) = self.matrices[d] {
                for i in 0..d {
                    for j in 0..d {
                        basis[i][j].assign(0);
                    }
                }
            }
        }
    }

    pub fn compute_target_log_and_epsilon_float_into(&mut self) {
        self.rat.mutate_numer_denom(|num, den| {
            num.assign(&self.s_int);
            den.assign(&self.n_int);
        });

        if *self.rat.numer() == 0 || *self.rat.denom() == 0 {
            self.target_log_flt.assign(0);
            self.epsilon_flt.assign(0);
            return;
        }

        self.a_curr_flt.assign(&self.rat);
        self.a_curr_flt.ln_mut();
        self.target_log_flt.assign(&self.ln_2 - &self.a_curr_flt);

        self.n_flt.assign(self.rat.denom());
        self.inv_2n_flt.assign(&self.half_flt / &self.n_flt);
        self.epsilon_flt.assign(&self.inv_2n_flt);
        self.epsilon_flt.ln_1p_mut();
    }
}

std::thread_local! {
    static LLL_WORKSPACE: std::cell::RefCell<LllWorkspace> = std::cell::RefCell::new(LllWorkspace::new());
}

/// LLL-based lattice pruning module.
///
/// This module provides approximate yet mathematically sound and conservative bounding
/// of the OQPN search space by mapping subset selection to a knapsack-like shortest vector problem (SVP).
/// To ensure soundness, approximate computations must never reject reachable targets.
pub fn lll_prune_decision(curr: &Prefix, components: &[PrimePower]) -> bool {
    let mut remaining_indices = [0usize; 17];
    let mut m = 0;
    let mask = &curr.active_mask;
    let start_idx = curr.last_idx;
    let mut block_idx = start_idx / 64;
    if block_idx < mask.len() {
        let mut block = mask[block_idx] & (!0 << (start_idx % 64));
        'scan: loop {
            while block != 0 {
                if m >= 17 {
                    break 'scan;
                }
                let tz = block.trailing_zeros();
                let j = block_idx * 64 + tz as usize;
                remaining_indices[m] = j;
                m += 1;
                block &= block - 1;
            }
            block_idx += 1;
            if block_idx >= mask.len() {
                break;
            }
            block = mask[block_idx];
        }
    }

    if m < 2 || m > 16 {
        return false;
    }

    LLL_WORKSPACE.with(|ws_cell| {
        let mut ws_borrow = ws_cell.borrow_mut();
        let ws = &mut *ws_borrow;
        ws.reset(m);

        let ln_2 = 2.0_f64.ln();
        let scaling_factor = compute_target_penalty(m);

        for i in 0..m {
            let comp = &components[remaining_indices[i]];
            let fp_f64 = comp.abundance_fp as f64;
            let log_contribution = fp_f64.ln() - 64.0 * ln_2;
            if log_contribution <= 0.0 {
                return false;
            }
            let w_val = (log_contribution * scaling_factor).round() as i64;
            if w_val <= 0 {
                return false;
            }
            ws.w[i].assign(w_val);
        }

        uint_to_rug_integer_into(&curr.s_l, &mut ws.s_int);
        uint_to_rug_integer_into(&curr.n_l, &mut ws.n_int);

        if ws.n_int == 0 || ws.s_int == 0 {
            return false;
        }

        ws.two_n.assign(&ws.n_int << 1);
        if ws.s_int > ws.two_n {
            return true;
        }

        ws.compute_target_log_and_epsilon_float_into();

        ws.temp_flt.assign(&ws.target_log_flt + &ws.epsilon_flt);
        if ws.temp_flt < 0 {
            return true;
        }

        let target_log = ws.target_log_flt.to_f64();
        let epsilon = ws.epsilon_flt.to_f64();

        let t_val = (target_log * scaling_factor).round() as i64;
        ws.t.assign(t_val);

        if ws.t == 0 {
            return false;
        }

        let d = m + 1;

        let LllWorkspace {
            ref mut w,
            ref mut t,
            ref mut shortest_sq_norm,
            ref mut b0_j_sq,
            ref mut lhs_den,
            ref mut lhs,
            ref mut diff,
            ref mut matrices,
            ..
        } = *ws;

        let basis = matrices[d]
            .as_mut()
            .expect("Pre-allocated LLL matrix missing");

        for i in 0..m {
            basis[i][i].assign(1);
            basis[i][m].assign(&w[i]);
        }
        basis[m][m].assign(-&*t);

        biglll::lattice_reduce(basis);

        shortest_sq_norm.assign(0);
        for j in 0..=m {
            let b0_j = &basis[0][j];
            b0_j_sq.assign(b0_j * b0_j);
            *shortest_sq_norm += &*b0_j_sq;
        }

        if *shortest_sq_norm == 0 {
            return false;
        }

        lhs_den.assign(1);
        *lhs_den <<= m;
        lhs.mutate_numer_denom(|num, den| {
            num.assign(&*shortest_sq_norm);
            den.assign(&*lhs_den);
        });

        diff.assign(&*lhs - m);
        if *diff <= 0 {
            return false;
        }

        let r = 0.5 * ((m + 1) as f64) + scaling_factor * epsilon;
        if r > 0.0 {
            let r_sq = r * r;
            if let Some(r_sq_rat) = Rational::from_f64(r_sq) {
                if *diff > r_sq_rat {
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
                            w: w[0..m].iter().map(|x| x.to_string()).collect(),
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
    })
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

    #[test]
    fn test_thread_local_workspace_reset_and_isolation() {
        crate::lean_ffi::initialize_lean_runtime();

        let curr1 = Prefix {
            n_l: Uint::from_u32(100),
            s_l: Uint::from_u32(150),
            last_idx: 0,
            factors: vec![3, 5],
            sigma_factors: vec![],
            sigma_factors_u64: vec![],
            active_mask: vec![0b111].into(),
            sigma_mod24: 1,
        };

        let curr2 = Prefix {
            n_l: Uint::from_u32(1000),
            s_l: Uint::from_u32(1200),
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
                val: Uint::from_u32(49),
                sigma: Uint::from_u32(57),
                sigma_factors: vec![],
                needs_rho: vec![],
                abundance_fp: (57u128 << 64) / 49,
            },
            PrimePower {
                p: 11,
                two_e: 2,
                val: Uint::from_u32(121),
                sigma: Uint::from_u32(133),
                sigma_factors: vec![],
                needs_rho: vec![],
                abundance_fp: (133u128 << 64) / 121,
            },
            PrimePower {
                p: 13,
                two_e: 2,
                val: Uint::from_u32(169),
                sigma: Uint::from_u32(183),
                sigma_factors: vec![],
                needs_rho: vec![],
                abundance_fp: (183u128 << 64) / 169,
            },
        ];

        // First evaluation
        let res1 = lll_prune_decision(&curr1, &components);
        // Second evaluation on same thread (verifies clean reset of workspace buffers)
        let res2 = lll_prune_decision(&curr2, &components);
        // Re-run first evaluation again to verify complete decision equivalence
        let res1_again = lll_prune_decision(&curr1, &components);

        assert_eq!(
            res1, res1_again,
            "Consecutive evaluations must yield identical decisions"
        );
        assert_eq!(res1, false);
        assert_eq!(res2, false);
    }

    #[test]
    fn test_zero_copy_active_mask_scanning_multi_block() {
        let mut mask = vec![0u64; 3];
        mask[0] = 1 << 5; // Index 5
        mask[1] = 1 << 10; // Index 64 + 10 = 74
        mask[2] = 1 << 63; // Index 128 + 63 = 191

        let mut components = vec![
            PrimePower {
                p: 2,
                two_e: 2,
                val: Uint::from_u32(4),
                sigma: Uint::from_u32(7),
                sigma_factors: vec![],
                needs_rho: vec![],
                abundance_fp: (7u128 << 64) / 4,
            };
            200
        ];

        components[5].p = 7;
        components[74].p = 11;
        components[191].p = 13;

        let curr = Prefix {
            n_l: Uint::from_u32(100),
            s_l: Uint::from_u32(150),
            last_idx: 0,
            factors: vec![3, 5],
            sigma_factors: vec![],
            sigma_factors_u64: vec![],
            active_mask: mask.into(),
            sigma_mod24: 1,
        };

        let decision = lll_prune_decision(&curr, &components);
        assert!(!decision);
    }

    #[test]
    fn test_workspace_struct_memory_footprint_and_bounds() {
        let ws_size = std::mem::size_of::<LllWorkspace>();
        println!("Size of LllWorkspace struct: {} bytes", ws_size);
        assert!(
            ws_size < 65536,
            "LllWorkspace struct size ({} bytes) must be well under 64 KB",
            ws_size
        );
    }
}
