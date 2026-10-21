#!/usr/bin/env python3
"""
UALBF Centralized Python Quality Verification Wrapper
===================================================

Orchestrates Python code quality, style, typing, and verification passes:
1. Mandatory code literal checks (scripts/check_literals.py)
2. Mandatory metadata verification (verify_metadata.py)
3. Optional linter passes (black, flake8, mypy) using importlib.util.find_spec
   and python3 -m execution.
4. Mandatory documentation specification checks (scripts/validate_docs.py --check-specs)
5. Mandatory tuning guide validation (scripts/validate_tuning_guide.py)
6. Mandatory unit test execution (pytest tests paper)

If an optional linter module (black, flake8, mypy) is absent from the Python
environment, a diagnostic warning is emitted detailing installation instructions,
and the pass is safely skipped without aborting check-python. If an installed
linter reports violations, or if any mandatory check fails, the process exits
with code 1.
"""

import importlib.util
import os
import shutil
import subprocess
import sys
from pathlib import Path


def resolve_project_root() -> Path:
    """Resolve the project root directory (ualbf-project)."""
    script_path = Path(__file__).resolve()
    # Script is located in <project_root>/scripts/check_python_quality.py
    project_root = script_path.parent.parent
    return project_root


def run_mandatory_script(cmd: list[str], description: str, cwd: Path) -> bool:
    """Run a mandatory Python script/command. Return True if successful."""
    print(f"==> [MANDATORY] Running {description}: {' '.join(cmd)}")
    res = subprocess.run(cmd, cwd=cwd)
    if res.returncode != 0:
        print(
            f"[ERROR] Mandatory check '{description}' failed with exit code {res.returncode}.",
            file=sys.stderr,
        )
        return False
    print(f"[PASS] {description}")
    return True


def run_optional_linter(module_name: str, args: list[str], cwd: Path) -> bool:
    """
    Check if optional linter module is installed via importlib.util.find_spec.
    If present, run via python3 -m <module_name> <args>.
    If missing, display a warning with installation instructions and skip.
    Return False if installed linter reports violations (non-zero exit), True otherwise.
    """
    spec = importlib.util.find_spec(module_name)
    if spec is None:
        print(
            f"[WARNING] Optional linter module '{module_name}' is not installed in the active Python environment."
        )
        print(
            f"          Skipping '{module_name}' check. To install: pip install {module_name}"
        )
        return True

    cmd = [sys.executable, "-m", module_name] + args
    print(f"==> [OPTIONAL LINTER] Running {module_name}: {' '.join(cmd)}")
    res = subprocess.run(cmd, cwd=cwd)
    if res.returncode != 0:
        print(
            f"[ERROR] Optional linter '{module_name}' reported violations (exit code {res.returncode}).",
            file=sys.stderr,
        )
        return False
    print(f"[PASS] {module_name}")
    return True


def run_unit_tests(cwd: Path) -> bool:
    """Run mandatory unit test suite via pytest."""
    description = "pytest unit test suite"
    print(f"==> [MANDATORY] Running {description}")

    pytest_spec = importlib.util.find_spec("pytest")
    pytest_cov_spec = importlib.util.find_spec("pytest_cov")

    cmd = []
    if pytest_spec is not None:
        cmd = [sys.executable, "-m", "pytest"]
    else:
        pytest_bin = shutil.which("pytest")
        if pytest_bin:
            cmd = [pytest_bin]
        else:
            print(
                "[ERROR] Mandatory check 'pytest' is not installed or found in PATH.",
                file=sys.stderr,
            )
            print("        To install pytest, run: pip install pytest", file=sys.stderr)
            return False

    if pytest_cov_spec is None:
        cmd.extend(["-o", "addopts="])

    cmd.extend(["tests", "paper"])

    print(f"Executing: {' '.join(cmd)}")
    res = subprocess.run(cmd, cwd=cwd)
    if res.returncode != 0:
        print(
            f"[ERROR] Unit tests failed with exit code {res.returncode}.",
            file=sys.stderr,
        )
        return False
    print(f"[PASS] {description}")
    return True


def main() -> int:
    project_root = resolve_project_root()

    # Ensure PATH contains ~/.local/bin if present
    user_local_bin = Path.home() / ".local" / "bin"
    if user_local_bin.exists() and str(user_local_bin) not in os.environ.get(
        "PATH", ""
    ):
        os.environ["PATH"] = f"{user_local_bin}:{os.environ.get('PATH', '')}"

    failures = 0

    # 1. Mandatory: Check hardcoded literals
    if not run_mandatory_script(
        [sys.executable, "scripts/check_literals.py"], "Check Literals", project_root
    ):
        failures += 1

    # 2. Mandatory: Verify metadata
    if not run_mandatory_script(
        [sys.executable, "verify_metadata.py"], "Verify Metadata", project_root
    ):
        failures += 1

    # 3. Optional Linters: black, flake8, mypy
    optional_linters = [
        ("black", ["--check", "."]),
        ("flake8", ["."]),
        ("mypy", ["."]),
    ]

    for module_name, linter_args in optional_linters:
        if not run_optional_linter(module_name, linter_args, project_root):
            failures += 1

    # 4. Mandatory: Validate docs & specs
    if not run_mandatory_script(
        [sys.executable, "scripts/validate_docs.py", "--check-specs"],
        "Validate Docs Specs",
        project_root,
    ):
        failures += 1

    # 5. Mandatory: Validate tuning guide
    if not run_mandatory_script(
        [sys.executable, "scripts/validate_tuning_guide.py"],
        "Validate Tuning Guide",
        project_root,
    ):
        failures += 1

    # 6. Mandatory: Run unit tests
    if not run_unit_tests(project_root):
        failures += 1

    if failures > 0:
        print(
            f"\n[FAILURE] Python quality check failed with {failures} error(s).",
            file=sys.stderr,
        )
        return 1

    print("\n[SUCCESS] All Python quality checks passed successfully!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
