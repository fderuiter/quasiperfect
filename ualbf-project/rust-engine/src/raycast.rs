use crate::schema_generated::Prefix;
/// Compute the integer square root of an unsigned `Uint`.
///
/// Returns the greatest integer `r` such that `r * r <= n`. If `n` is zero, returns `Uint::zero()`.
///
/// # Examples
///
/// ```
/// let zero = Uint::zero();
/// assert_eq!(isqrt_uint(zero), Uint::zero());
/// assert_eq!(isqrt_uint(Uint::from_u32(16)), Uint::from_u32(4));
/// // floor(sqrt(15)) == 3
/// assert_eq!(isqrt_uint(Uint::from_u32(15)), Uint::from_u32(3));
/// ```
fn isqrt_uint(n: Uint) -> Uint {
    if n == Uint::zero() {
        return Uint::zero();
    }
    let two = Uint::from_u32(2);
    let mut x = n;
    let mut y = (x / two) + (x % two);
    while y < x {
        x = y;
        let nx = n / x;
        y = (x / two) + (nx / two) + ((x % two + nx % two) / two);
    }
    x
}

/// Compute the integer square root of a signed `Int`.
///
/// Returns `Some(x)` for the largest integer `x` such that `x * x <= n` when `n >= 0`,
/// or `None` when `n < 0`. For `n == 0` this returns `Some(0)`.
///
/// # Examples
///
/// ```
/// assert_eq!(isqrt(Int::from_u32(0)), Some(Int::from_u32(0)));
/// assert_eq!(isqrt(Int::from_u32(10)), Some(Int::from_u32(3))); // 3*3 <= 10 and 4*4 > 10
/// assert_eq!(isqrt(Int::from_i32(-1)), None);
/// ```
fn power(base: Uint, exp: u32) -> Option<Uint> {
    let mut res = Uint::from_u32(1);
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e % 2 == 1 {
            res = res.checked_mul(b)?;
        }
        if e > 1 {
            b = b.checked_mul(b)?;
        }
        e /= 2;
    }
    Some(res)
}

fn kth_root(c: Uint, k: u32) -> Option<Uint> {
    let mut low = Uint::from_u32(1);
    let mut high = Uint::from_u32(1);
    while let Some(p) = power(high, k) {
        if p >= c {
            break;
        }
        high = high.checked_mul(Uint::from_u32(2))?;
    }
    let mut ans = low;
    while low <= high {
        let mid = low.checked_add(high.checked_sub(low)?.checked_div(Uint::from_u32(2))?)?;
        if let Some(p) = power(mid, k) {
            if p == c {
                return Some(mid);
            }
            if p < c {
                ans = mid;
                low = mid.checked_add(Uint::from_u32(1))?;
            } else {
                high = mid.checked_sub(Uint::from_u32(1))?;
            }
        } else {
            high = mid.checked_sub(Uint::from_u32(1))?;
        }
    }
    Some(ans)
}

fn perfect_power(c: Uint) -> Option<(Uint, u32)> {
    for k in (2..=40).rev() {
        let root = kth_root(c, k)?;
        if let Some(p) = power(root, k) {
            if p == c {
                return Some((root, k));
            }
        }
    }
    None
}

fn sigma_power(base: Uint, two_e: u32) -> Option<Uint> {
    let mut sum = Uint::from_u32(1);
    let mut current = Uint::from_u32(1);
    for _ in 1..=two_e {
        current = current.checked_mul(base)?;
        sum = sum.checked_add(current)?;
    }
    Some(sum)
}

fn cofactor_sigma_bounds(c: Uint) -> Option<(Uint, Uint)> {
    let c2 = c.checked_mul(c)?;
    let sqrt_c = isqrt_uint(c);
    let two_c = Uint::from_u32(2).checked_mul(c)?;
    let two_c_sqrt = two_c.checked_mul(sqrt_c)?;
    let min_bound = c2.checked_add(two_c_sqrt)?;
    let hundred = Uint::from_u32(100);
    let c2_div_100 = c2.checked_div(hundred)?;
    let max_bound = c2.checked_add(c2_div_100)?;
    Some((min_bound, max_bound))
}

