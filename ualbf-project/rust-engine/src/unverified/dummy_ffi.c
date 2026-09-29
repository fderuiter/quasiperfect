#include "../manifest_constants.h"
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

void lean_initialize_runtime_module() {}
void lean_initialize() {}
void initialize_Ualbf_C_Main() {}
void lean_initialize_thread() {}

typedef enum {
    DUMMY_OBJ_CTOR = 0,
    DUMMY_OBJ_EXTERNAL = 1
} dummy_obj_type;

typedef struct dummy_external_class {
    void (*finalize)(void*);
    void (*foreach)(void*, void*);
} dummy_external_class;

typedef struct dummy_header {
    uint32_t rc;                  // Reference count
    uint8_t  obj_type;            // DUMMY_OBJ_CTOR or DUMMY_OBJ_EXTERNAL
    uint8_t  num_fields;          // Number of fields in constructor
    uint16_t reserved;            // Alignment padding
    dummy_external_class* cls;    // Class descriptor for external object
    void* external_data;          // External data payload
} dummy_header;

void* lean_register_external_class(void* finalize, void* foreach) {
    dummy_external_class* cls = (dummy_external_class*)malloc(sizeof(dummy_external_class));
    if (!cls) return NULL;
    cls->finalize = (void (*)(void*))finalize;
    cls->foreach = (void (*)(void*, void*))foreach;
    return cls;
}

void* rs_lean_alloc_external(void* cls, void* data) {
    dummy_header* header = (dummy_header*)malloc(sizeof(dummy_header));
    if (!header) return NULL;
    header->rc = 1;
    header->obj_type = DUMMY_OBJ_EXTERNAL;
    header->num_fields = 0;
    header->reserved = 0;
    header->cls = (dummy_external_class*)cls;
    header->external_data = data;
    return (void*)header;
}

void* rs_lean_get_external_data(void* obj) {
    if (!obj || ((uintptr_t)obj & 1) == 1) return NULL;
    dummy_header* header = (dummy_header*)obj;
    if (header->obj_type == DUMMY_OBJ_EXTERNAL) {
        return header->external_data;
    }
    return NULL;
}

bool rs_lean_is_scalar(void* obj) {
    return ((uintptr_t)obj & 1) == 1;
}

void rs_lean_inc(void* obj) {
    if (!obj || ((uintptr_t)obj & 1) == 1) return;
    dummy_header* header = (dummy_header*)obj;
    header->rc++;
}

void rs_lean_dec(void* obj) {
    if (!obj || ((uintptr_t)obj & 1) == 1) return;
    dummy_header* header = (dummy_header*)obj;
    if (header->rc > 0) {
        header->rc--;
        if (header->rc == 0) {
            if (header->obj_type == DUMMY_OBJ_CTOR) {
                void** fields = (void**)(header + 1);
                for (uint8_t i = 0; i < header->num_fields; i++) {
                    rs_lean_dec(fields[i]);
                }
            } else if (header->obj_type == DUMMY_OBJ_EXTERNAL) {
                if (header->cls && header->cls->finalize) {
                    header->cls->finalize(header->external_data);
                } else if (header->external_data) {
                    free(header->external_data);
                }
            }
            free(header);
        }
    }
}

static void* alloc_dummy_ctor(uint8_t num_fields) {
    size_t sz = sizeof(dummy_header) + num_fields * sizeof(void*);
    dummy_header* header = (dummy_header*)malloc(sz);
    if (!header) return NULL;
    header->rc = 1;
    header->obj_type = DUMMY_OBJ_CTOR;
    header->num_fields = num_fields;
    header->reserved = 0;
    header->cls = NULL;
    header->external_data = NULL;
    void** fields = (void**)(header + 1);
    for (uint8_t i = 0; i < num_fields; i++) {
        fields[i] = NULL;
    }
    return (void*)header;
}

void* rs_lean_ctor_get(void* obj, unsigned int idx) {
    if (!obj || ((uintptr_t)obj & 1) == 1) return NULL;
    dummy_header* header = (dummy_header*)obj;
    if (header->obj_type == DUMMY_OBJ_CTOR && idx < header->num_fields) {
        void** fields = (void**)(header + 1);
        return fields[idx];
    }
    return NULL;
}

