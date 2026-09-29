#!/usr/bin/env python3
"""
Dedicated Paper Validation Submodule
=====================================
Validates LaTeX paper sources in ualbf-project/paper/:
  1. Checks \\input{...} file existence.
  2. Extracts \\label{...} anchors and validates \\ref{...}, \\cref{...}, \\Cref{...}, \\pageref{...}, \\autoref{...}.
  3. Validates \\cite{...} references against .bib files (e.g., references.bib).
  4. Validates code file references against repository workspace files.
"""

import os
import glob
import re
import sys
from typing import Dict, List, Optional, Set, Tuple


def find_project_roots(
    paper_dir: Optional[str] = None, repo_root: Optional[str] = None
) -> Tuple[str, str]:
    """Resolve paper_dir and repo_root paths."""
    if paper_dir is None or repo_root is None:
        current_script = os.path.abspath(__file__)
        scripts_dir = os.path.dirname(current_script)
        project_dir = os.path.dirname(scripts_dir)

        if repo_root is None:
            # If project_dir is ualbf-project, repo_root is project_dir's parent if docs_manifest is there
            if os.path.exists(os.path.join(project_dir, "..", "docs_manifest.json")):
                repo_root = os.path.abspath(os.path.join(project_dir, ".."))
            else:
                repo_root = project_dir

        if paper_dir is None:
            paper_dir = os.path.join(project_dir, "paper")

    return os.path.abspath(paper_dir), os.path.abspath(repo_root)


def strip_tex_line(line: str, in_code_block: bool) -> Tuple[str, bool]:
    """
    Strip LaTeX comments and handle code block boundaries (verbatim, lstlisting, minted).
    Returns (cleaned_line, new_in_code_block).
    """
    line_str = line.strip()

    # Check for code block environment changes
    if re.search(r"\\begin\{(?:verbatim|lstlisting|minted)\}", line_str):
        return "", True
    if re.search(r"\\end\{(?:verbatim|lstlisting|minted)\}", line_str):
        return "", False

    if in_code_block:
        return "", True

    # Strip comment lines or inline comments (not preceded by backslash)
    comment_split = re.split(r"(?<!\\)%", line, maxsplit=1)
    cleaned = comment_split[0]
    return cleaned, False


def extract_paper_labels(paper_dir: str) -> Set[str]:
    """Extract all \\label{...} anchors from LaTeX files in paper_dir."""
    labels: Set[str] = set()
    if not os.path.exists(paper_dir):
        return labels

    tex_files = glob.glob(os.path.join(paper_dir, "**", "*.tex"), recursive=True)
    for tf in tex_files:
        try:
            with open(tf, "r", encoding="utf-8") as f:
                lines = f.readlines()
        except Exception:
            continue

        in_code_block = False
        for line in lines:
            cleaned, in_code_block = strip_tex_line(line, in_code_block)
            if not cleaned or in_code_block:
                continue

            for m in re.findall(r"\\label\{([^}]+)\}", cleaned):
                lbl = m.strip()
                if lbl:
                    labels.add(lbl)

    return labels


def extract_bib_keys(paper_dir: str) -> Set[str]:
    """Extract all citation keys from .bib files in paper_dir."""
    keys: Set[str] = set()
    if not os.path.exists(paper_dir):
        return keys

    bib_files = glob.glob(os.path.join(paper_dir, "**", "*.bib"), recursive=True)
    for bf in bib_files:
        try:
            with open(bf, "r", encoding="utf-8") as f:
                content = f.read()
        except Exception:
            continue

        for m in re.findall(r"@\w+\s*\{\s*([^,\s]+)", content):
            key = m.strip()
            if key:
                keys.add(key)

    return keys


def extract_paper_refs(paper_dir: str) -> Dict[str, List[Tuple[str, int, str]]]:
    """
    Extract LaTeX references across paper files:
    Returns dict with keys 'inputs', 'refs', 'cites', 'code_files'.
    Each list item is a tuple: (rel_file_path, line_number, target_string).
    """
    results: Dict[str, List[Tuple[str, int, str]]] = {
        "inputs": [],
        "refs": [],
        "cites": [],
        "code_files": [],
    }

    if not os.path.exists(paper_dir):
        return results

    tex_files = glob.glob(os.path.join(paper_dir, "**", "*.tex"), recursive=True)
    path_code_pattern = re.compile(
        r"\b([A-Za-z0-9_/-]+\.(?:lean|rs|py|h|c|cpp|toml|json))\b"
    )

    for tf in sorted(tex_files):
        rel_tf = os.path.relpath(tf, paper_dir)
        try:
            with open(tf, "r", encoding="utf-8") as f:
                lines = f.readlines()
        except Exception:
            continue

        in_code_block = False
        for line_no, line in enumerate(lines, start=1):
            cleaned, in_code_block = strip_tex_line(line, in_code_block)
            if not cleaned or in_code_block:
                continue

            # Unescape TeX formatting macros for code file extraction
            cleaned_code = (
                cleaned.replace("\\allowbreak{}", "")
                .replace("\\allowbreak", "")
                .replace("\\_", "_")
            )

            # Extract \input{...}
            for m in re.findall(r"\\input\{([^}]+)\}", cleaned):
                inp = m.strip()
                if inp:
                    results["inputs"].append((rel_tf, line_no, inp))

            # Extract \ref{...}, \cref{...}, \Cref{...}, \pageref{...}, \autoref{...}
            for m in re.findall(
                r"\\(?:c|C)?(?:ref|pageref|autoref)\{([^}]+)\}", cleaned
            ):
                for ref_key in m.split(","):
                    rk = ref_key.strip()
                    if rk:
                        results["refs"].append((rel_tf, line_no, rk))

            # Extract \cite{...}, \cite[...]{...}, \citep{...}, \citeauthor{...}, etc.
            for m in re.findall(
                r"\\cite(?:[a-zA-Z]*)(?:\[[^\]]*\])?\{([^}]+)\}", cleaned
            ):
                for cite_key in m.split(","):
                    ck = cite_key.strip()
                    if ck:
                        results["cites"].append((rel_tf, line_no, ck))

            # Extract code file path references
            for m in path_code_pattern.findall(cleaned_code):
                code_path = m.strip()
                if code_path:
                    results["code_files"].append((rel_tf, line_no, code_path))

    return results


