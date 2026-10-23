use crate::types::{Uint, UintExt};

/// Centralized modular dispatch wrapper for starvation checks.
/// Automatically executes Verus-verified 128-bit fast paths when values safely fit,
/// and falls back to exact 512-bit mathematical checks to prevent silent skipped evaluations.
pub fn dispatch_starvation_check(
    s_l: Uint,
    n_l: Uint,
    best_remaining: u128,
    target_num: u64,
    target_den: u64,
) -> bool {
    let s_l_128_opt: Option<u128> = s_l.try_into().ok();
    let n_l_128_opt: Option<u128> = n_l.try_into().ok();

    if let (Some(s_l_128), Some(n_l_128)) = (s_l_128_opt, n_l_128_opt) {
        if n_l_128 <= u128::MAX / 2 {
            let best_num_opt = (best_remaining).checked_mul(target_den as u128);
            let best_den_opt = (1u128 << 63).checked_mul(target_num as u128);

            if let (Some(best_num), Some(best_den)) = (best_num_opt, best_den_opt) {
                if s_l_128 > 0 && n_l_128 > 0 && best_num > 0 && best_den > 0 {
                    if s_l_128 <= u128::MAX / best_num
                        && n_l_128 <= u128::MAX / 2
                        && (n_l_128 * 2) <= u128::MAX / best_den
                    {
                        return crate::verus_proofs::check_starvation_kill(
                            s_l_128, n_l_128, best_num, best_den,
                        );
                    }
                }
            }
        }
    }

    // Fallback to exact 512-bit mathematical check
    let best_remaining_u512 = Uint::from_u128(best_remaining);
    let target_num_u512 = Uint::from_u64(target_num);
    let target_den_u512 = Uint::from_u64(target_den);

    if let (Some(l), Some(r)) = (
        s_l.checked_mul(best_remaining_u512)
            .and_then(|x| x.checked_mul(target_den_u512)),
        n_l.checked_mul(target_num_u512)
            .and_then(|x| x.checked_shl(64)),
    ) {
        l < r
    } else {
        false
    }
}

/// Centralized modular dispatch wrapper for CDG forced pruning checks.
/// Automatically executes Verus-verified 128-bit fast paths when values safely fit,
/// and falls back to exact 512-bit mathematical checks to prevent silent skipped evaluations.
pub fn dispatch_cdg_forced_check(
    s_l: Uint,
    n_l: Uint,
    forced_num: Uint,
    forced_den: Uint,
    target_num: u64,
    target_den: u64,
) -> bool {
    let s_l_128_opt: Option<u128> = s_l.try_into().ok();
    let n_l_128_opt: Option<u128> = n_l.try_into().ok();
    let f_num_128_opt: Option<u128> = forced_num.try_into().ok();
    let f_den_128_opt: Option<u128> = forced_den.try_into().ok();

    if let (Some(s_l_128), Some(n_l_128), Some(f_num_128), Some(f_den_128)) =
        (s_l_128_opt, n_l_128_opt, f_num_128_opt, f_den_128_opt)
    {
        let t_num_val = target_num as u128;
        let t_den_val = target_den as u128;

        if s_l_128 > 0 && n_l_128 > 0 && f_num_128 > 0 && f_den_128 > 0 {
            let lhs_ok = s_l_128
                .checked_mul(f_num_128)
                .and_then(|x| x.checked_mul(t_den_val))
                .is_some();
            let rhs_ok = n_l_128
                .checked_mul(f_den_128)
                .and_then(|x| x.checked_mul(t_num_val))
                .is_some();

            if lhs_ok && rhs_ok {
                let pruned = crate::verus_proofs::check_cdg_forced_kill(
                    s_l_128, n_l_128, f_num_128, f_den_128, t_num_val, t_den_val,
                );
                if pruned {
                    return true;
                }
            }
        }
    }

    // Fallback to exact 512-bit mathematical check
    crate::universal_bounds::cpu_check_cdg_forced(
        &s_l,
        &n_l,
        &forced_num,
        &forced_den,
        target_num,
        target_den,
    )
}

/// Centralized Touchard mod-24 residue reachability check dispatch wrapper.
/// Routes reachability evaluations through proof-mode checks.
/// Returns `true` if the branch is mod-24 unreachable and should be pruned; `false` otherwise.
pub fn dispatch_touchard_reachability_check(
    sigma_mod24: u32,
    active_mask: &[u64],
    start_idx: usize,
    components: &[crate::types::PrimePower],
) -> bool {
    dispatch_touchard_reachability_check_internal(
        sigma_mod24,
        active_mask,
        start_idx,
        components,
        true, // UALBF.Engine.TouchardBridge.reachability_soundness is machine-checked verified
    )
}

