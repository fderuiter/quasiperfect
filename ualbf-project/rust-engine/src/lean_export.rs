// AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.

pub const EXPORTED_BOUNDS_MANIFEST_HASH: &str =
    "03621939f943cbd2642ccdcaa31840dbf415ebd424ded3d35012f277c0568c90";

use vstd::prelude::*;

verus! {
    pub open spec fn lean_prime_split_threshold() -> nat { 61 }
    pub open spec fn lean_prasad_sunitha_bound() -> nat { 15 }
    pub open spec fn lean_prasad_sunitha_combined() -> nat { 15 }
    pub open spec fn lean_div_5_coprime_3_bound() -> nat { 11 }
    pub open spec fn lean_div_5_coprime_3_combined() -> nat { 11 }
    pub open spec fn lean_hagis1982_combined() -> nat { 7 }
    pub open spec fn lean_qpn_totient_bound_num() -> nat { 20442 }
    pub open spec fn lean_qpn_totient_bound_den() -> nat { 10000 }
    pub open spec fn lean_target_min_log10() -> nat { 37 }
    pub open spec fn lean_target_max_log10() -> nat { 43 }
    pub open spec fn lean_sieve_limit() -> nat { 250000 }
    pub open spec fn lean_max_exponent() -> nat { 4 }
    pub open spec fn lean_prefix_stop_threshold() -> nat { 100000000000 }
    pub open spec fn lean_pollard_rho_iteration_limit() -> nat { 1000000 }
    pub open spec fn lean_pollard_rho_batch_size() -> nat { 128 }
    pub open spec fn lean_overflow_threshold_num() -> nat { 2000001 }
    pub open spec fn lean_overflow_threshold_den() -> nat { 1000000 }
    pub open spec fn lean_raycast_gpu_threshold() -> nat { 100000 }
    pub open spec fn lean_raycast_chunk_size() -> nat { 10000000 }
    pub open spec fn lean_conjectural_active() -> bool { false }
    pub open spec fn lean_conjectural_max_log10_ceiling() -> nat { 30 }
    pub open spec fn lean_crt_modulus_product() -> nat { 1155 }

    pub open spec fn lean_hagis1982_min_prime_factors() -> nat { 7 }
    pub open spec fn lean_hagis1982_offset() -> nat { 0 }
    pub open spec fn lean_prasad_sunitha_offset() -> nat { 0 }
    pub open spec fn lean_div_5_coprime_3_offset() -> nat { 0 }
    pub open spec fn lean_miller_rabin_20_base_sufficiency() -> bool { false }

    pub proof fn prove_prime_split_threshold_equivalence()
        ensures (crate::manifest_constants::PRIME_SPLIT_THRESHOLD as nat) == lean_prime_split_threshold()
    {}

    pub proof fn prove_prasad_sunitha_bound_equivalence()
        ensures (crate::manifest_constants::PRASAD_SUNITHA_PROOF_BOUND as nat) == lean_prasad_sunitha_bound()
    {}

    pub proof fn prove_prasad_sunitha_combined_equivalence()
        ensures (crate::manifest_constants::PRASAD_SUNITHA_BOUND_NO_3_5 as nat) == lean_prasad_sunitha_combined()
    {}

    pub proof fn prove_div_5_coprime_3_bound_equivalence()
        ensures (crate::manifest_constants::DIV_5_COPRIME_3_PROOF_BOUND as nat) == lean_div_5_coprime_3_bound()
    {}

    pub proof fn prove_div_5_coprime_3_combined_equivalence()
        ensures (crate::manifest_constants::DIV_5_COPRIME_3_BOUND as nat) == lean_div_5_coprime_3_combined()
    {}

    pub proof fn prove_baseline_min_prime_factors_equivalence()
        ensures (crate::manifest_constants::BASELINE_MIN_PRIME_FACTORS as nat) == lean_hagis1982_combined()
    {}

    pub proof fn prove_euler_ceiling_num_equivalence()
        ensures (crate::manifest_constants::EULER_CEILING_NUM as nat) == lean_qpn_totient_bound_num()
    {}

    pub proof fn prove_euler_ceiling_den_equivalence()
        ensures (crate::manifest_constants::EULER_CEILING_DEN as nat) == lean_qpn_totient_bound_den()
    {}

    pub proof fn prove_target_min_log10_equivalence()
        ensures (crate::manifest_constants::TARGET_MIN_LOG10 as nat) == lean_target_min_log10()
    {}

    pub proof fn prove_target_max_log10_equivalence()
        ensures (crate::manifest_constants::TARGET_MAX_LOG10 as nat) == lean_target_max_log10()
    {}

    pub proof fn prove_sieve_limit_equivalence()
        ensures (crate::manifest_constants::SIEVE_LIMIT as nat) == lean_sieve_limit()
    {}

    pub proof fn prove_max_exponent_equivalence()
        ensures (crate::manifest_constants::MAX_EXPONENT as nat) == lean_max_exponent()
    {}

    pub proof fn prove_prefix_stop_threshold_equivalence()
        ensures (crate::manifest_constants::PREFIX_STOP_THRESHOLD as nat) == lean_prefix_stop_threshold()
    {}

    pub proof fn prove_pollard_rho_iteration_limit_equivalence()
        ensures (crate::manifest_constants::POLLARD_RHO_ITERATION_LIMIT as nat) == lean_pollard_rho_iteration_limit()
    {}

    pub proof fn prove_pollard_rho_batch_size_equivalence()
        ensures (crate::manifest_constants::POLLARD_RHO_BATCH_SIZE as nat) == lean_pollard_rho_batch_size()
    {}

    pub proof fn prove_overflow_threshold_num_equivalence()
        ensures (crate::manifest_constants::OVERFLOW_THRESHOLD_NUM as nat) == lean_overflow_threshold_num()
    {}

    pub proof fn prove_overflow_threshold_den_equivalence()
        ensures (crate::manifest_constants::OVERFLOW_THRESHOLD_DEN as nat) == lean_overflow_threshold_den()
    {}

    pub proof fn prove_raycast_gpu_threshold_equivalence()
        ensures (crate::manifest_constants::RAYCAST_GPU_THRESHOLD as nat) == lean_raycast_gpu_threshold()
    {}

    pub proof fn prove_raycast_chunk_size_equivalence()
        ensures (crate::manifest_constants::RAYCAST_CHUNK_SIZE as nat) == lean_raycast_chunk_size()
    {}

    pub proof fn prove_conjectural_active_equivalence()
        ensures crate::manifest_constants::CONJECTURAL_ACTIVE == lean_conjectural_active()
    {}

    pub proof fn prove_conjectural_max_log10_ceiling_equivalence()
        ensures (crate::manifest_constants::CONJECTURAL_MAX_LOG10_CEILING as nat) == lean_conjectural_max_log10_ceiling()
    {}

    pub proof fn prove_crt_modulus_product_equivalence()
        ensures (crate::manifest_constants::CRT_MODULUS_PRODUCT as nat) == lean_crt_modulus_product()
    {}

    pub proof fn prove_combined_bounds() {
        assert(lean_hagis1982_combined() == lean_hagis1982_min_prime_factors() + lean_hagis1982_offset());
        assert(lean_prasad_sunitha_combined() == lean_prasad_sunitha_bound() + lean_prasad_sunitha_offset());
        assert(lean_div_5_coprime_3_combined() == lean_div_5_coprime_3_bound() + lean_div_5_coprime_3_offset());
    }
}
