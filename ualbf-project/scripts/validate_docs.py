"""
UALBF Documentation Validation Suite
====================================

This script performs automated verification that:
1. All markdown documentation files in the repository are registered in `docs_manifest.json`.
2. All relative markdown file links and section anchor references across registered documentation are valid.
3. Tuning guide parameters match bounds and profile manifests.
4. Specification manifest updates (`bounds_manifest.json` and `schema_manifest.json`) are in sync with generated code and proof artifacts (when `--check-specs` is passed).
"""

import os
import sys
import json
import glob
import re
import subprocess


def slugify(text: str) -> str:
    """Convert heading text to a markdown anchor slug following GitHub conventions."""
    # Remove HTML tags if present
    text = re.sub(r"<[^>]+>", "", text)
    # Convert to lowercase
    text = text.lower()
    # Strip punctuation except spaces and hyphens
    text = re.sub(r"[^\w\s-]", "", text)
    # Replace spaces / whitespace with hyphens
    text = re.sub(r"[\s]+", "-", text)
    return text.strip("-")


def extract_anchors(content: str) -> set:
    """Extract all valid section anchor slugs and explicit HTML anchors from markdown content."""
    anchors = set()
    in_code_block = False

    lines = content.splitlines()
    for line in lines:
        line_stripped = line.strip()
        if line_stripped.startswith("```"):
            in_code_block = not in_code_block
            continue
        if in_code_block:
            continue

        # Match ATX headings: # Heading
        m = re.match(r"^(#{1,6})\s+(.+)$", line_stripped)
        if m:
            heading_text = m.group(2).strip()
            # Strip trailing #s if any
            heading_text = re.sub(r"\s+#+$", "", heading_text)
            slug = slugify(heading_text)
            if slug:
                anchors.add(slug)
                # Also add single-hyphen collapsed slug for resilience
                anchors.add(re.sub(r"-+", "-", slug))

        # Match explicit HTML anchor names or ids: <a name="foo"> or <a id="foo">
        html_anchors = re.findall(
            r'<(?:a|span|div)[^>]*(?:name|id)=["\']([^"\'\s>]+)["\']',
            line,
            re.IGNORECASE,
        )
        for ha in html_anchors:
            anchors.add(ha)
            anchors.add(slugify(ha))

    return anchors


def extract_links(content: str) -> list:
    """
    Extract markdown links [text](target) and image links ![alt](target) outside fenced code blocks.
    Returns list of tuples: (line_no, link_text, target_url)
    """
    links = []
    in_code_block = False

    lines = content.splitlines()
    for idx, line in enumerate(lines, start=1):
        line_stripped = line.strip()
        if line_stripped.startswith("```"):
            in_code_block = not in_code_block
            continue
        if in_code_block:
            continue

        # Regex for markdown links: [text](url) and ![alt](url)
        matches = re.findall(r"!?\[([^\]]*)\]\(([^)]+)\)", line)
        for text, url in matches:
            links.append((idx, text, url.strip()))

    return links


def validate_markdown_links(repo_root: str, registered_files: list) -> bool:
    """Validate all relative links and section anchor references across registered markdown files."""
    file_anchors = {}
    valid = True

    # First pass: collect anchors from all registered markdown files
    for rel_path in registered_files:
        abs_path = os.path.join(repo_root, rel_path)
        if os.path.exists(abs_path):
            try:
                with open(abs_path, "r", encoding="utf-8") as f:
                    content = f.read()
                file_anchors[rel_path] = extract_anchors(content)
            except Exception as e:
                print(f"Error reading {rel_path}: {e}", file=sys.stderr)

    # Second pass: validate links and section anchors
    for rel_path in registered_files:
        abs_path = os.path.join(repo_root, rel_path)
        if not os.path.exists(abs_path):
            continue

        try:
            with open(abs_path, "r", encoding="utf-8") as f:
                content = f.read()
        except Exception:
            continue

        links = extract_links(content)
        dir_path = os.path.dirname(abs_path)

        for line_no, text, url in links:
            # Skip external links and email addresses
            if url.startswith(("http://", "https://", "mailto:", "ftp://", "tel:")):
                continue

            # Separate file path and anchor
            if "#" in url:
                target_path, anchor = url.split("#", 1)
            else:
                target_path, anchor = url, None

            # Remove query parameters if present
            if "?" in target_path:
                target_path = target_path.split("?", 1)[0]

            # If target_path is empty, it refers to an anchor in the current file
            if not target_path:
                target_abs = abs_path
                target_rel = rel_path
            else:
                target_abs = os.path.normpath(os.path.join(dir_path, target_path))
                target_rel = os.path.relpath(target_abs, repo_root)

            # Check if target file exists
            if not os.path.exists(target_abs):
                print(
                    f"Error: Broken relative link in '{rel_path}:{line_no}':\n"
                    f"  Link text: '{text}' -> Target path '{target_path}' not found (resolved to '{target_rel}').",
                    file=sys.stderr,
                )
                valid = False
                continue

            # If anchor is specified, check if anchor exists in target file
            if anchor:
                # Skip line number anchors (e.g. #L100 or #L100-L200) or non-markdown file anchors
                if re.match(
                    r"^L\d+(?:-L\d+)?$", anchor, re.IGNORECASE
                ) or not target_abs.endswith(".md"):
                    continue

                # Get target file anchors
                if target_rel in file_anchors:
                    anchors = file_anchors[target_rel]
                elif target_abs.endswith(".md"):
                    try:
                        with open(target_abs, "r", encoding="utf-8") as tf:
                            anchors = extract_anchors(tf.read())
                            file_anchors[target_rel] = anchors
                    except Exception:
                        anchors = set()
                else:
                    anchors = set()

                norm_anchor = slugify(anchor)
                collapsed_anchor = re.sub(r"-+", "-", norm_anchor)

                if (
                    norm_anchor not in anchors
                    and collapsed_anchor not in anchors
                    and anchor not in anchors
                ):
                    print(
                        f"Error: Broken section anchor in '{rel_path}:{line_no}':\n"
                        f"  Link text: '{text}' -> Section anchor '#{anchor}' not found in '{target_rel}'.",
                        file=sys.stderr,
                    )
                    valid = False

    return valid


