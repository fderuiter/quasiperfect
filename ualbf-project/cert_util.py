import os
import hashlib
import sys

_has_verification_lib = True
try:
    import verification_lib  # type: ignore

    hash_tcb = verification_lib.hash_tcb
    hash_extension_tcb = verification_lib.hash_extension_tcb
    check_path_continuity = verification_lib.check_path_continuity
    compute_verus_hashes = verification_lib.compute_verus_hashes
except ImportError:
    hash_tcb = None
    hash_extension_tcb = None
    check_path_continuity = None
    _has_verification_lib = False

    def clean_source_py(content: str) -> str:
        cleaned = []
        chars = list(content)
        i = 0
        n = len(chars)

        state = "Normal"
        depth = 0

        while i < n:
            if state == "Normal":
                if i + 1 < n and chars[i] == "/" and chars[i + 1] == "/":
                    state = "InLineComment"
                    i += 2
                elif i + 1 < n and chars[i] == "/" and chars[i + 1] == "*":
                    state = "InBlockComment"
                    depth = 1
                    i += 2
                elif chars[i] == '"':
                    state = "InString"
                    cleaned.append('"')
                    i += 1
                elif chars[i] == "'":
                    state = "InChar"
                    cleaned.append("'")
                    i += 1
                else:
                    cleaned.append(chars[i])
                    i += 1
            elif state == "InString":
                if chars[i] == "\\":
                    cleaned.append("\\")
                    if i + 1 < n:
                        cleaned.append(chars[i + 1])
                        i += 2
                    else:
                        i += 1
                elif chars[i] == '"':
                    state = "Normal"
                    cleaned.append('"')
                    i += 1
                else:
                    cleaned.append(chars[i])
                    i += 1
            elif state == "InChar":
                if chars[i] == "\\":
                    cleaned.append("\\")
                    if i + 1 < n:
                        cleaned.append(chars[i + 1])
                        i += 2
                    else:
                        i += 1
                elif chars[i] == "'":
                    state = "Normal"
                    cleaned.append("'")
                    i += 1
                else:
                    cleaned.append(chars[i])
                    i += 1
            elif state == "InLineComment":
                if chars[i] == "\n":
                    state = "Normal"
                    cleaned.append("\n")
                    i += 1
                else:
                    i += 1
            elif state == "InBlockComment":
                if i + 1 < n and chars[i] == "/" and chars[i + 1] == "*":
                    depth += 1
                    i += 2
                elif i + 1 < n and chars[i] == "*" and chars[i + 1] == "/":
                    depth -= 1
                    if depth == 0:
                        state = "Normal"
                    i += 2
                elif chars[i] == "\n":
                    cleaned.append("\n")
                    i += 1
                else:
                    i += 1
        return "".join(cleaned)

    def count_non_literal_braces_py(line: str) -> tuple[int, int]:
        chars = list(line)
        open_count = 0
        close_count = 0
        in_string = False
        in_char = False
        i = 0
        n = len(chars)

        while i < n:
            if in_string:
                if chars[i] == "\\":
                    i += 2
                elif chars[i] == '"':
                    in_string = False
                    i += 1
                else:
                    i += 1
            elif in_char:
                if chars[i] == "\\":
                    i += 2
                elif chars[i] == "'":
                    in_char = False
                    i += 1
                else:
                    i += 1
            else:
                if chars[i] == '"':
                    in_string = True
                    i += 1
                elif chars[i] == "'":
                    in_char = True
                    i += 1
                elif chars[i] == "{":
                    open_count += 1
                    i += 1
                elif chars[i] == "}":
                    close_count += 1
                    i += 1
                else:
                    i += 1
        return open_count, close_count

    def compute_verus_hashes_fallback(content: str) -> dict[str, str]:
        cleaned = clean_source_py(content)
        verus_hashes = {}
        current_fn = ""
        current_body = ""
        in_spec = False
        brace_count = 0
        module_stack = []
        module_brace_depth = 0

        kw_list = [
            "pub spec fn ",
            "pub open spec fn ",
            "pub uninterp spec fn ",
            "pub proof fn ",
            "pub fn ",
        ]

        for line in cleaned.splitlines():
            trimmed = line.strip()

            # Track module declarations
            if not in_spec:
                if "{" in trimmed and (
                    trimmed.startswith("mod ") or trimmed.startswith("pub mod ")
                ):
                    if trimmed.startswith("pub mod "):
                        mod_name = trimmed.removeprefix("pub mod ")
                    else:
                        mod_name = trimmed.removeprefix("mod ")
                    mod_name = mod_name.split("{", 1)[0].strip()
                    if mod_name:
                        module_stack.append(mod_name)
                        if "{" in trimmed:
                            module_brace_depth += 1

            matched_kw = None
            if not in_spec:
                for kw in kw_list:
                    if kw in line:
                        matched_kw = kw
                        break

            if not in_spec and matched_kw is not None:
                parts = line.split(matched_kw, 1)
                if len(parts) > 1:
                    bare_fn_name = parts[1].split("(", 1)[0].strip()
                    qualified_name = (
                        bare_fn_name
                        if not module_stack
                        else f"{'::'.join(module_stack)}::{bare_fn_name}"
                    )
                    current_fn = qualified_name
                    in_spec = True
                    current_body = line

                    open_b, close_b = count_non_literal_braces_py(line)
                    brace_count = open_b - close_b
                    if brace_count == 0 and "{" in line:
                        verus_hashes[current_fn] = hashlib.sha256(
                            current_body.encode("utf-8")
                        ).hexdigest()
                        in_spec = False
            elif in_spec:
                current_body += "\n" + line
                open_b, close_b = count_non_literal_braces_py(line)
                brace_count += open_b - close_b
                if brace_count == 0:
                    verus_hashes[current_fn] = hashlib.sha256(
                        current_body.encode("utf-8")
                    ).hexdigest()
                    in_spec = False
            elif not in_spec and module_brace_depth > 0:
                open_b, close_b = count_non_literal_braces_py(line)
                module_brace_depth += open_b
                if close_b > 0:
                    for _ in range(close_b):
                        if module_brace_depth > 0:
                            module_brace_depth -= 1
                            if module_stack:
                                module_stack.pop()

        return verus_hashes

    compute_verus_hashes = compute_verus_hashes_fallback


