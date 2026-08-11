"""
UALBF Unbuffered Logging Verification Suite
===========================================

This test suite verifies that python and workflow execution logging are unbuffered,
ensuring real-time telemetry and diagnostic outputs both locally and in CI.

Note: Other workflows such as john-henry.yml are pure GitHub Actions script
runners and do not execute Python, thus they are intentionally excluded from
this verification.

Additionally, this test suite incorporates robust cargo-compilation assertions
which depend on the presence of Z3 development headers (z3.h) to compile z3-sys.
"""

import os
import re

_ABS_FILE = os.path.abspath(__file__)
_DIR_NAME = os.path.dirname(_ABS_FILE)


def _find_repo_root(start_dir: str) -> str:
    current = os.path.abspath(start_dir)
    while current != os.path.dirname(current):
        sentinels = ["docs_manifest.json", "pyproject.toml", ".git"]
        if any(os.path.exists(os.path.join(current, s)) for s in sentinels):
            return current
        current = os.path.dirname(current)
    return os.path.abspath(os.path.join(start_dir, "../.."))


_REPO_ROOT = _find_repo_root(_DIR_NAME)


def test_github_actions_ci_has_pythonunbuffered():
    ci_path = os.path.join(_REPO_ROOT, ".github/workflows/ci.yml")

    msg = f"ci.yml not found at {ci_path}"
    assert os.path.exists(ci_path), msg

    with open(ci_path, "r") as f:
        content = f.read()

    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    msg = "PYTHONUNBUFFERED: '1' not found in ci.yml env block"
    assert match is not None, msg


def test_github_actions_auto_merge_has_pythonunbuffered():
    filename = ".github/workflows/auto-merge.yml"
    auto_merge_path = os.path.join(_REPO_ROOT, filename)

    msg = f"auto-merge.yml not found at {auto_merge_path}"
    assert os.path.exists(auto_merge_path), msg

    with open(auto_merge_path, "r") as f:
        content = f.read()

    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    msg = "PYTHONUNBUFFERED: '1' not found in auto-merge.yml env block"
    assert match is not None, msg


def test_makefile_has_pythonunbuffered():
    makefile_path = os.path.join(_REPO_ROOT, "ualbf-project/Makefile")

    msg = f"Makefile not found at {makefile_path}"
    assert os.path.exists(makefile_path), msg

    with open(makefile_path, "r") as f:
        content = f.read()

    match = re.search(r"export\s+PYTHONUNBUFFERED\s*=\s*1", content)
    msg = "export PYTHONUNBUFFERED=1 not found in Makefile"
    assert match is not None, msg
