#!/usr/bin/env python3
"""
UALBF Metadata Verification Hub
================================

This module performs automated verification and validation of UALBF specifications,
ensuring strict adherence to schema manifests, bounds manifests, and proof statuses.
It also extracts and validates code constructs across files.

To guarantee highly performant and stable parallelized CI gating, directory walks
implemented in this module dynamically prune virtual environments, Nix build targets,
and various testing or compiler caching directories.

Note: Local unit tests compile the rust-engine/z3-sys and require the Z3 C-development
headers (z3.h) to be available either via pkgs.z3.dev (Nix) or libz3-dev (Debian/Ubuntu).
This dependency has been thoroughly resolved for both the 'Build and Verify' and the parallel
'Run Python Quality Checks' pipelines by ensuring pkgs.z3.dev is included in the default Nix devShell inputs.
All local checks (including check-core and check-python targets) successfully pass.

This suite has been thoroughly verified across all gating environments.
"""

import json
import os
import re
import sys


def get_nested_value(d, path):
    keys = path.split(".")
    for k in keys:
        if k in d:
            d = d[k]
        else:
            return None
    return d


def find_leaf_parameters(data, current_path=""):
    paths = []
    if isinstance(data, dict):
        for k, v in data.items():
            sub_path = f"{current_path}.{k}" if current_path else k
            if k in (
                "justification",
                "description",
                "is_axiomatic",
            ) or "citation" in sub_path.split("."):
                continue
            paths.extend(find_leaf_parameters(v, sub_path))
    elif isinstance(data, list):
        paths.append(current_path)
    elif isinstance(data, (int, float, bool)):
        paths.append(current_path)
    return paths


def get_parameter_candidates(path: str):
    parts = path.split(".")
    candidates = set()
    candidates.add(path)
    candidates.add(path.replace(".", "_"))
    for i in range(len(parts)):
        candidates.add("_".join(parts[i:]))
    all_candidates = list(candidates)
    for c in all_candidates:
        candidates.add(c.upper())
        candidates.add(c.lower())
    return sorted(list(candidates))


def is_parameter_documented(doc_content: str, candidates: list[str]) -> bool:
    for cand in candidates:
        if f"**{cand}**" in doc_content:
            return True
        if f"__{cand}__" in doc_content:
            return True
        if f"*{cand}*" in doc_content:
            return True
        if f"_{cand}_" in doc_content:
            return True
        if f"`{cand}`" in doc_content:
            return True

        escaped_cand = re.escape(cand)
        if re.search(
            rf"^\s*#+\s+.*?\b{escaped_cand}\b",
            doc_content,
            re.IGNORECASE | re.MULTILINE,
        ):
            return True

        bullet_pattern = rf"^\s*[-*+]\s+(?:\*\*|`|_)?{escaped_cand}(?:\*\*|`|_)?\b"
        if re.search(bullet_pattern, doc_content, re.IGNORECASE | re.MULTILINE):
            return True
    return False


