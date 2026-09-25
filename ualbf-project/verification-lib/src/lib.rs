pub mod ffi_boundary;

#[cfg(feature = "signing")]
pub use ed25519_dalek;
#[cfg(feature = "signing")]
pub use hex;
#[cfg(feature = "signing")]
pub use sha2;

pub const CORE_TCB_FILES: &[&str] = &[
    "dfs_tree.rs",
    "pruning_dispatch.rs",
    "sieve.rs",
    "verus_proofs.rs",
    "manifest_constants.rs",
    "lean_ffi.rs",
    "unverified/dummy_ffi.c",
    "../../proof_manifest.json",
    "../build.rs",
    "../../bounds_manifest.json",
];

pub const EXTENSION_TCB_FILES: &[&str] = &[];

pub fn normalize_proof_manifest(content: &str) -> String {
    let zero_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.contains("\"verified_logic_hash\":")
            || line.contains("\"verified_extension_hash\":")
        {
            if let Some(first_colon) = line.find(':') {
                if let Some(q1) = line[first_colon..].find('"') {
                    let start = first_colon + q1 + 1;
                    if let Some(q2) = line[start..].find('"') {
                        let end = start + q2;
                        let mut new_line = String::new();
                        new_line.push_str(&line[..start]);
                        new_line.push_str(zero_hash);
                        new_line.push_str(&line[end..]);
                        lines.push(new_line);
                        continue;
                    }
                }
            }
        }
        lines.push(line.to_string());
    }
    let mut result = lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[macro_export]
#[cfg(feature = "signing")]
macro_rules! compute_core_tcb_hash_at_compile_time {
    () => {{
        use $crate::sha2::{Digest, Sha256};
        let mut logic_hasher = Sha256::new();
        logic_hasher.update(include_bytes!("dfs_tree.rs"));
        logic_hasher.update(include_bytes!("pruning_dispatch.rs"));
        logic_hasher.update(include_bytes!("sieve.rs"));
        logic_hasher.update(include_bytes!("verus_proofs.rs"));
        logic_hasher.update(include_bytes!("manifest_constants.rs"));
        logic_hasher.update(include_bytes!("lean_ffi.rs"));
        logic_hasher.update(include_bytes!("unverified/dummy_ffi.c"));

        let manifest_bytes = include_bytes!("../../proof_manifest.json");
        let manifest_str = String::from_utf8_lossy(manifest_bytes);
        let normalized = $crate::normalize_proof_manifest(&manifest_str);
        logic_hasher.update(normalized.as_bytes());

        logic_hasher.update(include_bytes!("../build.rs"));
        logic_hasher.update(include_bytes!("../../bounds_manifest.json"));
        $crate::hex::encode(logic_hasher.finalize())
    }};
}

#[macro_export]
#[cfg(not(feature = "signing"))]
macro_rules! compute_core_tcb_hash_at_compile_time {
    () => {
        "unverified_logic_hash".to_string()
    };
}

#[macro_export]
macro_rules! compute_extension_tcb_hash_at_compile_time {
    () => {
        "unverified_extension_hash".to_string()
    };
}

#[cfg(feature = "signing")]
pub fn compute_verified_core_hash_runtime(repo_root: &std::path::Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut logic_hasher = Sha256::new();
    let base_dir = repo_root.join("rust-engine/src");

    for file in CORE_TCB_FILES {
        let path = base_dir.join(file);
        let path = path.canonicalize().unwrap_or(path);
        let content = std::fs::read(&path)?;
        if path.file_name() == Some(std::ffi::OsStr::new("proof_manifest.json")) {
            let manifest_str = String::from_utf8_lossy(&content);
            let normalized = normalize_proof_manifest(&manifest_str);
            logic_hasher.update(normalized.as_bytes());
        } else {
            logic_hasher.update(&content);
        }
    }
    Ok(hex::encode(logic_hasher.finalize()))
}

#[cfg(feature = "signing")]
pub fn compute_verified_extension_hash_runtime(
    repo_root: &std::path::Path,
) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut logic_hasher = Sha256::new();
    let base_dir = repo_root.join("rust-engine/src");

    for file in EXTENSION_TCB_FILES {
        let path = base_dir.join(file);
        let path = path.canonicalize().unwrap_or(path);
        let content = std::fs::read(&path)?;
        logic_hasher.update(&content);
    }
    Ok(hex::encode(logic_hasher.finalize()))
}