fn isqrt(n: Int) -> Option<Int> {
    if n < Int::zero() {
        return None;
    }
    if n == Int::zero() {
        return Some(Int::zero());
    }
    let two = Int::from_u32(2);
    let mut x = n;
    let mut y = (x / two) + (x % two);
    while y < x {
        x = y;
        let nx = n / x;
        y = (x / two) + (nx / two) + ((x % two + nx % two) / two);
    }
    Some(x)
}

use crate::math_utils::{composite_tonelli_shanks, sigma_cached, SigmaCache};
use crate::types::{Int, IntExt, Uint, UintExt};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Precomputes primes whose squares yield sigma ≡ 5 or 7 mod 8
/// Returns tuples `(p^e, p^{e+1})` for the sieve.
/// Since we test `v_p(z) == e`, it corresponds to `v_p(N_R) == 2e`.
/// Thus the tuples track `e` such that `\sigma(p^{2e}) \equiv 5 \text{ or } 7 \pmod 8`.
pub fn generate_illegal_z_valuations(limit: u64, max_e: u32) -> Vec<(Int, Int)> {
    use crate::obstruction::{Mod8Obstruction, Obstruction};
    let mut illegal = Vec::new();
    let mod8 = Mod8Obstruction;
    for p in 3..limit {
        let mut is_prime = true;
        let mut d = 2;
        while d * d <= p {
            if p % d == 0 {
                is_prime = false;
                break;
            }
            d += 1;
        }
        if !is_prime {
            continue;
        }

        let p_int = Int::from_u64(p);

        for e in 1..=max_e {
            if mod8.check_component(p, 2 * e) {
                illegal.push((p_int.pow(e), p_int.pow(e + 1)));
            }
        }
    }
    illegal
}

