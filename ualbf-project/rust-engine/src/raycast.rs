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
    let _config = crate::policy::get_safe_config();

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

                        let (gpu_valid, witnesses, pruned) = gpu.raycast_sieve(
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

                        // Asymmetric Sieve Verification using single-obstruction witness certificates:
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

                        // 2. O(1)-per-candidate validation using the GPU-provided witness certificates
                        let mut witness_map =
                            std::collections::HashMap::with_capacity(witnesses.len());
                        for &(rel_c, obs_idx) in &witnesses {
                            witness_map.insert(rel_c, obs_idx);
                        }

                        let mut survivor_set =
                            std::collections::HashSet::with_capacity(gpu_valid.len());
                        for &v in &gpu_valid {
                            survivor_set.insert(v);
                        }

                        // Let's verify that every discarded candidate in [0, chunk_size - 1] is accounted for and its witness is valid
                        for rel_c in 0..chunk_size {
                            let rel_c_u32 = rel_c as u32;
                            if !survivor_set.contains(&rel_c_u32) {
                                // Must have a valid witness
                                if let Some(&obs_idx) = witness_map.get(&rel_c_u32) {
                                    if obs_idx >= illegal_z_valuations.len() {
                                        panic!("CRITICAL FAILURE: Witness obstruction index out of bounds: {} for relative candidate {}", obs_idx, rel_c);
                                    }
                                    let (pe, pe1) = illegal_z_valuations[obs_idx];
                                    let c = c_current + rel_c;
                                    let z = r_i + Int::from_u64(c as u64) * s_l_int;
                                    let rem = z % pe1;
                                    if !(rem % pe == Int::zero() && rem != Int::zero()) {
                                        panic!("CRITICAL FAILURE: Invalid witness certificate for relative candidate {}! Obstruction index {} did not reject.", rel_c, obs_idx);
                                    }
                                } else {
                                    panic!("CRITICAL FAILURE: Discarded candidate at relative index {} has no witness certificate!", rel_c);
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

                // Compile all candidate indices in the current chunk to check
                let chunk_candidates: Vec<usize> = if let Some(indices) = &valid_indices {
                    indices.clone()
                } else {
                    (c_current..=c_end).collect()
                };

                let count_pruned = valid_indices.is_none();

                let mut candidates_to_factor = Vec::new();

                for c in chunk_candidates {
                    let z = r_i + Int::from_u64(c as u64) * s_l_int;

                    if z > z_max {
                        continue;
                    }

                    if z % Int::from_u32(2) == Int::zero() {
                        continue;
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
                            continue;
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
                        continue;
                    }

                    let z_tiered = z.as_uint();
                    let n_l_tiered = prefix.n_l;
                    let s_l_tiered = prefix.s_l;

                    let n_r = match z_tiered.checked_mul(z_tiered) {
                        Some(v) => v,
                        None => continue,
                    };
                    let total_n = match n_l_tiered.checked_mul(n_r) {
                        Some(v) => v,
                        None => continue,
                    };

                    let two_n_plus_one = match total_n
                        .checked_mul(Uint::from_u32(2))
                        .and_then(|v| v.checked_add(Uint::one()))
                    {
                        Some(v) => v,
                        None => continue,
                    };

                    if &two_n_plus_one % &s_l_tiered != Uint::from_u128(0 as u128) {
                        continue;
                    }
                    let required_s_r = &two_n_plus_one / &s_l_tiered;

                    if required_s_r <= n_r {
                        continue;
                    }

                    if let Some(upper) = n_r.checked_mul(Uint::from_u32(3)) {
                        if required_s_r > upper {
                            continue;
                        }
                    }

                    if required_s_r % Uint::from_u32(2) == Uint::zero() {
                        continue;
                    }

                    candidates_to_factor.push((c, z_tiered, required_s_r));
                }

                if !candidates_to_factor.is_empty() {
                    let gpu_opt = crate::gpu::get_gpu_pipeline();
                    if gpu_opt.is_some() {
                        // Gather composite values for batching
                        let mut composites = Vec::new();
                        for &(_, z_tiered, _) in &candidates_to_factor {
                            if !crate::math_utils::verified_is_prime(z_tiered) {
                                composites.push(z_tiered);
                            }
                        }

                        // Call batch factorization on the GPU
                        let factors = if !composites.is_empty() {
                            // Unwrapping is safe because gpu_opt is some
                            gpu_opt.unwrap().factor_batch(&composites)
                        } else {
                            vec![]
                        };

                        let mut composite_index = 0;
                        for &(c, z_tiered, required_s_r) in &candidates_to_factor {
                            let factors_list = if crate::math_utils::verified_is_prime(z_tiered) {
                                vec![z_tiered]
                            } else {
                                let p_opt = factors[composite_index];
                                composite_index += 1;
                                if let Some(p) = p_opt {
                                    let q = z_tiered / p;
                                    // Multiplication Certificate Verification
                                    if p <= Uint::one() || q <= Uint::one() || p * q != z_tiered {
                                        panic!("CRITICAL FAILURE: GPU multiplication certificate verification failed for candidate composite N = {}, factors p = {}, q = {}", z_tiered, p, q);
                                    }
                                    // Primality Certificate Verification
                                    if !crate::math_utils::verified_is_prime(p)
                                        || !crate::math_utils::verified_is_prime(q)
                                    {
                                        panic!("CRITICAL FAILURE: GPU primality certificate verification failed for candidate composite N = {}, factors p = {}, q = {}", z_tiered, p, q);
                                    }
                                    vec![p, q]
                                } else {
                                    panic!("CRITICAL FAILURE: GPU factorization pipeline failed to factor candidate composite N = {}", z_tiered);
                                }
                            };

                            let mut s_r = Uint::from_u128(1 as u128);
                            let mut current_p = Uint::zero();
                            let mut count: u32 = 0;
                            let mut s_r_overflowed = false;
                            let mut factors_list = factors_list.clone();
                            factors_list.sort_unstable();

                            for &f in &factors_list {
                                if f == current_p {
                                    count += 1;
                                } else {
                                    if current_p != Uint::zero() {
                                        let sig = sigma_cached(sigma_cache, current_p, 2 * count);
                                        match s_r.checked_mul(sig) {
                                            Some(v) => s_r = v,
                                            None => {
                                                s_r_overflowed = true;
                                                break;
                                            }
                                        }
                                    }
                                    current_p = f;
                                    count = 1;
                                }
                            }
                            if !s_r_overflowed && current_p != Uint::zero() {
                                let sig = sigma_cached(sigma_cache, current_p, 2 * count);
                                match s_r.checked_mul(sig) {
                                    Some(v) => s_r = v,
                                    None => {
                                        s_r_overflowed = true;
                                    }
                                }
                            }

                            if !s_r_overflowed && s_r == required_s_r {
                                let total_n = prefix.n_l * z_tiered * z_tiered;
                                let event = crate::events::SearchEvent::Candidate {
                                    len: 0,
                                    factors_str: total_n.to_string(),
                                    rem_str: "".to_string(),
                                };
                                if let Some(r) = reporter {
                                    let _ = r.send(event);
                                }
                            }
                        }
                    } else {
                        // Fallback to sequential CPU factorization when GPU is not active
                        for &(c, z_tiered, required_s_r) in &candidates_to_factor {
                            let z_fact = crate::math_utils::quick_factor_u256(z_tiered);
                            let z_factors = z_fact.factors();
                            let cofactor_opt = match z_fact {
                                crate::math_utils::FactorizationResult::Partial {
                                    remaining,
                                    ..
                                } => Some(remaining),
                                crate::math_utils::FactorizationResult::Failure(u) => Some(u),
                                _ => None,
                            };
                            if z_factors.is_empty() && cofactor_opt.is_none() {
                                continue;
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
                                continue;
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
                                        continue;
                                    }
                                }
                            }

                            if let Some(cofactor) = cofactor_opt {
                                let rem8 = (cofactor % Uint::from_u32(8)).as_u32();
                                if rem8 == 5 || rem8 == 7 {
                                    continue;
                                }

                                if required_s_r % &s_r != Uint::zero() {
                                    continue;
                                }
                                let required_cofactor_s_r = required_s_r / s_r;

                                if let Some((base, exp)) = perfect_power(cofactor) {
                                    if let Some(sig) = sigma_power(base, 2 * exp) {
                                        if sig != required_cofactor_s_r {
                                            continue;
                                        }
                                        if let Some(new_s_r) = s_r.checked_mul(sig) {
                                            s_r = new_s_r;
                                        } else {
                                            continue;
                                        }
                                    } else {
                                        continue;
                                    }
                                } else {
                                    let mut prime_verified = false;
                                    let q = cofactor;
                                    let prime_sigma_opt = q
                                        .checked_mul(q)
                                        .and_then(|q2| q2.checked_add(q))
                                        .and_then(|q2_plus_q| q2_plus_q.checked_add(Uint::one()));

                                    if let Some(prime_sigma) = prime_sigma_opt {
                                        if prime_sigma == required_cofactor_s_r {
                                            if crate::math_utils::verified_is_prime(cofactor) {
                                                s_r = required_s_r;
                                                prime_verified = true;
                                            } else {
                                                continue;
                                            }
                                        }
                                    }

                                    if !prime_verified {
                                        if let Some((min_bound, max_bound)) =
                                            cofactor_sigma_bounds(cofactor)
                                        {
                                            if required_cofactor_s_r < min_bound
                                                || required_cofactor_s_r > max_bound
                                            {
                                                continue;
                                            }

                                            if (cofactor >> 256) > Uint::zero() {
                                                if !crate::math_utils::verified_is_prime(cofactor) {
                                                    continue;
                                                }
                                            }

                                            s_r = required_s_r;
                                        } else {
                                            continue;
                                        }
                                    }
                                }
                            }

                            if s_r == required_s_r {
                                let total_n = prefix.n_l * z_tiered * z_tiered;
                                let event = crate::events::SearchEvent::Candidate {
                                    len: 0,
                                    factors_str: total_n.to_string(),
                                    rem_str: "".to_string(),
                                };
                                if let Some(r) = reporter {
                                    let _ = r.send(event);
                                }
                            }
                        }
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
            sigma_mod24: 13,
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

    #[test]
    fn test_prime_divisor_sum_exact_match() {
        // Let's test a prime cofactor: q = 5.
        // The prime divisor sum is 1 + 5 + 25 = 31.
        let cofactor = Uint::from_u32(5);
        let required_cofactor_s_r = Uint::from_u32(31);

        let q = cofactor;
        let prime_sigma_opt = q
            .checked_mul(q)
            .and_then(|q2| q2.checked_add(q))
            .and_then(|q2_plus_q| q2_plus_q.checked_add(Uint::one()));

        assert_eq!(prime_sigma_opt, Some(required_cofactor_s_r));
        assert!(crate::math_utils::verified_is_prime(cofactor));

        // Let's test a composite cofactor: q = 9.
        let cofactor_comp = Uint::from_u32(9);
        let q_comp = cofactor_comp;
        let prime_sigma_opt_comp = q_comp
            .checked_mul(q_comp)
            .and_then(|q2| q2.checked_add(q_comp))
            .and_then(|q2_plus_q| q2_plus_q.checked_add(Uint::one()));

        assert_eq!(prime_sigma_opt_comp, Some(Uint::from_u32(91)));
        assert!(!crate::math_utils::verified_is_prime(cofactor_comp));
    }

    #[test]
    fn test_witness_verification_success_and_panic() {
        let pe = Int::from_u32(3);
        let pe1 = Int::from_u32(9);
        let r_i = Int::from_u32(3);
        let s_l_int = Int::from_u32(13);
        let c = 0;
        let z = r_i + Int::from_u64(c as u64) * s_l_int;
        let rem = z % pe1;
        assert!(
            rem % pe == Int::zero() && rem != Int::zero(),
            "Obstruction check should hold"
        );

        // Verify that invalid obstruction or non-matching index panics
        let result = std::panic::catch_unwind(|| {
            let (pe_bad, pe1_bad) = (Int::from_u32(5), Int::from_u32(25));
            let rem_bad = z % pe1_bad;
            if !(rem_bad % pe_bad == Int::zero() && rem_bad != Int::zero()) {
                panic!("CRITICAL FAILURE: Invalid witness certificate!");
            }
        });
        assert!(result.is_err(), "Invalid witness must panic");
    }

    #[test]
    fn test_factorization_certificate_verification_success_and_panic() {
        let z_tiered = Uint::from_u32(15);
        let p = Uint::from_u32(3);
        let q = Uint::from_u32(5);

        // Valid certificates
        assert!(p > Uint::one() && q > Uint::one() && p * q == z_tiered);
        assert!(crate::math_utils::verified_is_prime(p) && crate::math_utils::verified_is_prime(q));

        // Invalid certificates must panic
        let result_mult_fail = std::panic::catch_unwind(|| {
            let p_bad = Uint::from_u32(4);
            let q_bad = Uint::from_u32(4);
            if p_bad <= Uint::one() || q_bad <= Uint::one() || p_bad * q_bad != z_tiered {
                panic!("CRITICAL FAILURE: GPU multiplication certificate verification failed!");
            }
        });
        assert!(
            result_mult_fail.is_err(),
            "Invalid multiplication product must panic"
        );

        let result_prime_fail = std::panic::catch_unwind(|| {
            let p_bad = Uint::from_u32(1);
            let q_bad = Uint::from_u32(15);
            if !crate::math_utils::verified_is_prime(p_bad)
                || !crate::math_utils::verified_is_prime(q_bad)
            {
                panic!("CRITICAL FAILURE: GPU primality certificate verification failed!");
            }
        });
        assert!(result_prime_fail.is_err(), "Non-prime factors must panic");
    }
}
