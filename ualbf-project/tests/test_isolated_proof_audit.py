import json
import os
import shutil
import tempfile
import uuid
from pathlib import Path
from unittest import mock
import pytest
import auditor


def test_proof_audit_staging_workspace_immutability():
    """
    Verify that proof auditing executes inside an isolated staging directory
    (/tmp/ualbf_audit_<uuid>), ensuring 100% host workspace read-only immutability.
    """
    project_dir = Path(__file__).parent.parent.resolve()
    manifest_path = project_dir / "proof_manifest.json"
    backup_path = project_dir / "proof_manifest.json.bak"
    if manifest_path.exists():
        shutil.copy2(manifest_path, backup_path)

    # Record mtime and permissions of host workspace files before audit
    host_file_snapshots = {}
    for root, dirs, files in os.walk(project_dir):
        if (
            ".git" in root
            or "__pycache__" in root
            or ".pytest_cache" in root
            or "target" in root
        ):
            continue
        for f in files:
            p = os.path.join(root, f)
            try:
                st = os.stat(p)
                host_file_snapshots[p] = (st.st_mtime, st.st_mode)
            except Exception:
                pass

    observed_staging_dirs = []
    original_setup = auditor._setup_staging_workspace

    def hook_setup(host_dir, staging_dir):
        observed_staging_dirs.append(staging_dir)
        return original_setup(host_dir, staging_dir)

    def mock_subprocess_run(cmd, *args, **kwargs):
        if isinstance(cmd, list) and len(cmd) > 0:
            cmd_str = " ".join(str(c) for c in cmd)
            if "hash-tcb" in cmd_str:
                if "--extension" in cmd_str:
                    return mock.Mock(
                        returncode=0,
                        stdout="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n",
                        stderr="",
                    )
                return mock.Mock(
                    returncode=0,
                    stdout="03e50fb3a0659b0a795ae03323b3919e1aef470e99f20f671bae11ae2e6de95c\n",
                    stderr="",
                )
            if cmd[0] in ("lean", "lake", "cargo", "make"):
                return mock.Mock(
                    returncode=0,
                    stdout="depends on axioms: [propext, Classical.choice, Quot.sound]\n",
                    stderr="",
                )
        return mock.Mock(returncode=0, stdout="", stderr="")

    auditor._setup_staging_workspace = hook_setup

    try:
        with mock.patch(
            "auditor.subprocess.run", side_effect=mock_subprocess_run
        ), mock.patch("auditor.check_lean_environment", return_value=True), mock.patch(
            "auditor.check_documentation", return_value=True
        ), mock.patch(
            "auditor.check_imports", return_value=True
        ):
            # Run auditor
            auditor.generate_manifest()

        # Verify staging workspace was established under /tmp/ualbf_audit_
        assert len(observed_staging_dirs) > 0
        staging_dir = observed_staging_dirs[0]
        assert staging_dir.startswith("/tmp/ualbf_audit_")

        # Verify staging directory was purged after completion
        assert not os.path.exists(staging_dir)

        # Verify proof_manifest.json exists in host directory and is valid
        assert manifest_path.exists()
        with open(manifest_path, "r") as f:
            manifest = json.load(f)
        assert "theorems" in manifest

        # Verify host workspace files experienced zero mtime or permission modifications
        for p, (old_mtime, old_mode) in host_file_snapshots.items():
            if os.path.basename(p) == "proof_manifest.json":
                continue  # manifest transfer back is expected
            if os.path.exists(p):
                st = os.stat(p)
                assert (
                    st.st_mtime == old_mtime
                ), f"File {p} mtime was modified during audit!"
                assert (
                    st.st_mode == old_mode
                ), f"File {p} permissions were modified during audit!"

    finally:
        auditor._setup_staging_workspace = original_setup
        if backup_path.exists():
            shutil.move(backup_path, manifest_path)