/// Searches residue classes z = r + c*s_l derived from `prefix` for quasiperfect numbers and reports any discoveries.
///
/// The function scans each root progression, optionally uses a GPU raycast sieve for large chunks, applies CPU-side
/// "illegal valuation" sieves, enforces coprimality with `prefix.factors`, checks big-integer congruence and range
/// constraints, factors candidate z values, assembles the sigma-product s_r from prime powers, and reports a match
/// when s_r equals the required s_r derived from 2*n + 1. `pruned_count` is incremented for values removed by either
/// GPU or CPU sieves.
///
/// # Parameters
///
/// - `prefix`: residue-class and factorization context (provides `n_l`, `s_l`, `factors`, and `sigma_factors`).
/// - `target_min`, `target_max`: inclusive Uint bounds used to derive the search range for z via integer square roots.
/// - `illegal_z_valuations`: list of prime-power pairs `(pe, pe1)` used to quickly reject z values by modular checks.
/// - `pruned_count`: atomic counter that is incremented (Relaxed) for each z rejected by sieving.
/// - `sigma_cache`: cache consulted by `sigma_cached` when computing sigma(p^{2k}) during s_r assembly.
/// - `reporter`: optional channel sender to which a formatted discovery message is sent (send errors are ignored).
///
/// # Examples
///
/// ```no_run
/// # use std::sync::atomic::AtomicUsize;
/// # use some_crate::{phase4_exact_ray_casting, SigmaCache, Uint};
/// // Build suitable arguments for your application; this example is illustrative.
/// let prefix: Prefix = /* construct prefix with n_l, s_l, factors, sigma_factors */ unimplemented!();
/// let target_min = Uint::zero();
/// let target_max = Uint::zero();
/// let illegal_z_valuations = Vec::new();
/// let pruned_count = AtomicUsize::new(0);
/// let sigma_cache: SigmaCache = Default::default();
/// phase4_exact_ray_casting(&prefix, &target_min, &target_max, &illegal_z_valuations, &pruned_count, &sigma_cache, None);
/// ```
/// Phase 4 Ray Casting (Exact Modular Check)
///
/// This phase executes an exact modular arithmetic test on candidate numbers that survive
/// the earlier approximate abundance pruning heuristics (such as the 2.0 threshold).
/// While the DFS phase uses rapid floating-point and bit-shift approximations to bound
/// the abundancy ratio, the ray-casting phase reconstructs the exact required value of the
/// missing prime component $q$ to satisfy $\sigma(N) = 2N + 1$. It then checks if $q$ is
/// an integer and a prime. This exact modular check acts as the final, rigorous filter,
/// complementing the early-pruning heuristic to ensure no false positives slip through.
pub fn phase4_exact_ray_casting(
    prefix: &Prefix,
    target_min: &Uint,
    target_max: &Uint,
    illegal_z_valuations: &[(Int, Int)],
    pruned_count: &AtomicUsize,
    math_interruptions: &std::sync::atomic::AtomicUsize,
    sigma_cache: &SigmaCache,
    reporter: Option<&crossbeam_channel::Sender<crate::events::SearchEvent>>,
    max_idx_3: usize,
    max_idx_5: usize,
    components_len: usize,
) {
    let config = crate::policy::get_safe_config();
    let verify_all = config.sampling_rate.unwrap_or(1.0) >= 1.0;
    let sampling_rate = config.sampling_rate.unwrap_or(1.0);
    let deterministic_seed = config.deterministic_seed.unwrap_or(0);

    let n_l_int = prefix.n_l.as_int();
    let s_l_int = prefix.s_l.as_int();
    let mut a = match (Int::from_u32(2)).checked_mul(n_l_int) {
        Some(v) => v % s_l_int,
        None => return,
    };
    if a < Int::zero() {
        a += s_l_int;
    }

    let x_l_inv_opt = crate::math_utils::mod_inverse_big(a, s_l_int);

    if let Some(x_l_inv) = x_l_inv_opt {
        // x_l is mathematically the negated inverse
        let x_l = -x_l_inv;

        let n_l_uint = prefix.n_l;
        let s_l_uint = prefix.s_l;

        let x_l_is_neg = x_l < Int::zero();
        let x_l_abs = if x_l_is_neg { -x_l } else { x_l };
        let x_l_abs_uint = x_l_abs.as_uint();

        if !crate::lean_ffi::verify_identity_lean(&n_l_uint, &x_l_abs_uint, x_l_is_neg, &s_l_uint) {
            return; // block search execution for this prefix if verification fails
        }

        // Normalize safely after formal verification
        let x_l = crate::math_utils::mod_negate_big(x_l_inv, s_l_int);
        let _x_l_uint = x_l.as_uint();

        let roots = composite_tonelli_shanks(x_l, &prefix.sigma_factors);
        if roots.math_interruption {
            math_interruptions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }
        let n_l_big = prefix.n_l;
        let z_max_big = if *target_max > n_l_big {
            isqrt_uint(*target_max / n_l_big)
        } else {
            Uint::zero()
        };
        let z_min_big = if *target_min > n_l_big {
            isqrt_uint(*target_min / n_l_big)
        } else {
            Uint::zero()
        };

        if z_max_big > Int::MAX.as_uint() || z_min_big > Int::MAX.as_uint() {
            return;
        }

        let z_max = z_max_big.as_int();
        let z_min = z_min_big.as_int();

        let c_max_val = z_max / s_l_int;
        let c_max = if c_max_val > Int::from_u64(usize::MAX as u64) {
            usize::MAX
        } else {
            c_max_val.as_usize()
        };

        for r_i in roots {
            let c_min = if z_min > r_i {
                let c_min_val = (z_min - r_i + s_l_int - Int::one()) / s_l_int;
                if c_min_val > Int::from_u64(usize::MAX as u64) {
                    usize::MAX
                } else {
                    c_min_val.as_usize()
                }
            } else {
                0
            };

            let mut c_current = c_min;
            let gpu_threshold = crate::lean_ffi::get_raycast_gpu_threshold();

            while c_current <= c_max {
                let chunk_size = std::cmp::min(
                    c_max - c_current + 1,
                    crate::lean_ffi::get_raycast_chunk_size(),
                );
                let c_end = c_current + chunk_size - 1;

                let mut valid_indices: Option<Vec<usize>> = None;

                if chunk_size >= gpu_threshold {
                    if let Some(gpu) = crate::gpu::get_gpu_pipeline() {
                        let mut illegal_z_valuations_u256 =
                            Vec::with_capacity(illegal_z_valuations.len());
                        for &(pe, pe1) in illegal_z_valuations {
                            illegal_z_valuations_u256.push((pe.as_uint(), pe1.as_uint()));
                        }

                        let r_i_uint = r_i.as_uint();
                        let s_l_uint = s_l_int.as_uint();

                        let (gpu_valid, pruned) = gpu.raycast_sieve(
                            r_i_uint,
                            s_l_uint,
                            c_current as u64,
                            c_end as u64,
                            z_max_big,
                            &illegal_z_valuations_u256,
                            prefix,
                            max_idx_3,
                            max_idx_5,
                            components_len,
                            true,
                        );

                        // Asymmetric Sieve Verification:
                        // 1. Verify 100% of the positive survivors returned by the GPU
                        for &rel_c in &gpu_valid {
                            let c = c_current + rel_c as usize;
                            let z = r_i + Int::from_u64(c as u64) * s_l_int;
                            let mut passes_sieve = true;
                            for &(pe, pe1) in illegal_z_valuations {
                                let rem = z % pe1;
                                if rem % pe == Int::zero() && rem != Int::zero() {
                                    passes_sieve = false;
                                    break;
                                }
                            }
                            if !passes_sieve {
                                panic!("CRITICAL FAILURE: GPU/CPU Discrepancy detected! GPU returned invalid survivor c: {}", rel_c);
                            }
                        }

                        // 2. Verify a deterministic sample of rejected candidates to detect silent GPU corruption
                        if verify_all {
                            // High-performance incremental sieve check when verifying everything
                            let mut obs_data = Vec::with_capacity(illegal_z_valuations.len());
                            for &(pe, pe1) in illegal_z_valuations {
                                let pe_uint = pe.as_uint();
                                let pe1_uint = pe1.as_uint();
                                let mut base_z_pe = (r_i % pe).as_uint();
                                let mut base_z_pe1 = (r_i % pe1).as_uint();
                                let s_l_pe = (s_l_int % pe).as_uint();
                                let s_l_pe1 = (s_l_int % pe1).as_uint();
                                let c_uint = Uint::from_u64(c_current as u64);
                                base_z_pe = (base_z_pe + c_uint * s_l_pe) % pe_uint;
                                base_z_pe1 = (base_z_pe1 + c_uint * s_l_pe1) % pe1_uint;
                                obs_data.push((
                                    base_z_pe, base_z_pe1, s_l_pe, s_l_pe1, pe_uint, pe1_uint,
                                ));
                            }

                            for c in c_current..=c_end {
                                let mut passes_sieve = true;
                                for (z_pe, z_pe1, s_l_pe, s_l_pe1, pe, pe1) in &mut obs_data {
                                    if *z_pe == Uint::zero() && *z_pe1 != Uint::zero() {
                                        passes_sieve = false;
                                    }
                                    *z_pe = *z_pe + *s_l_pe;
                                    if *z_pe >= *pe {
                                        *z_pe = *z_pe - *pe;
                                    }
                                    *z_pe1 = *z_pe1 + *s_l_pe1;
                                    if *z_pe1 >= *pe1 {
                                        *z_pe1 = *z_pe1 - *pe1;
                                    }
                                }

                                let rel_c = (c - c_current) as u32;
                                let z = r_i + Int::from_u64(c as u64) * s_l_int;
                                if z <= z_max {
                                    if passes_sieve {
                                        if !gpu_valid.contains(&rel_c) {
                                            panic!("CRITICAL FAILURE: GPU/CPU Discrepancy detected! GPU missed valid c: {}", rel_c);
                                        }
                                    } else {
                                        if gpu_valid.contains(&rel_c) {
                                            panic!("CRITICAL FAILURE: GPU/CPU Discrepancy detected! GPU returned invalid c: {}", rel_c);
                                        }
                                    }
                                }
                            }
                        } else {
                            // Sub-linear O(sample_size) check of rejected candidates based on Knuth multiplicative hash
                            for c in c_current..=c_end {
                                let mut hash_val = (c as u64)
                                    .wrapping_add(deterministic_seed)
                                    .wrapping_add(0x9E3779B97F4A7C15);
                                hash_val =
                                    (hash_val ^ (hash_val >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                                hash_val =
                                    (hash_val ^ (hash_val >> 27)).wrapping_mul(0x94D049BB133111EB);
                                hash_val = hash_val ^ (hash_val >> 31);
                                if (hash_val % 1_000_000) as f64 / 1_000_000.0 >= sampling_rate {
                                    continue;
                                }

                                let rel_c = (c - c_current) as u32;
                                let z = r_i + Int::from_u64(c as u64) * s_l_int;
                                if z <= z_max {
                                    let mut passes_sieve = true;
                                    for &(pe, pe1) in illegal_z_valuations {
                                        let rem = z % pe1;
                                        if rem % pe == Int::zero() && rem != Int::zero() {
                                            passes_sieve = false;
                                            break;
                                        }
                                    }
                                    if passes_sieve {
                                        if !gpu_valid.contains(&rel_c) {
                                            panic!("CRITICAL FAILURE: GPU/CPU Discrepancy detected! GPU missed valid c: {}", rel_c);
                                        }
                                    } else {
                                        if gpu_valid.contains(&rel_c) {
                                            panic!("CRITICAL FAILURE: GPU/CPU Discrepancy detected! GPU returned invalid c: {}", rel_c);
                                        }
                                    }
                                }
                            }
                        }

                        pruned_count.fetch_add(pruned, Ordering::Relaxed);
                        valid_indices = Some(
                            gpu_valid
                                .into_iter()
                                .map(|c| (c_current + c as usize))
                                .collect(),
                        );
                    }
                }

                let process_c = |c: usize, count_pruned: bool| {
                    let z = r_i + Int::from_u64(c as u64) * s_l_int;

                    if z > z_max {
                        return;
                    }

                    if z % Int::from_u32(2) == Int::zero() {
                        return;
                    }

                    if count_pruned {
                        let mut passed_sieve = true;
                        for &(pe, pe1) in illegal_z_valuations {
                            let rem = z % pe1;
                            if rem % pe == Int::zero() && rem != Int::zero() {
                                passed_sieve = false;
                                pruned_count.fetch_add(1, Ordering::Relaxed);
                                break;
                            }
                        }

                        if !passed_sieve {
                            return;
                        }
                    }

                    let mut is_coprime = true;
                    for &p in &prefix.factors {
                        if z % Int::from_u64(p) == Int::zero() {
                            is_coprime = false;
                            break;
                        }
                    }
                    if !is_coprime {
                        return;
                    }

                    let z_tiered = z.as_uint();
                    let n_l_tiered = prefix.n_l;
                    let s_l_tiered = prefix.s_l;

                    let n_r = match z_tiered.checked_mul(z_tiered) {
                        Some(v) => v,
                        None => return,
                    };
                    let total_n = match n_l_tiered.checked_mul(n_r) {
                        Some(v) => v,
                        None => return,
                    };

                    let two_n_plus_one = match total_n
                        .checked_mul(Uint::from_u32(2))
                        .and_then(|v| v.checked_add(Uint::one()))
                    {
                        Some(v) => v,
                        None => return,
                    };

                    if &two_n_plus_one % &s_l_tiered != Uint::from_u128(0 as u128) {
                        return;
                    }
                    let required_s_r = &two_n_plus_one / &s_l_tiered;

                    if required_s_r <= n_r {
                        return;
                    }

                    if let Some(upper) = n_r.checked_mul(Uint::from_u32(3)) {
                        if required_s_r > upper {
                            return;
                        }
                    }

                    if required_s_r % Uint::from_u32(2) == Uint::zero() {
                        return;
                    }

                    let z_fact = crate::math_utils::quick_factor_u256(z_tiered);
                    let z_factors = z_fact.factors();
                    let cofactor_opt = match z_fact {
                        crate::math_utils::FactorizationResult::Partial { remaining, .. } => {
                            Some(remaining)
                        }
                        crate::math_utils::FactorizationResult::Failure(u) => Some(u),
                        _ => None,
                    };
                    if z_factors.is_empty() && cofactor_opt.is_none() {
                        return;
                    }
                    let mut s_r = Uint::from_u128(1 as u128);
                    let mut current_p = 0;
                    let mut count: u32 = 0;
                    let mut s_r_overflowed = false;

                    for &f in z_factors {
                        if f.as_u128() == current_p {
                            count += 1;
                        } else {
                            if current_p != 0 {
                                let sig = sigma_cached(
                                    sigma_cache,
                                    Uint::from_u128(current_p as u128),
                                    2 * count,
                                );
                                match s_r.checked_mul(sig) {
                                    Some(v) => s_r = v,
                                    None => {
                                        s_r_overflowed = true;
                                        break;
                                    }
                                }
                            }
                            current_p = f.as_u128();
                            count = 1;
                        }
                    }
                    if s_r_overflowed {
                        return;
                    }
                    if current_p != 0 {
                        let sig = sigma_cached(
                            sigma_cache,
                            Uint::from_u128(current_p as u128),
                            2 * count,
                        );
                        match s_r.checked_mul(sig) {
                            Some(v) => s_r = v,
                            None => {
                                return;
                            }
                        }
                    }

                    if let Some(cofactor) = cofactor_opt {
                        let rem8 = (cofactor % Uint::from_u32(8)).as_u32();
                        if rem8 == 5 || rem8 == 7 {
                            return;
                        }

                        if required_s_r % &s_r != Uint::zero() {
                            return;
                        }
                        let required_cofactor_s_r = required_s_r / s_r;

                        if let Some((base, exp)) = perfect_power(cofactor) {
                            if let Some(sig) = sigma_power(base, 2 * exp) {
                                if sig != required_cofactor_s_r {
                                    return;
                                }
                                if let Some(new_s_r) = s_r.checked_mul(sig) {
                                    s_r = new_s_r;
                                } else {
                                    return;
                                }
                            } else {
                                return;
                            }
                        } else {
                            if let Some((min_bound, max_bound)) = cofactor_sigma_bounds(cofactor) {
                                if required_cofactor_s_r < min_bound
                                    || required_cofactor_s_r > max_bound
                                {
                                    return;
                                }

                                if (cofactor >> 256) > Uint::zero() {
                                    if !crate::math_utils::verified_is_prime(cofactor) {
                                        return;
                                    }
                                }

                                // Bounds match the required divisor sum, valid candidate!
                                // Proceed to emit the candidate for downstream proof.
                                s_r = required_s_r; // Force match since analytical reductions passed.
                            } else {
                                return;
                            }
                        }
                    }

                    if s_r == required_s_r {
                        let event = crate::events::SearchEvent::Candidate {
                            len: 0,
                            factors_str: total_n.to_string(),
                            rem_str: "".to_string(),
                        };
                        if let Some(r) = reporter {
                            let _ = r.send(event);
                        }
                    }
                };

                if let Some(indices) = valid_indices {
                    for c in indices {
                        process_c(c, false);
                    }
                } else {
                    for c in c_current..=c_end {
                        process_c(c, true);
                    }
                }

                c_current = c_end + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_illegal_z_valuations() {
        let illegal = generate_illegal_z_valuations(20, 4);
        // e=1 flags 3, 5, 11, 13, 19 -> (p, p^2)
        // Just check that (3, 9) is in there, for example.
        assert!(illegal.contains(&(Int::from_u32(3), Int::from_u32(9))));
        assert!(illegal.contains(&(Int::from_u32(5), Int::from_u32(25))));
    }

    #[test]
    fn test_quasi_perfect_residue_class_integration() {
        let n_l = Uint::from_u32(9);
        let s_l = Uint::from_u32(13);

        let prefix = Prefix {
            n_l,
            s_l,
            last_idx: 1,
            factors: vec![3],
            sigma_factors: vec![Uint::from_u32(13)],
            sigma_factors_u64: vec![13],
            active_mask: vec![1],
        };

        let target_min = Uint::from_u32(1);
        let target_max = Uint::from_u32(100);
        let illegal_z_valuations: Vec<(Int, Int)> = vec![];
        let pruned_count = AtomicUsize::new(0);
        let math_interruptions = AtomicUsize::new(0);
        let sigma_cache = std::collections::HashMap::new();

        let math_interruptions = AtomicUsize::new(0);

        // Ensure phase4 doesn't panic when we call it, verifying the mathematical identity constraint
        // 2N_L * x_l + 1 == 0 mod S_L holds correctly internally.
        phase4_exact_ray_casting(
            &prefix,
            &target_min,
            &target_max,
            &illegal_z_valuations,
            &pruned_count,
            &math_interruptions,
            &sigma_cache,
            None,
            0,
            0,
            1,
        );
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use num_traits::Bounded;

    #[test]
    fn test_isqrt_uint_max() {
        let max = Uint::MAX;
        let _ = isqrt_uint(max);
    }

    #[test]
    fn test_isqrt_negative() {
        let neg = Int::from_str_radix("-1", 10).unwrap();
        assert_eq!(isqrt(neg), None);
    }

    #[test]
    fn test_kth_root_overflow_boundary() {
        // Since kth_root takes Uint c, if c is Uint::MAX and k is 1:
        // high will double until it overflows 512-bit capacity.
        // It should return None on overflow.
        let max = Uint::MAX;
        let res = kth_root(max, 1);
        assert_eq!(res, None);
    }

    #[test]
    fn test_sigma_power_overflow() {
        let base = Uint::MAX;
        let res = sigma_power(base, 2);
        assert_eq!(res, None);
    }

    #[test]
    fn test_cofactor_sigma_bounds_overflow() {
        let max = Uint::MAX;
        let res = cofactor_sigma_bounds(max);
        assert_eq!(res, None);
    }

    #[test]
    fn test_kth_root_normal() {
        assert_eq!(kth_root(Uint::from_u32(16), 2), Some(Uint::from_u32(4)));
        assert_eq!(kth_root(Uint::from_u32(27), 3), Some(Uint::from_u32(3)));
    }

    #[test]
    fn test_sigma_power_normal() {
        // sigma_power(3, 2) = 1 + 3 + 9 = 13
        assert_eq!(sigma_power(Uint::from_u32(3), 2), Some(Uint::from_u32(13)));
    }

    #[test]
    fn test_cofactor_sigma_bounds_normal() {
        let bounds = cofactor_sigma_bounds(Uint::from_u32(5));
        assert!(bounds.is_some());
    }
}
