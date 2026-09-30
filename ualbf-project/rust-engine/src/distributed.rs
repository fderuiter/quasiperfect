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

pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MB limit

pub fn write_framed_msg<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> std::io::Result<()> {
    let payload = serde_json::to_vec(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    if payload.len() > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Payload size {} exceeds max frame limit {}",
                payload.len(),
                MAX_FRAME_SIZE
            ),
        ));
    }
    let header = (payload.len() as u32).to_be_bytes();
    writer.write_all(&header)?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

pub fn send_message<T: Serialize>(
    stream_mutex: &Arc<Mutex<TcpStream>>,
    msg: &T,
) -> std::io::Result<()> {
    let mut stream = stream_mutex.lock().unwrap_or_else(|e| e.into_inner());
    write_framed_msg(&mut *stream, msg)
}

pub fn read_framed_payload<R: Read>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let mut header_buf = [0u8; 4];
    reader.read_exact(&mut header_buf)?;
    let length = u32::from_be_bytes(header_buf) as usize;
    if length > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Frame size {} exceeds max frame limit {}",
                length, MAX_FRAME_SIZE
            ),
        ));
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

pub fn recv_message<R: Read, T: for<'a> Deserialize<'a>>(reader: &mut R) -> std::io::Result<T> {
    let payload = read_framed_payload(reader)?;
    serde_json::from_slice(&payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
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
            active_mask: backbone.compatibility_matrix[i].clone().into(),
            sigma_mod24: (comp.sigma % Uint::from_u64(24)).as_u32(),
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

    for i in curr.last_idx..components.len() {
        let comp = &components[i];
        if !curr.factors.contains(&comp.p) {
            let row = &backbone.compatibility_matrix[i];
            let saved_state = curr.capture_state_and_intersect(row);
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
                } else {
                    curr.restore_state(&saved_state);
                }
            } else {
                curr.restore_state(&saved_state);
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

fn save_checkpoint(
    checkpoint_path: &str,
    queue: &[RangeWorkUnit],
    active_workers: &HashMap<usize, ActiveWorkerState>,
) {
    let active: Vec<RangeWorkUnit> = active_workers
        .values()
        .map(|w| w.active_task.clone())
        .collect();
    let pending = queue.to_vec();
    let schema = CheckpointSchema { active, pending };
    if let Ok(json) = serde_json::to_string(&schema) {
        let temp_path = format!("{}.tmp", checkpoint_path);
        if let Ok(mut file) = std::fs::File::create(&temp_path) {
            if file.write_all(json.as_bytes()).is_ok() {
                let _ = file.sync_all(); // Ensure durability before renaming
                drop(file);
                let _ = std::fs::rename(&temp_path, checkpoint_path);
            }
        }
    }
}

pub fn run_controller(addr: &str, units: Vec<RangeWorkUnit>) {
    let listener = match TcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind controller to {}: {}", addr, e);
            return;
        }
    };

    let heartbeat_timeout = crate::policy::get_safe_config().heartbeat_timeout_sec;

    let active_workers: Arc<Mutex<HashMap<usize, ActiveWorkerState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let worker_id_counter = Arc::new(AtomicUsize::new(1));

    println!("Controller listening on {}", addr);

    let checkpoint_path = "checkpoint.json";

    // Load from checkpoint if exists with fallback parsing
    let (initial_units, is_new_or_legacy) = load_checkpoint_or_fallback(checkpoint_path, units);

    let work_queue = Arc::new(Mutex::new(initial_units));
    let total_units = work_queue.lock().unwrap_or_else(|e| e.into_inner()).len();
    println!(
        "Partitioned search space into {} discrete pending work units.",
        total_units
    );

    // After constructing the initial queue, save the new schema immediately to persist the updated state
    if is_new_or_legacy {
        let queue = work_queue.lock().unwrap_or_else(|e| e.into_inner());
        let empty_workers = HashMap::new();
        save_checkpoint(checkpoint_path, &queue, &empty_workers);
    }

    let completed = Arc::new(AtomicUsize::new(0));

    let active_workers_monitor = Arc::clone(&active_workers);
    let work_queue_monitor = Arc::clone(&work_queue);
    let checkpoint_path_monitor = checkpoint_path.to_string();
    std::thread::spawn(move || {
        let timeout = Duration::from_secs(heartbeat_timeout);
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now = Instant::now();
            let mut to_remove = Vec::new();
            {
                let workers = active_workers_monitor
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                for (&id, state) in workers.iter() {
                    if now.duration_since(state.last_heartbeat) > timeout {
                        to_remove.push(id);
                    }
                }
            }
            if !to_remove.is_empty() {
                let mut queue = work_queue_monitor.lock().unwrap_or_else(|e| e.into_inner());
                let mut workers = active_workers_monitor
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let mut changed = false;
                for id in &to_remove {
                    if let Some(state) = workers.remove(id) {
                        println!("Worker {} timed out. Recovering task.", id);
                        queue.push(state.active_task);
                        changed = true;
                    }
                }
                if changed {
                    save_checkpoint(&checkpoint_path_monitor, &queue, &workers);
                }
            }
        }
    });

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let work_queue = Arc::clone(&work_queue);
            let completed = Arc::clone(&completed);
            let active_workers = Arc::clone(&active_workers);
            let worker_id = worker_id_counter.fetch_add(1, Ordering::Relaxed);
            let checkpoint_path_clone = checkpoint_path.to_string();

            thread::spawn(move || {
                let mut reader = match stream.try_clone() {
                    Ok(r) => r,
                    Err(_) => return,
                };
                let stream_mutex = Arc::new(Mutex::new(stream));

                loop {
                    match recv_message::<_, Message>(&mut reader) {
                        Ok(msg) => match msg {
                            Message::RequestWork => {
                                let mut queue =
                                    work_queue.lock().unwrap_or_else(|e| e.into_inner());
                                let work = queue.pop();
                                let mut workers =
                                    active_workers.lock().unwrap_or_else(|e| e.into_inner());
                                if let Some(ref w) = work {
                                    workers.insert(
                                        worker_id,
                                        ActiveWorkerState {
                                            active_task: w.clone(),
                                            last_heartbeat: Instant::now(),
                                        },
                                    );
                                }
                                save_checkpoint(&checkpoint_path_clone, &queue, &workers);
                                drop(workers);
                                drop(queue);

                                let reply = Message::WorkUnit(work);
                                if send_message(&stream_mutex, &reply).is_err() {
                                    break;
                                }
                            }
                            Message::Heartbeat => {
                                let mut workers =
                                    active_workers.lock().unwrap_or_else(|e| e.into_inner());
                                if let Some(state) = workers.get_mut(&worker_id) {
                                    state.last_heartbeat = Instant::now();
                                }
                            }
                            Message::WorkUnit(_) => {}
                            Message::Event(event) => {
                                if let Ok(event_json) = serde_json::to_string(&event) {
                                    println!("{}", event_json);
                                }
                                if let crate::events::SearchEvent::DFSComplete { .. } = event {
                                    let mut queue =
                                        work_queue.lock().unwrap_or_else(|e| e.into_inner());
                                    let mut workers =
                                        active_workers.lock().unwrap_or_else(|e| e.into_inner());
                                    if workers.remove(&worker_id).is_some() {
                                        let c = completed.fetch_add(1, Ordering::Relaxed) + 1;
                                        save_checkpoint(&checkpoint_path_clone, &queue, &workers);
                                        if c >= total_units {
                                            if let Ok(p4_json) = serde_json::to_string(
                                                &crate::events::SearchEvent::Phase {
                                                    phase: 4,
                                                    name: "All work units completed".to_string(),
                                                },
                                            ) {
                                                println!("{}", p4_json);
                                            }
                                            std::process::exit(0);
                                        }
                                    }
                                }
                            }
                        },
                        Err(_) => break, // Socket error or peer disconnected cleanly
                    }
                }

                // Connection closed unexpectedly
                let mut queue = work_queue.lock().unwrap_or_else(|e| e.into_inner());
                let mut workers = active_workers.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(state) = workers.remove(&worker_id) {
                    println!(
                        "Worker {} disconnected unexpectedly. Recovering task.",
                        worker_id
                    );
                    queue.push(state.active_task);
                    save_checkpoint(&checkpoint_path_clone, &queue, &workers);
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

    let stream = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to controller at {}: {}", addr, e);
            return (
                crate::dfs_tree::DfsTelemetry {
                    total_branches: 0,
                    abundance_pruned: 0,
                    raycast_pruned: 0,
                    search_space_density: 0.0,
                    math_interruptions: 0,
                    boundary_pruned: 0,
                },
                Vec::new(),
            );
        }
    };
    println!("Connected to controller at {}", addr);

    let mut reader = match stream.try_clone() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to clone stream handle: {}", e);
            return (
                crate::dfs_tree::DfsTelemetry {
                    total_branches: 0,
                    abundance_pruned: 0,
                    raycast_pruned: 0,
                    search_space_density: 0.0,
                    math_interruptions: 0,
                    boundary_pruned: 0,
                },
                Vec::new(),
            );
        }
    };
    let stream_mutex = Arc::new(Mutex::new(stream));

    let mut total_branches = 0;
    let mut total_abundance_pruned = 0;
    let mut total_raycast_pruned = 0;
    let mut total_math_interruptions = 0;
    let mut total_boundary_pruned = 0;
    let mut explored_ranges = Vec::new();

    loop {
        // Request work
        let req = Message::RequestWork;
        if send_message(&stream_mutex, &req).is_err() {
            eprintln!("Failed to send RequestWork to controller.");
            break;
        }

        let msg: Message = match recv_message(&mut reader) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Worker stream error or disconnect: {}", e);
                break;
            }
        };

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
                    active_mask: vec![u64::MAX; mask_len].into(),
                    sigma_mod24: 1,
                };

                let count = AtomicUsize::new(0);
                let pruned_count = AtomicUsize::new(0);
                let abundance_pruned = AtomicUsize::new(0);
                let boundary_pruned = AtomicUsize::new(0);
                let completed_weight_scaled = AtomicUsize::new(0);
                let math_interruptions = AtomicUsize::new(0);

                let (tx, rx) = crossbeam_channel::unbounded();
                let stream_mutex_reporter = Arc::clone(&stream_mutex);
                let reporter_thread = std::thread::spawn(move || {
                    while let Ok(msg) = rx.recv() {
                        let rep = Message::Event(msg);
                        if send_message(&stream_mutex_reporter, &rep).is_err() {
                            break;
                        }
                    }
                });

                let heartbeat_interval = crate::policy::get_safe_config().heartbeat_interval_sec;

                let (hb_tx, hb_rx) = crossbeam_channel::unbounded::<()>();
                let stream_mutex_hb = Arc::clone(&stream_mutex);
                let hb_thread = std::thread::spawn(move || {
                    let interval = std::time::Duration::from_secs(heartbeat_interval);
                    loop {
                        match hb_rx.recv_timeout(interval) {
                            Ok(_) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                                let rep = Message::Heartbeat;
                                if send_message(&stream_mutex_hb, &rep).is_err() {
                                    break;
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
                    &boundary_pruned,
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
                    0,
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
                total_boundary_pruned += boundary_pruned.load(Ordering::Relaxed);
                explored_ranges.push(range_bound.clone());
                let rep = Message::Event(crate::events::SearchEvent::DFSComplete {
                    total_branches: count.into_inner(),
                    ap: abundance_pruned.into_inner(),
                    rp: pruned_count.into_inner(),
                    bp: boundary_pruned.into_inner(),
                });
                if send_message(&stream_mutex, &rep).is_err() {
                    eprintln!("Failed to send DFSComplete event to controller.");
                    break;
                }
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
            boundary_pruned: total_boundary_pruned,
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
        let temp_path = "test_checkpoint_atomic.json";
        let _ = fs::remove_file(temp_path);
        let temp_tmp_path = "test_checkpoint_atomic.json.tmp";
        let _ = fs::remove_file(temp_tmp_path);

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

        save_checkpoint(temp_path, &queue, &active_workers);

        // Verify test_checkpoint_atomic.json exists and contains correct content
        assert!(std::path::Path::new(temp_path).exists());
        let content = fs::read_to_string(temp_path).unwrap();
        let schema: CheckpointSchema = serde_json::from_str(&content).unwrap();
        assert_eq!(schema.pending.len(), 1);
        assert_eq!(schema.active.len(), 1);
        assert_eq!(schema.pending[0].start_bound, vec![1, 2]);
        assert_eq!(schema.active[0].start_bound, vec![5, 6]);

        // Clean up
        let _ = fs::remove_file(temp_path);
        let _ = fs::remove_file(temp_tmp_path);
    }

    #[test]
    fn test_framing_length_prefix_roundtrip() {
        let msg = Message::RequestWork;
        let mut buf = Vec::new();
        write_framed_msg(&mut buf, &msg).unwrap();

        // Verify length prefix
        assert!(buf.len() > 4);
        let len_bytes: [u8; 4] = buf[..4].try_into().unwrap();
        let payload_len = u32::from_be_bytes(len_bytes) as usize;
        assert_eq!(payload_len, buf.len() - 4);

        // Read framed payload
        let mut cursor = std::io::Cursor::new(buf);
        let recv: Message = recv_message(&mut cursor).unwrap();
        match recv {
            Message::RequestWork => {}
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_coalesced_and_fragmented_frames() {
        let msg1 = Message::RequestWork;
        let msg2 = Message::Heartbeat;
        let msg3 = Message::WorkUnit(Some(RangeWorkUnit {
            start_bound: vec![1, 2, 3],
            end_bound: vec![4, 5, 6],
        }));

        let mut buf = Vec::new();
        write_framed_msg(&mut buf, &msg1).unwrap();
        write_framed_msg(&mut buf, &msg2).unwrap();
        write_framed_msg(&mut buf, &msg3).unwrap();

        let mut cursor = std::io::Cursor::new(buf);

        let r1: Message = recv_message(&mut cursor).unwrap();
        match r1 {
            Message::RequestWork => {}
            _ => panic!("Expected RequestWork"),
        }

        let r2: Message = recv_message(&mut cursor).unwrap();
        match r2 {
            Message::Heartbeat => {}
            _ => panic!("Expected Heartbeat"),
        }

        let r3: Message = recv_message(&mut cursor).unwrap();
        match r3 {
            Message::WorkUnit(Some(unit)) => {
                assert_eq!(unit.start_bound, vec![1, 2, 3]);
                assert_eq!(unit.end_bound, vec![4, 5, 6]);
            }
            _ => panic!("Expected WorkUnit"),
        }
    }

    #[test]
    fn test_max_frame_bound_exceeded() {
        let mut buf = Vec::new();
        // Claim length = 17 MB (exceeds 16 MB limit)
        let huge_len: u32 = 17 * 1024 * 1024;
        buf.extend_from_slice(&huge_len.to_be_bytes());
        buf.extend_from_slice(&[0u8; 100]); // Dummy bytes

        let mut cursor = std::io::Cursor::new(buf);
        let res = read_framed_payload(&mut cursor);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("exceeds max frame limit"));
    }

    #[test]
    fn test_corrupt_payload_graceful_error() {
        let mut buf = Vec::new();
        let payload = b"invalid json content {";
        let len_bytes = (payload.len() as u32).to_be_bytes();
        buf.extend_from_slice(&len_bytes);
        buf.extend_from_slice(payload);

        let mut cursor = std::io::Cursor::new(buf);
        let res: std::io::Result<Message> = recv_message(&mut cursor);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_tcp_controller_worker_end_to_end() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = stream.try_clone().unwrap();
            let stream_mutex = Arc::new(Mutex::new(stream));

            // Receive RequestWork
            let msg: Message = recv_message(&mut reader).unwrap();
            match msg {
                Message::RequestWork => {}
                _ => panic!("Expected RequestWork"),
            }

            // Send WorkUnit
            let unit = RangeWorkUnit {
                start_bound: vec![10],
                end_bound: vec![20],
            };
            send_message(&stream_mutex, &Message::WorkUnit(Some(unit))).unwrap();

            // Receive Event
            let msg2: Message = recv_message(&mut reader).unwrap();
            match msg2 {
                Message::Event(_) | Message::Heartbeat => {}
                _ => panic!("Expected Event or Heartbeat"),
            }
        });

        let client_stream = TcpStream::connect(local_addr).unwrap();
        let mut client_reader = client_stream.try_clone().unwrap();
        let client_mutex = Arc::new(Mutex::new(client_stream));

        // Send RequestWork
        send_message(&client_mutex, &Message::RequestWork).unwrap();

        // Recv WorkUnit
        let resp: Message = recv_message(&mut client_reader).unwrap();
        match resp {
            Message::WorkUnit(Some(unit)) => {
                assert_eq!(unit.start_bound, vec![10]);
                assert_eq!(unit.end_bound, vec![20]);
            }
            _ => panic!("Expected WorkUnit"),
        }

        // Send Event
        send_message(
            &client_mutex,
            &Message::Event(crate::events::SearchEvent::Phase {
                phase: 1,
                name: "Test".to_string(),
            }),
        )
        .unwrap();

        server_thread.join().unwrap();
    }

    #[test]
    fn test_concurrent_mutex_writes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = listener.local_addr().unwrap();

        let num_threads = 10;
        let msgs_per_thread = 50;

        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = stream.try_clone().unwrap();

            let total_expected = num_threads * msgs_per_thread;
            let mut received = 0;
            while received < total_expected {
                let msg: Message = recv_message(&mut reader).unwrap();
                match msg {
                    Message::Heartbeat => received += 1,
                    _ => panic!("Expected Heartbeat message"),
                }
            }
            assert_eq!(received, total_expected);
        });

        let client_stream = TcpStream::connect(local_addr).unwrap();
        let client_mutex = Arc::new(Mutex::new(client_stream));

        let mut handles = Vec::new();
        for _ in 0..num_threads {
            let mutex_clone = Arc::clone(&client_mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..msgs_per_thread {
                    send_message(&mutex_clone, &Message::Heartbeat).unwrap();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        server_thread.join().unwrap();
    }
}
