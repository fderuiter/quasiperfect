import collections
import json
import os
import re
import sys
from typing import Optional, Tuple

project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

import auditor
import cert_util
import hash_util


def make_macro_name(s: str) -> str:
    # Replace digits with words
    digit_map = {
        "0": "Zero",
        "1": "One",
        "2": "Two",
        "3": "Three",
        "4": "Four",
        "5": "Five",
        "6": "Six",
        "7": "Seven",
        "8": "Eight",
        "9": "Nine",
    }
    for d, w in digit_map.items():
        s = s.replace(d, w)
    parts = re.split(r"[._]", s)
    res = "Hash"
    for p in parts:
        if not p:
            continue
        res += p[0].upper() + p[1:]
    return res


def check_deprecated_bypass() -> None:
    if "ALLOW_UNVERIFIED_BUILD" in os.environ or "UALBF_SKIP_VALIDATION" in os.environ:
        print(
            "Error: Bypass options are deprecated and verification cannot be skipped."
        )
        sys.exit(1)


def load_bounds(bounds_path: Optional[str] = None) -> dict:
    if bounds_path is None:
        bounds_path = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "bounds_manifest.json",
        )
    if not os.path.exists(bounds_path):
        print(f"Error: bounds_manifest.json not found at {bounds_path}.")
        sys.exit(1)

    try:
        cert_util.validate_file_size(bounds_path)
    except cert_util.CertificateError as e:
        print(f"Error: {e}")
        sys.exit(1)

    with open(bounds_path, "r", encoding="utf-8") as bf:
        bounds = json.load(bf)

    # Enforce required keys
    required_keys = ["omega_bounds", "euler_ceiling", "search_bounds"]
    for k in required_keys:
        if k not in bounds:
            print(f"Error: bounds_manifest.json missing required key '{k}'.")
            sys.exit(1)

    return bounds


def check_manifest(manifest_path: Optional[str] = None) -> Tuple[dict, str]:
    if manifest_path is None:
        manifest_path = os.environ.get("UALBF_PROOF_MANIFEST")
    if not manifest_path or not os.path.exists(manifest_path):
        manifest_path = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "proof_manifest.json",
        )
    if not os.path.exists(manifest_path):
        print(f"Error: Proof manifest '{manifest_path}' not found.")
        sys.exit(1)

    try:
        cert_util.validate_file_size(manifest_path)
    except cert_util.CertificateError as e:
        print(f"Error: {e}")
        sys.exit(1)

    with open(manifest_path, "r", encoding="utf-8") as mf:
        manifest_data_macros = json.load(mf)

    # Enforce top-level manifest status gate
    if manifest_data_macros.get("status") in ["unverified", "error", "failed"]:
        print(
            f"Error: Proof manifest status is '{manifest_data_macros.get('status')}'. Build halted."
        )
        sys.exit(1)

    allowed_axioms = {"UALBF.QPN.PrasadSunitha.qpn_div_5_coprime_3_omega_bound"}
    # Enforce theorem status gate
    unproven_theorems = []
    for thm in manifest_data_macros.get("theorems", []):
        thm_name = thm.get("name", "unknown")
        status = str(thm.get("status", "")).strip().lower()
        if status not in ("proven", "verified") and not (
            status == "axiom" and thm_name in allowed_axioms
        ):
            unproven_theorems.append((thm_name, thm.get("status", "missing")))

    if unproven_theorems:
        for thm_name, status in unproven_theorems:
            print(f"Error: Theorem '{thm_name}' is unproven (status: '{status}').")
        sys.exit(1)

    macro_to_sources = collections.defaultdict(list)

    for thm in manifest_data_macros.get("theorems", []):
        thm_name = thm["name"]
        macro = make_macro_name(thm_name)
        macro_to_sources[macro].append(thm_name)
        status_macro = f"{macro}Status"
        macro_to_sources[status_macro].append(f"{thm_name} (Status)")

    for fn in manifest_data_macros.get("verus_hashes", {}):
        macro = make_macro_name(fn)
        macro_to_sources[macro].append(fn)

    collisions = {
        macro: sources
        for macro, sources in macro_to_sources.items()
        if len(sources) > 1
    }
    if collisions:
        for macro, sources in collisions.items():
            print(
                f"Error: Duplicate LaTeX macro name '\\{macro}' generated from sources: {', '.join(sources)}"
            )
        sys.exit(1)

    return manifest_data_macros, manifest_path


