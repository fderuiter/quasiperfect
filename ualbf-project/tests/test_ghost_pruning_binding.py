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

project_dir = Path(__file__).parent.parent
sys.path.insert(0, str(project_dir))

import auditor


def test_ghost_pruning_bindings_present_in_manifest():
    """
    Verify that auditor.generate_manifest() populates ghost_pruning_bindings
    in proof_manifest.json with Lean 4 theorem identifiers and SHA-256 hashes.
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        old_cwd = os.getcwd()
        os.chdir(tmpdir)
        try:
            # Create dummy bounds_manifest.json
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

            # Mock check functions so auditor succeeds
            with mock.patch("auditor.check_lean_environment", return_value=True), \
                 mock.patch("auditor.check_documentation", return_value=True), \
                 mock.patch("auditor.check_imports", return_value=True), \
                 mock.patch("subprocess.run", return_value=mock.Mock(returncode=0, stdout="", stderr="")):
                auditor.generate_manifest()

            with open("proof_manifest.json", "r") as f:
                manifest = json.load(f)

            assert "ghost_pruning_bindings" in manifest
            gb = manifest["ghost_pruning_bindings"]
            assert "check_starvation_kill" in gb
            assert "lean_abundancy_starvation_theorem" in gb
            assert "check_cdg_forced_kill" in gb
            assert "lemma_sigma_multiplicative" in gb

            # Verify that each binding points to a Lean theorem and SHA-256 hash
            starv_binding = gb["check_starvation_kill"]
            assert starv_binding["lean_theorem"] == "UALBF.QPN.AbundancyBound.abundancy_starvation"
            assert len(starv_binding["theorem_hash"]) == 64

        finally:
            os.chdir(old_cwd)


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Decouple Python checks from core builds under GHA environment"
)
def test_missing_ghost_binding_fails_cargo_build():
    """
    Test that if a required ghost pruning assumption binding is removed from
    ghost_pruning_bindings, cargo check fails to compile the engine.
    """
    manifest_path = project_dir / "proof_manifest.json"
    backup_path = project_dir / "proof_manifest.json.bak"
    shutil.copy(manifest_path, backup_path)

    try:
        with open(manifest_path, "r") as f:
            manifest = json.load(f)

        if "ghost_pruning_bindings" in manifest:
            manifest["ghost_pruning_bindings"].pop("check_starvation_kill", None)

        with open(manifest_path, "w") as f:
            json.dump(manifest, f)

        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()

        env = os.environ.copy()
        res = subprocess.run(
            ["cargo", "check"],
            cwd=str(project_dir / "rust-engine"),
            env=env,
            capture_output=True,
            text=True
        )

        assert res.returncode != 0
        assert "Search pruning assumption 'check_starvation_kill' lacks a matching Lean 4 manifest entry" in res.stderr

    finally:
        shutil.move(backup_path, manifest_path)
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Decouple Python checks from core builds under GHA environment"
)
def test_mismatched_ghost_theorem_hash_fails_cargo_build():
    """
    Test that if a theorem hash in ghost_pruning_bindings does not match the theorem's checksum,
    cargo check fails compilation with a SHA-256 hash mismatch error.
    """
    manifest_path = project_dir / "proof_manifest.json"
    backup_path = project_dir / "proof_manifest.json.bak"
    shutil.copy(manifest_path, backup_path)

    try:
        with open(manifest_path, "r") as f:
            manifest = json.load(f)

        if "ghost_pruning_bindings" in manifest and "check_starvation_kill" in manifest["ghost_pruning_bindings"]:
            manifest["ghost_pruning_bindings"]["check_starvation_kill"]["theorem_hash"] = "0" * 64

        with open(manifest_path, "w") as f:
            json.dump(manifest, f)

        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()

        env = os.environ.copy()
        res = subprocess.run(
            ["cargo", "check"],
            cwd=str(project_dir / "rust-engine"),
            env=env,
            capture_output=True,
            text=True
        )

        assert res.returncode != 0
        assert "SHA-256 hash mismatch for Lean theorem 'UALBF.QPN.AbundancyBound.abundancy_starvation'" in res.stderr

    finally:
        shutil.move(backup_path, manifest_path)
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()
