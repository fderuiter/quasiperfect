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
        logic_hasher.update(include_bytes!("../../proof_manifest.json"));
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
        logic_hasher.update(&content);
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
pub fn validate_certificate<'py>(
    py: Python<'py>,
    cert_json_str: &str,
) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::exceptions::{PyException, PyValueError};

    // Null-byte check before parsing
    if cert_json_str.contains('\0') {
        return Err(PyValueError::new_err(
            "Null byte detected in certificate content",
        ));
    }

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

    // Verify signature
    let is_valid = verify_signature(public_key, signature, &payload)
        .map_err(|e| PyException::new_err(format!("Signature verification error: {}", e)))?;

    if !is_valid {
        return Err(PyException::new_err("Invalid cryptographic signature"));
    }

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

#[cfg(feature = "python")]
#[pyfunction]
pub fn check_path_continuity(path_ranges_json: &str) -> PyResult<String> {
    use pyo3::exceptions::PyValueError;

    let mut ranges: Vec<RangeWorkUnit> = serde_json::from_str(path_ranges_json)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse path ranges JSON: {}", e)))?;

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

    let gaps_json = serde_json::to_string(&result)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize gaps JSON: {}", e)))?;

    Ok(gaps_json)
}

#[cfg(feature = "python")]
#[pymodule]
fn verification_lib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_certificate, m)?)?;
    m.add_function(wrap_pyfunction!(hash_tcb, m)?)?;
    m.add_function(wrap_pyfunction!(hash_extension_tcb, m)?)?;
    m.add_function(wrap_pyfunction!(check_path_continuity, m)?)?;
    Ok(())
}

#[cfg(feature = "signing")]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn verify_certificate(
    cert_json_ptr: *const std::ffi::c_char,
    pub_key_ptr: *const std::ffi::c_char,
    is_valid_out: *mut bool,
    out_manifest_hash_buf: *mut std::ffi::c_char,
    out_manifest_hash_len: usize,
) -> *mut std::ffi::c_void {
    use std::ffi::CStr;

    unsafe {
        *is_valid_out = false;
    }

    let write_error = |err: &str| unsafe {
        if !out_manifest_hash_buf.is_null() && out_manifest_hash_len > 0 {
            let bytes = err.as_bytes();
            let copy_len = std::cmp::min(bytes.len(), out_manifest_hash_len - 1);
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                out_manifest_hash_buf as *mut u8,
                copy_len,
            );
            *out_manifest_hash_buf.add(copy_len) = 0;
        }
    };

    if cert_json_ptr.is_null() || pub_key_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let cert_json_str = unsafe { CStr::from_ptr(cert_json_ptr) }.to_string_lossy();
    let expected_pub_key = unsafe { CStr::from_ptr(pub_key_ptr) }.to_string_lossy();

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

    unsafe {
        *is_valid_out = true;
    }
    write_error(manifest_hash);

    Box::into_raw(Box::new(cert)) as *mut std::ffi::c_void
}

#[cfg(feature = "signing")]
#[no_mangle]
pub extern "C" fn free_certificate(cert_ptr: *mut std::ffi::c_void) {
    if !cert_ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(cert_ptr as *mut serde_json::Value);
        }
    }
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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rust_sha256_file(path_ptr: *const std::ffi::c_char) -> *mut std::ffi::c_char {
    use sha2::{Digest, Sha256};
    if path_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(path_ptr) };
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
}

#[cfg(feature = "signing")]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rust_free_string(ptr: *mut std::ffi::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        }
    }
}
