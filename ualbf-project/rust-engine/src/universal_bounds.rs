use ualbf_macros::universal_pruning_bounds;

universal_pruning_bounds!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Uint;

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
}
