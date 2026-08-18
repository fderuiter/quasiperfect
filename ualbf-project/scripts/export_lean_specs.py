#!/usr/bin/env python3
import subprocess
import hashlib
import os
import json
import re


def generate_rust_types(schema, repo_root, schema_hash):
    # We will generate a file src/schema_generated.rs in rust-engine
    rust_path = os.path.join(repo_root, "rust-engine", "src", "schema_generated.rs")

    with open(rust_path, "w", encoding="utf-8") as f:
        f.write("// AUTO-GENERATED from schema_manifest.json. DO NOT EDIT.\n\n")
        f.write(f'pub const EXPORTED_SCHEMA_MANIFEST_HASH: &str = "{schema_hash}";\n\n')
        f.write("use crate::types::Uint;\n")
        f.write("use smallvec::SmallVec;\n")
        f.write("use serde::{Serialize, Deserialize};\n\n")

        for struct_name, struct_def in schema.items():
            if "fields" not in struct_def:
                continue
            fields = struct_def["fields"]

            # 1. Rust Struct (e.g. Prefix)
            f.write("#[derive(Clone, Debug)]\n")
            if struct_name == "SearchState":
                rust_name = "Prefix"  # In Rust, it's called Prefix
            else:
                rust_name = struct_name

            f.write(f"pub struct {rust_name} {{\n")
            for field in fields:
                f.write(f"    pub {field['name']}: {field['rust_type']},\n")
            f.write("}\n\n")

            # 2. Serialized Rust Struct (e.g. SerializedPrefix)
            ser_name = f"Serialized{rust_name}"
            f.write("#[derive(Serialize, Deserialize, Debug)]\n")
            f.write(f"pub struct {ser_name} {{\n")
            for field in fields:
                f.write(
                    f"    pub {field['name']}: {field.get('rust_ser_type', field['rust_type'])},\n"
                )
            f.write("}\n\n")

            # 3. Conversion methods
            f.write(f"impl {ser_name} {{\n")
            f.write(
                f"    pub fn from_{rust_name.lower()}(p: &{rust_name}) -> Self {{\n"
            )
            f.write("        Self {\n")
            for field in fields:
                conv = field.get("rust_ser_convert", "v.clone()").replace(
                    "v", f"p.{field['name']}"
                )
                f.write(f"            {field['name']}: {conv},\n")
            f.write("        }\n")
            f.write("    }\n\n")

            f.write(f"    pub fn to_{rust_name.lower()}(&self) -> {rust_name} {{\n")
            f.write(f"        {rust_name} {{\n")
            for field in fields:
                conv = field.get("rust_deser_convert", "v.clone()").replace(
                    "v", f"self.{field['name']}"
                )
                f.write(f"            {field['name']}: {conv},\n")
            f.write("        }\n")
            f.write("    }\n")
            f.write("}\n\n")

            # 4. Transport Rust Struct
            has_transport = any("ffi_transport_type" in field for field in fields)
            if has_transport:
                transport_name = f"{rust_name}Transport"
                f.write("#[repr(C)]\n")
                f.write("#[derive(Clone, Debug)]\n")
                f.write(f"pub struct {transport_name} {{\n")
                for field in fields:
                    if "ffi_transport_type" in field:
                        ffi_t = field["ffi_transport_type"]
                        if ffi_t == "U512":
                            f.write(
                                f"    pub {field['name']}: crate::lean_ffi::U512Data,\n"
                            )
                        elif ffi_t == "Array U512":
                            f.write(
                                f"    pub {field['name']}: *const crate::lean_ffi::U512Data,\n"
                            )
                            f.write(f"    pub {field['name']}_len: usize,\n")
                        else:
                            f.write(f"    pub {field['name']}: {ffi_t},\n")
                    else:
                        rust_t = field["rust_type"]
                        if "Vec<" in rust_t:
                            inner = rust_t.replace("Vec<", "").replace(">", "")
                            f.write(f"    pub {field['name']}: *const {inner},\n")
                            f.write(f"    pub {field['name']}_len: usize,\n")
                        else:
                            f.write(f"    pub {field['name']}: {rust_t},\n")
                f.write("}\n\n")

                # Conversion utilities
                f.write(f"impl {rust_name} {{\n")
                f.write(f"    pub fn to_transport(&self) -> {transport_name} {{\n")
                f.write(f"        {transport_name} {{\n")
                for field in fields:
                    if "ffi_transport_type" in field:
                        ffi_t = field["ffi_transport_type"]
                        if ffi_t == "U512":
                            f.write(f"            {field['name']}: {{\n")
                            f.write(
                                f"                let bytes = self.{field['name']}.to_le_bytes();\n"
                            )
                            f.write(
                                "                crate::lean_ffi::bytes_to_words::<{crate::lean_ffi::LIMB_COUNT * 8}, {crate::lean_ffi::LIMB_COUNT}>(&bytes)\n"
                            )
                            f.write("            },\n")
                        elif ffi_t == "Array U512":
                            f.write(
                                f"            {field['name']}: std::ptr::null(), // TODO: allocate arrays for FFI if needed\n"
                            )
                            f.write(
                                f"            {field['name']}_len: self.{field['name']}.len(),\n"
                            )
                    else:
                        rust_t = field["rust_type"]
                        if "Vec<" in rust_t:
                            f.write(
                                f"            {field['name']}: self.{field['name']}.as_ptr(),\n"
                            )
                            f.write(
                                f"            {field['name']}_len: self.{field['name']}.len(),\n"
                            )
                        else:
                            f.write(
                                f"            {field['name']}: self.{field['name']}.clone(),\n"
                            )
                f.write("        }\n")
                f.write("    }\n")
                f.write("}\n\n")

    subprocess.run(["cargo", "fmt", "--", rust_path], check=True, cwd=repo_root)


