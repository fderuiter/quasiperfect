import json
import os
import sys
import tempfile
import subprocess
import hashlib
import shutil
from pathlib import Path
from unittest import mock
import pytest

# Add ualbf-project to path
project_dir = Path(__file__).parent.parent
sys.path.insert(0, str(project_dir))

import auditor
from verify_cert import verify_certificate


def test_auditor_rejects_legacy_axiom():
    """
    Test that the proof auditor (auditor.py) treats the legacy FFI axiom as an
    unapproved axiom, marking the manifest generation as failed (status 'axiom' or 'error' and exit code 1).
    """
    # We will patch subprocess.run to simulate Lean environment returning different axioms.
    original_run = subprocess.run

    def mock_subprocess_run(args, *extra_args, **kwargs):
        # If running lake env lean find_axioms.lean, return the legacy axiom
        if isinstance(args, list) and "find_axioms.lean" in args[-1]:
            # Simulate a theorem depending on UALBF.FFI.rust_is_prime_sound
            stdout = "depends on axioms: [UALBF.FFI.rust_is_prime_sound, propext, Classical.choice, Quot.sound]"
            return mock.Mock(returncode=0, stdout=stdout, stderr="")
        
        # Intercept other commands to return success
        if isinstance(args, list) and (args[0] in ["lake", "cargo", "make"]):
            return mock.Mock(returncode=0, stdout="dummy_output", stderr="")
        
        return original_run(args, *extra_args, **kwargs)

    # Patch functions to avoid actually running cargo/building
    with mock.patch("subprocess.run", side_effect=mock_subprocess_run), \
         mock.patch("auditor.check_lean_environment", return_value=True), \
         mock.patch("auditor.check_documentation", return_value=True), \
         mock.patch("auditor.check_imports", return_value=True), \
         tempfile.TemporaryDirectory() as tmpdir:
        
        # Point auditor to write to the temp directory
        old_cwd = os.getcwd()
        os.chdir(tmpdir)
        try:
            # We also need a dummy bounds_manifest.json and verus_proofs.rs
            bounds_path = Path("bounds_manifest.json")
            with open(bounds_path, "w") as f:
                json.dump({
                    "omega_bounds": {
                        "prasad_sunitha": {"proof_bound": 10, "engine_justified_gap": 0, "is_axiomatic": False},
                        "hagis1982": {"proof_bound": 10, "engine_justified_gap": 0, "is_axiomatic": False}
                    },
                    "search_bounds": {
                        "target_min_log10": {"value": 35, "is_axiomatic": False},
                        "target_max_log10": {"value": 37, "is_axiomatic": False},
                        "sieve_limit": {"value": 1000, "is_axiomatic": False},
                        "max_exponent": {"value": 4, "is_axiomatic": False},
                        "prefix_stop_threshold": {"value": 100, "is_axiomatic": False},
                        "pollard_rho": {"iteration_limit": 100, "batch_size": 10, "is_axiomatic": False},
                        "raycast": {"gpu_threshold": 100, "chunk_size": 10, "is_axiomatic": False}
                    },
                    "euler_ceiling": {"num": 2, "den": 1, "is_axiomatic": False},
                    "overflow_threshold": {"num": 2, "den": 1, "is_axiomatic": False}
                }, f)
            
            # Create a dummy lean4-proofs directory structure
            Path("lean4-proofs").mkdir(parents=True, exist_ok=True)
            
            # Create dummy verus_proofs.rs
            Path("rust-engine/src").mkdir(parents=True, exist_ok=True)
            with open("rust-engine/src/verus_proofs.rs", "w") as f:
                f.write("verus! {}")

            # Running auditor.generate_manifest should exit with 1 because of the legacy axiom
            with pytest.raises(SystemExit) as exc_info:
                auditor.generate_manifest()
            
            assert exc_info.value.code == 1

            # Verify that the generated proof_manifest.json contains status 'axiom' for the theorem
            with open("proof_manifest.json", "r") as f:
                manifest = json.load(f)
            
            for thm in manifest["theorems"]:
                assert thm["status"] == "axiom"

        finally:
            os.chdir(old_cwd)


