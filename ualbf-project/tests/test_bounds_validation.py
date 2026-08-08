"""
Tests for bounds validation and synchronization.
Note: Running this test suite locally outside of Nix requires the Z3 development
headers package (`libz3-dev` on Ubuntu/Debian) to be installed on the host system.
"""

import json
import os
import re


def test_specification_parity():
    # 1. Parse bounds_manifest.json
    bounds_path = os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "..", "bounds_manifest.json"
    )
    with open(bounds_path, "r", encoding="utf-8") as f:
        bounds = json.load(f)

    ps_proof = bounds["omega_bounds"]["prasad_sunitha"]["proof_bound"]
    ps_gap = bounds["omega_bounds"]["prasad_sunitha"]["engine_justified_gap"]
    ps_combined = ps_proof + ps_gap

    hagis_proof = bounds["omega_bounds"]["hagis1982"]["proof_bound"]
    hagis_gap = bounds["omega_bounds"]["hagis1982"]["engine_justified_gap"]
    hagis_combined = hagis_proof + hagis_gap

    div_5_proof = bounds["omega_bounds"]["div_5_coprime_3"]["proof_bound"]
    div_5_gap = bounds["omega_bounds"]["div_5_coprime_3"]["engine_justified_gap"]
    div_5_combined = div_5_proof + div_5_gap

    crt_modulus_product = bounds["crt_obstruction"]["modulus_product"]

    # 2. Parse manifest_constants.rs (active engine constants)
    constants_path = os.path.join(
        os.path.dirname(os.path.abspath(__file__)),
        "..",
        "rust-engine",
        "src",
        "manifest_constants.rs",
    )

    engine_constants = {}
    if os.path.exists(constants_path):
        with open(constants_path, "r", encoding="utf-8") as f:
            for line in f:
                match = re.match(
                    r"^pub const ([A-Z0-9_]+):\s*[a-z0-9]+\s*=\s*([0-9]+);",
                    line.strip(),
                )
                if match:
                    engine_constants[match.group(1)] = int(match.group(2))

    assert (
        engine_constants.get("PRASAD_SUNITHA_PROOF_BOUND") == ps_proof
    ), "Active constant mismatch for PS proof bound"
    assert (
        engine_constants.get("PRASAD_SUNITHA_BOUND_NO_3_5") == ps_combined
    ), "Active constant mismatch for PS combined bound"
    assert (
        engine_constants.get("DIV_5_COPRIME_3_PROOF_BOUND") == div_5_proof
    ), "Active constant mismatch for div 5 coprime 3 proof bound"
    assert (
        engine_constants.get("DIV_5_COPRIME_3_BOUND") == div_5_combined
    ), "Active constant mismatch for div 5 coprime 3 combined bound"
    # BASELINE_MIN_PRIME_FACTORS is defined as baseline_proof + baseline_gap in build.rs
    assert (
        engine_constants.get("BASELINE_MIN_PRIME_FACTORS") == hagis_combined
    ), "Active constant mismatch for Hagis combined bound"
    assert (
        engine_constants.get("CRT_MODULUS_PRODUCT") == crt_modulus_product
    ), "Active constant mismatch for CRT modulus product"

    # 3. Parse lean_export.rs (generated specifications)
    specs_path = os.path.join(
        os.path.dirname(os.path.abspath(__file__)),
        "..",
        "rust-engine",
        "src",
        "lean_export.rs",
    )

    spec_constants = {}
    if os.path.exists(specs_path):
        with open(specs_path, "r", encoding="utf-8") as f:
            for line in f:
                match = re.match(
                    r".*pub open spec fn (lean_[a-z0-9_]+)\(\) -> nat \{\s*([0-9]+)\s*\}",
                    line.strip(),
                )
                if match:
                    spec_constants[match.group(1)] = int(match.group(2))

    assert (
        spec_constants.get("lean_prasad_sunitha_bound") == ps_proof
    ), "Spec mismatch for PS proof bound"
    assert (
        spec_constants.get("lean_prasad_sunitha_offset") == ps_gap
    ), "Spec mismatch for PS gap/offset"
    assert (
        spec_constants.get("lean_prasad_sunitha_combined") == ps_combined
    ), "Spec mismatch for PS combined bound"
    assert (
        spec_constants.get("lean_div_5_coprime_3_bound") == div_5_proof
    ), "Spec mismatch for div 5 coprime 3 proof bound"
    assert (
        spec_constants.get("lean_div_5_coprime_3_offset") == div_5_gap
    ), "Spec mismatch for div 5 coprime 3 gap/offset"
    assert (
        spec_constants.get("lean_div_5_coprime_3_combined") == div_5_combined
    ), "Spec mismatch for div 5 coprime 3 combined bound"

    assert (
        spec_constants.get("lean_hagis1982_min_prime_factors") == hagis_proof
    ), "Spec mismatch for Hagis proof bound"
    assert (
        spec_constants.get("lean_hagis1982_offset") == hagis_gap
    ), "Spec mismatch for Hagis gap/offset"
    assert (
        spec_constants.get("lean_hagis1982_combined") == hagis_combined
    ), "Spec mismatch for Hagis combined bound"
    assert (
        spec_constants.get("lean_crt_modulus_product") == crt_modulus_product
    ), "Spec mismatch for CRT modulus product"


