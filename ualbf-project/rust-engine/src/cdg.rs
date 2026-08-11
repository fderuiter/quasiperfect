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

/// The Component Dependency Graph (CDG) representing directed force relationships
/// and its strongly connected components (SCCs).
pub struct Cdg {
    /// The forced-candidate sets derived from each component.
    pub forced_candidates: Vec<Vec<usize>>,
    /// Directed adjacency-list graph representing component forces.
    pub adjacency: Vec<Vec<usize>>,
    /// Maps each component index to its corresponding SCC ID.
    pub scc_map: Vec<usize>,
    /// Member lists for each SCC.
    pub scc_components: Vec<Vec<usize>>,
}

struct Frame {
    u: usize,
    neighbor_idx: usize,
}

fn compute_sccs_iterative(adj: &[Vec<usize>]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let n = adj.len();
    let mut scc_map = vec![0; n];
    let mut scc_components = Vec::new();
    let mut index = 0;
    let mut indices = vec![None; n];
    let mut lowlink = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut tarjan_stack = Vec::new();
    let mut dfs_stack = Vec::new();

    for root in 0..n {
        if indices[root].is_none() {
            indices[root] = Some(index);
            lowlink[root] = index;
            index += 1;
            tarjan_stack.push(root);
            on_stack[root] = true;
            dfs_stack.push(Frame {
                u: root,
                neighbor_idx: 0,
            });

            while let Some(frame) = dfs_stack.last_mut() {
                let u = frame.u;
                let neighbor_idx = frame.neighbor_idx;

                if neighbor_idx < adj[u].len() {
                    let v = adj[u][neighbor_idx];
                    frame.neighbor_idx += 1;

                    if indices[v].is_none() {
                        indices[v] = Some(index);
                        lowlink[v] = index;
                        index += 1;
                        tarjan_stack.push(v);
                        on_stack[v] = true;
                        dfs_stack.push(Frame {
                            u: v,
                            neighbor_idx: 0,
                        });
                    } else if on_stack[v] {
                        lowlink[u] = lowlink[u].min(indices[v].unwrap());
                    }
                } else {
                    dfs_stack.pop();

                    if let Some(parent_frame) = dfs_stack.last_mut() {
                        let p = parent_frame.u;
                        lowlink[p] = lowlink[p].min(lowlink[u]);
                    }

                    if lowlink[u] == indices[u].unwrap() {
                        let mut component = Vec::new();
                        let scc_id = scc_components.len();
                        loop {
                            let v = tarjan_stack.pop().unwrap();
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
            }
        }
    }

    (scc_map, scc_components)
}

impl Cdg {
    /// Builds a new CDG from components, computing adjacency and iterative Tarjan SCCs.
    pub fn new(components: &[PrimePower]) -> Self {
        let forced_candidates = derive_forced_candidates(components);
        let adjacency = forced_candidates.clone();
        let (scc_map, scc_components) = compute_sccs_iterative(&adjacency);

        Self {
            forced_candidates,
            adjacency,
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

    #[test]
    fn test_cdg_scc_and_iterative_tarjan() {
        // C0: p=3, two_e=2 => d=3 => forces p % 3 == 1 (C1: p=7 % 3 == 1)
        // C1: p=7, two_e=1 => d=2 => forces p % 2 == 1 (C0: p=3, C1: p=7, C2: p=5)
        // C2: p=5, two_e=4 => d=5 => forces p % 5 == 1 (none of C0, C1, C2)
        let components = vec![
            make_dummy_component(3, 2),
            make_dummy_component(7, 1),
            make_dummy_component(5, 4),
        ];

        let cdg = Cdg::new(&components);

        // Verify adjacency-list representation of directed edges
        assert_eq!(cdg.adjacency[0], vec![1]);
        assert_eq!(cdg.adjacency[1], vec![0, 1, 2]);
        assert_eq!(cdg.adjacency[2], Vec::<usize>::new());

        // Verify that every component is assigned to exactly one SCC
        assert_eq!(cdg.scc_map.len(), 3);

        // Verify multi-node SCC: C0 and C1 are mutually dependent, so they must have the same scc_id
        let scc0 = cdg.scc_map[0];
        let scc1 = cdg.scc_map[1];
        let scc2 = cdg.scc_map[2];

        assert_eq!(scc0, scc1, "C0 and C1 must be in the same SCC");
        assert_ne!(scc0, scc2, "C2 must be in a different SCC from C0 and C1");

        // Verify that SCC member lists and lookup array agree
        for i in 0..components.len() {
            let scc_id = cdg.scc_map[i];
            assert!(scc_id < cdg.scc_components.len(), "SCC ID is out of bounds");
            assert!(
                cdg.scc_components[scc_id].contains(&i),
                "SCC member lists and lookup array do not agree for component {}",
                i
            );
        }

        // Verify scc_components sizes and contents
        assert_eq!(cdg.scc_components.len(), 2);
        let scc_of_c0_c1 = &cdg.scc_components[scc0];
        assert_eq!(scc_of_c0_c1.len(), 2);
        assert!(scc_of_c0_c1.contains(&0));
        assert!(scc_of_c0_c1.contains(&1));

        let scc_of_c2 = &cdg.scc_components[scc2];
        assert_eq!(scc_of_c2.len(), 1);
        assert!(scc_of_c2.contains(&2));
    }
}
