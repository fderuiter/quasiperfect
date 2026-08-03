import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
import pytest

from verify_cert import verify_certificate

def test_check_lean_environment_missing_fails():
    """
    Test that when Lean is missing:
    1. auditor.py exits with status code 1.
    2. A descriptive warning is printed to stderr.
    3. The proof manifest is generated and contains a global status set to "unverified".
    4. Individual theorems are marked as "unverified".
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)
        # Create a copy of auditor.py in a temp space or run it from the real path,
        # but change the current working directory to write proof_manifest.json inside the temp dir.
        
        env = os.environ.copy()
        # Force Lean to be missing by unsetting LEAN_SYSROOT and clearing PATH to not have lean
        env["LEAN_SYSROOT"] = "DUMMY" # non-existent or empty path
        # Filter path to remove anything that might contain lean
        new_path_parts = []
        for part in env.get("PATH", "").split(":"):
            if not os.path.exists(os.path.join(part, "lean")):
                new_path_parts.append(part)
        env["PATH"] = ":".join(new_path_parts)
        
        # Run auditor.py in tmpdir so it writes proof_manifest.json there
        auditor_path = Path("/app/ualbf-project/auditor.py").resolve()
        
        # Copy bounds_manifest.json to tmpdir since auditor.py searches for it
        bounds_src = Path("/app/ualbf-project/bounds_manifest.json")
        bounds_dest = tmp_path / "bounds_manifest.json"
        bounds_dest.write_text(bounds_src.read_text())
        
        res = subprocess.run(
            [sys.executable, str(auditor_path)],
            cwd=tmpdir,
            env=env,
            capture_output=True,
            text=True,
        )
        
        # Must exit with code 1
        assert res.returncode == 1
        
        # Stderr must contain descriptive warning
        assert "Lean 4 compiler toolchain not found" in res.stderr
        assert "The manifest has been tainted due to the missing compiler" in res.stderr
        assert "Manifest generation failed / tainted" in res.stderr
        
        # Check the generated proof_manifest.json
        manifest_file = tmp_path / "proof_manifest.json"
        assert manifest_file.exists()
        
        with open(manifest_file, "r") as f:
            manifest = json.load(f)
            
        assert manifest.get("status") == "unverified"
        assert len(manifest.get("theorems", [])) > 0
        for thm in manifest["theorems"]:
            assert thm["status"] == "unverified"


def test_verify_cert_rejects_global_unverified_manifest(tmp_path):
    """
    Test that verify_certificate exits with 1 when the manifest status is "unverified".
    """
    manifest = {
        "status": "unverified",
        "theorems": [
            {
                "name": "SomeTheorem",
                "file": "SomeFile.lean",
                "status": "proven",
                "checksum": "123"
            }
        ],
        "bounds_manifest_hash": "abc"
    }
    
    manifest_path = tmp_path / "proof_manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f)
        
    cert_path = tmp_path / "formal_certificate.json"
    with open(cert_path, "w") as f:
        json.dump({"dummy": "cert"}, f)
        
    with pytest.raises(SystemExit) as exc_info:
        verify_certificate(str(cert_path), str(manifest_path))
        
    assert exc_info.value.code == 1


def test_verify_cert_rejects_theorem_unverified_manifest(tmp_path):
    """
    Test that verify_certificate exits with 1 when any theorem status is "unverified".
    """
    manifest = {
        "theorems": [
            {
                "name": "SomeTheorem",
                "file": "SomeFile.lean",
                "status": "unverified",
                "checksum": "123"
            }
        ],
        "bounds_manifest_hash": "abc"
    }
    
    manifest_path = tmp_path / "proof_manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f)
        
    cert_path = tmp_path / "formal_certificate.json"
    with open(cert_path, "w") as f:
        json.dump({"dummy": "cert"}, f)
        
    with pytest.raises(SystemExit) as exc_info:
        verify_certificate(str(cert_path), str(manifest_path))
        
    assert exc_info.value.code == 1
