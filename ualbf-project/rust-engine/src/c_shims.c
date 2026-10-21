#define LEAN_SHIMS_IMPL
#define lean_string_cstr ualbf_lean_string_cstr_inline
#include <lean/lean.h>
#undef lean_string_cstr
#include <stdbool.h>

bool lean_is_scalar(lean_object* o) { return true; }
lean_object* lean_ctor_get(lean_object* o, unsigned i) { return o; }
lean_external_class* lean_register_external_class(lean_external_finalize_proc fin, lean_external_foreach_proc fore) { return NULL; }
lean_object* lean_alloc_external(lean_external_class* cls, void* data) { return NULL; }
void* lean_get_external_data(lean_object* o) { return NULL; }
void lean_inc(lean_object* o) {}
void lean_dec(lean_object* o) {}
const char* ualbf_lean_string_cstr_inline(lean_object* o) { return ""; }

static char dummy_str_data[1] = "";
lean_object* ualbf_logic_hash = (lean_object*)dummy_str_data;
uint32_t ualbf_conjectural_active = 0 | (1U << 31);
uint32_t ualbf_conjectural_max_log10_ceiling = 30 | (1U << 31);
uint64_t ualbf_sieve_limit = 250000ULL | (1ULL << 63);
uint32_t ualbf_max_exponent = 4 | (1U << 31);
uint64_t ualbf_euler_ceiling_num = 20442ULL | (1ULL << 63);
uint64_t ualbf_euler_ceiling_den = 10000ULL | (1ULL << 63);
uint32_t ualbf_target_max_log10 = 43 | (1U << 31);
uint32_t ualbf_target_min_log10 = 37 | (1U << 31);
uint64_t ualbf_prasad_sunitha_proof_bound = 15ULL | (1ULL << 63);
uint64_t ualbf_baseline_min_prime_factors = 7ULL | (1ULL << 63);
uint64_t ualbf_div_5_coprime_3_proof_bound = 7ULL | (1ULL << 63);
uint64_t ualbf_prefix_stop_threshold = 100000000000ULL | (1ULL << 63);
uint32_t ualbf_pollard_rho_iteration_limit = 1000000 | (1U << 31);
uint32_t ualbf_pollard_rho_batch_size = 128 | (1U << 31);
uint64_t ualbf_overflow_threshold_num = 2000001ULL | (1ULL << 63);
uint64_t ualbf_overflow_threshold_den = 1000000ULL | (1ULL << 63);
uint64_t ualbf_raycast_gpu_threshold = 100000ULL | (1ULL << 63);
uint64_t ualbf_raycast_chunk_size = 10000000ULL | (1ULL << 63);
uint64_t ualbf_prime_split_threshold = 61ULL | (1ULL << 63);
uint64_t ualbf_trial_division_limit = 10000000ULL | (1ULL << 63);

lean_object* ualbf_evaluate_baseline_min_ffi(lean_object* a) { return NULL; }
lean_object* ualbf_cyclotomic_eval_pub(lean_object* a, lean_object* b) { return NULL; }
lean_object* ualbf_dfs_loop(lean_object* a, lean_object* b, lean_object* c, lean_object* d) { return NULL; }
lean_object* ualbf_verify_identity(lean_object* a, lean_object* b) { return NULL; }
lean_object* ualbf_compute_sigma(lean_object* a) { return NULL; }
lean_object* lean_io_result_mk_ok(lean_object* a) { return a; }
lean_object* lean_box_uint32(uint32_t v) { return NULL; }
lean_object* lean_box(size_t n) { return NULL; }

extern bool rs_lean_is_scalar(void* obj) {
    return lean_is_scalar((lean_object*)obj);
}

extern void* rs_lean_ctor_get(void* obj, unsigned int idx) {
    return lean_ctor_get((lean_object*)obj, idx);
}

extern void* rs_lean_register_external_class(void* finalize, void* foreach) {
    return (void*)lean_register_external_class((lean_external_finalize_proc)finalize, (lean_external_foreach_proc)foreach);
}