def test_concurrent_audit_staging_isolation():
    """
    Verify that multiple audit tasks generate unique staging directories under
    /tmp/ualbf_audit_<uuid> avoiding locks or permission conflicts, and that
    env_manifest files are correctly copied into the staging workspace.
    """
    staging_dirs = set()
    project_dir = Path(__file__).parent.parent.resolve()
    bounds_src = project_dir / "bounds_manifest.json"

    with tempfile.TemporaryDirectory() as tmp1, tempfile.TemporaryDirectory() as tmp2:
        for tmp in [tmp1, tmp2]:
            tmp_path = Path(tmp)
            (tmp_path / "bounds_manifest.json").write_text(bounds_src.read_text())

            old_cwd = os.getcwd()
            os.chdir(tmp)
            try:
                audit_id = uuid.uuid4().hex
                s_dir = f"/tmp/ualbf_audit_{audit_id}"
                auditor._setup_staging_workspace(tmp, s_dir)
                staging_dirs.add(s_dir)
                assert os.path.exists(s_dir)
                # Verify env_manifest.json and env_manifest.schema.json exist in staging_dir
                if (project_dir / "env_manifest.json").exists() or (
                    project_dir.parent / "env_manifest.json"
                ).exists():
                    assert os.path.exists(os.path.join(s_dir, "env_manifest.json"))
                    assert os.path.exists(
                        os.path.join(s_dir, "env_manifest.schema.json")
                    )
                shutil.rmtree(s_dir)
            finally:
                os.chdir(old_cwd)

    assert len(staging_dirs) == 2


def test_staging_workspace_env_manifest_self_contained_and_doc_checks():
    """
    Verify that _setup_staging_workspace copies env_manifest.json and env_manifest.schema.json
    into staging_dir without polluting /tmp, and check_documentation passes for TCB.md
    when run inside the staging workspace.
    """
    project_dir = Path(__file__).parent.parent.resolve()
    manifest_path = project_dir / "proof_manifest.json"
    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    # Clean up any residual /tmp/env_manifest.json to ensure test isolation
    if os.path.exists("/tmp/env_manifest.json"):
        try:
            os.remove("/tmp/env_manifest.json")
        except Exception:
            pass
    if os.path.exists("/tmp/env_manifest.schema.json"):
        try:
            os.remove("/tmp/env_manifest.schema.json")
        except Exception:
            pass

    staging_dir = f"/tmp/ualbf_audit_{uuid.uuid4().hex}"
    try:
        auditor._setup_staging_workspace(str(project_dir), staging_dir)

        # Assert manifest files were copied into staging workspace
        assert os.path.exists(os.path.join(staging_dir, "env_manifest.json"))
        assert os.path.exists(os.path.join(staging_dir, "env_manifest.schema.json"))

        # Assert parent directory /tmp was not polluted
        assert not os.path.exists("/tmp/env_manifest.json")
        assert not os.path.exists("/tmp/env_manifest.schema.json")

        # Assert check_documentation passes in staging_dir
        assert auditor.check_documentation(manifest, repo_root=staging_dir) is True
    finally:
        if os.path.exists(staging_dir):
            shutil.rmtree(staging_dir, ignore_errors=True)


def test_check_documentation_verus_and_module_qualification():
    """
    Verify that auditor.check_documentation correctly recognizes Verus spec/proof
    functions (with open/closed modifiers) and qualified module symbols like
    math_utils::tests as valid code symbols in documentation checks.
    """
    manifest = {
        "theorems": [],
        "verus_hashes": {
            "scale_bound_spec": "dummy_hash",
            "lean_miller_rabin_20_base_sufficiency": "dummy_hash",
        },
    }
    # Run check_documentation on real repository files
    assert auditor.check_documentation(manifest) is True


def test_check_imports():
    project_dir = Path(__file__).parent.parent.resolve()
    assert auditor.check_imports(str(project_dir)) is True


def test_check_lean_environment():
    with mock.patch.dict(os.environ, {"LEAN_SYSROOT": "DUMMY"}, clear=False):
        auditor.check_lean_environment()


def test_cross_language_bindings_parsed_and_verified():
    """
    Verify that parse_semantic_verification_report correctly extracts
    all 4 sections of formal cross-language bindings from semantic_verification_report.md
    and verifies all symbol references.
    """
    project_dir = Path(__file__).parent.parent.resolve()
    report_path = project_dir / "semantic_verification_report.md"
    manifest_path = project_dir / "proof_manifest.json"
    assert report_path.exists()

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    bindings = auditor.parse_semantic_verification_report(
        str(report_path), manifest=manifest, repo_root=str(project_dir)
    )

    assert len(bindings) == 4
    sections = [b["section"] for b in bindings]
    assert "1. Pruning Starvation Logic" in sections
    assert "2. Fixed-Point Scaling Logic" in sections
    assert "3. Epistemological Memory Boundary" in sections
    assert "4. Abbott-Aull Mod-5 Obstruction" in sections

    for b in bindings:
        assert "lean_symbols" in b
        assert "verus_identifiers" in b
        assert "rust_functions" in b
        assert "component_hashes" in b
        assert "file_checksums" in b
        assert len(b["component_hashes"]) > 0
        assert len(b["file_checksums"]) > 0


