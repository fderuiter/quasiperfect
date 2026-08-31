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

from verify_cert import verify_certificate, check_continuity, verify_telemetry_paths
from cert_util import load_and_validate_cert, CertificateValidationError

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_manifest(theorems=None):
    """Return a minimal proof manifest dict."""
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    real_manifest_path = os.path.join(base_dir, "proof_manifest.json")
    proof_files = []
    if os.path.exists(real_manifest_path):
        with open(real_manifest_path, "r", encoding="utf-8") as f:
            real_m = json.load(f)
            proof_files = real_m.get("proof_files", [])

    if not proof_files:
        arith_file = "UALBF/Pure/Arithmetic.lean"
        arith_path = os.path.join(base_dir, "lean4-proofs", arith_file)
        try:
            with open(arith_path, "rb") as f:
                chk = hashlib.sha256(f.read()).hexdigest()
        except Exception:
            chk = "aabbccdd"
        proof_files = [{"file": arith_file, "checksum": chk}]

    if theorems is None:
        name = "UALBF.Pure.Arithmetic.foo"
        file = "UALBF/Pure/Arithmetic.lean"
        status = "proven"
        file_path = os.path.join(base_dir, "lean4-proofs", file)
        try:
            with open(file_path, "rb") as f:
                content = f.read()
        except Exception:
            content = b"mock content for " + file.encode("utf-8")
        checksum = hashlib.sha256(content).hexdigest()
        theorems = [
            {"name": name, "file": file, "status": status, "checksum": checksum},
        ]
    else:
        for t in theorems:
            tf = t["file"]
            file_path = os.path.join(base_dir, "lean4-proofs", tf)
            try:
                with open(file_path, "rb") as f:
                    content = f.read()
            except Exception:
                content = b"mock content for " + tf.encode("utf-8")
            if t.get("checksum") in ["x", "y", "allowed"]:
                t["checksum"] = hashlib.sha256(content).hexdigest()
            if not any(pf["file"] == tf for pf in proof_files):
                proof_files.append({"file": tf, "checksum": hashlib.sha256(content).hexdigest()})

    return {
        "theorems": theorems,
        "proof_files": proof_files,
    }


def sign_payload(payload_str: str) -> tuple[str, str]:
    """Return (public_key_hex, signature_hex) for the given payload string."""
    private_key = Ed25519PrivateKey.generate()
    sig = private_key.sign(payload_str.encode("utf-8"))
    pub = private_key.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    return pub.hex(), sig.hex()


def write_mock_manifest_files(tmpdir, manifest):
    """Write mock theorem and proof_files to tmpdir and update their checksums to physical hashes."""
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
    path_ranges: list | None = None,
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
            "path_ranges": (
                path_ranges
                if path_ranges is not None
                else [{"start_bound": [], "end_bound": []}]
            ),
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
    sidecar_hash = tel.get("sidecar_hash") or tel.get("sidecar_log_digest") or cert.get("sidecar_hash") or cert.get("sidecar_log_digest")
    if sidecar_hash:
        map_obj["sidecar_hash"] = sidecar_hash
    if "path_ranges" in tel:
        map_obj["path_ranges"] = tel["path_ranges"]
    elif "inner_paths" in tel:
        map_obj["path_ranges"] = tel["inner_paths"]
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
        local_cert = locals().get("cert")
        if local_cert is not None and "path_ranges" in local_cert.get("telemetry", {}):
            map_obj["path_ranges"] = local_cert["telemetry"]["path_ranges"]
        elif local_cert is not None and "inner_paths" in local_cert.get("telemetry", {}):
            map_obj["path_ranges"] = local_cert["telemetry"]["inner_paths"]
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

    def test_proof_file_modification_triggers_failure(self):
        """
        Verify that modifying a proof file's content triggers a verification failure.
        """
        manifest = make_manifest(
            [
                {
                    "name": "UALBF.Foo",
                    "file": "F.lean",
                    "status": "proven",
                    "checksum": "x",
                }
            ]
        )
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)

        # Manifest is written and verified initially
        verify_certificate(cert_path, manifest_path)

        # Now modify the physical theorem/proof file
        manifest_dir = os.path.dirname(os.path.abspath(manifest_path))
        file_path = os.path.join(manifest_dir, "F.lean")

        # Confirm file exists and then write modified content
        assert os.path.exists(file_path)
        with open(file_path, "wb") as f:
            f.write(b"modified theorem content")

        # Verify that checking again fails because of checksum mismatch
        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0

    def test_admit_tactic_triggers_failure(self):
        """
        Verify that detecting an 'admit' tactic in a proof file triggers a verification failure.
        """
        manifest = make_manifest(
            [
                {
                    "name": "UALBF.AdmitProof",
                    "file": "A.lean",
                    "status": "proven",
                    "checksum": "x",
                }
            ]
        )
        cert = build_cert("placeholder")
        cert_path, manifest_path = write_files(manifest, cert)
        manifest_dir = os.path.dirname(os.path.abspath(manifest_path))
        file_path = os.path.join(manifest_dir, "A.lean")

        assert os.path.exists(file_path)
        with open(file_path, "wb") as f:
            f.write(b"theorem foo : True := by\n  admit\n")

        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
        assert exc_info.value.code != 0