def test_auditor_allows_standard_lean_axioms():
    """
    Test that standard Lean core axioms like propext, Classical.choice, and Quot.sound
    are allowed to pass audit validation and result in status 'proven'.
    """
    original_run = subprocess.run

    def mock_subprocess_run(args, *extra_args, **kwargs):
        if isinstance(args, list) and "find_axioms.lean" in args[-1]:
            # Only standard axioms are present
            stdout = "depends on axioms: [propext, Classical.choice, Quot.sound]"
            return mock.Mock(returncode=0, stdout=stdout, stderr="")
        if isinstance(args, list) and (args[0] in ["lake", "cargo", "make"]):
            return mock.Mock(returncode=0, stdout="dummy_output", stderr="")
        return original_run(args, *extra_args, **kwargs)

    with mock.patch("subprocess.run", side_effect=mock_subprocess_run), \
         mock.patch("auditor.check_lean_environment", return_value=True), \
         mock.patch("auditor.check_documentation", return_value=True), \
         mock.patch("auditor.check_imports", return_value=True), \
         tempfile.TemporaryDirectory() as tmpdir:
        
        old_cwd = os.getcwd()
        os.chdir(tmpdir)
        try:
            bounds_path = Path("bounds_manifest.json")
            with open(bounds_path, "w") as f:
                json.dump({
                    "omega_bounds": {
                        "prasad_sunitha": {"proof_bound": 10, "engine_justified_gap": 0, "is_axiomatic": False},
                        "hagis1982": {"proof_bound": 10, "engine_justified_gap": 0, "is_axiomatic": False}
                    },
                    "search_bounds": {
                        "target_min_log10": {"value": 35, "is_axiomatic": False},
                        "target_max_log10": {"value": 37, "is_axiomatic": False},
                        "sieve_limit": {"value": 1000, "is_axiomatic": False},
                        "max_exponent": {"value": 4, "is_axiomatic": False},
                        "prefix_stop_threshold": {"value": 100, "is_axiomatic": False},
                        "pollard_rho": {"iteration_limit": 100, "batch_size": 10, "is_axiomatic": False},
                        "raycast": {"gpu_threshold": 100, "chunk_size": 10, "is_axiomatic": False}
                    },
                    "euler_ceiling": {"num": 2, "den": 1, "is_axiomatic": False},
                    "overflow_threshold": {"num": 2, "den": 1, "is_axiomatic": False}
                }, f)
            
            Path("lean4-proofs").mkdir(parents=True, exist_ok=True)
            Path("rust-engine/src").mkdir(parents=True, exist_ok=True)
            with open("rust-engine/src/verus_proofs.rs", "w") as f:
                f.write("verus! {}")

            # This should generate the manifest successfully without system exit
            auditor.generate_manifest()

            with open("proof_manifest.json", "r") as f:
                manifest = json.load(f)
            
            for thm in manifest["theorems"]:
                assert thm["status"] == "proven"

        finally:
            os.chdir(old_cwd)


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Decouple Python checks from core builds under GHA environment"
)
def test_build_script_panics_on_legacy_axiom():
    """
    Test that the compile-time validation gatekeeper (build.rs) panics when encountering
    the legacy FFI axiom in the proof manifest.
    """
    manifest_path = project_dir / "proof_manifest.json"
    backup_path = project_dir / "proof_manifest.json.bak"
    shutil.copy(manifest_path, backup_path)
    
    try:
        # Load and modify the manifest to contain the legacy axiom
        with open(manifest_path, "r") as f:
            manifest = json.load(f)
        
        manifest["theorems"] = [
            {
                "name": "UALBF.FFI.rust_is_prime_sound",
                "file": "UALBF/FFI.lean",
                "status": "axiom",
                "checksum": ""
            }
        ]
        
        payload = "UALBF.FFI.rust_is_prime_sound|UALBF/FFI.lean|axiom"
        manifest["theorems"][0]["checksum"] = hashlib.sha256(payload.encode("utf-8")).hexdigest()
        
        with open(manifest_path, "w") as f:
            json.dump(manifest, f)
            
        # Touch build.rs to force cargo to run the build script
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()
            
        # Run cargo check in the real rust-engine
        env = os.environ.copy()
        res = subprocess.run(
            ["cargo", "check", "--offline"],
            cwd=str(project_dir / "rust-engine"),
            env=env,
            capture_output=True,
            text=True
        )
        
        assert res.returncode != 0
        assert "is incomplete (status: axiom). Compilation halted." in res.stderr
        
    finally:
        # Restore the real manifest
        shutil.move(backup_path, manifest_path)
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Decouple Python checks from core builds under GHA environment"
)
def test_runtime_panics_on_legacy_axiom():
    """
    Test that the engine runtime panics and aborts execution during manifest validation
    if the legacy FFI axiom is present in the proof manifest.
    """
    # Build the engine binary first, while the manifest is clean/unmodified.
    engine_bin = project_dir / "target/debug/ualbf_engine"
    if not engine_bin.exists():
        engine_bin = project_dir / "target/release/ualbf_engine"
        
    # If binary doesn't exist, we build it once using cargo build in ualbf-project/rust-engine
    if not engine_bin.exists():
        subprocess.run(["cargo", "build", "--offline"], cwd=str(project_dir / "rust-engine"), check=True)
        engine_bin = project_dir / "target/debug/ualbf_engine"
        if not engine_bin.exists():
            engine_bin = project_dir / "rust-engine/target/debug/ualbf_engine"

    manifest_path = project_dir / "proof_manifest.json"
    backup_path = project_dir / "proof_manifest.json.bak"
    shutil.copy(manifest_path, backup_path)
    
    try:
        # Load and modify the manifest to contain the legacy axiom
        with open(manifest_path, "r") as f:
            manifest = json.load(f)
        
        manifest["theorems"] = [
            {
                "name": "UALBF.FFI.rust_is_prime_sound",
                "file": "UALBF/FFI.lean",
                "status": "axiom",
                "checksum": ""
            }
        ]
        
        payload = "UALBF.FFI.rust_is_prime_sound|UALBF/FFI.lean|axiom"
        manifest["theorems"][0]["checksum"] = hashlib.sha256(payload.encode("utf-8")).hexdigest()
        
        with open(manifest_path, "w") as f:
            json.dump(manifest, f)
            
        # Run the binary with UALBF_PROOF_MANIFEST env var pointing to our tampered proof_manifest.json
        env = os.environ.copy()
        env["UALBF_PROOF_MANIFEST"] = str(manifest_path)
        env["ALLOW_UNVERIFIED_BUILD"] = "1"  # just in case
        
        res = subprocess.run(
            [str(engine_bin)],
            cwd=str(project_dir),
            env=env,
            capture_output=True,
            text=True
        )
        
        # The runtime must panic and return non-zero
        assert res.returncode != 0
        assert "refuses to start/sign" in res.stderr or "refuses to start/sign" in res.stdout
        
    finally:
        # Restore the real manifest
        shutil.move(backup_path, manifest_path)


