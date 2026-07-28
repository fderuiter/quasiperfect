#!/usr/bin/env python3
import json
import os
import re
import sys


def get_nested_value(d, path):
    keys = path.split(".")
    for k in keys:
        if k in d:
            d = d[k]
        else:
            return None
    return d


def find_leaf_parameters(data, current_path=""):
    paths = []
    if isinstance(data, dict):
        for k, v in data.items():
            sub_path = f"{current_path}.{k}" if current_path else k
            if k in (
                "justification",
                "description",
                "is_axiomatic",
            ) or "citation" in sub_path.split("."):
                continue
            paths.extend(find_leaf_parameters(v, sub_path))
    elif isinstance(data, list):
        paths.append(current_path)
    elif isinstance(data, (int, float, bool)):
        paths.append(current_path)
    return paths


def get_parameter_candidates(path: str):
    parts = path.split(".")
    candidates = set()
    candidates.add(path)
    candidates.add(path.replace(".", "_"))
    for i in range(len(parts)):
        candidates.add("_".join(parts[i:]))
    all_candidates = list(candidates)
    for c in all_candidates:
        candidates.add(c.upper())
        candidates.add(c.lower())
    return sorted(list(candidates))


def is_parameter_documented(doc_content: str, candidates: list[str]) -> bool:
    for cand in candidates:
        if f"**{cand}**" in doc_content:
            return True
        if f"__{cand}__" in doc_content:
            return True
        if f"*{cand}*" in doc_content:
            return True
        if f"_{cand}_" in doc_content:
            return True
        if f"`{cand}`" in doc_content:
            return True

        escaped_cand = re.escape(cand)
        if re.search(
            rf"^\s*#+\s+.*?\b{escaped_cand}\b",
            doc_content,
            re.IGNORECASE | re.MULTILINE,
        ):
            return True

        bullet_pattern = rf"^\s*[-*+]\s+(?:\*\*|`|_)?{escaped_cand}(?:\*\*|`|_)?\b"
        if re.search(bullet_pattern, doc_content, re.IGNORECASE | re.MULTILINE):
            return True
    return False


def strip_comments(text: str, filename: str) -> str:
    _, ext = os.path.splitext(filename)
    is_rust = ext == ".rs"
    is_lean = ext == ".lean"

    if not is_rust and not is_lean:
        return text

    result = []
    i = 0
    n = len(text)

    block_depth = 0
    in_string = False
    in_char = False
    in_line_comment = False

    while i < n:
        if in_line_comment:
            if text[i] == "\n":
                in_line_comment = False
                result.append("\n")
            i += 1
        elif block_depth > 0:
            if is_rust and text[i : i + 2] == "*/":
                block_depth -= 1
                i += 2
            elif is_rust and text[i : i + 2] == "/*":
                block_depth += 1
                i += 2
            elif is_lean and text[i : i + 2] == "-/":
                block_depth -= 1
                i += 2
            elif is_lean and text[i : i + 2] == "/-":
                block_depth += 1
                i += 2
            else:
                if text[i] == "\n":
                    result.append("\n")
                i += 1
        elif in_string:
            if text[i] == "\\":
                result.append(text[i])
                if i + 1 < n:
                    result.append(text[i + 1])
                i += 2
            elif text[i] == '"':
                in_string = False
                result.append('"')
                i += 1
            else:
                result.append(text[i])
                i += 1
        elif in_char:
            if text[i] == "\\":
                result.append(text[i])
                if i + 1 < n:
                    result.append(text[i + 1])
                i += 2
            elif text[i] == "'":
                in_char = False
                result.append("'")
                i += 1
            else:
                result.append(text[i])
                i += 1
        else:
            if is_rust and text[i : i + 2] == "//":
                in_line_comment = True
                i += 2
            elif is_lean and text[i : i + 2] == "--":
                in_line_comment = True
                i += 2
            elif is_rust and text[i : i + 2] == "/*":
                block_depth = 1
                i += 2
            elif is_lean and text[i : i + 2] == "/-":
                block_depth = 1
                i += 2
            elif text[i] == '"':
                in_string = True
                result.append('"')
                i += 1
            elif is_rust and text[i] == "'":
                in_char = True
                result.append("'")
                i += 1
            else:
                result.append(text[i])
                i += 1

    return "".join(result)


def find_construct(content_stripped: str, construct: str, filename: str) -> bool:
    _, ext = os.path.splitext(filename)

    names_to_try = [construct]
    if "." in construct:
        names_to_try.append(construct.split(".")[-1])

    for name in names_to_try:
        if ext == ".rs":
            keywords = r"(fn|struct|enum|trait|union|const|static|type|mod)"
            pattern = rf"\b{keywords}\s+{re.escape(name)}\b"
        elif ext == ".lean":
            keywords = r"(def|theorem|lemma|structure|inductive|class|instance|abbrev)"
            pattern = rf"\b{keywords}\s+{re.escape(name)}\b"
        else:
            pattern = rf"\b{re.escape(name)}\b"

        if re.search(pattern, content_stripped):
            return True

    return False


