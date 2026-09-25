"""
Unit and integration tests for cert_util.py.

Covers:
- Standalone unit tests for format_duration, validate_sidecar_schema, size limits, and BoundedJSONLoader.
- Live native C/Rust extension integration tests for load_and_validate_cert and verification_lib.
- Parametrized tests comparing fallback Python parsing routines with expected and native extension outputs.
"""

import hashlib
import json
import os
import pytest  # type: ignore

pytest.importorskip("cryptography")

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey  # type: ignore
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat  # type: ignore

import cert_util
from cert_util import (
    format_duration,
    validate_sidecar_schema,
    get_max_cert_size_bytes,
    validate_file_size,
    load_and_validate_cert,
    BoundedJSONLoader,
    CertificateValidationError,
    clean_source_py,
    count_non_literal_braces_py,
    compute_verus_hashes_fallback,
    compute_verus_hashes,
)

try:
    import verification_lib  # type: ignore

    HAS_VERIFICATION_LIB = True
except ImportError:
    HAS_VERIFICATION_LIB = False


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def sign_payload(payload_str: str) -> tuple[str, str]:
    """Generate an Ed25519 keypair and sign the given payload string, returning (pub_hex, sig_hex)."""
    private_key = Ed25519PrivateKey.generate()
    sig = private_key.sign(payload_str.encode("utf-8"))
    pub = private_key.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    return pub.hex(), sig.hex()


