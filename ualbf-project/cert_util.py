import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Optional, Union

try:
    from cryptography.hazmat.primitives.asymmetric.ed25519 import (  # type: ignore
        Ed25519PrivateKey,
    )
    from cryptography.hazmat.primitives.serialization import (  # type: ignore
        Encoding,
        PublicFormat,
    )

    _HAS_CRYPTOGRAPHY = True
except ImportError:
    Ed25519PrivateKey = None  # type: ignore
    Encoding = None  # type: ignore
    PublicFormat = None  # type: ignore
    _HAS_CRYPTOGRAPHY = False

import env_util
import hash_util

from matrix_utils import (  # noqa: F401
    exact_det,
    mat_mul,
    gram_schmidt_ortho,
    verify_lll_conditions,
    compute_target_penalty,
)


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
                    verus_hashes[current_fn] = hash_util.hash_string(current_body)
                    in_spec = False
        elif in_spec:
            current_body += "\n" + line
            open_b, close_b = count_non_literal_braces_py(line)
            brace_count += open_b - close_b
            if brace_count == 0:
                verus_hashes[current_fn] = hash_util.hash_string(current_body)
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


_has_verification_lib = True
try:
    import verification_lib  # type: ignore

    hash_tcb = verification_lib.hash_tcb
    hash_extension_tcb = verification_lib.hash_extension_tcb
    check_path_continuity = verification_lib.check_path_continuity
    compute_verus_hashes = verification_lib.compute_verus_hashes
except Exception:
    hash_tcb = None
    hash_extension_tcb = None
    check_path_continuity = None
    _has_verification_lib = False
    compute_verus_hashes = compute_verus_hashes_fallback


CertificateError = hash_util.CertificateError


class CertificateJSONError(CertificateError):
    """Raised when a certificate file cannot be parsed as valid JSON."""

    pass


CertificateValidationError = hash_util.CertificateValidationError


