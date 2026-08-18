use crate::profile::{load_profile, PerformanceProfile};
use std::collections::HashMap;
use std::env;
use std::sync::OnceLock;

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
    pub sidecar_path: String,
    pub heartbeat_timeout_sec: u64,
    pub heartbeat_interval_sec: u64,
    pub verify_sidecar: Option<String>,
}

static CONFIG: OnceLock<EngineConfig> = OnceLock::new();

pub fn get_proof_mode_from<IArgs, IVars>(args: IArgs, vars: IVars) -> String
where
    IArgs: IntoIterator<Item = String>,
    IVars: IntoIterator<Item = (String, String)>,
{
    let args_vec: Vec<String> = args.into_iter().collect();
    let vars_map: HashMap<String, String> = vars.into_iter().collect();

    let mut cli_mode: Option<String> = None;
    let mut i = 0;
    while i < args_vec.len() {
        let arg = &args_vec[i];
        if arg.starts_with("--proof-mode=") {
            cli_mode = Some(arg.trim_start_matches("--proof-mode=").to_string());
        } else if arg == "--proof-mode" && i + 1 < args_vec.len() {
            cli_mode = Some(args_vec[i + 1].clone());
            i += 1;
        } else if arg == "--pure-proofs" {
            cli_mode = Some("pure".to_string());
        }
        i += 1;
    }

    let raw_mode = if let Some(m) = cli_mode {
        m
    } else if let Some(m) = vars_map.get("UALBF_PROOF_MODE") {
        m.clone()
    } else {
        "axiomatic".to_string()
    };

    match raw_mode.to_lowercase().as_str() {
        "pure" => "pure".to_string(),
        "axiomatic" => "axiomatic".to_string(),
        _ => panic!(
            "FATAL: Invalid proof mode '{}'. Allowed values are 'pure' or 'axiomatic'.",
            raw_mode
        ),
    }
}

pub fn get_proof_mode() -> String {
    get_proof_mode_from(env::args(), env::vars())
}

pub fn get_safe_config() -> &'static EngineConfig {
    CONFIG.get_or_init(|| parse_config())
}

pub fn parse_config() -> EngineConfig {
    parse_config_from(env::args(), env::vars())
}

