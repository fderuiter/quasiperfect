/* AUTO-GENERATED from verification-lib/src/lib.rs. DO NOT EDIT. */

#ifndef VERIFICATION_LIB_H
#define VERIFICATION_LIB_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void* verify_certificate(const char* cert_json_ptr, const char* pub_key_ptr, bool* is_valid_out, char* out_manifest_hash_buf, size_t out_manifest_hash_len);
void free_certificate(void* cert_ptr);
char* rust_sha256_file(const char* path_ptr);
void rust_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif /* VERIFICATION_LIB_H */
