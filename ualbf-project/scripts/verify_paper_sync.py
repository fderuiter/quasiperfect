#!/usr/bin/env python3
"""CLI verification tool for paper LaTeX macro synchronization and unit tests compliance.

Verifies:
  1. Unit tests in paper/test_ingest_cert.py pass.
  2. LaTeX macros in paper/telemetry.tex and table entries in paper/verification_manifest.tex
     match proof_manifest.json.
"""

import json
import os
import re
import sys
import tempfile
import unittest
from typing import Dict, Optional

script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(script_dir)
paper_dir = os.path.join(project_root, "paper")

if project_root not in sys.path:
    sys.path.insert(0, project_root)
if paper_dir not in sys.path:
    sys.path.insert(0, paper_dir)

import cert_util  # noqa: E402
import hash_util  # noqa: E402
import ingest_cert  # noqa: E402


def run_paper_unit_tests() -> bool:
    """Run unit tests in paper/test_ingest_cert.py."""
    print("=== Running Paper Ingest Unit Tests (paper/test_ingest_cert.py) ===")
    loader = unittest.TestLoader()
    suite = loader.discover(start_dir=paper_dir, pattern="test_ingest_cert.py")
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    return result.wasSuccessful()


def create_dummy_cert(manifest_path: str, bounds_path: str) -> dict:
    """Create minimal certificate dictionary aligned with manifest hash."""
    mbytes = cert_util.BoundedJSONLoader().read_file_bytes(manifest_path)
    mhash = hash_util.hash_bytes(mbytes)
    return {
        "manifest_hash": mhash,
        "verified_logic_hash": "0" * 64,
        "public_key": "0" * 32,
        "signature": "0" * 64,
        "telemetry": {
            "phase1_execution_time_ms": 100,
            "phase2_execution_time_ms": 5000,
            "total_branches_searched": 1000,
            "abundance_pruned": 200,
            "raycast_pruned": 0,
            "target_min_log10": 35,
            "target_max_log10": 37,
        },
    }


def parse_tex_macros(content: str) -> Dict[str, str]:
    """Parse \\newcommand{\\MacroName}{Value} entries from LaTeX content."""
    macros: Dict[str, str] = {}
    for line in content.splitlines():
        line = line.strip()
        m = re.match(r"\\newcommand\{\\([A-Za-z0-9_]+)\}\{(.+?)\}", line)
        if m:
            name, val = m.groups()
            if name in macros and macros[name] != val:
                macros[name] = f"CONFLICT:{macros[name]} vs {val}"
            else:
                macros[name] = val
    return macros


def generate_paper_macros(
    manifest_path: str, bounds_path: str, target_dir: str
) -> None:
    """Generate telemetry.tex and verification_manifest.tex in target_dir."""
    with tempfile.TemporaryDirectory() as tmp_cert_dir:
        dummy_cert_path = os.path.join(tmp_cert_dir, "dummy_cert.json")
        dummy_cert_data = create_dummy_cert(manifest_path, bounds_path)
        with open(dummy_cert_path, "w", encoding="utf-8") as f:
            json.dump(dummy_cert_data, f)

        orig_dummy = os.environ.get("UALBF_DUMMY_PAPER_CI")
        os.environ["UALBF_DUMMY_PAPER_CI"] = "1"
        try:
            ingest_cert.write_telemetry_tex(
                cert_path=dummy_cert_path,
                manifest_path=manifest_path,
                bounds_path=bounds_path,
                output_dir=target_dir,
            )
        finally:
            if orig_dummy is None:
                os.environ.pop("UALBF_DUMMY_PAPER_CI", None)
            else:
                os.environ["UALBF_DUMMY_PAPER_CI"] = orig_dummy


