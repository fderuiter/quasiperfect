use crate::profile::{load_profile, PerformanceProfile};
use std::env;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub target_min_log10: u32,
    pub target_max_log10: u32,
    pub sieve_limit: usize,
    pub max_exponent: u32,
    pub prefix_stop: u64,
    pub proof_manifest: String,
    pub enable_diagnostics: bool,
    pub mode: String,
    pub controller_addr: String,
    pub fp_rate: f64,
    pub perf_profile: PerformanceProfile,
    pub sampling_rate: Option<f64>,
    pub deterministic_seed: Option<u64>,
    pub trial_division_limit: usize,
    pub proof_mode: String,
}

use std::sync::OnceLock;

static CONFIG: OnceLock<EngineConfig> = OnceLock::new();

pub fn get_proof_mode() -> String {
    if let Ok(m) = env::var("UALBF_PROOF_MODE") {
        let m_lower = m.to_lowercase();
        if m_lower == "pure" {
            return "pure".to_string();
        } else if m_lower == "axiomatic" {
            return "axiomatic".to_string();
        }
    }
    for arg in env::args() {
        if arg.starts_with("--proof-mode=") {
            let mode = arg.trim_start_matches("--proof-mode=");
            if mode == "pure" {
                return "pure".to_string();
            } else if mode == "axiomatic" {
                return "axiomatic".to_string();
            }
        } else if arg == "--pure-proofs" {
            return "pure".to_string();
        }
    }
    "axiomatic".to_string()
}

pub fn get_safe_config() -> &'static EngineConfig {
    CONFIG.get_or_init(|| parse_config())
}