class BoundedJSONLoader:
    """
    Encapsulates JSON deserialization and certificate file reading with
    streaming size limits and AST depth verification to prevent resource exhaustion.
    """

    DEFAULT_MAX_SIZE_BYTES = 10 * 1024 * 1024  # 10 MB
    DEFAULT_MAX_DEPTH = 10

    def __init__(
        self,
        max_size_bytes: int = DEFAULT_MAX_SIZE_BYTES,
        max_depth: int = DEFAULT_MAX_DEPTH,
    ):
        self.max_size_bytes = max_size_bytes
        self.max_depth = max_depth

    def _lexical_prescan_depth(self, payload: str | bytes | bytearray) -> None:
        """
        Performs an O(N) iterative single-pass lexical pre-scan over raw JSON text or bytes
        to calculate container nesting depth prior to invoking json.loads.
        Raises CertificateValidationError immediately if peak nesting depth exceeds self.max_depth.
        """
        current_depth = 0
        in_string = False
        is_escaped = False
        max_depth = self.max_depth

        if isinstance(payload, (bytes, bytearray)):
            for b in payload:
                if in_string:
                    if is_escaped:
                        is_escaped = False
                    elif b == 92:  # \
                        is_escaped = True
                    elif b == 34:  # "
                        in_string = False
                else:
                    if b == 34:  # "
                        in_string = True
                    elif b == 123 or b == 91:  # { or [
                        current_depth += 1
                        if current_depth > max_depth:
                            raise CertificateValidationError(
                                f"JSON container nesting depth ({current_depth}) exceeds maximum allowed limit of {max_depth} levels."
                            )
                    elif b == 125 or b == 93:  # } or ]
                        if current_depth > 0:
                            current_depth -= 1
        elif isinstance(payload, str):
            for ch in payload:
                if in_string:
                    if is_escaped:
                        is_escaped = False
                    elif ch == "\\":
                        is_escaped = True
                    elif ch == '"':
                        in_string = False
                else:
                    if ch == '"':
                        in_string = True
                    elif ch == "{" or ch == "[":
                        current_depth += 1
                        if current_depth > max_depth:
                            raise CertificateValidationError(
                                f"JSON container nesting depth ({current_depth}) exceeds maximum allowed limit of {max_depth} levels."
                            )
                    elif ch == "}" or ch == "]":
                        if current_depth > 0:
                            current_depth -= 1
        else:
            raise CertificateValidationError(
                f"Invalid JSON input type: {type(payload)}"
            )

    def _verify_depth(self, obj: Any, current_depth: int = 0) -> None:
        if current_depth > self.max_depth:
            raise CertificateValidationError(
                f"JSON object nesting depth ({current_depth}) exceeds maximum allowed limit of {self.max_depth} levels."
            )
        if isinstance(obj, dict):
            for v in obj.values():
                self._verify_depth(v, current_depth + 1)
        elif isinstance(obj, list):
            for item in obj:
                self._verify_depth(item, current_depth + 1)

    def read_file_bytes(self, filepath_or_file: Any, chunk_size: int = 65536) -> bytes:
        if isinstance(filepath_or_file, (str, bytes, os.PathLike)):
            if not os.path.exists(filepath_or_file):
                path_str = (
                    filepath_or_file.decode("utf-8", errors="replace")
                    if isinstance(filepath_or_file, bytes)
                    else os.fspath(filepath_or_file)
                )
                raise CertificateValidationError(f"File not found: {path_str}")
            with open(filepath_or_file, "rb") as f:
                return self._read_stream_bytes(f, chunk_size=chunk_size)
        elif hasattr(filepath_or_file, "read"):
            return self._read_stream_bytes(filepath_or_file, chunk_size=chunk_size)
        else:
            raise CertificateValidationError(
                f"Invalid file source: {type(filepath_or_file)}"
            )

    def _read_stream_bytes(self, fp: Any, chunk_size: int = 65536) -> bytes:
        chunks = []
        total_size = 0
        while True:
            chunk = fp.read(chunk_size)
            if not chunk:
                break
            if isinstance(chunk, str):
                chunk = chunk.encode("utf-8")
            total_size += len(chunk)
            if total_size > self.max_size_bytes:
                raise CertificateValidationError(
                    f"File payload size ({total_size} bytes) exceeds maximum allowed limit of {self.max_size_bytes} bytes."
                )
            chunks.append(chunk)
        return b"".join(chunks)

    def read_file_text(
        self, filepath_or_file: Any, encoding: str = "utf-8", chunk_size: int = 65536
    ) -> str:
        data_bytes = self.read_file_bytes(filepath_or_file, chunk_size=chunk_size)
        try:
            return data_bytes.decode(encoding)
        except UnicodeDecodeError as e:
            raise CertificateValidationError(
                f"Invalid encoding ({encoding}) in file: {e}"
            )

    def loads(self, s: str | bytes | bytearray, **kwargs: Any) -> Any:
        encoding = kwargs.pop("encoding", "utf-8")
        if isinstance(s, (bytes, bytearray)):
            byte_len = len(s)
            text = s.decode(encoding)
        elif isinstance(s, str):
            byte_len = len(s.encode("utf-8"))
            text = s
        else:
            raise CertificateValidationError(f"Invalid JSON input type: {type(s)}")

        if byte_len > self.max_size_bytes:
            raise CertificateValidationError(
                f"JSON payload size ({byte_len} bytes) exceeds maximum allowed limit of {self.max_size_bytes} bytes."
            )

        self._lexical_prescan_depth(s)

        try:
            data = json.loads(text, **kwargs)
        except json.JSONDecodeError as e:
            raise CertificateValidationError(f"Invalid JSON payload: {e}")
        except CertificateValidationError:
            raise
        except Exception as e:
            raise CertificateValidationError(f"Failed to parse JSON payload: {e}")

        self._verify_depth(data, current_depth=0)
        return data

    def load(self, fp: Any, encoding: str = "utf-8", **kwargs: Any) -> Any:
        text = self.read_file_text(fp, encoding=encoding)
        return self.loads(text, **kwargs)

    def load_file(self, filepath: Any, encoding: str = "utf-8", **kwargs: Any) -> Any:
        return self.load(filepath, encoding=encoding, **kwargs)


