import json
import subprocess
import sys
from pathlib import Path

from scripts.validate_tuning_guide import validate_tuning_guide, extract_tuning_parameter_headers


def test_validate_tuning_guide_success():
    """Verify that validate_tuning_guide passes on clean repository files."""
    project_root = Path(__file__).resolve().parent.parent
    assert validate_tuning_guide(project_root) is True


def test_validate_tuning_guide_duplicate_header_fails(tmp_path):
    """Verify that duplicate parameter headers cause validation to fail."""
    project_dir = tmp_path / "ualbf-project"
    project_dir.mkdir()
    rust_engine_dir = project_dir / "rust-engine"
    rust_engine_dir.mkdir()

    # Copy actual bounds and profile templates
    orig_project = Path(__file__).resolve().parent.parent
    (project_dir / "bounds_manifest.json").write_text(
        (orig_project / "bounds_manifest.json").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    (rust_engine_dir / "profile.json.template").write_text(
        (orig_project / "rust-engine" / "profile.json.template").read_text(
            encoding="utf-8"
        ),
        encoding="utf-8",
    )

    # Read clean TUNING.md and inject a duplicate header
    clean_tuning = (orig_project / "TUNING.md").read_text(encoding="utf-8")
    duplicate_tuning = clean_tuning + "\n- **active_prime_slots**\n  - **Description**: Duplicate entry.\n"
    (project_dir / "TUNING.md").write_text(duplicate_tuning, encoding="utf-8")

    assert validate_tuning_guide(project_dir) is False


def test_validate_tuning_guide_nonexistent_manifest_key_fails(tmp_path):
    """Verify that non-existent/unknown manifest keys cause validation to fail."""
    project_dir = tmp_path / "ualbf-project"
    project_dir.mkdir()
    rust_engine_dir = project_dir / "rust-engine"
    rust_engine_dir.mkdir()

    orig_project = Path(__file__).resolve().parent.parent
    (project_dir / "bounds_manifest.json").write_text(
        (orig_project / "bounds_manifest.json").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    (rust_engine_dir / "profile.json.template").write_text(
        (orig_project / "rust-engine" / "profile.json.template").read_text(
            encoding="utf-8"
        ),
        encoding="utf-8",
    )

    clean_tuning = (orig_project / "TUNING.md").read_text(encoding="utf-8")
    invalid_tuning = clean_tuning + "\n- **nonexistent_param_key**\n  - **Description**: Invalid key.\n"
    (project_dir / "TUNING.md").write_text(invalid_tuning, encoding="utf-8")

    assert validate_tuning_guide(project_dir) is False


def test_validate_tuning_guide_cli():
    """Verify that executing scripts/validate_tuning_guide.py via subprocess returns exit code 0."""
    project_root = Path(__file__).resolve().parent.parent
    script_path = project_root / "scripts" / "validate_tuning_guide.py"

    res = subprocess.run(
        [sys.executable, str(script_path)],
        cwd=str(project_root),
        capture_output=True,
        text=True,
    )
    assert res.returncode == 0
    assert "Tuning Guide Validation Passed" in res.stdout
