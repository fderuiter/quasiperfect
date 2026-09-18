#!/usr/bin/env python3
"""
UALBF Tuning Guide Validator
============================

Validates parameter documentation in ualbf-project/TUNING.md against
bounds_manifest.json and rust-engine/profile.json.template.

Checks:
1. No duplicate parameter headers in TUNING.md.
2. All parameter headers in TUNING.md correspond to valid dot-notation paths in
   bounds_manifest.json or top-level keys in profile.json.template.
3. All tunable parameters in bounds_manifest.json and profile.json.template are
   documented in TUNING.md.
"""

import json
import os
import re
import sys
from pathlib import Path


def extract_bounds_leaf_paths(data, current_path=""):
    paths = set()
    if isinstance(data, dict):
        for k, v in data.items():
            sub_path = f"{current_path}.{k}" if current_path else k
            if k in (
                "justification",
                "description",
                "is_axiomatic",
                "conjecture_name",
            ) or "citation" in sub_path.split("."):
                continue
            paths.update(extract_bounds_leaf_paths(v, sub_path))
    elif isinstance(data, (list, int, float, bool, str)):
        if current_path:
            paths.add(current_path)
    return paths


def extract_tuning_parameter_headers(tuning_path):
    headers = []  # list of tuples: (key, line_number)
    if not os.path.exists(tuning_path):
        return headers

    with open(tuning_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    # Match bullet lines like: - **parameter_name**
    # Exclude indented sub-bullets like:   - **Description**: ...
    pattern = re.compile(r"^\s*-\s+\*\*([^*:]+)\*\*\s*$")

    for idx, line in enumerate(lines, start=1):
        m = pattern.match(line)
        if m:
            key = m.group(1).strip()
            headers.append((key, idx))

    return headers


def validate_tuning_guide(project_root=None):
    if project_root is None:
        project_root = Path(__file__).resolve().parent.parent
    else:
        project_root = Path(project_root)

    tuning_path = project_root / "TUNING.md"
    bounds_manifest_path = project_root / "bounds_manifest.json"
    profile_template_path = project_root / "rust-engine" / "profile.json.template"

    errors = []

    # Verify input files exist
    if not tuning_path.exists():
        errors.append(f"Tuning guide missing at {tuning_path}")
    if not bounds_manifest_path.exists():
        errors.append(f"Bounds manifest missing at {bounds_manifest_path}")
    if not profile_template_path.exists():
        errors.append(f"Profile template missing at {profile_template_path}")

    if errors:
        for err in errors:
            print(f"Error: {err}", file=sys.stderr)
        return False

    # Load manifests
    with open(bounds_manifest_path, "r", encoding="utf-8") as f:
        bounds_data = json.load(f)

    with open(profile_template_path, "r", encoding="utf-8") as f:
        profile_data = json.load(f)

    bounds_keys = extract_bounds_leaf_paths(bounds_data)
    profile_keys = set(profile_data.keys())
    valid_manifest_keys = bounds_keys.union(profile_keys)

    # Extract headers from TUNING.md
    headers = extract_tuning_parameter_headers(tuning_path)

    # 1. Check for duplicates
    seen_headers = {}
    for key, line_no in headers:
        if key in seen_headers:
            seen_headers[key].append(line_no)
        else:
            seen_headers[key] = [line_no]

    for key, line_nos in seen_headers.items():
        if len(line_nos) > 1:
            lines_str = ", ".join(map(str, line_nos))
            errors.append(
                f"Duplicate parameter header '{key}' found in TUNING.md on lines {lines_str}."
            )

    # 2. Check for unknown/mismatched parameter headers in TUNING.md
    documented_keys = set()
    for key, line_no in headers:
        documented_keys.add(key)
        if key not in valid_manifest_keys:
            errors.append(
                f"Error on TUNING.md:{line_no}: Parameter header '{key}' does not exist in bounds_manifest.json or profile.json.template."
            )

    # 3. Check for missing parameter documentations
    missing_from_docs = valid_manifest_keys - documented_keys
    if missing_from_docs:
        for missing_key in sorted(missing_from_docs):
            errors.append(
                f"Missing documentation: Parameter '{missing_key}' exists in manifest but is not documented in TUNING.md."
            )

    if errors:
        print("Tuning Guide Validation Failed:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return False

    print("Tuning Guide Validation Passed: All parameter headers aligned and verified.")
    return True


def main():
    success = validate_tuning_guide()
    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()
