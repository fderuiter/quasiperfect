import importlib.util
import os
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

# Add ualbf-project/scripts to sys.path so we can import check_python_quality directly
project_root = Path(__file__).resolve().parent.parent
scripts_dir = project_root / "scripts"
if str(scripts_dir) not in sys.path:
    sys.path.insert(0, str(scripts_dir))

import check_python_quality


def test_resolve_project_root():
    root = check_python_quality.resolve_project_root()
    assert (root / "scripts" / "check_python_quality.py").exists()


def test_run_optional_linter_missing_module(capsys):
    with patch("importlib.util.find_spec", return_value=None):
        result = check_python_quality.run_optional_linter(
            "non_existent_linter_xyz", ["."], project_root
        )
        assert result is True  # Missing linter should skip without failing
        captured = capsys.readouterr()
        assert "[WARNING]" in captured.out
        assert "non_existent_linter_xyz" in captured.out
        assert "pip install non_existent_linter_xyz" in captured.out


def test_run_optional_linter_present_and_succeeds():
    mock_spec = MagicMock()
    mock_completed_proc = MagicMock(returncode=0)
    with patch("importlib.util.find_spec", return_value=mock_spec):
        with patch("subprocess.run", return_value=mock_completed_proc) as mock_run:
            result = check_python_quality.run_optional_linter(
                "black", ["--check", "."], project_root
            )
            assert result is True
            mock_run.assert_called_once()
            args = mock_run.call_args[0][0]
            assert args[0] == sys.executable
            assert args[1:4] == ["-m", "black", "--check"]


def test_run_optional_linter_present_and_fails(capsys):
    mock_spec = MagicMock()
    mock_completed_proc = MagicMock(returncode=1)
    with patch("importlib.util.find_spec", return_value=mock_spec):
        with patch("subprocess.run", return_value=mock_completed_proc):
            result = check_python_quality.run_optional_linter(
                "mypy", ["."], project_root
            )
            assert result is False  # Failed linter should return False
            captured = capsys.readouterr()
            assert "[ERROR]" in captured.err
            assert "mypy" in captured.err


def test_run_mandatory_script_success():
    mock_completed_proc = MagicMock(returncode=0)
    with patch("subprocess.run", return_value=mock_completed_proc):
        result = check_python_quality.run_mandatory_script(
            [sys.executable, "verify_metadata.py"], "Verify Metadata", project_root
        )
        assert result is True


def test_run_mandatory_script_failure(capsys):
    mock_completed_proc = MagicMock(returncode=2)
    with patch("subprocess.run", return_value=mock_completed_proc):
        result = check_python_quality.run_mandatory_script(
            [sys.executable, "scripts/check_literals.py"], "Check Literals", project_root
        )
        assert result is False
        captured = capsys.readouterr()
        assert "[ERROR]" in captured.err


def test_main_execution():
    # Mocking individual check runners to verify main coordination logic
    with patch("check_python_quality.run_mandatory_script", return_value=True), \
         patch("check_python_quality.run_optional_linter", return_value=True), \
         patch("check_python_quality.run_unit_tests", return_value=True):
        exit_code = check_python_quality.main()
        assert exit_code == 0


def test_main_execution_failure():
    with patch("check_python_quality.run_mandatory_script", return_value=False), \
         patch("check_python_quality.run_optional_linter", return_value=True), \
         patch("check_python_quality.run_unit_tests", return_value=True):
        exit_code = check_python_quality.main()
        assert exit_code == 1