def validate_spec_sync(repo_root: str) -> bool:
    """Verify that generated specification artifacts match schema_manifest.json and bounds_manifest.json."""
    ualbf_project_dir = os.path.join(repo_root, "ualbf-project")
    spec_export_script = os.path.join(
        ualbf_project_dir, "scripts", "export_lean_specs.py"
    )

    if not os.path.exists(spec_export_script):
        print(
            f"Error: export_lean_specs.py not found at {spec_export_script}.",
            file=sys.stderr,
        )
        return False

    spec_files = [
        "rust-engine/src/schema_generated.rs",
        "lean4-proofs/UALBF/Engine/SearchState.lean",
        "rust-engine/src/lean_export.rs",
        "lean4-proofs/UALBF/FFI_generated.lean",
        "rust-engine/src/ffi_generated.rs",
        "rust-engine/src/manifest_constants.rs",
        "rust-engine/src/manifest_constants.h",
        "lean4-proofs/UALBF/ManifestConstants.lean",
        "../README.md",
        "TODO.md",
    ]

    # Read original contents
    original_contents = {}
    for rel_path in spec_files:
        full_path = os.path.join(ualbf_project_dir, rel_path)
        if os.path.exists(full_path):
            with open(full_path, "r", encoding="utf-8") as f:
                original_contents[rel_path] = f.read()

    # Run spec export
    res = subprocess.run(
        [sys.executable, spec_export_script],
        cwd=ualbf_project_dir,
        capture_output=True,
        text=True,
    )

    if res.returncode != 0:
        print(f"Error executing export_lean_specs.py:\n{res.stderr}", file=sys.stderr)
        return False

    # Read newly generated contents and restore original files to preserve workspace state
    new_contents = {}
    mismatched = []

    for rel_path in spec_files:
        full_path = os.path.join(ualbf_project_dir, rel_path)
        if os.path.exists(full_path):
            with open(full_path, "r", encoding="utf-8") as f:
                new_contents[rel_path] = f.read()

        # Restore original file content
        if rel_path in original_contents:
            with open(full_path, "w", encoding="utf-8") as f:
                f.write(original_contents[rel_path])

        if original_contents.get(rel_path) != new_contents.get(rel_path):
            mismatched.append(rel_path)

    if mismatched:
        print(
            "Error: Specification synchronization check failed!\n"
            "The following generated specification files are out of sync with bounds_manifest.json / schema_manifest.json:",
            file=sys.stderr,
        )
        for m in mismatched:
            print(f"  - ualbf-project/{m}", file=sys.stderr)
        print(
            "\nRemedy: Run 'make verify-sync' or 'python3 scripts/export_lean_specs.py' to update generated specification artifacts.",
            file=sys.stderr,
        )
        return False

    return True


