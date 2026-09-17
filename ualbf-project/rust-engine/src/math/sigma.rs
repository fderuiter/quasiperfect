use crate::types::Uint;
use crate::types::UintExt;
use std::collections::HashMap;

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
