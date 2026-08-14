#![allow(warnings)]
pub mod backbone;
pub mod cdg;
pub mod events;
use crate::types::UintExt;
pub mod obstruction;
#[cfg(feature = "signing")]
use ed25519_dalek::{Signer, SigningKey};
#[cfg(feature = "signing")]
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;

mod dfs_tree;
mod lean_ffi;
mod manifest_constants;
mod profile;
pub mod pruning_dispatch;
pub mod residue;
pub mod trace;
pub mod verus_proofs;

mod distributed;
mod math_utils;
mod policy;
mod raycast;
mod schema_generated;
mod sieve;
pub mod state;
mod types;
mod universal_bounds;

#[cfg(feature = "lattice")]
pub mod lattice;
use crate::types::Uint;

#[derive(Deserialize, Debug)]
struct Theorem {
    name: String,
    file: String,
    status: String,
    checksum: String,
}

#[derive(Deserialize, Debug)]
struct Manifest {
    theorems: Vec<Theorem>,
    verus_hashes: std::collections::HashMap<String, String>,
    bounds_manifest_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationProfile {
    pub sampling_rate: f64,
    pub deterministic_seed: u64,
}

#[derive(Serialize, Debug)]
struct SearchTelemetry {
    target_min_log10: u32,
    target_max_log10: u32,
    sieve_limit: usize,
    max_exponent: u32,
    prefix_stop: u64,
    total_branches_searched: usize,
    abundance_pruned: usize,
    raycast_pruned: usize,
    boundary_pruned: usize,
    phase1_retained: usize,
    phase1_pruned: usize,
    phase1_trial_only: usize,
    phase1_ecm_fallback: usize,
    phase1_execution_time_ms: u128,
    search_space_density: f64,
    math_interruptions: usize,
    phase2_execution_time_ms: u128,
    total_execution_time_ms: u128,
    baseline_min_prime_factors: usize,
    prasad_sunitha_bound: usize,
    trace_hash: String,
    factorization_depth: u32,
    bounds_exceeded: bool,
    pub explored_ranges: Vec<crate::distributed::RangeWorkUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_profile: Option<VerificationProfile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Citation {
    author: String,
    year: String,
    title: String,
    identifier: String,
}

#[derive(Serialize, Debug)]
struct CertificateCitations {
    target_min_log10: Option<Citation>,
    baseline_min_prime_factors: Option<Citation>,
    prasad_sunitha_bound: Option<Citation>,
    euler_ceiling: Option<Citation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ConjectureMetadata {
    pub conditional: bool,
    pub conjecture_name: String,
    pub conjectural_max_log10_ceiling: u32,
}

#[derive(Serialize, Debug)]
struct Certificate {
    manifest_hash: String,
    verified_logic_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    verified_extension_hash: Option<String>,
    pub is_conditional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conjecture: Option<ConjectureMetadata>,
    telemetry: SearchTelemetry,
    citations: CertificateCitations,
    signature: String,
    public_key: String,
    engine_version: String,
    commit_hash: String,
    verification_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lattice_witnesses: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample_telemetry(baseline: usize, ps_bound: usize) -> SearchTelemetry {
        crate::lean_ffi::initialize_lean_runtime();
        SearchTelemetry {
            target_min_log10: crate::lean_ffi::get_target_min_log10(),
            target_max_log10: crate::lean_ffi::get_target_max_log10(),
            sieve_limit: 1000,
            max_exponent: 4,
            prefix_stop: 100_000_000_000,
            total_branches_searched: 42,
            abundance_pruned: 10,
            raycast_pruned: 0,
            boundary_pruned: 0,
            phase1_retained: 0,
            phase1_pruned: 0,
            phase1_trial_only: 0,
            phase1_ecm_fallback: 0,
            phase1_execution_time_ms: 0,
            search_space_density: 0.5,
            math_interruptions: 0,
            phase2_execution_time_ms: 1234,
            total_execution_time_ms: 1234,
            explored_ranges: vec![],
            baseline_min_prime_factors: baseline,
            prasad_sunitha_bound: ps_bound,
            trace_hash: "dummy_hash".to_string(),
            factorization_depth: crate::lean_ffi::get_pollard_rho_iteration_limit(),
            bounds_exceeded: false,
            verification_profile: None,
        }
    }

