use crate::lean_ffi::{check_mod_3, check_mod_5, check_mod_9, check_touchard};
use crate::types::Uint;

pub trait Obstruction: Sync + Send {
    /// Check if a prime factor `q` of `sigma(p^{2e})` is forbidden.
    fn check_prime_factor(&self, _q: &Uint) -> bool {
        false
    }

    /// Statically check if a component `p^{2e}` is forbidden based on its base and exponent.
    fn check_component(&self, _p: u64, _two_e: u32) -> bool {
        false
    }
}

pub struct Mod8Obstruction;
impl Obstruction for Mod8Obstruction {
    fn check_prime_factor(&self, q: &Uint) -> bool {
        use crate::residue::IsValidMod8;
        !q.is_valid_mod_8()
    }

    fn check_component(&self, p: u64, two_e: u32) -> bool {
        use crate::residue::IsValidMod8;
        let p_mod = p & 7;
        let mut sum = 0;
        let mut term = 1;
        for _ in 0..=two_e {
            sum = (sum + term) & 7;
            term = (term * p_mod) & 7;
        }
        !sum.is_valid_mod_8()
    }
}

pub struct Mod3Obstruction;
impl Obstruction for Mod3Obstruction {
    /// Evaluates the geometric sum polynomial $\sigma(p^{2e}) = \sum_{k=0}^{2e} p^k \pmod 3$.
    ///
    /// Since for a QPN $\sigma(N) \equiv 1 \pmod 3$, $\sigma(N)$ cannot be a multiple of 3.
    /// Because $\sigma(N) = \prod \sigma(p^{2e})$, no single component can have $\sigma(p^{2e}) \equiv 0 \pmod 3$.
    /// Returns `true` (obstruction) if the sum evaluates to `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ualbf_engine::obstruction::{Obstruction, Mod3Obstruction};
    /// let ob = Mod3Obstruction;
    /// assert_eq!(ob.check_component(7, 4), false);
    /// assert_eq!(ob.check_component(7, 2), true);
    /// ```
    fn check_component(&self, p: u64, two_e: u32) -> bool {
        check_mod_3(p, two_e)
    }
}

pub struct Mod5Obstruction;
impl Obstruction for Mod5Obstruction {
    fn check_component(&self, p: u64, two_e: u32) -> bool {
        check_mod_5(p, two_e)
    }
}

pub struct TouchardObstruction;
impl Obstruction for TouchardObstruction {
    fn check_component(&self, p: u64, two_e: u32) -> bool {
        !check_touchard(p, two_e)
    }
}

pub struct Mod9Obstruction;
impl Obstruction for Mod9Obstruction {
    /// Evaluates the geometric sum polynomial $\sigma(p^{2e}) = \sum_{k=0}^{2e} p^k \pmod 9$.
    ///
    /// Since $\sigma(N) \equiv 1 \pmod 9$, $\sigma(N)$ cannot evaluate to a multiple of 3 (i.e. it cannot be 0, 3, or 6 modulo 9).
    /// If any single component has $\sigma(p^{2e}) \equiv 0, 3, \text{ or } 6 \pmod 9$, it forces the total product $\sigma(N)$ to be a multiple of 3,
    /// violating the strict $\sigma(N) \equiv 1 \pmod 9$ requirement.
    /// Returns `true` (obstruction) if the sum modulo 9 is a multiple of 3 (i.e. `sum % 3 == 0`).
    ///
    /// # Examples
    ///
    /// ```
    /// use ualbf_engine::obstruction::{Obstruction, Mod9Obstruction};
    /// let ob = Mod9Obstruction;
    /// assert_eq!(ob.check_component(13, 8), true);
    /// assert_eq!(ob.check_component(13, 2), true);
    /// ```
    fn check_component(&self, p: u64, two_e: u32) -> bool {
        check_mod_9(p, two_e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UintExt;

    #[test]
    fn test_mod5_abbott_aull_obstruction() {
        let filter = Mod5Obstruction;
        // p = 11 (1 mod 5), 2e = 4 => e = 2.
        // sigma(11^4) = 16105, which is divisible by 5.
        assert!(filter.check_component(11, 4));
        // p = 11, 2e = 2 => e = 1.
        // sigma(11^2) = 133, not divisible by 5.
        assert!(!filter.check_component(11, 2));
    }

    #[test]
    fn test_mod8_obstruction() {
        let filter = Mod8Obstruction;
        assert!(filter.check_prime_factor(&Uint::from_u64(5)));
        assert!(filter.check_prime_factor(&Uint::from_u64(7)));
        assert!(!filter.check_prime_factor(&Uint::from_u64(3)));
        assert!(!filter.check_prime_factor(&Uint::from_u64(17))); // 17 % 8 = 1, valid, so not forbidden
    }

    #[test]
    fn test_mod3_and_mod9_obstructions() {
        let mod3 = Mod3Obstruction;
        let mod9 = Mod9Obstruction;

        assert!(!mod3.check_component(7, 4));
        assert!(mod3.check_component(7, 2));

        assert!(mod9.check_component(13, 8));
        assert!(mod9.check_component(13, 2));
    }

    #[test]
    fn test_touchard_obstruction() {
        let filter = TouchardObstruction;
        // p = 3, 2e = 2 -> sum = 1 + 3 + 9 = 13 (odd, not 9 -> invalid, i.e. obstructed)
        assert!(filter.check_component(3, 2));

        // p = 5, 2e = 2 -> sum = 1 + 5 + 25 = 31 % 24 = 7 (odd, not 9 -> invalid, i.e. obstructed)
        assert!(filter.check_component(5, 2));

        // p = 17, 2e = 2 -> sum = 1 + 17 + 289 = 307 % 24 = 19 (odd, not 9 -> invalid, i.e. obstructed)
        assert!(filter.check_component(17, 2));

        // Let's test reachability of residues modulo 24
        // target final residues are [3, 19]
        // reachable ones are {3, 11, 19}
        assert!(crate::dfs_tree::is_touchard_reachable(3));
        assert!(crate::dfs_tree::is_touchard_reachable(11));
        assert!(crate::dfs_tree::is_touchard_reachable(19));

        assert!(!crate::dfs_tree::is_touchard_reachable(0));
        assert!(!crate::dfs_tree::is_touchard_reachable(5));
        assert!(!crate::dfs_tree::is_touchard_reachable(7));
    }
}

/// Deep Divisibility Chains
/// Recursively verifies that prime factors generated by $\sigma$ propagate their constraints.
/// For Odd Perfect Numbers (OPN), factors must be *present* in the candidate set.
/// For Quasiperfect Numbers (QPN), factors must be *disjoint* from the candidate set.
pub fn verify_deep_divisibility_chain(
    candidate_factors: &[u64],
    sigma_factors: &[u64],
    must_be_present: bool,
) -> bool {
    if must_be_present {
        // Transitive factor inclusion for OPN
        for &sf in sigma_factors {
            if !candidate_factors.contains(&sf) {
                return false;
            }
        }
        true
    } else {
        // Transitive factor exclusion for QPN
        for &sf in sigma_factors {
            if candidate_factors.contains(&sf) {
                return false;
            }
        }
        true
    }
}
