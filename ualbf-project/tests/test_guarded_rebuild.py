import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def test_guarded_rebuild_success():
    """
    Test that when lake build succeeds:
    1. The targeted intermediate C-IR directories/files (like UALBF/ and Validator.c) are deleted.
    2. The build proceeds beyond lake check (even if it eventually fails on
       missing files, the custom 'FATAL: Lean proof verification failed!'
       panic is NOT triggered).
    """
    project_dir = Path(__file__).parent.parent
    lean_project_dir = project_dir / "lean4-proofs"
    ir_dir = lean_project_dir / ".lake/build/ir"

    # Make sure we clean up only our targeted items within ir_dir to avoid destroying other precompiled objects
    ir_dir.mkdir(parents=True, exist_ok=True)
    ualbf_dir = ir_dir / "UALBF"
    if ualbf_dir.exists():
        shutil.rmtree(ualbf_dir)
    ualbf_dir.mkdir(parents=True, exist_ok=True)
    dummy_file = ualbf_dir / "dummy.c"
    dummy_file.write_text("void some_func() {}")
    
    validator_file = ir_dir / "Validator.c"
    if validator_file.exists():
        os.remove(validator_file)
    validator_file.write_text("void some_func() {}")

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
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()

        # Run cargo check in rust-engine
        res = subprocess.run(
            ["cargo", "check"],
            cwd=str(project_dir / "rust-engine"),
            env=env,
            capture_output=True,
            text=True,
        )

        # The dummy file and targeted directories/files should have been purged by build script
        assert (
            not dummy_file.exists()
        ), "The intermediate C-IR dummy file was not purged!"
        assert not ualbf_dir.exists(), "The intermediate UALBF directory was not purged!"
        assert not validator_file.exists(), "The intermediate Validator.c file was not purged!"

        # The custom panic should NOT be in the output
        assert "FATAL: Lean proof verification failed!" not in res.stderr


def test_guarded_rebuild_failure():
    """
    Test that when lake build fails:
    1. The build system immediately halts.
    2. The custom beautifully-formatted diagnostics are printed to stderr.
    """
    project_dir = Path(__file__).parent.parent
    lean_project_dir = project_dir / "lean4-proofs"
    ir_dir = lean_project_dir / ".lake/build/ir"

    # Do not destroy the entire precompiled ir_dir. Only clean up our own targets if present.
    ualbf_dir = ir_dir / "UALBF"
    if ualbf_dir.exists():
        shutil.rmtree(ualbf_dir)
    validator_file = ir_dir / "Validator.c"
    if validator_file.exists():
        os.remove(validator_file)

    # Create a temporary directory for mock tools and fake lean sysroot
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)

        # 1. Create a fake LEAN_SYSROOT include directory
        fake_sysroot = tmp_path / "mock_lean_sysroot"
        (fake_sysroot / "include").mkdir(parents=True, exist_ok=True)

        # 2. Create a mock lake executable that fails
        mock_lake = tmp_path / "lake"
        mock_lake.write_text(
            "#!/bin/sh\n" "echo 'Mock lake: simulated build error' >&2\n" "exit 1\n"
        )
        mock_lake.chmod(0o755)

        # Build environment
        env = os.environ.copy()
        env["LEAN_SYSROOT"] = str(fake_sysroot)
        env["PATH"] = f"{tmpdir}:{env.get('PATH', '')}"

        # Touch build.rs to force cargo to rerun it
        build_rs_path = project_dir / "rust-engine/build.rs"
        if build_rs_path.exists():
            build_rs_path.touch()

        # Run cargo check in rust-engine
        res = subprocess.run(
            ["cargo", "check"],
            cwd=str(project_dir / "rust-engine"),
            env=env,
            capture_output=True,
            text=True,
        )

        # The build must fail
        assert res.returncode != 0, "Cargo check succeeded when it should have failed!"

        # Verify detailed diagnostics and exact rerun command are in stderr
        assert "FATAL: Lean proof verification failed!" in res.stderr
        assert "Proof Logs / Build Directory:" in res.stderr
        assert (
            "To troubleshoot and rerun the verification manually, execute:"
            in res.stderr
        )
        assert "cd lean4-proofs && lake build UALBF" in res.stderr
