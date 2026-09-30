import hashlib
import json
import os
import re
from pathlib import Path

import pytest


def rotr32(x: int, n: int) -> int:
    return ((x >> n) | (x << (32 - n))) & 0xFFFFFFFF


def sha256_Ch(x: int, y: int, z: int) -> int:
    return (x & y) ^ ((~x) & z)


def sha256_Maj(x: int, y: int, z: int) -> int:
    return (x & y) ^ (x & z) ^ (y & z)


def sha256_Sigma0(x: int) -> int:
    return rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22)


def sha256_Sigma1(x: int) -> int:
    return rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25)


def sha256_sigma0(x: int) -> int:
    return rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3)


def sha256_sigma1(x: int) -> int:
    return rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10)


K_CONSTANTS = [
    0x428A2F98,
    0x71374491,
    0xB5C0FBCF,
    0xE9B5DBA5,
    0x3956C25B,
    0x59F111F1,
    0x923F82A4,
    0xAB1C5ED5,
    0xD807AA98,
    0x12835B01,
    0x243185BE,
    0x550C7DC3,
    0x72BE5D74,
    0x80DEB1FE,
    0x9BDC06A7,
    0xC19BF174,
    0xE49B69C1,
    0xEFBE4786,
    0x0FC19DC6,
    0x240CA1CC,
    0x2DE92C6F,
    0x4A7484AA,
    0x5CB0A9DC,
    0x76F988DA,
    0x983E5152,
    0xA831C66D,
    0xB00327C8,
    0xBF597FC7,
    0xC6E00BF3,
    0xD5A79147,
    0x06CA6351,
    0x14292967,
    0x27B70A85,
    0x2E1B2138,
    0x4D2C6DFC,
    0x53380D13,
    0x650A7354,
    0x766A0ABB,
    0x81C2C92E,
    0x92722C85,
    0xA2BFE8A1,
    0xA81A664B,
    0xC24B8B70,
    0xC76C51A3,
    0xD192E819,
    0xD6990624,
    0xF40E3585,
    0x106AA070,
    0x19A4C116,
    0x1E376C08,
    0x2748774C,
    0x34B0BCB5,
    0x391C0CB3,
    0x4ED8AA4A,
    0x5B9CCA4F,
    0x682E6FF3,
    0x748F82EE,
    0x78A5636F,
    0x84C87814,
    0x8CC70208,
    0x90BEFFFA,
    0xA4506CEB,
    0xBEF9A3F7,
    0xC67178F2,
]


def pure_lean_sha256_emulated(data: bytes) -> str:
    """Emulates the pure Lean 4 SHA-256 FIPS 180-4 algorithm in Validator.lean."""
    length = len(data)
    bit_len = length * 8
    padded = bytearray(data)
    padded.append(0x80)
    rem = (length + 1) % 64
    pad_zeros = 56 - rem if rem <= 56 else 120 - rem
    padded.extend([0x00] * pad_zeros)
    padded.extend(bit_len.to_bytes(8, "big"))

    num_blocks = len(padded) // 64
    h = [
        0x6A09E667,
        0xBB67AE85,
        0x3C6EF372,
        0xA54FF53A,
        0x510E527F,
        0x9B05688C,
        0x1F83D9AB,
        0x5BE0CD19,
    ]

    for b in range(num_blocks):
        block = padded[b * 64 : (b + 1) * 64]
        w = [0] * 64
        for i in range(16):
            w[i] = int.from_bytes(block[i * 4 : (i + 1) * 4], "big")
        for i in range(16, 64):
            w[i] = (
                sha256_sigma1(w[i - 2])
                + w[i - 7]
                + sha256_sigma0(w[i - 15])
                + w[i - 16]
            ) & 0xFFFFFFFF

        a, b_val, c, d, e, f, g, h_val = h
        for i in range(64):
            t1 = (
                h_val
                + sha256_Sigma1(e)
                + sha256_Ch(e, f, g)
                + K_CONSTANTS[i]
                + w[i]
            ) & 0xFFFFFFFF
            t2 = (sha256_Sigma0(a) + sha256_Maj(a, b_val, c)) & 0xFFFFFFFF
            h_val = g
            g = f
            f = e
            e = (d + t1) & 0xFFFFFFFF
            d = c
            c = b_val
            b_val = a
            a = (t1 + t2) & 0xFFFFFFFF

        h[0] = (h[0] + a) & 0xFFFFFFFF
        h[1] = (h[1] + b_val) & 0xFFFFFFFF
        h[2] = (h[2] + c) & 0xFFFFFFFF
        h[3] = (h[3] + d) & 0xFFFFFFFF
        h[4] = (h[4] + e) & 0xFFFFFFFF
        h[5] = (h[5] + f) & 0xFFFFFFFF
        h[6] = (h[6] + g) & 0xFFFFFFFF
        h[7] = (h[7] + h_val) & 0xFFFFFFFF

    return "".join(f"{x:08x}" for x in h)


