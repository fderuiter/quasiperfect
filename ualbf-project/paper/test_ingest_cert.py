"""Tests for the ingest_cert.py telemetry ingestion script.

Covers:
  - Missing cert file fatal error handling
  - Duplicate LaTeX macro collision detection
  - Proof manifest status gates (unverified, unproven theorems)
  - Deprecated bypass flags
"""

import io
import json
import os
import sys
import tempfile
import types
import unittest

paper_dir = os.path.dirname(os.path.abspath(__file__))
if paper_dir not in sys.path:
    sys.path.insert(0, paper_dir)

import ingest_cert  # noqa: E402

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_cert(extra_telemetry=None):
    """Return a minimal valid certificate dict accepted by ingest_cert.py."""
    tel = {
        "phase2_execution_time_ms": 5000,
        "total_branches_searched": 1000,
        "abundance_pruned": 200,
        "target_min_log10": 35,
        "target_max_log10": 37,
    }
    if extra_telemetry:
        tel.update(extra_telemetry)
    return {
        "manifest_hash": "fd91aafa6031fa4a084064097548449b4cca658d991183d2817115dc0c51233b",
        "verified_logic_hash": "1234567890abcdef1234567890abcdef",
        "public_key": "deadbeefdeadbeef",
        "signature": "cafebabecafebabecafebabe",
        "telemetry": tel,
    }


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestIngestCertMissingFile(unittest.TestCase):
    """Test that missing cert file raises a fatal error."""

    def test_missing_cert_raises_error(self):
        """When the cert file does not exist, ingest_cert.py must exit with a non-zero code."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            missing_path = os.path.join(tmp_dir, "no_cert.json")

            orig_env = os.environ.get("UALBF_CERT_PATH")
            orig_cwd = os.getcwd()

            # Mock verification_lib
            mock_verif = types.ModuleType("verification_lib")
            mock_verif.validate_certificate = lambda x: x
            mock_verif.hash_tcb = lambda: "tcb_hash"
            mock_verif.hash_extension_tcb = lambda: "ext_hash"
            mock_verif.check_path_continuity = lambda x: "{}"
            mock_verif.compute_verus_hashes = lambda x: {}
            orig_verif = sys.modules.get("verification_lib")
            sys.modules["verification_lib"] = mock_verif
            try:
                os.environ["UALBF_CERT_PATH"] = missing_path
                os.chdir(tmp_dir)

                with self.assertRaises(SystemExit) as cm:
                    ingest_cert.main(cert_path=missing_path, output_dir=tmp_dir)
                self.assertEqual(cm.exception.code, 1)
            finally:
                if orig_verif is not None:
                    sys.modules["verification_lib"] = orig_verif
                else:
                    sys.modules.pop("verification_lib", None)
                os.chdir(orig_cwd)
                if orig_env is None:
                    os.environ.pop("UALBF_CERT_PATH", None)
                else:
                    os.environ["UALBF_CERT_PATH"] = orig_env


class TestCollisionDetection(unittest.TestCase):
    def test_duplicate_macro_halts_execution(self):
        """When duplicate macros are generated, the script exits without writing telemetry.tex"""
        with tempfile.TemporaryDirectory() as root_dir:
            paper_dir = os.path.join(root_dir, "paper")
            os.mkdir(paper_dir)

            bounds_path = os.path.join(root_dir, "bounds_manifest.json")
            with open(bounds_path, "w", encoding="utf-8") as f:
                json.dump(
                    {
                        "omega_bounds": {
                            "prasad_sunitha": {
                                "proof_bound": 16,
                                "engine_justified_gap": 0,
                            },
                            "hagis1982": {"proof_bound": 7, "engine_justified_gap": 0},
                        },
                        "euler_ceiling": 100,
                        "search_bounds": {
                            "target_min_log10": {"value": 35},
                            "target_max_log10": {"value": 37},
                        },
                    },
                    f,
                )

            manifest_path = os.path.join(root_dir, "proof_manifest.json")
            with open(manifest_path, "w", encoding="utf-8") as f:
                json.dump(
                    {
                        "theorems": [
                            {
                                "name": "fermat_3",
                                "status": "Verified",
                                "checksum": "abc",
                            },
                            {
                                "name": "fermat.3",
                                "status": "Verified",
                                "checksum": "def",
                            },
                        ]
                    },
                    f,
                )

            sys.path.insert(0, root_dir)
            sys.modules.pop("cert_util", None)
            with open(
                os.path.join(root_dir, "cert_util.py"), "w", encoding="utf-8"
            ) as f:
                f.write("class CertificateError(Exception): pass\n")
                f.write("def load_and_validate_cert(path):\n")
                f.write("    import json\n")
                f.write("    return json.load(open(path))\n")

            cert_path = os.path.join(paper_dir, "cert.json")
            with open(cert_path, "w", encoding="utf-8") as f:
                json.dump(_minimal_cert(), f)

            orig_env = os.environ.get("UALBF_CERT_PATH")
            orig_cwd = os.getcwd()

            mock_verif = types.ModuleType("verification_lib")
            mock_verif.validate_certificate = lambda x: x
            mock_verif.hash_tcb = lambda: "tcb_hash"
            mock_verif.hash_extension_tcb = lambda: "ext_hash"
            mock_verif.check_path_continuity = lambda x: "{}"
            mock_verif.compute_verus_hashes = lambda x: {}
            orig_verif = sys.modules.get("verification_lib")
            sys.modules["verification_lib"] = mock_verif

            try:
                os.environ["UALBF_CERT_PATH"] = cert_path
                os.chdir(paper_dir)

                captured_out = io.StringIO()
                sys.stdout = captured_out

                with self.assertRaises(SystemExit) as cm:
                    ingest_cert.main(
                        cert_path=cert_path,
                        manifest_path=manifest_path,
                        bounds_path=bounds_path,
                        output_dir=paper_dir,
                    )

                self.assertEqual(cm.exception.code, 1)

                output = captured_out.getvalue()
                self.assertIn("Duplicate LaTeX macro name", output)
                self.assertIn("fermat_3", output)
                self.assertIn("fermat.3", output)

                self.assertFalse(
                    os.path.exists(os.path.join(paper_dir, "telemetry.tex"))
                )
            finally:
                sys.stdout = sys.__stdout__
                if orig_verif is not None:
                    sys.modules["verification_lib"] = orig_verif
                else:
                    sys.modules.pop("verification_lib", None)
                os.chdir(orig_cwd)
                if orig_env is None:
                    os.environ.pop("UALBF_CERT_PATH", None)
                else:
                    os.environ["UALBF_CERT_PATH"] = orig_env
                if root_dir in sys.path:
                    sys.path.remove(root_dir)
                sys.modules.pop("cert_util", None)


class TestManifestStatusGate(unittest.TestCase):
    def _run_ingest_script(self, root_dir, paper_dir, env_vars=None):
        bounds_path = os.path.join(root_dir, "bounds_manifest.json")
        manifest_path = os.path.join(root_dir, "proof_manifest.json")
        cert_path = os.path.join(paper_dir, "cert.json")
        if not os.path.exists(cert_path):
            with open(cert_path, "w", encoding="utf-8") as f:
                json.dump(_minimal_cert(), f)

        orig_env = os.environ.copy()
        orig_cwd = os.getcwd()

        mock_verif = types.ModuleType("verification_lib")
        mock_verif.validate_certificate = lambda x: x
        mock_verif.hash_tcb = lambda: "tcb_hash"
        mock_verif.hash_extension_tcb = lambda: "ext_hash"
        mock_verif.check_path_continuity = lambda x: "{}"
        mock_verif.compute_verus_hashes = lambda x: {}
        orig_verif = sys.modules.get("verification_lib")
        sys.modules["verification_lib"] = mock_verif

        try:
            os.environ.pop("UALBF_PROOF_MANIFEST", None)
            os.environ["UALBF_CERT_PATH"] = cert_path
            os.environ["UALBF_DUMMY_PAPER_CI"] = "1"
            if env_vars:
                os.environ.update(env_vars)

            os.chdir(paper_dir)

            captured_out = io.StringIO()
            sys.stdout = captured_out

            with self.assertRaises(SystemExit) as cm:
                ingest_cert.main(
                    cert_path=cert_path,
                    manifest_path=manifest_path,
                    bounds_path=bounds_path,
                    output_dir=paper_dir,
                )

            return cm.exception.code, captured_out.getvalue()
        finally:
            sys.stdout = sys.__stdout__
            if orig_verif is not None:
                sys.modules["verification_lib"] = orig_verif
            else:
                sys.modules.pop("verification_lib", None)
            os.chdir(orig_cwd)
            os.environ.clear()
            os.environ.update(orig_env)

    def _setup_environment(self, root_dir, theorems_status, manifest_status=None):
        paper_dir = os.path.join(root_dir, "paper")
        os.makedirs(paper_dir, exist_ok=True)

        with open(
            os.path.join(root_dir, "bounds_manifest.json"), "w", encoding="utf-8"
        ) as f:
            json.dump(
                {
                    "omega_bounds": {
                        "prasad_sunitha": {
                            "proof_bound": 16,
                            "engine_justified_gap": 0,
                        },
                        "hagis1982": {"proof_bound": 7, "engine_justified_gap": 0},
                    },
                    "euler_ceiling": 100,
                    "search_bounds": {
                        "target_min_log10": {"value": 35},
                        "target_max_log10": {"value": 37},
                    },
                },
                f,
            )

        manifest_data = {
            "theorems": [
                {
                    "name": thm_name,
                    "status": status,
                    "checksum": "abc",
                }
                for thm_name, status in theorems_status
            ]
        }
        if manifest_status:
            manifest_data["status"] = manifest_status

        with open(
            os.path.join(root_dir, "proof_manifest.json"), "w", encoding="utf-8"
        ) as f:
            json.dump(manifest_data, f)

        sys.path.insert(0, root_dir)
        sys.modules.pop("cert_util", None)
        with open(os.path.join(root_dir, "cert_util.py"), "w", encoding="utf-8") as f:
            f.write("class CertificateError(Exception): pass\n")
            f.write("def load_and_validate_cert(path):\n")
            f.write("    import json\n")
            f.write("    return json.load(open(path))\n")

        return paper_dir

    def test_unproven_theorem_status_halts_execution(self):
        """Unproven theorem status (e.g. sorry, stub, axiom, error) halts build before generating TeX files."""
        for bad_status in [
            "sorry",
            "stub",
            "axiom",
            "error",
            "unverified",
            "unknown_status",
        ]:
            with tempfile.TemporaryDirectory() as root_dir:
                paper_dir = self._setup_environment(
                    root_dir,
                    [
                        ("verified_theorem", "proven"),
                        ("incomplete_theorem", bad_status),
                    ],
                )
                exit_code, output = self._run_ingest_script(root_dir, paper_dir)

                self.assertEqual(exit_code, 1)
                self.assertIn("Error: Theorem 'incomplete_theorem' is unproven", output)
                self.assertIn(f"status: '{bad_status}'", output)
                self.assertFalse(
                    os.path.exists(os.path.join(paper_dir, "telemetry.tex"))
                )
                self.assertFalse(
                    os.path.exists(os.path.join(paper_dir, "verification_manifest.tex"))
                )

    def test_unverified_manifest_status_halts_execution(self):
        """Manifest with top-level 'unverified' status halts build immediately."""
        with tempfile.TemporaryDirectory() as root_dir:
            paper_dir = self._setup_environment(
                root_dir,
                [("some_theorem", "proven")],
                manifest_status="unverified",
            )
            exit_code, output = self._run_ingest_script(root_dir, paper_dir)

            self.assertEqual(exit_code, 1)
            self.assertIn("Proof manifest status is 'unverified'", output)
            self.assertFalse(os.path.exists(os.path.join(paper_dir, "telemetry.tex")))

    def test_deprecated_bypass_flags_halt_execution(self):
        """Passing deprecated bypass flags triggers immediate rejection."""
        for flag in ["ALLOW_UNVERIFIED_BUILD", "UALBF_SKIP_VALIDATION"]:
            with tempfile.TemporaryDirectory() as root_dir:
                paper_dir = self._setup_environment(
                    root_dir,
                    [("some_theorem", "proven")],
                )
                exit_code, output = self._run_ingest_script(
                    root_dir, paper_dir, env_vars={flag: "1"}
                )

                self.assertEqual(exit_code, 1)
                self.assertIn("Bypass options are deprecated", output)
                self.assertFalse(
                    os.path.exists(os.path.join(paper_dir, "telemetry.tex"))
                )


if __name__ == "__main__":
    unittest.main()