#[allow(clippy::too_many_arguments)]
pub fn format_payload(
    manifest_hash: &str,
    verified_logic_hash: &str,
    verified_extension_hash: Option<&str>,
    total_branches_searched: usize,
    target_min_log10: u32,
    target_max_log10: u32,
    trace_hash: &str,
    factorization_depth: u32,
    sampling_rate: Option<f64>,
    deterministic_seed: Option<u64>,
    is_conditional: Option<bool>,
    conjecture_name: Option<&str>,
    path_ranges: Option<serde_json::Value>,
    verification_mode: Option<&str>,
    sidecar_hash: Option<&str>,
) -> String {
    let mut map = std::collections::BTreeMap::new();
    map.insert(
        "manifest_hash",
        serde_json::Value::String(manifest_hash.to_string()),
    );
    map.insert(
        "verified_logic_hash",
        serde_json::Value::String(verified_logic_hash.to_string()),
    );
    if let Some(ext_hash) = verified_extension_hash {
        map.insert(
            "verified_extension_hash",
            serde_json::Value::String(ext_hash.to_string()),
        );
    }
    map.insert(
        "total_branches_searched",
        serde_json::Value::Number(serde_json::Number::from(total_branches_searched)),
    );
    map.insert(
        "target_min_log10",
        serde_json::Value::Number(serde_json::Number::from(target_min_log10)),
    );
    map.insert(
        "target_max_log10",
        serde_json::Value::Number(serde_json::Number::from(target_max_log10)),
    );
    map.insert(
        "trace_hash",
        serde_json::Value::String(trace_hash.to_string()),
    );
    if let Some(sh) = sidecar_hash {
        map.insert("sidecar_hash", serde_json::Value::String(sh.to_string()));
    }
    map.insert(
        "factorization_depth",
        serde_json::Value::Number(serde_json::Number::from(factorization_depth)),
    );

    if let Some(rate) = sampling_rate {
        map.insert(
            "sampling_rate",
            serde_json::Value::Number(serde_json::Number::from_f64(rate).unwrap()),
        );
    }
    if let Some(seed) = deterministic_seed {
        map.insert(
            "deterministic_seed",
            serde_json::Value::Number(serde_json::Number::from(seed)),
        );
    }
    if let Some(cond) = is_conditional {
        map.insert("is_conditional", serde_json::Value::Bool(cond));
    }
    if let Some(conj) = conjecture_name {
        map.insert(
            "conjecture_name",
            serde_json::Value::String(conj.to_string()),
        );
    }
    if let Some(ranges) = path_ranges {
        map.insert("path_ranges", ranges);
    }
    if let Some(mode) = verification_mode {
        map.insert(
            "verification_mode",
            serde_json::Value::String(mode.to_string()),
        );
    }

    serde_json::to_string(&map).unwrap()
}

#[cfg(feature = "signing")]
pub fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    payload: &str,
) -> Result<bool, String> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let pub_bytes = hex::decode(public_key_hex).map_err(|e| e.to_string())?;
    let sig_bytes = hex::decode(signature_hex).map_err(|e| e.to_string())?;

    let public_key = VerifyingKey::from_bytes(
        pub_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid key length")?,
    )
    .map_err(|e| e.to_string())?;
    let signature = Signature::from_bytes(
        sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid sig length")?,
    );

    Ok(public_key.verify(payload.as_bytes(), &signature).is_ok())
}