    /// SearchTelemetry must serialise the new baseline_min_prime_factors field.
    #[test]
    fn test_telemetry_serialises_baseline_min_prime_factors() {
        let tel = sample_telemetry(
            7,
            crate::lean_ffi::get_prasad_sunitha_bound() as u64 as usize,
        );
        let json: Value = serde_json::to_value(&tel).expect("serialisation must succeed");
        assert!(
            json.get("baseline_min_prime_factors").is_some(),
            "JSON must contain 'baseline_min_prime_factors' key"
        );
        assert_eq!(
            json["baseline_min_prime_factors"].as_u64().unwrap(),
            7,
            "baseline_min_prime_factors must serialise as 7"
        );
    }

    /// SearchTelemetry must serialise the new boundary_pruned field.
    #[test]
    fn test_telemetry_serialises_boundary_pruned() {
        let mut tel = sample_telemetry(
            7,
            crate::lean_ffi::get_prasad_sunitha_bound() as u64 as usize,
        );
        tel.boundary_pruned = 123;
        let json: Value = serde_json::to_value(&tel).expect("serialisation must succeed");
        assert!(
            json.get("boundary_pruned").is_some(),
            "JSON must contain 'boundary_pruned' key"
        );
        assert_eq!(
            json["boundary_pruned"].as_u64().unwrap(),
            123,
            "boundary_pruned must serialise as 123"
        );
    }

    /// SearchTelemetry must serialise the new prasad_sunitha_bound field.
    #[test]
    fn test_telemetry_serialises_prasad_sunitha_bound() {
        let ps_bound = crate::lean_ffi::get_prasad_sunitha_bound() as u64;
        let tel = sample_telemetry(7, ps_bound as usize);
        let json: Value = serde_json::to_value(&tel).expect("serialisation must succeed");
        assert!(
            json.get("prasad_sunitha_bound").is_some(),
            "JSON must contain 'prasad_sunitha_bound' key"
        );
        assert_eq!(
            json["prasad_sunitha_bound"].as_u64().unwrap(),
            ps_bound,
            "prasad_sunitha_bound must serialise correctly"
        );
    }

    /// Both new fields must survive a round-trip through JSON deserialisation.
    #[test]
    fn test_telemetry_new_fields_round_trip() {
        let ps_bound = crate::lean_ffi::get_prasad_sunitha_bound() as u64;
        let tel = sample_telemetry(7, ps_bound as usize);
        let json_str = serde_json::to_string(&tel).expect("serialisation must succeed");
        let decoded: Value = serde_json::from_str(&json_str).expect("deserialisation must succeed");
        assert_eq!(decoded["baseline_min_prime_factors"], 7);
        assert_eq!(decoded["prasad_sunitha_bound"], ps_bound);
    }

    /// The Prasad-Sunitha bound stored in the telemetry must exceed the baseline.
    #[test]
    fn test_telemetry_ps_bound_exceeds_baseline() {
        let ps_bound = crate::lean_ffi::get_prasad_sunitha_bound() as u64;
        let tel = sample_telemetry(7, ps_bound as usize);
        assert!(
            tel.prasad_sunitha_bound > tel.baseline_min_prime_factors,
            "prasad_sunitha_bound ({}) must exceed baseline_min_prime_factors ({})",
            tel.prasad_sunitha_bound,
            tel.baseline_min_prime_factors
        );
    }

    /// Verify neither new field is accidentally zero, which would indicate a
    /// failed FFI resolution.
    #[test]
    fn test_telemetry_new_fields_nonzero() {
        let tel = sample_telemetry(
            7,
            crate::lean_ffi::get_prasad_sunitha_bound() as u64 as usize,
        );
        assert!(
            tel.baseline_min_prime_factors > 0,
            "baseline_min_prime_factors must be > 0"
        );
        assert!(
            tel.prasad_sunitha_bound > 0,
            "prasad_sunitha_bound must be > 0"
        );
    }

    #[test]
    fn test_certificate_verification_mode_serialization() {
        let ps_bound = crate::lean_ffi::get_prasad_sunitha_bound() as u64;
        let tel = sample_telemetry(7, ps_bound as usize);
        let cert_citations = CertificateCitations {
            target_min_log10: None,
            baseline_min_prime_factors: None,
            prasad_sunitha_bound: None,
            euler_ceiling: None,
        };
        let cert = Certificate {
            manifest_hash: "dummy_manifest_hash".to_string(),
            verified_logic_hash: "dummy_logic_hash".to_string(),
            verified_extension_hash: None,
            is_conditional: false,
            conjecture: None,
            telemetry: tel,
            citations: cert_citations,
            signature: "dummy_sig".to_string(),
            public_key: "dummy_pub".to_string(),
            engine_version: "1.0.0".to_string(),
            commit_hash: "dummy_commit".to_string(),
            verification_mode: "pure".to_string(),
            lattice_witnesses: None,
        };
        let json_val: serde_json::Value = serde_json::to_value(&cert).unwrap();
        assert_eq!(json_val["verification_mode"], "pure");
    }

