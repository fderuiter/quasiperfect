import os
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


def test_unlisted_tag_missing_sha256_fails(tmp_path):
    manifest_file = tmp_path / "lake-manifest.json"
    manifest_data = {
        "packages": [
            {
                "name": "proofwidgets",
                "inputRev": "v0.0.100",
                "rev": "a84b3e2475d5c5ab979567b1ad8aea21b764bcf8",
            }
        ]
    }
    manifest_file.write_text(json.dumps(manifest_data))

    env = os.environ.copy()
    env["OFFLINE"] = "0"
    env["LAKE_OFFLINE"] = "0"
    env.pop("PROOFWIDGETS_SHA256", None)

    res = subprocess.run(
        ["bash", str(SCRIPT_PATH), str(manifest_file)],
        cwd=str(tmp_path),
        env=env,
        capture_output=True,
        text=True,
    )

    assert res.returncode == 1
    assert "Missing expected SHA256 checksum" in res.stderr
    assert "v0.0.100" in res.stderr


def test_sha256_checksum_mismatch_fails(tmp_path):
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

    # Setup mock curl
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    tar_content_dir = tmp_path / "tar_src"
    tar_content_dir.mkdir()
    (tar_content_dir / "index.js").write_text("// test bundle\nexport default {};\n")

    tar_path = tmp_path / "dummy.tar.gz"
    subprocess.run(
        ["tar", "-czf", str(tar_path), "-C", str(tar_content_dir), "index.js"],
        check=True,
    )

    curl_mock = bin_dir / "curl"
    curl_mock.write_text(f"""#!/usr/bin/env bash
for arg in "$@"; do
    if [[ "$arg" == *.tar.gz ]]; then
        cp "{tar_path}" "$arg"
        exit 0
    fi
done
exit 0
""")
    curl_mock.chmod(0o755)

    env = os.environ.copy()
    env["OFFLINE"] = "0"
    env["PATH"] = f"{bin_dir}:{env.get('PATH', '')}"
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

    assert res.returncode == 1
    assert "checksum mismatch" in res.stderr.lower()
    assert (
        "0000000000000000000000000000000000000000000000000000000000000000" in res.stderr
    )


def test_sha256_checksum_match_succeeds(tmp_path):
    import hashlib

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

    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    tar_content_dir = tmp_path / "tar_src"
    tar_content_dir.mkdir()
    (tar_content_dir / "index.js").write_text("// test bundle\nexport default {};\n")

    tar_path = tmp_path / "dummy.tar.gz"
    subprocess.run(
        ["tar", "-czf", str(tar_path), "-C", str(tar_content_dir), "index.js"],
        check=True,
    )
    actual_hash = hashlib.sha256(tar_path.read_bytes()).hexdigest()

    curl_mock = bin_dir / "curl"
    curl_mock.write_text(f"""#!/usr/bin/env bash
for arg in "$@"; do
    if [[ "$arg" == *.tar.gz ]]; then
        cp "{tar_path}" "$arg"
        exit 0
    fi
done
exit 0
""")
    curl_mock.chmod(0o755)

    env = os.environ.copy()
    env["OFFLINE"] = "0"
    env["PATH"] = f"{bin_dir}:{env.get('PATH', '')}"
    env["PROOFWIDGETS_SHA256"] = actual_hash

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