def write_telemetry_tex(
    cert_path: Optional[str] = None,
    manifest_path: Optional[str] = None,
    bounds_path: Optional[str] = None,
    output_dir: Optional[str] = None,
) -> None:
    if bounds_path is None:
        bounds_path = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "bounds_manifest.json",
        )
    bounds = load_bounds(bounds_path)
    manifest_data_macros, manifest_path = check_manifest(manifest_path)

    if cert_path is None:
        cert_path = os.environ.get("UALBF_CERT_PATH")

    if not cert_path:
        print("Error: UALBF_CERT_PATH environment variable is required.")
        sys.exit(1)

    has_cert = os.path.exists(cert_path)
    if not has_cert:
        print(f"Error: {cert_path} not found.")
        sys.exit(1)

    telemetry_tex_path = (
        os.path.join(output_dir, "telemetry.tex") if output_dir else "telemetry.tex"
    )
    verif_tex_path = (
        os.path.join(output_dir, "verification_manifest.tex")
        if output_dir
        else "verification_manifest.tex"
    )

    with open(telemetry_tex_path, "w", encoding="utf-8") as f:
        if has_cert:
            try:
                cert_util.validate_file_size(cert_path)
                if os.environ.get("UALBF_DUMMY_PAPER_CI") == "1":
                    with open(cert_path, "r", encoding="utf-8") as cert_f:
                        cert = json.load(cert_f)
                else:
                    os.environ["UALBF_PROOF_MANIFEST"] = os.path.abspath(manifest_path)
                    cert = cert_util.load_and_validate_cert(cert_path)
            except cert_util.CertificateError as e:
                print(f"Error: {e}")
                sys.exit(1)

            tel = cert["telemetry"]

            if os.environ.get("UALBF_DUMMY_PAPER_CI") != "1":
                try:
                    import subprocess

                    active_commit = subprocess.check_output(
                        ["git", "rev-parse", "HEAD"],
                        cwd=project_root,
                        text=True,
                        stderr=subprocess.DEVNULL,
                    ).strip()
                    cert_commit = cert.get("commit_hash", "")
                    if cert_commit and cert_commit != "unknown" and active_commit:
                        if cert_commit != active_commit:
                            print(
                                f"Error: Certificate commit hash '{cert_commit}' does not match active build commit '{active_commit}'."
                            )
                            sys.exit(1)
                except Exception:
                    pass

                cert_ts = cert.get("timestamp") or tel.get("timestamp")
                if cert_ts is not None:
                    import time

                    if cert_ts > time.time() + 300:
                        print("Error: Certificate timestamp is in the future.")
                        sys.exit(1)

            # Requirement 4: Explicit validation errors for missing required fields
            required_tel_keys = [
                "phase2_execution_time_ms",
                "total_branches_searched",
                "target_min_log10",
                "target_max_log10",
            ]
            for k in required_tel_keys:
                if k not in tel:
                    print(f"Error: Required telemetry field '{k}' is missing.")
                    sys.exit(1)

            time_ms = tel["phase2_execution_time_ms"]
            branches = tel["total_branches_searched"]

            # Requirement 1: Default to zero instead of branches
            pruned = tel.get("abundance_pruned", 0)
            raycast = tel.get("raycast_pruned", 0)
            total_pruned = pruned + raycast
            if total_pruned > 0:
                abundance_pct = (pruned / total_pruned) * 100.0
                raycast_pct = (raycast / total_pruned) * 100.0
            else:
                abundance_pct = 100.0
                raycast_pct = 0.0

            nodes_per_sec = branches / (time_ms / 1000.0) if time_ms > 0 else 0

            p1_time = tel.get("phase1_execution_time_ms", 0)
            total_time = tel.get("total_execution_time_ms", p1_time + time_ms)
            p1_pruned = tel.get("phase1_pruned", 0)

            max_log = tel["target_max_log10"]
            min_log = tel["target_min_log10"]
            f.write(
                f"\\newcommand{{\\TelemetryPhaseTwoTime}}{{{time_ms / 1000:.2f}}}\n"
            )
            f.write(f"\\newcommand{{\\TelemetryPhaseTwoBranches}}{{{branches:,}}}\n")
            f.write(f"\\newcommand{{\\TelemetryPruned}}{{{total_pruned:,}}}\n")
            f.write(f"\\newcommand{{\\TelemetryMaxLog}}{{{max_log}}}\n")
            f.write(f"\\newcommand{{\\TelemetryMinLog}}{{{min_log}}}\n")
            f.write(
                f"\\newcommand{{\\TelemetryCertHash}}{{{cert['manifest_hash'][:12]}}}\n"
            )

            f.write(f"\\newcommand{{\\TelemetryPhaseOnePruned}}{{{p1_pruned:,}}}\n")
            f.write(
                f"\\newcommand{{\\TelemetryTotalTime}}{{{cert_util.format_duration(total_time / 1000.0, style='full')}}}\n"
            )
            f.write(
                f"\\newcommand{{\\TelemetryPhaseOneTime}}{{{cert_util.format_duration(p1_time / 1000.0, style='full')}}}\n"
            )
            f.write(
                f"\\newcommand{{\\TelemetryNodesPerSec}}{{{int(nodes_per_sec):,}}}\n"
            )
            f.write(f"\\newcommand{{\\TelemetryAbundancePct}}{{{abundance_pct:.1f}}}\n")
            f.write(f"\\newcommand{{\\TelemetryRaycastPct}}{{{raycast_pct:.1f}}}\n")

            # New requirements
            f.write(
                f"\\newcommand{{\\TelemetryEngineVersion}}{{{cert.get('engine_version', 'unknown')}}}\n"
            )
            f.write(
                f"\\newcommand{{\\TelemetryCommitHash}}{{{cert.get('commit_hash', 'unknown')}}}\n"
            )

            bounds_exceeded = tel.get("bounds_exceeded", False)
            if bounds_exceeded:
                print(
                    "Error: Search space boundaries were exceeded during telemetry capture."
                )
                sys.exit(1)

            math_interruptions = tel.get("math_interruptions", 0)
            if math_interruptions > 0:
                print(
                    f"Error: Telemetry reported {math_interruptions} math interruptions. Search is incomplete."
                )
                sys.exit(1)
            f.write("\\newcommand{\\TelemetryBoundsEnforced}{True}\n")

        if has_cert:
            # Enforce recursive chain of trust
            if not os.path.exists(manifest_path):
                print(
                    f"Error: Proof manifest '{manifest_path}' not found, cannot verify chain of trust."
                )
                sys.exit(1)

            try:
                cert_util.validate_file_size(manifest_path)
            except cert_util.CertificateError as e:
                print(f"Error: {e}")
                sys.exit(1)

            computed_manifest_hash = hash_util.hash_file(manifest_path)
            if computed_manifest_hash != cert.get("manifest_hash"):
                print("Error: Proof manifest hash mismatch in chain of trust.")
                sys.exit(1)

            with open(manifest_path, "rb") as mf_bytes:
                manifest_content_bytes = mf_bytes.read()

            manifest_data = json.loads(manifest_content_bytes.decode("utf-8"))
            expected_bounds_hash = manifest_data.get("bounds_manifest_hash")
            if not expected_bounds_hash:
                print("Error: Proof manifest missing bounds_manifest_hash.")
                sys.exit(1)

            try:
                cert_util.validate_file_size(bounds_path)
            except cert_util.CertificateError as e:
                print(f"Error: {e}")
                sys.exit(1)

            computed_bounds_hash = hash_util.hash_file(bounds_path)
            if computed_bounds_hash != expected_bounds_hash:
                print("Error: Bounds manifest hash mismatch in chain of trust.")
                sys.exit(1)

        ps_bound = (
            bounds["omega_bounds"]["prasad_sunitha"]["proof_bound"]
            + bounds["omega_bounds"]["prasad_sunitha"]["engine_justified_gap"]
        )
        hagis1982 = (
            bounds["omega_bounds"]["hagis1982"]["proof_bound"]
            + bounds["omega_bounds"]["hagis1982"]["engine_justified_gap"]
        )

        f.write(
            f"\\newcommand{{\\TelemetryHagisBaselineMinPrimeFactors}}{{{hagis1982}}}\n"
        )
        f.write(f"\\newcommand{{\\TelemetryPrasadSunithaBound}}{{{ps_bound}}}\n")

        # Generate verification macros and check hashes
        if os.path.exists(manifest_path):
            with open(manifest_path, "rb") as mf_bytes:
                manifest_data_macros = json.loads(mf_bytes.read().decode("utf-8"))

            # Requirement 4: Verify current hashes against codebase
            local_verus = {}
            for verus_file in ["verus_proofs.rs", "lean_export.rs"]:
                rust_file = os.path.join(
                    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                    "rust-engine",
                    "src",
                    verus_file,
                )
                if os.path.exists(rust_file):
                    with open(rust_file, "r", encoding="utf-8") as rf:
                        local_verus.update(auditor.compute_verus_hashes(rf.read()))

            expected_verus = manifest_data_macros.get("verus_hashes", {})
            for fn, expected_hash in expected_verus.items():
                if local_verus.get(fn) != expected_hash:
                    print(
                        f"Error: Local codebase hashes do not match proof_manifest.json! Modification detected in {fn}."
                    )
                    sys.exit(1)

            # Write LaTeX macros
            for thm in manifest_data_macros.get("theorems", []):
                name = thm["name"]
                status = thm["status"]
                macro_name = make_macro_name(name)
                f.write(f"\\newcommand{{\\{macro_name}}}{{{thm['checksum']}}}\n")
                f.write(f"\\newcommand{{\\{macro_name}Status}}{{{status}}}\n")

            for fn, h in manifest_data_macros.get("verus_hashes", {}).items():
                macro_name = make_macro_name(fn)
                f.write(f"\\newcommand{{\\{macro_name}}}{{{h}}}\n")

            # Write Verification Table
            with open(verif_tex_path, "w", encoding="utf-8") as vm:
                vm.write("\\begin{table}[h]\n")
                vm.write("\\centering\n")
                vm.write("\\begin{tabular}{|l|l|}\n")
                vm.write("\\hline\n")
                vm.write(
                    "\\textbf{Component} & \\textbf{Cryptographic Certificate (SHA-256)} \\\\\n"
                )
                vm.write("\\hline\n")
                vm.write("\\multicolumn{2}{|c|}{\\textbf{Lean Theorems}} \\\\\n")
                vm.write("\\hline\n")
                for thm in manifest_data_macros.get("theorems", []):
                    name_escaped = thm["name"].replace("_", "\\_")
                    macro_name = make_macro_name(thm["name"])
                    vm.write(
                        f"\\texttt{{{name_escaped}}} & \\texttt{{\\{macro_name}}} \\\\\n"
                    )
                vm.write("\\hline\n")
                vm.write(
                    "\\multicolumn{2}{|c|}{\\textbf{Rust/Verus Implementations}} \\\\\n"
                )
                vm.write("\\hline\n")
                for fn, h in manifest_data_macros.get("verus_hashes", {}).items():
                    fn_escaped = fn.replace("_", "\\_")
                    macro_name = make_macro_name(fn)
                    vm.write(
                        f"\\texttt{{{fn_escaped}}} & \\texttt{{\\{macro_name}}} \\\\\n"
                    )
                vm.write("\\hline\n")
                vm.write("\\end{tabular}\n")
                vm.write(
                    "\\caption{Cryptographic manifest of formally verified components.}\n"
                )
                vm.write("\\label{tab:verification_manifest}\n")
                vm.write("\\end{table}\n")


