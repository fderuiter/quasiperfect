import os
import shutil
import json
import hashlib
import subprocess
from pathlib import Path
import pytest


def test_ffi_automation_dynamic_generation():
    """
    Test that modifying the bit width of the dynamic boundary type (U512)
    in schema_manifest.json and running export_lean_specs.py dynamically
    regenerates Rust/Lean getters, constructor, and types with the correct
    limb count.
    """
    project_dir = Path(__file__).parent.parent
    schema_path = project_dir / "schema_manifest.json"
    ffi_generated_rs = project_dir / "rust-engine/src/ffi_generated.rs"
    schema_generated_rs = project_dir / "rust-engine/src/schema_generated.rs"
    ffi_generated_lean = project_dir / "lean4-proofs/UALBF/FFI_generated.lean"

    # Backup original schema manifest and generated files
    schema_backup = schema_path.read_text(encoding="utf-8")
    rs_backup = ffi_generated_rs.read_text(encoding="utf-8")
    schema_gen_backup = schema_generated_rs.read_text(encoding="utf-8")
    lean_backup = ffi_generated_lean.read_text(encoding="utf-8")

    try:
        # 1. Modify schema_manifest to specify a 256-bit boundary type (4 limbs)
        schema_data = json.loads(schema_backup)
        schema_data["U512"] = {"bit_width": 256, "limb_width": 64}
        schema_path.write_text(json.dumps(schema_data, indent=2), encoding="utf-8")

        # Run generator
        res = subprocess.run(
            ["python3", "scripts/export_lean_specs.py"],
            cwd=str(project_dir),
            capture_output=True,
            text=True,
        )
        assert res.returncode == 0, f"Generator failed: {res.stderr}"

        # Verify generated Rust file contains LIMB_COUNT = 4 and exactly 4 getters
        rs_content = ffi_generated_rs.read_text(encoding="utf-8")
        assert "pub const LIMB_COUNT: usize = 4;" in rs_content
        assert "pub type U512Data = [u64; 4];" in rs_content
        assert "rust_u512_get_w0" in rs_content
        assert "rust_u512_get_w3" in rs_content
        assert "rust_u512_get_w4" not in rs_content
        # Check constructor takes exactly 4 arguments
        assert (
            'pub extern "C" fn rust_u512_mk(\n    w0: u64,\n    w1: u64,\n    w2: u64,\n    w3: u64,\n)'
            in rs_content
        )

        # Verify generated Lean file contains exactly 4 getters
        lean_content = ffi_generated_lean.read_text(encoding="utf-8")
        assert "def U512.mk (w0 w1 w2 w3 : UInt64)" in lean_content
        assert "rust_u512_get_w0" in lean_content
        assert "rust_u512_get_w3" in lean_content
        assert "rust_u512_get_w4" not in lean_content

    finally:
        # Restore backups
        schema_path.write_text(schema_backup, encoding="utf-8")
        ffi_generated_rs.write_text(rs_backup, encoding="utf-8")
        schema_generated_rs.write_text(schema_gen_backup, encoding="utf-8")
        ffi_generated_lean.write_text(lean_backup, encoding="utf-8")


def test_ffi_automation_out_of_sync_fails_cargo():
    """
    Test that if schema_manifest.json is manually changed but the files
    are not regenerated, Cargo compilation fails with our guardrail message.
    """
    project_dir = Path(__file__).parent.parent
    schema_path = project_dir / "schema_manifest.json"

    # Backup original schema manifest
    schema_backup = schema_path.read_text(encoding="utf-8")

    try:
        # 1. Modify schema_manifest to create mismatch
        schema_data = json.loads(schema_backup)
        schema_data["U512"]["bit_width"] = 1024
        schema_path.write_text(json.dumps(schema_data, indent=2), encoding="utf-8")

        # Touch build.rs to force cargo to rerun it
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()

        # Run cargo check in rust-engine
        env = os.environ.copy()
        res = subprocess.run(
            ["cargo", "check"],
            cwd=str(project_dir / "rust-engine"),
            env=env,
            capture_output=True,
            text=True,
        )

        # Build must fail because schema manifest is out of sync
        assert res.returncode != 0
        assert (
            "FATAL: Schema Manifest Synchronization Guardrail Triggered!" in res.stderr
        )

    finally:
        # Restore backup
        schema_path.write_text(schema_backup, encoding="utf-8")


