pub use crate::math::residue::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Uint, UintExt};

    #[test]
    fn test_is_valid_mod_8_u64() {
        // Congruent to 1 mod 8
        assert!(1u64.is_valid_mod_8());
        assert!(9u64.is_valid_mod_8());
        assert!(17u64.is_valid_mod_8());
        assert!(25u64.is_valid_mod_8());
        assert!(1001u64.is_valid_mod_8());

        // Congruent to 3 mod 8
        assert!(3u64.is_valid_mod_8());
        assert!(11u64.is_valid_mod_8());
        assert!(19u64.is_valid_mod_8());
        assert!(27u64.is_valid_mod_8());
        assert!(1003u64.is_valid_mod_8());

        // Non-congruent (0, 2, 4, 5, 6, 7 mod 8)
        assert!(!0u64.is_valid_mod_8());
        assert!(!2u64.is_valid_mod_8());
        assert!(!4u64.is_valid_mod_8());
        assert!(!5u64.is_valid_mod_8());
        assert!(!6u64.is_valid_mod_8());
        assert!(!7u64.is_valid_mod_8());
        assert!(!8u64.is_valid_mod_8());

        // Boundary u64 values
        assert!(!u64::MAX.is_valid_mod_8()); // u64::MAX % 8 == 7
        assert!((u64::MAX - 6).is_valid_mod_8()); // % 8 == 1
        assert!((u64::MAX - 4).is_valid_mod_8()); // % 8 == 3
    }

    #[test]
    fn test_is_valid_mod_8_u32() {
        // Congruent to 1 mod 8
        assert!(1u32.is_valid_mod_8());
        assert!(9u32.is_valid_mod_8());
        assert!(17u32.is_valid_mod_8());

        // Congruent to 3 mod 8
        assert!(3u32.is_valid_mod_8());
        assert!(11u32.is_valid_mod_8());
        assert!(19u32.is_valid_mod_8());

        // Non-congruent
        assert!(!0u32.is_valid_mod_8());
        assert!(!2u32.is_valid_mod_8());
        assert!(!4u32.is_valid_mod_8());
        assert!(!5u32.is_valid_mod_8());
        assert!(!6u32.is_valid_mod_8());
        assert!(!7u32.is_valid_mod_8());

        // Boundary u32 values
        assert!(!u32::MAX.is_valid_mod_8()); // % 8 == 7
        assert!((u32::MAX - 6).is_valid_mod_8()); // % 8 == 1
        assert!((u32::MAX - 4).is_valid_mod_8()); // % 8 == 3
    }

    #[test]
    fn test_is_valid_mod_8_u128() {
        // Congruent to 1 mod 8
        assert!(1u128.is_valid_mod_8());
        assert!(9u128.is_valid_mod_8());
        assert!(17u128.is_valid_mod_8());

        // Congruent to 3 mod 8
        assert!(3u128.is_valid_mod_8());
        assert!(11u128.is_valid_mod_8());
        assert!(19u128.is_valid_mod_8());

        // Non-congruent
        assert!(!0u128.is_valid_mod_8());
        assert!(!2u128.is_valid_mod_8());
        assert!(!4u128.is_valid_mod_8());
        assert!(!5u128.is_valid_mod_8());
        assert!(!6u128.is_valid_mod_8());
        assert!(!7u128.is_valid_mod_8());

        // Values exceeding u64::MAX
        let max_u64_128 = u64::MAX as u128;
        let c1_above = max_u64_128 + 2; // (2^64 - 1) + 2 = 2^64 + 1 -> % 8 == 1
        let c3_above = max_u64_128 + 4; // 2^64 + 3 -> % 8 == 3
        let c0_above = max_u64_128 + 1; // 2^64 -> % 8 == 0
        let c2_above = max_u64_128 + 3; // 2^64 + 2 -> % 8 == 2
        let c5_above = max_u64_128 + 6; // 2^64 + 5 -> % 8 == 5

        assert!(c1_above.is_valid_mod_8());
        assert!(c3_above.is_valid_mod_8());
        assert!(!c0_above.is_valid_mod_8());
        assert!(!c2_above.is_valid_mod_8());
        assert!(!c5_above.is_valid_mod_8());

        assert!(!u128::MAX.is_valid_mod_8()); // u128::MAX % 8 == 7
    }

    #[test]
    fn test_is_valid_mod_8_uint() {
        // Congruent to 1 mod 8
        assert!(Uint::from_u64(1).is_valid_mod_8());
        assert!(Uint::from_u64(9).is_valid_mod_8());
        assert!(Uint::from_u64(17).is_valid_mod_8());

        // Congruent to 3 mod 8
        assert!(Uint::from_u64(3).is_valid_mod_8());
        assert!(Uint::from_u64(11).is_valid_mod_8());
        assert!(Uint::from_u64(19).is_valid_mod_8());

        // Non-congruent
        assert!(!Uint::from_u64(0).is_valid_mod_8());
        assert!(!Uint::from_u64(2).is_valid_mod_8());
        assert!(!Uint::from_u64(4).is_valid_mod_8());
        assert!(!Uint::from_u64(5).is_valid_mod_8());
        assert!(!Uint::from_u64(6).is_valid_mod_8());
        assert!(!Uint::from_u64(7).is_valid_mod_8());

        // Values exceeding u64::MAX
        let max_u64_uint = Uint::from_u64(u64::MAX);
        let c1_above = max_u64_uint + Uint::from_u64(2); // % 8 == 1
        let c3_above = max_u64_uint + Uint::from_u64(4); // % 8 == 3
        let c0_above = max_u64_uint + Uint::from_u64(1); // % 8 == 0
        let c2_above = max_u64_uint + Uint::from_u64(3); // % 8 == 2
        let c5_above = max_u64_uint + Uint::from_u64(6); // % 8 == 5

        assert!(c1_above.is_valid_mod_8());
        assert!(c3_above.is_valid_mod_8());
        assert!(!c0_above.is_valid_mod_8());
        assert!(!c2_above.is_valid_mod_8());
        assert!(!c5_above.is_valid_mod_8());
    }
}