def generate_lean_types(schema, repo_root):
    lean_path = os.path.join(
        repo_root, "lean4-proofs", "UALBF", "Engine", "SearchState.lean"
    )
    with open(lean_path, "w", encoding="utf-8") as f:
        f.write("-- AUTO-GENERATED from schema_manifest.json. DO NOT EDIT.\n")
        f.write("import Mathlib.Data.Nat.Basic\n")
        f.write("import UALBF.FFI\n\n")
        f.write("set_option linter.all false\n\n")
        f.write("namespace UALBF.Engine\n\n")

        for struct_name, struct_def in schema.items():
            if "fields" not in struct_def:
                continue
            fields = struct_def["fields"]
            f.write(f"structure {struct_name} where\n")
            for field in fields:
                f.write(f"  {field['name']} : {field['lean_type']}\n")
            f.write("deriving Inhabited, Repr\n\n")

            has_transport = any("ffi_transport_type" in field for field in fields)
            if has_transport:
                transport_name = f"{struct_name}Transport"
                f.write(f"structure {transport_name} where\n")
                for field in fields:
                    if "ffi_transport_type" in field:
                        ffi_t = field["ffi_transport_type"]
                        if ffi_t == "U512":
                            f.write(f"  {field['name']} : UALBF.FFI.U512\n")
                        elif ffi_t == "Array U512":
                            f.write(f"  {field['name']} : Array UALBF.FFI.U512\n")
                    else:
                        f.write(f"  {field['name']} : {field['lean_type']}\n")
                f.write("deriving Inhabited\n\n")

                f.write(
                    f"def {transport_name}.toNative (t : {transport_name}) : {struct_name} := {{\n"
                )
                for field in fields:
                    if "ffi_transport_type" in field:
                        ffi_t = field["ffi_transport_type"]
                        if ffi_t == "U512":
                            f.write(
                                f"  {field['name']} := UALBF.FFI.fromU512 t.{field['name']},\n"
                            )
                        elif ffi_t == "Array U512":
                            f.write(
                                f"  {field['name']} := t.{field['name']}.map UALBF.FFI.fromU512,\n"
                            )
                    else:
                        f.write(f"  {field['name']} := t.{field['name']},\n")
                f.write("}\n\n")

        f.write("end UALBF.Engine\n")


