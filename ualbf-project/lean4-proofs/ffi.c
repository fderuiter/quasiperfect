#include <lean/lean.h>
#include <stdbool.h>
#include "verification_ffi.h"
#include "schema_generated.h"

static lean_external_class* g_cert_class = NULL;

static void cert_finalize(void* ptr) {
    free_certificate(ptr);
}
static void cert_foreach(void* ptr, b_lean_obj_arg b) {}

lean_obj_res lean_init_cert_class(lean_obj_arg w) {
    if (g_cert_class == NULL) {
        g_cert_class = lean_register_external_class(cert_finalize, cert_foreach);
    }
    return lean_io_result_mk_ok(lean_box(0));
}

lean_obj_res lean_sha256_file(b_lean_obj_arg path_obj) {
    const char* path = lean_string_cstr(path_obj);
    char* c_hash = rust_sha256_file(path);
    if (c_hash == NULL) {
        return lean_mk_string("");
    }
    lean_object* res = lean_mk_string(c_hash);
    rust_free_string(c_hash);
    return res;
}

lean_obj_res verify_certificate_ffi(b_lean_obj_arg cert_json, b_lean_obj_arg pub_key) {
    const char* c_cert_json = lean_string_cstr(cert_json);
    const char* c_pub_key = lean_string_cstr(pub_key);
    
    bool is_valid = false;
    char manifest_hash_buf[256];
    manifest_hash_buf[0] = '\0';
    void* cert_ptr = verify_certificate(c_cert_json, c_pub_key, &is_valid, manifest_hash_buf, sizeof(manifest_hash_buf));
    
    if (!is_valid) {
        lean_object* err_msg;
        if (manifest_hash_buf[0] != '\0') {
            err_msg = lean_mk_string(manifest_hash_buf);
        } else {
            err_msg = lean_mk_string("Invalid certificate or signature");
        }
        lean_object* res = lean_alloc_ctor(0, 1, 0); // Except.error
        lean_ctor_set(res, 0, err_msg);
        return res;
    }
    
    lean_object* hash_str = lean_mk_string(manifest_hash_buf);
    lean_object* cert_obj = lean_alloc_external(g_cert_class, cert_ptr);
    
    lean_object* tuple = lean_alloc_ctor(0, 2, 0); // Prod.mk
    lean_ctor_set(tuple, 0, hash_str);
    lean_ctor_set(tuple, 1, cert_obj);
    
    lean_object* res = lean_alloc_ctor(1, 1, 0); // Except.ok
    lean_ctor_set(res, 0, tuple);
    
    return res;
}

lean_obj_res rs_lean_io_result_mk_ok(lean_obj_arg a) {
    return lean_io_result_mk_ok(a);
}

lean_obj_res rs_lean_box_uint32(uint32_t v) {
    return lean_box_uint32(v);
}

lean_obj_res rs_lean_box_bool(bool v) {
    return lean_box(v ? 1 : 0);
}

lean_obj_res rs_lean_box_unit() {
    return lean_box(0);
}