def validate_sidecar_schema(sidecar_path: str) -> None:
    """
    Streams and validates line-by-line schema and numerical integrity for an overflow sidecar log.

    Each non-empty line must parse as a JSON object matching SearchEvent::Overflow schema:
    {"event": "overflow", "p": "<decimal string>", "pow": <non-negative integer>}

    Raises:
        CertificateValidationError: If any line is malformed, missing required fields,
                                    contains unexpected fields, or has incorrect types.
    """
    if not os.path.exists(sidecar_path):
        raise CertificateValidationError(f"Sidecar log file not found: {sidecar_path}")

    try:
        with open(sidecar_path, "r", encoding="utf-8") as f:
            for line_num, line in enumerate(f, start=1):
                line_str = line.strip()
                if not line_str:
                    continue
                try:
                    record = json.loads(line_str)
                except json.JSONDecodeError as e:
                    raise CertificateValidationError(
                        f"Line {line_num}: invalid JSON syntax in record '{line_str}': {e}"
                    )
                if not isinstance(record, dict):
                    raise CertificateValidationError(
                        f"Line {line_num}: record is not a JSON object: '{line_str}'"
                    )

                expected_keys = {"event", "p", "pow"}
                actual_keys = set(record.keys())
                if actual_keys != expected_keys:
                    missing = expected_keys - actual_keys
                    extra = actual_keys - expected_keys
                    err_msg = (
                        f"Line {line_num}: schema mismatch in record '{line_str}'."
                    )
                    if missing:
                        err_msg += f" Missing required key(s): {sorted(missing)}."
                    if extra:
                        err_msg += f" Unexpected extra key(s): {sorted(extra)}."
                    raise CertificateValidationError(err_msg)

                if record["event"] != "overflow":
                    raise CertificateValidationError(
                        f"Line {line_num}: invalid 'event' value in record '{line_str}' (expected 'overflow', got '{record['event']}')"
                    )

                if not isinstance(record["p"], str) or not record["p"].isdigit():
                    raise CertificateValidationError(
                        f"Line {line_num}: invalid 'p' field value in record '{line_str}' (expected decimal string)"
                    )

                if (
                    not isinstance(record["pow"], int)
                    or isinstance(record["pow"], bool)
                    or record["pow"] < 0
                ):
                    raise CertificateValidationError(
                        f"Line {line_num}: invalid 'pow' field value in record '{line_str}' (expected non-negative integer)"
                    )
    except UnicodeDecodeError as e:
        raise CertificateValidationError(
            f"Sidecar log contains invalid UTF-8 encoding: {e}"
        )


DEFAULT_MAX_CERT_SIZE_MB = 10.0


def get_max_cert_size_bytes() -> int:
    """Returns the maximum allowed certificate file size in bytes based on UALBF_MAX_CERT_SIZE_MB."""
    try:
        val = env_util.get_env_var("UALBF_MAX_CERT_SIZE_MB", DEFAULT_MAX_CERT_SIZE_MB)
        if isinstance(val, (int, float)) and val > 0:
            return int(val * 1024 * 1024)
    except (ValueError, TypeError):
        pass
    return int(DEFAULT_MAX_CERT_SIZE_MB * 1024 * 1024)


def validate_file_size(file_path: str, max_bytes: Optional[int] = None) -> None:
    """Validates that the given file size does not exceed max_bytes prior to reading."""
    if max_bytes is None:
        max_bytes = get_max_cert_size_bytes()

    if os.path.exists(file_path):
        file_size = os.path.getsize(file_path)
        if file_size > max_bytes:
            actual_mb = file_size / (1024 * 1024)
            max_mb = max_bytes / (1024 * 1024)
            raise CertificateValidationError(
                f"File size of '{file_path}' ({actual_mb:.2f} MB / {file_size} bytes) "
                f"exceeds maximum allowed limit of {max_mb:.2f} MB ({max_bytes} bytes)."
            )


