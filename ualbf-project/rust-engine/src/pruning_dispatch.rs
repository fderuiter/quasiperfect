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

            if let (Some(mut best_num), Some(mut best_den)) = (best_num_opt, best_den_opt) {
                while s_l_128.checked_mul(best_num).is_none()
                    || n_l_128
                        .checked_mul(best_den)
                        .and_then(|x| x.checked_mul(2))
                        .is_none()
                {
                    best_num = (best_num >> 1) + 1;
                    best_den >>= 1;
                    if best_den == 0 {
                        break;
                    }
                }

                if s_l_128 > 0 && n_l_128 > 0 && best_num > 0 && best_den > 0 {
                    if s_l_128 <= u128::MAX / best_num
                        && n_l_128 <= u128::MAX / 2
                        && (n_l_128 * 2) <= u128::MAX / best_den
                    {
                        let pruned = crate::verus_proofs::check_starvation_kill(
                            s_l_128, n_l_128, best_num, best_den,
                        );
                        if pruned {
                            return true;
                        }
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
}