def generate_verus_specs(bounds, repo_root, bounds_hash):
    export_path = os.path.join(repo_root, "rust-engine", "src", "lean_export.rs")
    with open(export_path, "w", encoding="utf-8") as f:
        tot_num = bounds["euler_ceiling"]["num"]
        tot_den = bounds["euler_ceiling"]["den"]

        hagis1982 = bounds["omega_bounds"]["hagis1982"]["proof_bound"]
        hagis1982_offset = bounds["omega_bounds"]["hagis1982"]["engine_justified_gap"]
        hagis1982_combined = hagis1982 + hagis1982_offset

        ps_bound = bounds["omega_bounds"]["prasad_sunitha"]["proof_bound"]
        ps_offset = bounds["omega_bounds"]["prasad_sunitha"]["engine_justified_gap"]
        ps_combined = ps_bound + ps_offset
        div_5_bound = bounds["omega_bounds"]["div_5_coprime_3"]["proof_bound"]
        div_5_offset = bounds["omega_bounds"]["div_5_coprime_3"]["engine_justified_gap"]
        div_5_combined = div_5_bound + div_5_offset
        mr_20_base_axiomatic = bounds.get("miller_rabin_20_base_sufficiency", {}).get(
            "is_axiomatic", False
        )
        conjectural_active = bounds.get("conjectural_bounds", {}).get("active", False)
        conjectural_max_log10_ceiling = bounds.get("conjectural_bounds", {}).get(
            "target_max_log10_ceiling", 0
        )

        crt_modulus_product = bounds["crt_obstruction"]["modulus_product"]
        crt_moduli = bounds["crt_obstruction"]["moduli"]

        prime_split_threshold = (
            bounds["search_bounds"].get("prime_split_threshold", {}).get("value", 61)
        )
        target_min_log10 = bounds["search_bounds"]["target_min_log10"]["value"]
        target_max_log10 = bounds["search_bounds"]["target_max_log10"]["value"]
        sieve_limit = bounds["search_bounds"]["sieve_limit"]["value"]
        max_exponent = bounds["search_bounds"]["max_exponent"]["value"]
        prefix_stop_threshold = bounds["search_bounds"]["prefix_stop_threshold"][
            "value"
        ]
        pollard_rho_iteration_limit = bounds["search_bounds"]["pollard_rho"][
            "iteration_limit"
        ]
        pollard_rho_batch_size = bounds["search_bounds"]["pollard_rho"]["batch_size"]
        overflow_num = bounds["overflow_threshold"]["num"]
        overflow_den = bounds["overflow_threshold"]["den"]
        raycast_gpu_threshold = bounds["search_bounds"]["raycast"]["gpu_threshold"]
        raycast_chunk_size = bounds["search_bounds"]["raycast"]["chunk_size"]

        f.write(f"""// AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.

pub const EXPORTED_BOUNDS_MANIFEST_HASH: &str = "{bounds_hash}";

use vstd::prelude::*;

verus! {{
    pub open spec fn lean_prime_split_threshold() -> nat {{ 61 }}
    pub open spec fn lean_prasad_sunitha_bound() -> nat {{ {ps_bound} }}
    pub open spec fn lean_prasad_sunitha_combined() -> nat {{ {ps_combined} }}
    pub open spec fn lean_div_5_coprime_3_bound() -> nat {{ {div_5_bound} }}
    pub open spec fn lean_div_5_coprime_3_combined() -> nat {{ {div_5_combined} }}
    pub open spec fn lean_hagis1982_combined() -> nat {{ {hagis1982_combined} }}
    pub open spec fn lean_qpn_totient_bound_num() -> nat {{ {tot_num} }}
    pub open spec fn lean_qpn_totient_bound_den() -> nat {{ {tot_den} }}
    pub open spec fn lean_target_min_log10() -> nat {{ {target_min_log10} }}
    pub open spec fn lean_target_max_log10() -> nat {{ {target_max_log10} }}
    pub open spec fn lean_sieve_limit() -> nat {{ {sieve_limit} }}
    pub open spec fn lean_max_exponent() -> nat {{ {max_exponent} }}
    pub open spec fn lean_prefix_stop_threshold() -> nat {{ {prefix_stop_threshold} }}
    pub open spec fn lean_pollard_rho_iteration_limit() -> nat {{ {pollard_rho_iteration_limit} }}
    pub open spec fn lean_pollard_rho_batch_size() -> nat {{ {pollard_rho_batch_size} }}
    pub open spec fn lean_overflow_threshold_num() -> nat {{ {overflow_num} }}
    pub open spec fn lean_overflow_threshold_den() -> nat {{ {overflow_den} }}
    pub open spec fn lean_raycast_gpu_threshold() -> nat {{ {raycast_gpu_threshold} }}
    pub open spec fn lean_raycast_chunk_size() -> nat {{ {raycast_chunk_size} }}
    pub open spec fn lean_conjectural_active() -> bool {{ {str(conjectural_active).lower()} }}
    pub open spec fn lean_conjectural_max_log10_ceiling() -> nat {{ {conjectural_max_log10_ceiling} }}
    pub open spec fn lean_crt_modulus_product() -> nat {{ {crt_modulus_product} }}

    pub open spec fn lean_hagis1982_min_prime_factors() -> nat {{ {hagis1982} }}
    pub open spec fn lean_hagis1982_offset() -> nat {{ {hagis1982_offset} }}
    pub open spec fn lean_prasad_sunitha_offset() -> nat {{ {ps_offset} }}
    pub open spec fn lean_div_5_coprime_3_offset() -> nat {{ {div_5_offset} }}
    pub open spec fn lean_miller_rabin_20_base_sufficiency() -> bool {{ {str(mr_20_base_axiomatic).lower()} }}

    pub proof fn prove_prime_split_threshold_equivalence()
        ensures (crate::manifest_constants::PRIME_SPLIT_THRESHOLD as nat) == lean_prime_split_threshold()
    {{}}

    pub proof fn prove_prasad_sunitha_bound_equivalence()
        ensures (crate::manifest_constants::PRASAD_SUNITHA_PROOF_BOUND as nat) == lean_prasad_sunitha_bound()
    {{}}

    pub proof fn prove_prasad_sunitha_combined_equivalence()
        ensures (crate::manifest_constants::PRASAD_SUNITHA_BOUND_NO_3_5 as nat) == lean_prasad_sunitha_combined()
    {{}}

    pub proof fn prove_div_5_coprime_3_bound_equivalence()
        ensures (crate::manifest_constants::DIV_5_COPRIME_3_PROOF_BOUND as nat) == lean_div_5_coprime_3_bound()
    {{}}

    pub proof fn prove_div_5_coprime_3_combined_equivalence()
        ensures (crate::manifest_constants::DIV_5_COPRIME_3_BOUND as nat) == lean_div_5_coprime_3_combined()
    {{}}

    pub proof fn prove_baseline_min_prime_factors_equivalence()
        ensures (crate::manifest_constants::BASELINE_MIN_PRIME_FACTORS as nat) == lean_hagis1982_combined()
    {{}}

    pub proof fn prove_euler_ceiling_num_equivalence()
        ensures (crate::manifest_constants::EULER_CEILING_NUM as nat) == lean_qpn_totient_bound_num()
    {{}}

    pub proof fn prove_euler_ceiling_den_equivalence()
        ensures (crate::manifest_constants::EULER_CEILING_DEN as nat) == lean_qpn_totient_bound_den()
    {{}}

    pub proof fn prove_target_min_log10_equivalence()
        ensures (crate::manifest_constants::TARGET_MIN_LOG10 as nat) == lean_target_min_log10()
    {{}}

    pub proof fn prove_target_max_log10_equivalence()
        ensures (crate::manifest_constants::TARGET_MAX_LOG10 as nat) == lean_target_max_log10()
    {{}}

    pub proof fn prove_sieve_limit_equivalence()
        ensures (crate::manifest_constants::SIEVE_LIMIT as nat) == lean_sieve_limit()
    {{}}

    pub proof fn prove_max_exponent_equivalence()
        ensures (crate::manifest_constants::MAX_EXPONENT as nat) == lean_max_exponent()
    {{}}

    pub proof fn prove_prefix_stop_threshold_equivalence()
        ensures (crate::manifest_constants::PREFIX_STOP_THRESHOLD as nat) == lean_prefix_stop_threshold()
    {{}}

    pub proof fn prove_pollard_rho_iteration_limit_equivalence()
        ensures (crate::manifest_constants::POLLARD_RHO_ITERATION_LIMIT as nat) == lean_pollard_rho_iteration_limit()
    {{}}

    pub proof fn prove_pollard_rho_batch_size_equivalence()
        ensures (crate::manifest_constants::POLLARD_RHO_BATCH_SIZE as nat) == lean_pollard_rho_batch_size()
    {{}}

    pub proof fn prove_overflow_threshold_num_equivalence()
        ensures (crate::manifest_constants::OVERFLOW_THRESHOLD_NUM as nat) == lean_overflow_threshold_num()
    {{}}

    pub proof fn prove_overflow_threshold_den_equivalence()
        ensures (crate::manifest_constants::OVERFLOW_THRESHOLD_DEN as nat) == lean_overflow_threshold_den()
    {{}}

    pub proof fn prove_raycast_gpu_threshold_equivalence()
        ensures (crate::manifest_constants::RAYCAST_GPU_THRESHOLD as nat) == lean_raycast_gpu_threshold()
    {{}}

    pub proof fn prove_raycast_chunk_size_equivalence()
        ensures (crate::manifest_constants::RAYCAST_CHUNK_SIZE as nat) == lean_raycast_chunk_size()
    {{}}

    pub proof fn prove_conjectural_active_equivalence()
        ensures crate::manifest_constants::CONJECTURAL_ACTIVE == lean_conjectural_active()
    {{}}

    pub proof fn prove_conjectural_max_log10_ceiling_equivalence()
        ensures (crate::manifest_constants::CONJECTURAL_MAX_LOG10_CEILING as nat) == lean_conjectural_max_log10_ceiling()
    {{}}

    pub proof fn prove_crt_modulus_product_equivalence()
        ensures (crate::manifest_constants::CRT_MODULUS_PRODUCT as nat) == lean_crt_modulus_product()
    {{}}

    pub proof fn prove_combined_bounds() {{
        assert(lean_hagis1982_combined() == lean_hagis1982_min_prime_factors() + lean_hagis1982_offset());
        assert(lean_prasad_sunitha_combined() == lean_prasad_sunitha_bound() + lean_prasad_sunitha_offset());
        assert(lean_div_5_coprime_3_combined() == lean_div_5_coprime_3_bound() + lean_div_5_coprime_3_offset());
    }}
}}
""")

    subprocess.run(["cargo", "fmt", "--", export_path], check=True, cwd=repo_root)


