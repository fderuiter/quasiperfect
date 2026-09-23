import json
import os
import sys
from typing import Any, Optional

import hash_util

from matrix_utils import (  # noqa: F401
    exact_det,
    mat_mul,
    gram_schmidt_ortho,
    verify_lll_conditions,
    compute_target_penalty,
)

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
        if isinstance(s, (bytes, bytearray)):
            byte_len = len(s)
            text = s.decode(kwargs.pop("encoding", "utf-8"))
        elif isinstance(s, str):
            byte_len = len(s.encode("utf-8"))
            text = s
        else:
            raise CertificateValidationError(f"Invalid JSON input type: {type(s)}")

        if byte_len > self.max_size_bytes:
            raise CertificateValidationError(
                f"JSON payload size ({byte_len} bytes) exceeds maximum allowed limit of {self.max_size_bytes} bytes."
            )

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
    env_val = os.getenv("UALBF_MAX_CERT_SIZE_MB")
    if env_val:
        try:
            val = float(env_val.strip())
            if val > 0:
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

    trusted_key = trusted_public_key or os.getenv("UALBF_TRUSTED_PUBLIC_KEY", None)
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

        # The native library validates the signature, key, and structure
        cert = verification_lib.validate_certificate(cert_str, trusted_key.strip())
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
    "UALBF.Engine.Bipartition.root_partition_complete_coverage",
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
    "UALBF.Fixed64.scaleBoundCeil_conservative",
    "UALBF.Engine.SieveSoundness.rust_sieve_soundness_mod_5",
    "UALBF.Engine.Mod5Bridge.ualbf_check_mod_5_soundness_ffi",
    "UALBF.Engine.SieveSoundness.ModularSieve",
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
