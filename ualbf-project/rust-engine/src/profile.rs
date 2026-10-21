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
    load_profile_from_path("profile.json")
}

pub fn load_profile_from_path<P: AsRef<std::path::Path>>(path: P) -> PerformanceProfile {
    let target_path = path.as_ref();

    if target_path.exists() {
        match fs::read_to_string(target_path) {
            Ok(content) => match serde_json::from_str::<PerformanceProfile>(&content) {
                Ok(profile) => return profile,
                Err(err) => {
                    eprintln!(
                        "Warning: Failed to parse profile at {:?}: {}. Falling back to default profile.",
                        target_path, err
                    );
                    return PerformanceProfile::default();
                }
            },
            Err(err) => {
                eprintln!(
                    "Warning: Failed to read profile at {:?}: {}. Falling back to default profile.",
                    target_path, err
                );
                return PerformanceProfile::default();
            }
        }
    }

    // Auto-instantiate profile.json if target_path does not exist
    let parent = target_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));

    let file_name = target_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let is_current_dir = parent == std::path::Path::new("") || parent == std::path::Path::new(".");

    let mut candidate_templates = vec![
        parent.join("profile.json.template"),
        parent.join(format!("{}.template", file_name)),
    ];

    if is_current_dir {
        candidate_templates
            .push(std::path::Path::new("rust-engine/profile.json.template").to_path_buf());
    }

    let mut template_path_found = None;
    for candidate in &candidate_templates {
        if candidate.exists() {
            template_path_found = Some(candidate.clone());
            break;
        }
    }

    if let Some(template_path) = template_path_found {
        if let Ok(content) = fs::read_to_string(&template_path) {
            if let Ok(profile) = serde_json::from_str::<PerformanceProfile>(&content) {
                if let Err(err) = fs::write(target_path, &content) {
                    eprintln!(
                        "Warning: Failed to auto-instantiate profile at {:?} from template {:?}: {}. Falling back to default profile.",
                        target_path, template_path, err
                    );
                } else {
                    eprintln!(
                        "Info: Auto-instantiated profile configuration at {:?} from template {:?}",
                        target_path, template_path
                    );
                }
                return profile;
            } else {
                eprintln!(
                    "Warning: Failed to parse profile template at {:?}. Falling back to default profile.",
                    template_path
                );
            }
        } else {
            eprintln!(
                "Warning: Failed to read profile template at {:?}. Falling back to default profile.",
                template_path
            );
        }
    }

    // Fallback: If no template exists or reading template failed, write default JSON
    let default_profile = PerformanceProfile::default();
    match serde_json::to_string_pretty(&default_profile) {
        Ok(pretty_json) => {
            if let Err(err) = fs::write(target_path, &pretty_json) {
                eprintln!(
                    "Warning: Failed to auto-generate default profile at {:?}: {}. Falling back to in-memory defaults.",
                    target_path, err
                );
            } else {
                eprintln!(
                    "Info: Auto-generated default profile configuration at {:?}",
                    target_path
                );
            }
        }
        Err(err) => {
            eprintln!(
                "Warning: Failed to serialize default profile: {}. Falling back to in-memory defaults.",
                err
            );
        }
    }

    default_profile
}

pub fn get_profile() -> &'static PerformanceProfile {
    static PROFILE: OnceLock<PerformanceProfile> = OnceLock::new();
    PROFILE.get_or_init(|| load_profile())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let temp_dir =
            std::env::temp_dir().join(format!("ualbf_test_profile_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // 1. Missing file auto-generation (when no template exists)
        let missing_path = temp_dir.join("non_existent_profile.json");
        let profile_missing = load_profile_from_path(&missing_path);
        assert_eq!(profile_missing.pollard_rho_batch_size, 128);
        assert_eq!(profile_missing.active_prime_slots, 64);
        assert!(missing_path.exists());

        // 2. Corrupt file fallback (must not overwrite corrupt file)
        let corrupt_path = temp_dir.join("corrupt_profile.json");
        std::fs::write(&corrupt_path, "{ invalid json }").expect("Failed to write corrupt file");

        let profile_corrupt = load_profile_from_path(&corrupt_path);
        assert_eq!(profile_corrupt.pollard_rho_batch_size, 128);
        assert_eq!(profile_corrupt.active_prime_slots, 64);
        assert_eq!(
            std::fs::read_to_string(&corrupt_path).unwrap(),
            "{ invalid json }"
        );

        // 3. Valid profile loading
        let valid_path = temp_dir.join("valid_profile.json");
        let valid_json = r#"{
            "pollard_rho_batch_size": 512,
            "active_prime_slots": 32,
            "engine_telemetry_interval_ms": 1500,
            "dashboard_telemetry_interval_ms": 300
        }"#;
        std::fs::write(&valid_path, valid_json).expect("Failed to write valid file");

        let profile_valid = load_profile_from_path(&valid_path);
        assert_eq!(profile_valid.pollard_rho_batch_size, 512);
        assert_eq!(profile_valid.active_prime_slots, 32);
        assert_eq!(profile_valid.engine_telemetry_interval_ms, 1500);
        assert_eq!(profile_valid.dashboard_telemetry_interval_ms, 300);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_auto_instantiate_from_template() {
        let temp_dir = std::env::temp_dir().join(format!("ualbf_test_tmpl_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let template_path = temp_dir.join("profile.json.template");
        let template_content = r#"{
            "pollard_rho_batch_size": 256,
            "active_prime_slots": 128,
            "engine_telemetry_interval_ms": 2000,
            "dashboard_telemetry_interval_ms": 500
        }"#;
        std::fs::write(&template_path, template_content).unwrap();

        let target_path = temp_dir.join("profile.json");
        assert!(!target_path.exists());

        // Initial execution copies template to target_path
        let profile = load_profile_from_path(&target_path);
        assert!(target_path.exists());
        assert_eq!(profile.pollard_rho_batch_size, 256);
        assert_eq!(profile.active_prime_slots, 128);
        assert_eq!(
            std::fs::read_to_string(&target_path).unwrap(),
            template_content
        );

        // Subsequent execution loads existing target_path without modifying it
        let profile_second = load_profile_from_path(&target_path);
        assert_eq!(profile_second.pollard_rho_batch_size, 256);
        assert_eq!(
            std::fs::read_to_string(&target_path).unwrap(),
            template_content
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_auto_instantiate_missing_template() {
        let temp_dir =
            std::env::temp_dir().join(format!("ualbf_test_no_tmpl_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let target_path = temp_dir.join("auto_profile.json");
        assert!(!target_path.exists());

        let profile = load_profile_from_path(&target_path);
        assert!(target_path.exists());
        assert_eq!(profile.pollard_rho_batch_size, 128);
        assert_eq!(profile.active_prime_slots, 64);

        let file_content = std::fs::read_to_string(&target_path).unwrap();
        let expected_pretty = serde_json::to_string_pretty(&PerformanceProfile::default()).unwrap();
        assert_eq!(file_content, expected_pretty);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_auto_instantiate_unwritable_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("ualbf_test_unwritable_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let invalid_target = temp_dir.join("non_existent_subdir").join("profile.json");
        let profile = load_profile_from_path(&invalid_target);
        assert_eq!(profile.pollard_rho_batch_size, 128);
        assert_eq!(profile.active_prime_slots, 64);

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
