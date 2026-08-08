#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::manual_abs_diff)]

use crate::types::{Int, Uint};
use crate::types::{IntExt, UintExt};
use prime_factorization::Factorization;
use std::collections::HashMap;
use std::panic::catch_unwind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactorizationResult {
    Complete(smallvec::SmallVec<[Uint; 8]>),
    Partial {
        known_factors: smallvec::SmallVec<[Uint; 8]>,
        remaining: Uint,
    },
    Failure(Uint),
}

impl FactorizationResult {
    /// Reports whether this `FactorizationResult` represents a complete factorization.
    ///
    /// # Returns
    ///
    /// `true` if the value is `FactorizationResult::Complete(_)`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let complete = FactorizationResult::Complete(smallvec::SmallVec::new());
    /// let partial = FactorizationResult::Partial { known_factors: smallvec::smallvec![], remaining: 2u128.into() };
    /// assert!(complete.is_complete());
    /// assert!(!partial.is_complete());
    /// ```
    pub fn is_complete(&self) -> bool {
        matches!(self, FactorizationResult::Complete(_))
    }
    /// Retrieve the known factors contained in this `FactorizationResult`.
    ///
    /// For `Complete` and `Partial` variants this returns a cloned list of the discovered factors;
    /// for `Failure` it returns an empty vector.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = FactorizationResult::Complete(smallvec::smallvec![Uint::from(2u64), Uint::from(3u64)]);
    /// assert_eq!(c.factors(), &[Uint::from(2u64), Uint::from(3u64)]);
    ///
    /// let p = FactorizationResult::Partial { known_factors: smallvec::smallvec![Uint::from(5u64)], remaining: Uint::from(7u64) };
    /// assert_eq!(p.factors(), &[Uint::from(5u64)]);
    ///
    /// let f = FactorizationResult::Failure(Uint::from(11u64));
    /// assert_eq!(f.factors(), &[]);
    /// ```
    pub fn factors(&self) -> &[Uint] {
        match self {
            FactorizationResult::Complete(f) => f.as_slice(),
            FactorizationResult::Partial { known_factors, .. } => known_factors.as_slice(),
            FactorizationResult::Failure(_) => &[],
        }
    }
}

/// Computes the product of `a` and `b` modulo `m` without overflowing `u128`.
///
/// This returns `(a * b) % m` while avoiding direct multiplication that could overflow `u128`.
///
/// # Panics
///
/// Panics if `m == 0`.
///
/// # Examples
///
/// ```
/// let r = mul_mod_u128(1_000_000_000_000_000_000_000u128, 3_000_000_000_000_000_000_000u128, 1_000_000_000u128);
/// assert_eq!(r, ((1_000_000_000_000_000_000_000u128 % 1_000_000_000u128) * (3_000_000_000_000_000_000_000u128 % 1_000_000_000u128)) % 1_000_000_000u128);
/// ```
/// Compute modular exponentiation: base^exp modulo m.
///
/// Returns the value of `base` raised to `exp` modulo `m`.
///
/// # Examples
///
/// ```
/// let r = pow_mod_u128(3u128, 13u128, 100u128);
/// assert_eq!(r, 3u128.pow(13) % 100);
/// ```
/// Determines whether a 128-bit unsigned integer is prime using a deterministic Miller–Rabin test with fixed bases up to 71.
///

pub struct TrialSieve {
    pub small_primes: Vec<u64>,
}

impl TrialSieve {
    /// Builds a TrialSieve containing all primes up to the given limit.
    ///
    /// The returned `TrialSieve` stores the list of prime numbers >= 2 and <= `limit`.
    ///
    /// # Examples
    ///
    /// ```
    /// let ts = TrialSieve::new(100);
    /// // 97 is the largest prime <= 100
    /// assert!(ts.small_primes.contains(&97));
    /// ```
    pub fn new(limit: u64) -> Self {
        let sieve = primal::Sieve::new(limit as usize);
        let small_primes: Vec<u64> = sieve.primes_from(2).map(|p| p as u64).collect();
        TrialSieve { small_primes }
    }

    /// Performs trial division on `n` using the sieve's stored small primes and, if a nontrivial cofactor remains, delegates further factoring to the rho-based factorizer.
    ///
    /// On success this returns `FactorizationResult::Complete` with a sorted list of prime factors. If trial division finds some factors but a remaining composite cofactor could not be fully factored by the rho step, this returns `FactorizationResult::Partial { known_factors, remaining }` where `known_factors` contains the discovered primes (sorted) and `remaining` is the unfactored cofactor. If the rho step reports a failure for the remaining cofactor, that failure is propagated as a `Partial` with the failure value in `remaining`.
    ///
    /// # Examples
    ///
    /// ```
    /// let sieve = TrialSieve::new(100);
    /// let res = sieve.factor(Uint::from_u128(12));
    /// match res {
    ///     FactorizationResult::Complete(factors) => {
    ///         assert_eq!(factors.to_vec(), vec![Uint::from_u128(2), Uint::from_u128(2), Uint::from_u128(3)]);
    ///     }
    ///     _ => panic!("expected complete factorization"),
    /// }
    /// ```
    pub fn factor(&self, mut n: Uint) -> FactorizationResult {
        if n <= Uint::one() {
            return FactorizationResult::Complete(smallvec::SmallVec::new());
        }
        let mut factors = smallvec::SmallVec::<[Uint; 8]>::new();
        for &p in &self.small_primes {
            let p_u = Uint::from_u128((p) as u128);
            if p_u * p_u > n {
                break;
            }
            while n % p_u == Uint::zero() {
                factors.push(p_u);
                n /= p_u;
            }
        }
        if n > Uint::one() {
            let limit_u = Uint::from_u128(self.small_primes.last().copied().unwrap_or(2) as u128);
            if n <= limit_u * limit_u {
                factors.push(n);
                return FactorizationResult::Complete(factors);
            } else {
                let rho_res = rho_factor_u256(n);
                match rho_res {
                    FactorizationResult::Complete(v) => {
                        factors.extend(v);
                        factors.sort_unstable();
                        return FactorizationResult::Complete(factors);
                    }
                    FactorizationResult::Partial {
                        known_factors,
                        remaining,
                    } => {
                        factors.extend(known_factors);
                        factors.sort_unstable();
                        return FactorizationResult::Partial {
                            known_factors: factors,
                            remaining,
                        };
                    }
                    FactorizationResult::Failure(u) => {
                        factors.sort_unstable();
                        return FactorizationResult::Partial {
                            known_factors: factors,
                            remaining: u,
                        };
                    }
                }
            }
        }
        factors.sort_unstable();
        FactorizationResult::Complete(factors)
    }
}

