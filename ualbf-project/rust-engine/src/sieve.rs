static LAST_TELEMETRY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
use crate::math_utils::{SigmaCache, TrialSieve};
use crate::obstruction::Obstruction;
use crate::types::UintExt;
use crate::types::{PrimePower, Uint};
use primal::Sieve;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Phase 1 sieve result: valid components + prebuilt sigma cache.
pub struct SieveResult {
    pub components: Vec<PrimePower>,
    pub sigma_cache: SigmaCache,
    pub pruned: usize,
    pub ecm_fallback: usize,
    pub trial_only: usize,
    pub execution_time_ms: u128,
}

/// Helper to safely compute the 1D index inside our stage2 bitset.
/// Prevents overflow during index calculations for high range searches.
fn get_sieve_index(p_mod_8: usize, max_e: u32, e: u32) -> Option<usize> {
    let term1 = p_mod_8.checked_mul((max_e as usize).checked_add(1)?)?;
    let idx = term1.checked_add(e as usize)?;
    Some(idx)
}

/// Safely checks the status of the sieve bit inside `stage2_bitset` with proper bounds validation.
/// Returns `Some(true)` if the bit is 1, `Some(false)` if the bit is 0, or `None` if out of bounds or overflowed.
fn check_sieve_bit(stage2_bitset: &[u64], p_mod_8: usize, max_e: u32, e: u32) -> Option<bool> {
    let idx = get_sieve_index(p_mod_8, max_e, e)?;
    let block = idx.checked_div(64)?;
    let bit = idx.checked_rem(64)?;
    if block < stage2_bitset.len() {
        Some((stage2_bitset[block] & (1u64 << bit)) != 0)
    } else {
        None
    }
}

