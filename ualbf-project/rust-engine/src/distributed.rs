use crate::math_utils::SigmaCache;
use crate::schema_generated::{Prefix, SerializedPrefix};
use crate::types::UintExt;
use crate::types::{Int, PrimePower, Uint};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RangeWorkUnit {
    pub start_bound: Vec<u64>,
    pub end_bound: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    RequestWork,
    WorkUnit(Option<RangeWorkUnit>),
    Event(crate::events::SearchEvent),
    Heartbeat,
}

pub fn generate_work_units(
    components: &[PrimePower],
    target_bound: &Uint,
    depth_limit: usize,
) -> Vec<RangeWorkUnit> {
    let lazy_cache: std::sync::Arc<Vec<std::sync::OnceLock<Result<Vec<Uint>, ()>>>> =
        std::sync::Arc::new(
            std::iter::repeat_with(std::sync::OnceLock::new)
                .take(components.len())
                .collect(),
        );
    let backbone = crate::backbone::SearchBackbone::new(components, &lazy_cache);

    let mut units = Vec::new();
    for i in 0..components.len() {
        let comp = &components[i];
        let mut curr = Prefix {
            n_l: comp.val,
            s_l: comp.sigma,
            last_idx: i + 1,
            factors: vec![comp.p],
            sigma_factors_u64: {
                let mut su = Vec::new();
                for sf in &comp.sigma_factors {
                    if *sf <= Uint::from_u128((u64::MAX) as u128) {
                        su.push(sf.as_u64());
                    }
                }
                su
            },
            sigma_factors: comp.sigma_factors.clone(),
            active_mask: backbone.compatibility_matrix[i].clone(),
        };
        expand_work_units(
            &mut curr,
            components,
            target_bound,
            depth_limit,
            0,
            &mut units,
            &backbone,
        );
    }

    let mut paths: Vec<Vec<u64>> = units.into_iter().map(|u| u.factors).collect();
    // Sort paths lexicographically just in case
    paths.sort();

    let mut ranges = Vec::new();
    if paths.is_empty() {
        ranges.push(RangeWorkUnit {
            start_bound: vec![],
            end_bound: vec![],
        });
    } else {
        ranges.push(RangeWorkUnit {
            start_bound: vec![],
            end_bound: paths[0].clone(),
        });
        for i in 0..paths.len() - 1 {
            ranges.push(RangeWorkUnit {
                start_bound: paths[i].clone(),
                end_bound: paths[i + 1].clone(),
            });
        }
        ranges.push(RangeWorkUnit {
            start_bound: paths.last().unwrap().clone(),
            end_bound: vec![],
        });
    }
    ranges
}

