import hashlib
from pathlib import Path
from typing import Union


def hash_bytes(data: bytes) -> str:
    """Computes the SHA-256 hex digest of raw bytes data."""
    return hashlib.sha256(data).hexdigest()


def hash_string(text: str, encoding: str = "utf-8") -> str:
    """Computes the SHA-256 hex digest of a string using the specified encoding (defaults to UTF-8)."""
    return hashlib.sha256(text.encode(encoding)).hexdigest()


def hash_file(filepath: Union[str, Path], chunk_size: int = 65536) -> str:
    """Computes the SHA-256 hex digest of a file using chunked streaming (defaults to 64KB chunks)."""
    path = Path(filepath)
    if not path.is_file():
        raise FileNotFoundError(f"File not found or is not a regular file: {filepath}")

    hasher = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(chunk_size):
            hasher.update(chunk)
    return hasher.hexdigest()


def raw_digest(data: bytes) -> bytes:
    """Computes the raw binary 32-byte SHA-256 digest of bytes data."""
    return hashlib.sha256(data).digest()


def hash_theorem_metadata(name: str, rel_file: str, status: str) -> str:
    """Computes the SHA-256 hex digest of formatted theorem metadata payload 'name|rel_file|status'."""
    payload = f"{name}|{rel_file}|{status}"
    return hash_string(payload, encoding="utf-8")