/// Internal helper for Touchard reachability dispatch supporting explicit verification flag testing.
pub fn dispatch_touchard_reachability_check_internal(
    sigma_mod24: u32,
    active_mask: &[u64],
    start_idx: usize,
    components: &[crate::types::PrimePower],
    is_verified: bool,
) -> bool {
    let mut reachable: u32 = 1 << (sigma_mod24 % 24);
    let mut block_idx = start_idx / 64;
    if block_idx < active_mask.len() {
        let mut block = active_mask[block_idx] & (!0u64 << (start_idx % 64));
        'outer: loop {
            while block != 0 {
                let tz = block.trailing_zeros();
                let j = block_idx * 64 + tz as usize;
                if j < components.len() {
                    let comp = &components[j];
                    let m = (comp.sigma % crate::types::Uint::from_u64(24)).as_u64() as usize;
                    if m != 1 {
                        let mut next_reachable = reachable;
                        for r in 0..24 {
                            if (reachable & (1 << r)) != 0 {
                                next_reachable |= 1 << ((r * m) % 24);
                            }
                        }
                        reachable = next_reachable;
                        if (reachable & ((1 << 3) | (1 << 19))) == ((1 << 3) | (1 << 19)) {
                            break 'outer;
                        }
                    }
                }
                block &= block - 1;
            }
            block_idx += 1;
            if block_idx >= active_mask.len() {
                break;
            }
            block = active_mask[block_idx];
        }
    }

    let is_unreachable = (reachable & ((1 << 3) | (1 << 19))) == 0;
    if !is_unreachable {
        return false;
    }

    // In pure proof mode, unproven reachability calculations must never execute hard branch elimination.
    if crate::policy::get_proof_mode() == "pure" && !is_verified {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starvation_under_128bit_bounds() {
        // Values that fit inside 128-bit bounds and trigger the fast path correctly.
        let s_l = Uint::from_u64(10);
        let n_l = Uint::from_u64(10);
        let best_remaining = 1u128 << 63; // ratio of 0.5
        let target_num = 2u64;
        let target_den = 1u64;

        // lhs ratio s_l/n_l * best_remaining/2^64 = 1 * 0.5 = 0.5
        // target ratio * 2 = 2 * 2 = 4
        // 0.5 < 4, so it should prune.
        let result = dispatch_starvation_check(s_l, n_l, best_remaining, target_num, target_den);
        assert!(result, "Should prune under 128-bit bounds");
    }

    #[test]
    fn test_starvation_overflow_fallback() {
        // s_l = 1 << 130, n_l = 1 << 130 (overflows u128)
        let s_l = Uint::one() << 130;
        let n_l = Uint::one() << 130;
        // best_remaining = 3 << 63
        let best_remaining = 3u128 << 63;
        let target_num = 2u64;
        let target_den = 1u64;

        // This should trigger the 512-bit fallback (since s_l and n_l overflow 128 bits)
        // and return true (prune) because lhs < rhs.
        let result = dispatch_starvation_check(s_l, n_l, best_remaining, target_num, target_den);
        assert!(result, "Should prune using 512-bit fallback");
    }

    #[test]
    fn test_starvation_overflow_fallback_no_prune() {
        // s_l = 1 << 130, n_l = 1 << 130
        let s_l = Uint::one() << 130;
        let n_l = Uint::one() << 130;
        // best_remaining = 5 << 63
        let best_remaining = 5u128 << 63;
        let target_num = 2u64;
        let target_den = 1u64;

        // s_l * best_remaining * target_den = 5 * (1 << 193)
        // (n_l * target_num) << 64 = 4 * (1 << 193)
        // lhs > rhs, so it should NOT prune.
        let result = dispatch_starvation_check(s_l, n_l, best_remaining, target_num, target_den);
        assert!(!result, "Should not prune using 512-bit fallback");
    }

    #[test]
    fn test_starvation_intermediate_overflow_fallback() {
        // s_l = 1 << 100, n_l = 1 << 100 (both fit in u128)
        let s_l = Uint::from_u128(1u128 << 100);
        let n_l = Uint::from_u128(1u128 << 100);
        // best_remaining = 1 << 63 (fits in u128)
        // best_num = (1 << 63) * 1
        // s_l * best_num = (1 << 100) * (1 << 63) = 1 << 163 (overflows u128!)
        let best_remaining = 1u128 << 63;
        let target_num = 2u64;
        let target_den = 1u64;

        // Intermediate multiplication overflows u128, so 128-bit fast path bounds check fails.
        // It must fallback directly to exact 512-bit check without mutating terms.
        // s_l * best_remaining * target_den = (1 << 100) * (1 << 63) * 1 = 1 << 163
        // n_l * target_num * (1 << 64) = (1 << 100) * 2 * (1 << 64) = 1 << 165
        // lhs < rhs (1 << 163 < 1 << 165), so it should prune.
        let result = dispatch_starvation_check(s_l, n_l, best_remaining, target_num, target_den);
        assert!(
            result,
            "Should prune using 512-bit fallback when intermediate product overflows u128"
        );
    }

    #[test]
    fn test_cdg_forced_overflow_fallback() {
        // s_l = 1 << 130, n_l = 1 << 130
        let s_l = Uint::one() << 130;
        let n_l = Uint::one() << 130;
        // forced_num = 3 << 64, forced_den = 1 << 64
        let forced_num = Uint::from_u128(3u128 << 64);
        let forced_den = Uint::from_u128(1u128 << 64);
        let target_num = 2u64;
        let target_den = 1u64;

        // lhs = s_l * forced_num * target_den = (1 << 130) * (3 << 64) * 1 = 3 * (1 << 194)
        // rhs = n_l * forced_den * target_num = (1 << 130) * (1 << 64) * 2 = 2 * (1 << 194)
        // lhs > rhs, so it should prune (returns true)
        let result =
            dispatch_cdg_forced_check(s_l, n_l, forced_num, forced_den, target_num, target_den);
        assert!(result, "Should prune in CDG forced fallback");
    }

    #[test]
    fn test_cdg_forced_overflow_fallback_no_prune() {
        // s_l = 1 << 130, n_l = 1 << 130
        let s_l = Uint::one() << 130;
        let n_l = Uint::one() << 130;
        // forced_num = 1 << 64, forced_den = 1 << 64
        let forced_num = Uint::from_u128(1u128 << 64);
        let forced_den = Uint::from_u128(1u128 << 64);
        let target_num = 2u64;
        let target_den = 1u64;

        // lhs = 1 * (1 << 194)
        // rhs = 2 * (1 << 194)
        // lhs < rhs, so it should NOT prune
        let result =
            dispatch_cdg_forced_check(s_l, n_l, forced_num, forced_den, target_num, target_den);
        assert!(!result, "Should not prune in CDG forced fallback");
    }

    #[test]
    fn test_cdg_forced_512bit_lhs_overflow() {
        let s_l = Uint::MAX;
        let n_l = Uint::from_u64(10);
        let forced_num = Uint::from_u64(2);
        let forced_den = Uint::from_u64(1);
        let target_num = 2u64;
        let target_den = 1u64;

        let result =
            dispatch_cdg_forced_check(s_l, n_l, forced_num, forced_den, target_num, target_den);
        assert!(!result, "Should return false on 512-bit lhs overflow");
    }

    #[test]
    fn test_cdg_forced_512bit_rhs_overflow() {
        let s_l = Uint::from_u64(10);
        let n_l = Uint::MAX;
        let forced_num = Uint::from_u64(2);
        let forced_den = Uint::from_u64(1);
        let target_num = 2u64;
        let target_den = 1u64;

        let result =
            dispatch_cdg_forced_check(s_l, n_l, forced_num, forced_den, target_num, target_den);
        assert!(!result, "Should return false on 512-bit rhs overflow");
    }

    #[test]
    fn test_cdg_forced_512bit_both_overflow() {
        let s_l = Uint::MAX;
        let n_l = Uint::MAX;
        let forced_num = Uint::from_u64(2);
        let forced_den = Uint::from_u64(1);
        let target_num = 2u64;
        let target_den = 1u64;

        let result =
            dispatch_cdg_forced_check(s_l, n_l, forced_num, forced_den, target_num, target_den);
        assert!(!result, "Should return false on 512-bit both overflow");
    }

    #[test]
    fn test_touchard_reachability_dispatch_reachable() {
        let active_mask = vec![1u64];
        let components = vec![];
        // sigma_mod24 = 3 is in {3, 19}, so reachable
        let result = dispatch_touchard_reachability_check(3, &active_mask, 0, &components);
        assert!(!result, "Reachable residue 3 must not be pruned");
    }

    #[test]
    fn test_touchard_reachability_dispatch_unreachable_pruned() {
        let active_mask = vec![1u64];
        // Component with sigma % 24 = 2.
        // Starting at sigma_mod24 = 2, multiplying by 2 yields 4, 8, 16, 8... None in {3, 19}.
        let comp = crate::types::PrimePower {
            p: 2,
            two_e: 2,
            val: Uint::from_u64(4),
            sigma: Uint::from_u64(2),
            sigma_factors: vec![],
            needs_rho: vec![],
            abundance_fp: 0,
        };
        let components = vec![comp];
        let result = dispatch_touchard_reachability_check(2, &active_mask, 0, &components);
        assert!(
            result,
            "Unreachable mod-24 state should be pruned when verified"
        );
    }

    #[test]
    fn test_touchard_reachability_dispatch_pure_unverified_fallback() {
        let active_mask = vec![1u64];
        let comp = crate::types::PrimePower {
            p: 2,
            two_e: 2,
            val: Uint::from_u64(4),
            sigma: Uint::from_u64(2),
            sigma_factors: vec![],
            needs_rho: vec![],
            abundance_fp: 0,
        };
        let components = vec![comp];

        let result_unverified =
            dispatch_touchard_reachability_check_internal(2, &active_mask, 0, &components, false);

        if crate::policy::get_proof_mode() == "pure" {
            assert!(
                !result_unverified,
                "Unverified reachability in pure proof mode must fall back (not prune)"
            );
        } else {
            assert!(
                result_unverified,
                "Axiomatic mode allows pruning even if unverified"
            );
        }
    }
}
