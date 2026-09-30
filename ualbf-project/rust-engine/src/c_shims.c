/* lean.h defines lean_string_cstr as static inline, so it has no linkable
   symbol. Rename the inline copy here and export a real lean_string_cstr
   below for the Rust extern declaration in lean_ffi.rs. */
#define lean_string_cstr ualbf_lean_string_cstr_inline
#include <lean/lean.h>
#undef lean_string_cstr
#include <stdbool.h>

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

extern uint8_t ualbf_mod_inverse_ok_limbs(
    uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
    uint8_t a_neg,
    uint64_t m0, uint64_t m1, uint64_t m2, uint64_t m3, uint64_t m4, uint64_t m5, uint64_t m6, uint64_t m7
);

extern uint64_t ualbf_mod_inverse_limb(
    uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
    uint8_t a_neg,
    uint64_t m0, uint64_t m1, uint64_t m2, uint64_t m3, uint64_t m4, uint64_t m5, uint64_t m6, uint64_t m7,
    uint32_t limb_idx
);

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