def map_type(t):
    t = t.strip()
    if t == "UInt8":
        return "u8"
    if t == "UInt32":
        return "u32"
    if t == "UInt64":
        return "u64"
    if t == "Bool":
        return "u8"
    if "U512" in t or t == "String" or t.startswith("IO "):
        return "*mut crate::lean_ffi::lean_object"
    if t == "Unit":
        return "()"
    return "UNKNOWN"


def generate_ffi_lean_spec(schema, repo_root, schema_hash):
    u512_def = schema.get("U512", {"bit_width": 512, "limb_width": 64})
    bit_width = u512_def.get("bit_width", 512)
    limb_width = u512_def.get("limb_width", 64)
    limb_count = bit_width // limb_width

    lean_generated_path = os.path.join(
        repo_root, "lean4-proofs", "UALBF", "FFI_generated.lean"
    )

    mk_args = " ".join(f"w{i}" for i in range(limb_count))
    mk_expr = "w0.toNat"
    for i in range(1, limb_count):
        mk_expr += f" +\n  w{i}.toNat * (2 ^ {i * limb_width})"

    getters = []
    getters.append(
        f'@[extern "rust_u512_get_w0"]\ndef U512.w0 (u : @& U512) : UInt64 :=\n  (u % 2^{limb_width}).toUInt64'
    )
    for i in range(1, limb_count):
        getters.append(
            f'@[extern "rust_u512_get_w{i}"]\ndef U512.w{i} (u : @& U512) : UInt64 :=\n  ((u / 2^{i * limb_width}) % 2^{limb_width}).toUInt64'
        )

    from_u512_expr = "u.w0.toNat"
    for i in range(1, limb_count):
        from_u512_expr += f" +\n  u.w{i}.toNat * (2 ^ {i * limb_width})"

    to_u512_expr = f"  U512.mk\n    (n % 2^{limb_width}).toUInt64"
    for i in range(1, limb_count):
        to_u512_expr += f"\n    ((n / 2^{i * limb_width}) % 2^{limb_width}).toUInt64"

    default_mk_args = " ".join("0" for _ in range(limb_count))

    prep_parts = []
    for j in range(1, limb_count):
        bits = j * limb_width
        val = 2**bits
        prep_parts.append(f"  have h2_{bits} : 2^{bits} = {val} := rfl;")

    prep_names = ", ".join(f"h2_{j * limb_width}" for j in range(1, limb_count))
    omega_prep = f"""syntax "u512_omega_prep" : tactic
macro_rules
  | `(tactic| u512_omega_prep) => `(tactic|
{chr(10).join(prep_parts)}
  rw [{prep_names}] at *
  )"""

    theorems = []
    for idx in range(limb_count):
        thm_args = " ".join(f"w{j}" for j in range(limb_count))
        h_decl = "\n  ".join(
            f"have _h{j} : w{j}.toNat < 2^64 := w{j}.toNat_lt"
            for j in range(limb_count)
        )
        theorems.append(
            f"""@[simp] theorem U512.w{idx}_mk ({thm_args} : UInt64) : U512.w{idx} (U512.mk {thm_args}) = w{idx} := by
  apply UInt64.ext
  simp [U512.w{idx}, U512.mk]
  {h_decl}
  u512_omega_prep
  omega"""
        )

    # Binary divide-and-conquer shift-addition algorithm generator
    L = [f"u.w{i}.toNat" for i in range(limb_count)]
    current_width = limb_width
    while len(L) > 1:
        next_L = []
        for i in range(0, len(L), 2):
            next_L.append(f"({L[i]} + ({L[i+1]} <<< {current_width}))")
        L = next_L
        current_width *= 2
    from_u512_fast_expr = L[0]

    with open(lean_generated_path, "w", encoding="utf-8") as f:
        f.write(f"""import Mathlib.Data.UInt
-- AUTO-GENERATED from schema_manifest.json. DO NOT EDIT.
set_option linter.all false

namespace UALBF.FFI

abbrev U512 : Type := Nat

@[extern "rust_u512_mk"]
def U512.mk ({mk_args} : UInt64) : U512 :=
  {mk_expr}

instance : Inhabited U512 where
  default := U512.mk {default_mk_args}

{chr(10).join(getters)}

{omega_prep}

{chr(10).join(theorems)}

def fromU512Fast (u : U512) : Nat :=
  {from_u512_fast_expr}

@[implemented_by fromU512Fast]
def fromU512 (u : U512) : Nat :=
  {from_u512_expr}

def toU512 (n : Nat) : U512 :=
{to_u512_expr}

def SCHEMA_MANIFEST_HASH : String := "{schema_hash}"

end UALBF.FFI
""")


