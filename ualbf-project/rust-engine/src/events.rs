use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SearchEvent {
    Phase {
        phase: u32,
        name: String,
    },
    Progress {
        elapsed_secs: f64,
        branches: usize,
        pruned: usize,
        abundance_pruned: usize,
        branches_per_sec: f64,
    },
    Prefix {
        len: usize,
        factors_str: String,
    },
    Candidate {
        len: usize,
        factors_str: String,
        rem_str: String,
    },
    StatusUpdate {
        c: usize,
        total_weight_scaled: usize,
        comp: usize,
        pr: usize,
        active_str: String,
        prefixes: usize,
        ap: usize,
    },
    DFSComplete {
        total_branches: usize,
        ap: usize,
        rp: usize,
        bp: usize,
    },
    RaycastDeferred {
        rem_str: String,
    },
    Done {
        target_min_log10: u32,
        target_max_log10: u32,
        elapsed_ms: u128,
    },
}
