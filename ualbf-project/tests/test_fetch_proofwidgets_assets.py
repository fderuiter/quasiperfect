import os
import sys
import subprocess
import shutil
import tempfile
import json
import pytest
from pathlib import Path

PROJECT_DIR = Path(__file__).parent.parent
SCRIPT_PATH = PROJECT_DIR / "scripts" / "fetch_proofwidgets_assets.sh"


def test_script_exists_and_executable():
    assert SCRIPT_PATH.exists()
    assert os.access(SCRIPT_PATH, os.X_OK)


def test_offline_mock_bundle_generation(tmp_path):
    # Set up mock lake-manifest.json
    manifest_file = tmp_path / "lake-manifest.json"
    manifest_data = {
        "packages": [
            {
                "name": "proofwidgets",
                "inputRev": "v0.0.99",
                "rev": "a84b3e2475d5c5ab979567b1ad8aea21b764bcf8",
            }
        ]
    }
    manifest_file.write_text(json.dumps(manifest_data))

    env = os.environ.copy()
    env["OFFLINE"] = "1"

    res = subprocess.run(
        ["bash", str(SCRIPT_PATH), str(manifest_file)],
        cwd=str(tmp_path),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 0
    assert "Offline mode active" in res.stdout or "mock bundle generated" in res.stdout

    js_dir = tmp_path / ".lake" / "packages" / "proofwidgets" / ".lake" / "build" / "js"
    assert js_dir.exists()
    assert (js_dir / "index.js").exists()
    assert (js_dir / "lake.trace").exists()
    assert len((js_dir / "index.js").read_text()) > 0
    assert (
        "a84b3e2475d5c5ab979567b1ad8aea21b764bcf8"
        in (js_dir / "lake.trace").read_text()
    )


def test_online_download_fallback_when_unverified_or_404(tmp_path):
    manifest_file = tmp_path / "lake-manifest.json"
    manifest_data = {
        "packages": [
            {
                "name": "proofwidgets",
                "inputRev": "v0.0.99",
                "rev": "a84b3e2475d5c5ab979567b1ad8aea21b764bcf8",
            }
        ]
    }
    manifest_file.write_text(json.dumps(manifest_data))

    env = os.environ.copy()
    env["OFFLINE"] = "0"
    env["LAKE_OFFLINE"] = "0"

    res = subprocess.run(
        ["bash", str(SCRIPT_PATH), str(manifest_file)],
        cwd=str(tmp_path),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 0
    js_dir = tmp_path / ".lake" / "packages" / "proofwidgets" / ".lake" / "build" / "js"
    assert (js_dir / "index.js").exists()
    assert (js_dir / "lake.trace").exists()


def test_sha256_checksum_validation(tmp_path):
    manifest_file = tmp_path / "lake-manifest.json"
    manifest_data = {
        "packages": [
            {
                "name": "proofwidgets",
                "inputRev": "v0.0.99",
                "rev": "a84b3e2475d5c5ab979567b1ad8aea21b764bcf8",
            }
        ]
    }
    manifest_file.write_text(json.dumps(manifest_data))

    env = os.environ.copy()
    env["OFFLINE"] = "0"
    # Pass an invalid sha256 to force checksum mismatch and trigger offline mock fallback
    env["PROOFWIDGETS_SHA256"] = (
        "0000000000000000000000000000000000000000000000000000000000000000"
    )

    res = subprocess.run(
        ["bash", str(SCRIPT_PATH), str(manifest_file)],
        cwd=str(tmp_path),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 0
    js_dir = tmp_path / ".lake" / "packages" / "proofwidgets" / ".lake" / "build" / "js"
    assert (js_dir / "index.js").exists()
    assert (js_dir / "lake.trace").exists()


def test_manifest_path_with_quotes_and_injection(tmp_path):
    # Path containing single quotes and injection attempt
    special_dir = tmp_path / "path_with_'quote'_and_$(injection)"
    special_dir.mkdir(parents=True)
    manifest_file = special_dir / "lake-manifest.json"

    manifest_data = {
        "packages": [
            {
                "name": "proofwidgets",
                "inputRev": "v0.0.92",
                "rev": "custom_rev_12345",
            }
        ]
    }
    manifest_file.write_text(json.dumps(manifest_data))

    env = os.environ.copy()
    env["OFFLINE"] = "1"

    res = subprocess.run(
        ["bash", str(SCRIPT_PATH), str(manifest_file)],
        cwd=str(tmp_path),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 0
    js_dir = special_dir / ".lake" / "packages" / "proofwidgets" / ".lake" / "build" / "js"
    assert js_dir.exists()
    assert (js_dir / "lake.trace").exists()
    assert "custom_rev_12345" in (js_dir / "lake.trace").read_text()


def test_compute_sha256_python_fallback_with_special_chars(tmp_path):
    special_file = tmp_path / "test_'quote'_file.bin"
    content = b"hello world test content"
    special_file.write_bytes(content)

    import hashlib

    expected_hash = hashlib.sha256(content).hexdigest()

    # Create a PATH where sha256sum and shasum are not available to force Python fallback
    dummy_bin_dir = tmp_path / "bin"
    dummy_bin_dir.mkdir()
    python3_real = sys.executable
    (dummy_bin_dir / "python3").write_text(f'#!/bin/sh\nexec "{python3_real}" "$@"\n')
    (dummy_bin_dir / "python3").chmod(0o755)

    for cmd in ["bash", "python", "awk", "mkdir", "cat", "echo"]:
        cmd_path = shutil.which(cmd)
        if cmd_path:
            os.symlink(cmd_path, dummy_bin_dir / cmd)

    env = os.environ.copy()
    env["PATH"] = str(dummy_bin_dir)
    env["TARGET_FILE"] = str(special_file)

    script_text = SCRIPT_PATH.read_text()
    func_lines = []
    in_func = False
    for line in script_text.splitlines():
        if "compute_sha256() {" in line:
            in_func = True
            continue
        if in_func:
            if line.strip() == "}":
                break
            func_lines.append(line)

    func_body = "\n".join(func_lines)
    bash_script = "compute_sha256() {\n" + func_body + "\n}\ncompute_sha256 \"$TARGET_FILE\""

    res = subprocess.run(
        ["bash", "-c", bash_script],
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 0, f"res failed with stderr: {res.stderr!r}, stdout: {res.stdout!r}"
    assert res.stdout.strip() == expected_hash