class TestSidecarLogVerification:
    def test_sidecar_log_digest_verification_pass(self, tmp_path):
        sidecar_file = tmp_path / "overflow_sidecar.log"
        sidecar_content = b"3,2\n5,4\n"
        sidecar_file.write_bytes(sidecar_content)
        sidecar_digest = hashlib.sha256(sidecar_content).hexdigest()

        manifest = make_manifest()
        cert = build_cert("placeholder")
        cert["telemetry"]["sidecar_hash"] = sidecar_digest
        cert_path, manifest_path = write_files(manifest, cert)

        from verify_cert import verify_sidecar_file
        with open(cert_path, "r", encoding="utf-8") as f:
            loaded_cert = json.load(f)

        # Re-verify sidecar file directly
        verify_sidecar_file(loaded_cert, str(sidecar_file))

    def test_sidecar_log_digest_mismatch_fails(self, tmp_path):
        sidecar_file = tmp_path / "overflow_sidecar.log"
        sidecar_content = b"3,2\n5,4\n"
        sidecar_file.write_bytes(sidecar_content)

        manifest = make_manifest()
        cert = build_cert("placeholder")
        cert["telemetry"]["sidecar_hash"] = "0" * 64
        cert_path, manifest_path = write_files(manifest, cert)

        from verify_cert import verify_sidecar_file
        with open(cert_path, "r", encoding="utf-8") as f:
            loaded_cert = json.load(f)

        with pytest.raises(SystemExit) as exc_info:
            verify_sidecar_file(loaded_cert, str(sidecar_file))
        assert exc_info.value.code != 0


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

        def write_signed_cert(idx, t_min, t_max, path_ranges):
            cert = build_cert(
                manifest["bounds_manifest_hash"],
                target_min_log10=t_min,
                target_max_log10=t_max,
                path_ranges=path_ranges,
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
            if "path_ranges" in tel:
                map_obj["path_ranges"] = tel["path_ranges"]
            elif "inner_paths" in tel:
                map_obj["path_ranges"] = tel["inner_paths"]
            payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
            pub_hex, sig_hex = sign_payload(payload)
            cert["signature"] = sig_hex
            cert["public_key"] = pub_hex

            with open(os.path.join(cert_dir, f"cert_{idx}.json"), "w") as f:
                json.dump(cert, f)

        # Write contiguous sequence OUT OF ORDER
        write_signed_cert(1, 35, 40, [{"start_bound": [2], "end_bound": [2, 3]}])
        write_signed_cert(2, 30, 35, [{"start_bound": [], "end_bound": [2]}])
        write_signed_cert(3, 40, 45, [{"start_bound": [2, 3], "end_bound": []}])

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
            "conjectural_max_log10_ceiling": 30,
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
            "conjecture_name": "ABC Conjecture",
        }
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        elif "inner_paths" in tel:
            map_obj["path_ranges"] = tel["inner_paths"]
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

    def test_verify_conditional_cert_prints_warning(self, tmp_path, capsys):
        """Verifying a conditional certificate prints a prominent warning message."""
        manifest = make_manifest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()

        cert = build_cert(manifest_hash)
        cert["is_conditional"] = True
        cert["conjecture"] = {
            "conditional": True,
            "conjecture_name": "ABC Conjecture",
            "conjectural_max_log10_ceiling": 30,
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
            "conjecture_name": "ABC Conjecture",
        }
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        elif "inner_paths" in tel:
            map_obj["path_ranges"] = tel["inner_paths"]
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex

        cert_path = os.path.join(str(tmp_path), "formal_certificate.json")
        manifest_path = os.path.join(str(tmp_path), "proof_manifest.json")
        bounds_path = os.path.join(str(tmp_path), "bounds_manifest.json")
        with open(cert_path, "w") as f:
            json.dump(cert, f)
        with open(manifest_path, "w") as f:
            f.write(manifest_content)
        with open(bounds_path, "wb") as f:
            f.write(b'{"dummy": "bounds"}')

        manifest["bounds_manifest_hash"] = hashlib.sha256(
            b'{"dummy": "bounds"}'
        ).hexdigest()
        with open(manifest_path, "w") as f:
            f.write(json.dumps(manifest))

        # Re-sign with correct manifest hash
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()
        cert["manifest_hash"] = manifest_hash
        map_obj["manifest_hash"] = manifest_hash
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        elif "inner_paths" in tel:
            map_obj["path_ranges"] = tel["inner_paths"]
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex
        with open(cert_path, "w") as f:
            json.dump(cert, f)

        os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
        try:
            verify_certificate(cert_path, manifest_path)
            captured = capsys.readouterr()
            assert (
                "WARNING: THIS CERTIFICATE WAS GENERATED IN CONJECTURAL MODE!"
                in captured.out
            )
            assert "ABC Conjecture" in captured.out
        finally:
            os.environ.pop("UALBF_PROOF_MANIFEST", None)


