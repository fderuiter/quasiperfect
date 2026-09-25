#[path = "../build.rs"]
mod build_script;

use build_script::*;
use std::collections::HashMap;

fn sample_valid_bounds_manifest_json() -> &'static str {
    r#"{
        "omega_bounds": {
            "prasad_sunitha": {
                "proof_bound": 15,
                "engine_justified_gap": 0,
                "is_axiomatic": false,
                "citation": null
            },
            "div_5_coprime_3": null,
            "hagis1982": {
                "proof_bound": 7,
                "engine_justified_gap": 0,
                "is_axiomatic": false,
                "citation": null
            }
        },
        "search_bounds": {
            "target_min_log10": { "value": 35, "is_axiomatic": false, "citation": null },
            "target_max_log10": { "value": 43, "is_axiomatic": false, "citation": null },
            "sieve_limit": { "value": 250000, "is_axiomatic": false, "citation": null },
            "max_exponent": { "value": 4, "is_axiomatic": false, "citation": null },
            "prefix_stop_threshold": { "value": 10000, "is_axiomatic": false, "citation": null },
            "pollard_rho": { "iteration_limit": 1000, "batch_size": 100, "is_axiomatic": false, "citation": null },
            "raycast": { "gpu_threshold": 1000, "chunk_size": 10000, "is_axiomatic": false },
            "prime_split_threshold": { "value": 61, "is_axiomatic": false, "citation": null }
        },
        "euler_ceiling": { "num": 20442, "den": 10000, "is_axiomatic": false, "citation": null },
        "overflow_threshold": { "num": 2, "den": 1, "is_axiomatic": false },
        "conjectural_bounds": null
    }"#
}

fn sample_valid_proof_manifest_json(bounds_hash: &str) -> String {
    let fns = [
        "check_starvation_kill",
        "check_cdg_forced_kill",
        "lean_abundancy_starvation_theorem",
        "verify_starvation_pruning",
        "is_starved",
        "is_cdg_forced_pruned",
        "lemma_sigma_multiplicative",
        "lemma_coprime_implies_multiplicative_nonlinear",
        "lemma_coprime_implies_multiplicative",
        "lemma_disjoint_by_construction",
        "prasad_sunitha_bound_satisfied",
        "verify_prasad_sunitha",
        "screen_mod_8",
        "is_valid_mod_8",
        "passes_raycast_sieve_spec",
        "verified_passes_raycast_sieve",
        "zsigmondy_preconditions_satisfied",
        "proof_verify_zsigmondy_preconditions",
        "lemma_composite_has_prime_factor_le_sqrt",
        "lemma_smallest_factor_is_prime",
        "lemma_modpow_mod_divisibility",
        "lemma_modpow_add_mul",
        "lemma_order_exists",
        "lemma_order_prime_factor",
        "lemma_divisibility_bounds",
        "lemma_fermat_little_theorem",
        "lemma_order_le_p_minus_1",
        "lemma_square_comparison_contradiction",
        "lemma_f_squared_gt_n_minus_1",
        "lemma_pocklington_certificate",
        "lemma_divisibility_transitive",
        "scale_bound_ceil",
    ];

    let mut thms_json = Vec::new();
    let mut bindings_json = Vec::new();

    for fn_name in fns {
        let thm_name = format!("{}_thm", fn_name);
        let hash = format!("hash_{}", fn_name);
        thms_json.push(format!(
            r#"{{ "name": "{}", "file": "UALBF/Engine/Bipartition.lean", "status": "proven", "checksum": "{}" }}"#,
            thm_name, hash
        ));
        bindings_json.push(format!(
            r#""{}": {{ "lean_theorem": "{}", "theorem_hash": "{}" }}"#,
            fn_name, thm_name, hash
        ));
    }

    format!(
        r#"{{
            "theorems": [ {} ],
            "verified_logic_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "verified_extension_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "verus_hashes": {{}},
            "ghost_pruning_bindings": {{ {} }},
            "proof_files": [],
            "bounds_manifest_hash": "{}"
        }}"#,
        thms_json.join(", "),
        bindings_json.join(", "),
        bounds_hash
    )
}

// =========================================================================
// BoundsManifest Unit Tests
// =========================================================================

