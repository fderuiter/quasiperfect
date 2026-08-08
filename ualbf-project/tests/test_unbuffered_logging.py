import os
import re


def test_github_actions_ci_has_pythonunbuffered():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    ci_path = os.path.join(repo_root, ".github/workflows/ci.yml")

    msg = f"ci.yml not found at {ci_path}"
    assert os.path.exists(ci_path), msg

    with open(ci_path, "r") as f:
        content = f.read()

    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    msg = "PYTHONUNBUFFERED: '1' not found in ci.yml env block"
    assert match is not None, msg


def test_github_actions_auto_merge_has_pythonunbuffered():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    auto_merge_path = os.path.join(repo_root, ".github/workflows/auto-merge.yml")

    msg = f"auto-merge.yml not found at {auto_merge_path}"
    assert os.path.exists(auto_merge_path), msg

    with open(auto_merge_path, "r") as f:
        content = f.read()

    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    msg = "PYTHONUNBUFFERED: '1' not found in auto-merge.yml env block"
    assert match is not None, msg


def test_makefile_has_pythonunbuffered():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    makefile_path = os.path.join(repo_root, "ualbf-project/Makefile")

    msg = f"Makefile not found at {makefile_path}"
    assert os.path.exists(makefile_path), msg

    with open(makefile_path, "r") as f:
        content = f.read()

    match = re.search(r"export\s+PYTHONUNBUFFERED\s*=\s*1", content)
    msg = "export PYTHONUNBUFFERED=1 not found in Makefile"
    assert match is not None, msg