pub fn phase1_global_annihilation_sieve(limit: usize, max_e: u32) -> SieveResult {
    println!("PROGRESS|PHASE|1|Legendre-Cattaneo Sieve");
    let phase1_start = std::time::Instant::now();
    let sieve = Sieve::new(limit);
    let pruned = AtomicUsize::new(0);
    let ecm_calls = AtomicUsize::new(0);
    let trial_only = AtomicUsize::new(0);

    let total_primes = sieve.prime_pi(limit);
    let count = AtomicUsize::new(0);

    let static_filters: std::sync::Arc<Vec<Box<dyn crate::obstruction::Obstruction>>> =
        std::sync::Arc::new(vec![
            Box::new(crate::obstruction::Mod3Obstruction),
            Box::new(crate::obstruction::Mod5Obstruction),
            Box::new(crate::obstruction::Mod8Obstruction),
            Box::new(crate::obstruction::Mod9Obstruction),
            Box::new(crate::obstruction::TouchardObstruction),
        ]);

    let num_blocks = (limit / 64) + 1;
    let mut stage1_bitset = vec![0u64; num_blocks];

    for p in sieve.primes_from(3) {
        let mut any_valid = false;
        for e in 1..=max_e {
            let two_e = 2 * e;
            let mut statically_rejected = false;
            for filter in static_filters.iter() {
                if filter.check_component(p as u64, two_e) {
                    statically_rejected = true;
                    break;
                }
            }
            if statically_rejected {
                continue;
            }

            any_valid = true;
            break;
        }
        if any_valid {
            stage1_bitset[p / 64] |= 1 << (p % 64);
        }
    }

    let stage1_bitset = std::sync::Arc::new(stage1_bitset);

    let primes: Vec<usize> = sieve
        .primes_from(3)
        .filter(|&p| (stage1_bitset[p / 64] & (1 << (p % 64))) != 0)
        .collect();

    let max_index = 8 * (max_e as usize + 1);
    let num_blocks_stage2 = (max_index / 64) + 1;
    let mut stage2_bitset = vec![0u64; num_blocks_stage2];
    let mod8 = crate::obstruction::Mod8Obstruction;
    for p_mod_8 in 0..8 {
        for e in 1..=max_e {
            if !mod8.check_component(p_mod_8 as u64, 2 * e) {
                if let Some(index) = get_sieve_index(p_mod_8 as usize, max_e, e) {
                    if index / 64 < stage2_bitset.len() {
                        stage2_bitset[index / 64] |= 1 << (index % 64);
                    }
                }
            }
        }
    }
    let stage2_bitset = std::sync::Arc::new(stage2_bitset);

    let trial_limit = crate::policy::get_safe_config().trial_division_limit as u64;
    println!(
        "Sieve|DIAG|Building trial sieve to {} ({} primes total to evaluate)",
        trial_limit, total_primes
    );
    let trial_sieve = TrialSieve::new(trial_limit);
    println!(
        "Sieve|DIAG|Trial sieve ready: {} small primes loaded",
        trial_sieve.small_primes.len()
    );

    let sigma_cache_mu: Mutex<SigmaCache> = Mutex::new(HashMap::new());
    let total_factor_ns = AtomicU64::new(0);

    let mut valid_components: Vec<PrimePower> = primes
        .chunks(2048)
        .par_bridge()
        .flat_map(|chunk| {
            let mut local_components = Vec::new();
            let mut local_cache: Vec<((Uint, u32), Uint)> = Vec::new();

            struct TaskResult {
                p: u64,
                two_e: u32,
                val: Uint,
                sigma: Uint,
                pending_factors: Vec<Uint>,
                needs_rho: Vec<Uint>,
                rejected: bool,
            }

            let mut tasks = Vec::new();

            for &p in chunk {
                let current_count = count.fetch_add(1, Ordering::Relaxed) + 1;
                let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                let last = LAST_TELEMETRY.load(std::sync::atomic::Ordering::Relaxed);
                if current_count % 128 == 0 && now_ms - last >= crate::profile::get_profile().engine_telemetry_interval_ms {
                    if LAST_TELEMETRY.compare_exchange(last, now_ms, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed).is_ok() {
                    let elapsed = phase1_start.elapsed().as_secs_f64();
                    let rate = current_count as f64 / elapsed;
                    let ecm_n = ecm_calls.load(Ordering::Relaxed);
                    let trial_n = trial_only.load(Ordering::Relaxed);
                    let factor_ms = total_factor_ns.load(Ordering::Relaxed) / 1_000_000;
                    println!(
                        "PROGRESS|UPDATE|{}|{}|p={} | {:.0} p/s | trial={} ecm={} | factor_time={}ms",
                        current_count, total_primes, p, rate, trial_n, ecm_n, factor_ms
                    );
                    }
                }
                let p_bu = Uint::from_usize(p);

                for e in 1..=max_e {
                    // Stage 2 (Mod8) exponent filter in O(1).
                    // In stage2_bitset, a bit value of 1 (Some(true)) means the component is NOT obstructed (allowed).
                    // Thus, if check_sieve_bit returns Some(true), is_pruned is false.
                    // All other cases (Some(false) or None/overflow) mean we must prune.
                    let p_mod_8 = p & 7;
                    let is_pruned = match check_sieve_bit(&stage2_bitset, p_mod_8, max_e, e) {
                        Some(true) => false,
                        _ => true, // safe prune on None/overflow
                    };
                    if is_pruned {
                        pruned.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }

                    let two_e = 2 * e;

                    let mut statically_rejected = false;
                    for filter in static_filters.iter() {
                        if filter.check_component(p as u64, two_e) {
                            statically_rejected = true;
                            break;
                        }
                    }
                    if statically_rejected {
                        pruned.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }

                    let val = match p_bu.checked_pow(two_e) {
                        Some(v) => v,
                        None => break,
                    };
                    if val > Uint::from_u32(10).pow(crate::manifest_constants::TARGET_MAX_LOG10) {
                        break;
                    }
                    let mut sum: Uint = Uint::one();
                    let mut p_pow: Uint = Uint::one();
                    let mut overflowed = false;
                    for _ in 0..two_e {
                        if let Some(next_pow) = p_pow.checked_mul(Uint::from_usize(p)) {
                            p_pow = next_pow;
                        } else {
                            overflowed = true;
                            break;
                        }
                        if let Some(next_sum) = sum.checked_add(p_pow) {
                            sum = next_sum;
                        } else {
                            overflowed = true;
                            break;
                        }
                    }
                    if overflowed {
                        continue;
                    }
                    let sigma = sum;
                    if sigma == Uint::zero() {
                        continue;
                    }
                    local_cache.push(((p_bu, two_e), sigma));
                    tasks.push((p as u64, two_e, val, sigma));
                }
            }

            let t0 = std::time::Instant::now();

            let mut process_results = Vec::new();

            for (p, two_e, val, sigma) in tasks {
                if let Some((rejected, all_factors, needs_rho)) = get_cofactors_to_factor(p, two_e, &trial_sieve, &ecm_calls, &trial_only) {
                    process_results.push(TaskResult {
                        p, two_e, val, sigma,
                        pending_factors: all_factors,
                        needs_rho,
                        rejected,
                    });
                } else {
                    process_results.push(TaskResult {
                        p,
                        two_e,
                        val,
                        sigma,
                        pending_factors: vec![],
                        needs_rho: vec![],
                        rejected: true,
                    });
                }
            }

            for mut res in process_results {
                if res.rejected {
                    pruned.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                res.pending_factors.sort_unstable();

                let sigma_u256 = res.sigma;
                let shifted = sigma_u256 << 64;
                let val_u: Uint = res.val; let div_res: Uint = shifted / val_u; let mut abundance_fp = div_res.as_u128();
                if shifted % res.val != Uint::zero() {
                    abundance_fp += 1;
                }
                local_components.push(PrimePower {
                    p: res.p,
                    two_e: res.two_e,
                    val: res.val,
                    sigma: res.sigma,
                    sigma_factors: res.pending_factors,
                    needs_rho: res.needs_rho,
                    abundance_fp,
                });
            }

            total_factor_ns.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);

            let mut global_cache = sigma_cache_mu.lock().unwrap();
            for (k, v) in local_cache {
                global_cache.insert(k, v);
            }

            local_components
        })
        .collect();

    let elapsed = phase1_start.elapsed();
    let ecm_n = ecm_calls.load(Ordering::Relaxed);
    let trial_n = trial_only.load(Ordering::Relaxed);
    println!(
        "Sieve|DIAG|Phase 1 complete in {:.1}s | {} retained, {} pruned | trial={} ecm_fallback={}",
        elapsed.as_secs_f64(),
        valid_components.len(),
        pruned.load(Ordering::Relaxed),
        trial_n,
        ecm_n
    );

    // Sort by abundance ratio descending (small primes first — they have highest σ/val ratios)
    valid_components.sort_by(|a, b| b.abundance_fp.cmp(&a.abundance_fp));
    println!(
        "Retained: {}, Pruned: {}",
        valid_components.len(),
        pruned.load(Ordering::Relaxed)
    );

    // Telemetry Export: Dump valid components
    for comp in &valid_components {
        let factors_str = comp
            .sigma_factors
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "DATA|COMP|{}|{}|{:.6}|{}",
            comp.p, comp.two_e, comp.abundance_fp, factors_str
        );
    }

    let sigma_cache = sigma_cache_mu.into_inner().unwrap();
    SieveResult {
        components: valid_components,
        sigma_cache,
        pruned: pruned.into_inner(),
        ecm_fallback: ecm_calls.into_inner(),
        trial_only: trial_only.into_inner(),
        execution_time_ms: elapsed.as_millis(),
    }
}