/// Factorizes `n` using Pollard–Rho (Brent) recursion and fallback factorization strategies.
///
/// The result is a `FactorizationResult`:
/// - `Complete(smallvec::SmallVec<[Uint; 8]>)` when all prime factors were found (vector is sorted).
/// - `Partial { known_factors, remaining }` when some factors were found but a composite remainder could not be fully factored; `remaining` is the unfactored cofactor (>= 2).
/// - `Failure(Uint)` when factorization could not proceed (for example, when Pollard–Rho fails and no safe fallback is available).
///
/// # Examples
///
/// ```
/// let n = Uint::from_u64(15);
/// match rho_factor_u256(n) {
///     FactorizationResult::Complete(factors) => {
///         assert_eq!(factors.to_vec(), vec![Uint::from_u64(3), Uint::from_u64(5)]);
///     }
///     _ => panic!("expected complete factorization"),
/// }
/// ```
pub fn rho_factor_u256(n: Uint) -> FactorizationResult {
    if n <= Uint::one() {
        return FactorizationResult::Complete(smallvec::SmallVec::new());
    }
    if verified_is_prime(n) {
        return FactorizationResult::Complete(smallvec::smallvec![n]);
    }

    let limit_256 = (Uint::one() << 256) - Uint::one();
    if n > limit_256 {
        return FactorizationResult::Partial {
            known_factors: smallvec::SmallVec::new(),
            remaining: n,
        };
    }

    if let Some(d) = pollard_rho_brent_u256(n) {
        let res_d = rho_factor_u256(d);
        let res_rem = rho_factor_u256(n / d);

        match (res_d, res_rem) {
            (FactorizationResult::Complete(mut f1), FactorizationResult::Complete(f2)) => {
                f1.extend(f2);
                f1.sort_unstable();
                FactorizationResult::Complete(f1)
            }
            (f1, f2) => {
                let mut known = smallvec::SmallVec::<[Uint; 8]>::new();
                let mut rem = Uint::one();

                match f1 {
                    FactorizationResult::Complete(v) => known.extend(v),
                    FactorizationResult::Partial {
                        known_factors,
                        remaining,
                    } => {
                        known.extend(known_factors);
                        rem *= remaining;
                    }
                    FactorizationResult::Failure(u) => rem *= u,
                };
                match f2 {
                    FactorizationResult::Complete(v) => known.extend(v),
                    FactorizationResult::Partial {
                        known_factors,
                        remaining,
                    } => {
                        known.extend(known_factors);
                        rem *= remaining;
                    }
                    FactorizationResult::Failure(u) => rem *= u,
                };

                known.sort_unstable();
                if rem > Uint::one() {
                    FactorizationResult::Partial {
                        known_factors: known,
                        remaining: rem,
                    }
                } else {
                    FactorizationResult::Complete(known)
                }
            }
        }
    } else {
        if n <= Uint::from_u128((u128::MAX) as u128) {
            if let Ok(fact) = catch_unwind(|| Factorization::run(n.as_u128())) {
                FactorizationResult::Complete(
                    fact.factors
                        .into_iter()
                        .map(|f| Uint::from_u128((f) as u128))
                        .collect(),
                )
            } else {
                FactorizationResult::Failure(n)
            }
        } else {
            FactorizationResult::Failure(n)
        }
    }
}

pub fn pollard_rho_brent_u256(n: Uint) -> Option<Uint> {
    if n & Uint::one() == Uint::zero() {
        return Some(Uint::from_u128((2u32) as u128));
    }
    for c in 1..40u32 {
        let mut x = Uint::from_u128((2u32) as u128);
        let mut y = Uint::from_u128((2u32) as u128);
        let mut d = Uint::one();

        let c_u = Uint::from_u128((c) as u128);
        let f = |x: Uint| -> Uint { add_mod_u256(mul_mod_u256(x, x, n), c_u, n) };

        let mut q = Uint::one();
        let mut ys = Uint::zero();
        let mut r = 1u32;

        while d == Uint::one() {
            x = y;
            for _ in 0..r {
                y = f(y);
            }
            let mut k = 0u32;
            while k < r && d == Uint::one() {
                ys = y;
                let batch = r - k;
                let batch = if batch > crate::profile::get_profile().pollard_rho_batch_size {
                    crate::profile::get_profile().pollard_rho_batch_size
                } else {
                    batch
                };
                for _ in 0..batch {
                    y = f(y);
                    let diff = if x > y { x - y } else { y - x };
                    q = mul_mod_u256(q, diff, n);
                }
                d = gcd_u256(q, n);
                k += batch;
            }
            r *= 2;
            if r > crate::lean_ffi::get_pollard_rho_iteration_limit() {
                break;
            }
        }

        if d != Uint::one() && d != n {
            return Some(d);
        }
        if d == n {
            loop {
                ys = f(ys);
                let diff = if x > ys { x - ys } else { ys - x };
                d = gcd_u256(diff, n);
                if d != Uint::one() {
                    break;
                }
            }
            if d != n {
                return Some(d);
            }
        }
    }
    None
}

pub type SigmaCache = HashMap<(Uint, u32), Uint>;