class CertificateError(Exception):
    """Base class for certificate-related errors."""

    pass


class CertificateJSONError(CertificateError):
    """Raised when a certificate file cannot be parsed as valid JSON."""

    pass


class CertificateValidationError(CertificateError):
    """Raised when a certificate is missing mandatory fields or fails structural validation."""

    pass


def load_and_validate_cert(cert_path):
    """
    Loads and validates an exhaustion certificate from the given path.
    Delegates to the shared Rust native library to ensure 100% schema parity
    and correct cryptographic logic.
    """
    if not _has_verification_lib:
        raise ImportError(
            "Native verification_lib not found. Please build the verification-lib extension (e.g. `maturin develop`)."
        )

    if not os.path.exists(cert_path):
        raise CertificateValidationError(f"Certificate file not found: {cert_path}")

    with open(cert_path, "r", encoding="utf-8") as f:
        cert_str = f.read()

    try:
        # If skip validation is requested, reject it completely
        if (
            "ALLOW_UNVERIFIED_BUILD" in os.environ
            or "UALBF_SKIP_VALIDATION" in os.environ
        ):
            print(
                "Error: Bypass options are deprecated and verification cannot be skipped.",
                file=sys.stderr,
            )
            sys.exit(1)

        # The native library validates the signature and structure
        cert = verification_lib.validate_certificate(cert_str)
    except Exception as e:
        raise CertificateValidationError(f"Validation failed: {e}")

    return cert


CORE_THEOREMS = [
    "UALBF.Engine.CyclotomicGraph.forced_inclusion",
    "UALBF.Engine.CyclotomicGraph.transitive_forced_inclusion",
    "UALBF.Engine.CyclotomicGraph.transitive_reachability_soundness",
    "UALBF.Engine.SieveSoundness.rust_sieve_soundness",
    "UALBF.Engine.Bipartition.prefix_sigma_coprime",
    "UALBF.Engine.Bipartition.ambs_suffix_target",
    "UALBF.Engine.Bipartition.no_solution_no_qpn",
    "UALBF.QPN.AbundancyBound.qpn_abundancy_target",
    "UALBF.QPN.AbundancyBound.qpn_totient_bound",
    "UALBF.QPN.AbundancyBound.abundancy_starvation",
    "UALBF.QPN.Obstruction.legendre_cattaneo_obstruction",
    "UALBF.QPN.BasicProperties.qpn_is_odd_square",
    "UALBF.QPN.PrasadSunitha.qpn_coprime_15_omega_bound",
    "UALBF.Engine.Obstruction.qpn_sigma_mod_3",
    "UALBF.Engine.Obstruction.qpn_sigma_mod_9",
    "UALBF.QPN.TouchardQPN.qpn_sigma_mod_24",
    "UALBF.Engine.TouchardBridge.touchard_bridge",
    "UALBF.FFI.fromU512_toU512",
    "UALBF.FFI.toU512_fromU512",
    "UALBF.FFI.modInverse_spec",
    "UALBF.FFI.U512.w0_mk",
    "UALBF.FFI.U512.w1_mk",
    "UALBF.FFI.U512.w2_mk",
    "UALBF.FFI.U512.w3_mk",
    "UALBF.FFI.U512.w4_mk",
    "UALBF.FFI.U512.w5_mk",
    "UALBF.FFI.U512.w6_mk",
    "UALBF.FFI.U512.w7_mk",
    "UALBF.Pure.ABCConjecture.derive_conjectural_ceiling",
    "UALBF.Pure.ABCConjecture.qpn_conjectural_pruning_sound",
    "UALBF.Engine.Mod1155Bridge.mod_eq_of_mod_eq_of_dvd",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod3",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod5",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod7",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod11",
    "UALBF.Engine.Mod1155Bridge.mod1155_soundness",
    "UALBF.Engine.Mod1155Bridge.ualbf_check_crt_1155_sound",
]


import time_utils


def format_duration(seconds: float, style: str = "short") -> str:
    """Unified duration formatting helper."""
    if seconds < 0:
        return "—"

    d, h, m, s = time_utils.decompose_duration(seconds)
    total_hours = d * 24 + h

    if style == "short":
        if total_hours > 0:
            return f"{total_hours + m/60.0:.1f}h"
        elif m > 0:
            return f"{m + s/60.0:.1f}m"
        else:
            return f"{s}s"
    elif style == "full":
        return f"{total_hours} hours, {m} minutes, {s} seconds"
    return str(seconds)