void* make_some(void* val) {
    dummy_header* obj = (dummy_header*)alloc_dummy_ctor(1);
    if (!obj) return NULL;
    void** fields = (void**)(obj + 1);
    fields[0] = val;
    return obj;
}

void* rs_lean_io_result_mk_ok(void* a) {
    dummy_header* obj = (dummy_header*)alloc_dummy_ctor(2);
    if (!obj) return NULL;
    void** fields = (void**)(obj + 1);
    fields[0] = a;
    fields[1] = (void*)1; // scalar unit
    return obj;
}

void* rs_lean_box_uint32(uint32_t v) {
    return (void*)((uintptr_t)v << 1 | 1);
}

void* rs_lean_box_bool(bool v) {
    return (void*)((uintptr_t)(v ? 1 : 0) << 1 | 1);
}

void* rs_lean_box_unit() {
    return (void*)1;
}

void* initialize_ualbf_UALBF(uint8_t builtin) { (void)builtin; return 0; }

uint8_t ualbf_check_mod_8(uint64_t q) { uint64_t r = q % 8; return (r == 1 || r == 3) ? 1 : 0; }

uint8_t ualbf_check_mod_3(uint64_t p, uint32_t two_e) {
    uint64_t p_mod = p % 3;
    uint64_t sum = 0;
    uint64_t term = 1;
    for (uint32_t i = 0; i <= two_e; i++) {
        sum = (sum + term) % 3;
        term = (term * p_mod) % 3;
    }
    return sum == 0 ? 1 : 0;
}

uint8_t ualbf_check_mod_5(uint64_t p, uint32_t two_e) {
    uint32_t e = two_e / 2;
    return (p % 5 == 1 && e % 5 == 2) ? 1 : 0;
}

uint8_t ualbf_check_mod_9(uint64_t p, uint32_t two_e) {
    uint64_t p_mod = p % 9;
    uint64_t sum = 0;
    uint64_t term = 1;
    for (uint32_t i = 0; i <= two_e; i++) {
        sum = (sum + term) % 9;
        term = (term * p_mod) % 9;
    }
    return (sum % 3 == 0) ? 1 : 0;
}

uint8_t ualbf_check_touchard(uint64_t p, uint32_t two_e) {
    return (two_e % 2 == 1 && p % 2 == 1) ? 1 : 0;
}

void* ualbf_compute_sigma(uint64_t p, uint64_t pow) {
    uint64_t* u512_data = (uint64_t*)malloc(64);
    if (!u512_data) return NULL;
    memset(u512_data, 0, 64);
    uint64_t sum = 1;
    uint64_t term = 1;
    for (uint32_t i = 1; i <= pow; i++) {
        term *= p;
        sum += term;
    }
    u512_data[0] = sum;
    void* u512_obj = rs_lean_alloc_external(NULL, u512_data);
    if (!u512_obj) {
        free(u512_data);
        return NULL;
    }
    return make_some(u512_obj);
}

void* ualbf_cyclotomic_eval_pub(uint32_t d, void* p) {
    (void)d; (void)p;
    uint64_t* u512_data = (uint64_t*)malloc(64);
    if (!u512_data) return NULL;
    memset(u512_data, 0, 64);
    u512_data[0] = 1;
    void* u512_obj = rs_lean_alloc_external(NULL, u512_data);
    if (!u512_obj) {
        free(u512_data);
        return NULL;
    }
    return make_some(u512_obj);
}

