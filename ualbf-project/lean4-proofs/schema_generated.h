// AUTO-GENERATED from schema_manifest.json. DO NOT EDIT.

#ifndef SCHEMA_GENERATED_H
#define SCHEMA_GENERATED_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <assert.h>

#define EXPORTED_SCHEMA_MANIFEST_HASH "9dee5c21477c18a6ea5bae0f83237dd66f3a8bbc2528ad18724d79074dff9fdb"

typedef struct U512Data {
    uint64_t limbs[8];
} U512Data;

typedef struct PrefixTransport {
    U512Data n_l;
    U512Data s_l;
    size_t last_idx;
    const uint64_t* factors;
    size_t factors_len;
    const U512Data* sigma_factors;
    size_t sigma_factors_len;
    const uint64_t* sigma_factors_u64;
    size_t sigma_factors_u64_len;
    const uint64_t* active_mask;
    size_t active_mask_len;
    uint32_t sigma_mod24;
} PrefixTransport;

typedef PrefixTransport SearchStateTransport;

_Static_assert(offsetof(PrefixTransport, n_l) == 0, "PrefixTransport.n_l offset mismatch");
_Static_assert(offsetof(PrefixTransport, s_l) == 64, "PrefixTransport.s_l offset mismatch");
_Static_assert(offsetof(PrefixTransport, last_idx) == 128, "PrefixTransport.last_idx offset mismatch");
_Static_assert(offsetof(PrefixTransport, factors) == 136, "PrefixTransport.factors offset mismatch");
_Static_assert(offsetof(PrefixTransport, factors_len) == 144, "PrefixTransport.factors_len offset mismatch");
_Static_assert(offsetof(PrefixTransport, sigma_factors) == 152, "PrefixTransport.sigma_factors offset mismatch");
_Static_assert(offsetof(PrefixTransport, sigma_factors_len) == 160, "PrefixTransport.sigma_factors_len offset mismatch");
_Static_assert(offsetof(PrefixTransport, sigma_factors_u64) == 168, "PrefixTransport.sigma_factors_u64 offset mismatch");
_Static_assert(offsetof(PrefixTransport, sigma_factors_u64_len) == 176, "PrefixTransport.sigma_factors_u64_len offset mismatch");
_Static_assert(offsetof(PrefixTransport, active_mask) == 184, "PrefixTransport.active_mask offset mismatch");
_Static_assert(offsetof(PrefixTransport, active_mask_len) == 192, "PrefixTransport.active_mask_len offset mismatch");
_Static_assert(offsetof(PrefixTransport, sigma_mod24) == 200, "PrefixTransport.sigma_mod24 offset mismatch");
_Static_assert(sizeof(PrefixTransport) == 208, "PrefixTransport size mismatch");

#endif // SCHEMA_GENERATED_H
