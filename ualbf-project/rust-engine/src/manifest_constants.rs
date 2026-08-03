// AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.
#[cfg(not(verus_keep_ghost))]
pub const PRIME_SPLIT_THRESHOLD: u64 = 61;
#[cfg(not(verus_keep_ghost))]
pub const PRASAD_SUNITHA_PROOF_BOUND: u64 = 15;
#[cfg(not(verus_keep_ghost))]
pub const PRASAD_SUNITHA_BOUND_NO_3_5: u64 = 15;
#[cfg(not(verus_keep_ghost))]
pub const DIV_5_COPRIME_3_PROOF_BOUND: u64 = 11;
#[cfg(not(verus_keep_ghost))]
pub const DIV_5_COPRIME_3_BOUND: u64 = 11;
#[cfg(not(verus_keep_ghost))]
pub const BASELINE_MIN_PRIME_FACTORS: u64 = 7;
#[cfg(not(verus_keep_ghost))]
pub const EULER_CEILING_NUM: u64 = 20442;
#[cfg(not(verus_keep_ghost))]
pub const EULER_CEILING_DEN: u64 = 10000;
#[cfg(not(verus_keep_ghost))]
pub const TARGET_MIN_LOG10: u32 = 37;
#[cfg(not(verus_keep_ghost))]
pub const TARGET_MAX_LOG10: u32 = 43;
#[cfg(not(verus_keep_ghost))]
pub const SIEVE_LIMIT: usize = 250000;
#[cfg(not(verus_keep_ghost))]
pub const MAX_EXPONENT: u32 = 4;
#[cfg(not(verus_keep_ghost))]
pub const PREFIX_STOP_THRESHOLD: u64 = 100000000000;
#[cfg(not(verus_keep_ghost))]
pub const POLLARD_RHO_ITERATION_LIMIT: u32 = 1000000;
#[cfg(not(verus_keep_ghost))]
pub const POLLARD_RHO_BATCH_SIZE: u32 = 128;
#[cfg(not(verus_keep_ghost))]
pub const OVERFLOW_THRESHOLD_NUM: u64 = 2000001;
#[cfg(not(verus_keep_ghost))]
pub const OVERFLOW_THRESHOLD_DEN: u64 = 1000000;
#[cfg(not(verus_keep_ghost))]
pub const RAYCAST_GPU_THRESHOLD: usize = 100000;
#[cfg(not(verus_keep_ghost))]
pub const RAYCAST_CHUNK_SIZE: usize = 10000000;
#[cfg(not(verus_keep_ghost))]
pub const CONJECTURAL_ACTIVE: bool = false;
#[cfg(not(verus_keep_ghost))]
pub const CONJECTURE_NAME: &str = "ABC Conjecture";
#[cfg(not(verus_keep_ghost))]
pub const CONJECTURAL_MAX_LOG10_CEILING: u32 = 30;
#[cfg(not(verus_keep_ghost))]
pub const TOUCHARD_MOD_24_MODULUS: u32 = 24;
#[cfg(not(verus_keep_ghost))]
pub const TOUCHARD_MOD_24_RESIDUES: [u32; 2] = [3, 19];
#[cfg(not(verus_keep_ghost))]
pub const CRT_MODULUS_PRODUCT: u32 = 1155;
#[cfg(not(verus_keep_ghost))]
pub const CRT_MODULI: [u32; 4] = [3, 5, 7, 11];
#[cfg(not(verus_keep_ghost))]
pub const MANIFEST_HASH: &str = "03621939f943cbd2642ccdcaa31840dbf415ebd424ded3d35012f277c0568c90";

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {
    pub const PRIME_SPLIT_THRESHOLD: u64 = 61;
    pub const PRASAD_SUNITHA_PROOF_BOUND: u64 = 15;
    pub const PRASAD_SUNITHA_BOUND_NO_3_5: u64 = 15;
    pub const DIV_5_COPRIME_3_PROOF_BOUND: u64 = 11;
    pub const DIV_5_COPRIME_3_BOUND: u64 = 11;
    pub const BASELINE_MIN_PRIME_FACTORS: u64 = 7;
    pub const EULER_CEILING_NUM: u64 = 20442;
    pub const EULER_CEILING_DEN: u64 = 10000;
    pub const TARGET_MIN_LOG10: u32 = 37;
    pub const TARGET_MAX_LOG10: u32 = 43;
    pub const SIEVE_LIMIT: usize = 250000;
    pub const MAX_EXPONENT: u32 = 4;
    pub const PREFIX_STOP_THRESHOLD: u64 = 100000000000;
    pub const POLLARD_RHO_ITERATION_LIMIT: u32 = 1000000;
    pub const POLLARD_RHO_BATCH_SIZE: u32 = 128;
    pub const OVERFLOW_THRESHOLD_NUM: u64 = 2000001;
    pub const OVERFLOW_THRESHOLD_DEN: u64 = 1000000;
    pub const RAYCAST_GPU_THRESHOLD: usize = 100000;
    pub const RAYCAST_CHUNK_SIZE: usize = 10000000;
    pub const CONJECTURAL_ACTIVE: bool = false;
    pub const CONJECTURAL_MAX_LOG10_CEILING: u32 = 30;
    pub const TOUCHARD_MOD_24_MODULUS: u32 = 24;
    pub const CRT_MODULUS_PRODUCT: u32 = 1155;
    pub const MANIFEST_HASH: &'static str = "03621939f943cbd2642ccdcaa31840dbf415ebd424ded3d35012f277c0568c90";
}
