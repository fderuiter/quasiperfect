"""
Tests for verify_cert.py

Covers the PR changes:
- Simplified payload format: "{manifest_hash}_{verified_logic_hash}_{total_branches}_{target_max_log10}"
- Removed environment/lockfile hash verification
- Removed docstrings
"""

import hashlib
import json
import os
import sys
import tempfile
import subprocess
import pytest  # type: ignore

# Import cryptography for creating test keypairs
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey  # type: ignore
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat  # type: ignore

from verify_cert import verify_certificate, check_continuity
from cert_util import load_and_validate_cert, CertificateValidationError

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_manifest(theorems=None):
    """Return a minimal proof manifest dict."""
    if theorems is None:
        name = "UALBF.Pure.Arithmetic.foo"
        file = "UALBF/Pure/Arithmetic.lean"
        status = "proven"
        payload = f"{name}|{file}|{status}"
        checksum = hashlib.sha256(payload.encode("utf-8")).hexdigest()
        theorems = [
            {"name": name, "file": file, "status": status, "checksum": checksum},
        ]
    else:
        for t in theorems:
            if t.get("checksum") in ["x", "y", "allowed"]:
                payload = f"{t['name']}|{t['file']}|{t['status']}"
                t["checksum"] = hashlib.sha256(payload.encode("utf-8")).hexdigest()

    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    arith_file = "UALBF/Pure/Arithmetic.lean"
    arith_path = os.path.join(base_dir, "lean4-proofs", arith_file)
    try:
        with open(arith_path, "rb") as f:
            chk = hashlib.sha256(f.read()).hexdigest()
    except Exception:
        chk = "aabbccdd"

    return {
        "theorems": theorems,
        "proof_files": [{"file": arith_file, "checksum": chk}],
    }


def sign_payload(payload_str: str) -> tuple[str, str]:
    """Return (public_key_hex, signature_hex) for the given payload string."""
    private_key = Ed25519PrivateKey.generate()
    sig = private_key.sign(payload_str.encode("utf-8"))
    pub = private_key.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    return pub.hex(), sig.hex()


def write_mock_manifest_files(tmpdir, manifest):
    """Write mock theorem and proof_files to tmpdir and update their checksums to content hashes."""
    # Ensure they exist and have matching content-level hashes
    for thm in manifest.get("theorems", []):
        file_path = os.path.join(tmpdir, thm["file"])
        os.makedirs(os.path.dirname(file_path), exist_ok=True)

        real_path = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "lean4-proofs",
            thm["file"],
        )
        if os.path.exists(real_path):
            with open(real_path, "rb") as rf:
                content = rf.read()
        else:
            content = b"mock content for " + thm["file"].encode("utf-8")

        with open(file_path, "wb") as f:
            f.write(content)

        thm["checksum"] = hashlib.sha256(content).hexdigest()

    for pf in manifest.get("proof_files", []):
        file_path = os.path.join(tmpdir, pf["file"])
        os.makedirs(os.path.dirname(file_path), exist_ok=True)

        real_path = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "lean4-proofs",
            pf["file"],
        )
        if os.path.exists(real_path):
            with open(real_path, "rb") as rf:
                content = rf.read()
        else:
            content = b"mock content for " + pf["file"].encode("utf-8")

        with open(file_path, "wb") as f:
            f.write(content)

        pf["checksum"] = hashlib.sha256(content).hexdigest()


def build_cert(
    manifest_hash: str,
    verified_logic_hash: str = "aabbccdd" * 8,
    total_branches: int = 42,
    target_max_log10: int = 37,
    target_min_log10: int = 35,
    tamper_sig: bool = False,
) -> dict:
    """Construct a minimal valid (or optionally tampered) certificate."""
    payload = (
        f"{manifest_hash}_{verified_logic_hash}_{total_branches}_{target_max_log10}"
    )
    pub_hex, sig_hex = sign_payload(payload)
    if tamper_sig:
        # Flip first byte of signature
        sig_bytes = bytearray(bytes.fromhex(sig_hex))
        sig_bytes[0] ^= 0xFF
        sig_hex = sig_bytes.hex()
    return {
        "manifest_hash": manifest_hash,
        "verified_logic_hash": verified_logic_hash,
        "telemetry": {
            "target_min_log10": target_min_log10,
            "target_max_log10": target_max_log10,
            "sieve_limit": 250000,
            "max_exponent": 4,
            "prefix_stop": 100000000000,
            "total_branches_searched": total_branches,
            "abundance_pruned": 5000,
            "search_space_density": 0.0042,
            "phase2_execution_time_ms": 12345,
            "total_execution_time_ms": 13000,
            "raycast_pruned": 100,
            "math_interruptions": 2,
        },
        "signature": sig_hex,
        "public_key": pub_hex,
    }


