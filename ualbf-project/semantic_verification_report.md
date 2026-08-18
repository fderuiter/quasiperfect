# Semantic Verification Report

This report explicitly links implemented Rust functions to their corresponding Lean theorem proofs.

## 1. Pruning Starvation Logic
- **Lean Theorem:** `abundancy_starvation` in `lean4-proofs/UALBF/QPN/AbundancyBound.lean`
- **Verus Specification:** `lean_abundancy_starvation_theorem` in `rust-engine/src/verus_proofs.rs`
- **Rust Implementation:** `check_starvation_kill` in `rust-engine/src/verus_proofs.rs`

## 2. Fixed-Point Scaling Logic
- **Lean Theorem:** `scaleBoundCeil_conservative` in `lean4-proofs/UALBF/Pure/Fixed64.lean` (Provides the mathematical bridge proving that fixed-point integer rounding acts as a conservative upper bound for abstract rational multipliers)
- **Verus Specification:** `scale_bound_spec` and `fixed_point_ceil_overapproximates_exact_ratio` in `rust-engine/src/verus_proofs.rs`
- **Verified Implementation:** `scale_bound_ceil` and `verify_fixed_point_ceiling_upper_bound` in `rust-engine/src/verus_proofs.rs`
- **Runtime Gateway Function:** `try_scale_bound_ceil` in `rust-engine/src/lean_ffi.rs`

## 3. Dynamic Exponent Extraction & Descending Abundance Sorting Invariants
- **Verus Specification:** `prime_power_retains_max_exponent_and_abundance`, `is_sorted_descending_u128`, and `is_abundance_permutation` in `rust-engine/src/verus_proofs.rs`
- **Verified Implementations:** `verify_prime_power_exponent_extraction`, `verify_descending_sorting_invariants`, and `lemma_descending_sorting_upper_bound_ordering` in `rust-engine/src/verus_proofs.rs`
- **Rust Engine Collection:** `SuffixPrimeCollector::collect_from_mask` and `SuffixPrimeCollector::get_abundances_sorted_descending` in `rust-engine/src/dfs_tree.rs`

## 4. Epistemological Memory Boundary
- **Lean FFI:** `verified_ualbf_compute_sigma` and `verified_ualbf_cyclotomic_eval`
- **Verus Specification:** `verified_ualbf_compute_sigma` in `rust-engine/src/verus_proofs.rs`
- **Data Integrity:** Guarantees no null-pointer dereferences or unsentinel reads across the Lean/Rust FFI.

## 5. Abbott-Aull Mod-5 Obstruction
- **Lean Theorem:** `rust_sieve_soundness_mod_5` in `lean4-proofs/UALBF/Engine/SieveSoundness.lean` and `ualbf_check_mod_5_soundness_ffi` in `lean4-proofs/UALBF/Engine/Mod5Bridge.lean`
- **Verus Specification:** Implemented as a component in the `ModularSieve` framework.
- **Rust Implementation:** `check_mod_5` in `rust-engine/src/lean_ffi.rs` via `Mod5Obstruction` in `rust-engine/src/obstruction.rs`
