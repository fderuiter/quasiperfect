use crate::types::Uint;
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::thread::JoinHandle;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GraphTopologyManifest {
    pub adjacency: Vec<Vec<usize>>,
    pub scc_map: Vec<usize>,
    pub scc_components: Vec<Vec<usize>>,
    pub forced_candidates: Vec<Vec<usize>>,
}

pub enum PruneReason {
    TargetBound,
    CdgForcedCascade {
        forced_num: Uint,
        forced_den: Uint,
        lhs: Uint,
        rhs: Uint,
        topology_manifest: Option<GraphTopologyManifest>,
        reachable_paths: Option<Vec<Vec<usize>>>,
    },
    UnconditionalStarvation {
        max_allowed: usize,
        static_best_remaining: u128,
        lhs: Uint,
        rhs: Uint,
    },
    OverflowKill {
        s_l_mul: Uint,
        n_l_mul: Uint,
    },
    EulerCeiling {
        num: Uint,
        den: Uint,
        euler_num: Uint,
        euler_den: Uint,
    },
    DynamicStarvation {
        dynamic_best_achievable_fp: u128,
        lhs: Uint,
        rhs: Uint,
    },
    MinFactors {
        dynamic_min_factors: usize,
        curr_factors: usize,
        remaining_components: usize,
    },
    Raycast,
    Touchard {
        sigma_mod24: u32,
    },
    Lll {
        m: usize,
        shortest_sq_norm: String,
        target_log: f64,
        epsilon: f64,
    },
}

pub struct TraceEvent {
    pub work_unit_id: usize,
    pub step_index: u64,
    pub factors: SmallVec<[u64; 16]>,
    pub n_l: Uint,
    pub s_l: Uint,
    pub reason: PruneReason,
    pub verification_status: &'static str,
}

#[derive(Serialize)]
struct SerializableTraceEvent<'a> {
    work_unit_id: usize,
    step_index: u64,
    factors: &'a [u64],
    n_l: String,
    s_l: String,
    reason: &'static str,
    verification_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_allowed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    static_best_remaining: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lhs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rhs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    den: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    euler_num: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    euler_den: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dynamic_best_achievable_fp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dynamic_min_factors: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    curr_factors: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remaining_components: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    m: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shortest_sq_norm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_log: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    epsilon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    forced_num: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    forced_den: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topology_manifest: Option<&'a GraphTopologyManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reachable_paths: Option<&'a [Vec<usize>]>,
}

pub struct TraceWriter {
    pub sender: Sender<TraceEvent>,
    pub handle: JoinHandle<()>,
}

impl TraceWriter {
    pub fn new(file_path: &str) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded::<TraceEvent>();
        let path = file_path.to_string();

