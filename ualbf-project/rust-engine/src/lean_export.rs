// AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.

pub const EXPORTED_BOUNDS_MANIFEST_HASH: &str =
    "5bb23907e6f27af1a035947ecfddd57e3ccfedda1c0d14f4d42b3855224abf57";

use vstd::prelude::*;

verus! {
    pub open spec fn lean_qpn_totient_bound_num() -> nat { 20442 }
    pub open spec fn lean_qpn_totient_bound_den() -> nat { 10000 }

    pub open spec fn lean_hagis1982_min_prime_factors() -> nat { 7 }
    pub open spec fn lean_hagis1982_offset() -> nat { 0 }
    pub open spec fn lean_hagis1982_combined() -> nat { 7 }

    pub open spec fn lean_prasad_sunitha_bound() -> nat { 15 }
    pub open spec fn lean_prasad_sunitha_offset() -> nat { 0 }
    pub open spec fn lean_prasad_sunitha_combined() -> nat { 15 }

    pub open spec fn lean_div_5_coprime_3_bound() -> nat { 11 }
    pub open spec fn lean_div_5_coprime_3_offset() -> nat { 0 }
    pub open spec fn lean_div_5_coprime_3_combined() -> nat { 11 }

    pub open spec fn lean_miller_rabin_20_base_sufficiency() -> bool { true }

    pub open spec fn lean_conjectural_active() -> bool { true }
    pub open spec fn lean_conjectural_max_log10_ceiling() -> nat { 30 }

    pub proof fn prove_combined_bounds() {
        assert(lean_hagis1982_combined() == lean_hagis1982_min_prime_factors() + lean_hagis1982_offset());
        assert(lean_prasad_sunitha_combined() == lean_prasad_sunitha_bound() + lean_prasad_sunitha_offset());
        assert(lean_div_5_coprime_3_combined() == lean_div_5_coprime_3_bound() + lean_div_5_coprime_3_offset());
    }
}
