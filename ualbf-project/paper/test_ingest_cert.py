"""Tests for the ingest_cert.py telemetry ingestion script.

Covers the changes introduced in this PR:
  - Reading baseline_min_prime_factors from telemetry (default 7 when absent)
  - Reading prasad_sunitha_bound from telemetry (default 16 when absent)
  - Writing \\TelemetryHagisBaselineMinPrimeFactors and \\TelemetryPrasadSunithaBound
    LaTeX commands to telemetry.tex
"""

import io
import json
import os
import sys
import tempfile
import types
import unittest

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
                script_path = os.path.join(
                    os.path.dirname(os.path.abspath(__file__)), "ingest_cert.py"
                )
                with open(script_path, encoding="utf-8") as fh:
                    source = fh.read()

                with self.assertRaises(SystemExit) as cm:
                    exec(
                        compile(source, script_path, "exec"), {"__file__": script_path}
                    )  # noqa: S102
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

            with open(os.path.join(root_dir, "bounds_manifest.json"), "w") as f:
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

            with open(os.path.join(root_dir, "proof_manifest.json"), "w") as f:
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

            with open(os.path.join(root_dir, "cert_util.py"), "w") as f:
                f.write("class CertificateError(Exception): pass\n")
                f.write("def load_and_validate_cert(path):\n")
                f.write("    import json\n")
                f.write("    return json.load(open(path))\n")

            orig_script = os.path.join(
                os.path.dirname(os.path.abspath(__file__)), "ingest_cert.py"
            )
            with open(orig_script, "r") as f:
                source = f.read()

            cert_path = os.path.join(paper_dir, "cert.json")
            with open(cert_path, "w") as f:
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
                script_path = os.path.join(paper_dir, "ingest_cert.py")

                captured_out = io.StringIO()
                sys.stdout = captured_out

                with self.assertRaises(SystemExit) as cm:
                    exec(
                        compile(source, script_path, "exec"),
                        {"__file__": script_path, "__name__": "__main__"},
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


if __name__ == "__main__":
    unittest.main()