def is_valid_code_file_path(code_path: str, repo_root: str, paper_dir: str) -> bool:
    """Check if code file reference exists in workspace repository."""
    candidate_bases = [
        repo_root,
        os.path.join(repo_root, "ualbf-project"),
        os.path.join(repo_root, "ualbf-project", "lean4-proofs", "UALBF"),
        os.path.join(repo_root, "ualbf-project", "lean4-proofs"),
        os.path.join(repo_root, "ualbf-project", "rust-engine", "src"),
        os.path.join(repo_root, "ualbf-project", "rust-engine"),
        paper_dir,
    ]

    for base in candidate_bases:
        target = os.path.normpath(os.path.join(base, code_path))
        if os.path.exists(target):
            return True

    # Fallback search by filename within ualbf-project
    filename = os.path.basename(code_path)
    ualbf_dir = os.path.join(repo_root, "ualbf-project")
    if os.path.exists(ualbf_dir):
        for root, _, files in os.walk(ualbf_dir):
            if filename in files:
                return True

    return False


def validate_paper_sources(
    paper_dir: Optional[str] = None, repo_root: Optional[str] = None
) -> bool:
    """
    Validate all LaTeX paper source files in paper_dir against workspace repo_root.
    Checks:
      - \\input{...} targets exist
      - Section anchors (\\ref, \\cref, \\Cref, etc.) resolve to \\label{...} anchors
      - \\cite{...} keys resolve to .bib entries
      - Code file paths exist in workspace
    """
    paper_dir_abs, repo_root_abs = find_project_roots(paper_dir, repo_root)

    if not os.path.exists(paper_dir_abs):
        print(f"Error: Paper directory '{paper_dir_abs}' not found.", file=sys.stderr)
        return False

    labels = extract_paper_labels(paper_dir_abs)
    bib_keys = extract_bib_keys(paper_dir_abs)
    refs_data = extract_paper_refs(paper_dir_abs)

    valid = True

    # 1. Validate \input{...} references
    for rel_tf, line_no, inp_path in refs_data["inputs"]:
        target_exact = os.path.join(paper_dir_abs, inp_path)
        target_tex = (
            target_exact if target_exact.endswith(".tex") else target_exact + ".tex"
        )

        containing_dir = os.path.dirname(os.path.join(paper_dir_abs, rel_tf))
        rel_exact = os.path.join(containing_dir, inp_path)
        rel_tex = rel_exact if rel_exact.endswith(".tex") else rel_exact + ".tex"

        if not (
            os.path.exists(target_exact)
            or os.path.exists(target_tex)
            or os.path.exists(rel_exact)
            or os.path.exists(rel_tex)
        ):
            print(
                f"Error: Broken \\input reference in '{rel_tf}:{line_no}': target '{inp_path}' not found.",
                file=sys.stderr,
            )
            valid = False

    # 2. Validate \ref / \cref / \Cref section anchors
    for rel_tf, line_no, ref_key in refs_data["refs"]:
        if ref_key not in labels:
            print(
                f"Error: Broken section anchor reference in '{rel_tf}:{line_no}': label '{ref_key}' not found.",
                file=sys.stderr,
            )
            valid = False

    # 3. Validate \cite citation keys
    for rel_tf, line_no, cite_key in refs_data["cites"]:
        if cite_key not in bib_keys:
            print(
                f"Error: Broken bibliography reference in '{rel_tf}:{line_no}': citation key '{cite_key}' not found.",
                file=sys.stderr,
            )
            valid = False

    # 4. Validate code file references
    for rel_tf, line_no, code_path in refs_data["code_files"]:
        if not is_valid_code_file_path(code_path, repo_root_abs, paper_dir_abs):
            print(
                f"Error: Broken code file reference in '{rel_tf}:{line_no}': file '{code_path}' not found in repository workspace.",
                file=sys.stderr,
            )
            valid = False

    if valid:
        print("Paper source validation passed successfully.")

    return valid


def main() -> None:
    paper_dir_arg = sys.argv[1] if len(sys.argv) > 1 else None
    repo_root_arg = sys.argv[2] if len(sys.argv) > 2 else None

    paper_dir, repo_root = find_project_roots(paper_dir_arg, repo_root_arg)
    ok = validate_paper_sources(paper_dir, repo_root)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