fn expand_work_units(
    curr: &mut Prefix,
    components: &[PrimePower],
    target_bound: &Uint,
    depth_limit: usize,
    depth: usize,
    units: &mut Vec<Prefix>,
    backbone: &crate::backbone::SearchBackbone,
) {
    if curr.n_l > *target_bound {
        return;
    }
    if depth >= depth_limit {
        units.push(curr.clone());
        return;
    }

    let saved_state = curr.capture_state();

    for i in saved_state.last_idx..components.len() {
        let comp = &components[i];
        if !curr.factors.contains(&comp.p) {
            if let (Some(next_n_l), Some(next_s_l)) = (
                saved_state.n_l.checked_mul(comp.val),
                saved_state.s_l.checked_mul(comp.sigma),
            ) {
                if next_n_l <= *target_bound {
                    curr.n_l = next_n_l;
                    curr.s_l = next_s_l;
                    curr.last_idx = i + 1;
                    curr.factors.push(comp.p);
                    curr.sigma_factors.extend_from_slice(&comp.sigma_factors);
                    for sf in &comp.sigma_factors {
                        if *sf <= Uint::from_u128(u64::MAX as u128) {
                            curr.sigma_factors_u64.push(sf.as_u64());
                        }
                    }

                    let row = &backbone.compatibility_matrix[i];
                    for k in 0..curr.active_mask.len() {
                        curr.active_mask[k] &= row[k];
                    }
                    expand_work_units(
                        curr,
                        components,
                        target_bound,
                        depth_limit,
                        depth + 1,
                        units,
                        backbone,
                    );
                    curr.restore_state(&saved_state);
                }
            }
        }
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};

use std::collections::HashMap;
use std::time::{Duration, Instant};

struct ActiveWorkerState {
    active_task: RangeWorkUnit,
    last_heartbeat: Instant,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CheckpointSchema {
    pub active: Vec<RangeWorkUnit>,
    pub pending: Vec<RangeWorkUnit>,
}

pub fn load_checkpoint_or_fallback(
    checkpoint_path: &str,
    default_units: Vec<RangeWorkUnit>,
) -> (Vec<RangeWorkUnit>, bool) {
    if let Ok(content) = std::fs::read_to_string(checkpoint_path) {
        println!("Resuming from {}", checkpoint_path);
        // Try to parse as the new unified checkpoint schema first
        if let Ok(schema) = serde_json::from_str::<CheckpointSchema>(&content) {
            let mut initial_queue = schema.pending;
            // The controller must put recovered active tasks at the front of the queue to prioritize their execution.
            // Since work_queue.pop() removes elements from the end of the vector, prepending/appending recovered active
            // tasks to the queue so they are processed next ensures priority. Since it's a LIFO behavior (i.e. elements
            // are popped from the end), appending active units to the end of the queue Vec puts them at the "front" of
            // execution priority.
            initial_queue.extend(schema.active);
            (initial_queue, true)
        } else if let Ok(legacy_units) = serde_json::from_str::<Vec<RangeWorkUnit>>(&content) {
            // Fallback parsing: convert flat legacy format into unassigned tasks
            (legacy_units, true)
        } else {
            // Reject corrupt or invalid JSON files by ignoring/falling back to generated units
            eprintln!(
                "Warning: corrupt or invalid checkpoint file {}. Falling back to generated units.",
                checkpoint_path
            );
            (default_units, false)
        }
    } else {
        (default_units, false)
    }
}

fn save_checkpoint(queue: &[RangeWorkUnit], active_workers: &HashMap<usize, ActiveWorkerState>) {
    let active: Vec<RangeWorkUnit> = active_workers
        .values()
        .map(|w| w.active_task.clone())
        .collect();
    let pending = queue.to_vec();
    let schema = CheckpointSchema { active, pending };
    if let Ok(json) = serde_json::to_string(&schema) {
        let temp_path = "checkpoint.json.tmp";
        let target_path = "checkpoint.json";
        if let Ok(mut file) = std::fs::File::create(temp_path) {
            if file.write_all(json.as_bytes()).is_ok() {
                let _ = file.sync_all(); // Ensure durability before renaming
                drop(file);
                let _ = std::fs::rename(temp_path, target_path);
            }
        }
    }
}

pub fn run_controller(addr: &str, units: Vec<RangeWorkUnit>) {
    let listener = TcpListener::bind(addr).unwrap();

    let heartbeat_timeout = std::env::var("UALBF_HEARTBEAT_TIMEOUT_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15);

    let active_workers: Arc<Mutex<HashMap<usize, ActiveWorkerState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let worker_id_counter = Arc::new(AtomicUsize::new(1));

    println!("Controller listening on {}", addr);

    // Load from checkpoint if exists with fallback parsing
    let (initial_units, is_new_or_legacy) = load_checkpoint_or_fallback("checkpoint.json", units);

    let work_queue = Arc::new(Mutex::new(initial_units));
    let total_units = work_queue.lock().unwrap().len();
    println!(
        "Partitioned search space into {} discrete pending work units.",
        total_units
    );

    // After constructing the initial queue, save the new schema immediately to persist the updated state
    if is_new_or_legacy {
        let queue = work_queue.lock().unwrap();
        let empty_workers = HashMap::new();
        save_checkpoint(&queue, &empty_workers);
    }

    let completed = Arc::new(AtomicUsize::new(0));

    let active_workers_monitor = Arc::clone(&active_workers);
    let work_queue_monitor = Arc::clone(&work_queue);
    std::thread::spawn(move || {
        let timeout = Duration::from_secs(heartbeat_timeout);
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now = Instant::now();
            let mut to_remove = Vec::new();
            {
                let workers = active_workers_monitor.lock().unwrap();
                for (&id, state) in workers.iter() {
                    if now.duration_since(state.last_heartbeat) > timeout {
                        to_remove.push(id);
                    }
                }
            }
            if !to_remove.is_empty() {
                let mut queue = work_queue_monitor.lock().unwrap();
                let mut workers = active_workers_monitor.lock().unwrap();
                let mut changed = false;
                for id in &to_remove {
                    if let Some(state) = workers.remove(id) {
                        println!("Worker {} timed out. Recovering task.", id);
                        queue.push(state.active_task);
                        changed = true;
                    }
                }
                if changed {
                    save_checkpoint(&queue, &workers);
                }
            }
        }
    });

    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let work_queue = Arc::clone(&work_queue);
            let completed = Arc::clone(&completed);
            let active_workers = Arc::clone(&active_workers);
            let worker_id = worker_id_counter.fetch_add(1, Ordering::Relaxed);

            thread::spawn(move || {
                let mut buf = vec![0; 1024 * 1024];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break, // Connection closed
                        Ok(n) => {
                            let msg: Result<Message, _> = serde_json::from_slice(&buf[..n]);
                            if let Ok(msg) = msg {
                                match msg {
                                    Message::RequestWork => {
                                        let mut queue = work_queue.lock().unwrap();
                                        let work = queue.pop();
                                        let mut workers = active_workers.lock().unwrap();
                                        if let Some(ref w) = work {
                                            workers.insert(
                                                worker_id,
                                                ActiveWorkerState {
                                                    active_task: w.clone(),
                                                    last_heartbeat: Instant::now(),
                                                },
                                            );
                                        }
                                        // Save checkpoint with updated queue and active workers
                                        save_checkpoint(&queue, &workers);
                                        let reply = Message::WorkUnit(work);
                                        let reply_bytes = serde_json::to_vec(&reply).unwrap();
                                        if stream.write_all(&reply_bytes).is_err() {
                                            break;
                                        }
                                    }
                                    Message::Heartbeat => {
                                        let mut workers = active_workers.lock().unwrap();
                                        if let Some(state) = workers.get_mut(&worker_id) {
                                            state.last_heartbeat = Instant::now();
                                        }
                                    }
                                    Message::WorkUnit(_) => {}
                                    Message::Event(event) => {
                                        println!("{}", serde_json::to_string(&event).unwrap());
                                        if let crate::events::SearchEvent::DFSComplete { .. } =
                                            event
                                        {
                                            let mut queue = work_queue.lock().unwrap();
                                            let mut workers = active_workers.lock().unwrap();
                                            if workers.remove(&worker_id).is_some() {
                                                let c =
                                                    completed.fetch_add(1, Ordering::Relaxed) + 1;
                                                save_checkpoint(&queue, &workers);
                                                if c >= total_units {
                                                    println!(
                                                        "{}",
                                                        serde_json::to_string(
                                                            &crate::events::SearchEvent::Phase {
                                                                phase: 4,
                                                                name: "All work units completed"
                                                                    .to_string()
                                                            }
                                                        )
                                                        .unwrap()
                                                    );
                                                    std::process::exit(0);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }

                // Connection closed unexpectedly
                let mut queue = work_queue.lock().unwrap();
                let mut workers = active_workers.lock().unwrap();
                if let Some(state) = workers.remove(&worker_id) {
                    println!(
                        "Worker {} disconnected unexpectedly. Recovering task.",
                        worker_id
                    );
                    queue.push(state.active_task);
                    save_checkpoint(&queue, &workers);
                }
            });
        }
    }
}

pub fn run_worker(
    addr: &str,
    components: &[PrimePower],
    stop_threshold: &Uint,
    target_min: &Uint,
    target_bound: &Uint,
    illegal_valuations: &[(Int, Int)],
    suffix_abundance: &[u128],
    total_weight_scaled: usize,
    sigma_cache: &SigmaCache,
    max_idx_3: usize,
    max_idx_5: usize,
) -> (crate::dfs_tree::DfsTelemetry, Vec<RangeWorkUnit>) {
    use std::sync::atomic::AtomicU64;

    let active_primes: Arc<[AtomicU64]> = std::iter::repeat_with(|| AtomicU64::new(0))
        .take(crate::profile::get_profile().active_prime_slots)
        .collect();
    let lazy_cache: Arc<Vec<std::sync::OnceLock<Result<Vec<Uint>, ()>>>> = Arc::new(
        std::iter::repeat_with(std::sync::OnceLock::new)
            .take(components.len())
            .collect(),
    );
    let backbone = Arc::new(crate::backbone::SearchBackbone::new(
        components,
        &lazy_cache,
    ));
    let mut stream = TcpStream::connect(addr).expect("Failed to connect to controller");
    println!("Connected to controller at {}", addr);
    let mut total_branches = 0;
    let mut total_abundance_pruned = 0;
    let mut total_raycast_pruned = 0;
    let mut total_math_interruptions = 0;
    let mut explored_ranges = Vec::new();

    loop {
        // Request work
        let req = Message::RequestWork;
        let req_bytes = serde_json::to_vec(&req).unwrap();
        stream.write_all(&req_bytes).unwrap();

        let mut buf = vec![0; 1024 * 1024]; // 1MB buffer
        let n = stream.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }

        let msg: Message = serde_json::from_slice(&buf[..n]).unwrap();
        match msg {
            Message::WorkUnit(Some(range_bound)) => {
                let mask_len = if !components.is_empty() {
                    backbone.compatibility_matrix[0].len()
                } else {
                    1
                };
                let mut prefix = Prefix {
                    n_l: Uint::from_u32(1),
                    s_l: Uint::from_u32(1),
                    last_idx: 0,
                    factors: vec![],
                    sigma_factors: vec![],
                    sigma_factors_u64: vec![],
                    active_mask: vec![u64::MAX; mask_len],
                };

                let count = AtomicUsize::new(0);
                let pruned_count = AtomicUsize::new(0);
                let abundance_pruned = AtomicUsize::new(0);
                let completed_weight_scaled = AtomicUsize::new(0);
                let math_interruptions = AtomicUsize::new(0);

                let (tx, rx) = crossbeam_channel::unbounded();
                let mut stream_clone = stream.try_clone().unwrap();
                let reporter_thread = std::thread::spawn(move || {
                    while let Ok(msg) = rx.recv() {
                        let rep = Message::Event(msg);
                        let rep_bytes = serde_json::to_vec(&rep).unwrap();
                        let _ = stream_clone.write_all(&rep_bytes);
                    }
                });

                let heartbeat_interval = std::env::var("UALBF_HEARTBEAT_INTERVAL_SEC")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5);

                let (hb_tx, hb_rx) = crossbeam_channel::unbounded::<()>();
                let mut hb_stream = stream.try_clone().unwrap();
                let hb_thread = std::thread::spawn(move || {
                    let interval = std::time::Duration::from_secs(heartbeat_interval);
                    loop {
                        match hb_rx.recv_timeout(interval) {
                            Ok(_) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                                let rep = Message::Heartbeat;
                                if let Ok(rep_bytes) = serde_json::to_vec(&rep) {
                                    if hb_stream.write_all(&rep_bytes).is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                let start_bound = if range_bound.start_bound.is_empty() {
                    None
                } else {
                    Some(range_bound.start_bound.as_slice())
                };
                let end_bound = if range_bound.end_bound.is_empty() {
                    None
                } else {
                    Some(range_bound.end_bound.as_slice())
                };

                crate::dfs_tree::explore_prefix(
                    start_bound,
                    end_bound,
                    &mut prefix,
                    components,
                    stop_threshold,
                    target_min,
                    target_bound,
                    illegal_valuations,
                    suffix_abundance,
                    &count,
                    &pruned_count,
                    &abundance_pruned,
                    &completed_weight_scaled,
                    &math_interruptions,
                    total_weight_scaled,
                    &active_primes,
                    0,
                    sigma_cache,
                    Some(&tx),
                    max_idx_3,
                    max_idx_5,
                    &lazy_cache,
                    &backbone,
                    None,
                );
                drop(tx);
                let _ = reporter_thread.join();

                drop(hb_tx);
                let _ = hb_thread.join();

                // Report back
                total_branches += count.load(Ordering::Relaxed);
                total_abundance_pruned += abundance_pruned.load(Ordering::Relaxed);
                total_raycast_pruned += pruned_count.load(Ordering::Relaxed);
                total_math_interruptions += math_interruptions.load(Ordering::Relaxed);
                explored_ranges.push(range_bound.clone());
                let rep = Message::Event(crate::events::SearchEvent::DFSComplete {
                    total_branches: count.into_inner(),
                    ap: abundance_pruned.into_inner(),
                    rp: pruned_count.into_inner(),
                });
                let rep_bytes = serde_json::to_vec(&rep).unwrap();
                stream.write_all(&rep_bytes).unwrap();
            }
            Message::WorkUnit(None) => {
                println!("No more work. Worker exiting.");
                break;
            }
            _ => {}
        }
    }
    (
        crate::dfs_tree::DfsTelemetry {
            total_branches,
            abundance_pruned: total_abundance_pruned,
            raycast_pruned: total_raycast_pruned,
            search_space_density: 0.0,
            math_interruptions: total_math_interruptions,
        },
        explored_ranges,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_save_and_load_checkpoint_unified() {
        let temp_path = "test_checkpoint_unified.json";
        let _ = fs::remove_file(temp_path);

        let p1 = RangeWorkUnit {
            start_bound: vec![1, 2],
            end_bound: vec![3, 4],
        };
        let p2 = RangeWorkUnit {
            start_bound: vec![5, 6],
            end_bound: vec![7, 8],
        };
        let a1 = RangeWorkUnit {
            start_bound: vec![9, 10],
            end_bound: vec![11, 12],
        };

        // Create a dummy active_workers mapping and test parsing
        let schema = CheckpointSchema {
            active: vec![a1.clone()],
            pending: vec![p1.clone(), p2.clone()],
        };

        let json = serde_json::to_string(&schema).unwrap();
        fs::write(temp_path, json).unwrap();

        // Load it back
        let default_units = vec![];
        let (loaded, ok) = load_checkpoint_or_fallback(temp_path, default_units);
        assert!(ok);
        // It should have: pending + active.
        // Since active is put at the end of the queue (pop priority): [p1, p2, a1]
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].start_bound, vec![1, 2]);
        assert_eq!(loaded[1].start_bound, vec![5, 6]);
        assert_eq!(loaded[2].start_bound, vec![9, 10]); // Recovered active task priority!

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_load_checkpoint_legacy_fallback() {
        let temp_path = "test_checkpoint_legacy.json";
        let _ = fs::remove_file(temp_path);

        let p1 = RangeWorkUnit {
            start_bound: vec![1, 2],
            end_bound: vec![3, 4],
        };
        let p2 = RangeWorkUnit {
            start_bound: vec![5, 6],
            end_bound: vec![7, 8],
        };

        let legacy_data = vec![p1, p2];
        let json = serde_json::to_string(&legacy_data).unwrap();
        fs::write(temp_path, json).unwrap();

        // Load it back
        let default_units = vec![];
        let (loaded, ok) = load_checkpoint_or_fallback(temp_path, default_units);
        assert!(ok);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].start_bound, vec![1, 2]);
        assert_eq!(loaded[1].start_bound, vec![5, 6]);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_load_checkpoint_corrupt_fallback() {
        let temp_path = "test_checkpoint_corrupt.json";
        let _ = fs::remove_file(temp_path);

        fs::write(temp_path, "{invalid_json: true").unwrap();

        let p_default = RangeWorkUnit {
            start_bound: vec![99],
            end_bound: vec![100],
        };
        let default_units = vec![p_default.clone()];

        let (loaded, ok) = load_checkpoint_or_fallback(temp_path, default_units);
        assert!(!ok); // Rejected corrupt JSON
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].start_bound, vec![99]);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_save_checkpoint_atomic() {
        // Backup existing checkpoint.json if it exists
        let backup_path = "checkpoint.json.bak";
        let original_exists = std::path::Path::new("checkpoint.json").exists();
        if original_exists {
            let _ = fs::rename("checkpoint.json", backup_path);
        }

        let p1 = RangeWorkUnit {
            start_bound: vec![1, 2],
            end_bound: vec![3, 4],
        };
        let a1 = RangeWorkUnit {
            start_bound: vec![5, 6],
            end_bound: vec![7, 8],
        };

        let queue = vec![p1];
        let mut active_workers = HashMap::new();
        active_workers.insert(
            42,
            ActiveWorkerState {
                active_task: a1,
                last_heartbeat: Instant::now(),
            },
        );

        save_checkpoint(&queue, &active_workers);

        // Verify checkpoint.json exists and contains correct content
        assert!(std::path::Path::new("checkpoint.json").exists());
        let content = fs::read_to_string("checkpoint.json").unwrap();
        let schema: CheckpointSchema = serde_json::from_str(&content).unwrap();
        assert_eq!(schema.pending.len(), 1);
        assert_eq!(schema.active.len(), 1);
        assert_eq!(schema.pending[0].start_bound, vec![1, 2]);
        assert_eq!(schema.active[0].start_bound, vec![5, 6]);

        // Clean up
        let _ = fs::remove_file("checkpoint.json");
        let _ = fs::remove_file("checkpoint.json.tmp").unwrap_or(()); // Just in case

        // Restore backup
        if original_exists {
            let _ = fs::rename(backup_path, "checkpoint.json");
        }
    }
}
