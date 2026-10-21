#ifndef LEAN_H
#define LEAN_H
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

typedef struct lean_object lean_object;
typedef uint8_t lean_obj_res;
typedef void (*lean_external_finalize_proc)(void*);
typedef void (*lean_external_foreach_proc)(void*, void*);
typedef struct lean_external_class lean_external_class;

#define LEAN_EXPORT

#ifndef LEAN_SHIMS_IMPL
static inline bool lean_is_scalar(lean_object* o) { return true; }
static inline lean_object* lean_ctor_get(lean_object* o, unsigned i) { return o; }
static inline lean_external_class* lean_register_external_class(lean_external_finalize_proc fin, lean_external_foreach_proc fore) { return NULL; }
static inline lean_object* lean_alloc_external(lean_external_class* cls, void* data) { return NULL; }
static inline void* lean_get_external_data(lean_object* o) { return NULL; }
static inline void lean_inc(lean_object* o) {}
static inline void lean_dec(lean_object* o) {}
static inline const char* ualbf_lean_string_cstr_inline(lean_object* o) { return ""; }
#else
bool lean_is_scalar(lean_object* o);
lean_object* lean_ctor_get(lean_object* o, unsigned i);
lean_external_class* lean_register_external_class(lean_external_finalize_proc fin, lean_external_foreach_proc fore);
lean_object* lean_alloc_external(lean_external_class* cls, void* data);
void* lean_get_external_data(lean_object* o);
void lean_inc(lean_object* o);
void lean_dec(lean_object* o);
const char* ualbf_lean_string_cstr_inline(lean_object* o);
#endif

#endif