def test_auditor_rejects_compilation_failure():
    """
    Test that the proof auditor flags compilation failures (subprocess returns non-zero code)
    and exits with a non-zero code.
    """
    original_run = subprocess.run

    def mock_subprocess_run(args, *extra_args, **kwargs):
        if isinstance(args, list) and "lake" in args[0]:
            return mock.Mock(returncode=1, stdout="", stderr="Compilation error")
        return original_run(args, *extra_args, **kwargs)

    with mock.patch("subprocess.run", side_effect=mock_subprocess_run), \
         mock.patch("auditor.check_lean_environment", return_value=True), \
         mock.patch("auditor.check_documentation", return_value=True), \
         mock.patch("auditor.check_imports", return_value=True), \
         tempfile.TemporaryDirectory() as tmpdir:
        
        old_cwd = os.getcwd()
        os.chdir(tmpdir)
        try:
            # Setup dummy files
            bounds_path = Path("bounds_manifest.json")
            with open(bounds_path, "w") as f:
                json.dump({
                    "omega_bounds": {
                        "prasad_sunitha": {"proof_bound": 10, "engine_justified_gap": 0, "is_axiomatic": False},
                        "hagis1982": {"proof_bound": 10, "engine_justified_gap": 0, "is_axiomatic": False}
                    },
                    "search_bounds": {
                        "target_min_log10": {"value": 35, "is_axiomatic": False},
                        "target_max_log10": {"value": 37, "is_axiomatic": False},
                        "sieve_limit": {"value": 1000, "is_axiomatic": False},
                        "max_exponent": {"value": 4, "is_axiomatic": False},
                        "prefix_stop_threshold": {"value": 100, "is_axiomatic": False},
                        "pollard_rho": {"iteration_limit": 100, "batch_size": 10, "is_axiomatic": False},
                        "raycast": {"gpu_threshold": 100, "chunk_size": 10, "is_axiomatic": False}
                    },
                    "euler_ceiling": {"num": 2, "den": 1, "is_axiomatic": False},
                    "overflow_threshold": {"num": 2, "den": 1, "is_axiomatic": False}
                }, f)
            
            Path("lean4-proofs").mkdir(parents=True, exist_ok=True)
            Path("rust-engine/src").mkdir(parents=True, exist_ok=True)
            with open("rust-engine/src/verus_proofs.rs", "w") as f:
                f.write("verus! {}")

            with pytest.raises(SystemExit) as exc_info:
                auditor.generate_manifest()
            
            assert exc_info.value.code == 1

        finally:
            os.chdir(old_cwd)


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Decouple Python checks from core builds under GHA environment"
)
def test_build_script_panics_on_undefined_status():
    """
    Test that compile-time validation (build.rs) panics when encountering
    an undefined or new theorem status like 'unknown_status'.
    """
    manifest_path = project_dir / "proof_manifest.json"
    backup_path = project_dir / "proof_manifest.json.bak"
    shutil.copy(manifest_path, backup_path)
    
    try:
        with open(manifest_path, "r") as f:
            manifest = json.load(f)
        
        manifest["theorems"] = [
            {
                "name": "UALBF.FFI.rust_is_prime_sound",
                "file": "UALBF/FFI.lean",
                "status": "unknown_status",
                "checksum": ""
            }
        ]
        
        payload = "UALBF.FFI.rust_is_prime_sound|UALBF/FFI.lean|unknown_status"
        manifest["theorems"][0]["checksum"] = hashlib.sha256(payload.encode("utf-8")).hexdigest()
        
        with open(manifest_path, "w") as f:
            json.dump(manifest, f)
            
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()
            
        env = os.environ.copy()
        res = subprocess.run(
            ["cargo", "check", "--offline"],
            cwd=str(project_dir / "rust-engine"),
            env=env,
            capture_output=True,
            text=True
        )
        
        assert res.returncode != 0
        assert "is incomplete (status: unknown_status). Compilation halted." in res.stderr
        
    finally:
        shutil.move(backup_path, manifest_path)
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()


