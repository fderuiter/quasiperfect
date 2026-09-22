import hashlib
import os
import pytest

import hash_util


def test_hash_bytes():
    data = b"hello world"
    expected = hashlib.sha256(data).hexdigest()
    assert hash_util.hash_bytes(data) == expected


def test_hash_string():
    text = "hello world python"
    expected = hashlib.sha256(text.encode("utf-8")).hexdigest()
    assert hash_util.hash_string(text) == expected
    assert hash_util.hash_string(text, encoding="utf-8") == expected


def test_raw_digest():
    data = b"binary data"
    expected = hashlib.sha256(data).digest()
    assert hash_util.raw_digest(data) == expected
    assert len(hash_util.raw_digest(data)) == 32


def test_hash_theorem_metadata():
    name = "UALBF.FFI.rust_is_prime_sound"
    rel_file = "UALBF/FFI.lean"
    status = "proven"
    expected_payload = f"{name}|{rel_file}|{status}"
    expected_hash = hashlib.sha256(expected_payload.encode("utf-8")).hexdigest()

    assert hash_util.hash_theorem_metadata(name, rel_file, status) == expected_hash


def test_hash_file_empty(tmp_path):
    empty_file = tmp_path / "empty.txt"
    empty_file.write_bytes(b"")

    expected = hashlib.sha256(b"").hexdigest()
    assert hash_util.hash_file(empty_file) == expected


def test_hash_file_chunked_streaming(tmp_path):
    # Create a 250KB file with pseudo-random content to span multiple 64KB chunks
    content = os.urandom(250 * 1024)
    file_path = tmp_path / "large_file.bin"
    file_path.write_bytes(content)

    expected = hashlib.sha256(content).hexdigest()

    # Test default 64KB chunking
    assert hash_util.hash_file(file_path) == expected
    # Test str and Path inputs
    assert hash_util.hash_file(str(file_path)) == expected
    # Test custom chunk sizes (e.g. 1024, 65536, 100000)
    assert hash_util.hash_file(file_path, chunk_size=1024) == expected
    assert hash_util.hash_file(file_path, chunk_size=65536) == expected
    assert hash_util.hash_file(file_path, chunk_size=100000) == expected


def test_hash_file_non_existent(tmp_path):
    non_existent = tmp_path / "does_not_exist.bin"
    with pytest.raises(FileNotFoundError):
        hash_util.hash_file(non_existent)