#[test]
fn test_parse_bounds_manifest_valid() {
    let json = sample_valid_bounds_manifest_json();
    let manifest = parse_bounds_manifest(json).expect("Should parse valid bounds manifest");
    assert_eq!(manifest.search_bounds.target_min_log10.value, 35);
    assert_eq!(manifest.search_bounds.target_max_log10.value, 43);
    assert!(validate_bounds_manifest(&manifest).is_ok());
}

#[test]
fn test_parse_bounds_manifest_malformed() {
    let json = r#"{ "omega_bounds": { "invalid_json": true }"#;
    let err = parse_bounds_manifest(json).expect_err("Should fail on malformed JSON");
    assert!(err.contains("Failed to parse bounds_manifest.json"));
}

#[test]
fn test_validate_bounds_manifest_invalid_prime_split_threshold() {
    let json = sample_valid_bounds_manifest_json().replace("\"value\": 61", "\"value\": 60");
    let manifest = parse_bounds_manifest(&json).unwrap();
    let err =
        validate_bounds_manifest(&manifest).expect_err("Should fail when split threshold != 61");
    assert!(err.contains("configured prime split threshold (60) does not equal the baseline of 61"));
}

#[test]
fn test_validate_bounds_manifest_missing_prime_split_threshold() {
    let mut manifest = parse_bounds_manifest(sample_valid_bounds_manifest_json()).unwrap();
    manifest.search_bounds.prime_split_threshold = None;
    let err =
        validate_bounds_manifest(&manifest).expect_err("Should fail when split threshold is None");
    assert!(err.contains("prime_split_threshold not found"));
}

#[test]
fn test_validate_bounds_manifest_inverted_log10_range() {
    let mut manifest = parse_bounds_manifest(sample_valid_bounds_manifest_json()).unwrap();
    manifest.search_bounds.target_min_log10.value = 40;
    manifest.search_bounds.target_max_log10.value = 35;
    let err = validate_bounds_manifest(&manifest).expect_err("Should fail when min > max");
    assert!(err.contains("exceeds target_max_log10"));
}

#[test]
fn test_validate_bounds_manifest_conjectural_ceiling_conflict() {
    let mut manifest = parse_bounds_manifest(sample_valid_bounds_manifest_json()).unwrap();
    manifest.conjectural_bounds = Some(ConjecturalBounds {
        active: true,
        conjecture_name: "test_conjecture".to_string(),
        target_max_log10_ceiling: 30,
    });
    manifest.search_bounds.target_min_log10.value = 35;
    let err = validate_bounds_manifest(&manifest)
        .expect_err("Should fail when conjectural ceiling < min floor");
    assert!(err.contains("Conflicting bounds parameters detected"));
}

#[test]
fn test_validate_bounds_manifest_missing_citation_for_axiomatic() {
    let mut manifest = parse_bounds_manifest(sample_valid_bounds_manifest_json()).unwrap();
    manifest.omega_bounds.hagis1982.is_axiomatic = true;
    manifest.omega_bounds.hagis1982.citation = None;
    let err = validate_bounds_manifest(&manifest)
        .expect_err("Should fail when axiomatic bound lacks citation");
    assert!(err.contains("baseline bound marked axiomatic but lacks citation metadata"));
}

#[test]
fn test_validate_bounds_manifest_target_min_log10_axiomatic() {
    let mut manifest = parse_bounds_manifest(sample_valid_bounds_manifest_json()).unwrap();
    manifest.search_bounds.target_min_log10.is_axiomatic = true;
    let err = validate_bounds_manifest(&manifest)
        .expect_err("Should fail when target_min_log10 is axiomatic");
    assert!(
        err.contains("search engine floor (target_min_log10) cannot rely on axiomatic assumptions")
    );
}

// =========================================================================
// ProofManifest Unit Tests
// =========================================================================

#[test]
fn test_parse_proof_manifest_valid() {
    let json = sample_valid_proof_manifest_json("test_bounds_hash");
    let manifest = parse_proof_manifest(&json).expect("Should parse valid proof manifest");
    assert_eq!(manifest.bounds_manifest_hash, "test_bounds_hash");
    assert!(validate_proof_manifest(&manifest, "test_bounds_hash").is_ok());
}

