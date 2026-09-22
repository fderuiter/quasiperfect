use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "event", rename_all = "snake_case")]
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
    Overflow {
        p: String,
        pow: u32,
    },
    Done {
        target_min_log10: u32,
        target_max_log10: u32,
        elapsed_ms: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_roundtrip(event: SearchEvent) {
        let json = serde_json::to_string(&event).expect("Failed to serialize SearchEvent");
        let deserialized: SearchEvent =
            serde_json::from_str(&json).expect("Failed to deserialize SearchEvent");
        let reserialized =
            serde_json::to_string(&deserialized).expect("Failed to re-serialize SearchEvent");

        assert_eq!(json, reserialized);
        assert_eq!(format!("{:?}", event), format!("{:?}", deserialized));
    }

    #[test]
    fn test_search_event_phase_roundtrip() {
        let event = SearchEvent::Phase {
            phase: 1,
            name: "Legendre-Cattaneo Sieve".to_string(),
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_progress_roundtrip() {
        let event = SearchEvent::Progress {
            elapsed_secs: 12.34,
            branches: 1000,
            pruned: 50,
            abundance_pruned: 10,
            branches_per_sec: 81.04,
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_prefix_roundtrip() {
        let event = SearchEvent::Prefix {
            len: 3,
            factors_str: "3^2, 5^2, 7^2".to_string(),
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_candidate_roundtrip() {
        let event = SearchEvent::Candidate {
            len: 4,
            factors_str: "3^2, 5^2, 7^2, 11^2".to_string(),
            rem_str: "123456789".to_string(),
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_status_update_roundtrip() {
        let event = SearchEvent::StatusUpdate {
            c: 1,
            total_weight_scaled: 100,
            comp: 5,
            pr: 2,
            active_str: "[3, 5]".to_string(),
            prefixes: 10,
            ap: 3,
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_dfs_complete_roundtrip() {
        let event = SearchEvent::DFSComplete {
            total_branches: 50000,
            ap: 1200,
            rp: 300,
            bp: 15,
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_raycast_deferred_roundtrip() {
        let event = SearchEvent::RaycastDeferred {
            rem_str: "987654321".to_string(),
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_overflow_roundtrip() {
        let event = SearchEvent::Overflow {
            p: "12345678901234567890".to_string(),
            pow: 2,
        };
        assert_roundtrip(event);
    }

    #[test]
    fn test_search_event_done_roundtrip() {
        let event = SearchEvent::Done {
            target_min_log10: 37,
            target_max_log10: 43,
            elapsed_ms: 12345678,
        };
        assert_roundtrip(event);
    }
}
