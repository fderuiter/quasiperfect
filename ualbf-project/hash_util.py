import hashlib
import os
from pathlib import Path
from typing import Optional, Union


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


def _get_max_file_size_bytes() -> int:
    env_val = os.getenv("UALBF_MAX_CERT_SIZE_MB")
    if env_val:
        try:
            val = float(env_val.strip())
            if val > 0:
                return int(val * 1024 * 1024)
        except (ValueError, TypeError):
            pass
    return int(10.0 * 1024 * 1024)


class CertificateError(Exception):
    """Base class for certificate-related errors."""

    pass


class CertificateValidationError(CertificateError, ValueError):
    """Raised when file size limits or hash validation bounds are exceeded."""

    pass


def hash_file_bounded(
    filepath: Union[str, Path],
    max_bytes: Optional[int] = None,
    chunk_size: int = 65536,
) -> str:
    """Computes SHA-256 hex digest of a file after enforcing configurable file size bounds."""
    path = Path(filepath)
    if not path.is_file():
        raise FileNotFoundError(f"File not found or is not a regular file: {filepath}")

    if max_bytes is None:
        max_bytes = _get_max_file_size_bytes()

    file_size = path.stat().st_size
    if file_size > max_bytes:
        actual_mb = file_size / (1024 * 1024)
        max_mb = max_bytes / (1024 * 1024)
        raise CertificateValidationError(
            f"File size of '{filepath}' ({actual_mb:.2f} MB / {file_size} bytes) "
            f"exceeds maximum allowed limit of {max_mb:.2f} MB ({max_bytes} bytes)."
        )

    return hash_file(path, chunk_size=chunk_size)


def raw_digest(data: bytes) -> bytes:
    """Computes the raw binary 32-byte SHA-256 digest of bytes data."""
    return hashlib.sha256(data).digest()


def hash_theorem_metadata(name: str, rel_file: str, status: str) -> str:
    """Computes the SHA-256 hex digest of formatted theorem metadata payload 'name|rel_file|status'."""
    payload = f"{name}|{rel_file}|{status}"
    return hash_string(payload, encoding="utf-8")