#[test]
fn test_parse_proof_manifest_malformed() {
    let json = r#"{ "theorems": "not_an_array" }"#;
    let err = parse_proof_manifest(json).expect_err("Should fail on malformed proof manifest");
    assert!(err.contains("Failed to parse proof_manifest.json"));
}

#[test]
fn test_validate_proof_manifest_bounds_hash_mismatch() {
    let manifest = parse_proof_manifest(&sample_valid_proof_manifest_json("hash_A")).unwrap();
    let err = validate_proof_manifest(&manifest, "hash_B")
        .expect_err("Should fail on bounds hash mismatch");
    assert!(err.contains("Configuration mismatch"));
}

#[test]
fn test_validate_proof_manifest_unproven_theorem() {
    let json = sample_valid_proof_manifest_json("hash_A")
        .replace("\"status\": \"proven\"", "\"status\": \"unproven\"");
    let manifest = parse_proof_manifest(&json).unwrap();
    let err =
        validate_proof_manifest(&manifest, "hash_A").expect_err("Should fail on unproven theorem");
    assert!(err.contains("is incomplete (status: unproven)"));
}

#[test]
fn test_validate_proof_manifest_whitelisted_axiom() {
    let mut manifest = parse_proof_manifest(&sample_valid_proof_manifest_json("hash_A")).unwrap();
    manifest.theorems.push(Theorem {
        name: "UALBF.QPN.PrasadSunitha.qpn_div_5_coprime_3_omega_bound".to_string(),
        file: "UALBF/QPN/PrasadSunitha.lean".to_string(),
        status: "axiom".to_string(),
        checksum: "hash_axiom".to_string(),
    });
    assert!(validate_proof_manifest(&manifest, "hash_A").is_ok());
}

#[test]
fn test_validate_proof_manifest_non_whitelisted_axiom() {
    let mut manifest = parse_proof_manifest(&sample_valid_proof_manifest_json("hash_A")).unwrap();
    manifest.theorems.push(Theorem {
        name: "unauthorized_axiom".to_string(),
        file: "UALBF/Bad.lean".to_string(),
        status: "axiom".to_string(),
        checksum: "hash_bad".to_string(),
    });
    let err = validate_proof_manifest(&manifest, "hash_A")
        .expect_err("Should fail on non-whitelisted axiom");
    assert!(err.contains(
        "Theorem 'unauthorized_axiom' in 'UALBF/Bad.lean' is incomplete (status: axiom)"
    ));
}

#[test]
fn test_validate_proof_manifest_missing_ghost_bindings() {
    let mut manifest = parse_proof_manifest(&sample_valid_proof_manifest_json("hash_A")).unwrap();
    manifest.ghost_pruning_bindings = None;
    let err = validate_proof_manifest(&manifest, "hash_A")
        .expect_err("Should fail when ghost_pruning_bindings is None");
    assert!(err.contains("missing required 'ghost_pruning_bindings' field"));
}

#[test]
fn test_validate_proof_manifest_missing_required_ghost_fn() {
    let mut manifest = parse_proof_manifest(&sample_valid_proof_manifest_json("hash_A")).unwrap();
    if let Some(ref mut bindings) = manifest.ghost_pruning_bindings {
        bindings.remove("lemma_disjoint_by_construction");
    }
    let err = validate_proof_manifest(&manifest, "hash_A")
        .expect_err("Should fail when required ghost binding is missing");
    assert!(err.contains("lacks a matching Lean 4 manifest entry in ghost_pruning_bindings"));
}

#[test]
fn test_validate_proof_manifest_unknown_lean_theorem() {
    let mut manifest = parse_proof_manifest(&sample_valid_proof_manifest_json("hash_A")).unwrap();
    if let Some(ref mut bindings) = manifest.ghost_pruning_bindings {
        bindings.insert(
            "check_starvation_kill".to_string(),
            GhostBinding {
                lean_theorem: "non_existent_thm".to_string(),
                theorem_hash: "hash_starvation".to_string(),
            },
        );
    }
    let err = validate_proof_manifest(&manifest, "hash_A")
        .expect_err("Should fail when binding references missing theorem");
    assert!(err.contains("references unknown Lean theorem 'non_existent_thm'"));
}

