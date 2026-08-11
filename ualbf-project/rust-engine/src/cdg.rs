use crate::types::PrimePower;
use std::collections::HashMap;

/// Enumerates every divisor d > 1 of two_e + 1.
/// Use the for d in 1..n { if n % d == 0 } trial-loop idiom.
pub fn get_divisors_greater_than_one(n: u32) -> Vec<u32> {
    let mut divisors = Vec::new();
    if n <= 1 {
        return divisors;
    }
    // Trial-loop idiom as requested
    for d in 1..n {
        if n % d == 0 {
            if d > 1 {
                divisors.push(d);
            }
            let other = n / d;
            if other > 1 {
                divisors.push(other);
            }
        }
    }
    divisors.sort_unstable();
    divisors.dedup();
    divisors
}

/// Derives the forced-candidate sets for each component based on Zsigmondy divisors.
///
/// For each component and each divisor d > 1 of (two_e + 1), we collect all component
/// indices whose prime satisfies p % d == 1.
///
/// Exposes the per-component forced-candidate sets as Vec<Vec<usize>>, indexed by component position.
pub fn derive_forced_candidates(components: &[PrimePower]) -> Vec<Vec<usize>> {
    let mut forced_candidates = vec![Vec::new(); components.len()];

    // Build a prime-to-component-index HashMap<u64, Vec<usize>>
    let mut prime_to_component_indices: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, comp) in components.iter().enumerate() {
        prime_to_component_indices
            .entry(comp.p)
            .or_insert_with(Vec::new)
            .push(idx);
    }

    // For each component position...
    for i in 0..components.len() {
        let comp = &components[i];
        let n = comp.two_e + 1;
        let divisors = get_divisors_greater_than_one(n);

        let mut candidates = Vec::new();
        for d in divisors {
            let d_u64 = d as u64;
            // Collect component indices whose prime satisfies p % d == 1
            for (&prime, indices) in &prime_to_component_indices {
                if prime % d_u64 == 1 {
                    candidates.extend(indices.iter().copied());
                }
            }
        }

        candidates.sort_unstable();
        candidates.dedup();
        forced_candidates[i] = candidates;
    }

    forced_candidates
}

/// Computes strongly connected components (SCC) of a directed graph.
/// Implementing iterative Tarjan's SCC using an explicit work stack.
pub fn compute_sccs(adj: &[Vec<usize>]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let n = adj.len();
    let mut indices = vec![None; n];
    let mut lowlink = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut tarjan_stack = Vec::new();
    let mut scc_map = vec![0; n];
    let mut scc_components = Vec::new();
    let mut next_index = 0;

    for start in 0..n {
        if indices[start].is_none() {
            let mut dfs_stack = vec![(start, 0)];

            indices[start] = Some(next_index);
            lowlink[start] = next_index;
            next_index += 1;
            tarjan_stack.push(start);
            on_stack[start] = true;

            while let Some(&(u, neighbor_idx)) = dfs_stack.last() {
                let neighbors = &adj[u];
                if neighbor_idx < neighbors.len() {
                    let v = neighbors[neighbor_idx];
                    dfs_stack.last_mut().unwrap().1 += 1;

                    if indices[v].is_none() {
                        indices[v] = Some(next_index);
                        lowlink[v] = next_index;
                        next_index += 1;
                        tarjan_stack.push(v);
                        on_stack[v] = true;

                        dfs_stack.push((v, 0));
                    } else if on_stack[v] {
                        lowlink[u] = lowlink[u].min(indices[v].unwrap());
                    }
                } else {
                    dfs_stack.pop();

                    if let Some(&(parent, _)) = dfs_stack.last() {
                        lowlink[parent] = lowlink[parent].min(lowlink[u]);
                    }

                    if lowlink[u] == indices[u].unwrap() {
                        let mut component = Vec::new();
                        let scc_id = scc_components.len();
                        while let Some(v) = tarjan_stack.pop() {
                            on_stack[v] = false;
                            component.push(v);
                            scc_map[v] = scc_id;
                            if v == u {
                                break;
                            }
                        }
                        component.sort_unstable();
                        scc_components.push(component);
                    }
                }
            }
        }
    }

    (scc_map, scc_components)
}

pub struct Cdg {
    pub forced_candidates: Vec<Vec<usize>>,
    pub adjacency_list: Vec<Vec<usize>>,
    pub scc_map: Vec<usize>,
    pub scc_components: Vec<Vec<usize>>,
}

impl Cdg {
    pub fn new(components: &[PrimePower]) -> Self {
        let forced_candidates = derive_forced_candidates(components);
        let adjacency_list = forced_candidates.clone();
        let (scc_map, scc_components) = compute_sccs(&adjacency_list);

        Self {
            forced_candidates,
            adjacency_list,
            scc_map,
            scc_components,
        }
    }
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

        let cdg = Cdg::new(&components);

        // For C0: d=3. Primes satisfying p % 3 == 1 are:
        // - C1: 7 % 3 = 1
        // - C2: 13 % 3 = 1
        // So C0 candidates should be [1, 2]
        assert_eq!(cdg.forced_candidates[0], vec![1, 2]);

        // For C1: d=2. Primes satisfying p % 2 == 1 are:
        // - C0: 3 % 2 = 1
        // - C1: 7 % 2 = 1
        // - C2: 13 % 2 = 1
        // So C1 candidates should be [0, 1, 2]
        assert_eq!(cdg.forced_candidates[1], vec![0, 1, 2]);

        // For C2: d=2. Candidates should also be [0, 1, 2]
        assert_eq!(cdg.forced_candidates[2], vec![0, 1, 2]);
    }

    #[test]
    fn test_scc_grouping() {
        // Build an adjacency list representing:
        // 0 -> 1
        // 1 -> 2
        // 2 -> 0 (making {0, 1, 2} an SCC)
        // 3 -> 2 (pointing into the SCC, but not reachable from it)
        let adj = vec![vec![1], vec![2], vec![0], vec![2]];

        let (scc_map, scc_components) = compute_sccs(&adj);

        // There should be 2 SCCs: {0, 1, 2} and {3}
        assert_eq!(scc_components.len(), 2);
        // Let's verify each node maps to its correct SCC
        assert_eq!(scc_map[0], scc_map[1]);
        assert_eq!(scc_map[0], scc_map[2]);
        assert_ne!(scc_map[0], scc_map[3]);

        // Verify members
        let mut sorted_components = scc_components.clone();
        sorted_components.sort_by_key(|c| c.len());
        assert_eq!(sorted_components[0], vec![3]);
        assert_eq!(sorted_components[1], vec![0, 1, 2]);
    }

    #[test]
    fn test_empty_components() {
        let components: Vec<PrimePower> = vec![];
        let cdg = Cdg::new(&components);
        assert!(cdg.forced_candidates.is_empty());
        assert!(cdg.scc_map.is_empty());
        assert!(cdg.scc_components.is_empty());
    }
}
