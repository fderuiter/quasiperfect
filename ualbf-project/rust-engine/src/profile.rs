use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerformanceProfile {
    pub pollard_rho_batch_size: u32,
    pub active_prime_slots: usize,
    pub engine_telemetry_interval_ms: u64,
    pub dashboard_telemetry_interval_ms: u64,
}

impl Default for PerformanceProfile {
    fn default() -> Self {
        Self {
            pollard_rho_batch_size: 128,
            active_prime_slots: 64,
            engine_telemetry_interval_ms: 1000,
            dashboard_telemetry_interval_ms: 250,
        }
    }
}

pub fn load_profile() -> PerformanceProfile {
    match fs::read_to_string("profile.json") {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(profile) => profile,
            Err(_) => PerformanceProfile::default(),
        },
        Err(_) => PerformanceProfile::default(),
    }
}

pub fn get_profile() -> &'static PerformanceProfile {
    static PROFILE: OnceLock<PerformanceProfile> = OnceLock::new();
    PROFILE.get_or_init(|| load_profile())
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_performance_profile_default() {
        let profile = PerformanceProfile::default();
        assert_eq!(profile.pollard_rho_batch_size, 128);
        assert_eq!(profile.active_prime_slots, 64);
        assert_eq!(profile.engine_telemetry_interval_ms, 1000);
        assert_eq!(profile.dashboard_telemetry_interval_ms, 250);
    }

    #[test]
    fn test_performance_profile_custom_json() {
        let json = r#"{
            "pollard_rho_batch_size": 256,
            "active_prime_slots": 128,
            "engine_telemetry_interval_ms": 2000,
            "dashboard_telemetry_interval_ms": 500
        }"#;
        let profile: PerformanceProfile =
            serde_json::from_str(json).expect("Failed to parse custom profile");
        assert_eq!(profile.pollard_rho_batch_size, 256);
        assert_eq!(profile.active_prime_slots, 128);
        assert_eq!(profile.engine_telemetry_interval_ms, 2000);
        assert_eq!(profile.dashboard_telemetry_interval_ms, 500);
    }

    #[test]
    fn test_load_profile_fallback_behavior() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let orig_dir = std::env::current_dir().expect("Failed to get current dir");
        let temp_dir =
            std::env::temp_dir().join(format!("ualbf_test_profile_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        std::env::set_current_dir(&temp_dir).expect("Failed to set current dir");

        // 1. Missing file fallback
        let profile_missing = load_profile();
        assert_eq!(profile_missing.pollard_rho_batch_size, 128);
        assert_eq!(profile_missing.active_prime_slots, 64);

        // 2. Corrupt file fallback
        let corrupt_path = temp_dir.join("profile.json");
        std::fs::write(&corrupt_path, "{ invalid json }").expect("Failed to write corrupt file");

        let profile_corrupt = load_profile();
        assert_eq!(profile_corrupt.pollard_rho_batch_size, 128);
        assert_eq!(profile_corrupt.active_prime_slots, 64);

        // 3. Valid profile loading
        let valid_json = r#"{
            "pollard_rho_batch_size": 512,
            "active_prime_slots": 32,
            "engine_telemetry_interval_ms": 1500,
            "dashboard_telemetry_interval_ms": 300
        }"#;
        std::fs::write(&corrupt_path, valid_json).expect("Failed to write valid file");

        let profile_valid = load_profile();
        assert_eq!(profile_valid.pollard_rho_batch_size, 512);
        assert_eq!(profile_valid.active_prime_slots, 32);
        assert_eq!(profile_valid.engine_telemetry_interval_ms, 1500);
        assert_eq!(profile_valid.dashboard_telemetry_interval_ms, 300);

        std::env::set_current_dir(orig_dir).expect("Failed to restore orig dir");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_profile_static_initialization() {
        let p1 = get_profile();
        let p2 = get_profile();

        assert!(std::ptr::eq(p1, p2));
        assert_eq!(p1.pollard_rho_batch_size, p2.pollard_rho_batch_size);
        assert_eq!(p1.active_prime_slots, p2.active_prime_slots);
    }
}
