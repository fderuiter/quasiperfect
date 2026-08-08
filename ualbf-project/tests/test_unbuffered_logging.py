import os
import re

def test_github_actions_ci_has_pythonunbuffered():
    ci_path = "/app/.github/workflows/ci.yml"
    if not os.path.exists(ci_path):
        ci_path = os.path.join(os.path.dirname(__file__), "../../.github/workflows/ci.yml")
    
    assert os.path.exists(ci_path), f"ci.yml not found at {ci_path}"
    
    with open(ci_path, "r") as f:
        content = f.read()
    
    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    assert match is not None, "PYTHONUNBUFFERED: '1' not found in ci.yml env block"


def test_github_actions_auto_merge_has_pythonunbuffered():
    auto_merge_path = "/app/.github/workflows/auto-merge.yml"
    if not os.path.exists(auto_merge_path):
        auto_merge_path = os.path.join(os.path.dirname(__file__), "../../.github/workflows/auto-merge.yml")
        
    assert os.path.exists(auto_merge_path), f"auto-merge.yml not found at {auto_merge_path}"
    
    with open(auto_merge_path, "r") as f:
        content = f.read()
        
    match = re.search(r"PYTHONUNBUFFERED\s*:\s*['\"]?1['\"]?", content)
    assert match is not None, "PYTHONUNBUFFERED: '1' not found in auto-merge.yml env block"


def test_makefile_has_pythonunbuffered():
    makefile_path = "/app/ualbf-project/Makefile"
    if not os.path.exists(makefile_path):
        makefile_path = os.path.join(os.path.dirname(__file__), "../Makefile")
        
    assert os.path.exists(makefile_path), f"Makefile not found at {makefile_path}"
    
    with open(makefile_path, "r") as f:
        content = f.read()
        
    match = re.search(r"export\s+PYTHONUNBUFFERED\s*=\s*1", content)
    assert match is not None, "export PYTHONUNBUFFERED=1 not found in Makefile"
