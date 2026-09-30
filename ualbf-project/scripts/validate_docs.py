"""
UALBF Documentation Validation Suite
====================================

This script performs automated verification that:
1. All markdown documentation files in the repository are registered in `docs_manifest.json`.
2. All relative markdown file links and section anchor references across registered documentation are valid.
3. Tuning guide parameters match bounds and profile manifests.
4. Specification manifest updates (`bounds_manifest.json` and `schema_manifest.json`) are in sync with generated code and proof artifacts (when `--check-specs` is passed).
"""

import ast
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
        if not rel_path.endswith(".md"):
            continue
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
        if not rel_path.endswith(".md"):
            continue
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
    if os.path.exists(os.path.join(repo_root, "ualbf-project")):
        ualbf_project_dir = os.path.join(repo_root, "ualbf-project")
        monorepo_root = repo_root
    else:
        ualbf_project_dir = repo_root
        monorepo_root = (
            os.path.dirname(repo_root)
            if os.path.basename(repo_root) == "ualbf-project"
            else repo_root
        )

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
        "lean4-proofs/schema_generated.h",
        "lean4-proofs/include/verification_lib.h",
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
        if rel_path.startswith("../"):
            full_path = os.path.normpath(
                os.path.join(monorepo_root, rel_path.removeprefix("../"))
            )
        else:
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
        if rel_path.startswith("../"):
            full_path = os.path.normpath(
                os.path.join(monorepo_root, rel_path.removeprefix("../"))
            )
        else:
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
        toolchain_path = os.path.join(repo_root, "lean4-proofs", "lean-toolchain")
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
    if not os.path.exists(manifest_path):
        manifest_path = os.path.join(os.path.dirname(repo_root), "docs_manifest.json")

    if os.path.exists(manifest_path):
        monorepo_root = os.path.dirname(manifest_path)
        with open(manifest_path, "r", encoding="utf-8") as f:
            manifest = json.load(f)
        docs_to_check = [os.path.join(monorepo_root, p) for p in manifest.keys()]
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


STANDARD_ENV_VARS = {
    "PATH",
    "PYTHONPATH",
    "CPATH",
    "LIBRARY_PATH",
    "LD_LIBRARY_PATH",
    "DYLD_LIBRARY_PATH",
    "HOME",
    "USER",
    "SHELL",
    "TERM",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TMPDIR",
    "TEMP",
    "TMP",
    "GITHUB_ACTIONS",
    "GITHUB_REPOSITORY",
    "GITHUB_EVENT_PATH",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "GITHUB_REF",
    "GITHUB_SHA",
    "GITHUB_WORKFLOW",
    "GITHUB_RUN_ID",
    "GITHUB_WORKSPACE",
    "REPO",
    "ISSUE_NUMBER",
    "PR_NUMBER",
    "SSL_CERT_FILE",
    "GIT_SSL_CAINFO",
    "NIX_ARCH",
    "PYTHONUNBUFFERED",
    "PWD",
}


class EnvVarASTVisitor(ast.NodeVisitor):
    def __init__(self, filename: str):
        self.filename = filename
        self.env_vars: list[tuple[int, str]] = []  # (lineno, var_name)

    def visit_Subscript(self, node: ast.Subscript):
        if isinstance(node.value, ast.Attribute) and isinstance(
            node.value.value, ast.Name
        ):
            if node.value.value.id == "os" and node.value.attr == "environ":
                if isinstance(node.slice, ast.Constant) and isinstance(
                    node.slice.value, str
                ):
                    self.env_vars.append((node.lineno, node.slice.value))
        self.generic_visit(node)

    def visit_Call(self, node: ast.Call):
        func = node.func
        if isinstance(func, ast.Attribute):
            if (
                isinstance(func.value, ast.Name)
                and func.value.id == "os"
                and func.attr == "getenv"
            ):
                if (
                    node.args
                    and isinstance(node.args[0], ast.Constant)
                    and isinstance(node.args[0].value, str)
                ):
                    self.env_vars.append((node.lineno, node.args[0].value))
            elif isinstance(func.value, ast.Attribute) and isinstance(
                func.value.value, ast.Name
            ):
                if (
                    func.value.value.id == "os"
                    and func.value.attr == "environ"
                    and func.attr in ("get", "pop", "setdefault")
                ):
                    if (
                        node.args
                        and isinstance(node.args[0], ast.Constant)
                        and isinstance(node.args[0].value, str)
                    ):
                        self.env_vars.append((node.lineno, node.args[0].value))
            elif (
                isinstance(func.value, ast.Name)
                and func.value.id == "env_util"
                and func.attr in ("get_env_var", "require_env_var")
            ):
                if (
                    node.args
                    and isinstance(node.args[0], ast.Constant)
                    and isinstance(node.args[0].value, str)
                ):
                    self.env_vars.append((node.lineno, node.args[0].value))
        elif isinstance(func, ast.Name) and func.id in (
            "get_env_var",
            "require_env_var",
        ):
            if (
                node.args
                and isinstance(node.args[0], ast.Constant)
                and isinstance(node.args[0].value, str)
            ):
                self.env_vars.append((node.lineno, node.args[0].value))

        self.generic_visit(node)

    def visit_Compare(self, node: ast.Compare):
        if len(node.ops) == 1 and isinstance(node.ops[0], (ast.In, ast.NotIn)):
            if isinstance(node.left, ast.Constant) and isinstance(node.left.value, str):
                comp = node.comparators[0]
                if isinstance(comp, ast.Attribute) and isinstance(comp.value, ast.Name):
                    if comp.value.id == "os" and comp.attr == "environ":
                        self.env_vars.append((node.lineno, node.left.value))
        self.generic_visit(node)