class TestPathContinuityValidation:
    def test_missing_path_ranges_field_fails(self, tmp_path):
        """If path_ranges is missing from telemetry, verification fails."""
        manifest = make_manifest()
        cert = build_cert("placeholder")
        # Explicitly remove path_ranges from telemetry
        del cert["telemetry"]["path_ranges"]

        cert_path, manifest_path = write_files(manifest, cert)

        # When running the verification, it should fail due to missing path_ranges
        with pytest.raises(SystemExit) as exc_info:
            verify_certificate(cert_path, manifest_path)
            verify_telemetry_paths([cert])
        assert exc_info.value.code != 0

    def test_path_ranges_gap_fails_and_writes_recovery(self, tmp_path, monkeypatch):
        """If there is a gap in path ranges, verification fails and writes recovery file."""
        manifest = make_manifest()

        # Create a gap: [] to [2], then [3] to [] (missing [2] to [3])
        path_ranges = [
            {"start_bound": [], "end_bound": [2]},
            {"start_bound": [3], "end_bound": []},
        ]
        cert = build_cert("placeholder", path_ranges=path_ranges)
        cert_path, manifest_path = write_files(manifest, cert)

        # Change directory to tmp_path so the recovery file is written there
        monkeypatch.chdir(tmp_path)

        recovery_file = os.path.join(str(tmp_path), "recovery_work_units.json")
        if os.path.exists(recovery_file):
            os.remove(recovery_file)

        from verify_cert import verify_telemetry_paths

        with pytest.raises(SystemExit) as exc_info:
            # We want to run the verify_telemetry_paths call
            verify_telemetry_paths([cert])
        assert exc_info.value.code != 0

        # Verify recovery file exists and contains the correct gap: start [2] -> end [3]
        assert os.path.exists(recovery_file)
        with open(recovery_file, "r") as f:
            gaps = json.load(f)
        assert len(gaps) == 1
        assert gaps[0]["start_bound"] == [2]
        assert gaps[0]["end_bound"] == [3]

    def test_path_ranges_multiple_gaps(self, tmp_path, monkeypatch):
        """Test detection of multiple gaps in the path chain."""
        # [2] to [3] is missing, [4] to [] is missing
        path_ranges = [
            {"start_bound": [], "end_bound": [2]},
            {"start_bound": [3], "end_bound": [4]},
        ]
        cert = build_cert("placeholder", path_ranges=path_ranges)
        monkeypatch.chdir(tmp_path)

        recovery_file = os.path.join(str(tmp_path), "recovery_work_units.json")
        if os.path.exists(recovery_file):
            os.remove(recovery_file)

        from verify_cert import verify_telemetry_paths

        with pytest.raises(SystemExit):
            verify_telemetry_paths([cert])

        assert os.path.exists(recovery_file)
        with open(recovery_file, "r") as f:
            gaps = json.load(f)
        assert len(gaps) == 2
        assert gaps[0]["start_bound"] == [2]
        assert gaps[0]["end_bound"] == [3]
        assert gaps[1]["start_bound"] == [4]
        assert gaps[1]["end_bound"] == []

    def test_path_ranges_performance_10k(self):
        """Verify processing of 10,000 path ranges is fast (under 2 seconds)."""
        import time

        # Generate 10,000 contiguous path ranges
        # [ [], [1] ], [ [1], [2] ], ..., [ [9999], [] ]
        path_ranges = []
        path_ranges.append({"start_bound": [], "end_bound": [1]})
        for i in range(1, 9999):
            path_ranges.append({"start_bound": [i], "end_bound": [i + 1]})
        path_ranges.append({"start_bound": [9999], "end_bound": []})

        import verification_lib  # type: ignore

        path_ranges_json = json.dumps(path_ranges)

        start_time = time.time()
        result_json = verification_lib.check_path_continuity(path_ranges_json)
        end_time = time.time()

        duration = end_time - start_time
        assert duration < 2.0

        result = json.loads(result_json)
        assert result["is_continuous"] is True
        assert len(result["gaps"]) == 0


class TestDirectMappingAndSchemaEnforcement:
    def test_null_byte_injection(self):
        """No certificate content preceding a null-byte can be parsed separately; fails immediately."""
        import verification_lib  # type: ignore

        raw_json_with_null = '{"manifest_hash": "abc", "\0": "tampered"}'
        with pytest.raises(ValueError, match="Null byte detected"):
            verification_lib.validate_certificate(raw_json_with_null)

    def test_tampered_path_ranges(self, tmp_path):
        """Certificates with modified search space boundaries or path ranges fail signature verification."""
        manifest = make_manifest()
        write_mock_manifest_files(str(tmp_path), manifest)
        bounds_content = b'{"dummy": "bounds"}'
        manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()

        # Write manifest file to disk
        manifest_path = os.path.join(str(tmp_path), "manifest.json")
        with open(manifest_path, "w") as f:
            f.write(manifest_content)

        # Build cert with path ranges
        cert = build_cert(
            manifest_hash, path_ranges=[{"start_bound": [1], "end_bound": [2]}]
        )
        cert["manifest_hash"] = manifest_hash

        # Sign it
        tel = cert["telemetry"]
        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": cert["verified_logic_hash"],
            "total_branches_searched": tel["total_branches_searched"],
            "target_min_log10": tel.get("target_min_log10", 35),
            "target_max_log10": tel["target_max_log10"],
            "trace_hash": tel.get("trace_hash", ""),
            "factorization_depth": tel.get("factorization_depth", 0),
            "path_ranges": tel["path_ranges"],
        }
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex

        # Check it passes first
        import verification_lib  # type: ignore

        os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
        res = verification_lib.validate_certificate(json.dumps(cert))
        assert isinstance(res, dict)

        # Tamper with path ranges
        cert["telemetry"]["path_ranges"] = [{"start_bound": [1], "end_bound": [3]}]
        with pytest.raises(Exception, match="Invalid cryptographic signature"):
            verification_lib.validate_certificate(json.dumps(cert))

    def test_telemetry_integer_overflow(self, tmp_path):
        """Certificates with telemetry integer values exceeding maximum representation limits are rejected."""
        manifest = make_manifest()
        write_mock_manifest_files(str(tmp_path), manifest)
        bounds_content = b'{"dummy": "bounds"}'
        manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()

        # Write manifest file to disk
        manifest_path = os.path.join(str(tmp_path), "manifest.json")
        with open(manifest_path, "w") as f:
            f.write(manifest_content)

        cert = build_cert(manifest_hash)
        cert["manifest_hash"] = manifest_hash

        # Insert overflowing integer in telemetry
        cert["telemetry"][
            "total_branches_searched"
        ] = 18446744073709551616  # 2^64 (exceeds u64 limit)

        # Resign with overflowed value
        tel = cert["telemetry"]
        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": cert["verified_logic_hash"],
            "total_branches_searched": tel["total_branches_searched"],
            "target_min_log10": tel.get("target_min_log10", 35),
            "target_max_log10": tel["target_max_log10"],
            "trace_hash": tel.get("trace_hash", ""),
            "factorization_depth": tel.get("factorization_depth", 0),
        }
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex

        import verification_lib  # type: ignore

        os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
        with pytest.raises(
            ValueError,
            match="Telemetry validation failed: Telemetry number .* exceeds 64-bit integer limits",
        ):
            verification_lib.validate_certificate(json.dumps(cert))

    def test_direct_pyobject_return(self, tmp_path):
        """The verification engine successfully transfers the validated certificate data to Python directly as a native dictionary without invoking a secondary JSON string parser."""
        manifest = make_manifest()
        write_mock_manifest_files(str(tmp_path), manifest)
        bounds_content = b'{"dummy": "bounds"}'
        manifest["bounds_manifest_hash"] = hashlib.sha256(bounds_content).hexdigest()
        manifest_content = json.dumps(manifest)
        manifest_hash = hashlib.sha256(manifest_content.encode()).hexdigest()

        # Write manifest file to disk
        manifest_path = os.path.join(str(tmp_path), "manifest.json")
        with open(manifest_path, "w") as f:
            f.write(manifest_content)

        cert = build_cert(manifest_hash)
        cert["manifest_hash"] = manifest_hash

        # Re-sign
        tel = cert["telemetry"]
        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": cert["verified_logic_hash"],
            "total_branches_searched": tel["total_branches_searched"],
            "target_min_log10": tel.get("target_min_log10", 35),
            "target_max_log10": tel["target_max_log10"],
            "trace_hash": tel.get("trace_hash", ""),
            "factorization_depth": tel.get("factorization_depth", 0),
        }
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex

        import cert_util

        os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
        cert_path = os.path.join(str(tmp_path), "cert.json")
        with open(cert_path, "w") as f:
            json.dump(cert, f)

        res = cert_util.load_and_validate_cert(cert_path)
        assert isinstance(res, dict)
        assert res["manifest_hash"] == manifest_hash


