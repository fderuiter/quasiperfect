use super::*;
use crate::types::{Int, Uint};
use crate::types::{IntExt, UintExt};
use std::collections::HashMap;

#[test]
fn test_is_valid_mod_8() {
    // 1 and 3 mod 8 are valid
    assert!(1u64.is_valid_mod_8());
    assert!(3u64.is_valid_mod_8());
    assert!(9u64.is_valid_mod_8());
    assert!(11u64.is_valid_mod_8());

    // 0, 2, 4, 5, 6, 7 mod 8 are invalid
    assert!(!0u64.is_valid_mod_8());
    assert!(!2u64.is_valid_mod_8());
    assert!(!4u64.is_valid_mod_8());
    assert!(!5u64.is_valid_mod_8());
    assert!(!6u64.is_valid_mod_8());
    assert!(!7u64.is_valid_mod_8());

    // Test u32, u128, and Uint impls
    assert!(3u32.is_valid_mod_8());
    assert!(!5u32.is_valid_mod_8());
    assert!(11u128.is_valid_mod_8());
    assert!(!13u128.is_valid_mod_8());

    let uint_1 = Uint::from_u64(1);
    let uint_3 = Uint::from_u64(3);
    let uint_5 = Uint::from_u64(5);
    assert!(uint_1.is_valid_mod_8());
    assert!(uint_3.is_valid_mod_8());
    assert!(!uint_5.is_valid_mod_8());
}

#[test]
fn test_cyclotomic_integration() {
    assert_eq!(small_divisors_pub(0), Vec::<u32>::new());
    assert_eq!(small_divisors_pub(1), vec![1]);
    assert_eq!(small_divisors_pub(12), vec![1, 2, 3, 4, 6, 12]);
    assert_eq!(small_divisors_pub(13), vec![1, 13]);

    match factor_sigma_cyclotomic(3, 2) {
        FactorizationResult::Complete(facs) => {
            assert_eq!(facs.to_vec(), vec![Uint::from_u64(13)]);
        }
        _ => panic!("Expected complete factorization"),
    }

    match factor_sigma_cyclotomic(2, 4) {
        FactorizationResult::Complete(facs) => {
            assert_eq!(facs.to_vec(), vec![Uint::from_u64(31)]);
        }
        _ => panic!("Expected complete factorization"),
    }

    match factor_sigma_cyclotomic(3, 4) {
        FactorizationResult::Complete(facs) => {
            assert_eq!(facs.to_vec(), vec![Uint::from_u64(11), Uint::from_u64(11)]);
        }
        _ => panic!("Expected complete factorization"),
    }

    match factor_sigma_cyclotomic(2, 5) {
        FactorizationResult::Complete(facs) => {
            assert_eq!(
                facs.to_vec(),
                vec![Uint::from_u64(3), Uint::from_u64(3), Uint::from_u64(7)]
            );
        }
        _ => panic!("Expected complete factorization"),
    }

    match factor_sigma_cyclotomic(5, 0) {
        FactorizationResult::Complete(facs) => {
            assert_eq!(facs.to_vec(), Vec::<Uint>::new());
        }
        _ => panic!("Expected complete factorization"),
    }
}

#[test]
fn test_mod_negate_big() {
    let m = Int::from_u32(10);
    assert_eq!(mod_negate_big(Int::from_u32(3), m), Int::from_u32(7));
    assert_eq!(mod_negate_big(Int::from_u32(0), m), Int::from_u32(0));
    assert_eq!(mod_negate_big(Int::from_u32(10), m), Int::from_u32(0));
    assert_eq!(mod_negate_big(Int::from_u32(13), m), Int::from_u32(7));
}

#[test]
fn test_solve_mod_2_k_custom() {
    let n = Int::from_u32(1);
    let roots = solve_mod_2_k(n, 3);
    assert_eq!(roots.len(), 4);
}

#[test]
fn test_verified_is_prime_edge_cases() {
    assert_eq!(verified_is_prime(Uint::zero()), false);
    assert_eq!(verified_is_prime(Uint::one()), false);
    assert_eq!(verified_is_prime(Uint::from_u32(2)), true);
    assert_eq!(verified_is_prime(Uint::from_u32(3)), true);
    assert_eq!(verified_is_prime(Uint::from_u32(4)), false);
    assert_eq!(verified_is_prime(Uint::from_u32(5)), true);
    assert_eq!(verified_is_prime(Uint::from_u32(9)), false);
    assert_eq!(verified_is_prime(Uint::from_u32(71)), true);
    assert_eq!(verified_is_prime(Uint::from_u32(72)), false);

    assert_eq!(verified_is_prime(Uint::from_u128(1_000_000_000_039)), true);
    assert_eq!(
        verified_is_prime(Uint::from_u128(1_000_000_000_039 * 5)),
        false
    );

    let prime_under_2_64 = Uint::from_u128(18446744073709551557_u128);
    assert_eq!(verified_is_prime(prime_under_2_64), true);

    let composite_under_2_64 = Uint::from_u128(18446744073709551558_u128);
    assert_eq!(verified_is_prime(composite_under_2_64), false);

    let prime_over_2_64 = Uint::from_u128(18446744073709551629_u128);
    assert_eq!(verified_is_prime(prime_over_2_64), true);

    let composite_over_2_64 = Uint::from_u128(18446744073709551617_u128);
    assert_eq!(verified_is_prime(composite_over_2_64), false);
}