def write_files(manifest: dict, cert: dict) -> tuple[str, str]:
    """Write manifest and cert to temp files, return (cert_path, manifest_path)."""
    tmpdir = tempfile.mkdtemp()
    cert_path = os.path.join(tmpdir, "formal_certificate.json")
    manifest_path = os.path.join(tmpdir, "proof_manifest.json")
    bounds_path = os.path.join(tmpdir, "bounds_manifest.json")

    # Create dummy bounds manifest
    bounds_content = b'{"dummy": "bounds"}'
    with open(bounds_path, "wb") as f:
        f.write(bounds_content)

    write_mock_manifest_files(tmpdir, manifest)

    if "bounds_manifest_hash" not in manifest:
        manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()

    manifest_content = json.dumps(manifest)
    manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()
    cert["manifest_hash"] = manifest_hash
    # Re-sign with correct manifest hash
    tel = cert["telemetry"]
    total_branches = tel["total_branches_searched"]
    target_max_log10 = tel["target_max_log10"]
    target_min_log10 = tel.get("target_min_log10", 35)
    trace_hash = tel.get("trace_hash", "")
    factorization_depth = tel.get("factorization_depth", 0)
    verified_logic_hash = cert["verified_logic_hash"]

    map_obj = {
        "manifest_hash": manifest_hash,
        "verified_logic_hash": verified_logic_hash,
        "total_branches_searched": total_branches,
        "target_min_log10": target_min_log10,
        "target_max_log10": target_max_log10,
        "trace_hash": trace_hash,
        "factorization_depth": factorization_depth,
    }
    payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
    pub_hex, sig_hex = sign_payload(payload)
    cert["signature"] = sig_hex
    cert["public_key"] = pub_hex

    with open(cert_path, "w", encoding="utf-8") as f:
        json.dump(cert, f)
    with open(manifest_path, "w", encoding="utf-8") as f:
        f.write(manifest_content)
    return cert_path, manifest_path


# ---------------------------------------------------------------------------
# Tests: missing files
# ---------------------------------------------------------------------------


class TestMissingFiles:
    def test_missing_cert_exits_1(self, tmp_path):
        cert_path = str(tmp_path / "nonexistent_cert.json")
        manifest_path = str(tmp_path / "proof_manifest.json")
        # Create manifest but not cert
        with open(manifest_path, "w", encoding="utf-8") as f:
            json.dump(make_manifest(), f)
        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0

    def test_missing_manifest_exits_1(self, tmp_path):
        manifest = make_manifest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()
        cert = build_cert(manifest_hash)
        cert_path = str(tmp_path / "formal_certificate.json")
        manifest_path = str(tmp_path / "nonexistent_manifest.json")
        with open(cert_path, "w", encoding="utf-8") as f:
            json.dump(cert, f)
        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0


# ---------------------------------------------------------------------------
# Tests: manifest hash verification
# ---------------------------------------------------------------------------


class TestManifestHashVerification:
    def test_correct_manifest_hash_passes(self, tmp_path, capsys):
        manifest = make_manifest()
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        # Should not raise SystemExit
        verify_certificate(cert_path, manifest_path)
        captured = capsys.readouterr()
        assert "signature is valid" in captured.out.lower()

    def test_tampered_manifest_exits(self, tmp_path):
        manifest = make_manifest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()
        cert = build_cert(manifest_hash)

        cert_path = str(tmp_path / "formal_certificate.json")
        manifest_path = str(tmp_path / "proof_manifest.json")

        # Put correct manifest hash in cert but different content in file
        with open(cert_path, "w", encoding="utf-8") as f:
            json.dump(cert, f)
        with open(manifest_path, "w", encoding="utf-8") as f:
            f.write('{"theorems": []}')  # different content

        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0