def validate_toolchain_sync(repo_root: str) -> bool:
    """Verify that marked documentation sections match the lean-toolchain manifest file."""
    toolchain_path = os.path.join(
        repo_root, "ualbf-project", "lean4-proofs", "lean-toolchain"
    )
    if not os.path.exists(toolchain_path):
        return True

    with open(toolchain_path, "r", encoding="utf-8") as f:
        raw_toolchain = f.read().strip()

    if ":" in raw_toolchain:
        env_str = raw_toolchain
        version_str = raw_toolchain.split(":")[-1]
    else:
        version_str = raw_toolchain
        env_str = f"leanprover/lean4:{version_str}"

    manifest_path = os.path.join(repo_root, "docs_manifest.json")
    if os.path.exists(manifest_path):
        with open(manifest_path, "r", encoding="utf-8") as f:
            manifest = json.load(f)
        docs_to_check = [os.path.join(repo_root, p) for p in manifest.keys()]
    else:
        docs_to_check = [
            os.path.join(repo_root, "README.md"),
            os.path.join(repo_root, "ualbf-project", "TODO.md"),
        ]

    version_pattern = re.compile(
        r"<!--\s*(?:TOOLCHAIN_VERSION|LEAN_TOOLCHAIN)_START\s*-->(.*?)<!--\s*(?:TOOLCHAIN_VERSION|LEAN_TOOLCHAIN)_END\s*-->",
        re.DOTALL,
    )
    env_pattern = re.compile(
        r"<!--\s*(?:TOOLCHAIN_ENV|LEAN_TOOLCHAIN_ENV)_START\s*-->(.*?)<!--\s*(?:TOOLCHAIN_ENV|LEAN_TOOLCHAIN_ENV)_END\s*-->",
        re.DOTALL,
    )

    mismatches = []
    for doc_path in docs_to_check:
        if not os.path.exists(doc_path):
            continue
        try:
            with open(doc_path, "r", encoding="utf-8") as f:
                content = f.read()
        except Exception:
            continue

        for match in version_pattern.finditer(content):
            found_version = match.group(1)
            if found_version != version_str:
                rel_p = os.path.relpath(doc_path, repo_root)
                mismatches.append(
                    f"{rel_p}: expected version '{version_str}', found '{found_version}'"
                )

        for match in env_pattern.finditer(content):
            found_env = match.group(1)
            if found_env != env_str:
                rel_p = os.path.relpath(doc_path, repo_root)
                mismatches.append(
                    f"{rel_p}: expected environment '{env_str}', found '{found_env}'"
                )

    if mismatches:
        print(
            "Error: Toolchain documentation synchronization check failed!\n"
            "The following marked documentation sections do not match lean-toolchain:",
            file=sys.stderr,
        )
        for m in mismatches:
            print(f"  - {m}", file=sys.stderr)
        print(
            "\nRemedy: Run 'make verify-sync' or 'python3 ualbf-project/scripts/export_lean_specs.py' to update documentation.",
            file=sys.stderr,
        )
        return False

    return True


def main():
    args = sys.argv[1:]
    check_specs = False
    pr_files_path = None

    filtered_args = []
    for arg in args:
        if arg == "--check-specs":
            check_specs = True
        else:
            filtered_args.append(arg)

    if filtered_args:
        pr_files_path = os.path.abspath(filtered_args[0])

    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    manifest_path = os.path.join(repo_root, "docs_manifest.json")

    if not os.path.exists(manifest_path):
        print(
            f"Error: docs_manifest.json not found at {manifest_path}.", file=sys.stderr
        )
        sys.exit(1)

    with open(manifest_path, "r", encoding="utf-8") as f:
        try:
            manifest = json.load(f)
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in docs_manifest.json - {e}", file=sys.stderr)
            sys.exit(1)

    # Change to repo root to find all .md files with relative paths
    os.chdir(repo_root)
    all_md_files = glob.glob("**/*.md", recursive=True)

    # Filter out common build, hidden, and virtual environment directories
    exclude_exact = {
        "target",
        "node_modules",
        "build",
        "dist",
        "lean-built",
        "test-env",
        "test_env",
        "env",
        "venv",
        "virtualenv",
        "virtualenvs",
        "lake-packages",
        "lake-manifest",
        "site-packages",
    }
    filtered_md_files = []
    for md_file in all_md_files:
        parts = md_file.split(os.sep)
        if not any(
            part.startswith(".")
            or part.startswith("result")
            or part.startswith("lake-")
            or part in exclude_exact
            for part in parts
        ):
            filtered_md_files.append(md_file)

    # Check if all .md files are registered in manifest
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

    # Validate relative links and section anchors across registered documentation
    registered_files = list(manifest.keys())
    if not validate_markdown_links(repo_root, registered_files):
        print("Documentation link/anchor validation failed.", file=sys.stderr)
        sys.exit(1)

    # Run tuning guide parameter validation against bounds and profile manifests
    scripts_dir = os.path.dirname(os.path.abspath(__file__))
    if scripts_dir not in sys.path:
        sys.path.insert(0, scripts_dir)
    from validate_tuning_guide import validate_tuning_guide

    ualbf_project_dir = os.path.join(repo_root, "ualbf-project")
    if not validate_tuning_guide(ualbf_project_dir):
        sys.exit(1)

    if not validate_toolchain_sync(repo_root):
        sys.exit(1)

    # Check specification sync if requested or in default mode without PR file path
    if check_specs or pr_files_path is None:
        if not validate_spec_sync(repo_root):
            sys.exit(1)

    # Check if a specific file list was provided (e.g. from PR)
    if pr_files_path and os.path.exists(pr_files_path):
        with open(pr_files_path, "r", encoding="utf-8") as f:
            pr_files = [line.strip() for line in f if line.strip()]

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

        if authoritative_touched:
            print("AUTHORITATIVE_TOUCHED=1")


if __name__ == "__main__":
    main()
