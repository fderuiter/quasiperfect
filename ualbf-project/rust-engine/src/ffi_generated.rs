// AUTO-GENERATED from Lean metadata. DO NOT EDIT.

pub const EXPORTED_SCHEMA_MANIFEST_HASH: &str =
    "dcf4d6074ba46690077ed7b21ec0a2f77b605f80374c4f43e90e2eab54b5ab08";

pub const LIMB_COUNT: usize = 8;

pub type U512Data = [u64; 8];

extern "C" {
    pub fn ualbf_verify_identity(
        n_l: *mut crate::lean_ffi::lean_object,
        x_l_abs: *mut crate::lean_ffi::lean_object,
        x_l_neg: u8,
        s_l: *mut crate::lean_ffi::lean_object,
    ) -> u8;
    pub fn ualbf_check_mod_8(q: u64) -> u8;
    pub fn ualbf_check_mod_3(p: u64, two_e: u32) -> u8;
    pub fn ualbf_check_mod_5(p: u64, two_e: u32) -> u8;
    pub fn ualbf_check_touchard(p: u64, two_e: u32) -> u8;
    pub fn ualbf_check_mod_9(p: u64, two_e: u32) -> u8;
    pub fn ualbf_compute_sigma(p: u64, pow: u64) -> *mut crate::lean_ffi::lean_object;
    pub fn ualbf_mod_inverse(
        a_obj: *mut crate::lean_ffi::lean_object,
        a_neg: u8,
        m_obj: *mut crate::lean_ffi::lean_object,
    ) -> *mut crate::lean_ffi::lean_object;
    pub fn ualbf_cyclotomic_eval_pub(
        d: u32,
        p: *mut crate::lean_ffi::lean_object,
    ) -> *mut crate::lean_ffi::lean_object;
    pub fn ualbf_static_suffix_bound_w0(k: u32) -> u64;
    pub fn ualbf_static_suffix_bound_w1(k: u32) -> u64;
    pub fn ualbf_dfs_loop(
        ctx: u64,
        w: *mut crate::lean_ffi::lean_object,
    ) -> *mut crate::lean_ffi::lean_object;
    pub fn ualbf_evaluate_baseline_min_ffi(
        contains_3: u8,
        contains_5: u8,
        skipped_3: u8,
        skipped_5: u8,
    ) -> u32;
    pub static ualbf_euler_ceiling_num: u64;
    pub static ualbf_euler_ceiling_den: u64;
    pub static ualbf_baseline_min_prime_factors: u64;
    pub static ualbf_prasad_sunitha_bound: u64;
    pub static ualbf_div_5_coprime_3_bound: u64;
    pub static ualbf_target_abundance_num: u64;
    pub static ualbf_target_abundance_den: u64;
    pub static ualbf_pollard_rho_iteration_limit: u32;
    pub static ualbf_pollard_rho_batch_size: u32;
    pub static ualbf_target_min_log10: u32;
    pub static ualbf_target_max_log10: u32;
    pub static ualbf_sieve_limit: u64;
    pub static ualbf_max_exponent: u32;
    pub static ualbf_prefix_stop_threshold: u64;
    pub static ualbf_raycast_gpu_threshold: u32;
    pub static ualbf_raycast_chunk_size: u32;
    pub fn ualbf_check_crt_1155(
        z_val: *mut crate::lean_ffi::lean_object,
        x_l_val: *mut crate::lean_ffi::lean_object,
    ) -> u8;
    pub static ualbf_logic_hash: *mut crate::lean_ffi::lean_object;
    pub static ualbf_conjectural_active: u32;
    pub static ualbf_conjectural_max_log10_ceiling: u32;
    pub fn ualbf_bloom_get_index(hash1: u64, hash2: u64, num_bits: u64, i: u32) -> u64;
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w0(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[0] }
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w1(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[1] }
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w2(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[2] }
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w3(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[3] }
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w4(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[4] }
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w5(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[5] }
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w6(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[6] }
}

#[no_mangle]
pub extern "C" fn rust_u512_get_w7(obj: *mut crate::lean_ffi::lean_object) -> u64 {
    unsafe { (*crate::lean_ffi::get_u512_ptr(obj))[7] }
}

#[no_mangle]
pub extern "C" fn rust_u512_mk(
    w0: u64,
    w1: u64,
    w2: u64,
    w3: u64,
    w4: u64,
    w5: u64,
    w6: u64,
    w7: u64,
) -> *mut crate::lean_ffi::lean_object {
    crate::lean_ffi::alloc_u512([w0, w1, w2, w3, w4, w5, w6, w7])
}