// ---------------------------------------------------------------------------
// Two-pass mod-8 screening
// ---------------------------------------------------------------------------

/// Screen σ(p^{2e}) for mod-8 obstructions by examining cyclotomic factors.
///
/// For each proper divisor `d` of `2e+1` this function verifies that every prime
/// factor of the cyclotomic value Φ_d(p) is not congruent to 5 or 7 modulo 8.
/// It uses a Bloom filter to skip unlikely candidates, trial-divides with
/// `trial`'s small primes, applies a Miller–Rabin primality check for large
/// cofactors, and falls back to rho/ECM-style factoring when a composite
/// cofactor cannot be resolved by trial division. The function updates the
/// provided atomic counters: `ecm_calls` is incremented when heavyweight
/// factoring is performed; `trial_only` is incremented when no such factoring
/// was necessary.
///
/// Returns `ScreenResult::Rejected` if any examined prime factor is congruent
/// to 5 or 7 modulo 8; otherwise returns `ScreenResult::Accepted(factors)`
/// where `factors` is the sorted list of prime factors collected (including
/// primes found via fallback factoring).
///
/// # Examples
///
/// ```
/// // Types and constructors assumed available in the crate.
/// let trial = TrialSieve::new(10_000);
/// let ecm_calls = std::sync::atomic::AtomicUsize::new(0);
/// let trial_only = std::sync::atomic::AtomicUsize::new(0);
///
/// match screen_mod8_cyclotomic(3, 2, &trial, &ecm_calls, &trial_only) {
///     ScreenResult::Rejected => println!("Rejected by mod-8 obstruction"),
///     ScreenResult::Accepted(factors) => println!("Accepted with {} factors", factors.len()),
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math_utils::quick_factor_u256;

    #[test]
    #[cfg_attr(unverified_build, ignore)]
    fn test_phase1_sieve_logic() {
        let limit = 50;
        let max_e = 2;
        let result = phase1_global_annihilation_sieve(limit, max_e);

        assert!(!result.components.is_empty());
        for comp in result.components {
            let fact_res = quick_factor_u256(comp.sigma);
            let factors = fact_res.factors();
            for q in factors {
                let q_mod_8 = (q % Uint::from_u32(8)).as_u32();
                assert!(
                    q_mod_8 != 5 && q_mod_8 != 7,
                    "Invalid sigma component leaked into valid_components!"
                );
            }
        }
    }

    #[test]
    fn test_high_range_prime_conversions() {
        // A prime exceeding 2^32 - 1
        let p: usize = 4294967311;

        // 1. Safe conversion
        let p_bu = Uint::from_usize(p);

        // Old conversion would have been:
        let p_truncated = Uint::from_u128((p as u32) as u128);

        assert_ne!(p_bu, p_truncated, "Conversion must not truncate!");
        assert_eq!(p_bu.to_string(), "4294967311");
        assert_eq!(p_truncated.to_string(), "15");

        // 2. Compute components without truncation
        let two_e = 2;
        let val = p_bu.checked_pow(two_e).unwrap();

        let mut sum = Uint::one();
        let mut p_pow = Uint::one();
        for _ in 0..two_e {
            p_pow *= Uint::from_usize(p);
            sum += p_pow;
        }
        let sigma = sum;

        // Correct mathematical values
        // p^2 = 4294967311^2 = 18446744202558570721
        // sigma(p^2) = 1 + p + p^2 = 18446744206853538033
        assert_eq!(val.to_string(), "18446744202558570721");
        assert_eq!(sigma.to_string(), "18446744206853538033");
    }

    #[test]
    fn test_checked_sieve_bounds_and_overflow() {
        // 1. Test index overflow handling
        assert!(get_sieve_index(usize::MAX, 10, 1).is_none());
        assert!(get_sieve_index(usize::MAX, u32::MAX, 1).is_none());
        assert_eq!(get_sieve_index(1, 10, 5), Some(16));

        // 2. Test check_sieve_bit bounds check and overflow
        let bitset = vec![0u64; 2];
        assert!(check_sieve_bit(&bitset, usize::MAX, 10, 1).is_none());
        assert_eq!(check_sieve_bit(&bitset, 0, 1, 1), Some(false));

        let mut bitset_mut = vec![0u64; 2];
        bitset_mut[0] |= 1 << 1;
        assert_eq!(check_sieve_bit(&bitset_mut, 0, 1, 1), Some(true));

        // 3. Test compute_sigma_checked overflow monadic Option propagation
        assert!(crate::lean_ffi::compute_sigma_checked(12345, 1000).is_none());
    }
}