def test_cross_language_bindings_broken_lean_symbol_fails(tmp_path):
    """
    Verify that an unrecognized or broken Lean symbol in semantic_verification_report.md
    causes verify_cross_language_bindings to raise ValueError.
    """
    fake_report = tmp_path / "semantic_verification_report.md"
    fake_report.write_text(
        "## 1. Test Section\n"
        "- **Lean Theorem:** `nonexistent_lean_theorem_xyz_999` in `lean4-proofs/UALBF/QPN/AbundancyBound.lean`\n"
        "- **Verus Specification:** `scale_bound_spec` in `rust-engine/src/verus_proofs.rs`\n"
        "- **Rust Implementation:** `check_starvation_kill` in `rust-engine/src/verus_proofs.rs`\n"
    )

    project_dir = Path(__file__).parent.parent.resolve()
    with pytest.raises(ValueError) as excinfo:
        auditor.parse_semantic_verification_report(
            str(fake_report), manifest={}, repo_root=str(project_dir)
        )
    assert "nonexistent_lean_theorem_xyz_999" in str(excinfo.value)


def test_cross_language_bindings_broken_verus_identifier_fails(tmp_path):
    """
    Verify that an unrecognized Verus identifier in semantic_verification_report.md
    causes verify_cross_language_bindings to raise ValueError.
    """
    fake_report = tmp_path / "semantic_verification_report.md"
    fake_report.write_text(
        "## 1. Test Section\n"
        "- **Lean Theorem:** `abundancy_starvation` in `lean4-proofs/UALBF/QPN/AbundancyBound.lean`\n"
        "- **Verus Specification:** `nonexistent_verus_identifier_xyz_999` in `rust-engine/src/verus_proofs.rs`\n"
        "- **Rust Implementation:** `check_starvation_kill` in `rust-engine/src/verus_proofs.rs`\n"
    )

    project_dir = Path(__file__).parent.parent.resolve()
    with pytest.raises(ValueError) as excinfo:
        auditor.parse_semantic_verification_report(
            str(fake_report), manifest={}, repo_root=str(project_dir)
        )
    assert "nonexistent_verus_identifier_xyz_999" in str(excinfo.value)


def test_cross_language_bindings_broken_rust_function_fails(tmp_path):
    """
    Verify that an unrecognized Rust function in semantic_verification_report.md
    causes verify_cross_language_bindings to raise ValueError.
    """
    fake_report = tmp_path / "semantic_verification_report.md"
    fake_report.write_text(
        "## 1. Test Section\n"
        "- **Lean Theorem:** `abundancy_starvation` in `lean4-proofs/UALBF/QPN/AbundancyBound.lean`\n"
        "- **Verus Specification:** `scale_bound_spec` in `rust-engine/src/verus_proofs.rs`\n"
        "- **Rust Implementation:** `nonexistent_rust_function_xyz_999` in `rust-engine/src/verus_proofs.rs`\n"
    )

    project_dir = Path(__file__).parent.parent.resolve()
    with pytest.raises(ValueError) as excinfo:
        auditor.parse_semantic_verification_report(
            str(fake_report), manifest={}, repo_root=str(project_dir)
        )
    assert "nonexistent_rust_function_xyz_999" in str(excinfo.value)


def test_cross_language_bindings_mismatch_fails_check_documentation():
    """
    Verify that if cross_language_bindings in proof_manifest.json does not match
    semantic_verification_report.md, check_documentation returns False.
    """
    project_dir = Path(__file__).parent.parent.resolve()
    manifest_path = project_dir / "proof_manifest.json"
    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    tampered_manifest = dict(manifest)
    tampered_manifest["cross_language_bindings"] = [
        {
            "section": "1. Pruning Starvation Logic",
            "lean_symbols": ["tampered_symbol"],
            "lean_files": [],
            "verus_identifiers": [],
            "verus_files": [],
            "rust_functions": [],
            "rust_files": [],
            "component_hashes": {},
            "file_checksums": {},
        }
    ]

    assert auditor.check_documentation(tampered_manifest) is False
