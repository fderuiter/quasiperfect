import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def test_guarded_rebuild_success():
    """
    Test that when lake build succeeds:
    1. The intermediate C-IR directory (.lake/build/ir) is deleted.
    2. The build proceeds beyond lake check (even if it eventually fails on
       missing files, the custom 'FATAL: Lean proof verification failed!'
       panic is NOT triggered).
    """
    lean_project_dir = Path("/app/ualbf-project/lean4-proofs")
    ir_dir = lean_project_dir / ".lake/build/ir"

    # Make sure we clean up and create the dummy ir_dir
    if ir_dir.exists():
        shutil.rmtree(ir_dir)
    ir_dir.mkdir(parents=True, exist_ok=True)
    dummy_file = ir_dir / "dummy.c"
    dummy_file.write_text("void some_func() {}")

    # Create a temporary directory for mock tools and fake lean sysroot
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)

        # 1. Create a fake LEAN_SYSROOT include directory
        fake_sysroot = tmp_path / "mock_lean_sysroot"
        (fake_sysroot / "include").mkdir(parents=True, exist_ok=True)

        # 2. Create a mock lake executable that succeeds
        mock_lake = tmp_path / "lake"
        mock_lake.write_text("#!/bin/sh\nexit 0\n")
        mock_lake.chmod(0o755)

        # Build environment
        env = os.environ.copy()
        env["LEAN_SYSROOT"] = str(fake_sysroot)
        env["PATH"] = f"{tmpdir}:{env.get('PATH', '')}"

        # Touch build.rs to force cargo to rerun it
        build_rs_path = Path("/app/ualbf-project/rust-engine/build.rs")
        if build_rs_path.exists():
            build_rs_path.touch()

        # Run cargo check in rust-engine
        res = subprocess.run(
            ["cargo", "check"],
            cwd="/app/ualbf-project/rust-engine",
            env=env,
            capture_output=True,
            text=True,
        )

        # The dummy file and directory should have been purged by build script
        assert not dummy_file.exists(), (
            "The intermediate C-IR dummy file was not purged!"
        )
        assert not ir_dir.exists(), (
            "The intermediate C-IR directory was not purged!"
        )

        # The custom panic should NOT be in the output
        assert "FATAL: Lean proof verification failed!" not in res.stderr


def test_guarded_rebuild_failure():
    """
    Test that when lake build fails:
    1. The build system immediately halts.
    2. The custom beautifully-formatted diagnostics are printed to stderr.
    """
    lean_project_dir = Path("/app/ualbf-project/lean4-proofs")
    ir_dir = lean_project_dir / ".lake/build/ir"

    if ir_dir.exists():
        shutil.rmtree(ir_dir)

    # Create a temporary directory for mock tools and fake lean sysroot
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)

        # 1. Create a fake LEAN_SYSROOT include directory
        fake_sysroot = tmp_path / "mock_lean_sysroot"
        (fake_sysroot / "include").mkdir(parents=True, exist_ok=True)

        # 2. Create a mock lake executable that fails
        mock_lake = tmp_path / "lake"
        mock_lake.write_text(
            "#!/bin/sh\n"
            "echo 'Mock lake: simulated build error' >&2\n"
            "exit 1\n"
        )
        mock_lake.chmod(0o755)

        # Build environment
        env = os.environ.copy()
        env["LEAN_SYSROOT"] = str(fake_sysroot)
        env["PATH"] = f"{tmpdir}:{env.get('PATH', '')}"

        # Touch build.rs to force cargo to rerun it
        build_rs_path = Path("/app/ualbf-project/rust-engine/build.rs")
        if build_rs_path.exists():
            build_rs_path.touch()

        # Run cargo check in rust-engine
        res = subprocess.run(
            ["cargo", "check"],
            cwd="/app/ualbf-project/rust-engine",
            env=env,
            capture_output=True,
            text=True,
        )

        # The build must fail
        assert res.returncode != 0, (
            "Cargo check succeeded when it should have failed!"
        )

        # Verify detailed diagnostics and exact rerun command are in stderr
        assert "FATAL: Lean proof verification failed!" in res.stderr
        assert "Proof Logs / Build Directory:" in res.stderr
        assert (
            "To troubleshoot and rerun the verification manually, execute:"
            in res.stderr
        )
        assert "cd lean4-proofs && lake build UALBF" in res.stderr
