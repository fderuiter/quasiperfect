use crate::types::{Int, Uint};
use crate::types::{IntExt, UintExt};
use std::collections::HashMap;

/// Computes the product of `a` and `b` modulo `m` without overflowing `u128`.
pub fn mul_mod_u128(a: u128, b: u128, m: u128) -> u128 {
    assert!(m > 0, "m must be greater than 0");
    ((a % m) * (b % m)) % m
}

/// Compute modular exponentiation: base^exp modulo m.
pub fn pow_mod_u128(mut base: u128, mut exp: u128, m: u128) -> u128 {
    if m == 1 {
        return 0;
    }
    let mut res = 1u128;
    base %= m;
    while exp > 0 {
        if exp % 2 == 1 {
            res = mul_mod_u128(res, base, m);
        }
        base = mul_mod_u128(base, base, m);
        exp /= 2;
    }
    res
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

    let a_1024 = <bnum::types::U1024 as bnum::cast::CastFrom<Uint>>::cast_from(a);
    let b_1024 = <bnum::types::U1024 as bnum::cast::CastFrom<Uint>>::cast_from(b);
    let m_1024 = <bnum::types::U1024 as bnum::cast::CastFrom<Uint>>::cast_from(m);
    let res_1024 = (a_1024 * b_1024) % m_1024;
    <Uint as bnum::cast::CastFrom<bnum::types::U1024>>::cast_from(res_1024)
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

pub fn gcd_u256(mut a: Uint, mut b: Uint) -> Uint {
    while b != Uint::zero() {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

pub fn mod_inverse_big(a: Int, m: Int) -> Option<Int> {
    if m <= Int::zero() {
        return None;
    }

    let a_neg = a < Int::zero();
    let a_abs = if a_neg { -a } else { a }.as_uint();

    crate::lean_ffi::compute_mod_inverse(&a_abs, a_neg, &m.as_uint()).map(|x| x.as_int())
}

pub fn mod_inverse_u512(a: Uint, m: Uint) -> Option<Uint> {
    if m <= Uint::one() {
        return None;
    }
    crate::lean_ffi::compute_mod_inverse(&a, false, &m)
}

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