def generate_ffi(repo_root, schema, schema_hash):
    ffi_paths = [
        os.path.join(repo_root, "lean4-proofs", "UALBF", "FFI_generated.lean"),
        os.path.join(repo_root, "lean4-proofs", "UALBF", "FFI.lean"),
        os.path.join(repo_root, "lean4-proofs", "UALBF", "BloomFilter.lean"),
    ]
    out_path = os.path.join(repo_root, "rust-engine", "src", "ffi_generated.rs")

    u512_def = schema.get("U512", {"bit_width": 512, "limb_width": 64})
    bit_width = u512_def.get("bit_width", 512)
    limb_width = u512_def.get("limb_width", 64)
    limb_count = bit_width // limb_width

    exports = []
    externs = []
    for ffi_path in ffi_paths:
        if not os.path.exists(ffi_path):
            continue
        with open(ffi_path, "r", encoding="utf-8") as f:
            content = f.read()
        exports.extend(
            re.findall(
                r"@\[export\s+(\w+)\]\n(?:private\s+|partial\s+|noncomputable\s+)?def\s+\w+\s*(.*?)\s*:\s*([a-zA-Z0-9_\. ]+?)(?:\s*:=|\n)",
                content,
                re.DOTALL,
            )
        )
        externs.extend(
            re.findall(
                r'@\[extern\s+"([^"]+)"\]\n(?:opaque|def)\s+(\S+)\s+(.*?)\n', content
            )
        )

    # Enforce Canonical Name and Signature Verification at Build-Time
    crt_export = None
    for exp in exports:
        if exp[0] == "ualbf_check_crt_1155":
            crt_export = exp
            break

    if crt_export is None:
        import sys
        print(
            "ERROR: FFI Naming Standardization / Build-Time Verification Failed!\n"
            "Expected canonical export function 'ualbf_check_crt_1155' was not found.\n"
            "Target file location: lean4-proofs/UALBF/FFI.lean\n"
            "Please ensure the modulo-1155 Chinese Remainder Theorem check is annotated with @[export ualbf_check_crt_1155].",
            file=sys.stderr
        )
        sys.exit(1)

    name, args_str, ret_type = crt_export
    args_str_norm = " ".join(args_str.strip().split())
    ret_type_norm = ret_type.strip()

    param_matches = re.findall(r"\(([^:]+):\s*([^)]+)\)", args_str_norm)
    params = []
    for p_names, p_type in param_matches:
        p_type = p_type.strip()
        for p_name in p_names.split():
            params.append((p_name, p_type))

    valid_sig = True
    if len(params) != 2:
        valid_sig = False
    else:
        for p_name, p_type in params:
            if p_type != "@& U512":
                valid_sig = False

    if ret_type_norm != "Bool":
        valid_sig = False

    if not valid_sig:
        import sys
        print(
            "ERROR: FFI Naming Standardization / Build-Time Verification Failed!\n"
            f"The signature of '{name}' is invalid.\n"
            f"Found signature: def ualbf_check_crt_1155_impl {args_str.strip()} : {ret_type_norm}\n"
            "Expected canonical name: ualbf_check_crt_1155\n"
            "Expected argument types: '@& U512' for both arguments\n"
            "Expected return type: 'Bool'\n"
            "Target file location: lean4-proofs/UALBF/FFI.lean\n"
            "Please ensure the function accepts two parameters of type '@& U512' and returns 'Bool'.",
            file=sys.stderr
        )
        sys.exit(1)

    out = []
    out.append("// AUTO-GENERATED from Lean metadata. DO NOT EDIT.\n")
    out.append(f'pub const EXPORTED_SCHEMA_MANIFEST_HASH: &str = "{schema_hash}";\n')
    out.append(f"pub const LIMB_COUNT: usize = {limb_count};\n")
    out.append(f"pub type U512Data = [u64; {limb_count}];\n")

    out.append('extern "C" {')
    for name, args_str, ret_type in exports:
        args = []
        if args_str.strip():
            for match in re.finditer(r"\(([^:]+):\s*([^)]+)\)", args_str):
                names = match.group(1).split()
                t = match.group(2)
                rt = map_type(t)
                for n in names:
                    args.append(f"{n}: {rt}")
        if ret_type.strip().startswith("IO "):
            args.append("w: *mut crate::lean_ffi::lean_object")
        ret = map_type(ret_type)
        if not args:
            out.append(f"    pub static {name}: {ret};")
        else:
            ret_str = f" -> {ret}" if ret != "()" else ""
            out.append(f"    pub fn {name}({', '.join(args)}){ret_str};")
    out.append("}\n")

    for i in range(limb_count):
        out.append(
            f'#[no_mangle]\npub extern "C" fn rust_u512_get_w{i}(obj: *mut crate::lean_ffi::lean_object) -> u64 {{ unsafe {{ (*crate::lean_ffi::get_u512_ptr(obj))[{i}] }} }}\n'
        )

    mk_args = ", ".join(f"w{i}: u64" for i in range(limb_count))
    mk_array = ", ".join(f"w{i}" for i in range(limb_count))
    out.append(f"""
#[no_mangle]
pub extern "C" fn rust_u512_mk({mk_args}) -> *mut crate::lean_ffi::lean_object {{
    crate::lean_ffi::alloc_u512([{mk_array}])
}}
""")

    with open(out_path, "w", encoding="utf-8") as f:
        f.write("\n".join(out))

    subprocess.run(["cargo", "fmt", "--", out_path], check=True, cwd=repo_root)
    print(f"FFI bindings generated to {out_path}")