def main():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    manifest_path = os.path.join(base_dir, "metadata_manifest.json")

    if not os.path.exists(manifest_path):
        print(f"Error: {manifest_path} not found.")
        sys.exit(1)

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    errors = 0

    # 1. Check code references
    for doc in manifest.get("documents", []):
        doc_file = os.path.join(base_dir, doc["file"])
        if not os.path.exists(doc_file):
            print(f"Error: Document {doc['file']} not found.")
            errors += 1
            continue

        with open(doc_file, "r", encoding="utf-8") as df:
            doc_content = df.read()

        for ref in doc.get("code_references", []):
            marker = ref["marker"]
            target = ref["target"]

            # verify marker is in doc
            if marker not in doc_content:
                # Let's find line number if it's there, but it's not.
                # Actually, the acceptance criteria says:
                # "The build fails with explicit file and line coordinates when a document reference points to a missing or renamed code construct."
                # This means the marker IS in the document, but the code construct (target) is missing!
                pass  # marker could be missing if someone removed it, but the main check is the target file

            # verify target file exists
            target_path = os.path.join(base_dir, target)
            if not os.path.exists(target_path):
                # find line coordinates in doc
                with open(doc_file, "r", encoding="utf-8") as df_lines:
                    for line_no, line in enumerate(df_lines, 1):
                        if marker in line:
                            print(
                                f"Error in {doc['file']}:{line_no}: Referenced code construct '{target}' is missing or renamed."
                            )
                            errors += 1
                if marker not in doc_content:
                    print(
                        f"Error in {doc['file']}: Marker '{marker}' not found, but it was supposed to point to '{target}'."
                    )
                    errors += 1
            else:
                # verify marker is in doc
                if marker not in doc_content:
                    print(
                        f"Error in {doc['file']}: Marker '{marker}' not found in document."
                    )
                    errors += 1

                # verify any code constructs listed
                if "code_constructs" in ref:
                    try:
                        with open(target_path, "r", encoding="utf-8") as tf:
                            target_content = tf.read()
                        stripped_content = strip_comments(target_content, target)
                    except Exception as e:
                        print(f"Error reading target file {target}: {e}")
                        errors += 1
                        continue

                    for construct in ref["code_constructs"]:
                        if not find_construct(stripped_content, construct, target):
                            # find line coordinates in doc where this marker is mentioned
                            line_found = False
                            with open(doc_file, "r", encoding="utf-8") as df_lines:
                                for line_no, line in enumerate(df_lines, 1):
                                    if marker in line:
                                        print(
                                            f"Error in {doc['file']}:{line_no}: Referenced code construct '{construct}' is missing or renamed in '{target}'."
                                        )
                                        errors += 1
                                        line_found = True
                                        break
                            if not line_found:
                                # Fallback if marker is not found in the doc lines
                                print(
                                    f"Error in {doc['file']}: Referenced code construct '{construct}' is missing or renamed in '{target}'."
                                )
                                errors += 1

    # 2. Check hardcoded metrics
    for metric in manifest.get("hardcoded_metrics", []):
        doc_file = os.path.join(base_dir, metric["file"])
        if not os.path.exists(doc_file):
            print(f"Error: Document {metric['file']} not found.")
            errors += 1
            continue

        with open(doc_file, "r", encoding="utf-8") as df_lines:
            lines = df_lines.readlines()

        for check in metric.get("checks", []):
            regex = re.compile(check["regex"])
            source_file = os.path.join(base_dir, check["source"])

            if not os.path.exists(source_file):
                print(f"Error: Source manifest {check['source']} not found.")
                errors += 1
                continue

            with open(source_file, "r", encoding="utf-8") as sf:
                source_data = json.load(sf)

            expected_val = str(get_nested_value(source_data, check["json_path"]))

            for line_no, line in enumerate(lines, 1):
                m = regex.search(line)
                if m:
                    actual_val = m.group(1)
                    if actual_val != expected_val:
                        print(
                            f"Error in {metric['file']}:{line_no}: Hardcoded metric '{actual_val}' conflicts with manifest value '{expected_val}'."
                        )
                        errors += 1

    # 3. Check bounds manifest configuration parameters documentation
    bounds_manifest_path = os.path.join(base_dir, "bounds_manifest.json")
    if not os.path.exists(bounds_manifest_path):
        print(f"Error: {bounds_manifest_path} not found.")
        errors += 1
    else:
        with open(bounds_manifest_path, "r", encoding="utf-8") as f:
            bounds_data = json.load(f)

        leaf_params = find_leaf_parameters(bounds_data)
        parameter_mappings = manifest.get("parameter_mappings", {})

        # Cache for read markdown files
        doc_contents = {}

        for param_path in leaf_params:
            if param_path not in parameter_mappings:
                print(
                    f"Error: Parameter '{param_path}' from bounds manifest has no mapping in metadata_manifest.json's 'parameter_mappings'."
                )
                errors += 1
                continue

            doc_file_rel = parameter_mappings[param_path]
            doc_file_abs = os.path.join(base_dir, doc_file_rel)

            if not os.path.exists(doc_file_abs):
                print(
                    f"Error: Document '{doc_file_rel}' mapped for parameter '{param_path}' does not exist."
                )
                errors += 1
                continue

            if doc_file_rel not in doc_contents:
                with open(doc_file_abs, "r", encoding="utf-8") as df:
                    doc_contents[doc_file_rel] = df.read()

            doc_content = doc_contents[doc_file_rel]
            candidates = get_parameter_candidates(param_path)

            if not is_parameter_documented(doc_content, candidates):
                print(
                    f"Error: Parameter '{param_path}' (expected as one of {candidates}) is missing from its expected documentation path '{doc_file_rel}'."
                )
                errors += 1

    if errors > 0:
        sys.exit(1)
    print("Metadata Verification Hub: All checks passed.")


if __name__ == "__main__":
    main()