def load_and_validate_cert(cert_path, trusted_public_key=None):
    """
    Loads and validates an exhaustion certificate from the given path.
    Delegates to the shared Rust native library to ensure 100% schema parity
    and correct cryptographic logic.
    """
    if not os.path.exists(cert_path):
        raise CertificateValidationError(f"Certificate file not found: {cert_path}")

    validate_file_size(cert_path)

    if not _has_verification_lib:
        raise ImportError(
            "Native verification_lib not found. Please build the verification-lib extension (e.g. `maturin develop`)."
        )

    trusted_key = trusted_public_key or env_util.get_env_var("UALBF_TRUSTED_PUBLIC_KEY")
    if not trusted_key or not trusted_key.strip():
        print(
            "ERROR: No trusted public key is pinned (UALBF_TRUSTED_PUBLIC_KEY not set).",
            file=sys.stderr,
        )
        raise CertificateValidationError(
            "ERROR: No trusted public key is pinned (UALBF_TRUSTED_PUBLIC_KEY not set)."
        )

    loader = BoundedJSONLoader()
    cert_str = loader.read_file_text(cert_path)

    try:
        # Halt execution on deprecated flags
        env_util.check_deprecated_env_vars()

        # The native library validates the signature, key, and structure
        cert = verification_lib.validate_certificate(cert_str, trusted_key.strip())
    except Exception as e:
        raise CertificateValidationError(f"Validation failed: {e}")

    return cert


