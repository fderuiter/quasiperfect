"""
Unit tests for scripts/check_literals.py
=========================================

Covers:
  - Lean comment stripping (line comments '--' and block comments '/- ... -/')
  - Bounds extraction from nested JSON structures with boolean filtering and ignored bounds
  - Forbidden Rust literal pattern regex matching
  - Lean literal scanning against bounds manifest
  - Full check_literals.main() execution
"""

import io
import json
import os
import re
import sys
import pytest

# Ensure ualbf-project root is on sys.path
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

import scripts.check_literals as check_literals  # noqa: E402
from scripts.check_literals import strip_comments, FORBIDDEN_PATTERNS  # noqa: E402


def test_strip_comments_line_comments():
    code = "def foo : Nat := 42 -- this is a line comment\n  + 1"
    cleaned = strip_comments(code)
    assert "this is a line comment" not in cleaned
    assert "def foo : Nat := 42" in cleaned
    assert "+ 1" in cleaned


def test_strip_comments_block_comments():
    code = "def bar : Nat := /- block comment -/ 10"
    cleaned = strip_comments(code)
    assert "block comment" not in cleaned
    assert "def bar : Nat :=  10" in cleaned

    multiline_code = "def baz : Nat :=\n/- multi\nline\ncomment -/\n  100"
    cleaned_multi = strip_comments(multiline_code)
    assert "multi" not in cleaned_multi
    assert "comment" not in cleaned_multi
    assert "def baz : Nat :=" in cleaned_multi
    assert "100" in cleaned_multi


def test_extract_bounds_nested_dict():
    nested_manifest = {
        "search_bounds": {
            "target_min_log10": {"value": 35},
            "target_max_log10": {"value": 37},
            "custom_bound": 9999,
            "is_active": True,  # Bool should be ignored
            "label": "test",
        },
        "euler_ceiling": 100,
        "omega_bounds": {
            "bound_a": 15,
            "bound_b": 43,
        },
    }

    bounds = set()

    def extract_bounds_local(d):
        for v in d.values():
            if isinstance(v, dict):
                extract_bounds_local(v)
            elif isinstance(v, int) and not isinstance(v, bool):
                bounds.add(str(v))

    extract_bounds_local(nested_manifest)

    # 35, 37, 100 are in ignored_bounds
    ignored_bounds = {
        "0",
        "4",
        "7",
        "35",
        "37",
        "128",
        "100000",
        "30",
        "11",
        "3",
        "19",
        "24",
        "1155",
        "5",
        "1",
        "2973",
        "59",
        "61",
        "67",
        "69",
    }
    filtered = bounds - ignored_bounds

    assert "9999" in filtered
    assert "15" in filtered
    assert "43" in filtered
    assert "True" not in filtered
    assert "test" not in filtered


def test_forbidden_patterns_matching():
    # Test positive matches
    sample_bad_1 = "pub const TARGET_ABUNDANCE : f64 = 2.0;"
    assert any(re.search(pat, sample_bad_1) for pat in FORBIDDEN_PATTERNS)

    sample_bad_2 = "let gpu_threshold = 1000;"
    assert any(re.search(pat, sample_bad_2) for pat in FORBIDDEN_PATTERNS)

    sample_bad_3 = "let c = chunk_size = std::cmp::min(len, 10);"
    assert any(re.search(pat, sample_bad_3) for pat in FORBIDDEN_PATTERNS)

    sample_bad_4 = "let val = x << 65;"
    assert any(re.search(pat, sample_bad_4) for pat in FORBIDDEN_PATTERNS)

    sample_bad_5 = "let ratio = 2.058;"
    assert any(re.search(pat, sample_bad_5) for pat in FORBIDDEN_PATTERNS)

    # Test compliant code
    sample_clean = "pub const TARGET_ABUNDANCE_INDEX: f64 = 2.0; let x = 1 << 64;"
    assert not any(re.search(pat, sample_clean) for pat in FORBIDDEN_PATTERNS)


