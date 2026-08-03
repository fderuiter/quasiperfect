use crate::types::Uint;
use crossbeam_channel::Sender;
use serde::Serialize;
use smallvec::SmallVec;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::thread::JoinHandle;

pub enum PruneReason {
    TargetBound,
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
}

pub struct TraceEvent {
    pub factors: SmallVec<[u64; 16]>,
    pub n_l: Uint,
    pub s_l: Uint,
    pub reason: PruneReason,
    pub verification_status: &'static str,
}

#[derive(Serialize)]
struct SerializableTraceEvent<'a> {
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
                };

                match &event.reason {
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
                }

                serde_json::to_writer(&mut writer, &ser_event).unwrap();
                writer.write_all(b"\n").unwrap();
            }
            writer.flush().unwrap();
        });

        TraceWriter { sender, handle }
    }
}