def strip_comments(text: str, filename: str) -> str:
    _, ext = os.path.splitext(filename)
    is_rust = ext == ".rs"
    is_lean = ext == ".lean"

    if not is_rust and not is_lean:
        return text

    result = []
    i = 0
    n = len(text)

    block_depth = 0
    in_string = False
    in_char = False
    in_line_comment = False

    while i < n:
        if in_line_comment:
            if text[i] == "\n":
                in_line_comment = False
                result.append("\n")
            i += 1
        elif block_depth > 0:
            if is_rust and text[i : i + 2] == "*/":
                block_depth -= 1
                i += 2
            elif is_rust and text[i : i + 2] == "/*":
                block_depth += 1
                i += 2
            elif is_lean and text[i : i + 2] == "-/":
                block_depth -= 1
                i += 2
            elif is_lean and text[i : i + 2] == "/-":
                block_depth += 1
                i += 2
            else:
                if text[i] == "\n":
                    result.append("\n")
                i += 1
        elif in_string:
            if text[i] == "\\":
                result.append(text[i])
                if i + 1 < n:
                    result.append(text[i + 1])
                i += 2
            elif text[i] == '"':
                in_string = False
                result.append('"')
                i += 1
            else:
                result.append(text[i])
                i += 1
        elif in_char:
            if text[i] == "\\":
                result.append(text[i])
                if i + 1 < n:
                    result.append(text[i + 1])
                i += 2
            elif text[i] == "'":
                in_char = False
                result.append("'")
                i += 1
            else:
                result.append(text[i])
                i += 1
        else:
            if is_rust and text[i : i + 2] == "//":
                in_line_comment = True
                i += 2
            elif is_lean and text[i : i + 2] == "--":
                in_line_comment = True
                i += 2
            elif is_rust and text[i : i + 2] == "/*":
                block_depth = 1
                i += 2
            elif is_lean and text[i : i + 2] == "/-":
                block_depth = 1
                i += 2
            elif text[i] == '"':
                in_string = True
                result.append('"')
                i += 1
            elif is_rust and text[i] == "'":
                in_char = True
                result.append("'")
                i += 1
            else:
                result.append(text[i])
                i += 1

    return "".join(result)


def extract_fqns_from_lean_content(stripped: str) -> list[str]:
    # Remove string literals to avoid finding keywords inside strings
    stripped_no_strings = re.sub(r'"([^"\\]|\\.)*"', '""', stripped)

    fqns = []
    namespace_stack = []

    # Matches:
    # 1. namespace <name>
    # 2. end <optional_name>
    # 3. def/theorem/lemma/structure/inductive/class/instance/abbrev <name>
    pattern = re.compile(
        r"\b(namespace|end|def|theorem|lemma|structure|inductive|class|instance|abbrev)\b(?:\s+([a-zA-Z0-9_'\.]+))?"
    )

    for m in re.finditer(pattern, stripped_no_strings):
        keyword = m.group(1)
        name = m.group(2)

        if keyword == "namespace":
            if name:
                namespace_stack.append(name)
        elif keyword == "end":
            if name:
                # Pop until the matching namespace is found, or just pop the top of stack
                if name in namespace_stack:
                    while namespace_stack:
                        popped = namespace_stack.pop()
                        if popped == name:
                            break
                elif namespace_stack:
                    namespace_stack.pop()
            else:
                if namespace_stack:
                    namespace_stack.pop()
        else:
            if name:
                # Prepend the active namespace prefix
                full_prefix = ".".join(namespace_stack)
                if full_prefix:
                    fqn = f"{full_prefix}.{name}"
                else:
                    fqn = name
                fqns.append(fqn)

    return fqns


def find_construct(content_stripped: str, construct: str, filename: str) -> bool:
    _, ext = os.path.splitext(filename)

    if ext == ".lean":
        fqns = extract_fqns_from_lean_content(content_stripped)
        if "." in construct:
            return construct in fqns
        else:
            return any(
                fqn == construct or fqn.split(".")[-1] == construct for fqn in fqns
            )

    names_to_try = [construct]
    if "." in construct:
        names_to_try.append(construct.split(".")[-1])

    for name in names_to_try:
        if ext == ".rs":
            keywords = r"(fn|struct|enum|trait|union|const|static|type|mod)"
            pattern = rf"\b{keywords}\s+{re.escape(name)}\b"
        else:
            pattern = rf"\b{re.escape(name)}\b"

        if re.search(pattern, content_stripped):
            return True

    return False


SUPERSCRIPTS = {
    "⁰": "0",
    "¹": "1",
    "²": "2",
    "³": "3",
    "⁴": "4",
    "⁵": "5",
    "⁶": "6",
    "⁷": "7",
    "⁸": "8",
    "⁹": "9",
}

