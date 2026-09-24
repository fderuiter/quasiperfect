#![allow(warnings)]
// build.rs — Compile Lean 4 C-IR into libUALBF.a, then link it with the Lean runtime.
#![allow(dead_code, clippy::needless_borrows_for_generic_args)]

use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn is_prime(n: u64) -> bool {
    if n <= 1 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Theorem {
    pub name: String,
    pub file: String,
    pub status: String,
    pub checksum: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GhostBinding {
    pub lean_theorem: String,
    pub theorem_hash: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProofManifest {
    pub theorems: Vec<Theorem>,
    pub verified_logic_hash: String,
    pub verified_extension_hash: String,
    pub verus_hashes: HashMap<String, String>,
    pub ghost_pruning_bindings: Option<HashMap<String, GhostBinding>>,
    pub proof_files: Vec<serde_json::Value>,
    pub bounds_manifest_hash: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub author: String,
    pub year: String,
    pub title: String,
    pub identifier: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrasadSunithaBounds {
    pub proof_bound: u64,
    pub engine_justified_gap: u64,
    pub is_axiomatic: bool,
    pub citation: Option<Citation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BaselineBounds {
    pub proof_bound: u64,
    pub engine_justified_gap: u64,
    pub is_axiomatic: bool,
    pub citation: Option<Citation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BoundValueU32 {
    pub value: u32,
    pub is_axiomatic: bool,
    pub citation: Option<Citation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BoundValueU64 {
    pub value: u64,
    pub is_axiomatic: bool,
    pub citation: Option<Citation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BoundValueUsize {
    pub value: usize,
    pub is_axiomatic: bool,
    pub citation: Option<Citation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PollardRhoBounds {
    pub iteration_limit: u32,
    pub batch_size: u32,
    pub is_axiomatic: bool,
    pub citation: Option<Citation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RaycastBounds {
    pub gpu_threshold: usize,
    pub chunk_size: usize,
    pub is_axiomatic: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchBounds {
    pub target_min_log10: BoundValueU32,
    pub target_max_log10: BoundValueU32,
    pub sieve_limit: BoundValueUsize,
    pub max_exponent: BoundValueU32,
    pub prefix_stop_threshold: BoundValueU64,
    pub pollard_rho: PollardRhoBounds,
    pub raycast: RaycastBounds,
    pub prime_split_threshold: Option<BoundValueU64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OmegaBounds {
    pub prasad_sunitha: PrasadSunithaBounds,
    pub div_5_coprime_3: Option<PrasadSunithaBounds>,
    pub hagis1982: BaselineBounds,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EulerCeiling {
    pub num: u64,
    pub den: u64,
    pub is_axiomatic: bool,
    pub citation: Option<Citation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OverflowThreshold {
    pub num: u64,
    pub den: u64,
    pub is_axiomatic: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConjecturalBounds {
    pub active: bool,
    pub conjecture_name: String,
    pub target_max_log10_ceiling: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BoundsManifest {
    pub omega_bounds: OmegaBounds,
    pub search_bounds: SearchBounds,
    pub euler_ceiling: EulerCeiling,
    pub overflow_threshold: OverflowThreshold,
    pub conjectural_bounds: Option<ConjecturalBounds>,
}

// --- Pure Helper Functions for Manifest Parsing & Validation ---

pub fn parse_bounds_manifest(content: &str) -> Result<BoundsManifest, String> {
    serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse bounds_manifest.json: {}", e))
}

pub fn validate_bounds_manifest(manifest: &BoundsManifest) -> Result<(), String> {
    if let Some(ref prime_split) = manifest.search_bounds.prime_split_threshold {
        let val = prime_split.value;
        if val != 61 {
            return Err(format!(
                "FATAL: Invalid configuration! The configured prime split threshold ({}) does not equal the baseline of 61. The threshold must equal 61.",
                val
            ));
        }
    } else {
        return Err("FATAL: prime_split_threshold not found in bounds_manifest.json!".to_string());
    }

    if let Some(ref cb) = manifest.conjectural_bounds {
        if cb.active {
            let floor = manifest.search_bounds.target_min_log10.value;
            let ceiling = cb.target_max_log10_ceiling;
            if ceiling < floor {
                return Err(format!(
                    "FATAL: Conflicting bounds parameters detected! Active conjectural ceiling (target_max_log10_ceiling = {}) is set below the target search floor (target_min_log10 = {}). This configuration is invalid.",
                    ceiling, floor
                ));
            }
        }
    }

    if manifest.omega_bounds.hagis1982.is_axiomatic
        && manifest.omega_bounds.hagis1982.citation.is_none()
    {
        return Err(
            "FATAL: baseline bound marked axiomatic but lacks citation metadata.".to_string(),
        );
    }
    if manifest.search_bounds.target_min_log10.is_axiomatic {
        return Err(
            "FATAL: search engine floor (target_min_log10) cannot rely on axiomatic assumptions."
                .to_string(),
        );
    }
    if manifest.omega_bounds.prasad_sunitha.is_axiomatic
        && manifest.omega_bounds.prasad_sunitha.citation.is_none()
    {
        return Err(
            "FATAL: prasad_sunitha marked axiomatic but lacks citation metadata.".to_string(),
        );
    }
    if let Some(ref div_5) = manifest.omega_bounds.div_5_coprime_3 {
        if div_5.is_axiomatic && div_5.citation.is_none() {
            return Err(
                "FATAL: div_5_coprime_3 marked axiomatic but lacks citation metadata.".to_string(),
            );
        }
    }
    if manifest.euler_ceiling.is_axiomatic && manifest.euler_ceiling.citation.is_none() {
        return Err(
            "FATAL: euler_ceiling marked axiomatic but lacks citation metadata.".to_string(),
        );
    }

    let target_min_log10 = manifest.search_bounds.target_min_log10.value;
    let target_max_log10 = manifest.search_bounds.target_max_log10.value;
    if target_min_log10 > target_max_log10 {
        return Err(format!(
            "FATAL: target_min_log10 ({}) exceeds target_max_log10 ({}). Inverted range boundaries are not permitted.",
            target_min_log10, target_max_log10
        ));
    }

    let prasad_proof = manifest.omega_bounds.prasad_sunitha.proof_bound;
    let primes = [
        7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83,
    ];
    let mut min_val: f64 = 1.0;
    for &p in primes.iter().take(prasad_proof as usize) {
        min_val *= (p as f64) * (p as f64);
    }
    let verified_floor = min_val.log10().floor() as u32;
    if target_max_log10 < verified_floor {
        return Err(format!(
            "FATAL: target_max_log10 ({}) cannot be lower than the highest available verified bound ({}).",
            target_max_log10, verified_floor
        ));
    }

    Ok(())
}

pub fn parse_proof_manifest(content: &str) -> Result<ProofManifest, String> {
    serde_json::from_str(content).map_err(|e| format!("Failed to parse proof_manifest.json: {}", e))
}

pub fn validate_proof_manifest(
    proof_manifest: &ProofManifest,
    current_bounds_manifest_hash: &str,
) -> Result<(), String> {
    if proof_manifest.bounds_manifest_hash != current_bounds_manifest_hash {
        return Err(format!(
            "FATAL: Configuration mismatch. The proof manifest bounds hash ('{}') does not match current bounds_manifest.json hash ('{}').",
            proof_manifest.bounds_manifest_hash,
            current_bounds_manifest_hash
        ));
    }

    let allowed_axioms = ["UALBF.QPN.PrasadSunitha.qpn_div_5_coprime_3_omega_bound"];
    for thm in &proof_manifest.theorems {
        let is_whitelisted = thm.status == "proven"
            || (thm.status == "axiom" && allowed_axioms.contains(&thm.name.as_str()));
        if !is_whitelisted {
            return Err(format!(
                "FATAL: Theorem '{}' in '{}' is incomplete (status: {}). Compilation halted.",
                thm.name, thm.file, thm.status
            ));
        }
    }

    let required_ghost_functions = [
        "check_starvation_kill",
        "check_cdg_forced_kill",
        "lean_abundancy_starvation_theorem",
        "verify_starvation_pruning",
        "lemma_sigma_multiplicative",
        "lemma_disjoint_by_construction",
    ];

    if let Some(ref bindings) = proof_manifest.ghost_pruning_bindings {
        let thm_map: HashMap<String, &Theorem> = proof_manifest
            .theorems
            .iter()
            .map(|t| (t.name.clone(), t))
            .collect();

        for fn_name in &required_ghost_functions {
            let binding = match bindings.get(*fn_name) {
                Some(b) => b,
                None => return Err(format!(
                    "FATAL: Search pruning assumption '{}' lacks a matching Lean 4 manifest entry in ghost_pruning_bindings!",
                    fn_name
                )),
            };

            let target_thm = match thm_map.get(&binding.lean_theorem) {
                Some(t) => t,
                None => {
                    return Err(format!(
                    "FATAL: Search pruning assumption '{}' references unknown Lean theorem '{}'!",
                    fn_name, binding.lean_theorem
                ))
                }
            };

            if target_thm.status != "proven" {
                return Err(format!(
                    "FATAL: Lean theorem '{}' bound to pruning assumption '{}' is incomplete (status: {}). Compilation halted.",
                    target_thm.name, fn_name, target_thm.status
                ));
            }

            if target_thm.checksum != binding.theorem_hash {
                return Err(format!(
                    "FATAL: SHA-256 hash mismatch for Lean theorem '{}' bound to pruning assumption '{}'! Expected: '{}', Found in binding: '{}'",
                    target_thm.name, fn_name, target_thm.checksum, binding.theorem_hash
                ));
            }
        }
    } else {
        return Err(
            "FATAL: proof_manifest.json is missing required 'ghost_pruning_bindings' field!"
                .to_string(),
        );
    }

    Ok(())
}

pub fn validate_schema_hash_sync(
    schema_manifest_content: &str,
    generated_rs_content: &str,
    error_message: &str,
) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(schema_manifest_content.as_bytes());
    let current_schema_hash = hex::encode(hasher.finalize());

    if let Some(idx) = generated_rs_content.find("pub const EXPORTED_SCHEMA_MANIFEST_HASH") {
        let rest = &generated_rs_content[idx..];
        let start = rest.find('"').unwrap_or(0) + 1;
        let end = rest[start..].find('"').unwrap_or(0) + start;
        if start < end {
            let recorded_hash = &rest[start..end];
            if current_schema_hash != recorded_hash {
                return Err(format!(
                    "{}\n\
                     Current hash : {}\n\
                     Recorded hash: {}\n\
                     Please run `scripts/export_lean_specs.py` to update before building.",
                    error_message, current_schema_hash, recorded_hash
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_bounds_hash_sync(
    manifest_content: &str,
    export_content: &str,
) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(manifest_content.as_bytes());
    let current_manifest_hash = hex::encode(hasher.finalize());

    if let Some(idx) = export_content.find("pub const EXPORTED_BOUNDS_MANIFEST_HASH") {
        let rest = &export_content[idx..];
        let start = rest.find('"').unwrap_or(0) + 1;
        let end = rest[start..].find('"').unwrap_or(0) + start;
        if start < end {
            let recorded_hash = &rest[start..end];
            if current_manifest_hash != recorded_hash {
                return Err(format!(
                    "FATAL: Mathematical Bound Synchronization Guardrail Triggered!\n\
                     The contents of 'bounds_manifest.json' have changed, but the Lean specifications \
                     have not been regenerated. This risks a silent desynchronization between \
                     mathematical bounds and verified specifications.\n\
                     Current hash : {}\n\
                     Recorded hash: {}\n\
                     Please run `scripts/export_lean_specs.py` (or `make rust`) to update the exported \
                     specifications before building the engine.",
                    current_manifest_hash, recorded_hash
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_constant_spec_equivalence(
    manifest_constants_content: &str,
    export_content: &str,
) -> Result<(), String> {
    let mut constants_map = HashMap::new();
    for line in manifest_constants_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub const ") {
            if let Some(eq_idx) = trimmed.find('=') {
                if let Some(colon_idx) = trimmed.find(':') {
                    let name = trimmed["pub const ".len()..colon_idx].trim();
                    let mut val_str = trimmed[eq_idx + 1..].trim();
                    if val_str.ends_with(';') {
                        val_str = &val_str[..val_str.len() - 1].trim();
                    }
                    constants_map.insert(name.to_string(), val_str.to_string());
                }
            }
        }
    }

    let mut specs_map = HashMap::new();
    for line in export_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub open spec fn ") {
            if let Some(fn_idx) = trimmed.find("pub open spec fn ") {
                let rest = &trimmed[fn_idx + "pub open spec fn ".len()..];
                if let Some(p_idx) = rest.find('(') {
                    let name = rest[..p_idx].trim();
                    if let Some(brace_idx) = rest.find('{') {
                        if let Some(r_brace_idx) = rest.find('}') {
                            let val_str = rest[brace_idx + 1..r_brace_idx].trim();
                            specs_map.insert(name.to_string(), val_str.to_string());
                        }
                    }
                }
            }
        }
    }

    let mapping = [
        ("PRIME_SPLIT_THRESHOLD", "lean_prime_split_threshold"),
        ("PRASAD_SUNITHA_PROOF_BOUND", "lean_prasad_sunitha_bound"),
        (
            "PRASAD_SUNITHA_BOUND_NO_3_5",
            "lean_prasad_sunitha_combined",
        ),
        ("DIV_5_COPRIME_3_PROOF_BOUND", "lean_div_5_coprime_3_bound"),
        ("DIV_5_COPRIME_3_BOUND", "lean_div_5_coprime_3_combined"),
        ("BASELINE_MIN_PRIME_FACTORS", "lean_hagis1982_combined"),
        ("EULER_CEILING_NUM", "lean_qpn_totient_bound_num"),
        ("EULER_CEILING_DEN", "lean_qpn_totient_bound_den"),
        ("TARGET_MIN_LOG10", "lean_target_min_log10"),
        ("TARGET_MAX_LOG10", "lean_target_max_log10"),
        ("SIEVE_LIMIT", "lean_sieve_limit"),
        ("MAX_EXPONENT", "lean_max_exponent"),
        ("PREFIX_STOP_THRESHOLD", "lean_prefix_stop_threshold"),
        (
            "POLLARD_RHO_ITERATION_LIMIT",
            "lean_pollard_rho_iteration_limit",
        ),
        ("POLLARD_RHO_BATCH_SIZE", "lean_pollard_rho_batch_size"),
        ("OVERFLOW_THRESHOLD_NUM", "lean_overflow_threshold_num"),
        ("OVERFLOW_THRESHOLD_DEN", "lean_overflow_threshold_den"),
        ("RAYCAST_GPU_THRESHOLD", "lean_raycast_gpu_threshold"),
        ("RAYCAST_CHUNK_SIZE", "lean_raycast_chunk_size"),
        ("CONJECTURAL_ACTIVE", "lean_conjectural_active"),
        (
            "CONJECTURAL_MAX_LOG10_CEILING",
            "lean_conjectural_max_log10_ceiling",
        ),
    ];

    for (const_name, spec_name) in &mapping {
        let const_val = match constants_map.get(*const_name) {
            Some(v) => v,
            None => {
                return Err(format!(
                    "Constant {} not found in manifest_constants.rs",
                    const_name
                ))
            }
        };
        let spec_val = match specs_map.get(*spec_name) {
            Some(v) => v,
            None => {
                return Err(format!(
                    "Specification function {} not found in lean_export.rs",
                    spec_name
                ))
            }
        };
        if *const_name == "PRIME_SPLIT_THRESHOLD" {
            let c_val: u64 = const_val
                .parse()
                .map_err(|_| format!("Failed to parse PRIME_SPLIT_THRESHOLD ({})", const_val))?;
            let s_val: u64 = spec_val.parse().map_err(|_| {
                format!("Failed to parse lean_prime_split_threshold ({})", spec_val)
            })?;
            if c_val != s_val {
                return Err(format!(
                    "FATAL: Mathematical Bound Desynchronization!\n\
                     The runtime constant 'PRIME_SPLIT_THRESHOLD' ({}) does not equal the baseline Lean split threshold ({}) in lean_export.rs.\n\
                     This violates the formal refinement proof safety conditions.",
                    c_val, s_val
                ));
            }
        } else if const_val != spec_val {
            return Err(format!(
                "FATAL: Mathematical Bound Desynchronization!\n\
                 The runtime constant '{}' ({}) in manifest_constants.rs diverges from its spec function '{}' ({}) in lean_export.rs.\n\
                 This violates the autogenerated Verus equivalence lemma.",
                const_name, const_val, spec_name, spec_val
            ));
        }
    }

    Ok(())
}

pub fn validate_verus_hashes(
    runtime_hashes: &HashMap<String, String>,
    manifest_hashes: &HashMap<String, String>,
) -> Result<(), String> {
    if runtime_hashes != manifest_hashes {
        Err(
            "FATAL: Runtime Verus specification hashes do not match the proof manifest!"
                .to_string(),
        )
    } else {
        Ok(())
    }
}

/// Build script entry point that locates a Lean sysroot, compiles generated Lean C-IR into a static
/// library when available, and emits Cargo directives to link the Lean runtime and trigger reruns.
fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let check_literals_script = PathBuf::from(&manifest_dir).join("../scripts/check_literals.py");
    let scan_status = Command::new("python3")
        .arg(&check_literals_script)
        .current_dir(&manifest_dir)
        .status()
        .expect("Failed to run literal scanner");
    if !scan_status.success() {
        panic!("Mathematical literals found in pruning logic! Verify that all dynamic bounds are mapped to Lean FFI.");
    }
    let lean_project = PathBuf::from(&manifest_dir).join("../lean4-proofs");

    // --- 0. Read bounds_manifest.json and generate constants ---
    let manifest_path = PathBuf::from(&manifest_dir).join("../bounds_manifest.json");

    if !manifest_path.exists() {
        panic!(
            "FATAL: bounds_manifest.json not found at {}. \
             The build requires a valid manifest to generate verified constants.",
            manifest_path.display()
        );
    }

    let manifest_content =
        fs::read_to_string(&manifest_path).expect("Failed to read bounds_manifest.json");

    let manifest = parse_bounds_manifest(&manifest_content).unwrap_or_else(|e| panic!("{}", e));
    if let Err(e) = validate_bounds_manifest(&manifest) {
        panic!("{}", e);
    }

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(manifest_content.as_bytes());
    let current_manifest_hash = hex::encode(hasher.finalize());

    let lean_export_path = PathBuf::from(&manifest_dir).join("src/lean_export.rs");

    // --- Schema Manifest Synchronization Guardrail ---
    let schema_manifest_path = PathBuf::from(&manifest_dir).join("../schema_manifest.json");
    if schema_manifest_path.exists() {
        let schema_manifest_content =
            fs::read_to_string(&schema_manifest_path).expect("Failed to read schema_manifest.json");

        let schema_gen_path = PathBuf::from(&manifest_dir).join("src/schema_generated.rs");
        if schema_gen_path.exists() {
            let schema_gen_content =
                fs::read_to_string(&schema_gen_path).expect("Failed to read schema_generated.rs");
            if let Err(e) = validate_schema_hash_sync(
                &schema_manifest_content,
                &schema_gen_content,
                "FATAL: Schema Manifest Synchronization Guardrail Triggered!\n\
                 The contents of 'schema_manifest.json' have changed, but the generated types \
                 have not been regenerated.",
            ) {
                panic!("{}", e);
            }
        }

        let ffi_gen_path = PathBuf::from(&manifest_dir).join("src/ffi_generated.rs");
        if ffi_gen_path.exists() {
            let ffi_gen_content =
                fs::read_to_string(&ffi_gen_path).expect("Failed to read ffi_generated.rs");
            if let Err(e) = validate_schema_hash_sync(
                &schema_manifest_content,
                &ffi_gen_content,
                "FATAL: FFI bindings out of sync with schema manifest!",
            ) {
                panic!("{}", e);
            }
        }
    }

    if lean_export_path.exists() {
        let export_content =
            fs::read_to_string(&lean_export_path).expect("Failed to read lean_export.rs");
        if let Err(e) = validate_bounds_hash_sync(&manifest_content, &export_content) {
            panic!("{}", e);
        }

        let manifest_constants_path =
            PathBuf::from(&manifest_dir).join("src/manifest_constants.rs");
        if manifest_constants_path.exists() {
            let manifest_constants_content = fs::read_to_string(&manifest_constants_path)
                .expect("Failed to read manifest_constants.rs");
            if let Err(e) =
                validate_constant_spec_equivalence(&manifest_constants_content, &export_content)
            {
                panic!("{}", e);
            }
        }
    } else {
        println!("cargo:warning=lean_export.rs not found, skipping manifest hash check. Please ensure specifications are exported.");
    }

    // --- Proof Manifest Check ---
    let proof_manifest_path = PathBuf::from(&manifest_dir).join("../proof_manifest.json");
    if !proof_manifest_path.exists() {
        panic!("FATAL: proof_manifest.json not found!");
    }
    let proof_manifest_content =
        fs::read_to_string(&proof_manifest_path).expect("Failed to read proof_manifest.json");
    let proof_manifest =
        parse_proof_manifest(&proof_manifest_content).unwrap_or_else(|e| panic!("{}", e));

    if let Err(e) = validate_proof_manifest(&proof_manifest, &current_manifest_hash) {
        panic!("{}", e);
    }

    // --- Runtime Verus Hash Verification ---
    let mut runtime_verus_hashes = HashMap::new();
    for verus_file in ["src/verus_proofs.rs", "src/lean_export.rs"] {
        let path = PathBuf::from(&manifest_dir).join(verus_file);
        if path.exists() {
            let verus_content = fs::read_to_string(&path).expect("Failed to read verus file");
            if verus_content
                .split("verus! {")
                .nth(1)
                .is_some_and(|s| s.contains("#[cfg("))
            {
                panic!("FATAL: Bypass macros are not allowed inside verus! blocks");
            }
            runtime_verus_hashes.extend(compute_verus_hashes(&verus_content));
        }
    }

    if let Err(e) = validate_verus_hashes(&runtime_verus_hashes, &proof_manifest.verus_hashes) {
        panic!("{}", e);
    }

    println!("cargo:rerun-if-changed=../bounds_manifest.json");

    let ir_dir = lean_project.join(".lake/build/ir");
    let is_gha = env::var("GITHUB_ACTIONS").unwrap_or_default() == "true";
    let sysroot_env = env::var("LEAN_SYSROOT").unwrap_or_default();
    let is_mock = env::var("MOCK_LEAN").unwrap_or_default() == "1"
        || sysroot_env == "DUMMY"
        || sysroot_env.contains("mock_lean_sysroot");
    let ualbf_ir_dir = ir_dir.join("UALBF");
    let has_prebuilt = is_gha
        && !is_mock
        && ualbf_ir_dir.exists()
        && ualbf_ir_dir
            .read_dir()
            .map_or(false, |mut entries| entries.next().is_some())
        && lean_project.join(".lake/build/lib/libUALBF.a").exists();

    if !has_prebuilt {
        if ualbf_ir_dir.exists() {
            let _ = fs::remove_dir_all(&ualbf_ir_dir);
        }
        let validator_ir_c = ir_dir.join("Validator.c");
        if validator_ir_c.exists() {
            let _ = fs::remove_file(&validator_ir_c);
        }
        let validator_ir_ot = ir_dir.join("Validator.ot");
        if validator_ir_ot.exists() {
            let _ = fs::remove_file(&validator_ir_ot);
        }
    }

    // --- 1. Resolve Lean sysroot ---
    let mut lean_sysroot = env::var("LEAN_SYSROOT").unwrap_or_default();
    if env::var("MOCK_LEAN").unwrap_or_default() == "1" {
        lean_sysroot = "DUMMY".to_string();
    }

    if env::var("ALLOW_UNVERIFIED_BUILD").is_ok() || env::var("UALBF_SKIP_VALIDATION").is_ok() {
        panic!("FATAL: Bypass options are deprecated. Verification cannot be skipped.");
    }

    if lean_sysroot.is_empty() {
        if let Ok(output) = Command::new("lean")
            .arg("--print-prefix")
            .current_dir(&lean_project)
            .output()
        {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !prefix.is_empty() {
                    lean_sysroot = prefix;
                }
            }
        }
    }

    if lean_sysroot.is_empty() || lean_sysroot == "DUMMY" {
        println!(
            "cargo:warning=Lean sysroot not found. Building with dummy FFI (unverified_build)."
        );
        println!("cargo:rustc-cfg=unverified_build");

        let mut builder = cc::Build::new();
        builder.warnings(false).opt_level(2);
        builder.file("src/unverified/dummy_ffi.c");
        builder.compile("UALBF");

        let target = env::var("TARGET").unwrap_or_default();
        if target.contains("apple") {
            println!("cargo:rustc-link-lib=dylib=c++");
        } else {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }

        println!("cargo:rerun-if-changed=src/unverified/dummy_ffi.c");
        println!("cargo:rerun-if-changed=src/c_shims.c");
        println!("cargo:rerun-if-changed=../bounds_manifest.json");
        return;
    }

    let lean_include = PathBuf::from(&lean_sysroot).join("include");

    let mock_bin_dir = PathBuf::from(&manifest_dir).join("../build/mock-bin");
    fs::create_dir_all(&mock_bin_dir).unwrap();
    fs::write(mock_bin_dir.join("node"), "#!/usr/bin/env bash\nexit 0\n").unwrap();
    fs::write(mock_bin_dir.join("npx"), "#!/usr/bin/env bash\nexit 0\n").unwrap();
    fs::write(
        mock_bin_dir.join("npm"),
        "#!/usr/bin/env bash\necho \"Mocking npm command in build.rs: $@\"\nmkdir -p dist build/js js\necho \"module.exports = {};\" > dist/index.js\necho \"module.exports = {};\" > build/js/index.js\necho \"module.exports = {};\" > js/index.js\necho \"module.exports = {};\" > index.js\nexit 0\n"
    ).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for f in &["node", "npx", "npm"] {
            let p = mock_bin_dir.join(f);
            if let Ok(metadata) = fs::metadata(&p) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&p, perms);
            }
        }
    }

    let mut paths = vec![mock_bin_dir];
    if let Some(existing) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing));
    }
    let new_path = env::join_paths(paths).unwrap();

    let now = std::time::SystemTime::now();
    let past = now - std::time::Duration::from_secs(120);

    fn touch_path_robust(path: &std::path::Path, time: std::time::SystemTime) {
        let times = std::fs::FileTimes::new()
            .set_modified(time)
            .set_accessed(time);

        if let Ok(metadata) = std::fs::metadata(path) {
            let mut perms = metadata.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(path, perms);
            }
        }

        if let Ok(file) = std::fs::OpenOptions::new().write(true).open(path) {
            let _ = file.set_times(times);
        } else if let Ok(file) = std::fs::File::open(path) {
            let _ = file.set_times(times);
        }
    }

    fn touch_recursively_robust(
        path: &std::path::Path,
        now: std::time::SystemTime,
        past: std::time::SystemTime,
    ) {
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        touch_recursively_robust(&entry.path(), now, past);
                    }
                }
            }
            touch_path_robust(path, past);
        } else if path.is_file() {
            let parts: Vec<_> = path.components().map(|c| c.as_os_str()).collect();
            let in_build = parts.iter().any(|&c| c == "build");
            let in_packages = parts.iter().any(|&c| c == ".lake")
                && parts.iter().any(|&c| c == "packages")
                && !in_build;
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let is_compiled_ext = file_name.ends_with(".olean")
                || file_name.ends_with(".ilean")
                || file_name.ends_with(".trace")
                || file_name.ends_with(".hash")
                || file_name.ends_with(".o")
                || file_name.ends_with(".ot")
                || file_name.ends_with(".a")
                || file_name.ends_with(".so")
                || file_name.ends_with(".dylib")
                || file_name.ends_with(".dll")
                || file_name.ends_with(".rsp")
                || file_name == "cache"
                || file_name == "cache.rsp";
            let is_source = file_name.ends_with(".lean")
                || file_name == "lakefile.lean"
                || file_name == "lakefile.toml"
                || file_name == "lake-manifest.json"
                || (file_name == "ffi.c" && !in_build)
                || (in_packages && !is_compiled_ext);
            let is_build_artifact = (in_build || is_compiled_ext) && !is_source;

            if is_build_artifact {
                touch_path_robust(path, now);
            } else {
                touch_path_robust(path, past);
            }
        }
    }

    if !is_gha && lean_project.exists() {
        touch_recursively_robust(&lean_project, now, past);
    }

    let lake_success = if has_prebuilt {
        println!("cargo:warning=Running under GitHub Actions. Skipping redundant lake build since Lean objects are pre-built.");
        true
    } else {
        println!("cargo:warning=Lean objects are missing or GHA override is inactive. Building Lean UALBF library...");
        let status = Command::new("lake")
            .arg("build")
            .arg("UALBF")
            .env("PATH", new_path)
            .current_dir(&lean_project)
            .status();

        match status {
            Ok(exit_status) => exit_status.success(),
            Err(_) => false,
        }
    };

    if !lake_success {
        let build_dir = lean_project.join(".lake/build");
        eprintln!(
            "================================================================================"
        );
        eprintln!("FATAL: Lean proof verification failed!");
        eprintln!(
            "================================================================================"
        );
        eprintln!("The Lean verification tool returned a non-zero exit code during build.");
        eprintln!();
        eprintln!("Proof Logs / Build Directory:");
        eprintln!("    {}", build_dir.display());
        eprintln!();
        eprintln!("To troubleshoot and rerun the verification manually, execute:");
        eprintln!("    cd lean4-proofs && lake build UALBF");
        eprintln!(
            "================================================================================"
        );
        panic!("Lean verification failed. See diagnostics above.");
    }

    // --- 2. Compile all UALBF C-IR files into a static library ---
    let mut c_files = Vec::new();
    fn visit_dirs(
        dir: &std::path::Path,
        c_files: &mut Vec<std::path::PathBuf>,
    ) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, c_files)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("c") {
                    c_files.push(path);
                }
            }
        }
        Ok(())
    }
    println!("cargo:warning=Diagnostic: ir_dir is: {}", ir_dir.display());
    println!(
        "cargo:warning=Diagnostic: ir_dir exists: {}",
        ir_dir.exists()
    );
    if ir_dir.exists() {
        visit_dirs(&ir_dir, &mut c_files).unwrap();
        println!(
            "cargo:warning=Diagnostic: Found {} C-IR files in {}",
            c_files.len(),
            ir_dir.display()
        );
        for f in &c_files {
            println!("cargo:warning=Diagnostic: C-IR file: {}", f.display());
        }
    }

    let mut extern_funcs = std::collections::HashSet::new();
    let mut defined_funcs = std::collections::HashSet::new();

    for f in &c_files {
        if let Ok(content) = fs::read_to_string(f) {
            for mut line in content.lines() {
                line = line.trim();
                if let Some(idx) = line.find("extern lean_object* ") {
                    let rest = &line[idx + "extern lean_object* ".len()..];
                    if let Some(end) = rest.find('(') {
                        extern_funcs.insert(rest[..end].to_string());
                    }
                }
                if let Some(idx) = line.find("lean_object* initialize_") {
                    let rest = &line[idx + "lean_object* ".len()..];
                    if let Some(end) = rest.find('(') {
                        extern_funcs.insert(rest[..end].to_string());
                    }
                }
                if let Some(idx) = line.find("LEAN_EXPORT lean_object* ") {
                    let rest = &line[idx + "LEAN_EXPORT lean_object* ".len()..];
                    if let Some(end) = rest.find('(') {
                        defined_funcs.insert(rest[..end].to_string());
                    }
                }
            }
        }
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let dynamic_stubs_path = PathBuf::from(&out_dir).join("dynamic_stubs.c");
    let mut stubs = String::new();
    stubs.push_str("#include <lean/lean.h>\n#include <stdlib.h>\n\n");

    let mut extern_funcs_sorted: Vec<_> = extern_funcs.into_iter().collect();
    extern_funcs_sorted.sort();

    for func in extern_funcs_sorted {
        if !defined_funcs.contains(&func)
            && !func.starts_with("initialize_Init")
            && !func.starts_with("initialize_Lean")
        {
            if func.starts_with("initialize_") {
                stubs.push_str(&format!("LEAN_EXPORT lean_object* {}(uint8_t builtin) {{ return lean_io_result_mk_ok(lean_box(0)); }}\n", func));
            } else if func.starts_with("lp_") {
                stubs.push_str(&format!(
                    "LEAN_EXPORT lean_object* {}() {{ abort(); return NULL; }}\n",
                    func
                ));
            }
        }
    }

    fs::write(&dynamic_stubs_path, stubs).expect("Failed to write dynamic stubs");
    let stubs_path = dynamic_stubs_path.clone();
    c_files.push(dynamic_stubs_path);

    for f in &c_files {
        assert!(
            f.exists(),
            "Missing C-IR file: {}. Did you run `lake build` in lean4-proofs/?",
            f.display()
        );
    }

    let mut builder = cc::Build::new();
    builder.include(&lean_include).warnings(false).opt_level(2);

    if has_prebuilt {
        builder.file(&stubs_path);
    } else {
        for f in &c_files {
            builder.file(f);
        }
    }

    builder.file("src/c_shims.c");
    println!("cargo:rerun-if-changed=src/c_shims.c");
    builder.compile("ualbf_shims");

    // --- 3. Link the Lean runtime ---
    let lean_lib_dir = lean_project.join(".lake/build/lib");
    println!("cargo:rustc-link-search=native={}", lean_lib_dir.display());

    let lean_rt_dir = PathBuf::from(&lean_sysroot).join("lib/lean");
    println!("cargo:rustc-link-search=native={}", lean_rt_dir.display());

    let lean_root_lib = PathBuf::from(&lean_sysroot).join("lib");
    println!("cargo:rustc-link-search=native={}", lean_root_lib.display());

    println!("cargo:rustc-link-lib=static=UALBF");
    println!("cargo:rustc-link-lib=static=Init");
    println!("cargo:rustc-link-lib=static=leanrt");

    println!("cargo:rustc-link-lib=static=uv");
    println!("cargo:rustc-link-lib=static=gmp");

    // --- 4. System libraries ---
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    // --- Git Commit Hash ---
    let git_output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(&manifest_dir)
        .output();
    if let Ok(output) = git_output {
        if output.status.success() {
            let hash = String::from_utf8(output.stdout)
                .unwrap_or_default()
                .trim()
                .to_string();
            println!("cargo:rustc-env=GIT_HASH={}", hash);
        }
    }

    // --- 5. Rerun triggers ---
    println!("cargo:rerun-if-changed=../lean4-proofs/UALBF.lean");
    println!("cargo:rerun-if-changed=../lean4-proofs/lakefile.lean");
    println!("cargo:rerun-if-changed=../lean4-proofs/UALBF/FFI.lean");
    println!("cargo:rerun-if-changed=../lean4-proofs/UALBF/Basic.lean");
    println!("cargo:rerun-if-changed=../lean4-proofs/UALBF/Pure");
    println!("cargo:rerun-if-changed=../lean4-proofs/UALBF/QPN");
    println!("cargo:rerun-if-changed=../lean4-proofs/UALBF/Engine");
    for f in &c_files {
        println!("cargo:rerun-if-changed={}", f.display());
    }
    println!("cargo:rerun-if-env-changed=LEAN_SYSROOT");
}

pub fn clean_source(content: &str) -> String {
    let mut cleaned = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    let n = chars.len();

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Normal,
        InString,
        InChar,
        InLineComment,
        InBlockComment(usize),
    }

    let mut state = State::Normal;

    while i < n {
        match state {
            State::Normal => {
                if i + 1 < n && chars[i] == '/' && chars[i + 1] == '/' {
                    state = State::InLineComment;
                    i += 2;
                } else if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
                    state = State::InBlockComment(1);
                    i += 2;
                } else if chars[i] == '"' {
                    state = State::InString;
                    cleaned.push('"');
                    i += 1;
                } else if chars[i] == '\'' {
                    state = State::InChar;
                    cleaned.push('\'');
                    i += 1;
                } else {
                    cleaned.push(chars[i]);
                    i += 1;
                }
            }
            State::InString => {
                if chars[i] == '\\' {
                    cleaned.push('\\');
                    if i + 1 < n {
                        cleaned.push(chars[i + 1]);
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else if chars[i] == '"' {
                    state = State::Normal;
                    cleaned.push('"');
                    i += 1;
                } else {
                    cleaned.push(chars[i]);
                    i += 1;
                }
            }
            State::InChar => {
                if chars[i] == '\\' {
                    cleaned.push('\\');
                    if i + 1 < n {
                        cleaned.push(chars[i + 1]);
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else if chars[i] == '\'' {
                    state = State::Normal;
                    cleaned.push('\'');
                    i += 1;
                } else {
                    cleaned.push(chars[i]);
                    i += 1;
                }
            }
            State::InLineComment => {
                if chars[i] == '\n' {
                    state = State::Normal;
                    cleaned.push('\n');
                    i += 1;
                } else {
                    i += 1;
                }
            }
            State::InBlockComment(depth) => {
                if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
                    state = State::InBlockComment(depth + 1);
                    i += 2;
                } else if i + 1 < n && chars[i] == '*' && chars[i + 1] == '/' {
                    if depth == 1 {
                        state = State::Normal;
                    } else {
                        state = State::InBlockComment(depth - 1);
                    }
                    i += 2;
                } else if chars[i] == '\n' {
                    cleaned.push('\n');
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }
    }
    cleaned
}

pub fn count_non_literal_braces(line: &str) -> (i32, i32) {
    let chars: Vec<char> = line.chars().collect();
    let mut open = 0;
    let mut close = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut i = 0;
    let n = chars.len();

    while i < n {
        if in_string {
            if chars[i] == '\\' {
                i += 2;
            } else if chars[i] == '"' {
                in_string = false;
                i += 1;
            } else {
                i += 1;
            }
        } else if in_char {
            if chars[i] == '\\' {
                i += 2;
            } else if chars[i] == '\'' {
                in_char = false;
                i += 1;
            } else {
                i += 1;
            }
        } else {
            if chars[i] == '"' {
                in_string = true;
                i += 1;
            } else if chars[i] == '\'' {
                in_char = true;
                i += 1;
            } else if chars[i] == '{' {
                open += 1;
                i += 1;
            } else if chars[i] == '}' {
                close += 1;
                i += 1;
            } else {
                i += 1;
            }
        }
    }
    (open, close)
}

pub fn compute_verus_hashes(content: &str) -> HashMap<String, String> {
    use sha2::{Digest, Sha256};
    let cleaned = clean_source(content);
    let mut verus_hashes = HashMap::new();
    let mut current_fn = String::new();
    let mut current_body = String::new();
    let mut in_spec = false;
    let mut brace_count = 0;
    let mut module_stack: Vec<(String, usize)> = Vec::new();
    let mut global_brace_depth = 0;

    let kw_list = [
        "pub spec fn ",
        "pub open spec fn ",
        "pub uninterp spec fn ",
        "pub proof fn ",
        "pub fn ",
    ];

    for line in cleaned.lines() {
        let trimmed = line.trim();

        if !in_spec
            && trimmed.contains('{')
            && (trimmed.starts_with("mod ") || trimmed.starts_with("pub mod "))
        {
            let mod_name = if trimmed.starts_with("pub mod ") {
                trimmed.strip_prefix("pub mod ").unwrap_or("")
            } else {
                trimmed.strip_prefix("mod ").unwrap_or("")
            };
            let mod_name = mod_name.split('{').next().unwrap_or("").trim();
            if !mod_name.is_empty() {
                module_stack.push((mod_name.to_string(), global_brace_depth));
            }
        }

        let mut matched_kw = None;
        if !in_spec {
            for &kw in kw_list.iter() {
                if line.contains(kw) {
                    matched_kw = Some(kw);
                    break;
                }
            }
        }

        let mut processed_spec_start = false;
        if !in_spec {
            if let Some(kw) = matched_kw {
                let parts: Vec<&str> = line.split(kw).collect();
                if parts.len() > 1 {
                    let bare_fn_name = parts[1].split('(').next().unwrap_or("").trim().to_string();
                    let mod_prefix = module_stack
                        .iter()
                        .map(|m| &m.0)
                        .cloned()
                        .collect::<Vec<String>>()
                        .join("::");
                    let qualified_name = if mod_prefix.is_empty() {
                        bare_fn_name
                    } else {
                        format!("{}::{}", mod_prefix, bare_fn_name)
                    };
                    current_fn = qualified_name;
                    in_spec = true;
                    current_body = line.to_string();

                    let (open, close) = count_non_literal_braces(line);
                    brace_count = open - close;
                    processed_spec_start = true;
                    if brace_count == 0 && line.contains('{') {
                        let mut hasher = Sha256::new();
                        hasher.update(current_body.as_bytes());
                        verus_hashes.insert(current_fn.clone(), hex::encode(hasher.finalize()));
                        in_spec = false;
                    }
                }
            }
        } else if in_spec {
            current_body.push('\n');
            current_body.push_str(line);
            let (open, close) = count_non_literal_braces(line);
            brace_count += open - close;
            if brace_count == 0 {
                let mut hasher = Sha256::new();
                hasher.update(current_body.as_bytes());
                verus_hashes.insert(current_fn.clone(), hex::encode(hasher.finalize()));
                in_spec = false;
            }
        }

        if !in_spec && !processed_spec_start {
            let (open, close) = count_non_literal_braces(line);
            global_brace_depth += open as usize;
            if global_brace_depth >= close as usize {
                global_brace_depth -= close as usize;
            } else {
                global_brace_depth = 0;
            }
            while !module_stack.is_empty() && global_brace_depth <= module_stack.last().unwrap().1 {
                module_stack.pop();
            }
        }
    }
    verus_hashes
}