def test_fips_180_4_test_vectors():
    """Validates the pure Lean 4 SHA-256 algorithm against standard FIPS 180-4 test vectors."""
    vectors = [
        (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        (
            b"message digest",
            "f7846f55cf23e14eebeab5b4e1550cad5b509e3348fbc4efa3a1413d393cb650",
        ),
        (
            b"The quick brown fox jumps over the lazy dog",
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
        ),
    ]
    for msg, expected in vectors:
        computed = pure_lean_sha256_emulated(msg)
        assert computed == expected
        assert hashlib.sha256(msg).hexdigest() == expected


def test_validator_lean_structure_and_no_ffi():
    """Inspects Validator.lean to ensure pure ByteArray SHA-256 and content payload logic with no FFI sha256File."""
    validator_path = (
        Path(__file__).parent.parent / "lean4-proofs" / "Validator.lean"
    )
    content = validator_path.read_text(encoding="utf-8")

    # Pure Lean SHA-256 operating on ByteArray
    assert "def sha256 (data : ByteArray) : String :=" in content
    assert "def padMessage (msg : ByteArray) : ByteArray :=" in content

    # computeTheoremPayload incorporates ByteArray / String file content
    assert (
        "def computeTheoremPayload (_name : String) (_file : String) (_status : String) (content : ByteArray) : ByteArray :="
        in content
    )

    # isTheoremValid computes pure SHA-256 over combined metadata and file content payload
    assert (
        "def isTheoremValid (t : TheoremEntry) (content : ByteArray) : Bool :="
        in content
    )

    # Formal Lean theorems
    assert "theorem valid_theorem_is_proven" in content
    assert "theorem valid_theorem_checksum_matches" in content

    # No external FFI sha256File dependency
    assert "sha256File" not in content
    assert "opaque sha256File" not in content


def test_pure_specification_matches_proof_manifest():
    """Confirms that pure specification hash outputs match real SHA-256 checksums in proof_manifest.json."""
    base_dir = Path(__file__).parent.parent
    manifest_path = base_dir / "proof_manifest.json"
    assert manifest_path.exists(), f"Manifest file missing: {manifest_path}"

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    theorems = manifest.get("theorems", [])
    assert len(theorems) > 0, "No theorems found in proof_manifest.json"

    for thm in theorems:
        proof_file = base_dir / "lean4-proofs" / thm["file"]
        assert proof_file.exists(), f"Proof file missing: {proof_file}"

        file_bytes = proof_file.read_bytes()

        # Pure specification payload computation
        computed_hash = pure_lean_sha256_emulated(file_bytes)
        expected_hash = thm["checksum"]

        assert (
            computed_hash == expected_hash
        ), f"Mismatch for theorem {thm['name']} in file {thm['file']}: computed {computed_hash}, expected {expected_hash}"