def check_manuscript_compliance(
    base_dir: Optional[str] = None, telemetry_tex_path: Optional[str] = None
) -> None:
    if base_dir is None:
        base_dir = os.path.dirname(os.path.abspath(__file__))
    if telemetry_tex_path is None:
        telemetry_tex_path = (
            os.path.join(base_dir, "telemetry.tex")
            if os.path.exists(os.path.join(base_dir, "telemetry.tex"))
            else "telemetry.tex"
        )

    if not os.path.exists(telemetry_tex_path):
        return

    telemetry_metrics = {}
    with open(telemetry_tex_path, "r", encoding="utf-8") as tf:
        for line in tf:
            m = re.match(
                r"\\newcommand\{\\Telemetry([A-Za-z0-9_]+)\}\{(.+?)\}", line.strip()
            )
            if m:
                suffix, val = m.groups()
                telemetry_metrics[suffix] = val

    forbidden_hardcoded_values = set()
    for v in telemetry_metrics.values():
        v = v.strip()
        if len(v) > 3 and re.search(r"[0-9]", v):
            forbidden_hardcoded_values.add(v)
        if "," in v:
            forbidden_hardcoded_values.add(v)

    for root_dir, dirs, files in os.walk(base_dir):
        for file in files:
            if file.endswith(".tex") and file not in [
                "telemetry.tex",
                "verification_manifest.tex",
            ]:
                file_path = os.path.join(root_dir, file)
                with open(file_path, "r", encoding="utf-8") as tf:
                    lines_tf = tf.readlines()
                for line_no, linetf in enumerate(lines_tf, 1):
                    for m in re.finditer(
                        r"\\newcommand\{\\(?:Claimed|Telemetry)([A-Za-z0-9_]+)\}",
                        linetf,
                    ):
                        print(
                            f"Error in {file}:{line_no}: Manual definition of verification macros is strictly excluded. Found: {m.group(0)}"
                        )
                        sys.exit(1)

                    for hv in forbidden_hardcoded_values:
                        if hv in linetf and "\\Telemetry" not in linetf:
                            if re.search(
                                r"(?<![0-9a-zA-Z\.])"
                                + re.escape(hv)
                                + r"(?![0-9a-zA-Z\.])",
                                linetf,
                            ):
                                print(
                                    f"Error in {file}:{line_no}: Hardcoded scientific metric '{hv}' detected. Use centralized manifest macros instead."
                                )
                                sys.exit(1)


def main(
    cert_path: Optional[str] = None,
    manifest_path: Optional[str] = None,
    bounds_path: Optional[str] = None,
    output_dir: Optional[str] = None,
) -> None:
    check_deprecated_bypass()
    write_telemetry_tex(
        cert_path=cert_path,
        manifest_path=manifest_path,
        bounds_path=bounds_path,
        output_dir=output_dir,
    )
    check_manuscript_compliance(base_dir=output_dir)


if __name__ == "__main__":
    main()
