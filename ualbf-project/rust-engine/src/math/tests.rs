use super::*;
use crate::types::{Int, Uint};
use crate::types::{IntExt, UintExt};
use proptest::prelude::*;
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

#[test]
fn test_mul_mod_u128_overflow() {
    // Edge case: m = 1
    assert_eq!(mul_mod_u128(10, 20, 1), 0);

    // Edge case: zero operands
    assert_eq!(mul_mod_u128(0, 100, 17), 0);
    assert_eq!(mul_mod_u128(100, 0, 17), 0);

    // Fast-path checked multiplication (no overflow)
    assert_eq!(mul_mod_u128(10, 20, 7), (10 * 20) % 7);

    // 128-bit product overflow test:
    // a = 2^70, b = 2^70, m = 2^120 + 1.
    // a * b = 2^140, which exceeds u128::MAX (2^128 - 1).
    // (2^140) % (2^120 + 1):
    // 2^120 = -1 (mod 2^120 + 1)
    // 2^140 = 2^20 * (2^120) = -2^20 = m - 2^20 (mod m).
    let a = 1u128 << 70;
    let b = 1u128 << 70;
    let m = (1u128 << 120) + 1;
    let expected = m - (1u128 << 20);
    let res = mul_mod_u128(a, b, m);
    assert_eq!(res, expected);

    // Operands near u128::MAX
    let max_a = u128::MAX - 1;
    let max_b = u128::MAX - 2;
    let max_m = u128::MAX - 5;
    let res_max = mul_mod_u128(max_a, max_b, max_m);
    // (u128::MAX - 1) % (u128::MAX - 5) = 4
    // (u128::MAX - 2) % (u128::MAX - 5) = 3
    // (4 * 3) % (u128::MAX - 5) = 12
    assert_eq!(res_max, 12);
}

#[test]
fn test_mul_mod_u512_overflow() {
    // Construct 512-bit numbers that overflow when multiplied directly.
    // a = 2^300, b = 2^300, m = 2^500 + 1.
    let a = Uint::one() << 300;
    let b = Uint::one() << 300;
    let m = (Uint::one() << 500) + Uint::one();

    // a * b = 2^600, which overflows Uint (U512).
    // (2^600) % (2^500 + 1):
    // 2^500 = -1 (mod 2^500 + 1)
    // 2^600 = 2^100 * (2^500) = -2^100 = m - 2^100 (mod m).
    let expected = m - (Uint::one() << 100);
    let res = mul_mod_u512(a, b, m);
    assert_eq!(res, expected);
}

#[test]
fn test_solve_crt_512bit_overflow() {
    // Product of moduli exceeds 512 bits.
    let m1 = Int::from_u128(1) << 250;
    let m2 = Int::from_u128(1) << 250;
    let m3 = Int::from_u128(1) << 250;
    let r1 = Int::from_u32(1);
    let r2 = Int::from_u32(2);
    let r3 = Int::from_u32(3);

    // total_mod = 2^750 > 2^512, which overflows 512-bit integer capacity.
    assert_eq!(solve_crt(&[r1, r2, r3], &[m1, m2, m3]), None);
}

#[test]
fn test_solve_crt_wide_intermediate() {
    // Test 200-bit moduli where total_mod is ~400 bits.
    // Intermediate term1 * m_i can be ~600 bits, which overflows 512-bit arithmetic.
    // Intermediate 1024-bit arithmetic must prevent truncation.
    let m1 = (Int::one() << 200) - Int::from_u32(1);
    let m2 = (Int::one() << 200) - Int::from_u32(3);
    let r1 = Int::from_u128(987654321);
    let r2 = Int::from_u128(123456789);

    let res = solve_crt(&[r1, r2], &[m1, m2]).expect("CRT solution should exist");
    assert!(res >= Int::zero());
    assert_eq!(res % m1, r1 % m1);
    assert_eq!(res % m2, r2 % m2);
}

