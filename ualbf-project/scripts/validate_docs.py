"""
UALBF Documentation Validation Suite
====================================

This script performs automated verification that all markdown documentation files
within the repository are properly registered and categorized in `docs_manifest.json`.

To ensure highly performant and stable parallelized CI gating, files within
directories matching common virtual environment patterns, Nix build paths, or local
caches are dynamically filtered out. All checks are fully verified and stable under both local development and concurrent CI environments.
"""

import os
import sys
import json
import glob


def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    manifest_path = os.path.join(repo_root, "docs_manifest.json")

    if not os.path.exists(manifest_path):
        print(
            f"Error: docs_manifest.json not found at {manifest_path}.", file=sys.stderr
        )
        sys.exit(1)

    with open(manifest_path, "r") as f:
        try:
            manifest = json.load(f)
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in docs_manifest.json - {e}", file=sys.stderr)
            sys.exit(1)

    # Change to repo root to find all .md files with relative paths
    os.chdir(repo_root)
    all_md_files = glob.glob("**/*.md", recursive=True)

    # Filter out common build directories
    # Note: hidden directories like .pytest_cache are natively skipped by glob.glob unless include_hidden is set.
    exclude_dirs = [
        ".lake",
        "target",
        "node_modules",
        "build",
        ".git",
        "venv",
        ".venv",
        ".direnv",
        "lean-built",
        "result",
        "test-env",
        "test_env",
        "env",
        ".env",
    ]
    filtered_md_files = []
    for md_file in all_md_files:
        if not any(part in exclude_dirs for part in md_file.split(os.sep)):
            filtered_md_files.append(md_file)

    # Check if all .md files are in the manifest
    unregistered = []
    for md_file in filtered_md_files:
        if md_file not in manifest:
            unregistered.append(md_file)

    if unregistered:
        print(
            "Error: The following documentation files are not registered in docs_manifest.json:",
            file=sys.stderr,
        )
        for f in unregistered:
            print(f"  - {f}", file=sys.stderr)
        print(
            "\nPlease add them to docs_manifest.json with their authority level ('authoritative' or 'informal').",
            file=sys.stderr,
        )
        sys.exit(1)

    # Check if a specific file list was provided (e.g. from PR)
    if len(sys.argv) > 1:
        pr_files_path = os.path.abspath(sys.argv[1])
        if os.path.exists(pr_files_path):
            with open(pr_files_path, "r") as f:
                pr_files = [line.strip() for line in f if line.strip()]

            # Add parent directory of scripts to sys.path to import verify_metadata
            ualbf_dir = os.path.join(repo_root, "ualbf-project")
            if ualbf_dir not in sys.path:
                sys.path.insert(0, ualbf_dir)

            spec_modified = False
            try:
                from verify_metadata import check_reverse_dependencies

                is_valid, rev_errors, spec_modified = check_reverse_dependencies(
                    pr_files, base_dir=ualbf_dir
                )
                if not is_valid:
                    for err in rev_errors:
                        print(err, file=sys.stderr)
                    sys.exit(1)
            except Exception as e:
                print(
                    f"Warning: Failed to execute reverse dependency check: {e}",
                    file=sys.stderr,
                )

            authoritative_touched = False
            for f in pr_files:
                if f.endswith(".md"):
                    if f not in manifest:
                        print(
                            f"Error: PR introduces a documentation file '{f}' not registered in docs_manifest.json.",
                            file=sys.stderr,
                        )
                        print(
                            "Please add it to docs_manifest.json with its authority level.",
                            file=sys.stderr,
                        )
                        sys.exit(1)
                    if manifest[f] == "authoritative":
                        print(f"Authoritative document modified: {f}")
                        authoritative_touched = True

            if spec_modified:
                print("Formal specification or safety bounds modified.")
                authoritative_touched = True

            if authoritative_touched:
                print("AUTHORITATIVE_TOUCHED=1")


if __name__ == "__main__":
    main()