#[test]
fn test_validate_proof_manifest_ghost_binding_hash_mismatch() {
    let mut manifest = parse_proof_manifest(&sample_valid_proof_manifest_json("hash_A")).unwrap();
    if let Some(ref mut bindings) = manifest.ghost_pruning_bindings {
        bindings.insert(
            "check_starvation_kill".to_string(),
            GhostBinding {
                lean_theorem: "check_starvation_kill_thm".to_string(),
                theorem_hash: "wrong_hash".to_string(),
            },
        );
    }
    let err = validate_proof_manifest(&manifest, "hash_A")
        .expect_err("Should fail on ghost binding hash mismatch");
    assert!(err.contains("SHA-256 hash mismatch for Lean theorem 'check_starvation_kill_thm'"));
}

// =========================================================================
// Source Cleaning, Brace Counting & Verus Hash Computation Unit Tests
// =========================================================================

#[test]
fn test_clean_source_comments_and_strings() {
    let code = r#"
        // This is a line comment with { braces }
        /* This is a block
           comment with { braces } */
        let str = "string with { braces } // and comment markers";
        let ch = '{';
        pub fn foo() {}
    "#;
    let cleaned = clean_source(code);
    assert!(!cleaned.contains("This is a line comment"));
    assert!(!cleaned.contains("This is a block"));
    assert!(cleaned.contains(r#""string with { braces } // and comment markers""#));
    assert!(cleaned.contains("'{'"));
    assert!(cleaned.contains("pub fn foo() {}"));
}

#[test]
fn test_count_non_literal_braces() {
    let line1 = r#"let s = "{ }"; let c = '{'; if true { return; }"#;
    let (open, close) = count_non_literal_braces(line1);
    assert_eq!(open, 1);
    assert_eq!(close, 1);

    let line2 = r#"pub open spec fn test_fn(x: u32) -> bool {"#;
    let (open2, close2) = count_non_literal_braces(line2);
    assert_eq!(open2, 1);
    assert_eq!(close2, 0);
}

#[test]
fn test_compute_verus_hashes_single_line() {
    let code = "pub spec fn test_spec() {}\n";
    let hashes = compute_verus_hashes(code);
    assert_eq!(hashes.len(), 1);
    assert!(hashes.contains_key("test_spec"));
}

#[test]
fn test_compute_verus_hashes_multi_line() {
    let code = r#"
        pub open spec fn multi_line_spec(a: u64, b: u64) -> bool {
            let sum = a + b;
            sum > 100
        }
    "#;
    let hashes = compute_verus_hashes(code);
    assert_eq!(hashes.len(), 1);
    assert!(hashes.contains_key("multi_line_spec"));
}

#[test]
fn test_compute_verus_hashes_nested_modules() {
    let code = r#"
        pub mod outer {
            mod inner {
                pub fn my_func() {
                    let x = 1;
                }
            }
        }
    "#;
    let hashes = compute_verus_hashes(code);
    assert_eq!(hashes.len(), 1);
    assert!(hashes.contains_key("outer::inner::my_func"));
}

#[test]
fn test_compute_verus_hashes_comments_and_string_braces() {
    let code = r#"
        // Comment with pub fn fake_fn() {
        pub fn real_fn() {
            let msg = "Hello {world}"; // {
        }
    "#;
    let hashes = compute_verus_hashes(code);
    assert_eq!(hashes.len(), 1);
    assert!(hashes.contains_key("real_fn"));
    assert!(!hashes.contains_key("fake_fn"));
}

// =========================================================================
// Constant & Spec Equivalence and Hash Synchronization Tests
// =========================================================================

#[test]
fn test_validate_constant_spec_equivalence_success() {
    let constants = r#"
        pub const PRIME_SPLIT_THRESHOLD: u64 = 61;
        pub const PRASAD_SUNITHA_PROOF_BOUND: u64 = 15;
        pub const PRASAD_SUNITHA_BOUND_NO_3_5: u64 = 15;
        pub const DIV_5_COPRIME_3_PROOF_BOUND: u64 = 11;
        pub const DIV_5_COPRIME_3_BOUND: u64 = 11;
        pub const BASELINE_MIN_PRIME_FACTORS: u64 = 7;
        pub const EULER_CEILING_NUM: u64 = 20442;
        pub const EULER_CEILING_DEN: u64 = 10000;
        pub const TARGET_MIN_LOG10: u32 = 35;
        pub const TARGET_MAX_LOG10: u32 = 37;
        pub const SIEVE_LIMIT: usize = 250000;
        pub const MAX_EXPONENT: u32 = 4;
        pub const PREFIX_STOP_THRESHOLD: u64 = 10000;
        pub const POLLARD_RHO_ITERATION_LIMIT: u32 = 1000;
        pub const POLLARD_RHO_BATCH_SIZE: u32 = 100;
        pub const OVERFLOW_THRESHOLD_NUM: u64 = 2;
        pub const OVERFLOW_THRESHOLD_DEN: u64 = 1;
        pub const RAYCAST_GPU_THRESHOLD: usize = 1000;
        pub const RAYCAST_CHUNK_SIZE: usize = 10000;
        pub const CONJECTURAL_ACTIVE: bool = false;
        pub const CONJECTURAL_MAX_LOG10_CEILING: u32 = 37;
    "#;

    let specs = r#"
        pub open spec fn lean_prime_split_threshold() -> u64 { 61 }
        pub open spec fn lean_prasad_sunitha_bound() -> u64 { 15 }
        pub open spec fn lean_prasad_sunitha_combined() -> u64 { 15 }
        pub open spec fn lean_div_5_coprime_3_bound() -> u64 { 11 }
        pub open spec fn lean_div_5_coprime_3_combined() -> u64 { 11 }
        pub open spec fn lean_hagis1982_combined() -> u64 { 7 }
        pub open spec fn lean_qpn_totient_bound_num() -> u64 { 20442 }
        pub open spec fn lean_qpn_totient_bound_den() -> u64 { 10000 }
        pub open spec fn lean_target_min_log10() -> u32 { 35 }
        pub open spec fn lean_target_max_log10() -> u32 { 37 }
        pub open spec fn lean_sieve_limit() -> usize { 250000 }
        pub open spec fn lean_max_exponent() -> u32 { 4 }
        pub open spec fn lean_prefix_stop_threshold() -> u64 { 10000 }
        pub open spec fn lean_pollard_rho_iteration_limit() -> u32 { 1000 }
        pub open spec fn lean_pollard_rho_batch_size() -> u32 { 100 }
        pub open spec fn lean_overflow_threshold_num() -> u64 { 2 }
        pub open spec fn lean_overflow_threshold_den() -> u64 { 1 }
        pub open spec fn lean_raycast_gpu_threshold() -> usize { 1000 }
        pub open spec fn lean_raycast_chunk_size() -> usize { 10000 }
        pub open spec fn lean_conjectural_active() -> bool { false }
        pub open spec fn lean_conjectural_max_log10_ceiling() -> u32 { 37 }
    "#;

    assert!(validate_constant_spec_equivalence(constants, specs).is_ok());
}

#[test]
fn test_validate_constant_spec_equivalence_mismatch() {
    let constants = r#"
        pub const PRIME_SPLIT_THRESHOLD: u64 = 60;
    "#;
    let specs = r#"
        pub open spec fn lean_prime_split_threshold() -> u64 { 61 }
    "#;
    let err = validate_constant_spec_equivalence(constants, specs)
        .expect_err("Should fail when split threshold constant != spec");
    assert!(err.contains(
        "PRIME_SPLIT_THRESHOLD' (60) does not equal the baseline Lean split threshold (61)"
    ));
}

#[test]
fn test_validate_verus_hashes() {
    let mut map_a = HashMap::new();
    map_a.insert("fn1".to_string(), "hash1".to_string());

    let mut map_b = HashMap::new();
    map_b.insert("fn1".to_string(), "hash1".to_string());

    assert!(validate_verus_hashes(&map_a, &map_b).is_ok());

    map_b.insert("fn1".to_string(), "hash2".to_string());
    assert!(validate_verus_hashes(&map_a, &map_b).is_err());
}
