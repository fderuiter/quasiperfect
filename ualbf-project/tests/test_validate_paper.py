"""
Unit tests for ualbf-project/scripts/validate_paper.py
======================================================
Tests label extraction, reference resolution, missing input detection, citation checks,
code file reference verification, line stripping, and CLI execution.
"""

import os
import sys
import tempfile
import unittest
from unittest.mock import patch

# Ensure scripts directory is on sys.path
scripts_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "scripts"))
if scripts_dir not in sys.path:
    sys.path.insert(0, scripts_dir)

import validate_paper  # noqa: E402


class TestValidatePaperUnit(unittest.TestCase):
    def test_find_project_roots_default(self):
        p_dir, r_root = validate_paper.find_project_roots()
        self.assertTrue(os.path.exists(p_dir))
        self.assertTrue(os.path.exists(r_root))

    def test_find_project_roots_custom(self):
        p_dir, r_root = validate_paper.find_project_roots("/tmp/paper", "/tmp/repo")
        self.assertEqual(p_dir, "/tmp/paper")
        self.assertEqual(r_root, "/tmp/repo")

    def test_find_project_roots_no_manifest(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            script_path = os.path.join(tmp_dir, "scripts", "validate_paper.py")
            os.makedirs(os.path.dirname(script_path), exist_ok=True)
            with patch.object(validate_paper, "__file__", script_path):
                p_dir, r_root = validate_paper.find_project_roots(None, None)
                self.assertEqual(r_root, tmp_dir)

    def test_strip_tex_line(self):
        # Code block begin
        cleaned, in_code = validate_paper.strip_tex_line("\\begin{lstlisting}", False)
        self.assertEqual(cleaned, "")
        self.assertTrue(in_code)

        # Inside code block
        cleaned, in_code = validate_paper.strip_tex_line("code line", True)
        self.assertEqual(cleaned, "")
        self.assertTrue(in_code)

        # Code block end
        cleaned, in_code = validate_paper.strip_tex_line("\\end{lstlisting}", True)
        self.assertEqual(cleaned, "")
        self.assertFalse(in_code)

        # Comment line
        cleaned, in_code = validate_paper.strip_tex_line("% comment", False)
        self.assertEqual(cleaned, "")
        self.assertFalse(in_code)

        # Inline comment
        cleaned, in_code = validate_paper.strip_tex_line("text % comment", False)
        self.assertEqual(cleaned, "text ")
        self.assertFalse(in_code)

        # Escaped percent
        cleaned, in_code = validate_paper.strip_tex_line("50\\% value", False)
        self.assertEqual(cleaned, "50\\% value")
        self.assertFalse(in_code)

    def test_extract_paper_labels_nonexistent_dir(self):
        labels = validate_paper.extract_paper_labels("/nonexistent/path/paper")
        self.assertEqual(labels, set())

    def test_extract_paper_labels_unreadable_file(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            fpath = os.path.join(tmp_dir, "test.tex")
            with open(fpath, "w", encoding="utf-8") as f:
                f.write("\\label{lbl1}\n")
            with patch("builtins.open", side_effect=PermissionError("Denied")):
                labels = validate_paper.extract_paper_labels(tmp_dir)
                self.assertEqual(labels, set())

    def test_extract_paper_labels_success(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            fpath = os.path.join(tmp_dir, "section.tex")
            with open(fpath, "w", encoding="utf-8") as f:
                f.write(
                    "\\section{Sec}\n\\label{sec:one}\n\\label{ sec:two }\n\\label{}\n"
                )
            labels = validate_paper.extract_paper_labels(tmp_dir)
            self.assertEqual(labels, {"sec:one", "sec:two"})

    def test_extract_bib_keys_nonexistent_dir(self):
        keys = validate_paper.extract_bib_keys("/nonexistent/path/paper")
        self.assertEqual(keys, set())

    def test_extract_bib_keys_unreadable_file(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            fpath = os.path.join(tmp_dir, "refs.bib")
            with open(fpath, "w", encoding="utf-8") as f:
                f.write("@article{key1,\nauthor={Author}\n}\n")
            with patch("builtins.open", side_effect=PermissionError("Denied")):
                keys = validate_paper.extract_bib_keys(tmp_dir)
                self.assertEqual(keys, set())

    def test_extract_bib_keys_success(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            fpath = os.path.join(tmp_dir, "refs.bib")
            with open(fpath, "w", encoding="utf-8") as f:
                f.write("@article{cattaneo1951,\n}\n@inproceedings{ hagis1982 ,\n}\n")
            keys = validate_paper.extract_bib_keys(tmp_dir)
            self.assertEqual(keys, {"cattaneo1951", "hagis1982"})

    def test_extract_paper_refs_nonexistent_dir(self):
        res = validate_paper.extract_paper_refs("/nonexistent/path/paper")
        self.assertEqual(res, {"inputs": [], "refs": [], "cites": [], "code_files": []})

    def test_extract_paper_refs_unreadable_file(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            fpath = os.path.join(tmp_dir, "test.tex")
            with open(fpath, "w", encoding="utf-8") as f:
                f.write("\\input{sub}\n")
            with patch("builtins.open", side_effect=PermissionError("Denied")):
                res = validate_paper.extract_paper_refs(tmp_dir)
                self.assertEqual(
                    res, {"inputs": [], "refs": [], "cites": [], "code_files": []}
                )

    def test_extract_paper_refs_success(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            fpath = os.path.join(tmp_dir, "main.tex")
            with open(fpath, "w", encoding="utf-8") as f:
                f.write(
                    "\\input{sec1}\n"
                    "See \\cref{sec:one, sec:two} and \\Cref{sec:one} or \\pageref{sec:one} or \\autoref{sec:two}.\n"
                    "Cited \\cite[p. 10]{cattaneo1951, hagis1982}.\n"
                    "Code in Pure/RationalBounds.lean and lean_ffi.rs.\n"
                )
            res = validate_paper.extract_paper_refs(tmp_dir)
            self.assertEqual(len(res["inputs"]), 1)
            self.assertEqual(res["inputs"][0][2], "sec1")
            self.assertEqual(len(res["refs"]), 5)
            self.assertEqual(len(res["cites"]), 2)
            self.assertIn("Pure/RationalBounds.lean", [c[2] for c in res["code_files"]])
            self.assertIn("lean_ffi.rs", [c[2] for c in res["code_files"]])

    def test_is_valid_code_file_path_base_and_search(self):
        with tempfile.TemporaryDirectory() as repo_dir:
            ualbf = os.path.join(repo_dir, "ualbf-project")
            lean_dir = os.path.join(ualbf, "lean4-proofs", "UALBF", "Pure")
            os.makedirs(lean_dir, exist_ok=True)
            target_file = os.path.join(lean_dir, "RationalBounds.lean")
            with open(target_file, "w", encoding="utf-8") as f:
                f.write("-- Lean code\n")

            # Match via candidate base
            self.assertTrue(
                validate_paper.is_valid_code_file_path(
                    "Pure/RationalBounds.lean", repo_dir, ualbf
                )
            )

            # Match via filename search
            self.assertTrue(
                validate_paper.is_valid_code_file_path(
                    "RationalBounds.lean", repo_dir, ualbf
                )
            )

            # Nonexistent path
            self.assertFalse(
                validate_paper.is_valid_code_file_path(
                    "NonExistent.lean", repo_dir, ualbf
                )
            )

    def test_validate_paper_sources_nonexistent_paper_dir(self):
        self.assertFalse(
            validate_paper.validate_paper_sources(
                "/nonexistent/paper", "/nonexistent/repo"
            )
        )

    def test_validate_paper_sources_failures(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            p_dir = os.path.join(tmp_dir, "paper")
            r_dir = tmp_dir
            os.makedirs(p_dir, exist_ok=True)

            # Write .bib
            with open(os.path.join(p_dir, "refs.bib"), "w", encoding="utf-8") as f:
                f.write("@article{valid_cite, author={A}}\n")

            # Write .tex with broken \input, broken \ref, broken \cite, broken code ref
            with open(os.path.join(p_dir, "main.tex"), "w", encoding="utf-8") as f:
                f.write(
                    "\\label{lbl_ok}\n"
                    "\\input{missing_section}\n"
                    "\\ref{lbl_broken}\n"
                    "\\cite{broken_cite}\n"
                    "Code in MissingFile.lean\n"
                )

            self.assertFalse(validate_paper.validate_paper_sources(p_dir, r_dir))

    def test_validate_paper_sources_success(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            p_dir = os.path.join(tmp_dir, "paper")
            r_dir = tmp_dir
            os.makedirs(p_dir, exist_ok=True)

            # Create input target file
            with open(os.path.join(p_dir, "sub.tex"), "w", encoding="utf-8") as f:
                f.write("\\label{lbl_sub}\n")

            # Create code file
            code_file = os.path.join(r_dir, "ualbf-project", "Sample.lean")
            os.makedirs(os.path.dirname(code_file), exist_ok=True)
            with open(code_file, "w", encoding="utf-8") as f:
                f.write("-- Sample\n")

            # Create .bib file
            with open(os.path.join(p_dir, "refs.bib"), "w", encoding="utf-8") as f:
                f.write("@article{cite_ok, author={A}}\n")

            # Create main.tex
            with open(os.path.join(p_dir, "main.tex"), "w", encoding="utf-8") as f:
                f.write(
                    "\\input{sub}\n"
                    "See \\cref{lbl_sub}.\n"
                    "Reference \\cite{cite_ok}.\n"
                    "Code in Sample.lean.\n"
                )

            self.assertTrue(validate_paper.validate_paper_sources(p_dir, r_dir))

    def test_main_cli(self):
        with patch.object(sys, "argv", ["validate_paper.py"]):
            with patch("validate_paper.validate_paper_sources", return_value=True):
                with self.assertRaises(SystemExit) as cm:
                    validate_paper.main()
                self.assertEqual(cm.exception.code, 0)

        with patch.object(sys, "argv", ["validate_paper.py", "/p_dir", "/r_root"]):
            with patch("validate_paper.validate_paper_sources", return_value=False):
                with self.assertRaises(SystemExit) as cm:
                    validate_paper.main()
                self.assertEqual(cm.exception.code, 1)


if __name__ == "__main__":
    unittest.main()
