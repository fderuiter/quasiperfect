// AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.

pub const EXPORTED_BOUNDS_MANIFEST_HASH: &str =
    "b7d17beb40c782e4ed72be2172e36aeb26d0ee1c1ae780742948c2705467ea08";

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

    pub open spec fn lean_miller_rabin_20_base_sufficiency() -> bool { true }

    pub open spec fn lean_conjectural_active() -> bool { true }
    pub open spec fn lean_conjectural_max_log10_ceiling() -> nat { 30 }

    pub proof fn prove_combined_bounds() {
        assert(lean_hagis1982_combined() == lean_hagis1982_min_prime_factors() + lean_hagis1982_offset());
        assert(lean_prasad_sunitha_combined() == lean_prasad_sunitha_bound() + lean_prasad_sunitha_offset());
    }
}
