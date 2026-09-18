"""
Unit tests for scripts/validate_docs.py
========================================

Covers:
  - Missing docs_manifest.json error handling
  - Malformed JSON in docs_manifest.json
  - Detection of unregistered markdown files
  - Build and virtual environment directory exclusions
  - PR argument file parsing and authoritative document flags
"""

import io
import json
import os
import sys
import types
import pytest

# Ensure ualbf-project root is on sys.path
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

import scripts.validate_docs as validate_docs  # noqa: E402


def _setup_mock_repo(
    tmp_path, manifest_data=None, create_manifest=True, malformed_manifest=False
):
    """Set up a mock repository structure under tmp_path with scripts/validate_docs.py present."""
    repo_root = tmp_path
    ualbf_dir = repo_root / "ualbf-project"
    scripts_dir = ualbf_dir / "scripts"
    scripts_dir.mkdir(parents=True, exist_ok=True)

    # Copy/place validate_docs.py in the mock scripts dir so os.path.dirname(__file__) resolves relative to tmp_path
    mock_validate_docs_path = scripts_dir / "validate_docs.py"
    with open(validate_docs.__file__, "r", encoding="utf-8") as f:
        mock_validate_docs_path.write_text(f.read(), encoding="utf-8")

    # Mock validate_tuning_guide in mock scripts dir
    mock_tuning_guide = scripts_dir / "validate_tuning_guide.py"
    mock_tuning_guide.write_text(
        "def validate_tuning_guide(path):\n    return True\n", encoding="utf-8"
    )

    manifest_path = repo_root / "docs_manifest.json"
    if create_manifest:
        if malformed_manifest:
            manifest_path.write_text("{ malformed json ...", encoding="utf-8")
        else:
            if manifest_data is None:
                manifest_data = {
                    "README.md": "authoritative",
                    "DOCS.md": "informal",
                }
            manifest_path.write_text(
                json.dumps(manifest_data, indent=2), encoding="utf-8"
            )

    return repo_root, mock_validate_docs_path


def test_missing_docs_manifest(tmp_path, monkeypatch):
    repo_root, mock_script = _setup_mock_repo(tmp_path, create_manifest=False)

    monkeypatch.setattr(sys, "argv", [str(mock_script)])
    monkeypatch.setattr(validate_docs, "__file__", str(mock_script))

    captured_err = io.StringIO()
    monkeypatch.setattr(sys, "stderr", captured_err)

    with pytest.raises(SystemExit) as exc_info:
        validate_docs.main()

    assert exc_info.value.code == 1
    assert "docs_manifest.json not found" in captured_err.getvalue()


def test_malformed_docs_manifest(tmp_path, monkeypatch):
    repo_root, mock_script = _setup_mock_repo(
        tmp_path, create_manifest=True, malformed_manifest=True
    )

    monkeypatch.setattr(sys, "argv", [str(mock_script)])
    monkeypatch.setattr(validate_docs, "__file__", str(mock_script))

    captured_err = io.StringIO()
    monkeypatch.setattr(sys, "stderr", captured_err)

    with pytest.raises(SystemExit) as exc_info:
        validate_docs.main()

    assert exc_info.value.code == 1
    assert "Invalid JSON in docs_manifest.json" in captured_err.getvalue()


def test_unregistered_markdown_file_detected(tmp_path, monkeypatch):
    manifest = {"README.md": "authoritative"}
    repo_root, mock_script = _setup_mock_repo(tmp_path, manifest_data=manifest)

    # Create registered README.md and unregistered UNREGISTERED.md
    (repo_root / "README.md").write_text("# README", encoding="utf-8")
    (repo_root / "UNREGISTERED.md").write_text("# Unregistered", encoding="utf-8")

    monkeypatch.setattr(sys, "argv", [str(mock_script)])
    monkeypatch.setattr(validate_docs, "__file__", str(mock_script))

    captured_err = io.StringIO()
    monkeypatch.setattr(sys, "stderr", captured_err)

    with pytest.raises(SystemExit) as exc_info:
        validate_docs.main()

    assert exc_info.value.code == 1
    assert "UNREGISTERED.md" in captured_err.getvalue()


def test_build_directories_excluded(tmp_path, monkeypatch):
    manifest = {"README.md": "authoritative"}
    repo_root, mock_script = _setup_mock_repo(tmp_path, manifest_data=manifest)

    (repo_root / "README.md").write_text("# README", encoding="utf-8")

    # Create markdown files inside build directories that should be excluded
    exclude_dirs = [
        ".lake",
        "target",
        "node_modules",
        "build",
        ".git",
        "venv",
        ".venv",
        ".direnv",
        "lean-built",
        "result",
        "test-env",
    ]
    for ed in exclude_dirs:
        d = repo_root / ed
        d.mkdir(parents=True, exist_ok=True)
        (d / "excluded.md").write_text("# Excluded", encoding="utf-8")

    monkeypatch.setattr(sys, "argv", [str(mock_script)])
    monkeypatch.setattr(validate_docs, "__file__", str(mock_script))

    # Should run and exit normally without error (no SystemExit or exit code 0)
    # validate_docs.main() does not exit when all files are registered unless pr_files requires it
    # We mock validate_tuning_guide to avoid needing full project structure
    monkeypatch.syspath_prepend(str(repo_root / "ualbf-project" / "scripts"))

    validate_docs.main()


def test_pr_argument_parsing_and_authoritative_doc(tmp_path, monkeypatch):
    manifest = {
        "README.md": "authoritative",
        "GUIDE.md": "informal",
    }
    repo_root, mock_script = _setup_mock_repo(tmp_path, manifest_data=manifest)

    (repo_root / "README.md").write_text("# README", encoding="utf-8")
    (repo_root / "GUIDE.md").write_text("# GUIDE", encoding="utf-8")

    # Case 1: PR argument file contains registered authoritative document
    pr_file = repo_root / "pr_modified.txt"
    pr_file.write_text("README.md\nGUIDE.md\n", encoding="utf-8")

    monkeypatch.setattr(sys, "argv", [str(mock_script), str(pr_file)])
    monkeypatch.setattr(validate_docs, "__file__", str(mock_script))

    captured_out = io.StringIO()
    monkeypatch.setattr(sys, "stdout", captured_out)

    validate_docs.main()

    out = captured_out.getvalue()
    assert "Authoritative document modified: README.md" in out
    assert "AUTHORITATIVE_TOUCHED=1" in out


def test_pr_argument_unregistered_file_causes_exit(tmp_path, monkeypatch):
    manifest = {
        "README.md": "authoritative",
    }
    repo_root, mock_script = _setup_mock_repo(tmp_path, manifest_data=manifest)

    (repo_root / "README.md").write_text("# README", encoding="utf-8")

    pr_file = repo_root / "pr_modified.txt"
    pr_file.write_text("README.md\nNEW_DOC.md\n", encoding="utf-8")

    monkeypatch.setattr(sys, "argv", [str(mock_script), str(pr_file)])
    monkeypatch.setattr(validate_docs, "__file__", str(mock_script))

    captured_err = io.StringIO()
    monkeypatch.setattr(sys, "stderr", captured_err)

    with pytest.raises(SystemExit) as exc_info:
        validate_docs.main()

    assert exc_info.value.code == 1
    assert (
        "PR introduces a documentation file 'NEW_DOC.md' not registered in docs_manifest.json"
        in captured_err.getvalue()
    )
