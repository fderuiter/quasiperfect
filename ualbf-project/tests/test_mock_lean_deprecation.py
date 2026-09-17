import os
import subprocess
import sys
from pathlib import Path
import pytest


def test_auditor_fails_when_mock_lean_set():
    """
    Test that running auditor.py with MOCK_LEAN=1 prints a fatal error message
    and exits with code 1.
    """
    project_dir = Path(__file__).parent.parent
    auditor_path = project_dir / "auditor.py"

    env = os.environ.copy()
    env["MOCK_LEAN"] = "1"

    res = subprocess.run(
        [sys.executable, str(auditor_path)],
        cwd=str(project_dir),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 1
    assert "MOCK_LEAN is forbidden" in res.stderr or "MOCK_LEAN is forbidden" in res.stdout


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Decouple Python checks from core builds under GHA environment",
)
def test_build_rs_succeeds_and_purges_ir_when_mock_lean_set():
    """
    Test that rust-engine/build.rs purges .lake/build/ir files and succeeds when MOCK_LEAN=1.
    """
    project_dir = Path(__file__).parent.parent
    rust_engine_dir = project_dir / "rust-engine"
    lean_project_dir = project_dir / "lean4-proofs"
    ir_dir = lean_project_dir / ".lake/build/ir"

    ir_dir.mkdir(parents=True, exist_ok=True)
    ualbf_dir = ir_dir / "UALBF"
    ualbf_dir.mkdir(parents=True, exist_ok=True)
    dummy_file = ualbf_dir / "stale.c"
    dummy_file.write_text("void stale_func() {}")

    env = os.environ.copy()
    env["MOCK_LEAN"] = "1"

    # Touch build.rs to force build script rerun
    build_rs_path = rust_engine_dir / "build.rs"
    if build_rs_path.exists():
        build_rs_path.touch()

    res = subprocess.run(
        ["cargo", "check"],
        cwd=str(rust_engine_dir),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 0
    assert not dummy_file.exists(), "Stale C-IR file was not purged under MOCK_LEAN=1!"
    assert not ualbf_dir.exists(), "Stale UALBF IR directory was not purged under MOCK_LEAN=1!"


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Decouple Python checks from core builds under GHA environment",
)
def test_build_rs_succeeds_and_purges_ir_when_lean_sysroot_dummy():
    """
    Test that rust-engine/build.rs purges .lake/build/ir files and succeeds when LEAN_SYSROOT=DUMMY.
    """
    project_dir = Path(__file__).parent.parent
    rust_engine_dir = project_dir / "rust-engine"
    lean_project_dir = project_dir / "lean4-proofs"
    ir_dir = lean_project_dir / ".lake/build/ir"

    ir_dir.mkdir(parents=True, exist_ok=True)
    ualbf_dir = ir_dir / "UALBF"
    ualbf_dir.mkdir(parents=True, exist_ok=True)
    dummy_file = ualbf_dir / "stale.c"
    dummy_file.write_text("void stale_func() {}")

    env = os.environ.copy()
    env.pop("MOCK_LEAN", None)
    env["LEAN_SYSROOT"] = "DUMMY"

    # Touch build.rs to force build script rerun
    build_rs_path = rust_engine_dir / "build.rs"
    if build_rs_path.exists():
        build_rs_path.touch()

    res = subprocess.run(
        ["cargo", "check"],
        cwd=str(rust_engine_dir),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 0
    assert not dummy_file.exists(), "Stale C-IR file was not purged under LEAN_SYSROOT=DUMMY!"
    assert not ualbf_dir.exists(), "Stale UALBF IR directory was not purged under LEAN_SYSROOT=DUMMY!"