def validate_env_manifest(repo_root: str) -> bool:
    """Validate structure of env_manifest.json against env_manifest.schema.json."""
    scripts_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(scripts_dir)
    if project_root not in sys.path:
        sys.path.insert(0, project_root)
    try:
        import env_util
    except ImportError:
        print("Error: Could not import env_util.", file=sys.stderr)
        return False

    manifest_path = os.path.join(repo_root, "env_manifest.json")
    schema_path = os.path.join(repo_root, "env_manifest.schema.json")

    if not os.path.exists(manifest_path):
        real_root = env_util.find_repo_root()
        manifest_path = os.path.join(real_root, "env_manifest.json")
        schema_path = os.path.join(real_root, "env_manifest.schema.json")
        if not os.path.exists(manifest_path):
            manifest_path = os.path.join(real_root, "ualbf-project", "env_manifest.json")
            schema_path = os.path.join(real_root, "ualbf-project", "env_manifest.schema.json")

    if not os.path.exists(manifest_path) or not os.path.exists(schema_path):
        return True

    try:
        env_util.load_manifest_and_schema(manifest_path, schema_path)
    except Exception as e:
        print(
            f"Error validating env_manifest.json against schema: {e}", file=sys.stderr
        )
        return False

    return True


def validate_env_vars(repo_root: str) -> bool:
    """Static analysis stage: Scan Python files for os.environ, os.getenv, and env_util calls to verify registration in env_manifest.json."""
    scripts_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(scripts_dir)
    if project_root not in sys.path:
        sys.path.insert(0, project_root)
    import env_util

    manifest_path = os.path.join(repo_root, "env_manifest.json")
    if not os.path.exists(manifest_path):
        real_root = env_util.find_repo_root()
        manifest_path = os.path.join(real_root, "env_manifest.json")
        if not os.path.exists(manifest_path):
            manifest_path = os.path.join(real_root, "ualbf-project", "env_manifest.json")

    if not os.path.exists(manifest_path):
        return True

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    registered_vars = set(manifest.keys())

    exclude_dirs = {
        "tests",
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
        ".git",
    }

    unregistered_findings = []

    for root_dir, dirs, files in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in exclude_dirs and not d.startswith(".")]

        for file in files:
            if not file.endswith(".py"):
                continue
            # Skip test files and env_util.py itself
            if (
                file.startswith("test_")
                or file.endswith("_test.py")
                or file in ("env_util.py",)
            ):
                continue

            rel_file = os.path.relpath(os.path.join(root_dir, file), repo_root)
            if any(
                part in exclude_dirs or part == "tests"
                for part in rel_file.split(os.sep)
            ):
                continue

            full_path = os.path.join(root_dir, file)
            try:
                with open(full_path, "r", encoding="utf-8") as pf:
                    code = pf.read()
                tree = ast.parse(code, filename=rel_file)
                visitor = EnvVarASTVisitor(rel_file)
                visitor.visit(tree)

                for line_no, var_name in visitor.env_vars:
                    if (
                        var_name not in registered_vars
                        and var_name not in STANDARD_ENV_VARS
                    ):
                        unregistered_findings.append((rel_file, line_no, var_name))
            except Exception:
                pass

    if unregistered_findings:
        print(
            "Error: Static AST scan detected unregistered environment variable references:",
            file=sys.stderr,
        )
        for rel_f, line_no, var_name in unregistered_findings:
            print(
                f"  - {rel_f}:{line_no}: Unregistered environment variable '{var_name}'",
                file=sys.stderr,
            )
        print(
            "\nRemedy: Register missing environment variables in env_manifest.json and document active ones in TCB.md / README.md.",
            file=sys.stderr,
        )
        return False

    return True