def main():
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    # 1. Load schema manifest
    schema_path = os.path.join(repo_root, "schema_manifest.json")
    if os.path.exists(schema_path):
        with open(schema_path, "r", encoding="utf-8") as f:
            schema_content = f.read()
            schema = json.loads(schema_content)
            schema_hash = hashlib.sha256(schema_content.encode("utf-8")).hexdigest()
        generate_rust_types(schema, repo_root, schema_hash)
        generate_lean_types(schema, repo_root)
        generate_ffi_lean_spec(schema, repo_root, schema_hash)
        print(f"Schema generated from {schema_path}")
    else:
        print(f"Warning: {schema_path} not found.")
        schema = {}
        schema_hash = "0" * 64

    # 2. Load bounds manifest
    bounds_path = os.path.join(repo_root, "bounds_manifest.json")
    if os.path.exists(bounds_path):

        with open(bounds_path, "r", encoding="utf-8") as f:
            bounds_content = f.read()
            bounds = json.loads(bounds_content)
            bounds_hash = hashlib.sha256(bounds_content.encode("utf-8")).hexdigest()
        generate_verus_specs(bounds, repo_root, bounds_hash)
        print(f"Verus specs generated from {bounds_path}")
        generate_ffi(repo_root, schema, schema_hash)

        # Generate manifest constants
        prasad_proof = bounds["omega_bounds"]["prasad_sunitha"]["proof_bound"]
        prasad_bound = (
            prasad_proof
            + bounds["omega_bounds"]["prasad_sunitha"]["engine_justified_gap"]
        )
        div_5_coprime_3_proof = bounds["omega_bounds"]["div_5_coprime_3"]["proof_bound"]
        div_5_coprime_3_bound = (
            div_5_coprime_3_proof
            + bounds["omega_bounds"]["div_5_coprime_3"]["engine_justified_gap"]
        )
        baseline_min = (
            bounds["omega_bounds"]["hagis1982"]["proof_bound"]
            + bounds["omega_bounds"]["hagis1982"]["engine_justified_gap"]
        )
        euler_num = bounds["euler_ceiling"]["num"]
        euler_den = bounds["euler_ceiling"]["den"]
        target_min_log10 = bounds["search_bounds"]["target_min_log10"]["value"]
        target_max_log10 = bounds["search_bounds"]["target_max_log10"]["value"]
        sieve_limit = bounds["search_bounds"]["sieve_limit"]["value"]
        max_exponent = bounds["search_bounds"]["max_exponent"]["value"]
        prefix_stop_threshold = bounds["search_bounds"]["prefix_stop_threshold"][
            "value"
        ]
        prime_split_threshold = (
            bounds["search_bounds"].get("prime_split_threshold", {}).get("value", 61)
        )

        # Validation checks on configuration parameters
        if prime_split_threshold != 61:
            raise ValueError(
                f"Safety Constraint Violation: prime_split_threshold ({prime_split_threshold}) must strictly equal static proof baseline 61 to ensure formal Lean proof coverage alignment."
            )

        pollard_rho_iteration_limit = bounds["search_bounds"]["pollard_rho"][
            "iteration_limit"
        ]
        pollard_rho_batch_size = bounds["search_bounds"]["pollard_rho"]["batch_size"]
        overflow_num = bounds["overflow_threshold"]["num"]
        overflow_den = bounds["overflow_threshold"]["den"]
        raycast_gpu_threshold = bounds["search_bounds"]["raycast"]["gpu_threshold"]
        raycast_chunk_size = bounds["search_bounds"]["raycast"]["chunk_size"]

        conjectural_active = bounds.get("conjectural_bounds", {}).get("active", False)
        conjecture_name = bounds.get("conjectural_bounds", {}).get(
            "conjecture_name", "None"
        )
        conjectural_max_log10 = bounds.get("conjectural_bounds", {}).get(
            "target_max_log10_ceiling", 0
        )

        touchard_mod = bounds["touchard_mod_24"]["modulus"]
        touchard_residues = bounds["touchard_mod_24"]["residues"]

        crt_modulus_product = bounds["crt_obstruction"]["modulus_product"]
        crt_moduli = bounds["crt_obstruction"]["moduli"]

        rust_code = f"""// AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.
#[cfg(not(verus_keep_ghost))]
pub const PRIME_SPLIT_THRESHOLD: u64 = {prime_split_threshold};
#[cfg(not(verus_keep_ghost))]
pub const PRASAD_SUNITHA_PROOF_BOUND: u64 = {prasad_proof};
#[cfg(not(verus_keep_ghost))]
pub const PRASAD_SUNITHA_BOUND_NO_3_5: u64 = {prasad_bound};
#[cfg(not(verus_keep_ghost))]
pub const DIV_5_COPRIME_3_PROOF_BOUND: u64 = {div_5_coprime_3_proof};
#[cfg(not(verus_keep_ghost))]
pub const DIV_5_COPRIME_3_BOUND: u64 = {div_5_coprime_3_bound};
#[cfg(not(verus_keep_ghost))]
pub const BASELINE_MIN_PRIME_FACTORS: u64 = {baseline_min};
#[cfg(not(verus_keep_ghost))]
pub const EULER_CEILING_NUM: u64 = {euler_num};
#[cfg(not(verus_keep_ghost))]
pub const EULER_CEILING_DEN: u64 = {euler_den};
#[cfg(not(verus_keep_ghost))]
pub const TARGET_MIN_LOG10: u32 = {target_min_log10};
#[cfg(not(verus_keep_ghost))]
pub const TARGET_MAX_LOG10: u32 = {target_max_log10};
#[cfg(not(verus_keep_ghost))]
pub const SIEVE_LIMIT: usize = {sieve_limit};
#[cfg(not(verus_keep_ghost))]
pub const MAX_EXPONENT: u32 = {max_exponent};
#[cfg(not(verus_keep_ghost))]
pub const PREFIX_STOP_THRESHOLD: u64 = {prefix_stop_threshold};
#[cfg(not(verus_keep_ghost))]
pub const POLLARD_RHO_ITERATION_LIMIT: u32 = {pollard_rho_iteration_limit};
#[cfg(not(verus_keep_ghost))]
pub const POLLARD_RHO_BATCH_SIZE: u32 = {pollard_rho_batch_size};
#[cfg(not(verus_keep_ghost))]
pub const OVERFLOW_THRESHOLD_NUM: u64 = {overflow_num};
#[cfg(not(verus_keep_ghost))]
pub const OVERFLOW_THRESHOLD_DEN: u64 = {overflow_den};
#[cfg(not(verus_keep_ghost))]
pub const RAYCAST_GPU_THRESHOLD: usize = {raycast_gpu_threshold};
#[cfg(not(verus_keep_ghost))]
pub const RAYCAST_CHUNK_SIZE: usize = {raycast_chunk_size};
#[cfg(not(verus_keep_ghost))]
pub const CONJECTURAL_ACTIVE: bool = {str(conjectural_active).lower()};
#[cfg(not(verus_keep_ghost))]
pub const CONJECTURE_NAME: &str = "{conjecture_name}";
#[cfg(not(verus_keep_ghost))]
pub const CONJECTURAL_MAX_LOG10_CEILING: u32 = {conjectural_max_log10};
#[cfg(not(verus_keep_ghost))]
pub const TOUCHARD_MOD_24_MODULUS: u32 = {touchard_mod};
#[cfg(not(verus_keep_ghost))]
pub const TOUCHARD_MOD_24_RESIDUES: [u32; {len(touchard_residues)}] = [{', '.join(map(str, touchard_residues))}];
#[cfg(not(verus_keep_ghost))]
pub const CRT_MODULUS_PRODUCT: u32 = {crt_modulus_product};
#[cfg(not(verus_keep_ghost))]
pub const CRT_MODULI: [u32; {len(crt_moduli)}] = [{', '.join(map(str, crt_moduli))}];
#[cfg(not(verus_keep_ghost))]
pub const MANIFEST_HASH: &str = "{bounds_hash}";

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {{
    pub const PRIME_SPLIT_THRESHOLD: u64 = {prime_split_threshold};
    pub const PRASAD_SUNITHA_PROOF_BOUND: u64 = {prasad_proof};
    pub const PRASAD_SUNITHA_BOUND_NO_3_5: u64 = {prasad_bound};
    pub const DIV_5_COPRIME_3_PROOF_BOUND: u64 = {div_5_coprime_3_proof};
    pub const DIV_5_COPRIME_3_BOUND: u64 = {div_5_coprime_3_bound};
    pub const BASELINE_MIN_PRIME_FACTORS: u64 = {baseline_min};
    pub const EULER_CEILING_NUM: u64 = {euler_num};
    pub const EULER_CEILING_DEN: u64 = {euler_den};
    pub const TARGET_MIN_LOG10: u32 = {target_min_log10};
    pub const TARGET_MAX_LOG10: u32 = {target_max_log10};
    pub const SIEVE_LIMIT: usize = {sieve_limit};
    pub const MAX_EXPONENT: u32 = {max_exponent};
    pub const PREFIX_STOP_THRESHOLD: u64 = {prefix_stop_threshold};
    pub const POLLARD_RHO_ITERATION_LIMIT: u32 = {pollard_rho_iteration_limit};
    pub const POLLARD_RHO_BATCH_SIZE: u32 = {pollard_rho_batch_size};
    pub const OVERFLOW_THRESHOLD_NUM: u64 = {overflow_num};
    pub const OVERFLOW_THRESHOLD_DEN: u64 = {overflow_den};
    pub const RAYCAST_GPU_THRESHOLD: usize = {raycast_gpu_threshold};
    pub const RAYCAST_CHUNK_SIZE: usize = {raycast_chunk_size};
    pub const CONJECTURAL_ACTIVE: bool = {str(conjectural_active).lower()};
    pub const CONJECTURAL_MAX_LOG10_CEILING: u32 = {conjectural_max_log10};
    pub const TOUCHARD_MOD_24_MODULUS: u32 = {touchard_mod};
    pub const CRT_MODULUS_PRODUCT: u32 = {crt_modulus_product};
    pub const MANIFEST_HASH: &'static str = "{bounds_hash}";
}}
"""
        with open(
            os.path.join(repo_root, "rust-engine", "src", "manifest_constants.rs"), "w"
        ) as f:
            f.write(rust_code)

        c_code = f"""// AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.
#define PRIME_SPLIT_THRESHOLD {prime_split_threshold}
#define PRASAD_SUNITHA_PROOF_BOUND {prasad_proof}
#define DIV_5_COPRIME_3_PROOF_BOUND {div_5_coprime_3_proof}
#define DIV_5_COPRIME_3_BOUND {div_5_coprime_3_bound}
#define BASELINE_MIN_PRIME_FACTORS {baseline_min}
#define EULER_CEILING_NUM {euler_num}
#define EULER_CEILING_DEN {euler_den}
#define TARGET_MIN_LOG10 {target_min_log10}
#define TARGET_MAX_LOG10 {target_max_log10}
#define SIEVE_LIMIT {sieve_limit}
#define MAX_EXPONENT {max_exponent}
#define PREFIX_STOP_THRESHOLD {prefix_stop_threshold}
#define POLLARD_RHO_ITERATION_LIMIT {pollard_rho_iteration_limit}
#define POLLARD_RHO_BATCH_SIZE {pollard_rho_batch_size}
#define OVERFLOW_THRESHOLD_NUM {overflow_num}
#define OVERFLOW_THRESHOLD_DEN {overflow_den}
#define RAYCAST_GPU_THRESHOLD {raycast_gpu_threshold}
#define RAYCAST_CHUNK_SIZE {raycast_chunk_size}
#define CONJECTURAL_ACTIVE {1 if conjectural_active else 0}
#define CONJECTURE_NAME "{conjecture_name}"
#define CONJECTURAL_MAX_LOG10_CEILING {conjectural_max_log10}
#define TOUCHARD_MOD_24_MODULUS {touchard_mod}
#define TOUCHARD_MOD_24_RESIDUES_LEN {len(touchard_residues)}
#define TOUCHARD_MOD_24_RESIDUES {{ {', '.join(map(str, touchard_residues))} }}
#define CRT_MODULUS_PRODUCT {crt_modulus_product}
#define CRT_MODULI_LEN {len(crt_moduli)}
#define CRT_MODULI {{ {', '.join(map(str, crt_moduli))} }}
"""
        with open(
            os.path.join(repo_root, "rust-engine", "src", "manifest_constants.h"), "w"
        ) as f:
            f.write(c_code)

        lean_code = f"""-- AUTO-GENERATED from bounds_manifest.json. DO NOT EDIT.
set_option linter.all false
namespace UALBF.Manifest

def PRIME_SPLIT_THRESHOLD : Nat := 61
def PRASAD_SUNITHA_PROOF_BOUND : Nat := {prasad_proof}
def PRASAD_SUNITHA_BOUND_NO_3_5 : Nat := {prasad_bound}
def DIV_5_COPRIME_3_PROOF_BOUND : Nat := {div_5_coprime_3_proof}
def DIV_5_COPRIME_3_BOUND : Nat := {div_5_coprime_3_bound}
def BASELINE_MIN_PRIME_FACTORS : Nat := {baseline_min}
def EULER_CEILING_NUM : Nat := {euler_num}
def EULER_CEILING_DEN : Nat := {euler_den}
def TARGET_MIN_LOG10 : Nat := {target_min_log10}
def TARGET_MAX_LOG10 : Nat := {target_max_log10}
def SIEVE_LIMIT : Nat := {sieve_limit}
def MAX_EXPONENT : Nat := {max_exponent}
def PREFIX_STOP_THRESHOLD : Nat := {prefix_stop_threshold}
def POLLARD_RHO_ITERATION_LIMIT : Nat := {pollard_rho_iteration_limit}
def POLLARD_RHO_BATCH_SIZE : Nat := {pollard_rho_batch_size}
def OVERFLOW_THRESHOLD_NUM : Nat := {overflow_num}
def OVERFLOW_THRESHOLD_DEN : Nat := {overflow_den}
def RAYCAST_GPU_THRESHOLD : Nat := {raycast_gpu_threshold}
def RAYCAST_CHUNK_SIZE : Nat := {raycast_chunk_size}

def CONJECTURAL_ACTIVE : Bool := {'true' if conjectural_active else 'false'}
def CONJECTURE_NAME : String := "{conjecture_name}"
def CONJECTURAL_MAX_LOG10_CEILING : Nat := {conjectural_max_log10}

def TOUCHARD_MOD_24_MODULUS : Nat := {touchard_mod}
def TOUCHARD_MOD_24_RESIDUES : Array Nat := #[{', '.join(map(str, touchard_residues))}]

def CRT_MODULUS_PRODUCT : Nat := {crt_modulus_product}
def CRT_MODULI : Array Nat := #[{', '.join(map(str, crt_moduli))}]

def PRIME_FACTOR_LIST : Array Nat := #[{', '.join(map(str, bounds.get('prime_factor_list', [])))}]
def STATIC_SUFFIX_BOUNDS : Array Nat := #[{', '.join(map(str, bounds.get('static_suffix_bounds', [])))}]

def LOGIC_HASH : String := "{bounds_hash}"

end UALBF.Manifest
"""
        with open(
            os.path.join(repo_root, "lean4-proofs", "UALBF", "ManifestConstants.lean"),
            "w",
        ) as f:
            f.write(lean_code)
    else:
        print(f"Warning: {bounds_path} not found.")


if __name__ == "__main__":
    main()
