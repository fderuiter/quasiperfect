#include "../manifest_constants.h"
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

void lean_initialize_runtime_module() {}
void lean_initialize() {}
void initialize_Ualbf_C_Main() {}
void lean_initialize_thread() {}

void* lean_register_external_class(void* finalize, void* foreach) { (void)finalize; (void)foreach; return 0; }
void* rs_lean_alloc_external(void* cls, void* data) { (void)cls; return data; }
void* rs_lean_get_external_data(void* obj) { return obj; }
void rs_lean_inc(void* obj) { (void)obj; }
void rs_lean_dec(void* obj) { (void)obj; }

bool rs_lean_is_scalar(void* obj) {
    return ((uintptr_t)obj & 1) == 1;
}

void* rs_lean_ctor_get(void* obj, unsigned int idx) {
    return *(void**)((uint8_t*)obj + 8 + idx * sizeof(void*));
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
    uint64_t p_mod = p % 24;
    uint64_t sum = 0;
    uint64_t term = 1;
    for (uint32_t i = 0; i <= two_e; i++) {
        sum = (sum + term) % 24;
        term = (term * p_mod) % 24;
    }
    return ((sum % 2 == 0) || (sum == 9)) ? 1 : 0;
}

void* make_some(void* val) {
    void** ptr = malloc(16);
    ptr[0] = (void*)0; // header
    ptr[1] = val;      // index 0 field
    return ptr;
}

void* ualbf_compute_sigma(uint64_t p, uint64_t pow) {
    uint64_t* u512_data = malloc(64);
    memset(u512_data, 0, 64);
    uint64_t sum = 1;
    uint64_t term = 1;
    for (uint32_t i = 1; i <= pow; i++) {
        term *= p;
        sum += term;
    }
    u512_data[0] = sum;
    void* u512_obj = rs_lean_alloc_external(NULL, u512_data);
    return make_some(u512_obj);
}

void* ualbf_cyclotomic_eval_pub(uint32_t d, void* p) {
    uint64_t* u512_data = malloc(64);
    memset(u512_data, 0, 64);
    u512_data[0] = 1;
    void* u512_obj = rs_lean_alloc_external(NULL, u512_data);
    return make_some(u512_obj);
}

void* ualbf_mod_inverse(void* a_obj, uint8_t a_neg, void* m_obj) {
    uint64_t* u512_data = malloc(64);
    memset(u512_data, 0, 64);
    u512_data[0] = 1;
    void* u512_obj = rs_lean_alloc_external(NULL, u512_data);
    return make_some(u512_obj);
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

uint64_t ualbf_bloom_get_index(uint64_t hash1, uint64_t hash2, uint64_t num_bits, uint32_t i) {
    uint64_t current = hash1 + (uint64_t)i * hash2 + (uint64_t)i * (uint64_t)i;
    return num_bits == 0 ? 0 : current % num_bits;
}

const char* lean_string_cstr(void* str) { (void)str; return "dummy_hash"; }
void* lean_mk_string(const char* s) { (void)s; return (void*)1; }
void* ualbf_logic_hash = (void*)1;