CORE_THEOREMS = [
    "UALBF.Engine.ruleA_safe",
    "UALBF.Engine.ruleB_safe",
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
    "UALBF.QPN.PrasadSunitha.qpn_div_5_coprime_3_omega_bound",
    "UALBF.Engine.Obstruction.qpn_sigma_mod_3",
    "UALBF.Engine.Obstruction.qpn_sigma_mod_9",
    "UALBF.QPN.TouchardQPN.qpn_sigma_mod_24",
    "UALBF.Engine.TouchardBridge.touchard_bridge",
    "UALBF.FFI.fromU512_toU512",
    "UALBF.FFI.toU512_fromU512",
    "UALBF.FFI.fromU512_eq_fromU512Fast",
    "UALBF.FFI.modInverse_spec",
    "UALBF.FFI.ualbf_mod_inverse_ok_limbs_eq",
    "UALBF.FFI.ualbf_mod_inverse_limb_eq",
    "UALBF.FFI.ualbf_check_crt_1155_limbs_eq",
    "UALBF.FFI.U512.w0_mk",
    "UALBF.FFI.U512.w1_mk",
    "UALBF.FFI.U512.w2_mk",
    "UALBF.FFI.U512.w3_mk",
    "UALBF.FFI.U512.w4_mk",
    "UALBF.FFI.U512.w5_mk",
    "UALBF.FFI.U512.w6_mk",
    "UALBF.FFI.U512.w7_mk",
    "UALBF.Pure.ABCConjecture.derive_conjectural_ceiling",
    "UALBF.Pure.ABCConjecture.conjectural_ceiling_size_exclusion",
    "UALBF.Engine.Mod1155Bridge.mod_eq_of_mod_eq_of_dvd",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod3",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod5",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod7",
    "UALBF.Engine.Mod1155Bridge.mod1155_to_mod11",
    "UALBF.Engine.Mod1155Bridge.mod1155_soundness",
    "UALBF.Engine.Mod1155Bridge.ualbf_check_crt_1155_sound",
    "UALBF.Fixed64.scaleBoundCeil_conservative",
    "UALBF.Engine.SieveSoundness.rust_sieve_soundness_mod_5",
    "UALBF.Engine.Mod5Bridge.ualbf_check_mod_5_soundness_ffi",
    "UALBF.Engine.SieveSoundness.ModularSieve",
    "UALBF.QPN.AbundancyBound.lean_abundancy_starvation_theorem",
    "UALBF.QPN.AbundancyBound.verify_starvation_pruning",
    "UALBF.QPN.AbundancyBound.is_starved_bound",
    "UALBF.Engine.CyclotomicGraph.is_cdg_forced_pruned",
    "UALBF.Engine.Bipartition.coprime_multiplicative_nonlinear",
    "UALBF.Engine.Bipartition.coprime_multiplicative",
    "UALBF.Engine.Bipartition.disjoint_by_construction",
    "UALBF.QPN.PrasadSunitha.verify_prasad_sunitha",
    "UALBF.QPN.TouchardQPN.screen_mod_8",
    "UALBF.QPN.Obstruction.is_valid_mod_8",
    "UALBF.Engine.SieveSoundness.passes_raycast_sieve_spec",
    "UALBF.Engine.SieveSoundness.verified_passes_raycast_sieve",
    "UALBF.Engine.CyclotomicGraph.zsigmondy_preconditions_satisfied",
    "UALBF.Engine.CyclotomicGraph.proof_verify_zsigmondy_preconditions",
    "UALBF.Pure.Arithmetic.lemma_composite_has_prime_factor_le_sqrt",
    "UALBF.Pure.Arithmetic.lemma_smallest_factor_is_prime",
    "UALBF.Pure.Arithmetic.lemma_modpow_mod_divisibility",
    "UALBF.Pure.Arithmetic.lemma_modpow_add_mul",
    "UALBF.Pure.Arithmetic.lemma_order_exists",
    "UALBF.Pure.Arithmetic.lemma_order_prime_factor",
    "UALBF.Pure.Arithmetic.lemma_divisibility_bounds",
    "UALBF.Pure.Arithmetic.lemma_fermat_little_theorem",
    "UALBF.Pure.Arithmetic.lemma_order_le_p_minus_1",
    "UALBF.Pure.Arithmetic.lemma_square_comparison_contradiction",
    "UALBF.Pure.Arithmetic.lemma_f_squared_gt_n_minus_1",
    "UALBF.Pure.Arithmetic.lemma_pocklington_certificate",
    "UALBF.Pure.Arithmetic.lemma_divisibility_transitive",
]

# No Lean axioms are whitelisted: any `axiom` reached by a core theorem fails the audit.
ALLOWED_AXIOMS: set[str] = set()


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


def verify_manifest_chain(manifest: dict, manifest_path: str, bounds_path: str) -> None:
    """
    Validates proof manifest hashes and bounds manifest hashes in a single call.

    Raises CertificateValidationError if any part of the chain of trust fails validation.
    """
    if not os.path.exists(manifest_path):
        raise CertificateValidationError(
            f"Proof manifest '{manifest_path}' not found, cannot verify chain of trust."
        )

    computed_manifest_hash = hash_util.hash_file_bounded(manifest_path)
    expected_manifest_hash = manifest.get("manifest_hash")
    if expected_manifest_hash and computed_manifest_hash != expected_manifest_hash:
        raise CertificateValidationError(
            f"Manifest hash mismatch!\nExpected: {expected_manifest_hash}\nGot:      {computed_manifest_hash}"
        )

    manifest_data = BoundedJSONLoader().load_file(manifest_path)
    expected_bounds_hash = manifest_data.get("bounds_manifest_hash")
    if not expected_bounds_hash:
        raise CertificateValidationError(
            "Proof manifest does not contain bounds_manifest_hash"
        )

    if not os.path.exists(bounds_path):
        raise CertificateValidationError(
            f"Bounds manifest '{bounds_path}' not found but hash is specified in proof manifest."
        )

    computed_bounds_hash = hash_util.hash_file_bounded(bounds_path)
    if computed_bounds_hash != expected_bounds_hash:
        raise CertificateValidationError(
            f"Bounds manifest hash mismatch!\nExpected: {expected_bounds_hash}\nGot:      {computed_bounds_hash}"
        )