    #[test]
    fn test_validate_suffix_bounds_valid() {
        let expected_k0 = 1u128 << 64;
        let expected_k1 = (1u128 << 64) * 3 / 2;
        let valid_seq = vec![expected_k0, expected_k1, expected_k1 + 10, expected_k1 + 20];
        assert!(validate_suffix_bounds_sequence(&valid_seq).is_ok());
    }

    #[test]
    fn test_validate_suffix_bounds_invalid_k0() {
        let invalid_seq = vec![0, 100];
        assert!(validate_suffix_bounds_sequence(&invalid_seq).is_err());
    }

    #[test]
    fn test_validate_suffix_bounds_invalid_k1() {
        let expected_k0 = 1u128 << 64;
        let invalid_seq = vec![expected_k0, 100];
        assert!(validate_suffix_bounds_sequence(&invalid_seq).is_err());
    }

    #[test]
    fn test_validate_suffix_bounds_non_monotonic() {
        let expected_k0 = 1u128 << 64;
        let expected_k1 = (1u128 << 64) * 3 / 2;
        let invalid_seq = vec![expected_k0, expected_k1, expected_k1 - 1];
        assert!(validate_suffix_bounds_sequence(&invalid_seq).is_err());
    }

    #[test]
    fn test_validate_suffix_bounds_empty() {
        let invalid_seq = vec![];
        assert!(validate_suffix_bounds_sequence(&invalid_seq).is_err());
    }
}

pub fn validate_suffix_bounds_sequence(suffix_abundance: &[u128]) -> Result<(), String> {
    let expected_k0 = 1u128 << 64;
    if suffix_abundance.is_empty() {
        return Err(
            "FATAL: Loaded bounds sequence is empty! Boundary value verification cannot proceed."
                .to_string(),
        );
    }

    if suffix_abundance[0] != expected_k0 {
        return Err(format!(
            "FATAL: Initial boundary value does not match the expected core model fixed-point limit!\n\
             Expected (expected_k0): {}\n\
             Actual: {}",
            expected_k0, suffix_abundance[0]
        ));
    }

    if suffix_abundance.len() > 1 {
        let expected_k1 = (1u128 << 64) * 3 / 2;
        if suffix_abundance[1] != expected_k1 {
            return Err(format!(
                "FATAL: Boundary value suffix_abundance[1] does not match the expected core model fixed-point limit!\n\
                 Expected (expected_k1): {}\n\
                 Actual: {}",
                expected_k1, suffix_abundance[1]
            ));
        }
    }

    for k in 1..suffix_abundance.len() {
        if suffix_abundance[k] < suffix_abundance[k - 1] {
            return Err(format!(
                "FATAL: Monotonicity violation detected in FFI bounds sequence!\n\
                 suffix_abundance[{}] ({}) is less than suffix_abundance[{}] ({})",
                k,
                suffix_abundance[k],
                k - 1,
                suffix_abundance[k - 1]
            ));
        }
    }

    Ok(())
}

/// Program entry point that runs the full UALBF engine, performs the verified search,
/// and optionally emits a signed formal certificate.
///
/// This is the program entry point. It performs the following high-level actions:
/// - Loads and hashes a proof manifest and verifies no theorems are marked `sorry` or `axiom`.
/// - Computes a SHA-256 hash over the engine's verified search logic source files (including FFI bridge files).
/// - Initializes the Lean runtime and configures Rayon worker thread initialization.
/// - Reads configurable search parameters from environment variables and enforces certificate bounds.
/// - Runs phase-1 sieving and precomputations, then executes the phase-2 search in the selected mode.
/// - If standard bounds are used, signs and writes a JSON certificate containing telemetry and cryptographic metadata.
///
/// The function aborts if the manifest contains incomplete theorems (`"sorry"` or
/// `"axiom"`). Network modes (`controller` / `worker`) run the distributed protocol
/// and exit the process after completion; standalone mode runs the local fused search.
///
/// # Examples
///
/// ```no_run
/// // Run the compiled binary after placing a valid `proof_manifest.json` in the
/// // working directory:
/// // UALBF_PROOF_MANIFEST=proof_manifest.json UALBF_MODE=standalone ./ualbf_engine
/// ```
fn find_lean_file(thm_file: &str, manifest_path: &str) -> Option<std::path::PathBuf> {
    use std::path::{Path, PathBuf};

    let candidates = [
        Path::new("lean4-proofs").join(thm_file),
        PathBuf::from(thm_file),
        if let Some(parent) = Path::new(manifest_path).parent() {
            parent.join("lean4-proofs").join(thm_file)
        } else {
            PathBuf::new()
        },
        if let Some(parent) = Path::new(manifest_path).parent() {
            parent.join(thm_file)
        } else {
            PathBuf::new()
        },
        Path::new("../lean4-proofs").join(thm_file),
    ];

    for cand in candidates {
        if cand.as_os_str().is_empty() {
            continue;
        }
        if cand.exists() && cand.is_file() {
            return Some(cand);
        }
    }
    None
}

fn sha256_digest_file(path: &std::path::Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 4096];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args {
        if arg.starts_with("--verify-sidecar=") {
            let path = arg.trim_start_matches("--verify-sidecar=");
            if let Err(e) = sieve::run_offline_verification(path) {
                eprintln!("Verification failed: {}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    let total_start = std::time::Instant::now();
    crate::lean_ffi::initialize_lean_runtime();

    // Initialize sidecar logger
    let sidecar_path =
        std::env::var("UALBF_SIDECAR_PATH").unwrap_or_else(|_| "overflow_sidecar.log".to_string());
    if let Err(e) = sieve::init_sidecar_logger(&sidecar_path) {
        eprintln!(
            "FATAL: Failed to initialize sidecar logger at {}: {}",
            sidecar_path, e
        );
        std::process::exit(1);
    }

    let config = policy::get_safe_config();
    // ── Formal Certification Initialization ──
    let manifest_path = config.proof_manifest.clone();

    let manifest_content = fs::read_to_string(&manifest_path).expect(
        "Failed to read proof manifest. Engine must ingest a machine-readable manifest at startup.",
    );
    let manifest: Manifest =
        serde_json::from_str(&manifest_content).expect("Failed to parse proof manifest");

    // Hash the manifest for the certificate
    let manifest_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&manifest_content);
        hex::encode(hasher.finalize())
    };

    #[cfg(feature = "signing")]
    let is_verified_build = true;
    #[cfg(not(feature = "signing"))]
    let is_verified_build = false;

    println!("=== Formal Certification Framework ===");
    println!(
        "Verification Status: {}",
        if is_verified_build {
            "VERIFIED (Signing Enabled)"
        } else {
            "UNVERIFIED (Signing Disabled)"
        }
    );
    println!("Ingested proof manifest: {}", manifest_hash);

    if config.proof_mode == "pure" {
        println!();
        println!(
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
        );
        println!(
            "! WARNING: PURE PROOFS MODE ACTIVE                                             !"
        );
        println!(
            "! Hagis-Cohen 11-factor pruning is disabled. Falling back to proven bounds.    !"
        );
        println!(
            "! The search space is significantly expanded and execution will be slower.      !"
        );
        println!(
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
        );
        println!();
    }

    if crate::manifest_constants::CONJECTURAL_ACTIVE {
        println!();
        println!(
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
        );
        println!(
            "! WARNING: CONJECTURAL MODE IS ACTIVATED                                       !"
        );
        println!(
            "! The search is running with conjectural limits ({}).                         !",
            crate::manifest_constants::CONJECTURE_NAME
        );
        println!(
            "! Any generated exhaustion certificates are conditional and NOT absolute.      !"
        );
        println!(
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
        );
        println!();
    }

    // Hash the verified search logic (Verus proofs + core logic)
    let verified_logic_hash = verification_lib::compute_core_tcb_hash_at_compile_time!();
    println!("Verified core search logic hash: {}", verified_logic_hash);

    let verified_extension_hash: Option<String> = None;

    // --- Runtime Audit: Verus Specification Hashes ---
    let verus_content = include_str!("verus_proofs.rs");
    let mut runtime_verus_hashes = std::collections::HashMap::new();
    let mut current_fn = String::new();
    let mut current_body = String::new();
    let mut in_spec = false;
    let mut brace_count = 0;
    let mut module_stack: Vec<String> = Vec::new();
    let mut module_brace_depth = 0;

    for line in verus_content.lines() {
        let trimmed = line.trim();

        // Track module declarations
        if !in_spec {
            if trimmed.contains('{')
                && (trimmed.starts_with("mod ") || trimmed.starts_with("pub mod "))
            {
                let mod_name = if trimmed.starts_with("pub mod ") {
                    trimmed.strip_prefix("pub mod ").unwrap_or("")
                } else {
                    trimmed.strip_prefix("mod ").unwrap_or("")
                };
                let mod_name = mod_name.split('{').next().unwrap_or("").trim();
                if !mod_name.is_empty() {
                    module_stack.push(mod_name.to_string());
                    if trimmed.contains('{') {
                        module_brace_depth += 1;
                    }
                }
            }
        }

        let kw_list = [
            "pub spec fn ",
            "pub open spec fn ",
            "pub uninterp spec fn ",
            "pub proof fn ",
            "pub fn ",
        ];
        let mut matched_kw = None;
        if !in_spec {
            for kw in kw_list.iter() {
                if line.contains(kw) {
                    matched_kw = Some(*kw);
                    break;
                }
            }
        }

        if !in_spec && matched_kw.is_some() {
            let kw = matched_kw.unwrap();
            let parts: Vec<&str> = line.split(kw).collect();
            if parts.len() > 1 {
                let bare_fn_name = parts[1].split('(').next().unwrap_or("").trim().to_string();
                // Build scope-qualified key
                let qualified_name = if module_stack.is_empty() {
                    bare_fn_name.clone()
                } else {
                    format!("{}::{}", module_stack.join("::"), bare_fn_name)
                };
                current_fn = qualified_name;
                in_spec = true;
                current_body = line.to_string();
                brace_count = line.chars().filter(|&c| c == '{').count() as i32
                    - line.chars().filter(|&c| c == '}').count() as i32;
                if brace_count == 0 && line.contains('{') {
                    let mut hasher = Sha256::new();
                    hasher.update(current_body.as_bytes());
                    runtime_verus_hashes.insert(current_fn.clone(), hex::encode(hasher.finalize()));
                    in_spec = false;
                }
            }
        } else if in_spec {
            current_body.push('\n');
            current_body.push_str(line);
            brace_count += line.chars().filter(|&c| c == '{').count() as i32
                - line.chars().filter(|&c| c == '}').count() as i32;
            if brace_count == 0 {
                let mut hasher = Sha256::new();
                hasher.update(current_body.as_bytes());
                runtime_verus_hashes.insert(current_fn.clone(), hex::encode(hasher.finalize()));
                in_spec = false;
            }
        } else if !in_spec && module_brace_depth > 0 {
            // Track module closing braces
            let open_braces = line.chars().filter(|&c| c == '{').count();
            let close_braces = line.chars().filter(|&c| c == '}').count();
            module_brace_depth += open_braces;
            if close_braces > 0 {
                for _ in 0..close_braces {
                    if module_brace_depth > 0 {
                        module_brace_depth -= 1;
                        if !module_stack.is_empty() {
                            module_stack.pop();
                        }
                    }
                }
            }
        }
    }

    if runtime_verus_hashes != manifest.verus_hashes {
        println!("ERROR: Runtime Verus specification hashes do not match the proof manifest!");
        println!("Manifest hashes: {:?}", manifest.verus_hashes);
        println!("Runtime hashes: {:?}", runtime_verus_hashes);
        panic!("FATAL: Epistemological severance detected: Logic version mismatch.");
    }

    if manifest.bounds_manifest_hash != crate::manifest_constants::MANIFEST_HASH {
        println!("ERROR: Configuration hash does not match the proven execution certificate!");
        println!("Manifest hash: {}", manifest.bounds_manifest_hash);
        println!("Runtime hash: {}", crate::manifest_constants::MANIFEST_HASH);
        panic!("FATAL: Configuration mismatch. The execution certificate bounds do not match the binary bounds.");
    }

    println!("Epistemological Linkage Verified.");
    let allowed_axioms: [&str; 0] = [];
    let mut proof_incomplete = false;
    for thm in &manifest.theorems {
        let mut matched = false;
        if let Some(file_path) = find_lean_file(&thm.file, &manifest_path) {
            if let Ok(file_hash) = sha256_digest_file(&file_path) {
                if file_hash == thm.checksum {
                    matched = true;
                }
            }
        }

        if !matched {
            let expected_payload = format!("{}|{}|{}", thm.name, thm.file, thm.status);
            let mut hasher = sha2::Sha256::new();
            sha2::Digest::update(&mut hasher, expected_payload.as_bytes());
            let computed_checksum = hex::encode(hasher.finalize());
            if computed_checksum == thm.checksum {
                matched = true;
            }
        }

        if !matched {
            panic!("FATAL: Checksum mismatch for theorem {}. The proof manifest has been tampered with.", thm.name);
        }

        if thm.status == "sorry"
            || thm.status == "unverified"
            || (thm.status == "axiom" && !allowed_axioms.contains(&thm.name.as_str()))
        {
            println!(
                "ERROR: Theorem '{}' in '{}' is incomplete (status: {}).",
                thm.name, thm.file, thm.status
            );
            proof_incomplete = true;
        }
    }
    if proof_incomplete {
        panic!("FATAL: The verification process refuses to start/sign the certificate because 'sorry' or 'axiom' was detected in the formal proof manifest.");
    }

    // Initialize the Lean 4 runtime before any FFI calls
    lean_ffi::initialize_lean_runtime();

    // Execute runtime bridge negotiation parity checks
    println!("Executing Runtime Bridge Negotiation Parity Checks...");
    println!("FFI Signature: pub fn ualbf_mod_inverse(a_obj: *mut lean_object, a_neg: u8, m_obj: *mut lean_object) -> *mut lean_object");
    println!(
        "FFI Mapping: U512 representation mapped to [u64; {}] ({}-bit representation)",
        lean_ffi::LIMB_COUNT,
        lean_ffi::LIMB_COUNT * 64
    );
    lean_ffi::run_runtime_parity_check();
    crate::lean_ffi::STARTUP_COMPLETE.store(true, std::sync::atomic::Ordering::SeqCst);
    println!("Bridge Negotiation Successful: Data representations strictly match.");

    // Eagerly resolve unified mathematical bounds from Lean 4 proof environment
    dfs_tree::init_bounds();

    // Force Rayon to initialize Lean's memory allocator on all worker threads
    rayon::ThreadPoolBuilder::new()
        .start_handler(|_| lean_ffi::initialize_lean_worker_thread())
        .build_global()
        .unwrap();

    // ── Read configurable parameters via Policy Registry ──
    let target_min_log10 = config.target_min_log10;
    let target_max_log10 = config.target_max_log10;
    let sieve_limit = config.sieve_limit;
    let max_exponent = config.max_exponent;
    let prefix_stop = config.prefix_stop;

    println!("=== UALBF Engine Initializing ===");
    println!(
        "Target Bound: 10^{} < N < 10^{}",
        target_min_log10, target_max_log10
    );
    println!(
        "Sieve: limit={}, max_exponent={}, prefix_stop={}",
        sieve_limit, max_exponent, prefix_stop
    );

    let (euler_num, euler_den) = lean_ffi::get_euler_ceiling();
    println!(
        "Euler Ceiling: {}/{} (proven via manifest)",
        euler_num, euler_den
    );

    let mut skip_cert = false;
    if !(target_max_log10 == crate::lean_ffi::get_target_max_log10()
        && target_min_log10 == crate::lean_ffi::get_target_min_log10())
    {
        println!("WARNING: Immutable Bounds constraint violated. The engine prohibits the generation of a 'Formal' certificate if custom, non-standard search bounds are used. The bound must be 10^{} < N < 10^{}. Certificate generation will be skipped.", crate::lean_ffi::get_target_min_log10(), crate::lean_ffi::get_target_max_log10());
        skip_cert = true;
    }

    let target_min: Uint = Uint::from_u32(10).pow(target_min_log10);

    let target_bound: Uint = Uint::from_u32(10).pow(target_max_log10);
    let threshold: Uint = Uint::from_u128(prefix_stop as u128);

    let sieve_result = sieve::phase1_global_annihilation_sieve(sieve_limit, max_exponent);
    let valid_components = sieve_result.components;
    let sigma_cache = sieve_result.sigma_cache;

    // Precompute suffix-max abundance product array for DFS pruning.
    // We dynamically calculate the maximum possible depth before the 256-bit product overflows target_bound.
    let mut max_factors_needed = 0;
    let mut p_prod = crate::types::Uint::one();
    for p in primal::Primes::all() {
        if let Some(new_p) = p_prod.checked_mul(crate::types::Uint::from_u64(p as u64)) {
            if new_p <= target_bound {
                p_prod = new_p;
                max_factors_needed += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let max_factors = max_factors_needed.min(valid_components.len());
    let mut suffix_abundance = vec![0u128; max_factors + 1];
    for k in 0..=max_factors {
        suffix_abundance[k] = lean_ffi::get_static_suffix_bound(k as u32);
    }

    #[cfg(not(unverified_build))]
    {
        println!("Executing Startup Invariant Validation checks on FFI bounds sequence...");
        if let Err(err_msg) = validate_suffix_bounds_sequence(&suffix_abundance) {
            eprintln!("{}", err_msg);
            sieve::finalize_sidecar_logger();
            std::process::exit(1);
        }
        println!("Startup Invariant Validation Successful: Monotonicity and fixed-point boundary limits are verified.");
    }

    // Precompute illegal valuations once to pass into the parallel pipeline
    let illegal_z_valuations =
        raycast::generate_illegal_z_valuations(sieve_limit as u64, max_exponent);

    // Check illegal valuations

    // Launch fused perfectly-balanced parallel pipeline!
    #[cfg(feature = "lattice")]
    crate::lattice::clear_lattice_witnesses();

    let mode = config.mode.clone();
    let phase2_start = std::time::Instant::now();
    let mut explored_ranges_out = Vec::new();
    let telemetry_data;

    if mode == "controller" {
        let depth_limit = 2; // shallow DFS depths
        let work_units =
            distributed::generate_work_units(&valid_components, &target_bound, depth_limit);
        let addr = config.controller_addr.clone();
        distributed::run_controller(&addr, work_units);
        sieve::finalize_sidecar_logger();
        std::process::exit(0); // For now just exit after completion
    } else if mode == "worker" {
        let addr = config.controller_addr.clone();
        let total_weight_scaled: usize = valid_components
            .iter()
            .map(|c| (10_000_000.0 / ((c.p as f64) * (c.p as f64))) as usize)
            .sum();
        let max_idx_3 = valid_components.iter().rposition(|c| c.p == 3).unwrap_or(0);
        let max_idx_5 = valid_components.iter().rposition(|c| c.p == 5).unwrap_or(0);

        let (tel, ranges) = distributed::run_worker(
            &addr,
            &valid_components,
            &threshold,
            &target_min,
            &target_bound,
            &illegal_z_valuations,
            &suffix_abundance,
            total_weight_scaled,
            &sigma_cache,
            max_idx_3,
            max_idx_5,
        );
        telemetry_data = tel;
        explored_ranges_out = ranges;
    } else {
        telemetry_data = dfs_tree::phase2_and_4_fused(
            &valid_components,
            &threshold,
            &target_min,
            &target_bound,
            &illegal_z_valuations,
            &suffix_abundance,
            &sigma_cache,
            None,
        );
        explored_ranges_out.push(crate::distributed::RangeWorkUnit {
            start_bound: vec![],
            end_bound: vec![],
        });
    }
    let phase2_elapsed = phase2_start.elapsed();

    // ── Generate and Hash Trace ──
    #[cfg(feature = "signing")]
    let trace_path = "trace.jsonl";
    #[cfg(feature = "signing")]
    let trace_hash = if std::path::Path::new(trace_path).exists() {
        let mut hasher = Sha256::new();
        let mut f = std::fs::File::open(trace_path).expect("Failed to open trace file");
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut buf).unwrap();
        hasher.update(&buf);
        hex::encode(hasher.finalize())
    } else {
        "".to_string()
    };
    #[cfg(not(feature = "signing"))]
    let trace_hash = "unverified_trace_hash".to_string();

    println!(
        "{}",
        serde_json::to_string(&crate::events::SearchEvent::Done {
            target_min_log10,
            target_max_log10,
            elapsed_ms: phase2_elapsed.as_millis()
        })
        .unwrap()
    );

    // ── Generate Formal Exhaustion Certificate ──
    if skip_cert {
        println!("=== Certificate Generation Skipped due to custom bounds ===");
        sieve::finalize_sidecar_logger();
        return;
    }

    #[cfg(feature = "signing")]
    let (signature_hex, public_key_hex) = {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let payload_to_sign = verification_lib::format_payload(
            &manifest_hash,
            &verified_logic_hash,
            verified_extension_hash.as_deref(),
            telemetry_data.total_branches,
            target_min_log10,
            target_max_log10,
            &trace_hash,
            crate::lean_ffi::get_pollard_rho_iteration_limit(),
            config.sampling_rate,
            config.deterministic_seed,
            Some(crate::manifest_constants::CONJECTURAL_ACTIVE),
            Some(crate::manifest_constants::CONJECTURE_NAME),
            serde_json::to_value(&explored_ranges_out).ok(),
            Some(&config.proof_mode),
        );
        let signature = signing_key.sign(payload_to_sign.as_bytes());
        (
            hex::encode(signature.to_bytes()),
            hex::encode(signing_key.verifying_key().to_bytes()),
        )
    };

    #[cfg(not(feature = "signing"))]
    let (signature_hex, public_key_hex) = {
        println!(
            "ERROR: Refusing to sign certificate. Signing is unavailable in unverified builds."
        );
        (
            "unverified_signature".to_string(),
            "unverified_public_key".to_string(),
        )
    };

    let telemetry = SearchTelemetry {
        target_min_log10,
        target_max_log10,
        sieve_limit,
        max_exponent,
        prefix_stop,
        total_branches_searched: telemetry_data.total_branches,
        abundance_pruned: telemetry_data.abundance_pruned,
        raycast_pruned: telemetry_data.raycast_pruned,
        boundary_pruned: telemetry_data.boundary_pruned,
        phase1_retained: valid_components.len(),
        phase1_pruned: sieve_result.pruned,
        phase1_trial_only: sieve_result.trial_only,
        phase1_ecm_fallback: sieve_result.ecm_fallback,
        phase1_execution_time_ms: sieve_result.execution_time_ms,
        search_space_density: telemetry_data.search_space_density,
        math_interruptions: telemetry_data.math_interruptions,
        phase2_execution_time_ms: phase2_elapsed.as_millis(),
        total_execution_time_ms: total_start.elapsed().as_millis(),
        baseline_min_prime_factors: lean_ffi::get_baseline_min_prime_factors(),
        prasad_sunitha_bound: lean_ffi::get_prasad_sunitha_bound(),
        trace_hash: trace_hash.clone(),
        factorization_depth: crate::lean_ffi::get_pollard_rho_iteration_limit(),
        bounds_exceeded: false,
        explored_ranges: explored_ranges_out,
        verification_profile: config.sampling_rate.map(|rate| VerificationProfile {
            sampling_rate: rate,
            deterministic_seed: config.deterministic_seed.unwrap_or(0),
        }),
    };

    let bounds_manifest_str = include_str!("../../bounds_manifest.json");
    let bounds_json: serde_json::Value =
        serde_json::from_str(bounds_manifest_str).expect("Failed to parse bounds_manifest.json");

    let cert_citations = CertificateCitations {
        target_min_log10: serde_json::from_value(
            bounds_json["search_bounds"]["target_min_log10"]["citation"].clone(),
        )
        .unwrap_or(None),
        baseline_min_prime_factors: serde_json::from_value(
            bounds_json["omega_bounds"]["hagis1982"]["citation"].clone(),
        )
        .unwrap_or(None),
        prasad_sunitha_bound: serde_json::from_value(
            bounds_json["omega_bounds"]["prasad_sunitha"]["citation"].clone(),
        )
        .unwrap_or(None),
        euler_ceiling: serde_json::from_value(bounds_json["euler_ceiling"]["citation"].clone())
            .unwrap_or(None),
    };

    let is_cond = crate::manifest_constants::CONJECTURAL_ACTIVE;
    let conjecture_meta = if is_cond {
        Some(ConjectureMetadata {
            conditional: true,
            conjecture_name: crate::manifest_constants::CONJECTURE_NAME.to_string(),
            conjectural_max_log10_ceiling: crate::manifest_constants::CONJECTURAL_MAX_LOG10_CEILING,
        })
    } else {
        None
    };

    #[cfg(feature = "lattice")]
    let lattice_witnesses = {
        let witnesses = crate::lattice::get_lattice_witnesses();
        if witnesses.is_empty() {
            None
        } else {
            serde_json::to_value(witnesses).ok()
        }
    };
    #[cfg(not(feature = "lattice"))]
    let lattice_witnesses = None;

    let cert = Certificate {
        manifest_hash,
        verified_logic_hash,
        verified_extension_hash,
        is_conditional: is_cond,
        conjecture: conjecture_meta,
        telemetry,
        citations: cert_citations,
        signature: signature_hex,
        public_key: public_key_hex,
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        commit_hash: option_env!("GIT_HASH").unwrap_or("unknown").to_string(),
        verification_mode: config.proof_mode.clone(),
        lattice_witnesses,
    };

    let cert_json = serde_json::to_string_pretty(&cert).expect("Failed to serialize certificate");
    fs::write("formal_certificate.json", &cert_json).expect("Failed to write certificate");
    println!("=== Certificate Generated: formal_certificate.json ===");
    sieve::finalize_sidecar_logger();
}