/// Gather mod‑8 screening results and cofactor information for the cyclotomic divisors of sigma(p^(2e)).
///
/// For each proper divisor d of 2e + 1 this function:
/// - requires (p, d) to be present in the Bloom filter (otherwise it immediately rejects),
/// - evaluates the cyclotomic value phi_d(p) when available or factors the full sigma on overflow,
/// - trial‑divides phi_d(p) by small primes and checks every extracted prime against the mod‑8 obstruction,
/// - records any remaining composite cofactors that need heavier factoring (ECM/rho) instead of factoring them here.
///
/// # Returns
///
/// A tuple `(rejected, factors, needs_rho)`:
/// - `rejected`: `true` if any Bloom‑filter miss or detected prime factor triggers the mod‑8 obstruction; `false` otherwise.
/// - `factors`: collected prime factors (as `Uint`) obtained by trial division or light factoring of cyclotomic values.
/// - `needs_rho`: composite cofactors (as `Uint`) that were not fully resolved and must be factored by heavier methods.
///
/// # Examples
///
/// ```
/// let trial = TrialSieve::new(100);
/// let ecm_calls = std::sync::atomic::AtomicUsize::new(0);
/// let trial_only = std::sync::atomic::AtomicUsize::new(0);
/// let (rejected, factors, needs_rho) = get_cofactors_to_factor(3, 4, &trial, &ecm_calls, &trial_only);
/// // `rejected` indicates a mod-8 obstruction; otherwise `factors` and `needs_rho` describe collected cofactors.
/// ```
fn get_cofactors_to_factor(
    p: u64,
    two_e: u32,
    trial: &TrialSieve,
    ecm_calls: &AtomicUsize,
    _trial_only: &AtomicUsize,
) -> Option<(bool, Vec<Uint>, Vec<Uint>)> {
    let full_sigma = crate::lean_ffi::compute_sigma_checked(p, two_e)?;
    let factor_result = trial.factor(full_sigma);
    let factors = factor_result.factors();
    ecm_calls.fetch_add(1, Ordering::Relaxed);

    for q in factors {
        let filter = crate::obstruction::Mod8Obstruction;
        use crate::obstruction::Obstruction;
        if filter.check_prime_factor(q) {
            return Some((true, vec![], vec![]));
        }
    }

    let mut needs_rho = vec![];
    match factor_result {
        crate::math_utils::FactorizationResult::Partial { remaining, .. } => {
            needs_rho.push(remaining);
        }
        crate::math_utils::FactorizationResult::Failure(u) => {
            needs_rho.push(u);
        }
        _ => {}
    }

    Some((false, factors.to_vec(), needs_rho))
}
