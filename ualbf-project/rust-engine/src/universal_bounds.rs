use ualbf_macros::universal_pruning_bounds;

universal_pruning_bounds!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Uint;
    use proptest::prelude::*;

    #[test]
    fn test_cpu_check_euler_ceiling_below() {
        // num / den = 20000 / 10000 = 2.0 < 20442 / 10000 = 2.0442
        // Below ceiling condition -> returns true
        let num = Uint::from_u64(20000);
        let den = Uint::from_u64(10000);
        let euler_num = Uint::from_u64(20442);
        let euler_den = Uint::from_u64(10000);

        assert!(cpu_check_euler_ceiling(&num, &den, &euler_num, &euler_den));
    }

    #[test]
    fn test_cpu_check_euler_ceiling_exact() {
        // num / den = 20442 / 10000 == 20442 / 10000
        // Exact ceiling condition -> returns false
        let num = Uint::from_u64(20442);
        let den = Uint::from_u64(10000);
        let euler_num = Uint::from_u64(20442);
        let euler_den = Uint::from_u64(10000);

        assert!(!cpu_check_euler_ceiling(&num, &den, &euler_num, &euler_den));
    }

    #[test]
    fn test_cpu_check_euler_ceiling_above() {
        // num / den = 21000 / 10000 = 2.1 > 20442 / 10000 = 2.0442
        // Above ceiling condition -> returns false
        let num = Uint::from_u64(21000);
        let den = Uint::from_u64(10000);
        let euler_num = Uint::from_u64(20442);
        let euler_den = Uint::from_u64(10000);

        assert!(!cpu_check_euler_ceiling(&num, &den, &euler_num, &euler_den));
    }

    #[test]
    fn test_cpu_check_euler_ceiling_lhs_overflow() {
        // num * euler_den overflows 512-bit Uint -> returns true without panicking
        let num = Uint::MAX;
        let den = Uint::from_u64(1);
        let euler_num = Uint::from_u64(1);
        let euler_den = Uint::from_u64(2);

        assert!(cpu_check_euler_ceiling(&num, &den, &euler_num, &euler_den));
    }

    #[test]
    fn test_cpu_check_euler_ceiling_rhs_overflow() {
        // den * euler_num overflows 512-bit Uint -> returns true without panicking
        let num = Uint::from_u64(1);
        let den = Uint::MAX;
        let euler_num = Uint::from_u64(2);
        let euler_den = Uint::from_u64(1);

        assert!(cpu_check_euler_ceiling(&num, &den, &euler_num, &euler_den));
    }

    #[test]
    fn test_cpu_check_euler_ceiling_both_overflow() {
        // both lhs and rhs overflow 512-bit Uint -> returns true without panicking
        let num = Uint::MAX;
        let den = Uint::MAX;
        let euler_num = Uint::MAX;
        let euler_den = Uint::MAX;

        assert!(cpu_check_euler_ceiling(&num, &den, &euler_num, &euler_den));
    }

    proptest! {
        #[test]
        fn test_cpu_check_euler_ceiling_property(
            num_raw in 1u64..=1_000_000_000u64,
            den_raw in 1u64..=1_000_000_000u64,
            euler_num_raw in 1u64..=1_000_000_000u64,
            euler_den_raw in 1u64..=1_000_000_000u64,
        ) {
            let num = Uint::from_u64(num_raw);
            let den = Uint::from_u64(den_raw);
            let euler_num = Uint::from_u64(euler_num_raw);
            let euler_den = Uint::from_u64(euler_den_raw);

            let result = cpu_check_euler_ceiling(&num, &den, &euler_num, &euler_den);
            let lhs = num_raw as u128 * euler_den_raw as u128;
            let rhs = den_raw as u128 * euler_num_raw as u128;
            let expected = lhs < rhs;

            prop_assert_eq!(result, expected);

            if lhs >= rhs {
                prop_assert!(!result, "False positive pruning when ratio >= ceiling");
            } else {
                prop_assert!(result, "Expected pruning decision when ratio < ceiling");
            }
        }
    }

    #[test]
    fn test_cpu_check_dusart_bound_below_threshold() {
        // Since p_last < validity_threshold, it must not prune (returns false)
        let s_l = Uint::from_u64(2);
        let n_l = Uint::from_u64(1);
        let p_last = 2000;
        let validity_threshold = 2973;
        let dusart_num = 1;
        let dusart_den = 5;
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_dusart_bound(
            &s_l,
            &n_l,
            p_last,
            validity_threshold,
            dusart_num,
            dusart_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_dusart_bound_prunes_correctly() {
        // Above threshold. Let's make upper bound strictly less than target.
        // If s_l = 19, n_l = 10, then s_l/n_l = 1.9.
        // Since p_last = 3000, validity_threshold = 2973.
        // dusart_num = 1, dusart_den = 5.
        // den_p_last = 5 * 3000 = 15000.
        // factor_num = 15001, factor_den = 15000.
        // upper_bound = 1.9 * (15001/15000) = 1.900127.
        // If target_num/target_den = 2 (which is 2.0).
        // Since 1.900127 < 2.0, this branch cannot reach the target, so it must prune (returns true).
        let s_l = Uint::from_u64(19);
        let n_l = Uint::from_u64(10);
        let p_last = 3000;
        let validity_threshold = 2973;
        let dusart_num = 1;
        let dusart_den = 5;
        let target_num = 2;
        let target_den = 1;

        assert!(cpu_check_dusart_bound(
            &s_l,
            &n_l,
            p_last,
            validity_threshold,
            dusart_num,
            dusart_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_dusart_bound_does_not_prune() {
        // Above threshold. Let's make upper bound greater than or equal to target.
        // s_l = 21, n_l = 10 (ratio 2.1).
        // Since 2.1 * (15001/15000) = 2.10014 > 2.0, it can meet/exceed target, so it must not prune (returns false).
        let s_l = Uint::from_u64(21);
        let n_l = Uint::from_u64(10);
        let p_last = 3000;
        let validity_threshold = 2973;
        let dusart_num = 1;
        let dusart_den = 5;
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_dusart_bound(
            &s_l,
            &n_l,
            p_last,
            validity_threshold,
            dusart_num,
            dusart_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_dusart_bound_rhs_overflow() {
        // When rhs multiplication overflows 512 bits, it must return false
        // to avoid unearned pruning (false positive pruning).
        let s_l = Uint::from_u64(10);
        let n_l = Uint::MAX;
        let p_last = 3000;
        let validity_threshold = 2973;
        let dusart_num = 1;
        let dusart_den = 5;
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_dusart_bound(
            &s_l,
            &n_l,
            p_last,
            validity_threshold,
            dusart_num,
            dusart_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_dusart_bound_lhs_overflow() {
        // When lhs multiplication overflows 512 bits, it must return false.
        let s_l = Uint::MAX;
        let n_l = Uint::from_u64(10);
        let p_last = 3000;
        let validity_threshold = 2973;
        let dusart_num = 1;
        let dusart_den = 5;
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_dusart_bound(
            &s_l,
            &n_l,
            p_last,
            validity_threshold,
            dusart_num,
            dusart_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_dusart_bound_both_overflow() {
        // When both lhs and rhs overflow 512 bits, it must return false.
        let s_l = Uint::MAX;
        let n_l = Uint::MAX;
        let p_last = 3000;
        let validity_threshold = 2973;
        let dusart_num = 1;
        let dusart_den = 5;
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_dusart_bound(
            &s_l,
            &n_l,
            p_last,
            validity_threshold,
            dusart_num,
            dusart_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_cdg_forced_lhs_overflow() {
        // When lhs multiplication overflows 512 bits, it must return false
        // to prevent unearned pruning signals.
        let s_l = Uint::MAX;
        let n_l = Uint::from_u64(10);
        let forced_num = Uint::from_u64(2);
        let forced_den = Uint::from_u64(1);
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_cdg_forced(
            &s_l,
            &n_l,
            &forced_num,
            &forced_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_cdg_forced_rhs_overflow() {
        // When rhs multiplication overflows 512 bits, it must return false.
        let s_l = Uint::from_u64(10);
        let n_l = Uint::MAX;
        let forced_num = Uint::from_u64(2);
        let forced_den = Uint::from_u64(1);
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_cdg_forced(
            &s_l,
            &n_l,
            &forced_num,
            &forced_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_cdg_forced_both_overflow() {
        // When both lhs and rhs overflow 512 bits, it must return false.
        let s_l = Uint::MAX;
        let n_l = Uint::MAX;
        let forced_num = Uint::from_u64(2);
        let forced_den = Uint::from_u64(1);
        let target_num = 2;
        let target_den = 1;

        assert!(!cpu_check_cdg_forced(
            &s_l,
            &n_l,
            &forced_num,
            &forced_den,
            target_num,
            target_den
        ));
    }

    #[test]
    fn test_cpu_check_abundancy_overflow_valid_prune() {
        // s_l = 21, n_l = 10, target = 2 / 1 -> 21 * 1 > 10 * 2 -> 21 > 20 (true)
        let s_l = Uint::from_u64(21);
        let n_l = Uint::from_u64(10);
        assert!(cpu_check_abundancy_overflow(&s_l, &n_l, 2, 1));
    }

    #[test]
    fn test_cpu_check_abundancy_overflow_valid_no_prune() {
        // s_l = 19, n_l = 10, target = 2 / 1 -> 19 * 1 > 10 * 2 -> 19 > 20 (false)
        let s_l = Uint::from_u64(19);
        let n_l = Uint::from_u64(10);
        assert!(!cpu_check_abundancy_overflow(&s_l, &n_l, 2, 1));
    }

    #[test]
    fn test_cpu_check_abundancy_overflow_lhs_overflow() {
        // s_l = Uint::MAX -> s_l * target_den overflows 512 bits -> returns false
        let s_l = Uint::MAX;
        let n_l = Uint::from_u64(10);
        assert!(!cpu_check_abundancy_overflow(&s_l, &n_l, 1, 2));
    }

    #[test]
    fn test_cpu_check_abundancy_overflow_rhs_overflow() {
        // n_l = Uint::MAX -> n_l * target_num overflows 512 bits -> returns false
        let s_l = Uint::from_u64(10);
        let n_l = Uint::MAX;
        assert!(!cpu_check_abundancy_overflow(&s_l, &n_l, 2, 1));
    }

    #[test]
    fn test_cpu_check_abundancy_overflow_both_overflow() {
        // Both s_l * target_den and n_l * target_num overflow 512 bits -> returns false
        let s_l = Uint::MAX;
        let n_l = Uint::MAX;
        assert!(!cpu_check_abundancy_overflow(&s_l, &n_l, 2, 2));
    }
}
