import json
import os
import sys
import tempfile
import subprocess
import hashlib
from pathlib import Path
from unittest import mock
import pytest

# Add ualbf-project to path
project_dir = Path(__file__).parent.parent
sys.path.insert(0, str(project_dir))

import auditor


def test_missing_namespace_prefix_fails_fast(capsys):
    """
    Test that core theorem registrations missing the root namespace prefix ('UALBF.')
    fail fast immediately with exit code 1 and an explicit error message.
    """
    with mock.patch("auditor.CORE_THEOREMS", ["forced_inclusion"]):
        with pytest.raises(SystemExit) as exc_info:
            auditor.generate_manifest()
        assert exc_info.value.code == 1

    captured = capsys.readouterr()
    assert "missing required root namespace prefix 'UALBF.'" in captured.err
    assert "forced_inclusion" in captured.err


def test_unresolved_symbol_axiom_verification_fails_fast(capsys):
    """
    Test that registered theorem symbols that cannot be resolved during axiom verification
    fail fast immediately with exit code 1.
    """
    bad_thm = "UALBF.Engine.CyclotomicGraph.non_existent_symbol"

    def mock_subprocess_run(args, *extra_args, **kwargs):
        if isinstance(args, list) and "find_axioms.lean" in args[-1]:
            stdout = f"error: unknown identifier '{bad_thm}'"
            return mock.Mock(returncode=1, stdout=stdout, stderr=stdout)
        if isinstance(args, list) and (args[0] in ["lake", "cargo", "make"]):
            return mock.Mock(returncode=0, stdout="dummy", stderr="")
        return subprocess.run(args, *extra_args, **kwargs)

    with mock.patch("subprocess.run", side_effect=mock_subprocess_run), \
         mock.patch("auditor.check_lean_environment", return_value=True), \
         mock.patch("auditor.CORE_THEOREMS", [bad_thm]):
        with pytest.raises(SystemExit) as exc_info:
            auditor.generate_manifest()
        assert exc_info.value.code == 1

    captured = capsys.readouterr()
    assert "could not be resolved during axiom verification" in captured.err


def test_unresolved_path_resolution_fails_fast(capsys):
    """
    Test that theorem registrations with no matching physical proof file fail fast
    without falling back to top-level module defaults (UALBF.lean).
    """
    unresolved_thm = "UALBF.NonExistentModule.some_theorem"

    with mock.patch("auditor.CORE_THEOREMS", [unresolved_thm]), \
         mock.patch("auditor.check_lean_environment", return_value=True):
        os.environ["MOCK_LEAN"] = "1"
        try:
            with pytest.raises(SystemExit) as exc_info:
                auditor.generate_manifest()
            assert exc_info.value.code == 1
        finally:
            os.environ.pop("MOCK_LEAN", None)

    captured = capsys.readouterr()
    assert "Source file path resolution failed" in captured.err
    assert "fallback to top-level module default prohibited" in captured.err


def test_valid_fully_qualified_theorem_registration(tmp_path):
    """
    Test that fully-qualified core theorems resolve to exact physical proof source files,
    generate matching SHA-256 checksums, and exit with code 0.
    """
    valid_thm = "UALBF.Engine.CyclotomicGraph.forced_inclusion"
    expected_rel_file = "UALBF/Engine/CyclotomicGraph.lean"
    real_file_path = project_dir / "lean4-proofs" / expected_rel_file
    assert real_file_path.exists()

    expected_hash = hashlib.sha256(real_file_path.read_bytes()).hexdigest()

    old_cwd = os.getcwd()
    os.chdir(tmp_path)
    try:
        bounds_path = tmp_path / "bounds_manifest.json"
        bounds_path.write_text(json.dumps({
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
        }))

        (tmp_path / "rust-engine/src").mkdir(parents=True, exist_ok=True)
        (tmp_path / "rust-engine/src/verus_proofs.rs").write_text("verus! {}")

        with mock.patch("auditor.CORE_THEOREMS", [valid_thm]), \
             mock.patch("auditor.check_lean_environment", return_value=True), \
             mock.patch("auditor.check_documentation", return_value=True), \
             mock.patch("auditor.check_imports", return_value=True):
            os.environ["MOCK_LEAN"] = "1"
            try:
                auditor.generate_manifest()
            finally:
                os.environ.pop("MOCK_LEAN", None)

        manifest_file = tmp_path / "proof_manifest.json"
        assert manifest_file.exists()

        manifest = json.loads(manifest_file.read_text())
        assert len(manifest["theorems"]) == 1
        entry = manifest["theorems"][0]
        assert entry["name"] == valid_thm
        assert entry["file"] == expected_rel_file
        assert entry["status"] == "proven"
        assert entry["checksum"] == expected_hash

    finally:
        os.chdir(old_cwd)