# ---------------------------------------------------------------------------
# Tests: signature verification
# ---------------------------------------------------------------------------


class TestSignatureVerification:
    def test_valid_signature_passes(self, capsys):
        manifest = make_manifest()
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        verify_certificate(cert_path, manifest_path)
        captured = capsys.readouterr()
        assert "valid" in captured.out.lower()

    def test_invalid_signature_exits(self, tmp_path):
        manifest = make_manifest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()
        cert = build_cert(manifest_hash, tamper_sig=True)

        cert_path = str(tmp_path / "formal_certificate.json")
        manifest_path = str(tmp_path / "proof_manifest.json")
        with open(cert_path, "w", encoding="utf-8") as f:
            json.dump(cert, f)
        with open(manifest_path, "w", encoding="utf-8") as f:
            f.write(manifest_content)

        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0

    def test_wrong_public_key_exits(self, tmp_path):
        """Signature from one key cannot be verified with a different key."""
        manifest = make_manifest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()
        cert = build_cert(manifest_hash)

        # Replace public key with a fresh random key
        different_key = Ed25519PrivateKey.generate()
        wrong_pub = different_key.public_key().public_bytes(
            Encoding.Raw, PublicFormat.Raw
        )
        cert["public_key"] = wrong_pub.hex()

        cert_path = str(tmp_path / "formal_certificate.json")
        manifest_path = str(tmp_path / "proof_manifest.json")
        with open(cert_path, "w", encoding="utf-8") as f:
            json.dump(cert, f)
        with open(manifest_path, "w", encoding="utf-8") as f:
            f.write(manifest_content)

        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0


# ---------------------------------------------------------------------------
# Tests: payload format (PR change — new simple string format)
# ---------------------------------------------------------------------------


class TestPayloadFormat:
    def test_payload_uses_new_format(self, tmp_path):
        """
        The PR changed the payload to canonical JSON.
        Ensure the exact format is expected by signing with the new format and verifying.
        """
        manifest = make_manifest()
        write_mock_manifest_files(str(tmp_path), manifest)
        bounds_content = b'{"dummy": "bounds"}'
        manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()

        verified_logic_hash = "deadbeef" * 8
        total_branches = 999
        target_max_log10 = 37
        target_min_log10 = 35
        trace_hash = "dummytrace"
        factorization_depth = 1000000

        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": verified_logic_hash,
            "total_branches_searched": total_branches,
            "target_min_log10": target_min_log10,
            "target_max_log10": target_max_log10,
            "trace_hash": trace_hash,
            "factorization_depth": factorization_depth,
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
                "trace_hash": trace_hash,
                "factorization_depth": factorization_depth,
            },
            "signature": sig_hex,
            "public_key": pub_hex,
        }

        cert_path = str(tmp_path / "cert.json")
        manifest_path = str(tmp_path / "manifest.json")
        bounds_path = str(tmp_path / "bounds_manifest.json")
        with open(cert_path, "w", encoding="utf-8") as f:
            json.dump(cert, f)
        with open(manifest_path, "w", encoding="utf-8") as f:
            f.write(manifest_content)
        with open(bounds_path, "wb") as f:
            f.write(bounds_content)

        # Should succeed without SystemExit
        verify_certificate(cert_path, manifest_path)

    def test_old_json_payload_format_fails(self, tmp_path):
        """
        The old payload was a JSON dict; signing with old format must fail verification
        because the verifier now uses the new string format.
        """
        manifest = make_manifest()
        bounds_content = b'{"dummy": "bounds"}'
        manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()

        verified_logic_hash = "deadbeef" * 8
        total_branches = 999
        target_max_log10 = 37

        # Sign using old JSON format
        old_payload = json.dumps(
            {
                "manifest_hash": manifest_hash,
                "telemetry": {
                    "total_branches_searched": total_branches,
                    "target_max_log10": target_max_log10,
                },
                "verified_logic_hash": verified_logic_hash,
            },
            separators=(",", ":"),
            sort_keys=True,
        )
        pub_hex, sig_hex = sign_payload(old_payload)

        cert = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": verified_logic_hash,
            "telemetry": {
                "target_min_log10": 35,
                "target_max_log10": target_max_log10,
                "sieve_limit": 250000,
                "max_exponent": 4,
                "prefix_stop": 100000000000,
                "total_branches_searched": total_branches,
                "abundance_pruned": 0,
                "search_space_density": 0.0,
                "phase2_execution_time_ms": 0,
            },
            "signature": sig_hex,
            "public_key": pub_hex,
        }

        cert_path = str(tmp_path / "cert.json")
        manifest_path = str(tmp_path / "manifest.json")
        bounds_path = str(tmp_path / "bounds_manifest.json")
        with open(cert_path, "w", encoding="utf-8") as f:
            json.dump(cert, f)
        with open(manifest_path, "w", encoding="utf-8") as f:
            f.write(manifest_content)
        with open(bounds_path, "wb") as f:
            f.write(bounds_content)

        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0