#[test]
fn test_pocklington() {
    let p = Uint::from_str_radix("1000000000039", 10).unwrap();
    assert!(generate_and_verify_pocklington(p));

    let composite = Uint::from_str_radix("1000000000037", 10).unwrap();
    assert!(!generate_and_verify_pocklington(composite));
}

#[test]
fn test_bls_fallback() {
    let n_str = "2379738973818137587966091241708611381752917711408835932379608374701813936555867387813613052614017647793415905146965240226643968001";
    let n = Uint::from_str_radix(n_str, 10).unwrap();
    assert!(generate_and_verify_pocklington(n));
    assert!(verified_is_prime(n));
}

#[test]
fn test_sigma_cached_large_prime() {
    let prime_over_2_64 = Uint::from_u128(18446744073709551629_u128);
    let cache = HashMap::new();
    let expected = prime_over_2_64
        .checked_mul(prime_over_2_64)
        .unwrap()
        .checked_add(prime_over_2_64)
        .unwrap()
        .checked_add(Uint::one())
        .unwrap();
    let computed = sigma_cached(&cache, prime_over_2_64, 2);
    assert_eq!(computed, expected);
}

#[test]
fn test_solve_mod_2_k_custom_5() {
    let n = Int::from_u32(17);
    let roots = solve_mod_2_k(n, 5);
    assert_eq!(roots.len(), 4);
}

#[cfg_attr(unverified_build, ignore)]
#[test]
fn test_solve_crt_128bit() {
    crate::lean_ffi::initialize_lean_runtime();
    let m1 = Int::from_u128(0xFFFFFFFFFFFFFFFF);
    let m2 = Int::from_u128(0xFFFFFFFFFFFFFFFE);
    let r1 = Int::from_u128(12345);
    let r2 = Int::from_u128(67890);
    let res = solve_crt(&[r1, r2], &[m1, m2]).expect("CRT should find a solution");
    assert_eq!(res % m1, r1);
    assert_eq!(res % m2, r2);
}

#[cfg_attr(unverified_build, ignore)]
#[test]
fn test_hensels_lift_basic() {
    crate::lean_ffi::initialize_lean_runtime();
    let root = Int::from_u128(3);
    let n = Int::from_u128(2);
    let p = Int::from_u128(7);
    let k = 2;
    let lifted = hensels_lift(root, n, p, k).unwrap();
    assert_eq!(lifted, Int::from_u128(10));
}

#[cfg_attr(unverified_build, ignore)]
#[test]
fn test_hensels_lift_k3() {
    crate::lean_ffi::initialize_lean_runtime();
    let root = Int::from_u128(3);
    let n = Int::from_u128(2);
    let p = Int::from_u128(7);
    let k = 3;
    let lifted = hensels_lift(root, n, p, k).unwrap();
    assert_eq!(lifted, Int::from_u128(108));
}

#[test]
fn test_quick_factor_u256_large_composite_remainder() {
    // 10007 and 10009 are primes > 10,000.
    // Their product 100160063 is > 10^8 (100,000,000).
    // Trial division in quick_factor_u256 stops at d = 10000,
    // leaving remaining = 100160063 >= 10^8.
    // quick_factor_u256 must invoke Pollard's rho / ECM fallback to completely factor it.
    let composite = Uint::from_u128(100160063);
    match quick_factor_u256(composite) {
        FactorizationResult::Complete(factors) => {
            assert_eq!(
                factors.to_vec(),
                vec![Uint::from_u128(10007), Uint::from_u128(10009)]
            );
        }
        res => panic!(
            "Expected complete factorization for 100160063, got {:?}",
            res
        ),
    }
}

#[cfg_attr(unverified_build, ignore)]
#[test]
fn test_hensels_lift_residue_failure() {
    crate::lean_ffi::initialize_lean_runtime();
    let root = Int::from_u128(1);
    let n = Int::from_u128(1);
    let p = Int::from_u128(2);
    let k = 3;
    assert_eq!(hensels_lift(root, n, p, k), None);
}
