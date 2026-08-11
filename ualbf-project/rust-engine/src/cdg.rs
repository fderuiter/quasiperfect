use crate::types::PrimePower;
use std::collections::HashMap;

/// Enumerates every divisor d > 1 of two_e + 1.
pub fn get_divisors_greater_than_one(n: u32) -> Vec<u32> {
    let mut divisors = Vec::new();
    if n <= 1 {
        return divisors;
    }
    let limit = (n as f64).sqrt() as u32;
    for d in 1..=limit {
        if n % d == 0 {
            if d > 1 {
                divisors.push(d);
            }
            let other = n / d;
            if other != d && other > 1 {
                divisors.push(other);
            }
        }
    }
    divisors.sort_unstable();
    divisors
}

/// Derives the forced-candidate sets for each component based on Zsigmondy divisors.
///
/// For each component and each divisor d > 1 of (two_e + 1), we collect all component
/// indices whose prime satisfies p % d == 1.
///
/// Exposes the forced-candidate sets as Vec<Vec<usize>>, indexed by component position.
pub fn derive_forced_candidates(components: &[PrimePower]) -> Vec<Vec<usize>> {
    let mut forced_candidates = vec![Vec::new(); components.len()];

    // Build a prime-to-component-index HashMap<u64, Vec<usize>>
    let mut prime_to_component_indices: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, comp) in components.iter().enumerate() {
        prime_to_component_indices
            .entry(comp.p)
            .or_default()
            .push(idx);
    }

    // For each component position...
    for i in 0..components.len() {
        let comp = &components[i];
        let n = comp.two_e + 1;
        let divisors = get_divisors_greater_than_one(n);

        let mut candidates = Vec::new();
        for &d in &divisors {
            let d_u64 = d as u64;
            // Collect component indices whose prime satisfies p % d == 1
            for (&prime, indices) in &prime_to_component_indices {
                if prime % d_u64 == 1 {
                    candidates.extend_from_slice(indices);
                }
            }
        }

        candidates.sort_unstable();
        candidates.dedup();
        forced_candidates[i] = candidates;
    }

    forced_candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Uint;

    fn make_dummy_component(p: u64, two_e: u32) -> PrimePower {
        use crate::types::UintExt;
        PrimePower {
            p,
            two_e,
            val: Uint::zero(),
            sigma: Uint::zero(),
            sigma_factors: vec![],
            needs_rho: vec![],
            abundance_fp: 0,
        }
    }

    #[test]
    fn test_divisor_enumeration() {
        assert_eq!(get_divisors_greater_than_one(1), Vec::<u32>::new());
        assert_eq!(get_divisors_greater_than_one(2), vec![2]);
        assert_eq!(get_divisors_greater_than_one(3), vec![3]);
        assert_eq!(get_divisors_greater_than_one(4), vec![2, 4]);
        assert_eq!(get_divisors_greater_than_one(9), vec![3, 9]);
        assert_eq!(get_divisors_greater_than_one(12), vec![2, 3, 4, 6, 12]);
    }

    #[test]
    fn test_forced_candidate_mapping() {
        // Components:
        // C0: p=3, two_e=2 (two_e+1 = 3 -> d=3)
        // C1: p=7, two_e=1 (two_e+1 = 2 -> d=2)
        // C2: p=13, two_e=1 (two_e+1 = 2 -> d=2)
        let components = vec![
            make_dummy_component(3, 2),
            make_dummy_component(7, 1),
            make_dummy_component(13, 1),
        ];

        let forced = derive_forced_candidates(&components);

        // For C0: d=3. Primes satisfying p % 3 == 1 are:
        // - C1: 7 % 3 = 1
        // - C2: 13 % 3 = 1
        // So C0 candidates should be [1, 2]
        assert_eq!(forced[0], vec![1, 2]);

        // For C1: d=2. Primes satisfying p % 2 == 1 are:
        // - C0: 3 % 2 = 1
        // - C1: 7 % 2 = 1
        // - C2: 13 % 2 = 1
        // So C1 candidates should be [0, 1, 2]
        assert_eq!(forced[1], vec![0, 1, 2]);

        // For C2: d=2. Candidates should also be [0, 1, 2]
        assert_eq!(forced[2], vec![0, 1, 2]);
    }

    #[test]
    fn test_empty_components() {
        let components: Vec<PrimePower> = vec![];
        let forced = derive_forced_candidates(&components);
        assert!(forced.is_empty());
    }
}