void* ualbf_mod_inverse(void* a_obj, uint8_t a_neg, void* m_obj) {
    if (!a_obj || !m_obj) return NULL;
    uint64_t* a_data = (uint64_t*)rs_lean_get_external_data(a_obj);
    uint64_t* m_data = (uint64_t*)rs_lean_get_external_data(m_obj);
    if (!a_data || !m_data) return NULL;

    bool a_fits = true, m_fits = true;
    for (int i = 1; i < 8; i++) {
        if (a_data[i] != 0) a_fits = false;
        if (m_data[i] != 0) m_fits = false;
    }

    if (a_fits && m_fits && m_data[0] > 1) {
        int64_t m_val = (int64_t)m_data[0];
        int64_t a_val = (int64_t)a_data[0];
        if (a_neg) a_val = -a_val;
        a_val = ((a_val % m_val) + m_val) % m_val;
        if (a_val == 0) return NULL;

        int64_t t = 0, new_t = 1;
        int64_t r = m_val, new_r = a_val;
        while (new_r != 0) {
            int64_t q = r / new_r;
            int64_t tmp_t = t - q * new_t;
            t = new_t;
            new_t = tmp_t;
            int64_t tmp_r = r - q * new_r;
            r = new_r;
            new_r = tmp_r;
        }
        if (r > 1) return NULL;
        if (t < 0) t += m_val;

        uint64_t* u512_data = (uint64_t*)malloc(64);
        if (!u512_data) return NULL;
        memset(u512_data, 0, 64);
        u512_data[0] = (uint64_t)t;
        void* u512_obj = rs_lean_alloc_external(NULL, u512_data);
        if (!u512_obj) {
            free(u512_data);
            return NULL;
        }
        return make_some(u512_obj);
    }

    return NULL;
}

uint8_t ualbf_mod_inverse_ok_limbs(
    uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
    uint8_t a_neg,
    uint64_t m0, uint64_t m1, uint64_t m2, uint64_t m3, uint64_t m4, uint64_t m5, uint64_t m6, uint64_t m7
) {
    (void)a0; (void)a1; (void)a2; (void)a3; (void)a4; (void)a5; (void)a6; (void)a7;
    (void)a_neg;
    (void)m0; (void)m1; (void)m2; (void)m3; (void)m4; (void)m5; (void)m6; (void)m7;
    return 0;
}

uint64_t ualbf_mod_inverse_limb(
    uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
    uint8_t a_neg,
    uint64_t m0, uint64_t m1, uint64_t m2, uint64_t m3, uint64_t m4, uint64_t m5, uint64_t m6, uint64_t m7,
    uint32_t limb_idx
) {
    (void)a0; (void)a1; (void)a2; (void)a3; (void)a4; (void)a5; (void)a6; (void)a7;
    (void)a_neg;
    (void)m0; (void)m1; (void)m2; (void)m3; (void)m4; (void)m5; (void)m6; (void)m7;
    (void)limb_idx;
    return 0;
}

bool ualbf_mod_inverse_raw(const uint64_t a_limbs[8], uint8_t a_neg, const uint64_t m_limbs[8], uint64_t out_limbs[8]) {
    (void)a_limbs; (void)a_neg; (void)m_limbs; (void)out_limbs;
    return false;
}
uint8_t ualbf_verify_identity(void* n_l, void* x_l_abs, uint8_t x_l_neg, void* s_l) { (void)n_l; (void)x_l_abs; (void)x_l_neg; (void)s_l; return 1; }

uint8_t ualbf_check_crt_1155(void* z_val, void* x_l_val) {
    uint64_t* z_data = (uint64_t*)rs_lean_get_external_data(z_val);
    uint64_t* x_l_data = (uint64_t*)rs_lean_get_external_data(x_l_val);
    if (!z_data || !x_l_data) return 1;
    uint64_t z = z_data[0];
    uint64_t xl = x_l_data[0];
    uint64_t z2 = z * z;
    if (z2 % 3 != xl % 3) return 0;
    if (z2 % 5 != xl % 5) return 0;
    if (z2 % 7 != xl % 7) return 0;
    if (z2 % 11 != xl % 11) return 0;
    return 1;
}