# ---------------------------------------------------------------------------
# Tests: manifest theorem checking
# ---------------------------------------------------------------------------


class TestTheoremChecking:
    def test_no_sorries_passes(self, capsys):
        manifest = make_manifest(
            [
                {
                    "name": "UALBF.Foo",
                    "file": "F.lean",
                    "status": "proven",
                    "checksum": "x",
                },
                {
                    "name": "UALBF.Bar",
                    "file": "F.lean",
                    "status": "proven",
                    "checksum": "y",
                },
            ]
        )
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        verify_certificate(cert_path, manifest_path)
        captured = capsys.readouterr()
        assert "0 sorries" in captured.out

    def test_sorry_theorem_exits(self, tmp_path):
        manifest = make_manifest(
            [
                {
                    "name": "UALBF.Foo",
                    "file": "F.lean",
                    "status": "proven",
                    "checksum": "x",
                },
                {
                    "name": "UALBF.BrokenTheorem",
                    "file": "B.lean",
                    "status": "sorry",
                    "checksum": "z",
                },
            ]
        )
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0

    def test_axiom_theorem_exits(self, tmp_path):
        manifest = make_manifest(
            [
                {
                    "name": "UALBF.SomeAxiom",
                    "file": "A.lean",
                    "status": "axiom",
                    "checksum": "a",
                },
            ]
        )
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0

    def test_allowed_axiom_rust_is_prime_sound_passes(self, capsys):
        """UALBF.FFI.rust_is_prime_sound is no longer whitelisted, so this should fail."""
        manifest = make_manifest(
            [
                {
                    "name": "UALBF.FFI.rust_is_prime_sound",
                    "file": "FFI.lean",
                    "status": "axiom",
                    "checksum": "allowed",
                },
            ]
        )
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        # Should exit due to zero-axiom policy

        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0

    def test_multiple_sorries_all_reported(self, tmp_path, capsys):
        manifest = make_manifest(
            [
                {
                    "name": "UALBF.Foo",
                    "file": "F.lean",
                    "status": "sorry",
                    "checksum": "a",
                },
                {
                    "name": "UALBF.Bar",
                    "file": "B.lean",
                    "status": "sorry",
                    "checksum": "b",
                },
            ]
        )
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        with pytest.raises(SystemExit):
            verify_certificate(cert_path, manifest_path)

    def test_empty_theorems_list_passes(self, capsys):
        manifest = make_manifest([])
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        verify_certificate(cert_path, manifest_path)
        captured = capsys.readouterr()
        assert "0 sorries" in captured.out


# ---------------------------------------------------------------------------
# Tests: bound output
# ---------------------------------------------------------------------------