#[inline]
pub fn sigma_cached(cache: &SigmaCache, p: Uint, pow: u32) -> Uint {
    if let Some(&val) = cache.get(&(p, pow)) {
        val
    } else if p <= Uint::from_u128(u64::MAX as u128) {
        crate::lean_ffi::compute_sigma(p.as_u128() as u64, pow)
    } else {
        // Fallback to pure Rust checked calculation if prime p exceeds u64 limits
        let mut sum = Uint::one();
        let mut current = Uint::one();
        for _ in 1..=pow {
            if let Some(next) = current.checked_mul(p) {
                current = next;
                if let Some(next_sum) = sum.checked_add(current) {
                    sum = next_sum;
                } else {
                    panic!(
                        "sigma_cached overflow: σ({}^{}) does not fit in 256 bits",
                        p, pow
                    );
                }
            } else {
                panic!(
                    "sigma_cached overflow: σ({}^{}) does not fit in 256 bits",
                    p, pow
                );
            }
        }
        sum
    }
}

pub fn mul_mod_u256(mut a: Uint, mut b: Uint, m: Uint) -> Uint {
    if m <= Uint::from_u128((0xFFFFFFFFFFFFFFFFu64) as u128) {
        return (a % m * (b % m)) % m;
    }
    a %= m;
    b %= m;
    if let Some(prod) = a.checked_mul(b) {
        return prod % m;
    }

    let mut a_bytes = [0u8; 128];
    let mut b_bytes = [0u8; 128];
    let mut m_bytes = [0u8; 128];
    a_bytes[..64].copy_from_slice(&a.to_le_bytes());
    b_bytes[..64].copy_from_slice(&b.to_le_bytes());
    m_bytes[..64].copy_from_slice(&m.to_le_bytes());

    let a_1024 = bnum::types::U1024::from_le_bytes(a_bytes);
    let b_1024 = bnum::types::U1024::from_le_bytes(b_bytes);
    let m_1024 = bnum::types::U1024::from_le_bytes(m_bytes);

    let prod_1024 = a_1024 * b_1024;
    let res_1024 = prod_1024 % m_1024;
    let res_bytes = res_1024.to_le_bytes();
    let mut res_512 = [0u8; 64];
    res_512.copy_from_slice(&res_bytes[..64]);
    Uint::from_le_bytes(res_512)
}

pub fn add_mod_u256(a: Uint, b: Uint, m: Uint) -> Uint {
    let a = a % m;
    let b = b % m;
    if a >= m - b {
        a - (m - b)
    } else {
        debug_assert!(
            a.checked_add(b).is_some(),
            "Overflow detected in add_mod_u256"
        );
        a + b
    }
}

pub fn modpow_u256(mut base: Uint, mut exp: Uint, modulus: Uint) -> Uint {
    if modulus <= Uint::one() {
        return Uint::zero();
    }
    let mut result = Uint::one();
    base %= modulus;
    while exp > Uint::zero() {
        if exp & Uint::one() == Uint::one() {
            result = mul_mod_u256(result, base, modulus);
        }
        exp >>= 1;
        base = mul_mod_u256(base, base, modulus);
    }
    result
}

pub fn verified_is_prime_low(n: Uint) -> bool {
    if n <= Uint::one() {
        return false;
    }
    let threshold = Uint::from_u128(1_u128 << 64);
    if n >= threshold {
        return false;
    }
    let n_u64 = n.as_u128() as u64;
    let bases_12: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    for &b in &bases_12 {
        if n_u64 == b {
            return true;
        }
        if n_u64 % b == 0 {
            return false;
        }
    }

    let mut d = n_u64 - 1;
    let mut s = 0;
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }

    for &base in &bases_12 {
        let mut x = modpow_u256(Uint::from_u64(base), Uint::from_u64(d), n);
        if x == Uint::one() || x == n - Uint::one() {
            continue;
        }
        let mut composite = true;
        for _ in 0..(s - 1) {
            x = mul_mod_u256(x, x, n);
            if x == Uint::one() {
                return false;
            }
            if x == n - Uint::one() {
                composite = false;
                break;
            }
        }
        if composite {
            return false;
        }
    }
    true
}