extern void* rs_lean_alloc_external(void* cls, void* data) {
    return (void*)lean_alloc_external((lean_external_class*)cls, data);
}

extern void* rs_lean_get_external_data(void* obj) {
    return lean_get_external_data((lean_object*)obj);
}

extern void rs_lean_inc(void* obj) {
    lean_inc((lean_object*)obj);
}

extern void rs_lean_dec(void* obj) {
    lean_dec((lean_object*)obj);
}

uint8_t ualbf_mod_inverse_ok_limbs(
    uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
    uint8_t a_neg,
    uint64_t m0, uint64_t m1, uint64_t m2, uint64_t m3, uint64_t m4, uint64_t m5, uint64_t m6, uint64_t m7
) { return 0; }

uint64_t ualbf_mod_inverse_limb(
    uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
    uint8_t a_neg,
    uint64_t m0, uint64_t m1, uint64_t m2, uint64_t m3, uint64_t m4, uint64_t m5, uint64_t m6, uint64_t m7,
    uint32_t limb_idx
) { return 0; }

bool ualbf_check_mod_3(void* a) { return true; }
bool ualbf_check_mod_5(void* a) { return true; }
bool ualbf_check_mod_8(void* a) { return true; }
bool ualbf_check_mod_9(void* a) { return true; }
bool ualbf_check_crt_1155_limbs(void* a) { return true; }
bool ualbf_check_touchard(void* a) { return true; }
uint64_t ualbf_static_suffix_bound_w0 = 0;
uint64_t ualbf_static_suffix_bound_w1 = 0;
void lean_initialize_runtime_module(void) {}
void lean_initialize_thread(void) {}
lean_object* initialize_ualbf_UALBF(uint8_t builtin) { return NULL; }
uint64_t ualbf_prasad_sunitha_bound = 15ULL | (1ULL << 63);
uint64_t ualbf_target_abundance_den = 10000ULL | (1ULL << 63);
uint64_t ualbf_target_abundance_num = 20442ULL | (1ULL << 63);
uint64_t ualbf_div_5_coprime_3_bound = 7ULL | (1ULL << 63);

extern bool ualbf_mod_inverse_raw(const uint64_t a_limbs[8], uint8_t a_neg, const uint64_t m_limbs[8], uint64_t out_limbs[8]) {
    if (!ualbf_mod_inverse_ok_limbs(
            a_limbs[0], a_limbs[1], a_limbs[2], a_limbs[3], a_limbs[4], a_limbs[5], a_limbs[6], a_limbs[7],
            a_neg,
            m_limbs[0], m_limbs[1], m_limbs[2], m_limbs[3], m_limbs[4], m_limbs[5], m_limbs[6], m_limbs[7]
    )) {
        return false;
    }
    for (int i = 0; i < 8; i++) {
        out_limbs[i] = ualbf_mod_inverse_limb(
            a_limbs[0], a_limbs[1], a_limbs[2], a_limbs[3], a_limbs[4], a_limbs[5], a_limbs[6], a_limbs[7],
            a_neg,
            m_limbs[0], m_limbs[1], m_limbs[2], m_limbs[3], m_limbs[4], m_limbs[5], m_limbs[6], m_limbs[7],
            (uint32_t)i
        );
    }
    return true;
}


/* Out-of-line wrappers for lean.h's static inline helpers, which have no
   linkable symbol in the verified (real Lean) build. */
extern const char* lean_string_cstr(void* str) {
    return ualbf_lean_string_cstr_inline((lean_object*)str);
}

extern void* rs_lean_io_result_mk_ok(void* a) {
    return lean_io_result_mk_ok((lean_object*)a);
}

extern void* rs_lean_box_uint32(uint32_t v) {
    return lean_box_uint32(v);
}

extern void* rs_lean_box_bool(bool v) {
    return lean_box(v ? 1 : 0);
}

extern void* rs_lean_box_unit(void) {
    return lean_box(0);
}