def get_verus_proof_hashes(rust_src_dir: Union[str, Path]) -> dict[str, str]:
    """
    Scans Rust source files (verus_proofs.rs, lean_export.rs) in rust_src_dir
    and returns computed function digest maps.
    """
    verus_hashes: dict[str, str] = {}
    rust_src_path = Path(rust_src_dir)
    loader = BoundedJSONLoader()
    for verus_file in ["verus_proofs.rs", "lean_export.rs"]:
        file_path = rust_src_path / verus_file
        if file_path.is_file():
            rf_text = loader.read_file_text(file_path)
            verus_hashes.update(compute_verus_hashes(rf_text))
    return dict(sorted(verus_hashes.items()))


def verify_theorem_checksum(
    thm: dict,
    manifest_path: Optional[str] = None,
    allow_missing_sources: bool = False,
) -> bool:
    """
    Compute and verify the checksum for a single theorem entry.
    The checksum is computed using the physical file content hash bounded by size limits.
    If the physical proof file is missing, returns False unless metadata fallback
    is explicitly enabled via allow_missing_sources parameter or UALBF_ALLOW_MISSING_SOURCES=1.
    """
    base_dir = os.path.dirname(os.path.abspath(__file__))
    file_path = os.path.join(base_dir, "lean4-proofs", thm["file"])

    if not os.path.exists(file_path) and manifest_path:
        manifest_dir = os.path.dirname(os.path.abspath(manifest_path))
        file_path = os.path.join(manifest_dir, "lean4-proofs", thm["file"])
        if not os.path.exists(file_path):
            file_path = os.path.join(manifest_dir, thm["file"])

    if not os.path.exists(file_path):
        file_path = os.path.join("lean4-proofs", thm["file"])

    if os.path.exists(file_path):
        computed = hash_util.hash_file_bounded(file_path)
        return computed == thm.get("checksum", "")
    else:
        explicit_fallback = allow_missing_sources or env_util.get_env_var(
            "UALBF_ALLOW_MISSING_SOURCES", False
        )
        if explicit_fallback:
            computed = hash_util.hash_theorem_metadata(
                thm["name"], thm["file"], thm["status"]
            )
            return computed == thm.get("checksum", "")
        else:
            return False