def test_ffi_naming_validation():
    """
    Test that missing or invalid ualbf_check_crt_1155 export
    causes export_lean_specs.py to fail with explicit errors.
    """
    project_dir = Path(__file__).parent.parent
    ffi_lean_path = project_dir / "lean4-proofs/UALBF/FFI.lean"
    
    # Backup original FFI.lean
    ffi_backup = ffi_lean_path.read_text(encoding="utf-8")
    
    try:
        # Case 1: Corrupted/Missing Export Name
        corrupted_content_1 = ffi_backup.replace(
            "@[export ualbf_check_crt_1155]",
            "@[export ualbf_check_crt_1155_mismatched]"
        )
        ffi_lean_path.write_text(corrupted_content_1, encoding="utf-8")
        
        res1 = subprocess.run(
            ["python3", "scripts/export_lean_specs.py"],
            cwd=str(project_dir),
            capture_output=True,
            text=True,
        )
        
        assert res1.returncode != 0
        assert "FFI Naming Standardization / Build-Time Verification Failed" in res1.stderr
        assert "Expected canonical export function 'ualbf_check_crt_1155' was not found." in res1.stderr
        assert "lean4-proofs/UALBF/FFI.lean" in res1.stderr

        # Case 2: Invalid/Wrong Signature Type (e.g. returning UInt8 instead of Bool)
        corrupted_content_2 = ffi_backup.replace(
            "def ualbf_check_crt_1155_impl (z_val : @& U512) (x_l_val : @& U512) : Bool :=",
            "def ualbf_check_crt_1155_impl (z_val : @& U512) (x_l_val : @& U512) : UInt8 :="
        )
        ffi_lean_path.write_text(corrupted_content_2, encoding="utf-8")
        
        res2 = subprocess.run(
            ["python3", "scripts/export_lean_specs.py"],
            cwd=str(project_dir),
            capture_output=True,
            text=True,
        )
        
        assert res2.returncode != 0
        assert "FFI Naming Standardization / Build-Time Verification Failed" in res2.stderr
        assert "The signature of 'ualbf_check_crt_1155' is invalid." in res2.stderr
        assert "Expected argument types: '@& U512' for both arguments" in res2.stderr
        assert "Expected return type: 'Bool'" in res2.stderr
        assert "lean4-proofs/UALBF/FFI.lean" in res2.stderr

    finally:
        # Restore backup
        ffi_lean_path.write_text(ffi_backup, encoding="utf-8")


def test_ffi_safety_layer_null_and_panic():
    """
    Test FFI safety layer under normal, null, and panic-inducing input conditions
    using ctypes to invoke exported C ABI functions.
    """
    import ctypes
    project_dir = Path(__file__).parent.parent

    # Build verification-lib shared library
    res = subprocess.run(
        ["cargo", "build", "-p", "verification-lib", "--features", "signing"],
        cwd=str(project_dir),
        capture_output=True,
        text=True,
    )
    assert res.returncode == 0, f"Failed to build verification-lib: {res.stderr}"

    lib_path = project_dir / "target/debug/libverification_lib.so"
    if not lib_path.exists():
        lib_path = project_dir / "target/debug/libverification_lib.dylib"
    if not lib_path.exists():
        pytest.skip("Shared library libverification_lib not found")

    lib = ctypes.CDLL(str(lib_path))

    # 1. Test rust_sha256_file with NULL
    lib.rust_sha256_file.argtypes = [ctypes.c_char_p]
    lib.rust_sha256_file.restype = ctypes.c_void_p
    res = lib.rust_sha256_file(None)
    assert res is None or res == 0

    # 2. Test rust_sha256_file with non-existent file
    res = lib.rust_sha256_file(b"non_existent_file_path_12345.txt")
    assert res is None or res == 0

    # 3. Test rust_free_string with NULL (must not crash)
    lib.rust_free_string.argtypes = [ctypes.c_void_p]
    lib.rust_free_string.restype = None
    lib.rust_free_string(None)

    # 4. Test verify_certificate with NULL arguments
    lib.verify_certificate.argtypes = [
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_bool),
        ctypes.c_char_p,
        ctypes.c_size_t,
    ]
    lib.verify_certificate.restype = ctypes.c_void_p
    cert_res = lib.verify_certificate(None, None, None, None, 0)
    assert cert_res is None or cert_res == 0

    # 5. Test verify_certificate with invalid JSON input (should fail gracefully)
    is_valid_out = ctypes.c_bool(True)
    buf = ctypes.create_string_buffer(256)
    cert_res = lib.verify_certificate(
        b"invalid json {",
        b"pubkey",
        ctypes.byref(is_valid_out),
        buf,
        256,
    )
    assert cert_res is None or cert_res == 0
    assert not is_valid_out.value
    assert b"Failed to parse JSON" in buf.value


