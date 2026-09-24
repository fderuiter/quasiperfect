use crate::math::modular::{add_mod_u256, gcd_u256, modpow_u256, mul_mod_u256};
use crate::types::Uint;
use crate::types::UintExt;
use prime_factorization::Factorization;
use serde::{Deserialize, Serialize};
use std::panic::catch_unwind;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CompositenessWitness {
    pub candidate: u64,
    pub witness_type: String,
    pub witness: u64,
}

static COMPOSITENESS_WITNESSES: Mutex<Vec<CompositenessWitness>> = Mutex::new(Vec::new());

pub fn clear_compositeness_witnesses() {
    if let Ok(mut lock) = COMPOSITENESS_WITNESSES.lock() {
        lock.clear();
    }
}

pub fn get_compositeness_witnesses() -> Vec<CompositenessWitness> {
    if let Ok(lock) = COMPOSITENESS_WITNESSES.lock() {
        lock.clone()
    } else {
        Vec::new()
    }
}

pub fn add_compositeness_witness(witness: CompositenessWitness) {
    if let Ok(mut lock) = COMPOSITENESS_WITNESSES.lock() {
        lock.push(witness);
    }
}

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
    pub fn is_complete(&self) -> bool {
        matches!(self, FactorizationResult::Complete(_))
    }

    pub fn factors(&self) -> &[Uint] {
        match self {
            FactorizationResult::Complete(f) => f.as_slice(),
            FactorizationResult::Partial { known_factors, .. } => known_factors.as_slice(),
            FactorizationResult::Failure(_) => &[],
        }
    }
}

pub struct TrialSieve {
    pub small_primes: Vec<u64>,
}

impl TrialSieve {
    pub fn new(limit: u64) -> Self {
        let sieve = primal::Sieve::new(limit as usize);
        let small_primes: Vec<u64> = sieve.primes_from(2).map(|p| p as u64).collect();
        TrialSieve { small_primes }
    }

    pub fn trial_factor_only(&self, mut n: Uint) -> (smallvec::SmallVec<[Uint; 8]>, Uint) {
        if n <= Uint::one() {
            return (smallvec::SmallVec::new(), Uint::one());
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
                (factors, Uint::one())
            } else {
                (factors, n)
            }
        } else {
            (factors, Uint::one())
        }
    }

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
                let mut v: smallvec::SmallVec<[Uint; 8]> = fact
                    .factors
                    .into_iter()
                    .map(|f| Uint::from_u128((f) as u128))
                    .collect();
                v.sort_unstable();
                FactorizationResult::Complete(v)
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
    for c in 1..5u32 {
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

pub fn find_compositeness_witness(n: Uint) -> Option<CompositenessWitness> {
    let threshold = Uint::from_u128(1_u128 << 64);
    if n <= Uint::one() || n >= threshold {
        return None;
    }
    let n_u64 = n.as_u128() as u64;

    let bases_12: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    for &b in &bases_12 {
        if n_u64 == b {
            return None;
        }
        if n_u64 % b == 0 {
            return Some(CompositenessWitness {
                candidate: n_u64,
                witness_type: "divisor".to_string(),
                witness: b,
            });
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
                return Some(CompositenessWitness {
                    candidate: n_u64,
                    witness_type: "miller_rabin".to_string(),
                    witness: base,
                });
            }
            if x == n - Uint::one() {
                composite = false;
                break;
            }
        }
        if composite {
            return Some(CompositenessWitness {
                candidate: n_u64,
                witness_type: "miller_rabin".to_string(),
                witness: base,
            });
        }
    }

    None
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
                    FactorizationResult::Complete(facs) => facs,
                    FactorizationResult::Partial { known_factors, .. } => known_factors,
                    FactorizationResult::Failure(_) => smallvec::SmallVec::new(),
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

    let threshold = Uint::from_u128(1_u128 << 64);
    if n < threshold {
        if let Some(witness) = find_compositeness_witness(n) {
            add_compositeness_witness(witness);
            return false;
        } else {
            return generate_and_verify_pocklington(n);
        }
    }

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

    generate_and_verify_pocklington(n)
}

pub fn quick_factor_u256(n: Uint) -> FactorizationResult {
    if n <= Uint::one() {
        return FactorizationResult::Complete(smallvec::SmallVec::new());
    }

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
            match rho_factor_u256(remaining) {
                FactorizationResult::Complete(v) => factors.extend(v),
                FactorizationResult::Partial {
                    known_factors,
                    remaining: rem,
                } => {
                    factors.extend(known_factors);
                    factors.sort_unstable();
                    return FactorizationResult::Partial {
                        known_factors: factors,
                        remaining: rem,
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

pub fn small_divisors_pub(n: u32) -> Vec<u32> {
    if n == 0 {
        return Vec::new();
    }
    let mut divisors = Vec::new();
    let limit = (n as f64).sqrt() as u32;
    for i in 1..=limit {
        if n % i == 0 {
            divisors.push(i);
            if i * i != n {
                divisors.push(n / i);
            }
        }
    }
    divisors.sort_unstable();
    divisors
}

pub fn factor_sigma_cyclotomic(p: u64, two_e: u32) -> FactorizationResult {
    if two_e == 0 {
        let full_sigma = match crate::lean_ffi::compute_sigma_checked(p, two_e) {
            Some(s) => s,
            None => return FactorizationResult::Failure(Uint::zero()),
        };
        return quick_factor_u256(full_sigma);
    }

    let n = match two_e.checked_add(1) {
        Some(v) => v,
        None => {
            let full_sigma = match crate::lean_ffi::compute_sigma_checked(p, two_e) {
                Some(s) => s,
                None => return FactorizationResult::Failure(Uint::zero()),
            };
            return quick_factor_u256(full_sigma);
        }
    };

    let divisors = small_divisors_pub(n);

    let mut evals = Vec::new();
    let mut fallback = false;

    for &d in &divisors {
        if d > 1 {
            match crate::lean_ffi::native_cyclotomic_eval(d, &Uint::from_u64(p)) {
                Some(v) => evals.push(v),
                None => {
                    fallback = true;
                    break;
                }
            }
        }
    }

    if fallback {
        let full_sigma = match crate::lean_ffi::compute_sigma_checked(p, two_e) {
            Some(s) => s,
            None => return FactorizationResult::Failure(Uint::zero()),
        };
        return quick_factor_u256(full_sigma);
    }

    let mut all_known_factors = smallvec::SmallVec::<[Uint; 8]>::new();
    let mut remaining_product = Uint::one();
    let mut any_partial = false;

    for v in evals {
        if v <= Uint::one() {
            continue;
        }
        match quick_factor_u256(v) {
            FactorizationResult::Complete(factors) => {
                all_known_factors.extend(factors);
            }
            FactorizationResult::Partial {
                known_factors,
                remaining,
            } => {
                all_known_factors.extend(known_factors);
                remaining_product *= remaining;
                any_partial = true;
            }
            FactorizationResult::Failure(u) => {
                remaining_product *= u;
                any_partial = true;
            }
        }
    }

    all_known_factors.sort_unstable();

    if any_partial {
        FactorizationResult::Partial {
            known_factors: all_known_factors,
            remaining: remaining_product,
        }
    } else {
        FactorizationResult::Complete(all_known_factors)
    }
}
