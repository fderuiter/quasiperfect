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