class TestBoundOutput:
    def test_bound_printed_correctly(self, capsys):
        manifest = make_manifest()
        cert = build_cert("placeholder", target_min_log10=35, target_max_log10=37)
        cert_path, manifest_path = write_files(manifest, cert)
        verify_certificate(cert_path, manifest_path)
        captured = capsys.readouterr()
        assert "10^35" in captured.out
        assert "10^37" in captured.out

    def test_custom_bounds_printed(self, capsys):
        manifest = make_manifest()
        cert = build_cert("placeholder", target_min_log10=10, target_max_log10=20)
        cert_path, manifest_path = write_files(manifest, cert)
        verify_certificate(cert_path, manifest_path)
        captured = capsys.readouterr()
        assert "10^10" in captured.out
        assert "10^20" in captured.out


class TestContinuityChecker:
    def test_continuity_success(self):

        certs = [
            {"telemetry": {"target_min_log10": 30, "target_max_log10": 35}},
            {"telemetry": {"target_min_log10": 35, "target_max_log10": 40}},
            {"telemetry": {"target_min_log10": 40, "target_max_log10": 45}},
        ]
        # Should not raise
        check_continuity(certs)
        # Should be sorted
        assert certs[0]["telemetry"]["target_min_log10"] == 30
        assert certs[1]["telemetry"]["target_min_log10"] == 35
        assert certs[-1]["telemetry"]["target_min_log10"] == 40

    def test_continuity_gap_fails(self):

        certs = [
            {"telemetry": {"target_min_log10": 30, "target_max_log10": 35}},
            {"telemetry": {"target_min_log10": 36, "target_max_log10": 40}},  # GAP
            {"telemetry": {"target_min_log10": 40, "target_max_log10": 45}},
        ]
        with pytest.raises(SystemExit) as exc_info:
            check_continuity(certs)
        assert exc_info.value.code != 0

    def test_continuity_overlap_fails(self):

        certs = [
            {"telemetry": {"target_min_log10": 30, "target_max_log10": 35}},
            {"telemetry": {"target_min_log10": 34, "target_max_log10": 40}},  # OVERLAP
            {"telemetry": {"target_min_log10": 40, "target_max_log10": 45}},
        ]
        with pytest.raises(SystemExit) as exc_info:
            check_continuity(certs)
        assert exc_info.value.code != 0

    def test_continuity_out_of_order_sorts_first(self):

        certs = [
            {"telemetry": {"target_min_log10": 40, "target_max_log10": 45}},
            {"telemetry": {"target_min_log10": 30, "target_max_log10": 35}},
            {"telemetry": {"target_min_log10": 35, "target_max_log10": 40}},
        ]
        # Should not raise
        check_continuity(certs)
        assert certs[0]["telemetry"]["target_min_log10"] == 30
        assert certs[-1]["telemetry"]["target_max_log10"] == 45


class TestAggregationE2E:
    def test_aggregation_integration(self):

        tmpdir = tempfile.mkdtemp()
        cert_dir = os.path.join(tmpdir, "certs")
        os.mkdir(cert_dir)
        manifest = make_manifest()
        write_mock_manifest_files(tmpdir, manifest)

        bounds_content = b'{"dummy": "bounds"}'
        with open(os.path.join(tmpdir, "bounds_manifest.json"), "wb") as f:
            f.write(bounds_content)
        manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()

        with open(os.path.join(tmpdir, "proof_manifest.json"), "w") as f:
            json.dump(manifest, f)

        def write_signed_cert(idx, t_min, t_max):
            cert = build_cert(
                manifest["bounds_manifest_hash"],
                target_min_log10=t_min,
                target_max_log10=t_max,
            )

            manifest_content = json.dumps(manifest)
            manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()
            cert["manifest_hash"] = manifest_hash

            tel = cert["telemetry"]
            map_obj = {
                "manifest_hash": manifest_hash,
                "verified_logic_hash": cert["verified_logic_hash"],
                "total_branches_searched": tel["total_branches_searched"],
                "target_min_log10": tel["target_min_log10"],
                "target_max_log10": tel["target_max_log10"],
                "trace_hash": tel.get("trace_hash", ""),
                "factorization_depth": tel.get("factorization_depth", 0),
            }
            payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
            pub_hex, sig_hex = sign_payload(payload)
            cert["signature"] = sig_hex
            cert["public_key"] = pub_hex

            with open(os.path.join(cert_dir, f"cert_{idx}.json"), "w") as f:
                json.dump(cert, f)

        # Write contiguous sequence OUT OF ORDER
        write_signed_cert(1, 35, 40)
        write_signed_cert(2, 30, 35)
        write_signed_cert(3, 40, 45)

        script_path = os.path.abspath(
            os.path.join(os.path.dirname(__file__), "..", "verify_cert.py")
        )
        env = os.environ.copy()

        res = subprocess.run(
            [
                sys.executable,
                script_path,
                "--cert",
                cert_dir,
                "--manifest",
                os.path.join(tmpdir, "proof_manifest.json"),
            ],
            cwd=tmpdir,
            capture_output=True,
            text=True,
            env=env,
        )
        assert res.returncode == 0

        with open(os.path.join(tmpdir, "meta_certificate.json"), "r") as f:
            meta = json.load(f)

        assert meta["telemetry"]["target_min_log10"] == 30
        assert meta["telemetry"]["target_max_log10"] == 45