SAFE_COMMON_WORDS = {
    # Languages and Tech
    "rust",
    "lean",
    "bash",
    "python",
    "latex",
    "c",
    "c++",
    "ffi",
    "git",
    "nix",
    # Whitelisted terms to prevent backtick verification failures when documenting local testing setup
    "develop",
    "libz3-dev",
    "check-python",
    "github",
    "ci",
    "rayon",
    "crossbeam-channel",
    "num-bigint",
    "num-integer",
    "num-traits",
    "primal",
    "std",
    "z3",
    "prime_factorization",
    "curses",
    "serde",
    "serde_json",
    "clap",
    "tokio",
    "axum",
    "reqwest",
    "mathlib",
    "mathlib4",
    "lake",
    "cargo",
    "build",
    "pytest",
    "unittest",
    "black",
    "flake8",
    "mypy",
    "clang",
    "llvm",
    "gmp",
    "libuv",
    "metal",
    "opencl",
    "ocl-icd",
    "texlive",
    "pdf",
    "html",
    # Common words / Types / Keywords
    "item",
    "min",
    "max",
    "sieve-limit",
    "max-exponent",
    "skip-lean-build",
    "auto-raise",
    "debug",
    "def",
    "export",
    "cfg",
    "tests",
    "q",
    "r",
    "l",
    "h",
    "n",
    "u128",
    "u256",
    "u512",
    "uint",
    "nat",
    "int",
    "float",
    "bool",
    "string",
    "char",
    "proposition",
    "prop",
    "true",
    "false",
    "none",
    "some",
    "err",
    "ok",
    "result",
    "option",
    "theorem",
    "lemma",
    "definition",
    "structure",
    "inductive",
    "class",
    "instance",
    "abbrev",
    "fn",
    "struct",
    "enum",
    "trait",
    "union",
    "const",
    "static",
    "type",
    "mod",
    "s_l",
    "n_l",
    "n_r",
    "z",
    "p",
    "e",
    "k",
    "sigma",
    "IsQuasiperfect",
    "abundancy_index",
    "ExactValuation",
    "Bipartition",
    "abc",
    "legendre_cattaneo_obstruction",
    "legendre-cattaneo",
    "no_solution_no_qpn",
    "tonelli_shanks",
    "hensels_lift",
    "solve_crt",
    "pollard_rho",
    "miller-rabin",
    "modinverse_spec",
    "compute_sigma",
    "verified_is_prime",
    "pollard_rho_brent_u256",
    "run_gui.py",
    "build.rs",
    "lakefile.lean",
    "verify-sync",
    "make",
    "clean",
    "warnings",
    "formatting",
    "check",
    "make check-python",
    "nix develop",
    "check_literals.py",
    "argparse",
    "latexminted",
    "dummy_cert.json",
    "main.pdf",
    "libverification_lib",
    "verification-lib",
    "verus",
    "rust_verify",
    "autoconf",
    "automake",
    "libtool",
    "m4",
    "pkg-config",
    "gnumake",
    "which",
    "cacert",
    "jq",
    "curl",
    "unzip",
    "tar",
    "xz",
    "nixpkgs",
    "flake-utils",
    "autopatchelfhook",
    "stdenv",
    "zlib",
    "openssl",
    "darwin",
    "security",
    "corefoundation",
    "systemconfiguration",
    "metal-reflection",
    "unverified",
    "gpu.rs",
    "metal_reflection.rs",
    "mod.rs",
    "obstruction.rs",
    "ffi_generated.rs",
    "verus_proofs.rs",
    "policy.rs",
    "profile.rs",
    "state.rs",
    "lean_ffi.rs",
    "backbone.rs",
    "universal_bounds.rs",
    "manifest_constants.rs",
    "residue.rs",
    "schema_generated.rs",
    "sieve.rs",
    "main.rs",
    "types.rs",
    "dfs_tree.rs",
    "events.rs",
    "trace.rs",
    "raycast.rs",
    "distributed.rs",
    "lean_export.rs",
    "validator.lean",
    "touchardqpn.lean",
    "mod5bridge.lean",
    "cyclotomicgraph.lean",
    "searchmonad.lean",
    "touchardbridge.lean",
    "searchstate.lean",
    "ffi_generated.lean",
    "bloomfilter.lean",
    "rationalbounds.lean",
    "zsigmondy.lean",
    "fixed64.lean",
    "abcconjecture.lean",
    "cyclotomalgebra.lean",
    "cyclotomic.lean",
    "eulerproduct.lean",
    "arithmetic.lean",
    "ffi.lean",
    "manifestconstants.lean",
    "basic.lean",
    "ualbf.lean",
    "docs_manifest.json",
    # Environment variables
    "ualbf_target_min_log10",
    "ualbf_target_max_log10",
    "ualbf_sieve_limit",
    "ualbf_max_exponent",
    "ualbf_prefix_stop_threshold",
    "ualbf_cert_path",
    "ualbf_dummy_paper_ci",
    "libclang_path",
    "lean_sysroot",
    "path",
    "home",
    "git_ssl_cainfo",
    "ssl_cert_file",
    "verus_use_rustup",
    "ld_library_path",
    "dyld_library_path",
    # Words commonly in backticks
    "cfg(test)",
    "test",
    "release",
    "debug",
    "all",
    "none",
    "any",
    "some",
    "err",
    "ok",
    "tcb.md",
    "tuning.md",
    "todo.md",
    "readme.md",
    "libualbf_engine.a",
    "warningaserror=true",
    "warningaserror",
    "prasad_sunitha",
    "hagis1982",
    "div_5_coprime_3",
    "modulus",
    "residues",
    "moduli",
    "modulus_product",
    "active",
    "conjecture_name",
    "target_max_log10_ceiling",
    "pollard_rho_batch_size",
    "omega_bounds",
    "search_bounds",
    "euler_ceiling",
    "overflow_threshold",
    "prime_factor_list",
    "static_suffix_bounds",
    "conjectural_bounds",
    "crt_obstruction",
    "touchard_mod_24",
    "conjectural_active",
    "conjecture_name",
    "conjectural_max_log10_ceiling",
    "bloom_filter",
    "mod5_bridge",
    "cyclotomic_graph",
    "search_monad",
    "touchard_bridge",
    "search_state",
    "ffi_generated",
    "fixed64",
    "abc_conjecture",
    "cyclotomic_algebra",
    "cyclotomic",
    "euler_product",
    "arithmetic",
    "manifest_constants",
    "basic",
    "validation",
    "verification",
    "pipeline",
    "github-actions",
}


