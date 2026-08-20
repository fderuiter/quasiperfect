"""
Tests for fully synchronized dynamic proof-mode guardrails and conjectural FFI bounds.
"""

import os
import re
import pytest

_ABS_FILE = os.path.abspath(__file__)
_DIR_NAME = os.path.dirname(_ABS_FILE)
_PROJECT_DIR = os.path.dirname(_DIR_NAME)


def test_lean_exports_conjectural_ffi():
    """
    Verify that Lean's FFI exports define conjectural parameters and set the 31st verified-bit.
    """
    ffi_path = os.path.join(_PROJECT_DIR, "lean4-proofs", "UALBF", "FFI.lean")
    assert os.path.exists(ffi_path), f"FFI.lean not found at {ffi_path}"

    with open(ffi_path, "r", encoding="utf-8") as f:
        content = f.read()

    # Check for exported ualbf_conjectural_active
    active_match = re.search(r"@\[export ualbf_conjectural_active\]", content)
    assert active_match is not None, "ualbf_conjectural_active not exported in FFI.lean"

    # Check for exported ualbf_conjectural_max_log10_ceiling
    ceiling_match = re.search(
        r"@\[export ualbf_conjectural_max_log10_ceiling\]", content
    )
    assert (
        ceiling_match is not None
    ), "ualbf_conjectural_max_log10_ceiling not exported in FFI.lean"

    # Check that verified-bit (1 <<< 31) is applied
    bit_check = re.findall(r"\(1\s*:\s*UInt32\)\s*<<<\s*31", content)
    assert (
        len(bit_check) >= 2
    ), "Verified-bit signature (1 <<< 31) not found for conjectural parameters"


def test_rust_ffi_getters_and_guardrails():
    """
    Verify that the Rust engine's lean_ffi.rs correctly binds conjectural getters,
    applies the verified-bit check, and strips the guardrail bit.
    """
    lean_ffi_path = os.path.join(_PROJECT_DIR, "rust-engine", "src", "lean_ffi.rs")
    assert os.path.exists(lean_ffi_path), f"lean_ffi.rs not found at {lean_ffi_path}"

    with open(lean_ffi_path, "r", encoding="utf-8") as f:
        content = f.read()

    # Verify presence of get_conjectural_active and check_verified_bit usage
    assert "pub fn get_conjectural_active()" in content
    assert 'check_verified_bit(val as u64, 31, "get_conjectural_active")' in content
    assert "val & !(1 << 31)" in content

    # Verify presence of get_conjectural_max_log10_ceiling and check_verified_bit usage
    assert "pub fn get_conjectural_max_log10_ceiling()" in content
    assert (
        'check_verified_bit(val as u64, 31, "get_conjectural_max_log10_ceiling")'
        in content
    )


def test_pure_proof_mode_dynamic_fallback():
    """
    Verify that the is_conjectural_active() helper function dynamically disables
    conjectural bounds under pure proof mode.
    """
    lean_ffi_path = os.path.join(_PROJECT_DIR, "rust-engine", "src", "lean_ffi.rs")
    with open(lean_ffi_path, "r", encoding="utf-8") as f:
        content = f.read()

    # Verify that is_conjectural_active checks the proof mode and falls back dynamically
    assert "pub fn is_conjectural_active()" in content
    assert 'crate::policy::get_proof_mode() == "pure"' in content