def create_signed_test_cert(
    manifest_path: str,
    bounds_path: Optional[str] = None,
    extra_telemetry: Optional[dict] = None,
    commit_hash: Optional[str] = None,
) -> tuple[dict, str]:
    """
    Creates an Ed25519-signed test certificate for testing and CI paper sync verification.
    Returns (cert_dict, public_key_hex).
    """
    mbytes = BoundedJSONLoader().read_file_bytes(manifest_path)
    manifest_hash = hash_util.hash_bytes(mbytes)

    project_root = os.path.dirname(os.path.abspath(__file__))
    verified_logic_hash = (
        hash_tcb(project_root) if (_has_verification_lib and hash_tcb) else "0" * 64
    )

    if commit_hash is None:
        try:
            commit_hash = subprocess.check_output(
                ["git", "rev-parse", "HEAD"],
                cwd=project_root,
                text=True,
                stderr=subprocess.DEVNULL,
            ).strip()
        except Exception:
            commit_hash = "unknown"

    if bounds_path is None:
        default_bounds = os.path.join(project_root, "bounds_manifest.json")
        if os.path.exists(default_bounds):
            bounds_path = default_bounds

    target_min = 37
    target_max = 43
    if bounds_path and os.path.exists(bounds_path):
        try:
            bdata = BoundedJSONLoader().load_file(bounds_path)
            sb = bdata.get("search_bounds", {})
            if "target_min_log10" in sb and isinstance(sb["target_min_log10"], dict):
                target_min = sb["target_min_log10"].get("value", target_min)
            if "target_max_log10" in sb and isinstance(sb["target_max_log10"], dict):
                target_max = sb["target_max_log10"].get("value", target_max)
        except Exception:
            pass

    tel = {
        "phase1_execution_time_ms": 100,
        "phase2_execution_time_ms": 5000,
        "total_branches_searched": 1000,
        "abundance_pruned": 200,
        "raycast_pruned": 0,
        "target_min_log10": target_min,
        "target_max_log10": target_max,
    }
    if extra_telemetry:
        tel.update(extra_telemetry)

    map_obj = {
        "manifest_hash": manifest_hash,
        "verified_logic_hash": verified_logic_hash,
        "total_branches_searched": tel["total_branches_searched"],
        "target_min_log10": tel["target_min_log10"],
        "target_max_log10": tel["target_max_log10"],
        "trace_hash": tel.get("trace_hash", ""),
        "factorization_depth": tel.get("factorization_depth", 0),
    }
    if "path_ranges" in tel:
        map_obj["path_ranges"] = tel["path_ranges"]

    if not _HAS_CRYPTOGRAPHY or Ed25519PrivateKey is None:
        raise ImportError(
            "cryptography package is required for creating signed test certificates."
        )

    priv = Ed25519PrivateKey.generate()
    pub_hex = priv.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw).hex()  # type: ignore[arg-type]
    payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
    sig_hex = priv.sign(payload.encode("utf-8")).hex()

    cert_data = {
        "manifest_hash": manifest_hash,
        "verified_logic_hash": verified_logic_hash,
        "public_key": pub_hex,
        "signature": sig_hex,
        "engine_version": "1.0.0",
        "commit_hash": commit_hash,
        "telemetry": tel,
    }

    return cert_data, pub_hex