def test_conjectural_bounds_conflict_fails_build():
    """
    Test that if conjectural bounds are active but the ceiling is set below the search floor,
    cargo check fails to compile and describes the conflicting parameters.
    """
    import subprocess
    import shutil
    from pathlib import Path

    project_dir = Path(__file__).parent.parent
    bounds_path = project_dir / "bounds_manifest.json"
    proof_path = project_dir / "proof_manifest.json"
    lean_export_path = project_dir / "rust-engine/src/lean_export.rs"
    manifest_constants_h = project_dir / "rust-engine/src/manifest_constants.h"
    manifest_constants_rs = project_dir / "rust-engine/src/manifest_constants.rs"
    manifest_constants_lean = project_dir / "lean4-proofs/UALBF/ManifestConstants.lean"

    # Backup files
    bounds_backup = bounds_path.read_text(encoding="utf-8")
    proof_backup = proof_path.read_text(encoding="utf-8")
    lean_export_backup = lean_export_path.read_text(encoding="utf-8")
    constants_h_backup = manifest_constants_h.read_text(encoding="utf-8")
    constants_rs_backup = manifest_constants_rs.read_text(encoding="utf-8")
    constants_lean_backup = manifest_constants_lean.read_text(encoding="utf-8")

    try:
        # Modify bounds_manifest to have active = true and ceiling = 30 (which is less than target_min_log10 = 37)
        bounds_data = json.loads(bounds_backup)
        bounds_data["conjectural_bounds"] = {
            "active": True,
            "conjecture_name": "ABC Conjecture",
            "target_max_log10_ceiling": 30
        }
        bounds_path.write_text(json.dumps(bounds_data, indent=2), encoding="utf-8")

        # 1. Run export_lean_specs.py to regenerate specifications
        subprocess.run(
            ["python3", "scripts/export_lean_specs.py"],
            cwd=str(project_dir),
            check=True
        )

        # 2. Run auditor.py with MOCK_LEAN=1 to update proof_manifest.json
        env = os.environ.copy()
        env["MOCK_LEAN"] = "1"
        subprocess.run(
            ["python3", "auditor.py"],
            cwd=str(project_dir),
            env=env
        )

        # Touch build.rs to force rerun
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()

        # 3. Run cargo check
        res = subprocess.run(
            ["cargo", "check"],
            cwd=str(project_dir / "rust-engine"),
            capture_output=True,
            text=True
        )

        assert res.returncode != 0
        assert "FATAL: Conflicting bounds parameters detected!" in res.stderr
        assert "target_max_log10_ceiling = 30" in res.stderr
        assert "target_min_log10 = 37" in res.stderr

    finally:
        # Restore backups
        bounds_path.write_text(bounds_backup, encoding="utf-8")
        proof_path.write_text(proof_backup, encoding="utf-8")
        lean_export_path.write_text(lean_export_backup, encoding="utf-8")
        manifest_constants_h.write_text(constants_h_backup, encoding="utf-8")
        manifest_constants_rs.write_text(constants_rs_backup, encoding="utf-8")
        manifest_constants_lean.write_text(constants_lean_backup, encoding="utf-8")
        # Touch build.rs to restore clean state
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()

