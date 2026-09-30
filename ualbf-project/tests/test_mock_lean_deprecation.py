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
    reason="Skip cargo subprocess test under GHA fast-feedback python checks",
)
def test_build_rs_fails_when_mock_lean_set():
    """
    Test that rust-engine/build.rs fails fast when MOCK_LEAN=1.
    """
    project_dir = Path(__file__).parent.parent
    rust_engine_dir = project_dir / "rust-engine"

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

    assert res.returncode != 0
    assert "FATAL: Lean sysroot not found" in res.stderr or "FATAL: Lean sysroot not found" in res.stdout


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Skip cargo subprocess test under GHA fast-feedback python checks",
)
def test_build_rs_fails_when_lean_sysroot_dummy():
    """
    Test that rust-engine/build.rs fails fast when LEAN_SYSROOT=DUMMY.
    """
    project_dir = Path(__file__).parent.parent
    rust_engine_dir = project_dir / "rust-engine"

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

    assert res.returncode != 0
    assert "FATAL: Lean sysroot not found" in res.stderr or "FATAL: Lean sysroot not found" in res.stdout


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Skip cargo subprocess test under GHA fast-feedback python checks",
)
def test_build_rs_fails_when_lean_sysroot_missing():
    """
    Test that rust-engine/build.rs fails fast when LEAN_SYSROOT is missing/unset.
    """
    project_dir = Path(__file__).parent.parent
    rust_engine_dir = project_dir / "rust-engine"

    env = os.environ.copy()
    env.pop("MOCK_LEAN", None)
    env.pop("LEAN_SYSROOT", None)

    # Filter PATH to ensure lean binary is not discoverable
    new_path_parts = [
        part for part in env.get("PATH", "").split(":")
        if not os.path.exists(os.path.join(part, "lean"))
    ]
    env["PATH"] = ":".join(new_path_parts)

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

    assert res.returncode != 0
    assert "FATAL: Lean sysroot not found" in res.stderr or "FATAL: Lean sysroot not found" in res.stdout
