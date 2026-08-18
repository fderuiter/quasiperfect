"""
Tests for compiler-driven statement hash & symbol indexing.

Verifies:
- Proof manifest includes normalized statement signature hashes for all core theorems.
- Proof manifest includes explicit symbol_map dictionary.
- Certificate verification fails if a statement signature hash does not match.
- Certificate verification fails if a theorem is unindexed or missing from symbol_map.
- Manuscript ingestion enforces explicit symbol index lookup.
"""

import hashlib
import json
import os
import sys
import subprocess
import pytest

import cert_util
from cert_util import (
    CORE_THEOREMS,
    SYMBOL_MAP,
    compute_statement_hash,
    extract_statement_from_file,
    normalize_statement,
)
from verify_cert import verify_certificate


@pytest.fixture(autouse=True)
def mock_verification_lib(monkeypatch):
    """Mock verification_lib for tests that call load_and_validate_cert or verify_certificate."""
    monkeypatch.setattr(cert_util, "_has_verification_lib", True)
    monkeypatch.setattr(
        cert_util,
        "check_path_continuity",
        lambda x: '{"is_continuous": true, "gaps": []}',
    )
    monkeypatch.setattr(
        cert_util,
        "load_and_validate_cert",
        lambda path: json.load(open(path, "r", encoding="utf-8")),
    )


def _build_minimal_cert(manifest_hash: str) -> dict:
    return {
        "manifest_hash": manifest_hash,
        "verified_logic_hash": "a" * 64,
        "telemetry": {
            "target_min_log10": 35,
            "target_max_log10": 37,
            "sieve_limit": 250000,
            "max_exponent": 4,
            "prefix_stop": 100000000000,
            "total_branches_searched": 42,
            "abundance_pruned": 5000,
            "search_space_density": 0.0042,
            "phase2_execution_time_ms": 12345,
            "total_execution_time_ms": 13000,
            "raycast_pruned": 100,
            "math_interruptions": 0,
            "path_ranges": [{"start_bound": [], "end_bound": []}],
        },
        "signature": "deadbeef",
        "public_key": "cafebabe",
    }


def test_manifest_contains_statement_hashes_and_symbol_map(tmp_path):
    """Verify auditor generates proof_manifest.json with statement_hash and symbol_map."""
    env = os.environ.copy()
    env["MOCK_LEAN"] = "1"
    auditor_path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "auditor.py"
    )

    subprocess.run(
        [sys.executable, auditor_path], cwd=str(tmp_path), env=env, check=True
    )

    manifest_file = tmp_path / "proof_manifest.json"
    assert manifest_file.exists()

    with open(manifest_file, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    assert "symbol_map" in manifest
    assert len(manifest["symbol_map"]) >= len(CORE_THEOREMS)

    for thm_name in CORE_THEOREMS:
        assert thm_name in manifest["symbol_map"]

    for thm in manifest["theorems"]:
        assert "statement_hash" in thm
        assert "statement_signature_hash" in thm
        assert len(thm["statement_hash"]) == 64
        assert thm["statement_hash"] == thm["statement_signature_hash"]


def test_verification_fails_on_statement_hash_mismatch(tmp_path):
    """Verify certificate verification fails when statement_hash is tampered with."""
    manifest = {
        "symbol_map": SYMBOL_MAP,
        "theorems": [
            {
                "name": "UALBF.Engine.CyclotomicGraph.forced_inclusion",
                "file": "UALBF/Engine/CyclotomicGraph.lean",
                "status": "proven",
                "checksum": "",
                "statement_hash": "0" * 64,  # Tampered hash
                "statement_signature_hash": "0" * 64,
            }
        ],
        "proof_files": [],
    }

    bounds_path = tmp_path / "bounds_manifest.json"
    bounds_content = b'{"dummy": "bounds"}'
    bounds_path.write_bytes(bounds_content)
    manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()

    manifest_content = json.dumps(manifest)
    manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()

    cert = _build_minimal_cert(manifest_hash)

    cert_path = tmp_path / "formal_certificate.json"
    manifest_path = tmp_path / "proof_manifest.json"
    cert_path.write_text(json.dumps(cert), encoding="utf-8")
    manifest_path.write_text(manifest_content, encoding="utf-8")

    with pytest.raises(SystemExit) as exc_info:
        verify_certificate(str(cert_path), str(manifest_path))

    assert exc_info.value.code != 0


def test_verification_fails_on_unindexed_symbol(tmp_path):
    """Verify certificate verification fails when a theorem is missing from symbol_map."""
    manifest = {
        "symbol_map": {},  # Empty symbol map -> unindexed theorem
        "theorems": [
            {
                "name": "UALBF.Engine.CyclotomicGraph.forced_inclusion",
                "file": "UALBF/Engine/CyclotomicGraph.lean",
                "status": "proven",
                "checksum": "",
                "statement_hash": "a" * 64,
                "statement_signature_hash": "a" * 64,
            }
        ],
        "proof_files": [],
    }

    bounds_path = tmp_path / "bounds_manifest.json"
    bounds_content = b'{"dummy": "bounds"}'
    bounds_path.write_bytes(bounds_content)
    manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()

    manifest_content = json.dumps(manifest)
    manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()

    cert = _build_minimal_cert(manifest_hash)

    cert_path = tmp_path / "formal_certificate.json"
    manifest_path = tmp_path / "proof_manifest.json"
    cert_path.write_text(json.dumps(cert), encoding="utf-8")
    manifest_path.write_text(manifest_content, encoding="utf-8")

    with pytest.raises(SystemExit) as exc_info:
        verify_certificate(str(cert_path), str(manifest_path))

    assert exc_info.value.code != 0


def test_statement_normalization_and_extraction():
    """Verify statement normalization and extraction logic."""
    raw = """
    /--
      Local helper comment
    --/
    theorem forced_inclusion {p e N : ℕ}
      (hp_prime : p.Prime) :
      -- inline comment
      ∀ d, d ∣ (2 * e + 1)
    """

    norm = normalize_statement(raw)
    assert "/--" not in norm
    assert "-- inline comment" not in norm
    assert "\n" not in norm
    assert "theorem forced_inclusion" in norm

    h1 = compute_statement_hash(raw)
    h2 = compute_statement_hash(norm)
    assert h1 == h2
    assert len(h1) == 64
