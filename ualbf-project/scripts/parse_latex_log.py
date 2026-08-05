#!/usr/bin/env python3
import sys
import os
import re


def unwrap_log_content(content: str) -> str:
    """
    Standard LaTeX log wraps lines at 79 or 80 characters.
    Unwraps these wrapped lines to reconstruct long lines and prevent warnings from being broken mid-sentence or mid-key.
    """
    lines = content.splitlines()
    if not lines:
        return ""
    unwrapped = [lines[0]]
    for i in range(1, len(lines)):
        prev_line = lines[i - 1]
        curr_line = lines[i]
        # In pdflatex, the default max_print_line is 79 characters, sometimes 80
        if len(prev_line) in (79, 80):
            unwrapped[-1] += curr_line
        else:
            unwrapped.append(curr_line)
    return "\n".join(unwrapped)


def parse_log(log_content: str):
    """
    Parses LaTeX log content for undefined references and citations.
    Returns a tuple of (missing_references, missing_citations).
    Each is a sorted list of unique missing keys.
    """
    # We will search on both raw and unwrapped content to ensure maximum coverage
    unwrapped_content = unwrap_log_content(log_content)

    # Define regexes for references and citations
    # These match either LaTeX warnings or package-specific warnings, extracting the keys.
    # We support two forms:
    # 1. Quoted keys, which can contain spaces/commas (e.g. `key1, key2') but NOT newlines.
    # 2. Unquoted keys, which are contiguous non-whitespace and non-quote sequences.
    ref_pattern = re.compile(
        r"(?:LaTeX|Package\s+[\w-]+)\s+Warning:\s+Reference\s+(?:[`'‘“\"]+([^`'’”“\"\r\n]+)['’\`”\"]+|([^`'’”“\"'\r\n ]+))\s+(?:on page\s+\d+\s+)?undefined",
        re.IGNORECASE,
    )
    cit_pattern = re.compile(
        r"(?:LaTeX|Package\s+[\w-]+)\s+Warning:\s+Citation\s+(?:[`'‘“\"]+([^`'’”“\"\r\n]+)['’\`”\"]+|([^`'’”“\"'\r\n ]+))\s+(?:on page\s+\d+\s+)?undefined",
        re.IGNORECASE,
    )

    missing_refs = set()
    missing_cits = set()

    for content in (log_content, unwrapped_content):
        # Scan references
        for match in ref_pattern.finditer(content):
            key_str = match.group(1) or match.group(2)
            if key_str:
                # Split by comma in case multiple keys are defined, e.g. \ref{key1, key2}
                for k in key_str.split(","):
                    k_clean = k.strip()
                    if k_clean:
                        missing_refs.add(k_clean)

        # Scan citations
        for match in cit_pattern.finditer(content):
            key_str = match.group(1) or match.group(2)
            if key_str:
                for k in key_str.split(","):
                    k_clean = k.strip()
                    if k_clean:
                        missing_cits.add(k_clean)

    return sorted(list(missing_refs)), sorted(list(missing_cits))


def main():
    if len(sys.argv) < 2:
        # Default to checking paper/main.log or main.log if no file is provided
        possible_defaults = ["paper/main.log", "main.log"]
        log_path = None
        for path in possible_defaults:
            if os.path.exists(path):
                log_path = path
                break
        if not log_path:
            print("Usage: python3 parse_latex_log.py <path_to_log_file>")
            print(
                "Error: No log file provided, and default log files ('paper/main.log', 'main.log') were not found."
            )
            sys.exit(1)
    else:
        log_path = sys.argv[1]

    if not os.path.exists(log_path):
        print(f"Error: LaTeX log file not found at '{log_path}'.")
        sys.exit(1)

    try:
        with open(log_path, "r", encoding="utf-8", errors="ignore") as f:
            content = f.read()
    except Exception as e:
        print(f"Error: Failed to read log file '{log_path}': {e}")
        sys.exit(1)

    missing_refs, missing_cits = parse_log(content)

    if missing_refs or missing_cits:
        print("=" * 72)
        print("                  LaTeX LOG VALIDATION FAILURE")
        print("=" * 72)
        print("CRITICAL: Undefined references or citations were detected in the build!")
        print(f"Source Log File: {log_path}\n")

        if missing_refs:
            print(f"Missing References ({len(missing_refs)}):")
            for ref in missing_refs:
                print(f"  - {ref}")
            print()

        if missing_cits:
            print(f"Missing Citations ({len(missing_cits)}):")
            for cit in missing_cits:
                print(f"  - {cit}")
            print()

        print("Please resolve these broken keys in your LaTeX source files.")
        print("=" * 72)
        sys.exit(1)
    else:
        print("=" * 72)
        print("                  LaTeX LOG VALIDATION SUCCESS")
        print("=" * 72)
        print("All references and citations are fully resolved.")
        print(f"Source Log File: {log_path}")
        print("=" * 72)
        sys.exit(0)


if __name__ == "__main__":
    main()