#[test]
fn test_solve_crt_negative_residues_and_normalization() {
    let m1 = Int::from_u32(7);
    let m2 = Int::from_u32(11);
    let r1 = Int::from_u32(3) - m1; // -4 (equiv to 3 mod 7)
    let r2 = Int::from_u32(5) - m2; // -6 (equiv to 5 mod 11)

    let res = solve_crt(&[r1, r2], &[m1, m2]).expect("CRT solution should exist");
    assert!(res >= Int::zero());
    assert!(res < m1 * m2);
    assert_eq!((res % m1 + m1) % m1, Int::from_u32(3));
    assert_eq!((res % m2 + m2) % m2, Int::from_u32(5));
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

#[test]
fn test_tonelli_shanks_large_two_adicity() {
    // p = 3 * 2^36 + 1 = 206,158,430,209 (two-adicity s = 36)
    let p1 = Int::from_u128(206158430209);
    let n1 = p1 - Int::one(); // -1 mod p, triggers m - i - 1 = 34
    let root1 = tonelli_shanks(n1, p1).expect("Square root of -1 mod p1 should exist");
    let sq1 = mul_mod_u256(root1.as_uint(), root1.as_uint(), p1.as_uint()).as_int();
    assert_eq!(sq1, n1);

    // p = 3 * 2^41 + 1 = 6,597,069,766,657 (two-adicity s = 41)
    let p2 = Int::from_u128(6597069766657);
    let n2 = p2 - Int::one(); // triggers m - i - 1 = 39
    let root2 = tonelli_shanks(n2, p2).expect("Square root of -1 mod p2 should exist");
    let sq2 = mul_mod_u256(root2.as_uint(), root2.as_uint(), p2.as_uint()).as_int();
    assert_eq!(sq2, n2);

    // Test a basic square n = 25
    let n3 = Int::from_u32(25);
    let root3 = tonelli_shanks(n3, p1).expect("Square root of 25 mod p1 should exist");
    let sq3 = mul_mod_u256(root3.as_uint(), root3.as_uint(), p1.as_uint()).as_int();
    assert_eq!(sq3, n3);

    // Test a non-quadratic residue (e.g. n = 11 mod p1)
    let n4 = Int::from_u32(11);
    assert_eq!(tonelli_shanks(n4, p1), None);
}

#[test]
fn test_try_as_conversions_uint() {
    let u_zero = Uint::zero();
    assert_eq!(u_zero.try_as_u128(), Some(0));
    assert_eq!(u_zero.try_as_u64(), Some(0));
    assert_eq!(u_zero.try_as_u32(), Some(0));
    assert_eq!(u_zero.try_as_usize(), Some(0));

    let u32_max = Uint::from_u32(u32::MAX);
    assert_eq!(u32_max.try_as_u32(), Some(u32::MAX));
    assert_eq!(u32_max.try_as_u64(), Some(u32::MAX as u64));
    assert_eq!(u32_max.try_as_u128(), Some(u32::MAX as u128));

    let u32_overflow = u32_max + Uint::one();
    assert_eq!(u32_overflow.try_as_u32(), None);
    assert_eq!(u32_overflow.try_as_u64(), Some((u32::MAX as u64) + 1));

    let u64_max = Uint::from_u64(u64::MAX);
    assert_eq!(u64_max.try_as_u64(), Some(u64::MAX));
    assert_eq!(u64_max.try_as_u32(), None);
    assert_eq!(u64_max.try_as_u128(), Some(u64::MAX as u128));

    let u64_overflow = u64_max + Uint::one();
    assert_eq!(u64_overflow.try_as_u64(), None);

    let u128_max = Uint::from_u128(u128::MAX);
    assert_eq!(u128_max.try_as_u128(), Some(u128::MAX));
    assert_eq!(u128_max.try_as_u64(), None);

    let u128_overflow = u128_max + Uint::one();
    assert_eq!(u128_overflow.try_as_u128(), None);
}

#[test]
fn test_try_as_conversions_int() {
    let i_zero = Int::zero();
    assert_eq!(i_zero.try_as_u128(), Some(0));
    assert_eq!(i_zero.try_as_u64(), Some(0));
    assert_eq!(i_zero.try_as_u32(), Some(0));
    assert_eq!(i_zero.try_as_usize(), Some(0));

    let i128_max = Int::from_u128(u128::MAX);
    assert_eq!(i128_max.try_as_u128(), Some(u128::MAX));

    let i128_overflow = i128_max + Int::one();
    assert_eq!(i128_overflow.try_as_u128(), None);
}

#[test]
fn test_rho_factor_u256_large_candidate() {
    // 1. Power of two exceeding 128 bits (2^128)
    let candidate_128 = Uint::one() << 128;
    let res_128 = rho_factor_u256(candidate_128);
    match res_128 {
        FactorizationResult::Complete(factors) => {
            assert_eq!(factors.len(), 128);
            let mut prod = Uint::one();
            for f in factors {
                prod *= f;
            }
            assert_eq!(prod, candidate_128);
        }
        _ => panic!("Expected complete factorization for 2^128"),
    }

    // 2. Odd integer exceeding 256 bits (2^300 + 1)
    let candidate_300 = (Uint::one() << 300) + Uint::one();
    let res_300 = rho_factor_u256(candidate_300);
    match res_300 {
        FactorizationResult::Partial { remaining, .. } => {
            assert_eq!(remaining, candidate_300);
        }
        _ => panic!("Expected partial factorization for 2^300 + 1 exceeding 256-bit limit"),
    }
}

#[test]
fn test_raw_limb_ffi_parity() {
    let a = Uint::from_u64(17);
    let m = Uint::from_u64(101);
    let inv = crate::lean_ffi::compute_mod_inverse(&a, false, &m);
    assert!(inv.is_some());
    let inv_val = inv.unwrap();
    assert_eq!((a * inv_val) % m, Uint::one());

    let a_neg = Uint::from_u64(17);
    let inv_neg = crate::lean_ffi::compute_mod_inverse(&a_neg, true, &m);
    assert!(inv_neg.is_some());
    let inv_neg_val = inv_neg.unwrap();
    let a_effective = m - a_neg;
    assert_eq!((a_effective * inv_neg_val) % m, Uint::one());

    let z = Uint::from_u64(12);
    let xl = Uint::from_u64(144);
    assert!(crate::lean_ffi::check_crt_1155(&z, &xl));
}

#[test]
fn test_mul_mod_fast_paths_comprehensive() {
    // 128-bit fast path
    let a_128 = Uint::from_u128(123456789012345678901234567890_u128);
    let b_128 = Uint::from_u128(987654321098765432109876543210_u128);
    let m_128 = Uint::from_u128(1000000000000000000000000000007_u128);
    let res_256 = mul_mod_u256(a_128, b_128, m_128);
    let res_512 = mul_mod_u512(a_128, b_128, m_128);
    let expected_128 = mul_mod_u128(a_128.as_u128(), b_128.as_u128(), m_128.as_u128());
    assert_eq!(res_256, Uint::from_u128(expected_128));
    assert_eq!(res_512, Uint::from_u128(expected_128));

    // 256-bit fast path (product exceeds 256 bits but fits in 512 bits)
    let a_256 = Uint::one() << 200;
    let b_256 = Uint::one() << 200;
    let m_256 = (Uint::one() << 250) + Uint::from_u64(1);
    let res_256_path = mul_mod_u256(a_256, b_256, m_256);
    // (2^200 * 2^200) % (2^250 + 1) = 2^400 % (2^250 + 1)
    // 2^250 = -1 (mod 2^250 + 1) => 2^400 = 2^150 * (2^250) = -2^150 = m_256 - 2^150
    let expected_256 = m_256 - (Uint::one() << 150);
    assert_eq!(res_256_path, expected_256);

    // Operands > 256 bits (uses U1024 fallback)
    let a_512 = Uint::one() << 300;
    let b_512 = Uint::one() << 300;
    let m_512 = (Uint::one() << 500) + Uint::from_u64(1);
    let res_512_path = mul_mod_u512(a_512, b_512, m_512);
    let expected_512 = m_512 - (Uint::one() << 100);
    assert_eq!(res_512_path, expected_512);

    // Edge case: m = 1
    assert_eq!(mul_mod_u256(a_256, b_256, Uint::one()), Uint::zero());
    assert_eq!(mul_mod_u512(a_256, b_256, Uint::one()), Uint::zero());
}

#[test]
fn test_trial_sieve_spf_and_primitive_division() {
    let limit = 10_000_000u64;
    let trial_sieve = TrialSieve::new(limit);

    // Verify SPF array size and memory limit (< 40 MB)
    assert_eq!(trial_sieve.spf.len(), 10_000_001);
    let memory_bytes = trial_sieve.spf.len() * std::mem::size_of::<u32>();
    assert!(
        memory_bytes <= 40 * 1024 * 1024,
        "SPF array memory {} bytes exceeds 40 MB!",
        memory_bytes
    );

    // Test O(1) factor retrieval for small inputs
    let facs_12 = trial_sieve.get_spf_factors(12).unwrap();
    assert_eq!(facs_12, vec![2, 2, 3]);

    let facs_1k = trial_sieve.get_spf_factors(1000).unwrap();
    assert_eq!(facs_1k, vec![2, 2, 2, 5, 5, 5]);

    let facs_million = trial_sieve.get_spf_factors(1_000_000).unwrap();
    assert_eq!(facs_million, vec![2, 2, 2, 2, 2, 2, 5, 5, 5, 5, 5, 5]);

    let facs_prime = trial_sieve.get_spf_factors(9_999_991).unwrap();
    assert_eq!(facs_prime, vec![9_999_991]);

    // Test TrialSieve::trial_factor_only with SPF fast path
    let (uint_facs_12, rem_12) = trial_sieve.trial_factor_only(Uint::from_u64(12));
    assert_eq!(rem_12, Uint::one());
    let u64_facs_12: Vec<u64> = uint_facs_12.iter().map(|f| f.as_u64()).collect();
    assert_eq!(u64_facs_12, vec![2, 2, 3]);

    // Test trial division for u64 inputs > 10^7
    let big_composite = Uint::from_u64(10_000_019 * 2);
    let (uint_facs_bc, rem_bc) = trial_sieve.trial_factor_only(big_composite);
    assert_eq!(rem_bc, Uint::one());
    let u64_facs_bc: Vec<u64> = uint_facs_bc.iter().map(|f| f.as_u64()).collect();
    assert_eq!(u64_facs_bc, vec![2, 10_000_019]);

    // Test 512-bit input that reduces to 64-bit during trial division
    let uint_512_val = (Uint::one() << 100) * Uint::from_u64(12); // divisible by 2^102 * 3
    let (facs_512, rem_512) = trial_sieve.trial_factor_only(uint_512_val);
    assert!(facs_512.len() >= 2);
    let prod: Uint = facs_512.iter().copied().fold(Uint::one(), |acc, x| acc * x) * rem_512;
    assert_eq!(prod, uint_512_val);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn prop_mul_mod_u128(
        a in any::<u128>(),
        b in any::<u128>(),
        m in 1u128..=u128::MAX,
    ) {
        let res = mul_mod_u128(a, b, m);
        let expected = if m == 1 {
            0
        } else {
            let a_big = num_bigint::BigUint::from(a);
            let b_big = num_bigint::BigUint::from(b);
            let m_big = num_bigint::BigUint::from(m);
            u128::try_from((a_big * b_big) % m_big).unwrap()
        };
        prop_assert_eq!(res, expected);
        prop_assert_eq!(mul_mod_u128(a, b, m), mul_mod_u128(b, a, m));
    }

    #[test]
    fn prop_modpow_u256(
        base_u in any::<u128>(),
        exp_u in 0u64..10000u64,
        m_u in 1u128..=u128::MAX,
    ) {
        let base = Uint::from_u128(base_u);
        let exp = Uint::from_u64(exp_u);
        let modulus = Uint::from_u128(m_u);
        let res = modpow_u256(base, exp, modulus);
        if modulus <= Uint::one() {
            prop_assert_eq!(res, Uint::zero());
        } else {
            let base_big = num_bigint::BigUint::from(base_u);
            let exp_big = num_bigint::BigUint::from(exp_u);
            let m_big = num_bigint::BigUint::from(m_u);
            let expected_big = base_big.modpow(&exp_big, &m_big);
            let expected_u128 = u128::try_from(expected_big).unwrap();
            prop_assert_eq!(res, Uint::from_u128(expected_u128));
        }
    }

    #[test]
    fn prop_mod_inverse_u512_identity(
        a_u in 1u128..=u128::MAX,
        m_u in 2u128..=u128::MAX,
    ) {
        let a = Uint::from_u128(a_u);
        let m = Uint::from_u128(m_u);
        let g = gcd_u256(a, m);
        let inv_opt = mod_inverse_u512(a, m);
        if g == Uint::one() {
            let inv = inv_opt.expect("Inverse must exist for coprime a and m");
            prop_assert!(inv < m, "a_inv must be less than m");
            let prod_mod = mul_mod_u512(a, inv, m);
            prop_assert_eq!(prod_mod, Uint::one(), "Modular inverse identity (a * a_inv) % m == 1 failed");
        } else {
            prop_assert_eq!(inv_opt, None, "Inverse should not exist when gcd(a, m) > 1");
        }
    }

    #[test]
    fn prop_tonelli_shanks_quadratic_residue(
        p_idx in 0usize..1000usize,
        x_raw in any::<u64>(),
    ) {
        let primes = [
            3u64, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73,
            79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151,
            10007, 10009, 1000003, 1000033, 1000000007, 206158430209
        ];
        let p_val = primes[p_idx % primes.len()];
        let p = Int::from_u64(p_val);
        let x = Int::from_u64(x_raw % p_val);
        let n = mul_mod_u256(x.as_uint(), x.as_uint(), p.as_uint()).as_int();

        if let Some(r) = tonelli_shanks(n, p) {
            let sq = mul_mod_u256(r.as_uint(), r.as_uint(), p.as_uint()).as_int();
            prop_assert_eq!(sq, n, "Tonelli-Shanks output r^2 % p must equal n % p");
        } else {
            prop_assert!(false, "Tonelli-Shanks failed for valid quadratic residue n = x^2 mod p");
        }
    }

    #[test]
    fn prop_sigma_cached_multiplicativity(
        p_idx1 in 0usize..500usize,
        p_idx2 in 0usize..500usize,
        pow1 in 1u32..=5u32,
        pow2 in 1u32..=5u32,
    ) {
        let primes = [
            2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71,
            73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151
        ];
        let p1_val = primes[p_idx1 % primes.len()];
        let p2_val = primes[p_idx2 % primes.len()];
        if p1_val != p2_val {
            let cache = HashMap::new();
            let sig1 = sigma_cached(&cache, Uint::from_u64(p1_val), pow1);
            let sig2 = sigma_cached(&cache, Uint::from_u64(p2_val), pow2);

            let combined_sigma = sig1 * sig2;

            let mut sum = Uint::zero();
            let mut p1_p = Uint::one();
            let p1_u = Uint::from_u64(p1_val);
            let p2_u = Uint::from_u64(p2_val);

            for _ in 0..=pow1 {
                let mut p2_p = Uint::one();
                for _ in 0..=pow2 {
                    sum += p1_p * p2_p;
                    p2_p *= p2_u;
                }
                p1_p *= p1_u;
            }

            prop_assert_eq!(combined_sigma, sum, "sigma_cached multiplicativity failed for coprime inputs");
        }
    }
}