class TestManifestSecurityValidation:
    def test_missing_manifest_raises_error(self, tmp_path):
        """If manifest file is missing, validation raises error."""
        manifest = make_manifest()
        cert = build_cert("dummy_manifest_hash")
        cert_path, manifest_path = write_files(manifest, cert)

        # Set environment variable to a non-existent file
        os.environ["UALBF_PROOF_MANIFEST"] = os.path.join(
            str(tmp_path), "non_existent_manifest.json"
        )
        try:
            with pytest.raises(CertificateValidationError) as exc_info:
                load_and_validate_cert(cert_path)
            err_msg = str(exc_info.value)
            assert "Failed to retrieve runtime manifest" in err_msg
        finally:
            os.environ.pop("UALBF_PROOF_MANIFEST", None)

    def test_manifest_hash_mismatch_raises_error(self, tmp_path):
        """If manifest hash is mismatched, validation raises error."""
        manifest = make_manifest()
        cert = build_cert("dummy_manifest_hash")
        cert_path, manifest_path = write_files(manifest, cert)

        # Tamper with manifest file content so its hash changes
        with open(manifest_path, "w") as f:
            f.write(
                json.dumps(
                    {
                        "theorems": [],
                        "proof_files": [],
                        "bounds_manifest_hash": "dummy",
                    }
                )
            )

        os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
        try:
            with pytest.raises(CertificateValidationError) as exc_info:
                load_and_validate_cert(cert_path)
            assert "Manifest hash mismatch" in str(exc_info.value)
        finally:
            os.environ.pop("UALBF_PROOF_MANIFEST", None)


class TestConditionalCertificates:
    def test_conditional_cert_success(self, tmp_path):
        """A certificate with conditional metadata verifies successfully."""
        manifest = make_manifest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()
        
        cert = build_cert(manifest_hash)
        cert["is_conditional"] = True
        cert["conjecture"] = {
            "conditional": True,
            "conjecture_name": "ABC Conjecture",
            "conjectural_max_log10_ceiling": 30
        }
        
        # We need to manually format the payload for signing inside the test
        tel = cert["telemetry"]
        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": cert["verified_logic_hash"],
            "total_branches_searched": tel["total_branches_searched"],
            "target_min_log10": tel.get("target_min_log10", 35),
            "target_max_log10": tel["target_max_log10"],
            "trace_hash": tel.get("trace_hash", ""),
            "factorization_depth": tel.get("factorization_depth", 0),
            "is_conditional": True,
            "conjecture_name": "ABC Conjecture"
        }
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex
        
        cert_path = os.path.join(str(tmp_path), "formal_certificate.json")
        manifest_path = os.path.join(str(tmp_path), "proof_manifest.json")
        with open(cert_path, "w") as f:
            json.dump(cert, f)
        with open(manifest_path, "w") as f:
            f.write(manifest_content)
            
        os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
        try:
            # Should not raise any error
            meta = load_and_validate_cert(cert_path)
            assert meta["is_conditional"] is True
            assert meta["conjecture"]["conjecture_name"] == "ABC Conjecture"
        finally:
            os.environ.pop("UALBF_PROOF_MANIFEST", None)