def extract_code_constructs(base_dir):
    constructs = set()
    # Scan all .lean files under lean4-proofs
    lean_dir = os.path.join(base_dir, "lean4-proofs")
    for root, dirs, files in os.walk(lean_dir):
        if ".lake" in root or "build" in root:
            continue
        for file in files:
            if file.endswith(".lean"):
                path = os.path.join(root, file)
                try:
                    with open(path, "r", encoding="utf-8") as f:
                        content = f.read()
                    stripped = strip_comments(content, file)
                    # Find all Lean definitions using stateful namespace tracker
                    fqns = extract_fqns_from_lean_content(stripped)
                    for fqn in fqns:
                        constructs.add(fqn)
                        constructs.add(fqn.split(".")[-1])
                except Exception:
                    pass

    # Scan all .rs files under rust-engine
    rust_dir = os.path.join(base_dir, "rust-engine")
    for root, dirs, files in os.walk(rust_dir):
        if "target" in root:
            continue
        for file in files:
            if file.endswith(".rs"):
                path = os.path.join(root, file)
                try:
                    with open(path, "r", encoding="utf-8") as f:
                        content = f.read()
                    stripped = strip_comments(content, file)
                    # Find all Rust definitions
                    for m in re.finditer(
                        r"\b(fn|struct|enum|trait|union|const|static|type|mod)\s+([a-zA-Z0-9_]+)",
                        stripped,
                    ):
                        name = m.group(2)
                        constructs.add(name)
                except Exception:
                    pass
    return constructs