#[allow(dead_code)]
#[allow(clippy::manual_range_contains)]
fn validate_telemetry_numbers(val: &serde_json::Value) -> Result<(), String> {
    match val {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 {
                    let fits_i64 = (i64::MIN as f64..=i64::MAX as f64).contains(&f);
                    let fits_u64 = (0.0..18446744073709551616.0).contains(&f);
                    if !fits_i64 && !fits_u64 {
                        return Err(format!(
                            "Telemetry number {} exceeds 64-bit integer limits",
                            n
                        ));
                    }
                }
            } else {
                if n.as_i64().is_none() && n.as_u64().is_none() {
                    return Err(format!(
                        "Telemetry number {} exceeds 64-bit integer limits",
                        n
                    ));
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                validate_telemetry_numbers(v)?;
            }
        }
        serde_json::Value::Object(obj) => {
            for (_, v) in obj {
                validate_telemetry_numbers(v)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "python")]
fn serde_to_py<'py>(py: Python<'py>, value: &serde_json::Value) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::IntoPyObject;
    match value {
        serde_json::Value::Null => Ok(py.None().into_bound(py)),
        serde_json::Value::Bool(b) => {
            let py_val = (*b)
                .into_pyobject(py)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(py_val.as_any().clone())
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let py_val = i
                    .into_pyobject(py)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(py_val.as_any().clone())
            } else if let Some(u) = n.as_u64() {
                let py_val = u
                    .into_pyobject(py)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(py_val.as_any().clone())
            } else if let Some(f) = n.as_f64() {
                let py_val = f
                    .into_pyobject(py)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(py_val.as_any().clone())
            } else {
                Err(pyo3::exceptions::PyValueError::new_err(
                    "Invalid number value",
                ))
            }
        }
        serde_json::Value::String(s) => {
            let py_val = s
                .as_str()
                .into_pyobject(py)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(py_val.as_any().clone())
        }
        serde_json::Value::Array(arr) => {
            let list = PyList::new(py, Vec::<Bound<'py, PyAny>>::new())?;
            for val in arr {
                list.append(serde_to_py(py, val)?)?;
            }
            Ok(list.as_any().clone())
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, serde_to_py(py, v)?)?;
            }
            Ok(dict.as_any().clone())
        }
    }
}

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyList};

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (cert_json_str, trusted_public_key=None))]
pub fn validate_certificate<'py>(
    py: Python<'py>,
    cert_json_str: &str,
    trusted_public_key: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::exceptions::{PyException, PyValueError};

    // Null-byte check before parsing
    if cert_json_str.contains('\0') {
        return Err(PyValueError::new_err(
            "Null byte detected in certificate content",
        ));
    }

    // Resolve expected trusted public key from argument or environment variable
    let env_key = std::env::var("UALBF_TRUSTED_PUBLIC_KEY").ok();
    let trusted_key_str = match trusted_public_key.filter(|k| !k.trim().is_empty()) {
        Some(k) => k.trim(),
        None => match env_key.as_deref().filter(|k| !k.trim().is_empty()) {
            Some(k) => k.trim(),
            None => {
                return Err(PyValueError::new_err(
                    "ERROR: No trusted public key is pinned (UALBF_TRUSTED_PUBLIC_KEY not set).",
                ));
            }
        },
    };

    // Parse the JSON string
    let cert: serde_json::Value = serde_json::from_str(cert_json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse certificate JSON: {}", e)))?;

    let obj = cert
        .as_object()
        .ok_or_else(|| PyValueError::new_err("Certificate is not a JSON object"))?;

    let telemetry_val = obj
        .get("telemetry")
        .ok_or_else(|| PyValueError::new_err("Missing 'telemetry' object"))?;

    // Perform strict telemetry number validation
    validate_telemetry_numbers(telemetry_val)
        .map_err(|e| PyValueError::new_err(format!("Telemetry validation failed: {}", e)))?;

    let telemetry = telemetry_val
        .as_object()
        .ok_or_else(|| PyValueError::new_err("Invalid 'telemetry' object"))?;

    // Extract signed fields
    let manifest_hash = obj
        .get("manifest_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let verified_logic_hash = obj
        .get("verified_logic_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let verified_extension_hash = obj.get("verified_extension_hash").and_then(|v| v.as_str());
    let public_key = obj.get("public_key").and_then(|v| v.as_str()).unwrap_or("");
    let signature = obj.get("signature").and_then(|v| v.as_str()).unwrap_or("");

    // Check mandatory fields to prevent empty strings being valid
    if manifest_hash.is_empty() {
        return Err(PyValueError::new_err("Missing manifest_hash"));
    }
    if public_key.is_empty() {
        return Err(PyValueError::new_err("Missing public_key"));
    }
    if signature.is_empty() {
        return Err(PyValueError::new_err("Missing signature"));
    }

    // Ensure embedded certificate public key matches trusted public key
    if public_key != trusted_key_str {
        return Err(PyValueError::new_err(format!(
            "ERROR: Certificate public key does not match trusted signer key!\nCertificate key: {}\nTrusted key: {}",
            public_key, trusted_key_str
        )));
    }

    let actual_manifest_hash = get_manifest_hash_at_runtime().map_err(|e| {
        PyException::new_err(format!("Failed to retrieve runtime manifest hash: {}", e))
    })?;
    if manifest_hash != actual_manifest_hash {
        return Err(PyException::new_err(
            "Manifest hash mismatch in core verification engine!",
        ));
    }

    let total_branches_searched = telemetry
        .get("total_branches_searched")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let target_min_log10 = telemetry
        .get("target_min_log10")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let target_max_log10 = telemetry
        .get("target_max_log10")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let trace_hash = telemetry
        .get("trace_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let factorization_depth = telemetry
        .get("factorization_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let (sampling_rate, deterministic_seed) = if let Some(profile) = telemetry
        .get("verification_profile")
        .and_then(|v| v.as_object())
    {
        (
            profile.get("sampling_rate").and_then(|v| v.as_f64()),
            profile.get("deterministic_seed").and_then(|v| v.as_u64()),
        )
    } else {
        (None, None)
    };

    let is_conditional = obj.get("is_conditional").and_then(|v| v.as_bool());
    let conjecture_name = obj
        .get("conjecture")
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("conjecture_name"))
        .and_then(|v| v.as_str());

    let path_ranges = telemetry
        .get("path_ranges")
        .or_else(|| telemetry.get("inner_paths"))
        .or_else(|| telemetry.get("explored_ranges"))
        .cloned();

    let sidecar_hash = telemetry
        .get("sidecar_hash")
        .or_else(|| telemetry.get("sidecar_log_digest"))
        .or_else(|| obj.get("sidecar_hash"))
        .or_else(|| obj.get("sidecar_log_digest"))
        .and_then(|v| v.as_str());

    let verification_mode = obj.get("verification_mode").and_then(|v| v.as_str());

    // Reconstruct payload
    let payload = format_payload(
        manifest_hash,
        verified_logic_hash,
        verified_extension_hash,
        total_branches_searched,
        target_min_log10,
        target_max_log10,
        trace_hash,
        factorization_depth,
        sampling_rate,
        deterministic_seed,
        is_conditional,
        conjecture_name,
        path_ranges,
        verification_mode,
        sidecar_hash,
    );

    // Verify signature strictly against the verified trusted key
    let is_valid = verify_signature(trusted_key_str, signature, &payload)
        .map_err(|e| PyException::new_err(format!("Signature verification error: {}", e)))?;

    if !is_valid {
        return Err(PyException::new_err("Invalid cryptographic signature"));
    }

    // Return PyObject/PyDict directly to Python
    serde_to_py(py, &cert)
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn hash_tcb(repo_root: &str) -> PyResult<String> {
    use pyo3::exceptions::PyException;
    let path = std::path::Path::new(repo_root);
    compute_verified_core_hash_runtime(path)
        .map_err(|e| PyException::new_err(format!("Failed to hash TCB: {}", e)))
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn hash_extension_tcb(repo_root: &str) -> PyResult<String> {
    use pyo3::exceptions::PyException;
    let path = std::path::Path::new(repo_root);
    compute_verified_extension_hash_runtime(path)
        .map_err(|e| PyException::new_err(format!("Failed to hash extension TCB: {}", e)))
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RangeWorkUnit {
    pub start_bound: Vec<u64>,
    pub end_bound: Vec<u64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ContinuityResult {
    pub is_continuous: bool,
    pub gaps: Vec<RangeWorkUnit>,
}

pub fn compute_path_continuity(path_ranges_json: &str) -> Result<String, String> {
    let mut ranges: Vec<RangeWorkUnit> = serde_json::from_str(path_ranges_json)
        .map_err(|e| format!("Failed to parse path ranges JSON: {}", e))?;

    // Sort ranges lexicographically by start_bound, then by end_bound
    ranges.sort_by(|a, b| match a.start_bound.cmp(&b.start_bound) {
        std::cmp::Ordering::Equal => a.end_bound.cmp(&b.end_bound),
        other => other,
    });

    let mut gaps: Vec<RangeWorkUnit> = Vec::new();
    let mut is_continuous = true;

    if ranges.is_empty() {
        is_continuous = false;
        // Entire space is missing
        gaps.push(RangeWorkUnit {
            start_bound: vec![],
            end_bound: vec![],
        });
    } else {
        // Check gap at the beginning
        if !ranges[0].start_bound.is_empty() {
            is_continuous = false;
            gaps.push(RangeWorkUnit {
                start_bound: vec![],
                end_bound: ranges[0].start_bound.clone(),
            });
        }

        // Check gaps between adjacent ranges
        for i in 0..ranges.len() - 1 {
            let current_end = &ranges[i].end_bound;
            let next_start = &ranges[i + 1].start_bound;
            if current_end != next_start {
                is_continuous = false;
                if current_end < next_start {
                    gaps.push(RangeWorkUnit {
                        start_bound: current_end.clone(),
                        end_bound: next_start.clone(),
                    });
                }
            }
        }

        // Check gap at the end
        if let Some(last_range) = ranges.last() {
            if !last_range.end_bound.is_empty() {
                is_continuous = false;
                gaps.push(RangeWorkUnit {
                    start_bound: last_range.end_bound.clone(),
                    end_bound: vec![],
                });
            }
        }
    }

    let result = ContinuityResult {
        is_continuous,
        gaps,
    };

    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize gaps JSON: {}", e))
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn check_path_continuity(path_ranges_json: &str) -> PyResult<String> {
    use pyo3::exceptions::PyValueError;
    compute_path_continuity(path_ranges_json).map_err(PyValueError::new_err)
}

/// Strips comments and docstrings, preserving string and character literals.
/// Replaces non-newline comment characters with space/nothing, but keeps newlines to preserve line numbering and structure.
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

fn count_non_literal_braces(line: &str) -> (i32, i32) {
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

pub fn compute_verus_hashes(content: &str) -> std::collections::HashMap<String, String> {
    use sha2::{Digest, Sha256};
    let cleaned = clean_source(content);
    let mut verus_hashes = std::collections::HashMap::new();
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

        // Track module declarations
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

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "compute_verus_hashes")]
pub fn compute_verus_hashes_py(
    content: &str,
) -> PyResult<std::collections::HashMap<String, String>> {
    Ok(compute_verus_hashes(content))
}

#[cfg(feature = "python")]
#[pymodule]
fn verification_lib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_certificate, m)?)?;
    m.add_function(wrap_pyfunction!(hash_tcb, m)?)?;
    m.add_function(wrap_pyfunction!(hash_extension_tcb, m)?)?;
    m.add_function(wrap_pyfunction!(check_path_continuity, m)?)?;
    m.add_function(wrap_pyfunction!(compute_verus_hashes_py, m)?)?;
    Ok(())
}

pub const MAX_PATH_LEN: usize = 4096;
pub const MAX_CERT_JSON_LEN: usize = 10 * 1024 * 1024;
pub const MAX_PUB_KEY_LEN: usize = 64 * 1024;

#[cfg(feature = "signing")]
#[no_mangle]
pub extern "C" fn verify_certificate(
    cert_json_ptr: *const std::ffi::c_char,
    pub_key_ptr: *const std::ffi::c_char,
    is_valid_out: *mut bool,
    out_manifest_hash_buf: *mut std::ffi::c_char,
    out_manifest_hash_len: usize,
) -> *mut std::ffi::c_void {
    use ffi_boundary::{FfiMutPtr, FfiPtr};

    safe_ffi_boundary!(std::ptr::null_mut(), {
        if let Some(mut valid_out) = FfiMutPtr::new(is_valid_out) {
            *valid_out = false;
        }

        let write_error = |err: &str| {
            if let Some(mut buf_ptr) = FfiMutPtr::new(out_manifest_hash_buf) {
                if out_manifest_hash_len > 0 {
                    let bytes = err.as_bytes();
                    let copy_len = std::cmp::min(bytes.len(), out_manifest_hash_len - 1);
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr(),
                            buf_ptr.as_mut() as *mut std::ffi::c_char as *mut u8,
                            copy_len,
                        );
                        *(buf_ptr.as_mut() as *mut std::ffi::c_char as *mut std::ffi::c_char)
                            .add(copy_len) = 0;
                    }
                }
            }
        };

        let cert_json_ffi = match FfiPtr::new(cert_json_ptr) {
            Some(p) => p,
            None => return std::ptr::null_mut(),
        };
        let pub_key_ffi = match FfiPtr::new(pub_key_ptr) {
            Some(p) => p,
            None => return std::ptr::null_mut(),
        };

        let cert_json_c = match cert_json_ffi.to_cstr_bounded(MAX_CERT_JSON_LEN) {
            Ok(c) => c,
            Err(_) => return std::ptr::null_mut(),
        };
        let pub_key_c = match pub_key_ffi.to_cstr_bounded(MAX_PUB_KEY_LEN) {
            Ok(c) => c,
            Err(_) => return std::ptr::null_mut(),
        };

        let cert_json_str = cert_json_c.to_string_lossy();
        let expected_pub_key = pub_key_c.to_string_lossy();

        let cert: serde_json::Value = match serde_json::from_str(&cert_json_str) {
            Ok(c) => c,
            Err(_) => {
                write_error("Failed to parse JSON");
                return std::ptr::null_mut();
            }
        };

        let obj = match cert.as_object() {
            Some(o) => o,
            None => {
                write_error("Certificate is not a JSON object");
                return std::ptr::null_mut();
            }
        };

        let telemetry_val = match obj.get("telemetry") {
            Some(t) => t,
            None => {
                write_error("Missing or invalid telemetry object");
                return std::ptr::null_mut();
            }
        };

        if let Err(e) = validate_telemetry_numbers(telemetry_val) {
            write_error(&format!("Telemetry validation failed: {}", e));
            return std::ptr::null_mut();
        }

        let telemetry = match telemetry_val.as_object() {
            Some(t) => t,
            None => {
                write_error("Telemetry is not a JSON object");
                return std::ptr::null_mut();
            }
        };

        let manifest_hash = obj
            .get("manifest_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let verified_logic_hash = obj
            .get("verified_logic_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let verified_extension_hash = obj.get("verified_extension_hash").and_then(|v| v.as_str());
        let public_key = obj.get("public_key").and_then(|v| v.as_str()).unwrap_or("");
        let signature = obj.get("signature").and_then(|v| v.as_str()).unwrap_or("");

        if public_key != expected_pub_key {
            write_error("Certificate public key does not match trusted signer key!");
            return std::ptr::null_mut();
        }

        match get_manifest_hash_at_runtime() {
            Ok(actual_manifest_hash) => {
                if manifest_hash != actual_manifest_hash {
                    write_error("Manifest hash mismatch in core verification engine!");
                    return std::ptr::null_mut();
                }
            }
            Err(e) => {
                write_error(&format!("Failed to retrieve runtime manifest hash: {}", e));
                return std::ptr::null_mut();
            }
        }

        let total_branches_searched = telemetry
            .get("total_branches_searched")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let target_min_log10 = telemetry
            .get("target_min_log10")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let target_max_log10 = telemetry
            .get("target_max_log10")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let trace_hash = telemetry
            .get("trace_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let factorization_depth = telemetry
            .get("factorization_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let (sampling_rate, deterministic_seed) = if let Some(profile) = telemetry
            .get("verification_profile")
            .and_then(|v| v.as_object())
        {
            (
                profile.get("sampling_rate").and_then(|v| v.as_f64()),
                profile.get("deterministic_seed").and_then(|v| v.as_u64()),
            )
        } else {
            (None, None)
        };

        let is_conditional = obj.get("is_conditional").and_then(|v| v.as_bool());
        let conjecture_name = obj
            .get("conjecture")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("conjecture_name"))
            .and_then(|v| v.as_str());

        let path_ranges = telemetry
            .get("path_ranges")
            .or_else(|| telemetry.get("inner_paths"))
            .or_else(|| telemetry.get("explored_ranges"))
            .cloned();

        let sidecar_hash = telemetry
            .get("sidecar_hash")
            .or_else(|| telemetry.get("sidecar_log_digest"))
            .or_else(|| obj.get("sidecar_hash"))
            .or_else(|| obj.get("sidecar_log_digest"))
            .and_then(|v| v.as_str());

        let verification_mode = obj.get("verification_mode").and_then(|v| v.as_str());

        let payload = format_payload(
            manifest_hash,
            verified_logic_hash,
            verified_extension_hash,
            total_branches_searched,
            target_min_log10,
            target_max_log10,
            trace_hash,
            factorization_depth,
            sampling_rate,
            deterministic_seed,
            is_conditional,
            conjecture_name,
            path_ranges,
            verification_mode,
            sidecar_hash,
        );

        let is_valid = verify_signature(public_key, signature, &payload).unwrap_or(false);

        if !is_valid || manifest_hash.is_empty() || signature.is_empty() {
            write_error("Invalid cryptographic signature!");
            return std::ptr::null_mut();
        }

        if let Some(mut valid_out) = FfiMutPtr::new(is_valid_out) {
            *valid_out = true;
        }
        write_error(manifest_hash);

        Box::into_raw(Box::new(cert)) as *mut std::ffi::c_void
    })
}

#[cfg(feature = "signing")]
#[no_mangle]
pub extern "C" fn free_certificate(cert_ptr: *mut std::ffi::c_void) {
    use ffi_boundary::FfiMutPtr;

    safe_ffi_boundary!((), {
        if let Some(mut ptr) = FfiMutPtr::new(cert_ptr) {
            unsafe {
                let _ =
                    Box::from_raw(ptr.as_mut() as *mut std::ffi::c_void as *mut serde_json::Value);
            }
        }
    });
}

#[cfg(feature = "signing")]
fn get_manifest_hash_at_runtime() -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let (paths_to_try, is_explicit) = match std::env::var("UALBF_PROOF_MANIFEST") {
        Ok(path) => (vec![std::path::PathBuf::from(path)], true),
        Err(_) => (
            vec![
                std::path::PathBuf::from("proof_manifest.json"),
                std::path::PathBuf::from("../proof_manifest.json"),
                std::path::PathBuf::from("../../proof_manifest.json"),
            ],
            false,
        ),
    };

    let mut content = None;
    for path in &paths_to_try {
        if path.exists() {
            if let Ok(bytes) = std::fs::read(path) {
                content = Some(bytes);
                break;
            }
        }
    }

    let bytes = content.ok_or_else(|| {
        if is_explicit {
            format!(
                "explicitly configured manifest not found: {:?}",
                paths_to_try[0]
            )
        } else {
            "proof_manifest.json not found".to_string()
        }
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(feature = "signing")]
#[no_mangle]
pub extern "C" fn rust_sha256_file(path_ptr: *const std::ffi::c_char) -> *mut std::ffi::c_char {
    use ffi_boundary::FfiPtr;
    use sha2::{Digest, Sha256};

    safe_ffi_boundary!(std::ptr::null_mut(), {
        let path_ffi = match FfiPtr::new(path_ptr) {
            Some(p) => p,
            None => return std::ptr::null_mut(),
        };
        let c_str = match path_ffi.to_cstr_bounded(MAX_PATH_LEN) {
            Ok(c) => c,
            Err(_) => return std::ptr::null_mut(),
        };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let bytes = match std::fs::read(path_str) {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash_str = hex::encode(hasher.finalize());
        let c_string = std::ffi::CString::new(hash_str).unwrap();
        c_string.into_raw()
    })
}

#[cfg(feature = "signing")]
#[no_mangle]
pub extern "C" fn rust_free_string(ptr: *mut std::ffi::c_char) {
    use ffi_boundary::FfiMutPtr;

    safe_ffi_boundary!((), {
        if let Some(mut p) = FfiMutPtr::new(ptr) {
            unsafe {
                let _ = std::ffi::CString::from_raw(p.as_mut() as *mut std::ffi::c_char);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_comments_simple() {
        let code = "fn main() {\n  // comment here\n  let x = 1; /* block comment */\n}";
        let cleaned = clean_source(code);
        assert!(!cleaned.contains("comment here"));
        assert!(!cleaned.contains("block comment"));
        assert!(cleaned.contains("fn main() {"));
        assert!(cleaned.contains("let x = 1;"));
    }

    #[test]
    fn test_clean_source_nested_comments_and_literals() {
        let code = r#"
            // Line comment with { braces }
            /* Outer comment
               /* Nested comment */
            */
            let s = "String with // not a comment and /* block */";
            let c = '/';
        "#;
        let cleaned = clean_source(code);
        assert!(!cleaned.contains("Line comment"));
        assert!(!cleaned.contains("Outer comment"));
        assert!(!cleaned.contains("Nested comment"));
        assert!(cleaned.contains(r#"let s = "String with // not a comment and /* block */";"#));
        assert!(cleaned.contains("let c = '/';"));
    }

    #[test]
    fn test_braces_in_comments_and_strings() {
        let code = r#"
            pub fn my_func() {
                // } unmatched commented brace
                let s = "{ braces in string }";
                let c = '}'; // char brace
            }
        "#;
        let hashes = compute_verus_hashes(code);
        assert!(hashes.contains_key("my_func"));
    }

    #[test]
    fn test_path_continuity_contiguous() {
        let input = r#"[
            {"start_bound": [], "end_bound": [10]},
            {"start_bound": [10], "end_bound": [20]},
            {"start_bound": [20], "end_bound": []}
        ]"#;
        let res_json = compute_path_continuity(input).expect("Should parse");
        let res: ContinuityResult = serde_json::from_str(&res_json).unwrap();
        assert!(res.is_continuous);
        assert!(res.gaps.is_empty());
    }

    #[test]
    fn test_path_continuity_out_of_order() {
        let input = r#"[
            {"start_bound": [10], "end_bound": [20]},
            {"start_bound": [], "end_bound": [10]},
            {"start_bound": [20], "end_bound": []}
        ]"#;
        let res_json = compute_path_continuity(input).expect("Should parse");
        let res: ContinuityResult = serde_json::from_str(&res_json).unwrap();
        assert!(res.is_continuous);
        assert!(res.gaps.is_empty());
    }

    #[test]
    fn test_path_continuity_overlapping() {
        let input = r#"[
            {"start_bound": [], "end_bound": [15]},
            {"start_bound": [10], "end_bound": []}
        ]"#;
        let res_json = compute_path_continuity(input).expect("Should parse");
        let res: ContinuityResult = serde_json::from_str(&res_json).unwrap();
        assert!(!res.is_continuous);
    }

    #[test]
    fn test_path_continuity_gap_recovery() {
        // Gap in middle
        let input_mid_gap = r#"[
            {"start_bound": [], "end_bound": [10]},
            {"start_bound": [15], "end_bound": []}
        ]"#;
        let res_json = compute_path_continuity(input_mid_gap).expect("Should parse");
        let res: ContinuityResult = serde_json::from_str(&res_json).unwrap();
        assert!(!res.is_continuous);
        assert_eq!(res.gaps.len(), 1);
        assert_eq!(res.gaps[0].start_bound, vec![10]);
        assert_eq!(res.gaps[0].end_bound, vec![15]);

        // Empty input
        let empty_json = compute_path_continuity("[]").expect("Should parse");
        let res_empty: ContinuityResult = serde_json::from_str(&empty_json).unwrap();
        assert!(!res_empty.is_continuous);
        assert_eq!(res_empty.gaps.len(), 1);
    }

    #[test]
    fn test_validate_telemetry_numbers_valid() {
        let v1 = serde_json::json!(100);
        let v2 = serde_json::json!(-500);
        let v3 = serde_json::json!(9223372036854775807u64);
        let v4 = serde_json::json!({"branches": 1000, "depths": [1, 2, 3]});
        assert!(validate_telemetry_numbers(&v1).is_ok());
        assert!(validate_telemetry_numbers(&v2).is_ok());
        assert!(validate_telemetry_numbers(&v3).is_ok());
        assert!(validate_telemetry_numbers(&v4).is_ok());
    }

    #[test]
    fn test_validate_telemetry_numbers_exceeds_limits() {
        // Number exceeding 64-bit integer limits
        let v_huge = serde_json::json!(1e25);
        assert!(validate_telemetry_numbers(&v_huge).is_err());
    }

    #[cfg(feature = "signing")]
    #[test]
    fn test_hash_tcb_runtime() {
        let temp_dir = std::env::temp_dir().join("ualbf_tcb_test");
        let base_dir = temp_dir.join("rust-engine/src");
        let _ = std::fs::create_dir_all(&base_dir);

        for file in CORE_TCB_FILES {
            let path = base_dir.join(file);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, b"test content");
        }

        let hash_res = compute_verified_core_hash_runtime(&temp_dir);
        assert!(hash_res.is_ok());
        let hash = hash_res.unwrap();
        assert_eq!(hash.len(), 64);

        let hash_ext_res = compute_verified_extension_hash_runtime(&temp_dir);
        assert!(hash_ext_res.is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[cfg(feature = "signing")]
    #[test]
    fn test_rust_free_string_and_sha256() {
        let temp_path = std::env::temp_dir().join("test_sha256_file.txt");
        let _ = std::fs::write(&temp_path, b"hello world");

        let c_path = std::ffi::CString::new(temp_path.to_str().unwrap()).unwrap();
        let ptr = rust_sha256_file(c_path.as_ptr());
        assert!(!ptr.is_null());

        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(
            c_str.to_str().unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        // Safe free pointer
        rust_free_string(ptr);

        // Safe free null pointer
        rust_free_string(std::ptr::null_mut());

        let _ = std::fs::remove_file(&temp_path);
    }

    #[cfg(feature = "signing")]
    #[test]
    fn test_rust_sha256_file_non_null_terminated() {
        let no_null_path = vec![b'a'; MAX_PATH_LEN];
        let ptr = rust_sha256_file(no_null_path.as_ptr() as *const std::ffi::c_char);
        assert!(ptr.is_null());
    }

    #[cfg(feature = "signing")]
    #[test]
    fn test_verify_certificate_non_null_terminated() {
        let no_null_json = vec![b'{'; 100];
        let valid_key = b"pubkey\0";
        let mut is_valid = true;
        let mut err_buf = [0i8; 256];

        let res = verify_certificate(
            no_null_json.as_ptr() as *const std::ffi::c_char,
            valid_key.as_ptr() as *const std::ffi::c_char,
            &mut is_valid,
            err_buf.as_mut_ptr(),
            256,
        );
        assert!(res.is_null());
        assert!(!is_valid);

        let valid_json = b"{}\0";
        let no_null_key = vec![b'k'; 100];
        let res2 = verify_certificate(
            valid_json.as_ptr() as *const std::ffi::c_char,
            no_null_key.as_ptr() as *const std::ffi::c_char,
            &mut is_valid,
            err_buf.as_mut_ptr(),
            256,
        );
        assert!(res2.is_null());
        assert!(!is_valid);
    }

    #[test]
    fn test_normalize_proof_manifest() {
        let raw = r#"{
  "verified_logic_hash": "73b1c6a115b09bf8a39ac1c0536aefe55ef90731b75b3427cdddba8a365ee9f3",
  "verified_extension_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}"#;
        let normalized = normalize_proof_manifest(raw);
        assert!(normalized.contains("\"verified_logic_hash\": \"0000000000000000000000000000000000000000000000000000000000000000\""));
        assert!(normalized.contains("\"verified_extension_hash\": \"0000000000000000000000000000000000000000000000000000000000000000\""));
    }
}
