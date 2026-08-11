use crate::types::{PrimePower, Uint, UintExt};
use rayon::prelude::*;
use std::sync::Arc;

pub struct SearchBackbone {
    pub compatibility_matrix: Vec<Vec<u64>>,
    pub min_n_product: Vec<Vec<Uint>>,
    pub num_components: usize,
    pub forced_candidates: Vec<Vec<usize>>,
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

        let results: Vec<(Vec<u64>, Vec<usize>)> = (0..n)
            .into_par_iter()
            .map(|i| {
                let mut row = vec![0u64; num_u64];
                let comp_i = &components[i];
                let sigma_i_u64 = &pre_resolved_factors[i];
                let divisors = crate::cdg::get_divisors_greater_than_one(comp_i.two_e + 1);

                let mut forced_candidates_i = Vec::new();

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

                    for &d in &divisors {
                        if comp_j.p % (d as u64) == 1 {
                            forced_candidates_i.push(j);
                            break;
                        }
                    }
                }

                forced_candidates_i.sort_unstable();
                forced_candidates_i.dedup();

                (row, forced_candidates_i)
            })
            .collect();

        let mut compatibility_matrix = Vec::with_capacity(n);
        let mut forced_candidates = Vec::with_capacity(n);
        for (row, candidates) in results {
            compatibility_matrix.push(row);
            forced_candidates.push(candidates);
        }

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

        let (scc_map, scc_components) = crate::cdg::compute_sccs(&forced_candidates);

        Self {
            compatibility_matrix,
            min_n_product,
            num_components: n,
            forced_candidates,
            scc_map,
            scc_components,
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
    use crate::types::{PrimePower, Uint, UintExt};
    use std::sync::Arc;

    fn make_dummy_component(p: u64, two_e: u32, val: u64) -> PrimePower {
        PrimePower {
            p,
            two_e,
            val: Uint::from_u64(val),
            sigma: Uint::zero(),
            sigma_factors: vec![],
            needs_rho: vec![],
            abundance_fp: 0,
        }
    }

    #[test]
    fn test_backbone_forced_candidates() {
        let components = vec![
            make_dummy_component(3, 2, 9),
            make_dummy_component(7, 1, 7),
            make_dummy_component(13, 1, 13),
        ];
        let lazy_cache = Arc::new(vec![
            std::sync::OnceLock::new(),
            std::sync::OnceLock::new(),
            std::sync::OnceLock::new(),
        ]);
        let backbone = SearchBackbone::new(&components, &lazy_cache);

        // Verify the O(n2) loop computed equivalent forced_candidates and scc_map
        assert_eq!(backbone.forced_candidates[0], vec![1, 2]);
        assert_eq!(backbone.forced_candidates[1], vec![0, 1, 2]);
        assert_eq!(backbone.forced_candidates[2], vec![0, 1, 2]);

        assert_eq!(backbone.scc_map[0], backbone.scc_map[1]);
        assert_eq!(backbone.scc_map[1], backbone.scc_map[2]);
    }
}