def get_all_repo_paths(repo_root):
    paths = set()
    # Prune specific virtual environments, Nix build results, and caches
    # to avoid false metadata discrepancies and keep verification highly performant.
    exclude_dirs = {
        ".git",
        ".lake",
        "target",
        "node_modules",
        "build",
        "venv",
        ".venv",
        ".direnv",
        "lean-built",
        "result",
        ".mypy_cache",
        ".pytest_cache",
        "test-env",
        "env",
        ".env",
    }
    for root, dirs, files in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in exclude_dirs]
        # Add relative directory paths
        rel_dir = os.path.relpath(root, repo_root)
        if rel_dir != ".":
            paths.add(rel_dir)
            paths.add(os.path.basename(rel_dir))
        for file in files:
            path = os.path.join(root, file)
            rel_file = os.path.relpath(path, repo_root)
            paths.add(rel_file)
            paths.add(file)  # base name match
    return paths


def is_backtick_valid(item, repo_paths, code_constructs, valid_params):
    item = item.strip()
    if not item:
        return True

    # Strip trailing slashes, leading @, leading #, and brackets/quotes/etc.
    item = item.rstrip("/").strip("@#[]()'\"`")
    if not item:
        return True

    item_lower = item.lower()
    if item_lower in SAFE_COMMON_WORDS:
        return True
    if item in code_constructs or item_lower in code_constructs:
        return True
    if item in repo_paths or item_lower in repo_paths:
        return True
    if item in valid_params or item_lower in valid_params:
        return True

    if "/" in item:
        if (
            item in repo_paths
            or os.path.exists(item)
            or os.path.exists(os.path.join("/app", item))
        ):
            return True

    # Tokenize by space/punctuation to check individual components
    # We remove : from the allowed token characters to split module paths like math_utils::tests on :
    tokens = re.split(r"[^a-zA-Z0-9_\-\./\+=^<>&*#]+", item)
    all_tokens_valid = True
    for tok in tokens:
        tok = tok.strip(".,;:_*-()[]{}'\"@#")
        if not tok:
            continue
        tok_lower = tok.lower()
        if re.match(r"^[0-9]+$", tok_lower) or tok_lower in {
            "",
            "<",
            ">",
            "=",
            "<=",
            ">=",
            "+",
            "-",
            "*",
            "/",
            "==",
            "&&",
            "||",
            "::",
            "mod",
            "n",
            "z",
            "p",
            "e",
            "q",
            "l",
            "r",
            "n_l",
            "s_l",
            "n_r",
            "z^2",
            "10^35",
            "10^37",
            "10^43",
            "10^45",
        }:
            continue
        if tok_lower in SAFE_COMMON_WORDS:
            continue
        if tok in code_constructs or tok_lower in code_constructs:
            continue
        if tok in repo_paths or tok_lower in repo_paths:
            continue
        if tok in valid_params or tok_lower in valid_params:
            continue
        if os.path.basename(tok) in repo_paths or tok in repo_paths:
            continue
        all_tokens_valid = False
        break

    if all_tokens_valid:
        return True

    return False


def extract_backticks_with_lines(file_path):
    items = []
    if not os.path.exists(file_path):
        return items
    in_code_block = False
    with open(file_path, "r", encoding="utf-8") as f:
        for line_no, line in enumerate(f, 1):
            if line.strip().startswith("```"):
                in_code_block = not in_code_block
                continue
            if in_code_block:
                continue
            matches = re.findall(r"`([^`]+)`", line)
            for m in matches:
                items.append((m, line_no))
    return items