        let handle = std::thread::spawn(move || {
            let file = File::create(&path).expect("Failed to create trace file");
            let mut writer = BufWriter::with_capacity(1024 * 1024 * 8, file); // 8MB buffer

            for event in receiver {
                let mut ser_event = SerializableTraceEvent {
                    work_unit_id: event.work_unit_id,
                    step_index: event.step_index,
                    factors: &event.factors,
                    n_l: event.n_l.to_string(),
                    s_l: event.s_l.to_string(),
                    reason: "",
                    verification_status: event.verification_status,
                    max_allowed: None,
                    static_best_remaining: None,
                    lhs: None,
                    rhs: None,
                    num: None,
                    den: None,
                    euler_num: None,
                    euler_den: None,
                    dynamic_best_achievable_fp: None,
                    dynamic_min_factors: None,
                    curr_factors: None,
                    remaining_components: None,
                    m: None,
                    shortest_sq_norm: None,
                    target_log: None,
                    epsilon: None,
                    forced_num: None,
                    forced_den: None,
                    topology_manifest: None,
                    reachable_paths: None,
                };

                match &event.reason {
                    PruneReason::CdgForcedCascade {
                        forced_num,
                        forced_den,
                        lhs,
                        rhs,
                        topology_manifest,
                        reachable_paths,
                    } => {
                        ser_event.reason = "cdg_forced_cascade";
                        ser_event.forced_num = Some(forced_num.to_string());
                        ser_event.forced_den = Some(forced_den.to_string());
                        ser_event.lhs = Some(lhs.to_string());
                        ser_event.rhs = Some(rhs.to_string());
                        ser_event.topology_manifest = topology_manifest.as_ref();
                        ser_event.reachable_paths = reachable_paths.as_deref();
                    }
                    PruneReason::TargetBound => {
                        ser_event.reason = "target_bound";
                    }
                    PruneReason::UnconditionalStarvation {
                        max_allowed,
                        static_best_remaining,
                        lhs,
                        rhs,
                    } => {
                        ser_event.reason = "unconditional_starvation";
                        ser_event.max_allowed = Some(*max_allowed);
                        ser_event.static_best_remaining = Some(static_best_remaining.to_string());
                        ser_event.lhs = Some(lhs.to_string());
                        ser_event.rhs = Some(rhs.to_string());
                    }
                    PruneReason::OverflowKill { s_l_mul, n_l_mul } => {
                        ser_event.reason = "overflow_kill";
                        ser_event.lhs = Some(s_l_mul.to_string());
                        ser_event.rhs = Some(n_l_mul.to_string());
                    }
                    PruneReason::EulerCeiling {
                        num,
                        den,
                        euler_num,
                        euler_den,
                    } => {
                        ser_event.reason = "euler_ceiling";
                        ser_event.num = Some(num.to_string());
                        ser_event.den = Some(den.to_string());
                        ser_event.euler_num = Some(euler_num.to_string());
                        ser_event.euler_den = Some(euler_den.to_string());
                    }
                    PruneReason::DynamicStarvation {
                        dynamic_best_achievable_fp,
                        lhs,
                        rhs,
                    } => {
                        ser_event.reason = "dynamic_starvation";
                        ser_event.dynamic_best_achievable_fp =
                            Some(dynamic_best_achievable_fp.to_string());
                        ser_event.lhs = Some(lhs.to_string());
                        ser_event.rhs = Some(rhs.to_string());
                    }
                    PruneReason::MinFactors {
                        dynamic_min_factors,
                        curr_factors,
                        remaining_components,
                    } => {
                        ser_event.reason = "min_factors";
                        ser_event.dynamic_min_factors = Some(*dynamic_min_factors);
                        ser_event.curr_factors = Some(*curr_factors);
                        ser_event.remaining_components = Some(*remaining_components);
                    }
                    PruneReason::Raycast => {
                        ser_event.reason = "raycast";
                    }
                    PruneReason::Touchard { sigma_mod24 } => {
                        ser_event.reason = "touchard";
                        ser_event.lhs = Some(sigma_mod24.to_string());
                    }
                    PruneReason::Lll {
                        m,
                        shortest_sq_norm,
                        target_log,
                        epsilon,
                    } => {
                        ser_event.reason = "lattice_lll";
                        ser_event.m = Some(*m);
                        ser_event.shortest_sq_norm = Some(shortest_sq_norm.clone());
                        ser_event.target_log = Some(*target_log);
                        ser_event.epsilon = Some(*epsilon);
                    }
                }

                serde_json::to_writer(&mut writer, &ser_event).unwrap();
                writer.write_all(b"\n").unwrap();
            }
            writer.flush().unwrap();
        });

        TraceWriter { sender, handle }
    }
}

pub fn canonicalize_trace_file(file_path: &str) -> std::io::Result<()> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(());
    }

    struct Record {
        work_unit_id: usize,
        step_index: u64,
        line: String,
    }

    let mut records: Vec<Record> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let work_unit_id = v.get("work_unit_id").and_then(|w| w.as_u64()).unwrap_or(0) as usize;
            let step_index = v.get("step_index").and_then(|s| s.as_u64()).unwrap_or(0);
            records.push(Record {
                work_unit_id,
                step_index,
                line: trimmed.to_string(),
            });
        }
    }

    records.sort_by(|a, b| {
        a.work_unit_id
            .cmp(&b.work_unit_id)
            .then_with(|| a.step_index.cmp(&b.step_index))
            .then_with(|| a.line.cmp(&b.line))
    });

    let mut file = File::create(file_path)?;
    for r in records {
        file.write_all(r.line.as_bytes())?;
        file.write_all(b"\n")?;
    }
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_canonicalization_determinism() {
        let temp_dir = std::env::temp_dir();
        let trace_path = temp_dir.join("test_trace.jsonl");
        let path_str = trace_path.to_str().unwrap();

        let uncanonical_content = r#"{"work_unit_id": 2, "step_index": 0, "n_l": "100", "reason": "raycast"}
{"work_unit_id": 0, "step_index": 1, "n_l": "200", "reason": "target_bound"}
{"work_unit_id": 0, "step_index": 0, "n_l": "150", "reason": "touchard"}
{"work_unit_id": 1, "step_index": 0, "n_l": "300", "reason": "min_factors"}
"#;
        std::fs::write(path_str, uncanonical_content).unwrap();

        canonicalize_trace_file(path_str).unwrap();

        let canonical_content = std::fs::read_to_string(path_str).unwrap();
        let expected = r#"{"work_unit_id": 0, "step_index": 0, "n_l": "150", "reason": "touchard"}
{"work_unit_id": 0, "step_index": 1, "n_l": "200", "reason": "target_bound"}
{"work_unit_id": 1, "step_index": 0, "n_l": "300", "reason": "min_factors"}
{"work_unit_id": 2, "step_index": 0, "n_l": "100", "reason": "raycast"}
"#;
        assert_eq!(canonical_content, expected);

        canonicalize_trace_file(path_str).unwrap();
        let re_canonical_content = std::fs::read_to_string(path_str).unwrap();
        assert_eq!(re_canonical_content, expected);

        let _ = std::fs::remove_file(path_str);
    }
}
