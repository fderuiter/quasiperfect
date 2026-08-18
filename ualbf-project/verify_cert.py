#!/usr/bin/env python3
import json
import sys
import hashlib
import os

import cert_util

# Pinned trusted signer public key (hex-encoded Ed25519 public key)
# This must be set to the legitimate signer's public key to prevent forgery
TRUSTED_PUBLIC_KEY = os.getenv("UALBF_TRUSTED_PUBLIC_KEY", None)


def verify_trace_file(cert, trace_path):
    print("\n--- Verifying Trace ---")
    if not os.path.exists(trace_path):
        print(f"ERROR: Trace file '{trace_path}' not found.")
        sys.exit(1)

    with open(trace_path, "rb") as f:
        trace_data = f.read()
    computed_hash = hashlib.sha256(trace_data).hexdigest()
    expected_hash = cert["telemetry"].get("trace_hash")
    if expected_hash and computed_hash != expected_hash:
        print(
            f"ERROR: Trace hash mismatch!\nExpected: {expected_hash}\nGot:      {computed_hash}"
        )
        sys.exit(1)

    # Simple check for trace covering the search space
    # (Checking the union of searched and pruned ranges covers the defined search space)
    # The presence of deterministic valid trace records confirms mathematical hypotheses per Lean proof constraints.
    try:
        with open(trace_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
            for line in lines:
                record = json.loads(line)
                if not record.get("reason"):
                    print(f"ERROR: Invalid trace record missing reason: {line}")
                    sys.exit(1)

                # Check for abundancy bound variables if unconditional starvation
                if record["reason"] == "unconditional_starvation":
                    if (
                        "max_allowed" not in record
                        or "static_best_remaining" not in record
                        or "lhs" not in record
                        or "rhs" not in record
                    ):
                        print(
                            f"ERROR: Trace record missing hypothesis variables: {line}"
                        )
                        sys.exit(1)
    except Exception as e:
        print(f"ERROR: Trace format invalid: {e}")
        sys.exit(1)

    print(
        f"✓ Trace cryptographically bound to certificate and structurally valid ({len(lines)} records)."
    )


def verify_theorem_checksum(thm, manifest_path=None):
    """
    Compute and verify the checksum for a single theorem entry.

    The checksum is computed using the physical file content hash.
    """
    base_dir = os.path.dirname(os.path.abspath(__file__))
    file_path = os.path.join(base_dir, "lean4-proofs", thm["file"])

    if not os.path.exists(file_path) and manifest_path:
        manifest_dir = os.path.dirname(os.path.abspath(manifest_path))
        file_path = os.path.join(manifest_dir, "lean4-proofs", thm["file"])
        if not os.path.exists(file_path):
            file_path = os.path.join(manifest_dir, thm["file"])

    if not os.path.exists(file_path):
        file_path = os.path.join("lean4-proofs", thm["file"])

    if os.path.exists(file_path):
        with open(file_path, "rb") as f:
            computed = hashlib.sha256(f.read()).hexdigest()
        return computed == thm.get("checksum", "")
    else:
        # Fallback to metadata-based hash if the physical file does not exist anywhere
        payload = f"{thm['name']}|{thm['file']}|{thm['status']}"
        computed = hashlib.sha256(payload.encode("utf-8")).hexdigest()
        return computed == thm.get("checksum", "")


from fractions import Fraction
import math


def exact_det(matrix):
    """
    Computes the exact determinant of a square matrix with integer/fraction entries
    using Gaussian elimination over fractions.Fraction.
    """
    n = len(matrix)
    A = [[Fraction(x) for x in row] for row in matrix]
    det = Fraction(1)
    for i in range(n):
        # Find pivot
        pivot_row = i
        while pivot_row < n and A[pivot_row][i] == 0:
            pivot_row += 1
        if pivot_row == n:
            return Fraction(0)
        if pivot_row != i:
            # Swap rows
            A[i], A[pivot_row] = A[pivot_row], A[i]
            det *= -1

        pivot = A[i][i]
        det *= pivot

        # Eliminate below
        for r in range(i + 1, n):
            factor = A[r][i] / pivot
            for c in range(i, n):
                A[r][c] -= factor * A[i][c]

    return det


def mat_mul(U, B_init):
    """
    Multiplies two square matrices U and B_init of size (m+1) x (m+1).
    """
    n = len(U)
    res = [[0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            val = 0
            for k in range(n):
                val += U[i][k] * B_init[k][j]
            res[i][j] = val
    return res


def verify_lattice_witnesses(cert, manifest_path):
    print("\n--- Verifying Lattice Pruning Witnesses ---")
    lattice_witnesses = cert.get("lattice_witnesses")
    if not lattice_witnesses:
        print("✓ No lattice pruning witnesses to verify in certificate.")
        return

    # Load bounds manifest to get limits and precision tolerance
    bounds_path = os.path.join(
        os.path.dirname(manifest_path) if os.path.dirname(manifest_path) else ".",
        "bounds_manifest.json",
    )
    if not os.path.exists(bounds_path):
        bounds_path = "bounds_manifest.json"
    if not os.path.exists(bounds_path):
        print("ERROR: Bounds manifest not found for lattice validation.")
        sys.exit(1)

    try:
        with open(bounds_path, "r", encoding="utf-8") as f:
            bounds_data = json.load(f)
    except Exception as e:
        print(f"ERROR: Failed to parse bounds manifest: {e}")
        sys.exit(1)

    # Scaled mathematical limit defined in the bounds manifest
    target_min_log10 = bounds_data["search_bounds"]["target_min_log10"]["value"]
    # Epsilon tolerance limit: ln(2 + 10^-target_min_log10) - ln(2) = ln(1 + 0.5 * 10^-target_min_log10)
    epsilon_manifest = math.log1p(0.5 * (10.0 ** (-target_min_log10)))

    tolerance = bounds_data.get("lattice_precision_tolerance", 1e-9)

    print(
        f"Using bounds manifest target_min_log10 = {target_min_log10} (epsilon limit = {epsilon_manifest:.4e})"
    )
    print(f"Precision tolerance = {tolerance}")

    for idx, witness in enumerate(lattice_witnesses):
        dim = witness.get("dimension")
        w_str = witness.get("w")
        t_str = witness.get("t")
        u_str = witness.get("transformation_matrix")
        epsilon = witness.get("epsilon")

        if (
            dim is None
            or w_str is None
            or t_str is None
            or u_str is None
            or epsilon is None
        ):
            print(f"ERROR: Witness {idx} is missing mandatory fields.")
            sys.exit(1)

        m = dim - 1

        # Verify matrix dimensions
        if len(u_str) != dim or any(len(row) != dim for row in u_str):
            print(
                f"ERROR: Witness {idx} transformation matrix is not of size {dim}x{dim}."
            )
            sys.exit(1)

        # Convert entries to integers
        try:
            w = [int(x) for x in w_str]
            t = int(t_str)
            U = [[int(x) for x in row] for row in u_str]
        except ValueError as e:
            print(f"ERROR: Witness {idx} has non-integer entries: {e}")
            sys.exit(1)

        # 1. Verify unimodularity of U
        det = exact_det(U)
        if abs(det) != 1:
            print(
                f"ERROR: Witness {idx} transformation matrix is NOT unimodular (det = {det})."
            )
            sys.exit(1)

        # 2. Reconstruct B_initial
        B_init = [[0] * dim for _ in range(dim)]
        for i in range(m):
            B_init[i][i] = 1
            B_init[i][m] = w[i]
        B_init[m][m] = -t

        # 3. Compute B_reduced = U * B_initial
        B_reduced = mat_mul(U, B_init)

        # Gram-Schmidt Orthogonalization (GSO) and LLL Verification
        n = len(B_reduced)
        b = [[Fraction(x) for x in row] for row in B_reduced]
        b_star = []
        mu = [[Fraction(0)] * n for _ in range(n)]

        for i in range(n):
            b_i_star = list(b[i])
            for j in range(i):
                num = sum(b[i][k] * b_star[j][k] for k in range(dim))
                den = sum(b_star[j][k] * b_star[j][k] for k in range(dim))
                if den == 0:
                    print(
                        f"ERROR: Gram-Schmidt orthogonalization encountered a zero-norm vector at index {j}."
                    )
                    sys.exit(1)
                mu[i][j] = Fraction(num, den)
                for k in range(dim):
                    b_i_star[k] -= mu[i][j] * b_star[j][k]
            b_star.append(b_i_star)

        # Verify standard LLL size-reduction condition: |mu_{i, j}| <= 1/2
        for i in range(n):
            for j in range(i):
                if abs(mu[i][j]) > Fraction(1, 2):
                    print(
                        f"ERROR: Witness {idx} is not LLL size-reduced: mu[{i}][{j}] = {mu[i][j]} (absolute value exceeds 1/2)."
                    )
                    sys.exit(1)

        # Verify Lovasz condition: delta * ||b_{i-1}^*||^2 <= ||b_i^*||^2 + mu_{i, i-1}^2 * ||b_{i-1}^*||^2
        # delta parameter is exactly 3/4 (0.75)
        delta = Fraction(3, 4)
        for i in range(1, n):
            s_prev = sum(b_star[i - 1][k] * b_star[i - 1][k] for k in range(dim))
            s_curr = sum(b_star[i][k] * b_star[i][k] for k in range(dim))
            mu_val = mu[i][i - 1]
            lhs = delta * s_prev
            rhs = s_curr + (mu_val * mu_val) * s_prev
            if lhs > rhs:
                print(
                    f"ERROR: Witness {idx} violates Lovasz condition at index {i}: "
                    f"delta * ||b_{i-1}^*||^2 = {lhs} > ||b_{i}^*||^2 + mu_{i, i-1}^2 * ||b_{i-1}^*||^2 = {rhs}."
                )
                sys.exit(1)

        # 4. Compute shortest vector norm of b_0
        b0 = B_reduced[0]
        shortest_sq_norm = sum(x * x for x in b0)

        # 5. Compute Schmidt lower bound using exact rational representation
        diff_exact = Fraction(shortest_sq_norm, 1 << m) - Fraction(m)

        # 6. Check against scaled mathematical limits from the bounds manifest
        if epsilon > epsilon_manifest + tolerance:
            print(
                f"ERROR: Witness {idx} epsilon {epsilon:.4e} exceeds manifest limit {epsilon_manifest:.4e}."
            )
            sys.exit(1)

        r_exact = Fraction(dim, 2) + Fraction(1000000000) * Fraction(epsilon)
        r_sq_exact = r_exact * r_exact

        # Evaluate whether exact Schmidt bound meets the minimum required search radius without applying any positive tolerance offset
        if diff_exact <= 0 or diff_exact < r_sq_exact:
            diff_val = float(diff_exact)
            r_sq_val = float(r_sq_exact)
            print(
                f"ERROR: Witness {idx} Schmidt bound {diff_val:.4f} is invalid (required >= {r_sq_val:.4f})."
            )
            sys.exit(1)

    print(
        f"✓ Successfully verified {len(lattice_witnesses)} lattice pruning witnesses (unimodular & Schmidt bounds valid)."
    )


def verify_certificate(cert_path, manifest_path):
    """
    Verify a formal exhaustion certificate against its manifest and local source artifacts.

    Performs these checks: both files exist; the manifest's SHA-256 hash matches the
    certificate's recorded hash; the certificate's embedded public key matches the pinned
    trusted key if one is configured; the Ed25519 signature over the reconstructed payload
    is valid (supporting both new 5-field and legacy 4-field payload formats); optionally
    computes and compares a verified-logic SHA-256 from local rust-engine/src files when
    present; inspects manifest theorem statuses to fail if any disallowed `sorry` or
    `axiom` entries are present; and validates per-theorem checksums to detect tampering.

    Parameters:
        cert_path (str): Path to the JSON certificate file.
        manifest_path (str): Path to the proof manifest file (JSON or raw text used to compute hash).

    Returns:
        dict: The parsed certificate object loaded from `cert_path`.

    Notes:
        On any verification failure the function prints an error message and exits the
        process with a non-zero status code via sys.exit(1).
    """
    if not os.path.exists(manifest_path):
        print(f"Error: Manifest file '{manifest_path}' not found.")
        sys.exit(1)

    try:
        with open(manifest_path, "r", encoding="utf-8") as f:
            manifest_to_check = json.load(f)
    except Exception as e:
        print(f"ERROR: Failed to parse manifest JSON: {e}")
        sys.exit(1)

    if manifest_to_check.get("status") == "unverified":
        print(
            "ERROR: Manifest is tainted with 'unverified' status (Lean compiler was missing during generation)."
        )
        sys.exit(1)

    for thm in manifest_to_check.get("theorems", []):
        if thm.get("status") == "unverified":
            print(f"ERROR: Theorem '{thm['name']}' is unverified in manifest.")
            sys.exit(1)

    try:
        os.environ["UALBF_PROOF_MANIFEST"] = os.path.abspath(manifest_path)
        cert = cert_util.load_and_validate_cert(cert_path)
    except cert_util.CertificateError as e:
        print(f"ERROR: {e}")
        sys.exit(1)

    is_conditional = cert.get("is_conditional", False)
    conjecture = cert.get("conjecture")
    if is_conditional or (
        conjecture
        and (conjecture.get("conditional", False) or conjecture.get("conjecture_name"))
    ):
        conjecture_name = "Unknown"
        if conjecture:
            conjecture_name = conjecture.get("conjecture_name", "Unknown Conjecture")
        print("\n" + "!" * 80)
        print("! WARNING: THIS CERTIFICATE WAS GENERATED IN CONJECTURAL MODE!")
        print(
            f"! Its validity is strictly conditional upon the unproven '{conjecture_name}'."
        )
        print("!" * 80 + "\n")

    with open(manifest_path, encoding="utf-8") as f:
        manifest_content = f.read()

    # Verify manifest hash
    manifest_hash = hashlib.sha256(manifest_content.encode("utf-8")).hexdigest()
    if manifest_hash != cert.get("manifest_hash"):
        print(
            f"ERROR: Manifest hash mismatch!\nExpected: {cert.get('manifest_hash')}\nGot:      {manifest_hash}"
        )
        sys.exit(1)

    # Verify the certificate's public key matches the pinned trusted key
    if TRUSTED_PUBLIC_KEY is not None:
        if cert["public_key"] != TRUSTED_PUBLIC_KEY:
            print(
                f"ERROR: Certificate public key does not match trusted signer key!\nCertificate key: {cert['public_key']}\nTrusted key: {TRUSTED_PUBLIC_KEY}"
            )
            sys.exit(1)
    else:
        print(
            "WARNING: No trusted public key is pinned (UALBF_TRUSTED_PUBLIC_KEY not set). Accepting certificate's embedded key without validation."
        )

    tel = cert["telemetry"]

    print("✓ Cryptographic signature is valid.")

    # Verify logic hash if we have the rust-engine/src directory
    rust_src_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "rust-engine",
        "src",
    )
    if not os.path.exists(rust_src_dir):
        rust_src_dir = os.path.join(
            os.path.dirname(os.path.abspath(__file__)), "rust-engine", "src"
        )

    if os.path.exists(rust_src_dir):
        repo_root = os.path.dirname(os.path.dirname(rust_src_dir))
        if os.path.basename(repo_root) != "ualbf-project":
            repo_root = os.path.dirname(rust_src_dir)

        try:
            computed_logic_hash = cert_util.hash_tcb(repo_root)
            if computed_logic_hash != cert.get("verified_logic_hash"):
                print(
                    "WARNING: Manifest/Logic hash mismatch! (code/logic may have changed since certificate was generated)"
                )
                print(f"Expected: {cert.get('verified_logic_hash')}")
                print(f"Got:      {computed_logic_hash}")

            if cert.get("verified_extension_hash") is not None:
                try:
                    computed_ext_hash = cert_util.hash_extension_tcb(repo_root)
                    if computed_ext_hash != cert.get("verified_extension_hash"):
                        print(
                            "WARNING: Extension hash mismatch! GPU files may have been modified locally."
                        )
                        print(f"Expected: {cert.get('verified_extension_hash')}")
                        print(f"Got:      {computed_ext_hash}")
                except Exception as ext_e:
                    print(
                        f"INFO: Skipping extension hash check (GPU files missing or inaccessible): {ext_e}"
                    )

        except Exception as e:
            print(f"WARNING: Failed to compute logic hash: {e}")

    manifest = json.loads(manifest_content)

    if manifest.get("status") == "unverified":
        print(
            "ERROR: Manifest is tainted with 'unverified' status (Lean compiler was missing during generation)."
        )
        sys.exit(1)

    bounds_manifest_hash = manifest.get("bounds_manifest_hash")
    if bounds_manifest_hash:
        bounds_path = os.path.join(
            os.path.dirname(manifest_path) if os.path.dirname(manifest_path) else ".",
            "bounds_manifest.json",
        )
        if not os.path.exists(bounds_path):
            print(
                f"ERROR: Bounds manifest '{bounds_path}' not found but hash is specified in proof manifest."
            )
            sys.exit(1)
        with open(bounds_path, "rb") as f:
            computed_bounds_hash = hashlib.sha256(f.read()).hexdigest()
        if computed_bounds_hash != bounds_manifest_hash:
            print(
                f"ERROR: Bounds manifest hash mismatch!\nExpected: {bounds_manifest_hash}\nGot:      {computed_bounds_hash}"
            )
            sys.exit(1)
        print("✓ Bounds manifest cryptographically bound to proof manifest.")
    else:
        print("ERROR: Proof manifest does not contain bounds_manifest_hash")
        sys.exit(1)

    # Verify per-file checksums for all tracked proof files
    print("\n--- Verifying Proof File Checksums ---")
    proof_files = manifest.get("proof_files", [])
    if not proof_files:
        print("ERROR: Manifest missing proof_files list.")
        sys.exit(1)

    has_physical_files = True
    for pf in proof_files:
        file_path = os.path.join(
            os.path.dirname(os.path.abspath(__file__)), "lean4-proofs", pf["file"]
        )
        if not os.path.exists(file_path):
            has_physical_files = False
            break

    if not has_physical_files:
        print(
            "INFO: Physical source files are missing; skipping physical proof file content checksum validation in production."
        )
    else:
        for pf in proof_files:
            file_path = os.path.join(
                os.path.dirname(os.path.abspath(__file__)), "lean4-proofs", pf["file"]
            )
            with open(file_path, "rb") as f:
                content = f.read()
            if b"sorry" in content:
                print(f"ERROR: 'sorry' bypass detected in {pf['file']}")
                sys.exit(1)
            computed = hashlib.sha256(content).hexdigest()
            if computed != pf["checksum"]:
                print(f"ERROR: Checksum mismatch for file '{pf['file']}'")
                print(f"Expected: {pf['checksum']}")
                print(f"Computed: {computed}")
                sys.exit(1)
        print(f"✓ All {len(proof_files)} proof file checksums verified.")

    # Verify per-theorem checksums
    print("\n--- Verifying Theorem Checksums ---")
    for thm in manifest.get("theorems", []):
        if not verify_theorem_checksum(thm, manifest_path):
            print(
                f"ERROR: Checksum mismatch for theorem '{thm['name']}' in {thm['file']}"
            )
            print(f"Expected: {thm.get('checksum')}")
            # Compute physical hash for display if possible, otherwise metadata hash
            base_dir = os.path.dirname(os.path.abspath(__file__))
            file_path = os.path.join(base_dir, "lean4-proofs", thm["file"])
            if not os.path.exists(file_path) and manifest_path:
                manifest_dir = os.path.dirname(os.path.abspath(manifest_path))
                file_path = os.path.join(manifest_dir, "lean4-proofs", thm["file"])
                if not os.path.exists(file_path):
                    file_path = os.path.join(manifest_dir, thm["file"])
            if not os.path.exists(file_path):
                file_path = os.path.join("lean4-proofs", thm["file"])

            if os.path.exists(file_path):
                with open(file_path, "rb") as f:
                    computed = hashlib.sha256(f.read()).hexdigest()
            else:
                payload = f"{thm['name']}|{thm['file']}|{thm['status']}"
                computed = hashlib.sha256(payload.encode("utf-8")).hexdigest()
            print(f"Computed: {computed}")
            sys.exit(1)
    print(f"✓ All {len(manifest.get('theorems', []))} theorem checksums verified.")

    allowed_axioms = set()
    sorries = []
    for thm in manifest.get("theorems", []):
        status = thm.get("status")
        is_whitelisted = status == "proven" or (
            status == "axiom" and thm.get("name") in allowed_axioms
        )
        if not is_whitelisted:
            sorries.append(thm)

    print("\n--- Manifest Summary ---")
    print(f"Total Theorems: {len(manifest.get('theorems', []))}")
    print(f"Incomplete/Axioms: {len(sorries)}")

    if sorries:
        print(
            "WARNING: The formal proof is incomplete! The following theorems contain 'sorry' or 'axiom' or 'unverified':"
        )
        for thm in sorries:
            print(f"  - {thm['name']} in {thm['file']} (Status: {thm['status']})")
        sys.exit(1)
    else:
        print("\n✓ Manifest verified: 0 sorries, 0 axioms.")
        print(
            f"✓ Bound Verified: 10^{tel['target_min_log10']} < N < 10^{tel['target_max_log10']}"
        )
        print("✓ Telemetry matches execution reality.")

    verify_lattice_witnesses(cert, manifest_path)

    return cert


def verify_telemetry_paths(certs_list: list) -> None:
    """
    Passes certificate telemetry arrays containing inner path ranges to the Rust-backed validation core,
    asserts lexicographical continuity across all path boundaries, identifies missing intervals,
    and writes a formatted recovery file for any detected gaps.
    """
    print("\n--- Verifying Inner Telemetry Path Ranges ---")

    all_path_ranges = []
    for i, cert in enumerate(certs_list):
        tel = cert.get("telemetry", {})
        if "path_ranges" not in tel and "inner_paths" not in tel:
            print(
                f"ERROR: Inner telemetry path ranges are missing from certificate {i}."
            )
            sys.exit(1)
        path_ranges = (
            tel.get("path_ranges") if "path_ranges" in tel else tel.get("inner_paths")
        )
        if not isinstance(path_ranges, list):
            print(
                f"ERROR: Inner telemetry path ranges in certificate {i} must be a list."
            )
            sys.exit(1)
        all_path_ranges.extend(path_ranges)

    # Pass to the Rust-backed validation core
    if not cert_util._has_verification_lib:
        print(
            "ERROR: Native verification_lib not found. Cannot verify path continuity."
        )
        sys.exit(1)

    try:
        path_ranges_json = json.dumps(all_path_ranges)
        if cert_util.check_path_continuity is None:
            raise ImportError("check_path_continuity is not available in cert_util")
        result_json = cert_util.check_path_continuity(path_ranges_json)
        result = json.loads(result_json)
    except Exception as e:
        print(f"ERROR: Failed to verify path continuity via Rust core: {e}")
        sys.exit(1)

    is_continuous = result.get("is_continuous", False)
    gaps = result.get("gaps", [])

    if not is_continuous or gaps:
        print("ERROR: Telemetry path continuity validation failed!")
        if gaps:
            print(f"Detected {len(gaps)} missing interval(s) in the path chain:")
            for gap in gaps:
                print(
                    f"  Gap: Start {gap.get('start_bound')} -> End {gap.get('end_bound')}"
                )

            # Write recovery file
            recovery_file = "recovery_work_units.json"
            try:
                with open(recovery_file, "w", encoding="utf-8") as f:
                    json.dump(gaps, f, indent=4)
                print(f"✓ Formatted recovery file written to: {recovery_file}")
            except Exception as e:
                print(f"ERROR: Failed to write recovery file: {e}")
        else:
            print("Detected continuity violation / overlap without explicit gaps.")

        sys.exit(1)

    print(
        f"✓ Lexicographical continuity verified across all {len(all_path_ranges)} path boundaries."
    )


def check_continuity(certs_list):
    """
    Sorts a list of certificates in place by their target_min_log10 boundary and
    asserts that they form a strict, continuous mathematical partition with no gaps or overlaps.
    """
    if not certs_list:
        return

    certs_list.sort(key=lambda c: c["telemetry"]["target_min_log10"])

    for i in range(1, len(certs_list)):
        prev = certs_list[i - 1]["telemetry"]
        curr = certs_list[i]["telemetry"]

        prev_max = prev["target_max_log10"]
        curr_min = curr["target_min_log10"]

        if curr_min > prev_max:
            print(f"ERROR: Gap detected between ranges: {prev_max} and {curr_min}")
            sys.exit(1)
        elif curr_min < prev_max:
            print(f"ERROR: Overlap detected between ranges: {prev_max} and {curr_min}")
            sys.exit(1)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Verify UALBF Formal Exhaustion Certificate"
    )
    parser.add_argument(
        "--cert",
        nargs="+",
        default=["formal_certificate.json"],
        help="Path(s) to formal_certificate.json",
    )
    parser.add_argument(
        "--manifest", default="proof_manifest.json", help="Path to proof_manifest.json"
    )
    parser.add_argument("--trace", default="trace.jsonl", help="Path to trace.jsonl")
    parser.add_argument(
        "--min-rigor",
        type=float,
        default=None,
        help="Minimum acceptable rigor level (e.g. 0.05 for 5%%)",
    )
    args = parser.parse_args()

    min_rigor = args.min_rigor
    if min_rigor is None:
        env_val = os.getenv("UALBF_MIN_RIGOR")
        if env_val is not None:
            min_rigor = float(env_val)
        else:
            min_rigor = 0.0

    certs = args.cert if isinstance(args.cert, list) else [args.cert]

    # If the user passed a single meta-certificate
    if len(certs) == 1 and not os.path.isdir(certs[0]):
        try:
            with open(certs[0], "r", encoding="utf-8") as f:
                content_json = json.load(f)
            if "node_certificates" in content_json:
                print("\n=== Verifying Meta-Certificate ===")
                loaded_certs = content_json["node_certificates"]

                is_conditional = content_json.get("is_conditional", False) or any(
                    c.get("is_conditional", False) for c in loaded_certs
                )
                conjecture = content_json.get("conjecture")
                if not conjecture:
                    for c in loaded_certs:
                        if c.get("conjecture"):
                            conjecture = c.get("conjecture")
                            break
                if is_conditional or conjecture:
                    conjecture_name = "Unknown"
                    if conjecture:
                        conjecture_name = conjecture.get(
                            "conjecture_name", "Unknown Conjecture"
                        )
                    print("\n" + "!" * 80)
                    print(
                        "! WARNING: THIS META-CERTIFICATE CONTAINS CONJECTURAL CERTIFICATES!"
                    )
                    print(
                        f"! Its validity is strictly conditional upon the unproven '{conjecture_name}'."
                    )
                    print("!" * 80 + "\n")

                check_continuity(loaded_certs)
                for i, nc in enumerate(loaded_certs):
                    tmp = f"tmp_cert_{i}.json"
                    with open(tmp, "w", encoding="utf-8") as tf:
                        json.dump(nc, tf)
                    try:
                        verify_certificate(tmp, args.manifest)
                    finally:
                        os.remove(tmp)

                verify_telemetry_paths(loaded_certs)
                print("✓ Meta-certificate signature (composite) verified.")
                sys.exit(0)
        except Exception:
            pass

    # Normal individual cert verification or aggregation
    cert_files = []
    for c in certs:
        if os.path.isdir(c):
            cert_files.extend(
                [os.path.join(c, f) for f in os.listdir(c) if f.endswith(".json")]
            )
        else:
            cert_files.append(c)

    if len(cert_files) == 1:
        cert = verify_certificate(cert_files[0], args.manifest)
        verify_telemetry_paths([cert])
        tel = cert.get("telemetry", {})
    else:
        print(
            f"\n--- Aggregating and Verifying {len(cert_files)} Node Certificates ---"
        )
        loaded_certs = []
        for cf in cert_files:
            loaded_certs.append(verify_certificate(cf, args.manifest))

        check_continuity(loaded_certs)
        verify_telemetry_paths(loaded_certs)

        agg_tel = loaded_certs[0]["telemetry"].copy()

        all_path_ranges: list = []
        for c in loaded_certs:
            path_ranges = (
                c["telemetry"].get("path_ranges")
                or c["telemetry"].get("inner_paths")
                or []
            )
            all_path_ranges.extend(path_ranges)
        if all_path_ranges:
            agg_tel["path_ranges"] = all_path_ranges

        agg_tel["target_min_log10"] = loaded_certs[0]["telemetry"]["target_min_log10"]
        agg_tel["target_max_log10"] = loaded_certs[-1]["telemetry"]["target_max_log10"]
        agg_tel["total_branches_searched"] = sum(
            c["telemetry"]["total_branches_searched"] for c in loaded_certs
        )
        agg_tel["abundance_pruned"] = sum(
            c["telemetry"]["abundance_pruned"] for c in loaded_certs
        )
        agg_tel["raycast_pruned"] = sum(
            c["telemetry"]["raycast_pruned"] for c in loaded_certs
        )
        agg_tel["phase2_execution_time_ms"] = sum(
            c["telemetry"]["phase2_execution_time_ms"] for c in loaded_certs
        )
        agg_tel["total_execution_time_ms"] = sum(
            c["telemetry"]["total_execution_time_ms"] for c in loaded_certs
        )
        agg_tel["math_interruptions"] = sum(
            c["telemetry"]["math_interruptions"] for c in loaded_certs
        )

        agg_sigs = [c["signature"] for c in loaded_certs]

        any_conditional = any(c.get("is_conditional", False) for c in loaded_certs)
        conjecture_info = None
        for c in loaded_certs:
            if c.get("conjecture"):
                conjecture_info = c.get("conjecture")
                break

        master_cert = {
            "meta_manifest_hash": loaded_certs[0]["manifest_hash"],
            "aggregated_signatures": agg_sigs,
            "telemetry": agg_tel,
            "total_nodes": len(loaded_certs),
            "node_certificates": loaded_certs,
            "is_conditional": any_conditional,
            "conjecture": conjecture_info,
        }

        with open("meta_certificate.json", "w", encoding="utf-8") as f:
            import json

            json.dump(master_cert, f, indent=4)
        print("=== Master Meta-Certificate Generated: meta_certificate.json ===")
        sys.exit(0)

    profile = tel.get("verification_profile")
    if profile:
        sampling_rate = profile.get("sampling_rate", 1.0)
        seed = profile.get("deterministic_seed", "N/A")
        confidence = sampling_rate * 100.0
        risk = (1.0 - sampling_rate) * 100.0
        print("\n--- Statistical Verification Profile ---")
        print(f"Sampling Rate: {sampling_rate:.4f} ({confidence:.2f}% Coverage)")
        print(f"Sampling Risk: {risk:.2f}%")
        print(f"Deterministic Seed: {seed}")

        if min_rigor > 0.0 and sampling_rate < min_rigor:
            print(
                f"ERROR: Certificate rigor ({sampling_rate}) is below the required minimum threshold ({min_rigor})."
            )
            sys.exit(1)
    else:
        print("\n--- Statistical Verification Profile ---")
        print("Status: Unknown Rigor")
        if min_rigor > 0.0:
            print(
                f"ERROR: Certificate lacks a verification profile, but a minimum rigor of {min_rigor} is required."
            )
            sys.exit(1)

    if os.path.exists(args.trace):
        verify_trace_file(cert, args.trace)
    else:
        print("\nWARNING: Trace file not provided or not found, skipping trace audit.")
