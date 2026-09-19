"""
Unit tests for scripts/validate_docs.py
========================================

Covers:
  - Missing docs_manifest.json error handling
  - Malformed JSON in docs_manifest.json
  - Detection of unregistered markdown files
  - Build and virtual environment directory exclusions
  - PR argument file parsing and authoritative document flags
  - Documentation link and anchor validation
  - Specification synchronization verification
"""

import io
import json
import os
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest import mock

import pytest

# Ensure ualbf-project root and scripts directory are on sys.path
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

scripts_dir = os.path.join(project_root, "scripts")
if scripts_dir not in sys.path:
    sys.path.insert(0, scripts_dir)

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
        "lake-packages",
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

    monkeypatch.syspath_prepend(str(repo_root / "ualbf-project" / "scripts"))
    monkeypatch.setattr(validate_docs, "validate_spec_sync", lambda repo_root: True)

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


class TestSlugifyAndAnchorExtraction(unittest.TestCase):
    def test_slugify(self):
        self.assertEqual(validate_docs.slugify("Overview"), "overview")
        self.assertEqual(
            validate_docs.slugify("1. Quick Start & Setup!"), "1-quick-start-setup"
        )
        self.assertEqual(validate_docs.slugify("Heading <a name='tag'></a>"), "heading")

    def test_extract_anchors(self):
        content = """# Main Heading

## Sub Heading

```markdown
# Ignored Code Heading
```

<a name="custom-html-anchor"></a>
### Section 2.1 (Details)
"""
        anchors = validate_docs.extract_anchors(content)
        self.assertIn("main-heading", anchors)
        self.assertIn("sub-heading", anchors)
        self.assertIn("custom-html-anchor", anchors)
        self.assertIn("section-21-details", anchors)
        self.assertNotIn("ignored-code-heading", anchors)

    def test_extract_links(self):
        content = """
Here is a [valid link](relative/doc.md#section).
![An image](images/logo.png)

```markdown
[Ignored in code](ignored.md)
```
"""
        links = validate_docs.extract_links(content)
        self.assertEqual(len(links), 2)
        self.assertEqual(links[0][1], "valid link")
        self.assertEqual(links[0][2], "relative/doc.md#section")
        self.assertEqual(links[1][1], "An image")
        self.assertEqual(links[1][2], "images/logo.png")


class TestLinkValidation(unittest.TestCase):
    def test_validate_markdown_links_success(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            doc1 = tmp_path / "doc1.md"
            doc2 = tmp_path / "doc2.md"

            doc1.write_text(
                "# Doc 1\n\nLink to [Doc 2](doc2.md#section-title).\nLink to [Self](#doc-1).\nLink to [External](https://example.com).\n",
                encoding="utf-8",
            )
            doc2.write_text(
                "# Doc 2\n\n## Section Title\n\nContent.\n", encoding="utf-8"
            )

            registered = ["doc1.md", "doc2.md"]
            valid = validate_docs.validate_markdown_links(tmpdir, registered)
            self.assertTrue(valid)

    def test_validate_markdown_links_missing_file(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            doc1 = tmp_path / "doc1.md"
            doc1.write_text("[Missing File](non_existent.md)\n", encoding="utf-8")

            registered = ["doc1.md"]
            valid = validate_docs.validate_markdown_links(tmpdir, registered)
            self.assertFalse(valid)

    def test_validate_markdown_links_missing_anchor(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            doc1 = tmp_path / "doc1.md"
            doc2 = tmp_path / "doc2.md"

            doc1.write_text(
                "[Bad Anchor](doc2.md#non-existent-anchor)\n", encoding="utf-8"
            )
            doc2.write_text("# Doc 2\n\n## Existing Section\n", encoding="utf-8")

            registered = ["doc1.md", "doc2.md"]
            valid = validate_docs.validate_markdown_links(tmpdir, registered)
            self.assertFalse(valid)

    def test_validate_markdown_links_skips_line_anchors_and_code_files(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            doc1 = tmp_path / "doc1.md"
            code1 = tmp_path / "Main.lean"

            doc1.write_text(
                "[Code line reference](Main.lean#L100-L120)\n", encoding="utf-8"
            )
            code1.write_text(
                'def main : IO Unit := IO.println "hello"\n', encoding="utf-8"
            )

            registered = ["doc1.md"]
            valid = validate_docs.validate_markdown_links(tmpdir, registered)
            self.assertTrue(valid)


class TestSpecSyncValidation(unittest.TestCase):
    def test_validate_spec_sync_pass(self):
        repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
        # Run validate_spec_sync on clean repo root
        result = validate_docs.validate_spec_sync(repo_root)
        self.assertTrue(result)

    def test_validate_spec_sync_detects_mismatch(self):
        repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
        bounds_path = os.path.join(repo_root, "ualbf-project", "bounds_manifest.json")
        spec_full = os.path.join(
            repo_root, "ualbf-project", "rust-engine", "src", "ffi_generated.rs"
        )

        original_bounds = Path(bounds_path).read_text(encoding="utf-8")
        original_spec = Path(spec_full).read_text(encoding="utf-8")
        try:
            # Modify bounds manifest content
            data = json.loads(original_bounds)
            data["omega_bounds"]["prasad_sunitha"]["proof_bound"] = 99
            Path(bounds_path).write_text(json.dumps(data, indent=2), encoding="utf-8")

            # validate_spec_sync should detect mismatch and restore original generated spec files
            result = validate_docs.validate_spec_sync(repo_root)
            self.assertFalse(result)

            # Confirm original generated spec file content was preserved
            restored_spec = Path(spec_full).read_text(encoding="utf-8")
            self.assertEqual(restored_spec, original_spec)
        finally:
            Path(bounds_path).write_text(original_bounds, encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