def verify_paper_macro_sync(
    manifest_path: Optional[str] = None,
    bounds_path: Optional[str] = None,
    paper_directory: Optional[str] = None,
) -> bool:
    """Assert telemetry.tex and verification_manifest.tex match proof_manifest.json."""
    print(
        "=== Verifying LaTeX Macro Synchronization against proof_manifest.json ==="
    )
    if manifest_path is None:
        manifest_path = os.path.join(project_root, "proof_manifest.json")
    if bounds_path is None:
        bounds_path = os.path.join(project_root, "bounds_manifest.json")
    if paper_directory is None:
        paper_directory = paper_dir

    if not os.path.exists(manifest_path):
        print(f"Error: proof_manifest.json not found at {manifest_path}")
        return False
    if not os.path.exists(bounds_path):
        print(f"Error: bounds_manifest.json not found at {bounds_path}")
        return False

    telemetry_path = os.path.join(paper_directory, "telemetry.tex")
    verification_path = os.path.join(paper_directory, "verification_manifest.tex")

    # Ensure TeX macro files exist on disk in paper directory
    if not os.path.exists(telemetry_path) or not os.path.exists(
        verification_path
    ):
        print("LaTeX macro files not found on disk. Generating paper TeX macros...")
        generate_paper_macros(manifest_path, bounds_path, paper_directory)

    # Read on-disk TeX macro files
    with open(telemetry_path, "r", encoding="utf-8") as f:
        on_disk_telemetry = f.read()

    with open(verification_path, "r", encoding="utf-8") as f:
        on_disk_verification = f.read()

    disk_macros = parse_tex_macros(on_disk_telemetry)
    manifest_data = cert_util.BoundedJSONLoader().load_file(manifest_path)
    mismatches = []

    # Verify theorems
    for thm in manifest_data.get("theorems", []):
        name = thm["name"]
        checksum = thm["checksum"]
        status = thm["status"]
        macro_name = ingest_cert.make_macro_name(name)
        status_macro_name = f"{macro_name}Status"
        table_entry = f"\\texttt{{\\{macro_name}}}"

        # Check checksum macro
        actual_checksum = disk_macros.get(macro_name)
        if actual_checksum is None:
            mismatches.append(
                f"Theorem '{name}' checksum macro '\\{macro_name}' missing from {telemetry_path}."
            )
        elif actual_checksum != checksum:
            mismatches.append(
                f"Theorem '{name}' checksum macro '\\{macro_name}' in {telemetry_path} is '{actual_checksum}', but expected '{checksum}' from {manifest_path}."
            )

        # Check status macro
        actual_status = disk_macros.get(status_macro_name)
        if actual_status is None:
            mismatches.append(
                f"Theorem '{name}' status macro '\\{status_macro_name}' missing from {telemetry_path}."
            )
        elif actual_status != status:
            mismatches.append(
                f"Theorem '{name}' status macro '\\{status_macro_name}' in {telemetry_path} is '{actual_status}', but expected '{status}' from {manifest_path}."
            )

        # Check verification table entry
        if table_entry not in on_disk_verification:
            mismatches.append(
                f"Theorem '{name}' macro '{table_entry}' missing from verification table in {verification_path}."
            )

    # Verify verus hashes
    verus_hashes = manifest_data.get("verus_hashes", {})
    for fn, expected_hash in verus_hashes.items():
        macro_name = ingest_cert.make_macro_name(fn)
        table_entry = f"\\texttt{{\\{macro_name}}}"

        actual_hash = disk_macros.get(macro_name)
        if actual_hash is None:
            mismatches.append(
                f"Verus function '{fn}' macro '\\{macro_name}' missing from {telemetry_path}."
            )
        elif actual_hash != expected_hash:
            mismatches.append(
                f"Verus function '{fn}' macro '\\{macro_name}' in {telemetry_path} is '{actual_hash}', but expected '{expected_hash}' from {manifest_path}."
            )

        if table_entry not in on_disk_verification:
            mismatches.append(
                f"Verus function '{fn}' macro '{table_entry}' missing from verification table in {verification_path}."
            )

    if mismatches:
        print("Error: LaTeX paper macros are out of sync with proof_manifest.json!")
        for error_msg in mismatches:
            print(f"  - {error_msg}")
        return False

    print("LaTeX paper macros are synchronized with proof_manifest.json.")
    return True


def main() -> None:
    """Main CLI entrypoint."""
    tests_ok = run_paper_unit_tests()
    if not tests_ok:
        print("\n[FAIL] Paper unit tests failed!")
        sys.exit(1)

    sync_ok = verify_paper_macro_sync()
    if not sync_ok:
        print("\n[FAIL] Paper LaTeX macro synchronization failed!")
        sys.exit(1)

    print("\n[PASS] All paper sync and compliance checks passed successfully.")
    sys.exit(0)


if __name__ == "__main__":
    main()