pub fn parse_config() -> EngineConfig {
    // Check for deprecated GPU-related CLI arguments or environment variables
    for arg in env::args() {
        if arg.contains("gpu") || arg.contains("GPU") {
            eprintln!("WARNING: Runtime flag '{}' is deprecated. The unverified GPU path has been eliminated; all calculations now run securely on the CPU.", arg);
        }
    }
    for (key, val) in env::vars() {
        if key.contains("GPU") {
            eprintln!("WARNING: Environment variable '{}' is deprecated. The unverified GPU path has been eliminated; all calculations now run securely on the CPU.", key);
        }
    }

    let target_min_log10 = match env::var("UALBF_TARGET_MIN_LOG10") {
        Ok(v) => v
            .parse()
            .expect("FATAL: UALBF_TARGET_MIN_LOG10 must be a valid u32"),
        Err(_) => crate::lean_ffi::get_target_min_log10(),
    };

    let target_max_log10 = match env::var("UALBF_TARGET_MAX_LOG10") {
        Ok(v) => v
            .parse()
            .expect("FATAL: UALBF_TARGET_MAX_LOG10 must be a valid u32"),
        Err(_) => crate::lean_ffi::get_target_max_log10(),
    };

    let sieve_limit = match env::var("UALBF_SIEVE_LIMIT") {
        Ok(v) => v
            .parse()
            .expect("FATAL: UALBF_SIEVE_LIMIT must be a valid usize"),
        Err(_) => crate::lean_ffi::get_sieve_limit(),
    };

    let max_exponent = match env::var("UALBF_MAX_EXPONENT") {
        Ok(v) => v
            .parse()
            .expect("FATAL: UALBF_MAX_EXPONENT must be a valid u32"),
        Err(_) => crate::lean_ffi::get_max_exponent(),
    };

    let prefix_stop = match env::var("UALBF_PREFIX_STOP_THRESHOLD") {
        Ok(v) => v
            .parse()
            .expect("FATAL: UALBF_PREFIX_STOP_THRESHOLD must be a valid u64"),
        Err(_) => crate::lean_ffi::get_prefix_stop_threshold(),
    };

    let proof_manifest =
        env::var("UALBF_PROOF_MANIFEST").unwrap_or_else(|_| "../proof_manifest.json".to_string());

    let enable_diagnostics = env::var("UALBF_ENABLE_DIAGNOSTICS")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    let mode = env::var("UALBF_MODE").unwrap_or_else(|_| "standalone".to_string());

    let controller_addr = env::var("UALBF_CONTROLLER_ADDR").unwrap_or_else(|_| {
        if mode == "controller" {
            "0.0.0.0:8080".to_string()
        } else {
            "127.0.0.1:8080".to_string()
        }
    });

    let fp_rate = env::var("UALBF_FP_RATE")
        .unwrap_or_else(|_| "0.01".to_string())
        .parse::<f64>()
        .expect("FATAL: UALBF_FP_RATE must be a valid f64");

    let perf_profile = load_profile();

    let sampling_rate = env::var("UALBF_SAMPLING_RATE").ok().map(|v| {
        v.parse::<f64>()
            .expect("FATAL: UALBF_SAMPLING_RATE must be a valid f64")
    });
    let deterministic_seed = env::var("UALBF_DETERMINISTIC_SEED").ok().map(|v| {
        v.parse::<u64>()
            .expect("FATAL: UALBF_DETERMINISTIC_SEED must be a valid u64")
    });

    let trial_division_limit = env::var("UALBF_TRIAL_DIVISION_LIMIT")
        .unwrap_or_else(|_| "10000000".to_string())
        .parse::<usize>()
        .expect("FATAL: UALBF_TRIAL_DIVISION_LIMIT must be a valid usize");

    let proof_mode = get_proof_mode();

    let config = EngineConfig {
        target_min_log10,
        target_max_log10,
        sieve_limit,
        max_exponent,
        prefix_stop,
        proof_manifest,
        enable_diagnostics,
        mode,
        controller_addr,
        fp_rate,
        perf_profile,
        sampling_rate,
        deterministic_seed,
        trial_division_limit,
        proof_mode,
    };

    if config.target_max_log10 < config.target_min_log10 {
        panic!("FATAL: The runtime search range is empty. target_max_log10 ({}) is less than target_min_log10 ({}).", config.target_max_log10, config.target_min_log10);
    }

    // Check that the minimum search bound respects the proven mathematical floor
    if config.target_min_log10 < crate::lean_ffi::get_target_min_log10() {
        panic!("FATAL: Runtime value for UALBF_TARGET_MIN_LOG10 ({}) expands below proven manifest minimum ({}). The requested bound requires a formal proof in the manifest first.", config.target_min_log10, crate::lean_ffi::get_target_min_log10());
    }

    if config.target_max_log10 > crate::lean_ffi::get_target_max_log10() {
        panic!("FATAL: Runtime value for UALBF_TARGET_MAX_LOG10 ({}) exceeds proven manifest maximum ({}). The requested bound requires a formal proof in the manifest first.", config.target_max_log10, crate::lean_ffi::get_target_max_log10());
    }

    if config.sieve_limit > crate::lean_ffi::get_sieve_limit() {
        panic!("FATAL: Runtime value for UALBF_SIEVE_LIMIT ({}) exceeds proven manifest maximum ({}). The requested bound requires a formal proof in the manifest first.", config.sieve_limit, crate::lean_ffi::get_sieve_limit());
    }

    if config.max_exponent > crate::lean_ffi::get_max_exponent() {
        panic!("FATAL: Runtime value for UALBF_MAX_EXPONENT ({}) exceeds proven manifest maximum ({}). The requested bound requires a formal proof in the manifest first.", config.max_exponent, crate::lean_ffi::get_max_exponent());
    }

    if config.prefix_stop > crate::lean_ffi::get_prefix_stop_threshold() {
        panic!("FATAL: Runtime value for UALBF_PREFIX_STOP_THRESHOLD ({}) exceeds proven manifest maximum ({}). The requested bound requires a formal proof in the manifest first.", config.prefix_stop, crate::lean_ffi::get_prefix_stop_threshold());
    }

    if config.trial_division_limit > 100_000_000 {
        panic!("FATAL: Configured value for UALBF_TRIAL_DIVISION_LIMIT ({}) exceeds the maximum safe limit allowed (100000000).", config.trial_division_limit);
    }

    config
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;
    pub(crate) static TEST_MUTEX: Mutex<()> = Mutex::new(());
    use super::*;
    use std::env;

    #[test]
    fn test_policy_clamping_max() {
        let _guard = TEST_MUTEX.lock().unwrap();
        env::set_var("UALBF_TARGET_MAX_LOG10", "100");
        let result = std::panic::catch_unwind(|| {
            parse_config();
        });
        env::remove_var("UALBF_TARGET_MAX_LOG10");
        assert!(
            result.is_err(),
            "Expected panic when TARGET_MAX_LOG10 exceeds limits"
        );
    }

    #[test]
    fn test_policy_clamping_min() {
        let _guard = TEST_MUTEX.lock().unwrap();
        env::set_var("UALBF_TARGET_MIN_LOG10", "1");
        let result = std::panic::catch_unwind(|| {
            parse_config();
        });
        env::remove_var("UALBF_TARGET_MIN_LOG10");
        assert!(
            result.is_err(),
            "Expected panic when TARGET_MIN_LOG10 expands below limits"
        );

        env::set_var("UALBF_TARGET_MIN_LOG10", "38");
        env::set_var("UALBF_TARGET_MAX_LOG10", "40");
        let result = std::panic::catch_unwind(|| {
            parse_config();
        });
        env::remove_var("UALBF_TARGET_MIN_LOG10");
        env::remove_var("UALBF_TARGET_MAX_LOG10");
        assert!(
            result.is_ok(),
            "Expected success when safely narrowing search space"
        );
    }

    #[test]
    fn test_policy_trial_division_limit_propagation() {
        let _guard = TEST_MUTEX.lock().unwrap();
        env::set_var("UALBF_TRIAL_DIVISION_LIMIT", "50000000");
        let config = parse_config();
        env::remove_var("UALBF_TRIAL_DIVISION_LIMIT");
        assert_eq!(config.trial_division_limit, 50_000_000);
    }

    #[test]
    fn test_policy_trial_division_limit_safe_bounds() {
        let _guard = TEST_MUTEX.lock().unwrap();
        env::set_var("UALBF_TRIAL_DIVISION_LIMIT", "100000001");
        let result = std::panic::catch_unwind(|| {
            parse_config();
        });
        env::remove_var("UALBF_TRIAL_DIVISION_LIMIT");
        assert!(
            result.is_err(),
            "Expected panic when TRIAL_DIVISION_LIMIT exceeds limits"
        );
    }

    #[test]
    fn test_get_proof_mode() {
        let _guard = TEST_MUTEX.lock().unwrap();
        env::set_var("UALBF_PROOF_MODE", "pure");
        assert_eq!(get_proof_mode(), "pure");

        env::set_var("UALBF_PROOF_MODE", "axiomatic");
        assert_eq!(get_proof_mode(), "axiomatic");

        env::remove_var("UALBF_PROOF_MODE");
        assert_eq!(get_proof_mode(), "axiomatic");
    }
}
