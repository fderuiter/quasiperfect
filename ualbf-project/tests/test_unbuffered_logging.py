import os
import re


def test_github_actions_ci_has_pythonunbuffered():
    ci_path = "/app/.github/workflows/ci.yml"
    if not os.path.exists(ci_path):
        base_dir = os.path.dirname(__file__)
        ci_path = os.path.join(base_dir, "../../.github/workflows/ci.yml")

    msg = f"ci.yml not found at {ci_path}"
    assert os.path.exists(ci_path), msg

    with open(ci_path, "r") as f:
        content = f.read()

    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    msg = "PYTHONUNBUFFERED: '1' not found in ci.yml env block"
    assert match is not None, msg


def test_github_actions_auto_merge_has_pythonunbuffered():
    auto_merge_path = "/app/.github/workflows/auto-merge.yml"
    if not os.path.exists(auto_merge_path):
        base_dir = os.path.dirname(__file__)
        auto_merge_path = os.path.join(
            base_dir, "../../.github/workflows/auto-merge.yml"
        )

    msg = f"auto-merge.yml not found at {auto_merge_path}"
    assert os.path.exists(auto_merge_path), msg

    with open(auto_merge_path, "r") as f:
        content = f.read()

    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    msg = "PYTHONUNBUFFERED: '1' not found in auto-merge.yml env block"
    assert match is not None, msg


def test_makefile_has_pythonunbuffered():
    makefile_path = "/app/ualbf-project/Makefile"
    if not os.path.exists(makefile_path):
        base_dir = os.path.dirname(__file__)
        makefile_path = os.path.join(base_dir, "../Makefile")

    msg = f"Makefile not found at {makefile_path}"
    assert os.path.exists(makefile_path), msg

    with open(makefile_path, "r") as f:
        content = f.read()

    match = re.search(r"export\s+PYTHONUNBUFFERED\s*=\s*1", content)
    msg = "export PYTHONUNBUFFERED=1 not found in Makefile"
    assert match is not None, msg