pub fn parse_config_from<IArgs, IVars>(args: IArgs, vars: IVars) -> EngineConfig
where
    IArgs: IntoIterator<Item = String>,
    IVars: IntoIterator<Item = (String, String)>,
{
    let args_vec: Vec<String> = args.into_iter().collect();
    let vars_map: HashMap<String, String> = vars.into_iter().collect();

    // Check for deprecated GPU-related CLI arguments or environment variables
    for arg in args_vec.iter().skip(1) {
        if arg.contains("gpu") || arg.contains("GPU") {
            eprintln!(
                "WARNING: Runtime flag '{}' is deprecated. The unverified GPU path has been eliminated; all calculations now run securely on the CPU.",
                arg
            );
        }
    }
    for (key, _val) in vars_map.iter() {
        if key.contains("GPU") {
            eprintln!(
                "WARNING: Environment variable '{}' is deprecated. The unverified GPU path has been eliminated; all calculations now run securely on the CPU.",
                key
            );
        }
    }

    let mut cli_map: HashMap<String, String> = HashMap::new();

    let value_flags = [
        "--target-min-log10",
        "--target-max-log10",
        "--sieve-limit",
        "--max-exponent",
        "--prefix-stop",
        "--proof-manifest",
        "--mode",
        "--controller-addr",
        "--fp-rate",
        "--sampling-rate",
        "--deterministic-seed",
        "--trial-division-limit",
        "--proof-mode",
        "--sidecar-path",
        "--heartbeat-timeout-sec",
        "--heartbeat-interval-sec",
        "--verify-sidecar",
        "--test-threads",
        "--format",
        "--color",
        "--logfile",
        "--shuffle-seed",
    ];

    let mut i = 1;
    while i < args_vec.len() {
        let arg = &args_vec[i];
        if arg.starts_with('-') {
            let (key, val) = if let Some(eq_pos) = arg.find('=') {
                let key = &arg[..eq_pos];
                let val = &arg[eq_pos + 1..];
                (key.to_string(), val.to_string())
            } else if value_flags.contains(&arg.as_str()) {
                if i + 1 < args_vec.len() && !args_vec[i + 1].starts_with('-') {
                    let key = arg.to_string();
                    let val = args_vec[i + 1].to_string();
                    i += 1;
                    (key, val)
                } else {
                    (arg.to_string(), "true".to_string())
                }
            } else if arg == "--pure-proofs" {
                ("--proof-mode".to_string(), "pure".to_string())
            } else {
                (arg.to_string(), "true".to_string())
            };

            match key.as_str() {
                "--target-min-log10"
                | "--target-max-log10"
                | "--sieve-limit"
                | "--max-exponent"
                | "--prefix-stop"
                | "--proof-manifest"
                | "--enable-diagnostics"
                | "--mode"
                | "--controller-addr"
                | "--fp-rate"
                | "--sampling-rate"
                | "--deterministic-seed"
                | "--trial-division-limit"
                | "--proof-mode"
                | "--sidecar-path"
                | "--heartbeat-timeout-sec"
                | "--heartbeat-interval-sec"
                | "--verify-sidecar" => {
                    cli_map.insert(key, val);
                }
                k if k.contains("gpu") || k.contains("GPU") => {
                    // Deprecated GPU flag, warning already issued
                }
                "--nocapture"
                | "--exact"
                | "--test-threads"
                | "--format"
                | "--color"
                | "--show-output"
                | "--bench"
                | "--test"
                | "--ignored"
                | "--include-ignored"
                | "--force-run-ignored"
                | "--quiet"
                | "--list"
                | "--logfile"
                | "--shuffle"
                | "--shuffle-seed"
                | "--ensure-time"
                | "--help"
                | "--version"
                | "-h"
                | "-q"
                | "-v"
                | "-Z" => {
                    // Ignore cargo test / libtest harness flags
                }
                _ => {
                    panic!(
                        "FATAL: Unrecognized command-line flag '{}'. Initialization aborted.",
                        arg
                    );
                }
            }
        }
        i += 1;
    }

    let get_opt = |cli_key: &str, env_key: &str| -> Option<String> {
        if let Some(v) = cli_map.get(cli_key) {
            Some(v.clone())
        } else if let Some(v) = vars_map.get(env_key) {
            Some(v.clone())
        } else {
            None
        }
    };

    let target_min_log10 = match get_opt("--target-min-log10", "UALBF_TARGET_MIN_LOG10") {
        Some(v) => v
            .parse::<u32>()
            .expect("FATAL: UALBF_TARGET_MIN_LOG10 / --target-min-log10 must be a valid u32"),
        None => crate::lean_ffi::get_target_min_log10(),
    };

    let target_max_log10 = match get_opt("--target-max-log10", "UALBF_TARGET_MAX_LOG10") {
        Some(v) => v
            .parse::<u32>()
            .expect("FATAL: UALBF_TARGET_MAX_LOG10 / --target-max-log10 must be a valid u32"),
        None => crate::lean_ffi::get_target_max_log10(),
    };

    let sieve_limit = match get_opt("--sieve-limit", "UALBF_SIEVE_LIMIT") {
        Some(v) => v
            .parse::<usize>()
            .expect("FATAL: UALBF_SIEVE_LIMIT / --sieve-limit must be a valid usize"),
        None => crate::lean_ffi::get_sieve_limit(),
    };

    let max_exponent = match get_opt("--max-exponent", "UALBF_MAX_EXPONENT") {
        Some(v) => v
            .parse::<u32>()
            .expect("FATAL: UALBF_MAX_EXPONENT / --max-exponent must be a valid u32"),
        None => crate::lean_ffi::get_max_exponent(),
    };

    let prefix_stop = match get_opt("--prefix-stop", "UALBF_PREFIX_STOP_THRESHOLD") {
        Some(v) => v
            .parse::<u64>()
            .expect("FATAL: UALBF_PREFIX_STOP_THRESHOLD / --prefix-stop must be a valid u64"),
        None => crate::lean_ffi::get_prefix_stop_threshold(),
    };

    let proof_manifest = get_opt("--proof-manifest", "UALBF_PROOF_MANIFEST")
        .unwrap_or_else(|| "../proof_manifest.json".to_string());

    let enable_diagnostics = get_opt("--enable-diagnostics", "UALBF_ENABLE_DIAGNOSTICS")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    let mode_raw = get_opt("--mode", "UALBF_MODE").unwrap_or_else(|| "standalone".to_string());
    let mode = match mode_raw.to_lowercase().as_str() {
        "standalone" => "standalone".to_string(),
        "controller" => "controller".to_string(),
        "worker" => "worker".to_string(),
        _ => panic!(
            "FATAL: Invalid engine mode '{}'. Allowed values are 'standalone', 'controller', or 'worker'.",
            mode_raw
        ),
    };

    let controller_addr =
        get_opt("--controller-addr", "UALBF_CONTROLLER_ADDR").unwrap_or_else(|| {
            if mode == "controller" {
                "0.0.0.0:8080".to_string()
            } else {
                "127.0.0.1:8080".to_string()
            }
        });

    let fp_rate = match get_opt("--fp-rate", "UALBF_FP_RATE") {
        Some(v) => v
            .parse::<f64>()
            .expect("FATAL: UALBF_FP_RATE / --fp-rate must be a valid f64"),
        None => 0.01,
    };

    let perf_profile = load_profile();

    let sampling_rate = get_opt("--sampling-rate", "UALBF_SAMPLING_RATE").map(|v| {
        v.parse::<f64>()
            .expect("FATAL: UALBF_SAMPLING_RATE / --sampling-rate must be a valid f64")
    });

    let deterministic_seed = get_opt("--deterministic-seed", "UALBF_DETERMINISTIC_SEED").map(|v| {
        v.parse::<u64>()
            .expect("FATAL: UALBF_DETERMINISTIC_SEED / --deterministic-seed must be a valid u64")
    });

    let trial_division_limit = match get_opt("--trial-division-limit", "UALBF_TRIAL_DIVISION_LIMIT")
    {
        Some(v) => v.parse::<usize>().expect(
            "FATAL: UALBF_TRIAL_DIVISION_LIMIT / --trial-division-limit must be a valid usize",
        ),
        None => 10_000_000,
    };

    let proof_mode_raw =
        get_opt("--proof-mode", "UALBF_PROOF_MODE").unwrap_or_else(|| "axiomatic".to_string());
    let proof_mode = match proof_mode_raw.to_lowercase().as_str() {
        "pure" => "pure".to_string(),
        "axiomatic" => "axiomatic".to_string(),
        _ => panic!(
            "FATAL: Invalid proof mode '{}'. Allowed values are 'pure' or 'axiomatic'.",
            proof_mode_raw
        ),
    };

    let sidecar_path = get_opt("--sidecar-path", "UALBF_SIDECAR_PATH")
        .unwrap_or_else(|| "overflow_sidecar.log".to_string());

    let heartbeat_timeout_sec =
        match get_opt("--heartbeat-timeout-sec", "UALBF_HEARTBEAT_TIMEOUT_SEC") {
            Some(v) => v.parse::<u64>().expect(
                "FATAL: UALBF_HEARTBEAT_TIMEOUT_SEC / --heartbeat-timeout-sec must be a valid u64",
            ),
            None => 15,
        };

    let heartbeat_interval_sec = match get_opt(
        "--heartbeat-interval-sec",
        "UALBF_HEARTBEAT_INTERVAL_SEC",
    ) {
        Some(v) => v.parse::<u64>().expect(
            "FATAL: UALBF_HEARTBEAT_INTERVAL_SEC / --heartbeat-interval-sec must be a valid u64",
        ),
        None => 5,
    };

    let verify_sidecar = get_opt("--verify-sidecar", "UALBF_VERIFY_SIDECAR");

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
        sidecar_path,
        heartbeat_timeout_sec,
        heartbeat_interval_sec,
        verify_sidecar,
    };

    if config.target_max_log10 < config.target_min_log10 {
        panic!("FATAL: The runtime search range is empty. target_max_log10 ({}) is less than target_min_log10 ({}).", config.target_max_log10, config.target_min_log10);
    }

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
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

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
        let cfg = parse_config_from(
            vec!["engine".to_string()],
            vec![("UALBF_PROOF_MODE".to_string(), "pure".to_string())],
        );
        assert_eq!(cfg.proof_mode, "pure");

        let cfg = parse_config_from(
            vec!["engine".to_string()],
            vec![("UALBF_PROOF_MODE".to_string(), "axiomatic".to_string())],
        );
        assert_eq!(cfg.proof_mode, "axiomatic");

        let cfg = parse_config_from(vec!["engine".to_string()], Vec::<(String, String)>::new());
        assert_eq!(cfg.proof_mode, "axiomatic");
        env::remove_var("UALBF_PROOF_MODE");
    }

    #[test]
    fn test_cli_overrides_env_precedence() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let args = vec![
            "engine".to_string(),
            "--target-min-log10=38".to_string(),
            "--proof-mode=pure".to_string(),
        ];
        let vars = vec![
            ("UALBF_TARGET_MIN_LOG10".to_string(), "35".to_string()),
            ("UALBF_PROOF_MODE".to_string(), "axiomatic".to_string()),
        ];
        let cfg = parse_config_from(args, vars);
        assert_eq!(cfg.target_min_log10, 38);
        assert_eq!(cfg.proof_mode, "pure");
    }

    #[test]
    fn test_invalid_proof_mode_panics() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let args = vec!["engine".to_string(), "--proof-mode=puer".to_string()];
        let vars = vec![];
        let res = std::panic::catch_unwind(|| parse_config_from(args, vars));
        assert!(res.is_err(), "Expected panic on invalid proof mode string");
    }

    #[test]
    fn test_invalid_engine_mode_panics() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let args = vec!["engine".to_string(), "--mode=invalid_mode".to_string()];
        let vars = vec![];
        let res = std::panic::catch_unwind(|| parse_config_from(args, vars));
        assert!(res.is_err(), "Expected panic on invalid engine mode string");
    }

    #[test]
    fn test_unrecognized_cli_flag_panics() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let args = vec!["engine".to_string(), "--prof-mode=pure".to_string()];
        let vars = vec![];
        let res = std::panic::catch_unwind(|| parse_config_from(args, vars));
        assert!(res.is_err(), "Expected panic on unrecognized CLI flag");
    }

    #[test]
    fn test_pure_proofs_flag() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let args = vec!["engine".to_string(), "--pure-proofs".to_string()];
        let vars = vec![("UALBF_PROOF_MODE".to_string(), "axiomatic".to_string())];
        let cfg = parse_config_from(args, vars);
        assert_eq!(cfg.proof_mode, "pure");
    }
}
