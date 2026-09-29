"""
Unit Test Suite for Centralized Build Environment Manifest and Validation
==========================================================================

Verifies:
1. Schema validation of env_manifest.json against env_manifest.schema.json.
2. Runtime typed environment variable accessors (get_env_var, require_env_var).
3. Immediate execution halt when deprecated bypass flags are present in os.environ.
4. Static AST scanning in validate_docs.py catching unregistered environment accesses.
5. Documentation alignment checking for active environment variables in TCB.md / README.md.
"""

import json
import os
import sys
import unittest
from unittest import mock

# Ensure ualbf-project root and scripts are in sys.path
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
scripts_dir = os.path.join(project_root, "scripts")
if project_root not in sys.path:
    sys.path.insert(0, project_root)
if scripts_dir not in sys.path:
    sys.path.insert(0, scripts_dir)

import env_util
import validate_docs


class TestEnvManifestAndUtil(unittest.TestCase):
    def setUp(self):
        self.repo_root = env_util.find_repo_root()

    def test_env_manifest_exists_and_validates_against_schema(self):
        manifest_path = os.path.join(self.repo_root, "env_manifest.json")
        schema_path = os.path.join(self.repo_root, "env_manifest.schema.json")

        self.assertTrue(os.path.exists(manifest_path), "env_manifest.json must exist at repo root")
        self.assertTrue(os.path.exists(schema_path), "env_manifest.schema.json must exist at repo root")

        manifest, schema = env_util.load_manifest_and_schema(manifest_path, schema_path)
        self.assertIsInstance(manifest, dict)
        self.assertIn("UALBF_TRUSTED_PUBLIC_KEY", manifest)
        self.assertIn("ALLOW_UNVERIFIED_BUILD", manifest)

    def test_schema_validation_rejects_invalid_manifest(self):
        invalid_manifest = {
            "INVALID_VAR": {
                "type": "invalid_type",
                "default": None,
                "status": "active",
                "description": "Test"
            }
        }
        schema = env_util.load_manifest_and_schema()[1]
        with self.assertRaises(ValueError):
            env_util.validate_manifest_schema(invalid_manifest, schema)

    def test_get_env_var_defaults_and_types(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            # Test default values
            self.assertEqual(env_util.get_env_var("UALBF_MAX_CERT_SIZE_MB"), 10.0)
            self.assertEqual(env_util.get_env_var("UALBF_SIEVE_LIMIT"), 250000)
            self.assertEqual(env_util.get_env_var("UALBF_TARGET_MIN_LOG10"), 35)
            self.assertIsNone(env_util.get_env_var("UALBF_TRUSTED_PUBLIC_KEY"))

        with mock.patch.dict(
            os.environ,
            {
                "UALBF_MAX_CERT_SIZE_MB": "25.5",
                "UALBF_DUMMY_PAPER_CI": "1",
                "UALBF_SIEVE_LIMIT": "500000",
                "UALBF_TRUSTED_PUBLIC_KEY": "1234567890abcdef",
            },
            clear=True,
        ):
            self.assertEqual(env_util.get_env_var("UALBF_MAX_CERT_SIZE_MB"), 25.5)
            self.assertTrue(env_util.get_env_var("UALBF_DUMMY_PAPER_CI"))
            self.assertEqual(env_util.get_env_var("UALBF_SIEVE_LIMIT"), 500000)
            self.assertEqual(env_util.get_env_var("UALBF_TRUSTED_PUBLIC_KEY"), "1234567890abcdef")

    def test_require_env_var(self):
        with mock.patch.dict(os.environ, {"UALBF_TRUSTED_PUBLIC_KEY": "abcd"}, clear=True):
            val = env_util.require_env_var("UALBF_TRUSTED_PUBLIC_KEY")
            self.assertEqual(val, "abcd")

        with mock.patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(ValueError):
                env_util.require_env_var("UALBF_TRUSTED_PUBLIC_KEY")

    def test_unregistered_variable_raises_error(self):
        with self.assertRaises(ValueError):
            env_util.get_env_var("NON_EXISTENT_VAR_12345")

    def test_deprecated_flags_halt_execution(self):
        for deprecated_flag in ["ALLOW_UNVERIFIED_BUILD", "UALBF_SKIP_VALIDATION"]:
            with mock.patch.dict(os.environ, {deprecated_flag: "1"}, clear=True):
                with self.assertRaises(SystemExit) as cm:
                    env_util.check_deprecated_env_vars()
                self.assertEqual(cm.exception.code, 1)

    def test_static_ast_scanner_and_doc_alignment(self):
        self.assertTrue(validate_docs.validate_env_manifest(self.repo_root))
        self.assertTrue(validate_docs.validate_env_vars(self.repo_root))
        self.assertTrue(validate_docs.validate_env_docs_alignment(self.repo_root))


if __name__ == "__main__":
    unittest.main()