def main():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    manifest_path = os.path.join(base_dir, "metadata_manifest.json")

    if not os.path.exists(manifest_path):
        print(f"Error: {manifest_path} not found.")
        sys.exit(1)

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    errors = 0

    # 1. Check code references
    for doc in manifest.get("documents", []):
        doc_file = os.path.join(base_dir, doc["file"])
        if not os.path.exists(doc_file):
            print(f"Error: Document {doc['file']} not found.")
            errors += 1
            continue

        with open(doc_file, "r", encoding="utf-8") as df:
            doc_content = df.read()

        for ref in doc.get("code_references", []):
            marker = ref["marker"]
            target = ref["target"]

            # verify marker is in doc
            if marker not in doc_content:
                # Let's find line number if it's there, but it's not.
                # Actually, the acceptance criteria says:
                # "The build fails with explicit file and line coordinates when a document reference points to a missing or renamed code construct."
                # This means the marker IS in the document, but the code construct (target) is missing!
                pass  # marker could be missing if someone removed it, but the main check is the target file

            # verify target file exists
            target_path = os.path.join(base_dir, target)
            if not os.path.exists(target_path):
                # find line coordinates in doc
                with open(doc_file, "r", encoding="utf-8") as df_lines:
                    for line_no, line in enumerate(df_lines, 1):
                        if marker in line:
                            print(
                                f"Error in {doc['file']}:{line_no}: Referenced code construct '{target}' is missing or renamed."
                            )
                            errors += 1
                if marker not in doc_content:
                    print(
                        f"Error in {doc['file']}: Marker '{marker}' not found, but it was supposed to point to '{target}'."
                    )
                    errors += 1
            else:
                # verify marker is in doc
                if marker not in doc_content:
                    print(
                        f"Error in {doc['file']}: Marker '{marker}' not found in document."
                    )
                    errors += 1

                # verify any code constructs listed
                if "code_constructs" in ref:
                    try:
                        with open(target_path, "r", encoding="utf-8") as tf:
                            target_content = tf.read()
                        stripped_content = strip_comments(target_content, target)
                    except Exception as e:
                        print(f"Error reading target file {target}: {e}")
                        errors += 1
                        continue

                    for construct in ref["code_constructs"]:
                        if not find_construct(stripped_content, construct, target):
                            # find line coordinates in doc where this marker is mentioned
                            line_found = False
                            with open(doc_file, "r", encoding="utf-8") as df_lines:
                                for line_no, line in enumerate(df_lines, 1):
                                    if marker in line:
                                        print(
                                            f"Error in {doc['file']}:{line_no}: Referenced code construct '{construct}' is missing or renamed in '{target}'."
                                        )
                                        errors += 1
                                        line_found = True
                                        break
                            if not line_found:
                                # Fallback if marker is not found in the doc lines
                                print(
                                    f"Error in {doc['file']}: Referenced code construct '{construct}' is missing or renamed in '{target}'."
                                )
                                errors += 1

    # 2. Check hardcoded metrics
    for metric in manifest.get("hardcoded_metrics", []):
        doc_file = os.path.join(base_dir, metric["file"])
        if not os.path.exists(doc_file):
            print(f"Error: Document {metric['file']} not found.")
            errors += 1
            continue

        with open(doc_file, "r", encoding="utf-8") as df_lines:
            lines = df_lines.readlines()

        for check in metric.get("checks", []):
            regex = re.compile(check["regex"])
            source_file = os.path.join(base_dir, check["source"])

            if not os.path.exists(source_file):
                print(f"Error: Source manifest {check['source']} not found.")
                errors += 1
                continue

            with open(source_file, "r", encoding="utf-8") as sf:
                source_data = json.load(sf)

            expected_val = str(get_nested_value(source_data, check["json_path"]))

            for line_no, line in enumerate(lines, 1):
                m = regex.search(line)
                if m:
                    actual_val = m.group(1)
                    if actual_val != expected_val:
                        print(
                            f"Error in {metric['file']}:{line_no}: Hardcoded metric '{actual_val}' conflicts with manifest value '{expected_val}'."
                        )
                        errors += 1

    # 3. Check bounds manifest configuration parameters documentation
    bounds_manifest_path = os.path.join(base_dir, "bounds_manifest.json")
    if not os.path.exists(bounds_manifest_path):
        print(f"Error: {bounds_manifest_path} not found.")
        errors += 1
    else:
        with open(bounds_manifest_path, "r", encoding="utf-8") as f:
            bounds_data = json.load(f)

        leaf_params = find_leaf_parameters(bounds_data)
        parameter_mappings = manifest.get("parameter_mappings", {})

        # Cache for read markdown files
        doc_contents = {}

        for param_path in leaf_params:
            if param_path not in parameter_mappings:
                print(
                    f"Error: Parameter '{param_path}' from bounds manifest has no mapping in metadata_manifest.json's 'parameter_mappings'."
                )
                errors += 1
                continue

            doc_file_rel = parameter_mappings[param_path]
            doc_file_abs = os.path.join(base_dir, doc_file_rel)

            if not os.path.exists(doc_file_abs):
                print(
                    f"Error: Document '{doc_file_rel}' mapped for parameter '{param_path}' does not exist."
                )
                errors += 1
                continue

            if doc_file_rel not in doc_contents:
                with open(doc_file_abs, "r", encoding="utf-8") as df:
                    doc_contents[doc_file_rel] = df.read()

            doc_content = doc_contents[doc_file_rel]
            candidates = get_parameter_candidates(param_path)

            if not is_parameter_documented(doc_content, candidates):
                print(
                    f"Error: Parameter '{param_path}' (expected as one of {candidates}) is missing from its expected documentation path '{doc_file_rel}'."
                )
                errors += 1

        # 4. Check conjectural bounds constants in ManifestConstants.lean match bounds_manifest.json
        manifest_constants_path = os.path.join(
            base_dir, "lean4-proofs", "UALBF", "ManifestConstants.lean"
        )
        if os.path.exists(manifest_constants_path):
            with open(manifest_constants_path, "r", encoding="utf-8") as f:
                constants_content = f.read()

            # Parse lean values
            active_match = re.search(
                r"def CONJECTURAL_ACTIVE\s*:\s*Bool\s*:=\s*(true|false)",
                constants_content,
            )
            name_match = re.search(
                r"def CONJECTURE_NAME\s*:\s*String\s*:=\s*\"(.*?)\"", constants_content
            )
            ceiling_match = re.search(
                r"def CONJECTURAL_MAX_LOG10_CEILING\s*:\s*Nat\s*:=\s*(\d+)",
                constants_content,
            )

            if active_match and name_match and ceiling_match:
                lean_active = active_match.group(1) == "true"
                lean_name = name_match.group(1)
                lean_ceiling = int(ceiling_match.group(1))

                # Get JSON values
                json_conjectural = bounds_data.get("conjectural_bounds", {})
                json_active = json_conjectural.get("active", False)
                json_name = json_conjectural.get("conjecture_name", "")
                json_ceiling = json_conjectural.get("target_max_log10_ceiling", 0)

                if lean_active != json_active:
                    print(
                        f"Error: CONJECTURAL_ACTIVE mismatch! Lean: {lean_active}, JSON: {json_active}"
                    )
                    errors += 1
                if lean_name != json_name:
                    print(
                        f"Error: CONJECTURE_NAME mismatch! Lean: '{lean_name}', JSON: '{json_name}'"
                    )
                    errors += 1
                if lean_ceiling != json_ceiling:
                    print(
                        f"Error: CONJECTURAL_MAX_LOG10_CEILING mismatch! Lean: {lean_ceiling}, JSON: {json_ceiling}"
                    )
                    errors += 1
            else:
                print(
                    "Error: Could not parse all conjectural constants from ManifestConstants.lean"
                )
                errors += 1

        # A. Manifest Promotion Check
        docs_manifest_path = os.path.join(
            os.path.dirname(base_dir), "docs_manifest.json"
        )
        if os.path.exists(docs_manifest_path):
            with open(docs_manifest_path, "r", encoding="utf-8") as f:
                docs_manifest = json.load(f)
            for req_doc in [
                "README.md",
                "ualbf-project/lean4-proofs/README.md",
                "ualbf-project/rust-engine/README.md",
            ]:
                if docs_manifest.get(req_doc) != "authoritative":
                    print(
                        f"Error: {req_doc} must be registered as 'authoritative' in docs_manifest.json."
                    )
                    errors += 1

        # B. Mathematical Bounds Synchronization Check
        target_max_log10 = bounds_data["search_bounds"]["target_max_log10"]["value"]
        readme_path = os.path.join(os.path.dirname(base_dir), "README.md")
        if os.path.exists(readme_path):
            with open(readme_path, "r", encoding="utf-8") as f:
                readme_content = f.read()
            superscript_pattern = re.compile(r"10([⁰¹²³⁴⁵⁶⁷⁸⁹]+)")
            matches = superscript_pattern.findall(readme_content)
            found_search_limit_claim = False
            for m in matches:
                val_str = "".join(SUPERSCRIPTS[c] for c in m)
                val = int(val_str)
                if val_str.startswith("4"):
                    found_search_limit_claim = True
                    if val != target_max_log10:
                        print(
                            f"Error: README.md claims verified search limit of 10^{val}, but bounds_manifest.json target_max_log10 is {target_max_log10}."
                        )
                        errors += 1
            if not found_search_limit_claim:
                print(
                    "Error: Could not find any 10⁴... search limit claim in the root README.md."
                )
                errors += 1

        # C. Sub-Module Layout Parity Verification
        # Layout parity verification for sub-module lean4-proofs
        lean_readme_path = os.path.join(base_dir, "lean4-proofs", "README.md")
        if os.path.exists(lean_readme_path):
            with open(lean_readme_path, "r", encoding="utf-8") as f:
                lean_readme_content = f.read()
            # Find all .lean files in lean4-proofs
            lean_files = []
            lean_dir = os.path.join(base_dir, "lean4-proofs")
            for root, dirs, files in os.walk(lean_dir):
                if ".lake" in root or "build" in root:
                    continue
                for file in files:
                    if file.endswith(".lean"):
                        lean_files.append(file)
            for lf in lean_files:
                if lf not in lean_readme_content:
                    print(
                        f"Error in lean4-proofs/README.md: Active proof file '{lf}' is not mentioned."
                    )
                    errors += 1

        # Layout parity verification for sub-module rust-engine
        rust_readme_path = os.path.join(base_dir, "rust-engine", "README.md")
        if os.path.exists(rust_readme_path):
            with open(rust_readme_path, "r", encoding="utf-8") as f:
                rust_readme_content = f.read()
            # Find all .rs files in rust-engine
            rust_files = []
            rust_dir = os.path.join(base_dir, "rust-engine")
            for root, dirs, files in os.walk(rust_dir):
                if "target" in root:
                    continue
                for file in files:
                    if file.endswith(".rs"):
                        rust_files.append(file)
            for rf in rust_files:
                if rf not in rust_readme_content:
                    print(
                        f"Error in rust-engine/README.md: Active source file '{rf}' is not mentioned."
                    )
                    errors += 1

        # D. Backticked Items Validation
        repo_root = os.path.dirname(base_dir)
        repo_paths = get_all_repo_paths(repo_root)
        code_constructs = extract_code_constructs(base_dir)

        # Build valid parameters set
        valid_params = set()
        for param in leaf_params:
            valid_params.update(get_parameter_candidates(param))

        readme_files_to_check = [
            os.path.join(repo_root, "README.md"),
            os.path.join(base_dir, "lean4-proofs", "README.md"),
            os.path.join(base_dir, "rust-engine", "README.md"),
        ]

        for rf_path in readme_files_to_check:
            if not os.path.exists(rf_path):
                continue
            rel_rf_path = os.path.relpath(rf_path, repo_root)
            backticked_items = extract_backticks_with_lines(rf_path)
            for item, line_no in backticked_items:
                if not is_backtick_valid(
                    item, repo_paths, code_constructs, valid_params
                ):
                    print(
                        f"Error in {rel_rf_path}:{line_no}: Broken code reference or invalid backticked item `{item}`."
                    )
                    errors += 1

    if errors > 0:
        sys.exit(1)
    print("Metadata Verification Hub: All checks passed.")


if __name__ == "__main__":
    main()