def test_main_clean_execution(tmp_path, monkeypatch):
    # Setup mock workspace
    repo_root = tmp_path
    scripts_dir = repo_root / "scripts"
    scripts_dir.mkdir(parents=True, exist_ok=True)
    rust_dir = repo_root / "rust-engine" / "src"
    rust_dir.mkdir(parents=True, exist_ok=True)
    lean_dir = repo_root / "lean4-proofs" / "UALBF"
    lean_dir.mkdir(parents=True, exist_ok=True)

    # Create clean rust file
    (rust_dir / "main.rs").write_text(
        'fn main() { println!("Hello"); }', encoding="utf-8"
    )

    # Create clean lean file
    (lean_dir / "Basic.lean").write_text("def x : Nat := 0", encoding="utf-8")

    # Create bounds_manifest.json with ignored bounds
    bounds_manifest = {
        "search_bounds": {
            "target_min_log10": {"value": 35},
        }
    }
    (repo_root / "bounds_manifest.json").write_text(
        json.dumps(bounds_manifest), encoding="utf-8"
    )

    monkeypatch.setattr(
        check_literals, "__file__", str(scripts_dir / "check_literals.py")
    )
    monkeypatch.setattr(check_literals, "FILES_TO_CHECK", ["src/main.rs"])

    # Run check_literals.main()
    with pytest.raises(SystemExit) as exc_info:
        check_literals.main()

    assert exc_info.value.code == 0


def test_main_forbidden_rust_pattern_fails(tmp_path, monkeypatch):
    repo_root = tmp_path
    scripts_dir = repo_root / "scripts"
    scripts_dir.mkdir(parents=True, exist_ok=True)
    rust_dir = repo_root / "rust-engine" / "src"
    rust_dir.mkdir(parents=True, exist_ok=True)
    lean_dir = repo_root / "lean4-proofs" / "UALBF"
    lean_dir.mkdir(parents=True, exist_ok=True)

    # Create bad rust file matching forbidden pattern
    (rust_dir / "bad.rs").write_text(
        "pub const TARGET_ABUNDANCE : f64 = 2.0;", encoding="utf-8"
    )
    (lean_dir / "Basic.lean").write_text("def x : Nat := 0", encoding="utf-8")

    bounds_manifest = {"search_bounds": {"target_min_log10": {"value": 35}}}
    (repo_root / "bounds_manifest.json").write_text(
        json.dumps(bounds_manifest), encoding="utf-8"
    )

    monkeypatch.setattr(
        check_literals, "__file__", str(scripts_dir / "check_literals.py")
    )
    monkeypatch.setattr(check_literals, "FILES_TO_CHECK", ["src/bad.rs"])

    captured_err = io.StringIO()
    monkeypatch.setattr(sys, "stderr", captured_err)

    with pytest.raises(SystemExit) as exc_info:
        check_literals.main()

    assert exc_info.value.code == 1
    assert (
        "ERROR: Found forbidden hardcoded literal matching" in captured_err.getvalue()
    )


def test_main_lean_literal_fail(tmp_path, monkeypatch):
    repo_root = tmp_path
    scripts_dir = repo_root / "scripts"
    scripts_dir.mkdir(parents=True, exist_ok=True)
    rust_dir = repo_root / "rust-engine" / "src"
    rust_dir.mkdir(parents=True, exist_ok=True)
    lean_dir = repo_root / "lean4-proofs" / "UALBF"
    lean_dir.mkdir(parents=True, exist_ok=True)

    (rust_dir / "clean.rs").write_text("fn foo() {}", encoding="utf-8")

    # Create lean file containing non-ignored manifest bound literal (e.g., 9999)
    (lean_dir / "Proof.lean").write_text(
        "def proof_bound : Nat := 9999", encoding="utf-8"
    )

    bounds_manifest = {"custom_threshold": 9999}
    (repo_root / "bounds_manifest.json").write_text(
        json.dumps(bounds_manifest), encoding="utf-8"
    )

    monkeypatch.setattr(
        check_literals, "__file__", str(scripts_dir / "check_literals.py")
    )
    monkeypatch.setattr(check_literals, "FILES_TO_CHECK", ["src/clean.rs"])

    captured_err = io.StringIO()
    monkeypatch.setattr(sys, "stderr", captured_err)

    with pytest.raises(SystemExit) as exc_info:
        check_literals.main()

    assert exc_info.value.code == 1
    assert (
        "ERROR: Found forbidden literal '9999' from manifest" in captured_err.getvalue()
    )