pub fn generate_and_verify_pocklington(n: Uint) -> bool {
    if n <= Uint::one() {
        return false;
    }
    if n == Uint::from_u32(2) {
        return true;
    }
    let n_minus_1 = n - Uint::one();
    let mut f = Uint::one();
    let mut unique_prime_factors = Vec::new();
    let mut remaining = n_minus_1;

    let small_primes: [u32; 25] = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97,
    ];
    for &p in &small_primes {
        let p_u = Uint::from_u32(p);
        if remaining % p_u == Uint::zero() {
            unique_prime_factors.push(p_u);
            while remaining % p_u == Uint::zero() {
                f *= p_u;
                remaining /= p_u;
            }
        }
    }

    let mut is_pocklington = f.checked_mul(f).map_or(true, |f2| f2 > n_minus_1);
    let mut is_bls = if is_pocklington {
        false
    } else {
        f.checked_mul(f)
            .and_then(|f2| f2.checked_mul(f))
            .map_or(true, |f3| f3 > n_minus_1)
    };

    if remaining > Uint::one() && !is_pocklington && !is_bls {
        if verified_is_prime(remaining) {
            f *= remaining;
            unique_prime_factors.push(remaining);
        } else {
            let limit_256 = (Uint::one() << 256) - Uint::one();
            if remaining <= limit_256 {
                let facs = match quick_factor_u256(remaining) {
                    crate::math_utils::FactorizationResult::Complete(facs) => facs,
                    crate::math_utils::FactorizationResult::Partial { known_factors, .. } => {
                        known_factors
                    }
                    crate::math_utils::FactorizationResult::Failure(_) => smallvec::SmallVec::new(),
                };
                let mut last_p = Uint::zero();
                for p in facs {
                    f *= p;
                    if p != last_p {
                        unique_prime_factors.push(p);
                        last_p = p;
                    }
                }
            }
        }
        is_pocklington = f.checked_mul(f).map_or(true, |f2| f2 > n_minus_1);
        is_bls = if is_pocklington {
            false
        } else {
            f.checked_mul(f)
                .and_then(|f2| f2.checked_mul(f))
                .map_or(true, |f3| f3 > n_minus_1)
        };
    }

    if !is_pocklington && !is_bls {
        return false;
    }

    let bases: [u32; 10] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];
    for &base in &bases {
        let a = Uint::from_u32(base);
        if a % n == Uint::zero() {
            continue;
        }
        if modpow_u256(a, n_minus_1, n) != Uint::one() {
            return false;
        }

        let mut valid_a = true;
        for &q in &unique_prime_factors {
            let exponent = n_minus_1 / q;
            let a_pow = modpow_u256(a, exponent, n);
            let a_pow_minus_1 = if a_pow == Uint::zero() {
                n - Uint::one()
            } else {
                a_pow - Uint::one()
            };

            if gcd_u256(a_pow_minus_1, n) != Uint::one() {
                valid_a = false;
                break;
            }
        }

        if valid_a {
            if is_pocklington {
                return true;
            } else if is_bls {
                // Perform the BLS check
                let r_val = n_minus_1 / f;
                let double_f = f.checked_mul(Uint::from_u32(2)).unwrap_or(Uint::MAX);
                let q_div = r_val / double_f;
                let r_rem = r_val % double_f;
                let (q, r, r_is_negative) = if r_rem <= f {
                    (q_div, r_rem, false)
                } else {
                    (q_div + Uint::one(), double_f - r_rem, true)
                };

                let mut is_composite = false;
                if !r_is_negative {
                    if let Some(r_sq_val) = r.checked_mul(r) {
                        if let Some(eight_q_val) = Uint::from_u32(8).checked_mul(q) {
                            if r_sq_val >= eight_q_val {
                                let diff = r_sq_val - eight_q_val;
                                let s = diff.isqrt();
                                if s * s == diff {
                                    if let Some(s_plus_2) = s.checked_add(Uint::from_u32(2)) {
                                        if r >= s_plus_2 {
                                            is_composite = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if is_composite {
                    return false;
                } else {
                    return true;
                }
            }
        }
    }

    false
}

pub fn verified_is_prime(n: Uint) -> bool {
    if n <= Uint::one() {
        return false;
    }

    // Hybrid Tiered Primality: Route small inputs (< 2^64) through the proven deterministic 12-base Miller-Rabin test.
    let threshold = Uint::from_u128(1_u128 << 64);
    if n < threshold {
        if verified_is_prime_low(n) {
            return generate_and_verify_pocklington(n);
        } else {
            return false;
        }
    }

    // For all inputs >= 2^64:
    // First, run a fast non-binding Miller-Rabin pre-filter.
    // If the pre-filter determines the number is composite, reject it.
    // Otherwise, generate and verify a Pocklington certificate for absolute mathematical certainty.
    let bases: [u32; 20] = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71,
    ];

    for &b in &bases {
        if n == Uint::from_u128(b as u128) {
            return true;
        }
    }

    for &b in &bases {
        if n % Uint::from_u128(b as u128) == Uint::zero() {
            return false;
        }
    }

    let mut d = n - Uint::one();
    let mut s = 0;
    while d & Uint::one() == Uint::zero() {
        d >>= 1;
        s += 1;
    }

    let mut is_mr_prime = true;
    for &base_u32 in &bases {
        let a = Uint::from_u128(base_u32 as u128);
        if a >= n {
            continue;
        }
        let mut x = modpow_u256(a, d, n);
        if x == Uint::one() || x == n - Uint::one() {
            continue;
        }
        let mut composite = true;
        for _ in 0..(s - 1) {
            x = mul_mod_u256(x, x, n);
            if x == Uint::one() {
                is_mr_prime = false;
                break;
            }
            if x == n - Uint::one() {
                composite = false;
                break;
            }
        }
        if !is_mr_prime || composite {
            is_mr_prime = false;
            break;
        }
    }

    if !is_mr_prime {
        return false;
    }

    // Candidate passed the fast pre-filter. Validate using a verified Pocklington certificate.
    generate_and_verify_pocklington(n)
}

/// Compute the greatest common divisor of two unsigned integers using the Euclidean algorithm.
///
/// # Examples
///
/// ```
/// let a = Uint::from(48u64);
/// let b = Uint::from(18u64);
/// let g = gcd_u256(a, b);
/// assert_eq!(g, Uint::from(6u64));
/// ```
///
/// # Returns
///
/// The greatest common divisor of `a` and `b`.
fn gcd_u256(mut a: Uint, mut b: Uint) -> Uint {
    while b != Uint::zero() {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Performs a fast, primarily trial-based factorization of `n`, returning any discovered prime factors
/// and indicating if a remainder remained unfactored or if factorization failed.
///
/// This function:
/// - Removes small prime factors (2,3,5,7,11,13) and continues trial division with an incremental
///   pattern up to a small bound (trial divisors < 10_000).
/// - If a nontrivial remainder remains, accepts it as a factor when it is small or prime; otherwise
///   delegates to heavier factorization routines (u128-based `Factorization::run` when applicable,
///   or `rho_factor_u256`). If those routines cannot fully factor the remainder or fail, this
///   function returns a `Partial` result containing all known factors and the unfactored remainder.
///
/// # Returns
///
/// - `FactorizationResult::Complete(factors)` when all prime factors of `n` were found (possibly an
///   empty vector for `n <= 1`).
/// - `FactorizationResult::Partial { known_factors, remaining }` when some factors were found but a
///   nontrivial cofactor could not be fully factored; `known_factors` are the discovered factors
///   (sorted) and `remaining` is the unfactored cofactor.
/// - `FactorizationResult::Failure(u)` when factorization ultimately failed and `u` is the
///   remaining unfactored value (this variant is propagated as `Partial` by this function when
///   appropriate).
///
/// # Examples
///
/// ```
/// # use crate::math_utils::{quick_factor_u256, FactorizationResult};
/// # use crate::Uint;
/// let n = Uint::from_u128(60);
/// match quick_factor_u256(n) {
///     FactorizationResult::Complete(mut v) => {
///         v.sort_unstable();
///         let expected = vec![Uint::from_u128(2), Uint::from_u128(2), Uint::from_u128(3), Uint::from_u128(5)];
///         assert_eq!(v.to_vec(), expected);
///     }
///     other => panic!("unexpected result: {:?}", other),
/// }
/// ```
pub fn quick_factor_u256(n: Uint) -> FactorizationResult {
    if n <= Uint::one() {
        return FactorizationResult::Complete(smallvec::SmallVec::new());
    }

    // Hybrid Tiered Primality: Route small inputs (< 2^64) through the verified exact trial-division algorithm.
    let threshold = Uint::from_u128(1_u128 << 64);
    if n < threshold {
        let mut n_u64 = n.as_u128() as u64;
        let mut factors = smallvec::SmallVec::<[Uint; 8]>::new();
        while n_u64 % 2 == 0 {
            factors.push(Uint::from_u128(2));
            n_u64 /= 2;
        }
        let mut d = 3u64;
        while (d * d) <= n_u64 && d < 10_000 {
            while n_u64 % d == 0 {
                factors.push(Uint::from_u128(d as u128));
                n_u64 /= d;
            }
            d += 2;
        }
        if n_u64 > 1 {
            let n_uint = Uint::from_u64(n_u64);
            if verified_is_prime(n_uint) {
                factors.push(n_uint);
            } else {
                match rho_factor_u256(n_uint) {
                    FactorizationResult::Complete(v) => factors.extend(v),
                    FactorizationResult::Partial {
                        known_factors,
                        remaining,
                    } => {
                        factors.extend(known_factors);
                        factors.sort_unstable();
                        return FactorizationResult::Partial {
                            known_factors: factors,
                            remaining,
                        };
                    }
                    FactorizationResult::Failure(u) => {
                        factors.sort_unstable();
                        return FactorizationResult::Partial {
                            known_factors: factors,
                            remaining: u,
                        };
                    }
                }
            }
        }
        factors.sort_unstable();
        return FactorizationResult::Complete(factors);
    }

    let mut remaining = n;
    let mut factors = smallvec::SmallVec::<[Uint; 8]>::new();
    for &p_u32 in &[2u32, 3, 5, 7, 11, 13] {
        let p = Uint::from_u128((p_u32) as u128);
        while remaining % p == Uint::zero() {
            factors.push(p);
            remaining /= p;
        }
    }
    let mut d = Uint::from_u128((17u32) as u128);
    while d * d <= remaining && d < Uint::from_u128((10_000u32) as u128) {
        while remaining % d == Uint::zero() {
            factors.push(d);
            remaining /= d;
        }
        d += Uint::from_u128((2u32) as u128);
        while remaining % d == Uint::zero() {
            factors.push(d);
            remaining /= d;
        }
        d += Uint::from_u128((4u32) as u128);
    }
    if remaining > Uint::one() {
        if remaining < Uint::from_u128((100_000_000u32) as u128) || verified_is_prime(remaining) {
            factors.push(remaining);
        } else {
            if remaining <= Uint::from_u128((u128::MAX) as u128) {
                if let Ok(res) = catch_unwind(|| Factorization::run(remaining.as_u128())) {
                    factors.extend(res.factors.into_iter().map(Uint::from_u128));
                } else {
                    let rho_res = rho_factor_u256(remaining);
                    match rho_res {
                        FactorizationResult::Complete(v) => factors.extend(v),
                        FactorizationResult::Partial {
                            known_factors,
                            remaining: r,
                        } => {
                            factors.extend(known_factors);
                            factors.sort_unstable();
                            return FactorizationResult::Partial {
                                known_factors: factors,
                                remaining: r,
                            };
                        }
                        FactorizationResult::Failure(u) => {
                            factors.sort_unstable();
                            return FactorizationResult::Partial {
                                known_factors: factors,
                                remaining: u,
                            };
                        }
                    }
                }
            } else {
                let limit_256 = (Uint::one() << 256) - Uint::one();
                if remaining <= limit_256 {
                    let ecm_factors = rho_factor_u256(remaining);
                    match ecm_factors {
                        FactorizationResult::Complete(v) => factors.extend(v),
                        FactorizationResult::Partial {
                            known_factors,
                            remaining: r,
                        } => {
                            factors.extend(known_factors);
                            factors.sort_unstable();
                            return FactorizationResult::Partial {
                                known_factors: factors,
                                remaining: r,
                            };
                        }
                        FactorizationResult::Failure(u) => {
                            factors.sort_unstable();
                            return FactorizationResult::Partial {
                                known_factors: factors,
                                remaining: u,
                            };
                        }
                    }
                } else {
                    // Over 256 bits: avoid heavy trial division to prevent timeouts
                    factors.sort_unstable();
                    return FactorizationResult::Partial {
                        known_factors: factors,
                        remaining,
                    };
                }
            }
        }
    }
    factors.sort_unstable();
    FactorizationResult::Complete(factors)
}

/// Return all positive divisors of `n` in ascending order.
///
/// The result includes `1` and `n` when applicable. For `n == 0` the function returns an empty vector.
///
/// # Examples
///
/// ```
/// assert_eq!(small_divisors_pub(1), vec![1]);
/// assert_eq!(small_divisors_pub(12), vec![1, 2, 3, 4, 6, 12]);
/// assert_eq!(small_divisors_pub(13), vec![13, 1].into_iter().collect::<Vec<_>>().iter().cloned().collect::<Vec<u32>>()); // demonstrate prime handling
/// ```
/// Factorizes σ(p, two_e) by evaluating its cyclotomic components and factoring each result.
///
/// For each divisor `d` of `two_e + 1` (excluding `1`), this function attempts to evaluate the
/// `d`-th cyclotomic polynomial at `p` and factor the value. If a cyclotomic evaluation is
/// unavailable (`None`), the function falls back to factoring the full value returned by
/// `crate::lean_ffi::compute_sigma(p, two_e)`. Found prime factors are collected and returned in
/// sorted order. If any component factorization is incomplete or fails, known factors are returned
/// along with a remaining unfactored cofactor in the `Partial` variant; a total failure for a
/// component is accumulated into the remaining cofactor and also yields `Partial`.
///
/// # Returns
///
/// `FactorizationResult::Complete(factors)` when all cyclotomic components were fully factored;
/// `FactorizationResult::Partial { known_factors, remaining }` when one or more components could
/// not be fully factored (with `remaining` being the product of unfactored cofactors); or
/// `FactorizationResult::Failure(u)` only when the fallback full-sigma factoring produced a failure
/// (propagated from the underlying factoring routine).
///
/// # Examples
///
/// ```
/// let p = 3u64;
/// let two_e = 2u32;
/// let res = factor_sigma_cyclotomic(p, two_e);
/// match res {
///     FactorizationResult::Complete(factors) => {
///         let prod = factors.into_iter().fold(Uint::one(), |acc, x| acc * x);
///         assert_eq!(prod, crate::lean_ffi::compute_sigma(p, two_e));
///     }
///     _ => panic!("expected complete factorization for this example"),
/// }
/// ```
/// Computes the modular inverse of `a` modulo `m`, returning `None` if no inverse exists or if `m <= 0`.
///
/// The result `x` satisfies `0 <= x < m` and `(a * x) % m == 1` when present.
///
/// # Examples
///
/// ```
/// // 3 * 4 ≡ 1 (mod 11)
/// assert_eq!(mod_inverse_big(Int::from(3), Int::from(11)), Some(Int::from(4)));
///
/// // 2 has no inverse modulo 4
/// assert_eq!(mod_inverse_big(Int::from(2), Int::from(4)), None);
/// ```
pub fn mod_inverse_big(a: Int, m: Int) -> Option<Int> {
    if m <= Int::zero() {
        return None;
    }

    let a_neg = a < Int::zero();
    let a_abs = if a_neg { -a } else { a }.as_uint();

    crate::lean_ffi::compute_mod_inverse(&a_abs, a_neg, &m.as_uint()).map(|x| x.as_int())
}

pub fn solve_crt(residues: &[Int], moduli: &[Int]) -> Option<Int> {
    let mut total_mod = Uint::one();
    for &m in moduli {
        total_mod *= m.as_uint();
    }

    let mut x = Uint::zero();
    for (&r, &m) in residues.iter().zip(moduli.iter()) {
        let m_u = m.as_uint();
        let r_u = {
            let mut val = r % m;
            if val < Int::zero() {
                val += m;
            }
            val.as_uint()
        };
        let m_i = total_mod / m_u;
        let m_i_mod_m = m_i % m_u;

        let y_i = mod_inverse_u512(m_i_mod_m, m_u)?;

        let term1 = (r_u * y_i) % total_mod;
        let term2 = (term1 * m_i) % total_mod;
        x = (x + term2) % total_mod;
    }

    Some(x.as_int())
}

pub fn tonelli_shanks(n: Int, p: Int) -> Option<Int> {
    if p <= Int::zero() {
        return None;
    }
    let mut n_mod_p = n % p;
    if n_mod_p < Int::zero() {
        n_mod_p += p;
    }

    if n_mod_p == Int::zero() {
        return Some(Int::zero());
    }
    if p == Int::from_u128((2u32) as u128) {
        return Some(n_mod_p);
    }

    let p_minus_one = p - Int::one();
    let mut q = p_minus_one;
    let mut s = 0u32;
    while q % Int::from_u128((2u32) as u128) == Int::zero() {
        q /= Int::from_u128((2u32) as u128);
        s += 1;
    }

    if modpow_u256(
        n_mod_p.as_uint(),
        (p_minus_one / Int::from_u128((2u32) as u128)).as_uint(),
        p.as_uint(),
    ) != Uint::one()
    {
        return None;
    }

    let mut z = Uint::from_u128((2u32) as u128);
    while modpow_u256(
        z,
        (p_minus_one / Int::from_u128((2u32) as u128)).as_uint(),
        p.as_uint(),
    ) != p_minus_one.as_uint()
    {
        z += Uint::one();
    }

    let mut m = s;
    let mut c = modpow_u256(z, q.as_uint(), p.as_uint()).as_int();
    let mut t = modpow_u256(n_mod_p.as_uint(), q.as_uint(), p.as_uint()).as_int();
    let mut r = modpow_u256(
        n_mod_p.as_uint(),
        ((q + Int::one()) / Int::from_u128((2u32) as u128)).as_uint(),
        p.as_uint(),
    )
    .as_int();

    loop {
        if t == Int::zero() {
            return Some(Int::zero());
        }
        if t == Int::one() {
            return Some(r.as_int());
        }

        let mut t2i = t;
        let mut i = 0u32;
        while i < m {
            if t2i == Int::one() {
                break;
            }
            t2i = mul_mod_u256(t2i.as_uint(), t2i.as_uint(), p.as_uint()).as_int();
            i += 1;
        }

        if i == m {
            return None;
        }

        let exp = 1u32 << (m - i - 1);
        let b = modpow_u256(c.as_uint(), Uint::from_u128((exp) as u128), p.as_uint()).as_int();

        m = i;
        c = mul_mod_u256(b.as_uint(), b.as_uint(), p.as_uint()).as_int();
        t = mul_mod_u256(t.as_uint(), c.as_uint(), p.as_uint()).as_int();
        r = mul_mod_u256(r.as_uint(), b.as_uint(), p.as_uint()).as_int();
    }
}

pub fn hensels_lift(root: Int, n: Int, p: Int, k: u32) -> Option<Int> {
    let mut current_r = root;
    let mut current_mod = p;

    for _ in 1..k {
        current_mod *= p;

        let r_sqr = mul_mod_u256(
            current_r.as_uint(),
            current_r.as_uint(),
            current_mod.as_uint(),
        )
        .as_int();
        let mut diff = (r_sqr.as_int() - n + current_mod) % current_mod;
        if diff < Int::zero() {
            diff += current_mod;
        }

        let two_r = (Int::from_u128((2u32) as u128) * current_r) % current_mod;

        if let Some(inv_two_r) = mod_inverse_big(two_r, current_mod) {
            let adjustment =
                mul_mod_u256(diff.as_uint(), inv_two_r.as_uint(), current_mod.as_uint()).as_int();
            current_r = (current_r - adjustment) % current_mod;
            if current_r < Int::zero() {
                current_r += current_mod;
            }
        } else {
            return None;
        }
    }
    Some(current_r)
}

pub struct RootIterator {
    prime_roots: Vec<Vec<Int>>,
    moduli: Vec<Int>,
    indices: Vec<usize>,
    done: bool,
    pub math_interruption: bool,
}

impl Iterator for RootIterator {
    type Item = Int;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.prime_roots.is_empty() {
            return None;
        }

        loop {
            let current_residues: Vec<Int> = self
                .indices
                .iter()
                .enumerate()
                .map(|(i, &idx)| self.prime_roots[i][idx])
                .collect();

            let root_opt = solve_crt(&current_residues, &self.moduli);

            let mut carry = true;
            for i in 0..self.prime_roots.len() {
                if carry {
                    self.indices[i] += 1;
                    if self.indices[i] >= self.prime_roots[i].len() {
                        self.indices[i] = 0;
                    } else {
                        carry = false;
                    }
                }
            }
            if carry {
                self.done = true;
            }

            if let Some(combined_root) = root_opt {
                return Some(combined_root);
            }

            if self.done {
                return None;
            }
        }
    }
}

pub fn solve_mod_2_k(n: Int, k: u32) -> Vec<Int> {
    assert!(k < 256, "k must be < 256 for solve_mod_2_k");
    let mask = (Uint::one() << k) - Uint::one();
    let n_u256 = n.as_uint() & mask;

    if k == 1 {
        return vec![(n_u256 % Uint::from_u128((2u32) as u128)).as_int()];
    }
    if k == 2 {
        if n_u256 % Uint::from_u128((4u32) as u128) == Uint::one() {
            return vec![Int::one(), Int::from_u128((3u32) as u128)];
        } else if n_u256 % Uint::from_u128((4u32) as u128) == Uint::zero() {
            return vec![Int::zero(), Int::from_u128((2u32) as u128)];
        } else {
            return vec![];
        }
    }

    if n_u256 % Uint::from_u128((8u32) as u128) != Uint::one() {
        if n_u256 % Uint::from_u128((2u32) as u128) == Uint::zero() {
            if k <= 12 {
                let mut roots = vec![];
                let mod_k = Uint::one() << k;
                let mut i = Uint::zero();
                while i < mod_k {
                    if mul_mod_u256(i, i, mod_k) == n_u256 {
                        roots.push(i.as_int());
                    }
                    i += Uint::one();
                }
                return roots;
            }
        }
        return vec![];
    }

    let mut r = Uint::one();
    for m in 4..=k {
        let mod_m = Uint::one() << m;
        let r_sqr = mul_mod_u256(r, r, mod_m);
        let n_mod_m = n_u256 & ((Uint::one() << m) - Uint::one());
        if r_sqr != n_mod_m {
            r += Uint::one() << (m - 2);
        }
    }

    let mod_k = Uint::one() << k;
    let mut roots = vec![
        r.as_int(),
        (mod_k - r).as_int(),
        ((r + (Uint::one() << (k - 1))) % mod_k).as_int(),
        ((mod_k - ((r + (Uint::one() << (k - 1))) % mod_k)) % mod_k).as_int(),
    ];
    roots.sort_unstable();
    roots.dedup();
    roots
}

pub fn composite_tonelli_shanks(n: Int, m_factors: &[Uint]) -> RootIterator {
    let mut prime_counts: HashMap<Int, u32> = HashMap::new();
    for &f in m_factors {
        *prime_counts.entry(f.as_int()).or_insert(0) += 1;
    }

    let mut moduli = Vec::new();
    let mut prime_roots = Vec::new();

    for (p, k) in prime_counts {
        let p_pow_k = p.pow(k);

        if p == Int::from_u128((2u32) as u128) {
            let p_roots = solve_mod_2_k(n, k);
            if p_roots.is_empty() {
                return RootIterator {
                    prime_roots: vec![],
                    moduli: vec![],
                    indices: vec![],
                    done: true,
                    math_interruption: true,
                };
            }
            prime_roots.push(p_roots);
            moduli.push(p_pow_k);
            continue;
        }

        let mut p_roots = Vec::new();
        if let Some(r) = tonelli_shanks(n, p) {
            if let Some(r_lifted) = hensels_lift(r, n, p, k) {
                p_roots.push(r_lifted);

                let mut neg_r = p_pow_k - r_lifted;
                neg_r %= p_pow_k;
                if neg_r != r_lifted {
                    p_roots.push(neg_r);
                }
            } else {
                return RootIterator {
                    prime_roots: vec![],
                    moduli: vec![],
                    indices: vec![],
                    done: true,
                    math_interruption: true,
                };
            }
        } else {
            return RootIterator {
                prime_roots: vec![],
                moduli: vec![],
                indices: vec![],
                done: true,
                math_interruption: false,
            };
        }

        prime_roots.push(p_roots);
        moduli.push(p_pow_k);
    }

    let indices = vec![0; prime_roots.len()];
    let done = prime_roots.is_empty();

    RootIterator {
        prime_roots,
        moduli,
        indices,
        done,
        math_interruption: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
        println!("roots for 1 mod 8: {:?}", roots);
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

        // Large prime and composite
        assert_eq!(verified_is_prime(Uint::from_u128(1_000_000_000_039)), true);
        assert_eq!(
            verified_is_prime(Uint::from_u128(1_000_000_000_039 * 5)),
            false
        );

        // Primes and composites around the 2^64 boundary
        // Prime just below 2^64: 18446744073709551557 (2^64 - 59)
        let prime_under_2_64 = Uint::from_u128(18446744073709551557_u128);
        assert_eq!(verified_is_prime(prime_under_2_64), true);

        // Composite just below 2^64
        let composite_under_2_64 = Uint::from_u128(18446744073709551558_u128);
        assert_eq!(verified_is_prime(composite_under_2_64), false);

        // Prime just above 2^64: 18446744073709551629 (2^64 + 13)
        let prime_over_2_64 = Uint::from_u128(18446744073709551629_u128);
        assert_eq!(verified_is_prime(prime_over_2_64), true);

        // Composite just above 2^64
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
        // This prime candidate n has f^2 <= n-1 < f^3 when factored with the Pocklington/BLS routine
        // f = 14216887496626725604656092894182942316875435081728000
        // n = 2379738973818137587966091241708611381752917711408835932379608374701813936555867387813613052614017647793415905146965240226643968001
        let n_str = "2379738973818137587966091241708611381752917711408835932379608374701813936555867387813613052614017647793415905146965240226643968001";
        let n = Uint::from_str_radix(n_str, 10).unwrap();
        assert!(generate_and_verify_pocklington(n));
        assert!(verified_is_prime(n));
    }

    #[test]
    fn test_sigma_cached_large_prime() {
        // Prime over 2^64: 18446744073709551629 (2^64 + 13)
        let prime_over_2_64 = Uint::from_u128(18446744073709551629_u128);
        let cache = HashMap::new();
        // sigma(p^2) = p^2 + p + 1
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
}
#[test]
fn test_solve_mod_2_k_custom_5() {
    let n = Int::from_u32(17);
    let roots = solve_mod_2_k(n, 5);
    println!("roots for 17 mod 32: {:?}", roots);
    assert_eq!(roots.len(), 4);
}

#[cfg_attr(unverified_build, ignore)]
#[test]
fn test_solve_crt_128bit() {
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
    // x^2 = 2 (mod 7), x = 3 or 4
    // Lift x=3 to mod 49: x^2 = 2 (mod 49). x = 10
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
    // Lift x=3 to mod 343: x^2 = 2 (mod 343). x = 108
    let root = Int::from_u128(3);
    let n = Int::from_u128(2);
    let p = Int::from_u128(7);
    let k = 3;
    let lifted = hensels_lift(root, n, p, k).unwrap();
    assert_eq!(lifted, Int::from_u128(108));
}

#[cfg_attr(unverified_build, ignore)]
#[test]
fn test_hensels_lift_residue_failure() {
    // trigger a residue failure
    // We want `mod_inverse_big(two_r, current_mod)` to return None.
    // two_r = (2 * current_r) % current_mod.
    // If p=2, current_mod = 2^k, two_r is even, so gcd(two_r, current_mod) >= 2.
    // So `mod_inverse_big(two_r, current_mod)` will return `None`.
    let root = Int::from_u128(1);
    let n = Int::from_u128(1);
    let p = Int::from_u128(2);
    let k = 3;
    assert_eq!(hensels_lift(root, n, p, k), None);
}

/// Compute the multiplicative inverse of `a` modulo `m`, if one exists.
///
/// Returns `Some(x)` such that `(a * x) % m == 1` when `gcd(a, m) == 1`; returns `None` if `m <= 1` or no inverse exists.
///
/// # Examples
///
/// ```
/// let a = Uint::from(3u64);
/// let m = Uint::from(11u64);
/// let inv = mod_inverse_u512(a, m).expect("inverse exists");
/// assert_eq!((a * inv) % m, Uint::one());
/// ```
pub fn mod_inverse_u512(a: Uint, m: Uint) -> Option<Uint> {
    if m <= Uint::one() {
        return None;
    }
    crate::lean_ffi::compute_mod_inverse(&a, false, &m)
}

/// Compute the modular negation of `val` modulo `m`.
///
/// If `m == 0`, returns `val` unchanged. Otherwise returns `0` when `val % m == 0`,
/// or `m - (val % m)` for the modular negation in the range `1..m-1`.
///
/// # Examples
///
/// ```
/// let a = mod_negate_big(3u128.into(), 7u128.into());
/// assert_eq!(a, 4u128.into());
///
/// let b = mod_negate_big(8u128.into(), 8u128.into());
/// assert_eq!(b, 0u128.into());
///
/// let c = mod_negate_big(5u128.into(), 0u128.into());
/// assert_eq!(c, 5u128.into());
/// ```
/// Compute the modular negation of `val` modulo `m`.
///
/// Returns the unique value `r` in the range `[0, m)` such that `(val + r) % m == 0`. If `m <= 0`,
/// the input `val` is returned unchanged.
///
/// # Examples
///
/// ```
/// let a = Int::from(3);
/// let m = Int::from(7);
/// assert_eq!(mod_negate_big(a, m), Int::from(4)); // 3 + 4 ≡ 0 (mod 7)
/// ```
pub fn mod_negate_big(val: Int, m: Int) -> Int {
    if m <= Int::zero() {
        return val;
    }
    let mut v = val % m;
    if v < Int::zero() {
        v += m;
    }
    if v == Int::zero() {
        Int::zero()
    } else {
        m - v
    }
}