def verify_meta_certificate_envelope(
    meta_cert_data: dict,
    manifest_path: str,
    verified_leaf_certs: list[dict],
) -> dict:
    """
    Validates top-level meta-certificate envelope integrity, manifest hash binding,
    aggregated signatures matching verified leaf certificates, and re-aggregated telemetry.

    Parameters:
        meta_cert_data (dict): Parsed meta-certificate payload dictionary.
        manifest_path (str): Path to the proof manifest file.
        verified_leaf_certs (list[dict]): List of verified leaf certificate dictionaries in exact order.

    Returns:
        dict: The verified meta_cert_data dictionary.

    Raises:
        CertificateValidationError: On any schema, manifest hash, node count, signature array,
                                    or re-aggregated telemetry mismatch.
    """
    env_util.check_deprecated_env_vars()

    if not isinstance(meta_cert_data, dict):
        raise CertificateValidationError("Meta-certificate data must be a dictionary.")

    # 1. Top-level schema validation
    required_keys = {
        "meta_manifest_hash": str,
        "aggregated_signatures": list,
        "telemetry": dict,
        "total_nodes": int,
    }

    for key, expected_type in required_keys.items():
        if key not in meta_cert_data:
            raise CertificateValidationError(
                f"Meta-certificate missing required top-level key '{key}'."
            )
        val = meta_cert_data[key]
        if expected_type is int and isinstance(val, bool):
            raise CertificateValidationError(
                f"Meta-certificate top-level key '{key}' must be an integer, got bool."
            )
        if not isinstance(val, expected_type):
            raise CertificateValidationError(
                f"Meta-certificate top-level key '{key}' must be of type {expected_type.__name__}, got {type(val).__name__}."
            )

    # 2. Top-level manifest hash validation
    if not os.path.exists(manifest_path):
        raise CertificateValidationError(
            f"Proof manifest file not found: '{manifest_path}'"
        )

    computed_manifest_hash = hash_util.hash_file_bounded(manifest_path)
    if meta_cert_data["meta_manifest_hash"] != computed_manifest_hash:
        raise CertificateValidationError(
            f"Top-level meta_manifest_hash mismatch!\nExpected: {computed_manifest_hash}\nGot:      {meta_cert_data['meta_manifest_hash']}"
        )

    # 3. Total nodes validation
    if not isinstance(verified_leaf_certs, list):
        raise CertificateValidationError(
            "verified_leaf_certs parameter must be a list."
        )

    if len(verified_leaf_certs) == 0:
        raise CertificateValidationError(
            "Cannot verify meta-certificate envelope with empty leaf certificate array."
        )

    if meta_cert_data["total_nodes"] != len(verified_leaf_certs):
        raise CertificateValidationError(
            f"Meta-certificate total_nodes ({meta_cert_data['total_nodes']}) does not match "
            f"verified leaf certificate count ({len(verified_leaf_certs)})."
        )

    # 4. Aggregated signatures validation
    aggregated_sigs = meta_cert_data["aggregated_signatures"]
    if len(aggregated_sigs) != len(verified_leaf_certs):
        raise CertificateValidationError(
            f"Meta-certificate aggregated_signatures length ({len(aggregated_sigs)}) "
            f"does not match verified leaf certificate count ({len(verified_leaf_certs)})."
        )

    for idx, (sig, leaf) in enumerate(zip(aggregated_sigs, verified_leaf_certs)):
        if not isinstance(sig, str):
            raise CertificateValidationError(
                f"Aggregated signature at index {idx} must be a string."
            )
        if not isinstance(leaf, dict):
            raise CertificateValidationError(
                f"Leaf certificate at index {idx} must be a dictionary."
            )
        leaf_sig = leaf.get("signature")
        if not leaf_sig or sig != leaf_sig:
            raise CertificateValidationError(
                f"Aggregated signature at index {idx} does not match leaf certificate signature.\n"
                f"Expected: {leaf_sig}\n"
                f"Got:      {sig}"
            )

    # 5. Leaf telemetry re-aggregation validation
    for idx, leaf in enumerate(verified_leaf_certs):
        if "telemetry" not in leaf or not isinstance(leaf["telemetry"], dict):
            raise CertificateValidationError(
                f"Leaf certificate at index {idx} is missing a valid 'telemetry' dictionary."
            )

    leaf_tels = [leaf["telemetry"] for leaf in verified_leaf_certs]

    sum_fields = [
        "total_branches_searched",
        "abundance_pruned",
        "raycast_pruned",
        "phase2_execution_time_ms",
        "total_execution_time_ms",
        "math_interruptions",
    ]

    for f in sum_fields:
        for idx, t in enumerate(leaf_tels):
            if f not in t:
                raise CertificateValidationError(
                    f"Leaf certificate telemetry at index {idx} is missing required field '{f}'."
                )

    if "target_min_log10" not in leaf_tels[0]:
        raise CertificateValidationError(
            "Leaf certificate telemetry at index 0 is missing 'target_min_log10'."
        )
    if "target_max_log10" not in leaf_tels[-1]:
        raise CertificateValidationError(
            "Leaf certificate telemetry at last index is missing 'target_max_log10'."
        )

    expected_telemetry = {
        "target_min_log10": leaf_tels[0]["target_min_log10"],
        "target_max_log10": leaf_tels[-1]["target_max_log10"],
    }
    for f in sum_fields:
        expected_telemetry[f] = sum(t[f] for t in leaf_tels)

    if expected_telemetry.get("math_interruptions", 0) > 0:
        raise CertificateValidationError(
            f"Meta-certificate envelope contains non-zero math_interruptions ({expected_telemetry['math_interruptions']})."
        )

    top_telemetry = meta_cert_data["telemetry"]
    for field, expected_val in expected_telemetry.items():
        if field not in top_telemetry:
            raise CertificateValidationError(
                f"Top-level telemetry missing required field '{field}'."
            )
        actual_val = top_telemetry[field]
        if actual_val != expected_val:
            raise CertificateValidationError(
                f"Top-level telemetry field '{field}' mismatch!\n"
                f"Expected re-aggregated value: {expected_val}\n"
                f"Got:                          {actual_val}"
            )

    return meta_cert_data