def validate_env_docs_alignment(repo_root: str) -> bool:
    """Verify that all active variables in env_manifest.json appear in TCB.md or README.md."""
    scripts_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(scripts_dir)
    if project_root not in sys.path:
        sys.path.insert(0, project_root)
    import env_util

    manifest_path = os.path.join(repo_root, "env_manifest.json")
    if not os.path.exists(manifest_path):
        real_root = env_util.find_repo_root()
        manifest_path = os.path.join(real_root, "env_manifest.json")
        if not os.path.exists(manifest_path):
            manifest_path = os.path.join(real_root, "ualbf-project", "env_manifest.json")

    if not os.path.exists(manifest_path):
        return True

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    active_vars = [k for k, v in manifest.items() if v.get("status") == "active"]

    real_root = env_util.find_repo_root()
    doc_paths = [
        os.path.join(repo_root, "README.md"),
        os.path.join(repo_root, "ualbf-project", "TCB.md"),
        os.path.join(repo_root, "TCB.md"),
        os.path.join(real_root, "README.md"),
        os.path.join(real_root, "ualbf-project", "TCB.md"),
        os.path.join(real_root, "TCB.md"),
    ]

    doc_contents = ""
    for dp in set(doc_paths):
        if os.path.exists(dp):
            with open(dp, "r", encoding="utf-8") as df:
                doc_contents += "\n" + df.read()

    missing_vars = [v for v in active_vars if v not in doc_contents]

    if missing_vars:
        print(
            "Error: Environment variable documentation alignment check failed!\n"
            "The following active environment variables are not documented in TCB.md or README.md:",
            file=sys.stderr,
        )
        for mv in missing_vars:
            print(f"  - {mv}", file=sys.stderr)
        return False

    return True


