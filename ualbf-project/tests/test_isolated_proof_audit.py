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
                    stdout="d213dde4e45aaccdc0dd98494f1061b1926e0a15146a08622404ab2a624260b8\n",
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
    /tmp/ualbf_audit_<uuid> avoiding locks or permission conflicts.
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
                shutil.rmtree(s_dir)
            finally:
                os.chdir(old_cwd)

    assert len(staging_dirs) == 2
