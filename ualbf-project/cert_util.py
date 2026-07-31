import json
import os

import sys

_has_verification_lib = True
try:
    import verification_lib  # type: ignore

    hash_tcb = verification_lib.hash_tcb
    hash_extension_tcb = verification_lib.hash_extension_tcb
except ImportError:
    hash_tcb = None
    hash_extension_tcb = None
    _has_verification_lib = False


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
        validated_str = verification_lib.validate_certificate(cert_str)
        cert = json.loads(validated_str)
    except Exception as e:
        raise CertificateValidationError(f"Validation failed: {e}")

    return cert


CORE_THEOREMS = [
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