def test_verify_certificate_rejects_undefined_status():
    """
    Test that the certificate validation tool rejects manifests containing undefined,
    error, or non-whitelisted theorem statuses (e.g. 'error' or 'unverified').
    """
    manifest_path = project_dir / "proof_manifest.json"
    backup_path = project_dir / "proof_manifest.json.bak"
    shutil.copy(manifest_path, backup_path)
    
    try:
        with open(manifest_path, "r") as f:
            manifest = json.load(f)
        
        manifest["theorems"][0]["status"] = "corrupted_status"
        thm = manifest["theorems"][0]
        # Calculate physical file content hash under the new design
        file_path = project_dir / "lean4-proofs" / thm["file"]
        with open(file_path, "rb") as f:
            thm["checksum"] = hashlib.sha256(f.read()).hexdigest()
        
        with open(manifest_path, "w") as f:
            json.dump(manifest, f)
            
        with open(manifest_path, "r", encoding="utf-8") as f:
            content = f.read()
        computed_hash = hashlib.sha256(content.encode("utf-8")).hexdigest()
        
        mock_cert = {
            "manifest_hash": computed_hash,
            "public_key": "",
            "telemetry": {
                "target_min_log10": 35,
                "target_max_log10": 37,
                "total_branches_searched": 100,
                "abundance_pruned": 0,
                "raycast_pruned": 0,
                "phase2_execution_time_ms": 0,
                "total_execution_time_ms": 0,
                "math_interruptions": 0,
                "path_ranges": []
            },
            "signature": "",
            "is_conditional": False,
            "conjecture": None,
            "verified_logic_hash": "dummy",
            "bounds_manifest_hash": "dummy"
        }
        
        with mock.patch("cert_util.load_and_validate_cert", return_value=mock_cert), \
             mock.patch("verify_cert.TRUSTED_PUBLIC_KEY", None):
            with pytest.raises(SystemExit) as exc_info:
                verify_certificate("dummy_cert.json", str(manifest_path))
            assert exc_info.value.code == 1

    finally:
        shutil.move(backup_path, manifest_path)

