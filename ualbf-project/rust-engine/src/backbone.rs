use crate::types::{PrimePower, Uint, UintExt};
use rayon::prelude::*;
use std::sync::Arc;

pub struct SearchBackbone {
    pub compatibility_matrix: Vec<Vec<u64>>,
    pub min_n_product: Vec<Vec<Uint>>,
    pub num_components: usize,
    pub forced_candidates: Vec<Vec<usize>>,
    pub adjacency: Vec<Vec<usize>>,
    pub scc_map: Vec<usize>,
    pub scc_components: Vec<Vec<usize>>,
}

impl SearchBackbone {
    pub fn new(
        components: &[PrimePower],
        lazy_cache: &Arc<Vec<std::sync::OnceLock<Result<Vec<Uint>, ()>>>>,
    ) -> Self {
        let n = components.len();
        println!("Backbone|DIAG|Building backbone for {} components", n);
        let num_u64 = (n + 63) / 64;

        let pre_resolved_factors: Vec<Vec<u64>> = (0..n)
            .into_par_iter()
            .map(|i| {
                let comp = &components[i];
                let lazy =
                    crate::dfs_tree::resolve_lazy_factors(comp, &lazy_cache[i]).unwrap_or_default();
                let mut sigma = comp.sigma_factors.clone();
                sigma.extend_from_slice(&lazy);
                sigma
                    .iter()
                    .filter_map(|x| {
                        if *x <= Uint::from_u64(u64::MAX) {
                            Some(x.as_u64())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .collect();

        let compatibility_matrix: Vec<Vec<u64>> = (0..n)
            .into_par_iter()
            .map(|i| {
                let mut row = vec![0u64; num_u64];
                let comp_i = &components[i];
                let sigma_i_u64 = &pre_resolved_factors[i];

                for j in 0..n {
                    let comp_j = &components[j];
                    let sigma_j_u64 = &pre_resolved_factors[j];

                    let mut compatible = true;
                    if comp_i.p == comp_j.p {
                        compatible = false;
                    } else if sigma_i_u64.contains(&comp_j.p) {
                        compatible = false;
                    } else if sigma_j_u64.contains(&comp_i.p) {
                        compatible = false;
                    }

                    if compatible {
                        row[j / 64] |= 1 << (j % 64);
                    }
                }
                row
            })
            .collect();

        let min_n_product: Vec<Vec<Uint>> = (0..n)
            .into_par_iter()
            .map(|i| {
                let mut product = Uint::one();
                let mut count = 0;
                let mut last_p = 0;
                let mut products = Vec::new();

                for j in i..n {
                    let comp = &components[j];
                    if comp.p != last_p {
                        if let Some(next_p) = product.checked_mul(comp.val) {
                            product = next_p;
                            last_p = comp.p;
                            products.push(product);
                            count += 1;
                            if count >= n {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                products
            })
            .collect();

        let cdg = crate::cdg::Cdg::new(components);

        Self {
            compatibility_matrix,
            min_n_product,
            num_components: n,
            forced_candidates: cdg.forced_candidates,
            adjacency: cdg.adjacency,
            scc_map: cdg.scc_map,
            scc_components: cdg.scc_components,
        }
    }

    pub fn max_allowed_factors(
        &self,
        start_idx: usize,
        current_n: Uint,
        target_bound: Uint,
    ) -> usize {
        if start_idx >= self.num_components {
            return 0;
        }
        let products = &self.min_n_product[start_idx];
        let mut max_allowed = 0;
        for (i, &p) in products.iter().enumerate() {
            if let Some(next_n) = current_n.checked_mul(p) {
                if next_n <= target_bound {
                    max_allowed = i + 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        max_allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    #[test]
    fn test_backbone_new_empty_inputs() {
        let components: Vec<PrimePower> = vec![];
        let lazy_cache: Arc<Vec<OnceLock<Result<Vec<Uint>, ()>>>> = Arc::new(vec![]);

        let backbone = SearchBackbone::new(&components, &lazy_cache);

        assert_eq!(backbone.num_components, 0);
        assert!(backbone.compatibility_matrix.is_empty());
        assert!(backbone.min_n_product.is_empty());

        assert_eq!(
            backbone.max_allowed_factors(0, Uint::one(), Uint::from_u64(1000)),
            0
        );
        assert_eq!(
            backbone.max_allowed_factors(5, Uint::one(), Uint::from_u64(1000)),
            0
        );
    }

    #[test]
    fn test_backbone_multi_component_pre_resolved_and_compatibility() {
        // Create 3 components:
        // comp0: p = 3, val = 9, sigma_factors = [13]
        // comp1: p = 5, val = 25, sigma_factors = [31]
        // comp2: p = 13, val = 169, sigma_factors = [3, 61]
        let comp0 = PrimePower {
            p: 3,
            two_e: 2,
            val: Uint::from_u64(9),
            sigma: Uint::from_u64(13),
            sigma_factors: vec![Uint::from_u64(13)],
            needs_rho: vec![],
            abundance_fp: 0,
        };
        let comp1 = PrimePower {
            p: 5,
            two_e: 2,
            val: Uint::from_u64(25),
            sigma: Uint::from_u64(31),
            sigma_factors: vec![Uint::from_u64(31)],
            needs_rho: vec![],
            abundance_fp: 0,
        };
        let comp2 = PrimePower {
            p: 13,
            two_e: 2,
            val: Uint::from_u64(169),
            sigma: Uint::from_u64(183),
            sigma_factors: vec![Uint::from_u64(3), Uint::from_u64(61)],
            needs_rho: vec![],
            abundance_fp: 0,
        };

        let components = vec![comp0, comp1, comp2];

        // Set up lazy cache: comp0 lazy cache includes 7
        let lock0 = OnceLock::new();
        let _ = lock0.set(Ok(vec![Uint::from_u64(7)]));
        let lock1 = OnceLock::new();
        let _ = lock1.set(Ok(vec![]));
        let lock2 = OnceLock::new();
        let _ = lock2.set(Ok(vec![]));

        let lazy_cache = Arc::new(vec![lock0, lock1, lock2]);

        let backbone = SearchBackbone::new(&components, &lazy_cache);

        assert_eq!(backbone.num_components, 3);
        assert_eq!(backbone.compatibility_matrix.len(), 3);

        // Check compatibility matrix bits:
        // comp0 (p=3) & comp1 (p=5): compatible -> bit 1 in row 0 is 1
        assert_ne!(backbone.compatibility_matrix[0][0] & (1 << 1), 0);
        // comp0 (sigma_factors contains 13) & comp2 (p=13): INcompatible -> bit 2 in row 0 is 0
        assert_eq!(backbone.compatibility_matrix[0][0] & (1 << 2), 0);
        // comp2 (sigma_factors contains 3) & comp0 (p=3): INcompatible -> bit 0 in row 2 is 0
        assert_eq!(backbone.compatibility_matrix[2][0] & (1 << 0), 0);
    }

    #[test]
    fn test_backbone_max_allowed_factors() {
        let comp0 = PrimePower {
            p: 3,
            two_e: 2,
            val: Uint::from_u64(9),
            sigma: Uint::from_u64(13),
            sigma_factors: vec![Uint::from_u64(13)],
            needs_rho: vec![],
            abundance_fp: 0,
        };
        let comp1 = PrimePower {
            p: 5,
            two_e: 2,
            val: Uint::from_u64(25),
            sigma: Uint::from_u64(31),
            sigma_factors: vec![Uint::from_u64(31)],
            needs_rho: vec![],
            abundance_fp: 0,
        };
        let comp2 = PrimePower {
            p: 7,
            two_e: 2,
            val: Uint::from_u64(49),
            sigma: Uint::from_u64(57),
            sigma_factors: vec![Uint::from_u64(19)],
            needs_rho: vec![],
            abundance_fp: 0,
        };

        let components = vec![comp0, comp1, comp2];
        let lazy_cache = Arc::new(vec![OnceLock::new(), OnceLock::new(), OnceLock::new()]);

        let backbone = SearchBackbone::new(&components, &lazy_cache);

        // min_n_product[0]: [9, 225, 11025]
        // 1 * 9 = 9 <= 100 -> 1 factor
        // 1 * 225 = 225 > 100 -> stop -> 1 factor
        assert_eq!(
            backbone.max_allowed_factors(0, Uint::one(), Uint::from_u64(100)),
            1
        );

        // 1 * 225 <= 1000 -> 2 factors
        // 1 * 11025 > 1000 -> stop -> 2 factors
        assert_eq!(
            backbone.max_allowed_factors(0, Uint::one(), Uint::from_u64(1000)),
            2
        );

        // Target bound 20000 -> all 3 factors allowed
        assert_eq!(
            backbone.max_allowed_factors(0, Uint::one(), Uint::from_u64(20000)),
            3
        );

        // current_n = 50, target = 1000
        // 50 * 9 = 450 <= 1000
        // 50 * 225 = 11250 > 1000 -> 1 factor
        assert_eq!(
            backbone.max_allowed_factors(0, Uint::from_u64(50), Uint::from_u64(1000)),
            1
        );

        // start_idx out of bounds
        assert_eq!(
            backbone.max_allowed_factors(3, Uint::one(), Uint::from_u64(1000)),
            0
        );

        // Overflow boundary condition: current_n large enough to cause overflow
        let huge_n = Uint::MAX;
        assert_eq!(
            backbone.max_allowed_factors(0, huge_n, Uint::from_u64(1000)),
            0
        );
    }
}