def make_valid_manifest_and_cert(
    tmp_path,
    total_branches: int = 42,
    target_min_log10: int = 35,
    target_max_log10: int = 37,
) -> tuple[dict, str, str, str]:
    """Construct a mock proof_manifest.json file and a matching signed certificate dict."""
    manifest = {"theorems": [], "proof_files": []}
    manifest_content = json.dumps(manifest)
    manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()

    manifest_path = os.path.join(str(tmp_path), "proof_manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        f.write(manifest_content)

    verified_logic_hash = "aabbccdd" * 8
    map_obj = {
        "manifest_hash": manifest_hash,
        "verified_logic_hash": verified_logic_hash,
        "total_branches_searched": total_branches,
        "target_min_log10": target_min_log10,
        "target_max_log10": target_max_log10,
        "trace_hash": "",
        "factorization_depth": 0,
        "path_ranges": [{"start_bound": [], "end_bound": []}],
    }
    payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
    pub_hex, sig_hex = sign_payload(payload)

    cert = {
        "manifest_hash": manifest_hash,
        "verified_logic_hash": verified_logic_hash,
        "telemetry": {
            "target_min_log10": target_min_log10,
            "target_max_log10": target_max_log10,
            "sieve_limit": 250000,
            "max_exponent": 4,
            "prefix_stop": 100000000000,
            "total_branches_searched": total_branches,
            "abundance_pruned": 0,
            "search_space_density": 0.0,
            "phase2_execution_time_ms": 0,
            "path_ranges": [{"start_bound": [], "end_bound": []}],
        },
        "signature": sig_hex,
        "public_key": pub_hex,
    }
    return cert, pub_hex, sig_hex, manifest_path


# ---------------------------------------------------------------------------
# 1. Standalone Unit Tests: format_duration
# ---------------------------------------------------------------------------


class TestFormatDuration:
    def test_negative_seconds_returns_dash(self):
        assert format_duration(-1.0) == "—"
        assert format_duration(-100.0) == "—"

    def test_short_style_seconds_only(self):
        assert format_duration(0.0, style="short") == "0s"
        assert format_duration(45.2, style="short") == "45s"

    def test_short_style_minutes(self):
        # 150 seconds = 2 min 30 sec = 2.5m
        assert format_duration(150.0, style="short") == "2.5m"

    def test_short_style_hours(self):
        # 3600 seconds = 1 hour = 1.0h
        assert format_duration(3600.0, style="short") == "1.0h"
        # 5400 seconds = 1.5h
        assert format_duration(5400.0, style="short") == "1.5h"

    def test_full_style(self):
        # 3661 seconds = 1 hour, 1 minute, 1 second
        assert format_duration(3661.0, style="full") == "1 hours, 1 minutes, 1 seconds"
        assert format_duration(0.0, style="full") == "0 hours, 0 minutes, 0 seconds"

    def test_unknown_style_returns_str_seconds(self):
        assert format_duration(123.45, style="invalid") == "123.45"


# ---------------------------------------------------------------------------
# 2. Standalone Unit Tests: validate_sidecar_schema
# ---------------------------------------------------------------------------


class TestValidateSidecarSchema:
    def test_valid_sidecar_schema_passes(self, tmp_path):
        sidecar = tmp_path / "overflow.log"
        rec1 = json.dumps({"event": "overflow", "p": "3", "pow": 2})
        rec2 = json.dumps({"event": "overflow", "p": "101", "pow": 0})
        sidecar.write_text(f"\n{rec1}\n\n  {rec2}  \n", encoding="utf-8")
        validate_sidecar_schema(str(sidecar))

    def test_nonexistent_file_raises_validation_error(self, tmp_path):
        nonexistent = tmp_path / "does_not_exist.log"
        with pytest.raises(CertificateValidationError, match="not found"):
            validate_sidecar_schema(str(nonexistent))

    def test_malformed_json_raises_validation_error(self, tmp_path):
        bad_json = tmp_path / "bad.log"
        bad_json.write_text('{"event": "overflow", p: 3}\n', encoding="utf-8")
        with pytest.raises(
            CertificateValidationError, match="Line 1: invalid JSON syntax"
        ):
            validate_sidecar_schema(str(bad_json))

    def test_non_object_record_raises_validation_error(self, tmp_path):
        bad_rec = tmp_path / "array.log"
        bad_rec.write_text('["overflow", "3", 2]\n', encoding="utf-8")
        with pytest.raises(
            CertificateValidationError, match="record is not a JSON object"
        ):
            validate_sidecar_schema(str(bad_rec))

    def test_missing_required_keys_raises_validation_error(self, tmp_path):
        missing = tmp_path / "missing_key.log"
        missing.write_text(
            json.dumps({"event": "overflow", "p": "3"}) + "\n", encoding="utf-8"
        )
        with pytest.raises(CertificateValidationError, match="Missing required key"):
            validate_sidecar_schema(str(missing))

    def test_extra_keys_raises_validation_error(self, tmp_path):
        extra = tmp_path / "extra_key.log"
        extra.write_text(
            json.dumps({"event": "overflow", "p": "3", "pow": 2, "unexpected": 1})
            + "\n",
            encoding="utf-8",
        )
        with pytest.raises(CertificateValidationError, match="Unexpected extra key"):
            validate_sidecar_schema(str(extra))

    def test_invalid_event_value_raises_validation_error(self, tmp_path):
        invalid_evt = tmp_path / "bad_event.log"
        invalid_evt.write_text(
            json.dumps({"event": "progress", "p": "3", "pow": 2}) + "\n",
            encoding="utf-8",
        )
        with pytest.raises(CertificateValidationError, match="invalid 'event' value"):
            validate_sidecar_schema(str(invalid_evt))

    def test_invalid_p_field_raises_validation_error(self, tmp_path):
        for bad_p in [3, "abc", "3.14", "-5"]:
            f = tmp_path / "bad_p.log"
            f.write_text(
                json.dumps({"event": "overflow", "p": bad_p, "pow": 2}) + "\n",
                encoding="utf-8",
            )
            with pytest.raises(
                CertificateValidationError, match="invalid 'p' field value"
            ):
                validate_sidecar_schema(str(f))

    def test_invalid_pow_field_raises_validation_error(self, tmp_path):
        for bad_pow in ["2", -1, True, 2.5]:
            f = tmp_path / "bad_pow.log"
            f.write_text(
                json.dumps({"event": "overflow", "p": "3", "pow": bad_pow}) + "\n",
                encoding="utf-8",
            )
            with pytest.raises(
                CertificateValidationError, match="invalid 'pow' field value"
            ):
                validate_sidecar_schema(str(f))


# ---------------------------------------------------------------------------
# 3. Standalone Unit Tests: File Size Limits
# ---------------------------------------------------------------------------


class TestFileSizeLimits:
    def test_get_max_cert_size_bytes_default(self, monkeypatch):
        monkeypatch.delenv("UALBF_MAX_CERT_SIZE_MB", raising=False)
        assert get_max_cert_size_bytes() == 10 * 1024 * 1024

    def test_get_max_cert_size_bytes_env_override(self, monkeypatch):
        monkeypatch.setenv("UALBF_MAX_CERT_SIZE_MB", "2.5")
        assert get_max_cert_size_bytes() == int(2.5 * 1024 * 1024)

    def test_get_max_cert_size_bytes_env_invalid_fallback(self, monkeypatch):
        monkeypatch.setenv("UALBF_MAX_CERT_SIZE_MB", "invalid_number")
        assert get_max_cert_size_bytes() == 10 * 1024 * 1024

    def test_validate_file_size_under_limit_passes(self, tmp_path):
        f = tmp_path / "small.json"
        f.write_bytes(b"x" * 1024)
        validate_file_size(str(f), max_bytes=2048)

    def test_validate_file_size_exceeding_limit_raises(self, tmp_path):
        f = tmp_path / "large.json"
        f.write_bytes(b"x" * 2049)
        with pytest.raises(
            CertificateValidationError, match="exceeds maximum allowed limit"
        ):
            validate_file_size(str(f), max_bytes=2048)

    def test_validate_file_size_nonexistent_file_ignored(self, tmp_path):
        # validate_file_size only checks if file exists; if not found, it doesn't raise size error
        nonexistent = tmp_path / "missing.json"
        validate_file_size(str(nonexistent))


# ---------------------------------------------------------------------------
# 4. Standalone Unit Tests: BoundedJSONLoader
# ---------------------------------------------------------------------------


class TestBoundedJSONLoaderUnit:
    def test_default_limits(self):
        loader = BoundedJSONLoader()
        assert loader.max_size_bytes == 10 * 1024 * 1024
        assert loader.max_depth == 10

    def test_valid_json_within_limits(self):
        loader = BoundedJSONLoader()
        data = loader.loads('{"key": "value", "numbers": [1, 2, 3]}')
        assert data["key"] == "value"
        assert data["numbers"] == [1, 2, 3]

    def test_bytes_input(self):
        loader = BoundedJSONLoader()
        data = loader.loads(b'{"key": "value"}')
        assert data["key"] == "value"

    def test_invalid_input_type_raises(self):
        loader = BoundedJSONLoader()
        with pytest.raises(CertificateValidationError, match="Invalid JSON input type"):
            loader.loads(12345)  # type: ignore

    def test_payload_exceeding_max_size_raises(self):
        loader = BoundedJSONLoader(max_size_bytes=50)
        large_json = json.dumps({"data": "x" * 100})
        with pytest.raises(
            CertificateValidationError, match="exceeds maximum allowed limit"
        ):
            loader.loads(large_json)

    def test_depth_within_limit_passes(self):
        loader = BoundedJSONLoader(max_depth=5)
        obj = 1
        for _ in range(5):
            obj = {"a": obj}
        data = loader.loads(json.dumps(obj))
        assert data is not None

    def test_depth_exceeding_limit_raises(self):
        loader = BoundedJSONLoader(max_depth=5)
        obj = 1
        for _ in range(6):
            obj = {"a": obj}
        with pytest.raises(CertificateValidationError, match="nesting depth"):
            loader.loads(json.dumps(obj))

    def test_depth_verification_list_and_dict(self):
        loader = BoundedJSONLoader(max_depth=3)
        # depth 3: dict -> list -> dict -> int
        data = {"a": [{"b": 1}]}
        assert loader.loads(json.dumps(data)) == data

    def test_read_file_text_and_bytes(self, tmp_path):
        fpath = tmp_path / "test.txt"
        fpath.write_text("Hello World", encoding="utf-8")
        loader = BoundedJSONLoader()
        assert loader.read_file_text(str(fpath)) == "Hello World"
        assert loader.read_file_bytes(str(fpath)) == b"Hello World"

    def test_read_file_stream_object(self, tmp_path):
        fpath = tmp_path / "test.json"
        fpath.write_text('{"foo": "bar"}')
        loader = BoundedJSONLoader()
        with open(fpath, "r", encoding="utf-8") as fp:
            data = loader.load(fp)
            assert data["foo"] == "bar"

    def test_file_not_found_raises(self):
        loader = BoundedJSONLoader()
        with pytest.raises(CertificateValidationError, match="File not found"):
            loader.read_file_text("/nonexistent/file.json")

    def test_invalid_json_syntax_raises(self):
        loader = BoundedJSONLoader()
        with pytest.raises(CertificateValidationError, match="Invalid JSON payload"):
            loader.loads("{invalid json")


# ---------------------------------------------------------------------------
# 5. Live Native Extension & Integration Tests: load_and_validate_cert
# ---------------------------------------------------------------------------


class TestLiveCertificateValidation:
    def test_load_and_validate_cert_success(self, tmp_path, monkeypatch):
        cert, pub_hex, sig_hex, manifest_path = make_valid_manifest_and_cert(tmp_path)
        cert_file = tmp_path / "cert.json"
        cert_file.write_text(json.dumps(cert), encoding="utf-8")

        monkeypatch.setenv("UALBF_PROOF_MANIFEST", manifest_path)
        monkeypatch.setenv("UALBF_TRUSTED_PUBLIC_KEY", pub_hex)
        res = load_and_validate_cert(str(cert_file))
        assert isinstance(res, dict)
        assert res["manifest_hash"] == cert["manifest_hash"]

    def test_load_and_validate_cert_missing_trusted_key_raises(
        self, tmp_path, monkeypatch
    ):
        cert, pub_hex, sig_hex, manifest_path = make_valid_manifest_and_cert(tmp_path)
        cert_file = tmp_path / "cert.json"
        cert_file.write_text(json.dumps(cert), encoding="utf-8")

        monkeypatch.setenv("UALBF_PROOF_MANIFEST", manifest_path)
        monkeypatch.delenv("UALBF_TRUSTED_PUBLIC_KEY", raising=False)
        with pytest.raises(
            CertificateValidationError, match="No trusted public key is pinned"
        ):
            load_and_validate_cert(str(cert_file), trusted_public_key="")

    def test_load_and_validate_cert_mismatched_trusted_key_raises(
        self, tmp_path, monkeypatch
    ):
        cert, pub_hex, sig_hex, manifest_path = make_valid_manifest_and_cert(tmp_path)
        cert_file = tmp_path / "cert.json"
        cert_file.write_text(json.dumps(cert), encoding="utf-8")

        wrong_key = "00" * 32
        monkeypatch.setenv("UALBF_PROOF_MANIFEST", manifest_path)
        monkeypatch.setenv("UALBF_TRUSTED_PUBLIC_KEY", wrong_key)
        with pytest.raises(CertificateValidationError, match="Validation failed"):
            load_and_validate_cert(str(cert_file))

    def test_load_and_validate_cert_missing_file_raises(self, tmp_path):
        nonexistent = tmp_path / "missing_cert.json"
        with pytest.raises(
            CertificateValidationError, match="Certificate file not found"
        ):
            load_and_validate_cert(str(nonexistent))

    def test_load_and_validate_cert_tampered_signature_raises(
        self, tmp_path, monkeypatch
    ):
        cert, pub_hex, sig_hex, manifest_path = make_valid_manifest_and_cert(tmp_path)
        # Tamper with signature
        cert["signature"] = "0" * 128
        cert_file = tmp_path / "cert.json"
        cert_file.write_text(json.dumps(cert), encoding="utf-8")

        monkeypatch.setenv("UALBF_PROOF_MANIFEST", manifest_path)
        monkeypatch.setenv("UALBF_TRUSTED_PUBLIC_KEY", pub_hex)
        with pytest.raises(CertificateValidationError, match="Validation failed"):
            load_and_validate_cert(str(cert_file))

    def test_load_and_validate_cert_bypass_env_vars_rejected(
        self, tmp_path, monkeypatch
    ):
        cert, pub_hex, sig_hex, manifest_path = make_valid_manifest_and_cert(tmp_path)
        cert_file = tmp_path / "cert.json"
        cert_file.write_text(json.dumps(cert), encoding="utf-8")

        monkeypatch.setenv("UALBF_PROOF_MANIFEST", manifest_path)
        monkeypatch.setenv("UALBF_TRUSTED_PUBLIC_KEY", pub_hex)
        monkeypatch.setenv("UALBF_SKIP_VALIDATION", "1")
        with pytest.raises(SystemExit) as exc_info:
            load_and_validate_cert(str(cert_file))
        assert exc_info.value.code == 1

    def test_load_and_validate_cert_when_verification_lib_missing_raises_import_error(
        self, tmp_path, monkeypatch
    ):
        cert, pub_hex, sig_hex, manifest_path = make_valid_manifest_and_cert(tmp_path)
        cert_file = tmp_path / "cert.json"
        cert_file.write_text(json.dumps(cert), encoding="utf-8")

        monkeypatch.setenv("UALBF_PROOF_MANIFEST", manifest_path)
        monkeypatch.setattr(cert_util, "_has_verification_lib", False)
        with pytest.raises(ImportError, match="Native verification_lib not found"):
            load_and_validate_cert(str(cert_file), trusted_public_key=pub_hex)

    @pytest.mark.skipif(
        not HAS_VERIFICATION_LIB,
        reason="verification_lib native extension not available",
    )
    def test_native_validate_certificate_direct(self, tmp_path, monkeypatch):
        cert, pub_hex, sig_hex, manifest_path = make_valid_manifest_and_cert(tmp_path)
        monkeypatch.setenv("UALBF_PROOF_MANIFEST", manifest_path)
        cert_str = json.dumps(cert)
        res = verification_lib.validate_certificate(cert_str, pub_hex)
        assert isinstance(res, dict)
        assert res["manifest_hash"] == cert["manifest_hash"]

    @pytest.mark.skipif(
        not HAS_VERIFICATION_LIB,
        reason="verification_lib native extension not available",
    )
    def test_native_validate_certificate_null_byte_rejection(self):
        raw_json_with_null = '{"manifest_hash": "abc", "\0": "tampered"}'
        with pytest.raises(ValueError, match="Null byte detected"):
            verification_lib.validate_certificate(raw_json_with_null, "00" * 32)


# ---------------------------------------------------------------------------
# 6. Parametrized Fallback vs Native Routine Tests
# ---------------------------------------------------------------------------


class TestFallbackParsingRoutines:
    @pytest.mark.parametrize(
        "source, expected",
        [
            ("// comment\nlet x = 1;", "\nlet x = 1;"),
            ("/* block comment */ let y = 2;", " let y = 2;"),
            ('let s = "// not a comment";', 'let s = "// not a comment";'),
            ('let s = "hello \\" world";', 'let s = "hello \\" world";'),
            ("let c = '\\''; // comment", "let c = '\\''; "),
            ("let c = '\\\\'; // comment", "let c = '\\\\'; "),
            ("/* line1 \n line2 */ let z = 3;", "\n let z = 3;"),
            ("/* nested /* block */ comment */ fn foo() {}", " fn foo() {}"),
            ("let c = 'a'; // comment", "let c = 'a'; "),
        ],
    )
    def test_clean_source_py(self, source, expected):
        cleaned = clean_source_py(source)
        assert cleaned == expected

    @pytest.mark.parametrize(
        "line, expected_open, expected_close",
        [
            ("fn foo() { bar(); }", 1, 1),
            ('let s = "{ not a brace }";', 0, 0),
            ('let s = "a{\\"b";', 0, 0),
            ("let c = '\\''; { }", 1, 1),
            ("let c = '\\\\'; { }", 1, 1),
            ("let c = '{'; }", 0, 1),
            ("{ { { } }", 3, 2),
            (
                "// { comment }",
                1,
                1,
            ),  # count_non_literal_braces_py processes line as passed
        ],
    )
    def test_count_non_literal_braces_py(self, line, expected_open, expected_close):
        open_b, close_b = count_non_literal_braces_py(line)
        assert open_b == expected_open
        assert close_b == expected_close

    @pytest.mark.parametrize(
        "content, expected_fn_names",
        [
            (
                "pub fn simple_fn(x: u32) {\n    x + 1;\n}\n",
                ["simple_fn"],
            ),
            (
                "pub spec fn spec_calc() -> bool {\n    true\n}\n"
                "pub proof fn proof_lemma() {\n    assert(true);\n}\n",
                ["spec_calc", "proof_lemma"],
            ),
            (
                "pub mod inner {\n"
                "    pub open spec fn open_spec() -> u32 {\n        42\n    }\n"
                "}\n",
                ["inner::open_spec"],
            ),
        ],
    )
    def test_compute_verus_hashes_fallback(self, content, expected_fn_names):
        hashes = compute_verus_hashes_fallback(content)
        assert set(hashes.keys()) == set(expected_fn_names)
        for fn_name in expected_fn_names:
            assert isinstance(hashes[fn_name], str)
            assert len(hashes[fn_name]) == 64  # sha256 hex string

    @pytest.mark.skipif(
        not HAS_VERIFICATION_LIB,
        reason="verification_lib native extension not available",
    )
    @pytest.mark.parametrize(
        "verus_code",
        [
            "pub fn standalone_func() {\n    let x = 1;\n}\n",
            "pub spec fn helper_spec() -> bool {\n    true\n}\n",
            "pub mod math {\n    pub proof fn lemma_one() {\n        assert(true);\n    }\n}\n",
        ],
    )
    def test_fallback_matches_native_verus_hashes(self, verus_code):
        native_hashes = compute_verus_hashes(verus_code)
        fallback_hashes = compute_verus_hashes_fallback(verus_code)
        assert native_hashes == fallback_hashes


class TestVerifyManifestChainAndVerusHelpers:
    def test_verify_manifest_chain_success(self, tmp_path):
        bounds_content = json.dumps({"bounds": "ok"})
        bounds_file = tmp_path / "bounds_manifest.json"
        bounds_file.write_text(bounds_content, encoding="utf-8")
        bounds_hash = hashlib.sha256(bounds_content.encode("utf-8")).hexdigest()

        proof_manifest = {"bounds_manifest_hash": bounds_hash, "theorems": []}
        proof_content = json.dumps(proof_manifest)
        proof_file = tmp_path / "proof_manifest.json"
        proof_file.write_text(proof_content, encoding="utf-8")
        proof_hash = hashlib.sha256(proof_content.encode("utf-8")).hexdigest()

        cert = {"manifest_hash": proof_hash}

        # Should pass without raising
        cert_util.verify_manifest_chain(cert, str(proof_file), str(bounds_file))

    def test_verify_manifest_chain_missing_proof_manifest(self, tmp_path):
        missing_proof = tmp_path / "missing_proof.json"
        bounds_file = tmp_path / "bounds.json"
        bounds_file.write_text("{}", encoding="utf-8")

        with pytest.raises(CertificateValidationError, match="Proof manifest .* not found"):
            cert_util.verify_manifest_chain({}, str(missing_proof), str(bounds_file))

    def test_verify_manifest_chain_mismatched_manifest_hash(self, tmp_path):
        bounds_file = tmp_path / "bounds.json"
        bounds_file.write_text("{}", encoding="utf-8")

        proof_file = tmp_path / "proof.json"
        proof_file.write_text('{"bounds_manifest_hash": "abc"}', encoding="utf-8")

        cert = {"manifest_hash": "wrong_hash"}

        with pytest.raises(CertificateValidationError, match="Manifest hash mismatch"):
            cert_util.verify_manifest_chain(cert, str(proof_file), str(bounds_file))

    def test_verify_manifest_chain_mismatched_bounds_hash(self, tmp_path):
        bounds_file = tmp_path / "bounds.json"
        bounds_file.write_text('{"real": "data"}', encoding="utf-8")

        proof_file = tmp_path / "proof.json"
        proof_file.write_text('{"bounds_manifest_hash": "wrong_bounds_hash"}', encoding="utf-8")

        cert_hash = hashlib.sha256(proof_file.read_bytes()).hexdigest()
        cert = {"manifest_hash": cert_hash}

        with pytest.raises(CertificateValidationError, match="Bounds manifest hash mismatch"):
            cert_util.verify_manifest_chain(cert, str(proof_file), str(bounds_file))

    def test_get_verus_proof_hashes(self, tmp_path):
        rust_dir = tmp_path / "rust-src"
        rust_dir.mkdir()

        verus_proofs = rust_dir / "verus_proofs.rs"
        verus_proofs.write_text("pub fn test_proof() {\n    let a = 1;\n}\n", encoding="utf-8")

        lean_export = rust_dir / "lean_export.rs"
        lean_export.write_text("pub spec fn test_export() -> bool {\n    true\n}\n", encoding="utf-8")

        hashes = cert_util.get_verus_proof_hashes(str(rust_dir))
        assert "test_proof" in hashes
        assert "test_export" in hashes

    def test_verify_theorem_checksum_metadata_fallback(self, tmp_path):
        thm = {
            "name": "UALBF.Test.theorem_1",
            "file": "NonExistent.lean",
            "status": "proven",
        }
        payload = f"{thm['name']}|{thm['file']}|{thm['status']}"
        expected = hashlib.sha256(payload.encode("utf-8")).hexdigest()
        thm["checksum"] = expected

        assert cert_util.verify_theorem_checksum(thm) is True