def test_post_execution_schmidt_bound_lattice_certification(tmp_path):
    """
    Test Post-Execution Schmidt-Bound Lattice Certification.
    Verify:
    1. Valid lattice witness successfully passes offline verification.
    2. Non-unimodular transformation matrix is rejected with exit(1).
    3. Invalid Schmidt bound is rejected with exit(1).
    4. Out-of-bounds epsilon is rejected with exit(1).
    """
    import os
    import json
    import pytest
    from verify_cert import verify_certificate

    # 1. Create a mock bounds manifest with target_min_log10 = 37 and tolerance = 1e-9
    bounds_manifest = {
        "search_bounds": {"target_min_log10": {"value": 37, "is_axiomatic": False}},
        "lattice_precision_tolerance": 1e-9,
    }
    bounds_content = json.dumps(bounds_manifest).encode("utf-8")
    bounds_hash = hashlib.sha256(bounds_content).hexdigest()
    bounds_path = os.path.join(str(tmp_path), "bounds_manifest.json")
    with open(bounds_path, "wb") as f:
        f.write(bounds_content)

    # 2. Create a mock manifest containing bounds_manifest_hash
    manifest = make_manifest()
    manifest["bounds_manifest_hash"] = bounds_hash
    manifest_content = json.dumps(manifest)
    manifest_path = os.path.join(str(tmp_path), "proof_manifest.json")
    with open(manifest_path, "w") as f:
        f.write(manifest_content)

    # Re-sign helper to build valid certified payload
    def build_signed_cert_with_witnesses(witnesses):
        manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()
        cert = build_cert(manifest_hash)
        cert["manifest_hash"] = manifest_hash
        cert["lattice_witnesses"] = witnesses

        tel = cert["telemetry"]
        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": cert["verified_logic_hash"],
            "total_branches_searched": tel["total_branches_searched"],
            "target_min_log10": tel.get("target_min_log10", 37),
            "target_max_log10": tel["target_max_log10"],
            "trace_hash": tel.get("trace_hash", ""),
            "factorization_depth": tel.get("factorization_depth", 0),
        }
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex
        return cert

    # Construct a mathematically valid witness (fully LLL-reduced and satisfying Schmidt bound):
    # Let m = 2 (dim = 3).
    # w = [80, 130], t = 150.
    # B_init = [[1, 0, 80], [0, 1, 130], [0, 0, -150]]
    # Let U be: [[-3, 3, 1], [4, 1, 3], [-2, 0, -1]]
    # B_reduced = U * B_init = [[-3, 3, 0], [4, 1, 0], [-2, 0, -10]]
    # b0 = [-3, 3, 0]
    # norm(b0)^2 = 18
    # diff = 18 / 4 - 2 = 4.5 - 2 = 2.5
    # epsilon = 1e-15, dim = 3 (m = 2)
    # r = 1.5 + 1e9 * 1e-15 = 1.500001
    # r_sq = 2.250003000001
    # diff = 2.5 >= 2.250003000001, so diff is valid!
    valid_witnesses = [
        {
            "dimension": 3,
            "w": ["80", "130"],
            "t": "150",
            "transformation_matrix": [
                ["-3", "3", "1"],
                ["4", "1", "3"],
                ["-2", "0", "-1"],
            ],
            "epsilon": 1e-15,
            "target_log": 0.1,
        }
    ]

    # Case 1: Valid witnesses pass
    cert_valid = build_signed_cert_with_witnesses(valid_witnesses)
    cert_valid_path = os.path.join(str(tmp_path), "cert_valid.json")
    with open(cert_valid_path, "w") as f:
        json.dump(cert_valid, f)

    os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
    verify_certificate(cert_valid_path, manifest_path)  # Must not exit/error!

    # Case 2: Non-unimodular transformation matrix is rejected
    non_unimodular_witnesses = [
        {
            "dimension": 3,
            "w": ["100", "200"],
            "t": "50",
            "transformation_matrix": [
                ["2", "0", "0"],  # Det of this U is 2, not 1 or -1!
                ["0", "1", "0"],
                ["0", "0", "1"],
            ],
            "epsilon": 1e-15,
            "target_log": 0.1,
        }
    ]
    cert_invalid_uni = build_signed_cert_with_witnesses(non_unimodular_witnesses)
    cert_invalid_uni_path = os.path.join(str(tmp_path), "cert_invalid_uni.json")
    with open(cert_invalid_uni_path, "w") as f:
        json.dump(cert_invalid_uni, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_invalid_uni_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 3: Invalid Schmidt bound is rejected
    # (Let's make U result in a very small shortest vector, e.g. b0 has norm 0 or diff <= r_sq)
    # Let w = [0, 0], t = 1, epsilon = 0.1 (not allowed directly but let's make diff small)
    # B_init = [[1, 0, 0], [0, 1, 0], [0, 0, -1]]
    # U = identity, so B_reduced = B_init, b0 = [1, 0, 0] -> norm(b0)^2 = 1.
    # diff = 1/4 - 2 = -1.75 <= 0, which is invalid!
    invalid_bound_witnesses = [
        {
            "dimension": 3,
            "w": ["0", "0"],
            "t": "1",
            "transformation_matrix": [
                ["1", "0", "0"],
                ["0", "1", "0"],
                ["0", "0", "1"],
            ],
            "epsilon": 1e-15,
            "target_log": 0.1,
        }
    ]
    cert_invalid_bound = build_signed_cert_with_witnesses(invalid_bound_witnesses)
    cert_invalid_bound_path = os.path.join(str(tmp_path), "cert_invalid_bound.json")
    with open(cert_invalid_bound_path, "w") as f:
        json.dump(cert_invalid_bound, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_invalid_bound_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 4: Out-of-bounds epsilon is rejected
    out_of_bounds_epsilon_witnesses = [
        {
            "dimension": 3,
            "w": ["100", "200"],
            "t": "50",
            "transformation_matrix": [
                ["1", "0", "0"],
                ["0", "1", "0"],
                ["0", "0", "1"],
            ],
            "epsilon": 0.5,  # Exceeds epsilon_manifest (~0.0) massively!
            "target_log": 0.1,
        }
    ]
    cert_invalid_eps = build_signed_cert_with_witnesses(out_of_bounds_epsilon_witnesses)
    cert_invalid_eps_path = os.path.join(str(tmp_path), "cert_invalid_eps.json")
    with open(cert_invalid_eps_path, "w") as f:
        json.dump(cert_invalid_eps, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_invalid_eps_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 5: Borderline Schmidt bound within tolerance but strictly below exact radius is rejected
    # (Shows zero-tolerance exact rational verification)
    # dim = 3, m = 2. We set shortest_sq_norm = 17, so diff = 17/4 - 2 = 2.25.
    # We choose epsilon = 3.3333333333333335e-20.
    # Then r_exact = 1.5 + 10^9 * epsilon = 1.5000000000333333
    # r_sq_exact = 2.250000000100000000001111111... > 2.25
    # The true difference is r_sq_exact - diff_exact ≈ 1e-10 < tolerance (1e-9).
    # This borderline case must be rejected.
    borderline_witnesses = [
        {
            "dimension": 3,
            "w": ["4", "0"],
            "t": "1",
            "transformation_matrix": [
                ["1", "0", "0"],
                ["0", "1", "0"],
                ["0", "0", "1"],
            ],
            "epsilon": 3.3333333333333335e-20,
            "target_log": 0.1,
        }
    ]
    cert_borderline = build_signed_cert_with_witnesses(borderline_witnesses)
    cert_borderline_path = os.path.join(str(tmp_path), "cert_borderline.json")
    with open(cert_borderline_path, "w") as f:
        json.dump(cert_borderline, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_borderline_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 6: Basis is not size-reduced is rejected
    non_sizered_witnesses = [{
        "dimension": 3,
        "w": ["100", "200"],
        "t": "50",
        "transformation_matrix": [
            ["1", "0", "0"],
            ["0", "1", "0"],
            ["0", "0", "1"]
        ],
        "epsilon": 1e-15,
        "target_log": 0.1
    }]
    cert_non_sizered = build_signed_cert_with_witnesses(non_sizered_witnesses)
    cert_non_sizered_path = os.path.join(str(tmp_path), "cert_non_sizered.json")
    with open(cert_non_sizered_path, "w") as f:
        json.dump(cert_non_sizered, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_non_sizered_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 7: Basis size-reduced but violates Lovasz condition is rejected
    non_lovasz_witnesses = [{
        "dimension": 3,
        "w": ["4", "0"],
        "t": "2",
        "transformation_matrix": [
            ["1", "0", "0"],
            ["0", "1", "0"],
            ["0", "0", "1"]
        ],
        "epsilon": 1e-15,
        "target_log": 0.1
    }]
    cert_non_lovasz = build_signed_cert_with_witnesses(non_lovasz_witnesses)
    cert_non_lovasz_path = os.path.join(str(tmp_path), "cert_non_lovasz.json")
    with open(cert_non_lovasz_path, "w") as f:
        json.dump(cert_non_lovasz, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_non_lovasz_path, manifest_path)
    assert excinfo.value.code == 1


from verify_cert import verify_trace_file


class TestGraphTopologyVerification:
    def test_valid_topology_manifest_passes(self, tmp_path):
        trace_path = os.path.join(tmp_path, "trace.jsonl")
        topology_manifest = {
            "adjacency": [[1], [2], []],
            "scc_map": [0, 1, 2],
            "scc_components": [[0], [1], [2]],
            "forced_candidates": [[1], [2], []]
        }
        reachable_paths = [[0, 1, 2]]
        record = {
            "reason": "cdg_forced_cascade",
            "verification_status": "formally verified (graph topology audited)",
            "topology_manifest": topology_manifest,
            "reachable_paths": reachable_paths,
            "n_l": "1",
            "s_l": "1",
            "factors": []
        }
        trace_content = json.dumps(record) + "\n"
        with open(trace_path, "w", encoding="utf-8") as f:
            f.write(trace_content)

        trace_hash = hashlib.sha256(trace_content.encode("utf-8")).hexdigest()
        cert = {"telemetry": {"trace_hash": trace_hash}}

        # Should pass without SystemExit
        verify_trace_file(cert, trace_path)

    def test_missing_topology_manifest_fails(self, tmp_path):
        trace_path = os.path.join(tmp_path, "trace.jsonl")
        record = {
            "reason": "cdg_forced_cascade",
            "verification_status": "formally verified",
            "n_l": "1",
            "s_l": "1",
            "factors": []
        }
        trace_content = json.dumps(record) + "\n"
        with open(trace_path, "w", encoding="utf-8") as f:
            f.write(trace_content)

        trace_hash = hashlib.sha256(trace_content.encode("utf-8")).hexdigest()
        cert = {"telemetry": {"trace_hash": trace_hash}}

        with pytest.raises(SystemExit) as excinfo:
            verify_trace_file(cert, trace_path)
        assert excinfo.value.code == 1

    def test_invalid_neighbor_out_of_bounds_fails(self, tmp_path):
        trace_path = os.path.join(tmp_path, "trace.jsonl")
        topology_manifest = {
            "adjacency": [[99]], # 99 is out of bounds
            "scc_map": [0],
            "scc_components": [[0]],
            "forced_candidates": [[99]]
        }
        record = {
            "reason": "cdg_forced_cascade",
            "verification_status": "formally verified",
            "topology_manifest": topology_manifest,
            "reachable_paths": [[0]],
            "n_l": "1",
            "s_l": "1",
            "factors": []
        }
        trace_content = json.dumps(record) + "\n"
        with open(trace_path, "w", encoding="utf-8") as f:
            f.write(trace_content)

        trace_hash = hashlib.sha256(trace_content.encode("utf-8")).hexdigest()
        cert = {"telemetry": {"trace_hash": trace_hash}}

        with pytest.raises(SystemExit) as excinfo:
            verify_trace_file(cert, trace_path)
        assert excinfo.value.code == 1

    def test_scc_mismatch_fails(self, tmp_path):
        trace_path = os.path.join(tmp_path, "trace.jsonl")
        topology_manifest = {
            "adjacency": [[1], []],
            "scc_map": [0, 0], # node 1 mapped to scc 0, but missing from scc_components[0]
            "scc_components": [[0], [1]],
            "forced_candidates": [[1], []]
        }
        record = {
            "reason": "cdg_forced_cascade",
            "verification_status": "formally verified",
            "topology_manifest": topology_manifest,
            "reachable_paths": [[0, 1]],
            "n_l": "1",
            "s_l": "1",
            "factors": []
        }
        trace_content = json.dumps(record) + "\n"
        with open(trace_path, "w", encoding="utf-8") as f:
            f.write(trace_content)

        trace_hash = hashlib.sha256(trace_content.encode("utf-8")).hexdigest()
        cert = {"telemetry": {"trace_hash": trace_hash}}

        with pytest.raises(SystemExit) as excinfo:
            verify_trace_file(cert, trace_path)
        assert excinfo.value.code == 1

    def test_unreachable_path_step_fails(self, tmp_path):
        trace_path = os.path.join(tmp_path, "trace.jsonl")
        topology_manifest = {
            "adjacency": [[], []], # no edges
            "scc_map": [0, 1],
            "scc_components": [[0], [1]],
            "forced_candidates": [[], []]
        }
        # Path claims 0 -> 1, but 0 -> 1 is NOT reachable
        reachable_paths = [[0, 1]]
        record = {
            "reason": "cdg_forced_cascade",
            "verification_status": "formally verified",
            "topology_manifest": topology_manifest,
            "reachable_paths": reachable_paths,
            "n_l": "1",
            "s_l": "1",
            "factors": []
        }
        trace_content = json.dumps(record) + "\n"
        with open(trace_path, "w", encoding="utf-8") as f:
            f.write(trace_content)

        trace_hash = hashlib.sha256(trace_content.encode("utf-8")).hexdigest()
        cert = {"telemetry": {"trace_hash": trace_hash}}

        with pytest.raises(SystemExit) as excinfo:
            verify_trace_file(cert, trace_path)
        assert excinfo.value.code == 1


def test_compositeness_witness_certification(tmp_path):
    """
    Test Compositeness Witness Certification for Candidates under 2^64.
    Verify:
    1. Valid divisor witness passes verification.
    2. Valid Miller-Rabin witness passes verification.
    3. Invalid divisor witness is rejected with exit(1).
    4. Invalid Miller-Rabin witness is rejected with exit(1).
    5. Out-of-bounds candidate is rejected with exit(1).
    6. Unknown witness_type is rejected with exit(1).
    """
    from verify_cert import verify_certificate

    bounds_manifest = {
        "search_bounds": {
            "target_min_log10": {
                "value": 35,
                "is_axiomatic": False
            }
        }
    }
    bounds_content = json.dumps(bounds_manifest).encode("utf-8")
    bounds_hash = hashlib.sha256(bounds_content).hexdigest()
    bounds_path = os.path.join(str(tmp_path), "bounds_manifest.json")
    with open(bounds_path, "wb") as f:
        f.write(bounds_content)

    manifest = make_manifest()
    manifest["bounds_manifest_hash"] = bounds_hash
    manifest_content = json.dumps(manifest)
    manifest_path = os.path.join(str(tmp_path), "proof_manifest.json")
    with open(manifest_path, "w") as f:
        f.write(manifest_content)

    def build_signed_cert_with_comp_witnesses(witnesses):
        manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()
        cert = build_cert(manifest_hash)
        cert["manifest_hash"] = manifest_hash
        cert["compositeness_witnesses"] = witnesses

        tel = cert["telemetry"]
        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": cert["verified_logic_hash"],
            "total_branches_searched": tel["total_branches_searched"],
            "target_min_log10": tel.get("target_min_log10", 35),
            "target_max_log10": tel["target_max_log10"],
            "trace_hash": tel.get("trace_hash", ""),
            "factorization_depth": tel.get("factorization_depth", 0),
        }
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex
        return cert

    # Case 1: Valid divisor and Miller-Rabin witnesses pass
    valid_witnesses = [
        {"candidate": 100, "witness_type": "divisor", "witness": 2},
        {"candidate": 1681, "witness_type": "miller_rabin", "witness": 3},
    ]
    cert_valid = build_signed_cert_with_comp_witnesses(valid_witnesses)
    cert_valid_path = os.path.join(str(tmp_path), "cert_valid_comp.json")
    with open(cert_valid_path, "w") as f:
        json.dump(cert_valid, f)

    os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
    verify_certificate(cert_valid_path, manifest_path)  # Must pass without error

    # Case 2: Invalid divisor witness (3 does not divide 100) is rejected
    invalid_div_witnesses = [
        {"candidate": 100, "witness_type": "divisor", "witness": 3}
    ]
    cert_invalid_div = build_signed_cert_with_comp_witnesses(invalid_div_witnesses)
    cert_invalid_div_path = os.path.join(str(tmp_path), "cert_invalid_div.json")
    with open(cert_invalid_div_path, "w") as f:
        json.dump(cert_invalid_div, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_invalid_div_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 3: Invalid Miller-Rabin witness (17 is prime, 2 is not a witness) is rejected
    invalid_mr_witnesses = [
        {"candidate": 17, "witness_type": "miller_rabin", "witness": 2}
    ]
    cert_invalid_mr = build_signed_cert_with_comp_witnesses(invalid_mr_witnesses)
    cert_invalid_mr_path = os.path.join(str(tmp_path), "cert_invalid_mr.json")
    with open(cert_invalid_mr_path, "w") as f:
        json.dump(cert_invalid_mr, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_invalid_mr_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 4: Candidate >= 2^64 is rejected
    out_of_bounds_witnesses = [
        {"candidate": 18446744073709551616, "witness_type": "divisor", "witness": 2}
    ]
    cert_oob = build_signed_cert_with_comp_witnesses(out_of_bounds_witnesses)
    cert_oob_path = os.path.join(str(tmp_path), "cert_oob.json")
    with open(cert_oob_path, "w") as f:
        json.dump(cert_oob, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_oob_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 5: Unknown witness_type is rejected
    unknown_type_witnesses = [
        {"candidate": 100, "witness_type": "magic", "witness": 2}
    ]
    cert_unknown = build_signed_cert_with_comp_witnesses(unknown_type_witnesses)
    cert_unknown_path = os.path.join(str(tmp_path), "cert_unknown.json")
    with open(cert_unknown_path, "w") as f:
        json.dump(cert_unknown, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_unknown_path, manifest_path)
    assert excinfo.value.code == 1


def test_verify_gpu_witnesses_cases(tmp_path):
    """
    Validates that:
    1. Valid GPU witnesses successfully pass offline verification.
    2. Invalid GPU residue/congruence is rejected.
    3. Invalid Bloom filter index is rejected.
    """
    import os
    import json
    import struct
    import hashlib
    import pytest
    from verify_cert import verify_certificate

    # Helper to calculate hashes & indices
    def get_component_hashes(p: int, two_e: int) -> tuple[int, int]:
        data = struct.pack(">Q I", p, two_e)
        h = hashlib.sha256(data).digest()
        hash1 = struct.unpack(">Q", h[0:8])[0]
        hash2 = struct.unpack(">Q", h[8:16])[0]
        return hash1, hash2

    def ualbf_bloom_get_index(hash1: int, hash2: int, num_bits: int, i: int) -> int:
        cur = hash1 + i * hash2 + (i * (i - 1)) // 2
        max_bits = 1 if num_bits == 0 else num_bits
        return cur % max_bits

    # 1. Create a mock bounds manifest
    bounds_manifest = {
        "search_bounds": {"target_min_log10": {"value": 37, "is_axiomatic": False}},
        "lattice_precision_tolerance": 1e-9,
    }
    bounds_content = json.dumps(bounds_manifest).encode("utf-8")
    bounds_hash = hashlib.sha256(bounds_content).hexdigest()
    bounds_path = os.path.join(str(tmp_path), "bounds_manifest.json")
    with open(bounds_path, "wb") as f:
        f.write(bounds_content)

    # 2. Create a mock manifest
    manifest = make_manifest()
    manifest["bounds_manifest_hash"] = bounds_hash
    manifest_content = json.dumps(manifest)
    manifest_path = os.path.join(str(tmp_path), "proof_manifest.json")
    with open(manifest_path, "w") as f:
        f.write(manifest_content)

    # Re-sign helper to build valid certified payload
    def build_signed_cert_with_gpu_witnesses(witnesses):
        manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()
        cert = build_cert(manifest_hash)
        cert["manifest_hash"] = manifest_hash
        cert["gpu_witnesses"] = witnesses

        tel = cert["telemetry"]
        map_obj = {
            "manifest_hash": manifest_hash,
            "verified_logic_hash": cert["verified_logic_hash"],
            "total_branches_searched": tel["total_branches_searched"],
            "target_min_log10": tel.get("target_min_log10", 37),
            "target_max_log10": tel["target_max_log10"],
            "trace_hash": tel.get("trace_hash", ""),
            "factorization_depth": tel.get("factorization_depth", 0),
        }
        if "path_ranges" in tel:
            map_obj["path_ranges"] = tel["path_ranges"]
        payload = json.dumps(map_obj, separators=(",", ":"), sort_keys=True)
        pub_hex, sig_hex = sign_payload(payload)
        cert["signature"] = sig_hex
        cert["public_key"] = pub_hex
        return cert

    # Component p=3, two_e=2 -> residues:
    # mod 3: 1+3+9 = 13 = 1
    # mod 5: 1+3+9 = 13 = 3
    # mod 7: 1+3+9 = 13 = 6
    # mod 11: 1+3+9 = 13 = 2
    # Not obstructed, so obstructing_modulus = 0, is_obstructed = False
    h1, h2 = get_component_hashes(3, 2)
    b_idx = [ualbf_bloom_get_index(h1, h2, 1048576, i) for i in range(4)]

    valid_gpu_witnesses = [
        {
            "p": 3,
            "two_e": 2,
            "is_obstructed": False,
            "obstructing_modulus": 0,
            "residues": [1, 3, 6, 2],
            "bloom_indices": b_idx,
        }
    ]

    # Case 1: Valid passes
    cert_valid = build_signed_cert_with_gpu_witnesses(valid_gpu_witnesses)
    cert_valid_path = os.path.join(str(tmp_path), "cert_gpu_valid.json")
    with open(cert_valid_path, "w") as f:
        json.dump(cert_valid, f)

    os.environ["UALBF_PROOF_MANIFEST"] = manifest_path
    verify_certificate(cert_valid_path, manifest_path)

    # Case 2: Invalid residue rejected
    invalid_res_witnesses = [
        {
            "p": 3,
            "two_e": 2,
            "is_obstructed": False,
            "obstructing_modulus": 0,
            "residues": [9, 3, 6, 2],  # 9 is wrong mod 3
            "bloom_indices": b_idx,
        }
    ]
    cert_invalid_res = build_signed_cert_with_gpu_witnesses(invalid_res_witnesses)
    cert_invalid_res_path = os.path.join(str(tmp_path), "cert_gpu_invalid_res.json")
    with open(cert_invalid_res_path, "w") as f:
        json.dump(cert_invalid_res, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_invalid_res_path, manifest_path)
    assert excinfo.value.code == 1

    # Case 3: Invalid bloom index rejected
    wrong_b_idx = list(b_idx)
    wrong_b_idx[0] += 1
    invalid_idx_witnesses = [
        {
            "p": 3,
            "two_e": 2,
            "is_obstructed": False,
            "obstructing_modulus": 0,
            "residues": [1, 3, 6, 2],
            "bloom_indices": wrong_b_idx,
        }
    ]
    cert_invalid_idx = build_signed_cert_with_gpu_witnesses(invalid_idx_witnesses)
    cert_invalid_idx_path = os.path.join(str(tmp_path), "cert_gpu_invalid_idx.json")
    with open(cert_invalid_idx_path, "w") as f:
        json.dump(cert_invalid_idx, f)

    with pytest.raises(SystemExit) as excinfo:
        verify_certificate(cert_invalid_idx_path, manifest_path)
    assert excinfo.value.code == 1


def test_trace_canonicalization_verification(tmp_path):
    trace_path = os.path.join(tmp_path, "trace.jsonl")
    lines_out_of_order = [
        {"work_unit_id": 1, "step_index": 0, "reason": "target_bound", "n_l": "200", "s_l": "100", "factors": []},
        {"work_unit_id": 0, "step_index": 1, "reason": "min_factors", "n_l": "150", "s_l": "100", "factors": []},
        {"work_unit_id": 0, "step_index": 0, "reason": "touchard", "n_l": "100", "s_l": "100", "factors": []},
    ]
    with open(trace_path, "w", encoding="utf-8") as f:
        for rec in lines_out_of_order:
            f.write(json.dumps(rec) + "\n")

    lines_sorted = [
        {"work_unit_id": 0, "step_index": 0, "reason": "touchard", "n_l": "100", "s_l": "100", "factors": []},
        {"work_unit_id": 0, "step_index": 1, "reason": "min_factors", "n_l": "150", "s_l": "100", "factors": []},
        {"work_unit_id": 1, "step_index": 0, "reason": "target_bound", "n_l": "200", "s_l": "100", "factors": []},
    ]
    canonical_text = "".join(json.dumps(rec) + "\n" for rec in lines_sorted)
    expected_trace_hash = hashlib.sha256(canonical_text.encode("utf-8")).hexdigest()

    cert = {"telemetry": {"trace_hash": expected_trace_hash}}

    verify_trace_file(cert, trace_path)

    with open(trace_path, "r", encoding="utf-8") as f:
        content = f.read()
    assert content == canonical_text
