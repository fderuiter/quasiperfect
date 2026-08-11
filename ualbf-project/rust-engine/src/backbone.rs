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

fn compute_sccs(adj: &[Vec<usize>]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let n = adj.len();
    let mut scc_map = vec![0; n];
    let mut scc_components = Vec::new();
    let mut index = 0;
    let mut indices = vec![None; n];
    let mut lowlink = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut stack = Vec::new();

    fn strongconnect(
        u: usize,
        adj: &[Vec<usize>],
        index: &mut usize,
        indices: &mut [Option<usize>],
        lowlink: &mut [usize],
        on_stack: &mut [bool],
        stack: &mut Vec<usize>,
        scc_map: &mut [usize],
        scc_components: &mut Vec<Vec<usize>>,
    ) {
        indices[u] = Some(*index);
        lowlink[u] = *index;
        *index += 1;
        stack.push(u);
        on_stack[u] = true;

        for &v in &adj[u] {
            if indices[v].is_none() {
                strongconnect(v, adj, index, indices, lowlink, on_stack, stack, scc_map, scc_components);
                lowlink[u] = lowlink[u].min(lowlink[v]);
            } else if on_stack[v] {
                lowlink[u] = lowlink[u].min(indices[v].unwrap());
            }
        }

        if lowlink[u] == indices[u].unwrap() {
            let mut component = Vec::new();
            let scc_id = scc_components.len();
            loop {
                let v = stack.pop().unwrap();
                on_stack[v] = false;
                component.push(v);
                scc_map[v] = scc_id;
                if v == u {
                    break;
                }
            }
            scc_components.push(component);
        }
    }

    for i in 0..n {
        if indices[i].is_none() {
            strongconnect(
                i,
                adj,
                &mut index,
                &mut indices,
                &mut lowlink,
                &mut on_stack,
                &mut stack,
                &mut scc_map,
                &mut scc_components,
            );
        }
    }

    (scc_map, scc_components)
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

        let forced_candidates = crate::cdg::derive_forced_candidates(components);
        let (scc_map, scc_components) = compute_sccs(&forced_candidates);

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