def validate_auditor_doc_checks(repo_root: str) -> bool:
    """
    Perform auditor documentation checks against proof_manifest.json, verifying backticked
    code symbols, unquoted static symbols, and Lean theorem proof statuses in authoritative documentation.
    """
    proof_manifest_path = os.path.join(repo_root, "proof_manifest.json")
    if not os.path.exists(proof_manifest_path):
        proof_manifest_path = os.path.join(
            repo_root, "ualbf-project", "proof_manifest.json"
        )

    if not os.path.exists(proof_manifest_path):
        print(
            f"Warning: proof_manifest.json not found at {proof_manifest_path}; skipping symbol and theorem verification.",
            file=sys.stderr,
        )
        return True

    try:
        with open(proof_manifest_path, "r", encoding="utf-8") as f:
            manifest = json.load(f)
    except Exception as e:
        print(
            f"Warning: Failed to load proof_manifest.json: {e}; skipping symbol and theorem verification.",
            file=sys.stderr,
        )
        return True

    scripts_dir = os.path.dirname(os.path.abspath(__file__))
    ualbf_project_dir = os.path.dirname(scripts_dir)
    if ualbf_project_dir not in sys.path:
        sys.path.insert(0, ualbf_project_dir)

    try:
        import auditor

        return auditor.check_documentation(manifest, repo_root=repo_root)
    except Exception as e:
        print(
            f"Error executing auditor documentation verification: {e}",
            file=sys.stderr,
        )
        return False


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
        repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
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

    # Find all .md files in the repository, whitelisting .jules/skills/ and .agents/skills/ hidden directories
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
    allowed_dot_prefixes = (".jules/skills/", ".agents/skills/")

    filtered_md_files = []
    for root_dir, dirs, files in os.walk(repo_root):
        rel_root = os.path.relpath(root_dir, repo_root)
        norm_rel_root = "" if rel_root == "." else rel_root.replace("\\", "/")

        pruned_dirs = []
        for d in dirs:
            if d in exclude_exact or d.startswith("result") or d.startswith("lake-"):
                continue
            if d.startswith("."):
                candidate_rel = f"{norm_rel_root}/{d}" if norm_rel_root else d
                if (
                    candidate_rel in (".jules", ".agents", ".jules/skills", ".agents/skills")
                    or candidate_rel.startswith(allowed_dot_prefixes)
                ):
                    pruned_dirs.append(d)
            else:
                pruned_dirs.append(d)
        dirs[:] = pruned_dirs

        for f in files:
            if f.endswith(".md"):
                rel_path = os.path.relpath(os.path.join(root_dir, f), repo_root).replace("\\", "/")
                filtered_md_files.append(rel_path)

    # Check that all registered manifest entries exist on disk
    missing_registered = []
    for rel_path in manifest.keys():
        if not os.path.exists(os.path.join(repo_root, rel_path)):
            missing_registered.append(rel_path)

    if missing_registered:
        print(
            "Error: The following registered files in docs_manifest.json do not exist on disk:",
            file=sys.stderr,
        )
        for f in missing_registered:
            print(f"  - {f}", file=sys.stderr)
        sys.exit(1)

    # Check if all .md files are registered in manifest
    unregistered = []
    for md_file in filtered_md_files:
        if md_file not in manifest and f"ualbf-project/{md_file}" not in manifest:
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

    # Check if all .tex files in paper/ are registered in manifest
    # Paths are taken relative to the repository root, like the manifest keys,
    # so the result does not depend on the directory the script runs from.
    all_tex_files = [
        os.path.relpath(p, repo_root)
        for p in glob.glob(os.path.join(repo_root, "**", "*.tex"), recursive=True)
    ]
    filtered_tex_files = []
    generated_tex_names = {"telemetry.tex", "verification_manifest.tex"}
    for tex_file in all_tex_files:
        parts = tex_file.split(os.sep)
        if not any(
            part.startswith(".")
            or part.startswith("_")
            or part.startswith("result")
            or part.startswith("lake-")
            or part in exclude_exact
            for part in parts
        ):
            if os.path.basename(tex_file) in generated_tex_names:
                continue
            filtered_tex_files.append(tex_file.replace("\\", "/"))

    unregistered_tex = []
    for tex_file in filtered_tex_files:
        if tex_file not in manifest:
            unregistered_tex.append(tex_file)

    if unregistered_tex:
        print(
            "Error: The following LaTeX paper files are not registered in docs_manifest.json:",
            file=sys.stderr,
        )
        for f in unregistered_tex:
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

    # Perform auditor documentation verification against proof_manifest.json
    if not validate_auditor_doc_checks(repo_root):
        print("Auditor documentation verification failed.", file=sys.stderr)
        sys.exit(1)

    # Run paper source validation
    scripts_dir = os.path.dirname(os.path.abspath(__file__))
    if scripts_dir not in sys.path:
        sys.path.insert(0, scripts_dir)
    import validate_paper

    paper_dir = os.path.join(repo_root, "ualbf-project", "paper")
    if os.path.exists(paper_dir):
        if not validate_paper.validate_paper_sources(paper_dir, repo_root):
            print("Error: Paper validation failed.", file=sys.stderr)
            sys.exit(1)

    # Run tuning guide parameter validation against bounds and profile manifests
    from validate_tuning_guide import validate_tuning_guide

    ualbf_project_dir = os.path.join(repo_root, "ualbf-project")
    if not os.path.exists(ualbf_project_dir):
        ualbf_project_dir = repo_root
    if not validate_tuning_guide(ualbf_project_dir):
        sys.exit(1)

    if not validate_toolchain_sync(repo_root):
        sys.exit(1)

    if not validate_env_manifest(repo_root):
        sys.exit(1)

    if not validate_env_vars(repo_root):
        sys.exit(1)

    if not validate_env_docs_alignment(repo_root):
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
            if f in manifest:
                if manifest[f] == "authoritative":
                    print(f"Authoritative document modified: {f}")
                    authoritative_touched = True
            elif f.endswith(".md") or f.endswith(".tex"):
                print(
                    f"Error: PR introduces a documentation file '{f}' not registered in docs_manifest.json.",
                    file=sys.stderr,
                )
                print(
                    "Please add it to docs_manifest.json with its authority level.",
                    file=sys.stderr,
                )
                sys.exit(1)

        if authoritative_touched:
            print("AUTHORITATIVE_TOUCHED=1")


if __name__ == "__main__":
    main()