uint8_t ualbf_check_crt_1155_limbs(
    uint64_t z0, uint64_t z1, uint64_t z2, uint64_t z3, uint64_t z4, uint64_t z5, uint64_t z6, uint64_t z7,
    uint64_t xl0, uint64_t xl1, uint64_t xl2, uint64_t xl3, uint64_t xl4, uint64_t xl5, uint64_t xl6, uint64_t xl7
) {
    (void)z1; (void)z2; (void)z3; (void)z4; (void)z5; (void)z6; (void)z7;
    (void)xl1; (void)xl2; (void)xl3; (void)xl4; (void)xl5; (void)xl6; (void)xl7;
    uint64_t sq = z0 * z0;
    if (sq % 3 != xl0 % 3) return 0;
    if (sq % 5 != xl0 % 5) return 0;
    if (sq % 7 != xl0 % 7) return 0;
    if (sq % 11 != xl0 % 11) return 0;
    return 1;
}

uint64_t ualbf_static_suffix_bound_w0(uint32_t k) { (void)k; return 0; }
uint64_t ualbf_static_suffix_bound_w1(uint32_t k) { (void)k; return 0; }

uint64_t ualbf_euler_ceiling_num = (1ULL << 63) | EULER_CEILING_NUM;
uint64_t ualbf_euler_ceiling_den = (1ULL << 63) | EULER_CEILING_DEN;
uint64_t ualbf_baseline_min_prime_factors = (1ULL << 63) | BASELINE_MIN_PRIME_FACTORS;
uint64_t ualbf_prasad_sunitha_bound = (1ULL << 63) | PRASAD_SUNITHA_PROOF_BOUND;
uint64_t ualbf_div_5_coprime_3_bound = (1ULL << 63) | DIV_5_COPRIME_3_PROOF_BOUND;

uint64_t ualbf_target_abundance_num = (1ULL << 63) | 2;
uint64_t ualbf_target_abundance_den = (1ULL << 63) | 1;

uint32_t ualbf_pollard_rho_iteration_limit = (1U << 31) | POLLARD_RHO_ITERATION_LIMIT;
uint32_t ualbf_pollard_rho_batch_size = (1U << 31) | POLLARD_RHO_BATCH_SIZE;

void ualbf_dfs_loop(uint64_t ctx) { (void)ctx; }
uint32_t ualbf_evaluate_baseline_min_ffi(uint8_t contains_3, uint8_t contains_5, uint8_t skipped_3, uint8_t skipped_5) {
    if (!contains_3 && !contains_5 && skipped_3 && skipped_5) return PRASAD_SUNITHA_PROOF_BOUND;
    if (!contains_3 && contains_5 && skipped_3) return DIV_5_COPRIME_3_BOUND;
    return BASELINE_MIN_PRIME_FACTORS;
}

uint32_t ualbf_target_min_log10 = (1U << 31) | TARGET_MIN_LOG10;
uint32_t ualbf_target_max_log10 = (1U << 31) | TARGET_MAX_LOG10;
uint64_t ualbf_sieve_limit = (1ULL << 63) | SIEVE_LIMIT;
uint32_t ualbf_max_exponent = (1U << 31) | MAX_EXPONENT;
uint64_t ualbf_prefix_stop_threshold = (1ULL << 63) | PREFIX_STOP_THRESHOLD;
uint32_t ualbf_raycast_gpu_threshold = (1U << 31) | RAYCAST_GPU_THRESHOLD;
uint32_t ualbf_raycast_chunk_size = (1U << 31) | RAYCAST_CHUNK_SIZE;
uint32_t ualbf_conjectural_active = (1U << 31) | CONJECTURAL_ACTIVE;
uint32_t ualbf_conjectural_max_log10_ceiling = (1U << 31) | CONJECTURAL_MAX_LOG10_CEILING;

uint64_t ualbf_bloom_get_index(uint64_t hash1, uint64_t hash2, uint64_t num_bits, uint32_t i) {
    uint64_t current = hash1 + (uint64_t)i * hash2 + (uint64_t)i * (uint64_t)i;
    return num_bits == 0 ? 0 : current % num_bits;
}

const char* lean_string_cstr(void* str) { (void)str; return "dummy_hash"; }
void* lean_mk_string(const char* s) { (void)s; return (void*)1; }
void* ualbf_logic_hash = (void*)1;
