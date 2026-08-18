from verify_metadata import (
    strip_comments,
    find_construct,
    find_leaf_parameters,
    get_parameter_candidates,
    is_parameter_documented,
    is_backtick_valid,
    extract_backticks_with_lines,
    SUPERSCRIPTS,
)


def test_strip_comments_rust():
    # Test single-line comment stripping
    rust_code = """
    // This is a comment
    fn main() {
        let x = 42; // line comment
    }
    """
    stripped = strip_comments(rust_code, "test.rs")
    assert "This is a comment" not in stripped
    assert "line comment" not in stripped
    assert "fn main()" in stripped
    assert "let x = 42;" in stripped


def test_strip_comments_rust_block():
    # Test block comment stripping (with nesting)
    rust_code = """
    /* Block comment
       with multiple lines */
    struct MyStruct {
        /* Nested /* block */ comment */
        y: u32,
    }
    """
    stripped = strip_comments(rust_code, "test.rs")
    assert "Block comment" not in stripped
    assert "Nested" not in stripped
    assert "struct MyStruct" in stripped
    assert "y: u32" in stripped


def test_strip_comments_rust_strings():
    # Test that comments inside string and char literals are not stripped
    rust_code = r"""
    fn test() {
        let s = "Keep this // comment string intact";
        let c = '/';
        let block_s = "Keep /* block */ comment";
    }
    """
    stripped = strip_comments(rust_code, "test.rs")
    assert "Keep this // comment string intact" in stripped
    assert "Keep /* block */ comment" in stripped


def test_strip_comments_lean():
    # Test single-line and block comment stripping in Lean
    lean_code = """
    -- This is a Lean single-line comment
    theorem my_theorem : 1 + 1 = 2 := by rfl
    /- This is a Lean block comment
       /- with nested block -/
       and more content -/
    def my_def := 42
    """
    stripped = strip_comments(lean_code, "test.lean")
    assert "Lean single-line comment" not in stripped
    assert "Lean block comment" not in stripped
    assert "nested block" not in stripped
    assert "theorem my_theorem" in stripped
    assert "def my_def" in stripped


def test_strip_comments_lean_strings():
    # Test string literal preservation in Lean
    lean_code = """
    def s := "Keep -- comment inside string"
    def multi := "Keep /- block -/ inside string"
    """
    stripped = strip_comments(lean_code, "test.lean")
    assert "Keep -- comment inside string" in stripped
    assert "Keep /- block -/ inside string" in stripped


def test_find_construct_rust():
    # Test construct finding in Rust
    code = """
    pub struct TargetStruct {
        x: u32,
    }
    fn target_fn() {}
    pub trait TargetTrait {}
    """
    stripped = strip_comments(code, "test.rs")
    assert find_construct(stripped, "TargetStruct", "test.rs") is True
    assert find_construct(stripped, "target_fn", "test.rs") is True
    assert find_construct(stripped, "TargetTrait", "test.rs") is True
    assert find_construct(stripped, "MissingStruct", "test.rs") is False


def test_find_construct_lean():
    # Test construct finding in Lean
    code = """
    theorem my_theorem : True := by trivial
    def my_definition := 123
    structure MyStructure where
      x : Nat
    """
    stripped = strip_comments(code, "test.lean")
    assert find_construct(stripped, "my_theorem", "test.lean") is True
    assert find_construct(stripped, "my_definition", "test.lean") is True
    assert find_construct(stripped, "MyStructure", "test.lean") is True
    assert find_construct(stripped, "missing_theorem", "test.lean") is False


def test_find_construct_namespace_qualified():
    # Test that namespaces are correctly handled by find_construct
    code = """
    namespace MyNamespace
    theorem my_theorem : True := by trivial
    end MyNamespace
    """
    stripped = strip_comments(code, "test.lean")
    assert find_construct(stripped, "MyNamespace.my_theorem", "test.lean") is True


def test_find_construct_commented_out_ignored():
    # Verify that commented-out constructs are ignored
    code = """
    // fn commented_out_rust() {}
    /*
    struct CommentedStruct {}
    */
    -- def commented_out_lean := 1
    /-
    theorem commented_theorem : True
    -/
    """
    stripped_rs = strip_comments(code, "test.rs")
    stripped_lean = strip_comments(code, "test.lean")

    assert find_construct(stripped_rs, "commented_out_rust", "test.rs") is False
    assert find_construct(stripped_rs, "CommentedStruct", "test.rs") is False
    assert find_construct(stripped_lean, "commented_out_lean", "test.lean") is False
    assert find_construct(stripped_lean, "commented_theorem", "test.lean") is False


def test_find_leaf_parameters():
    mock_data = {
        "top_param": 42,
        "nested": {
            "justification": "This should be ignored",
            "is_axiomatic": True,
            "citation": {"author": "Someone"},
            "valid_leaf": 3.14,
            "nested_again": {"inner_list": [1, 2, 3]},
        },
        "description": "Ignore this",
    }
    extracted = find_leaf_parameters(mock_data)
    assert "top_param" in extracted
    assert "nested.valid_leaf" in extracted
    assert "nested.nested_again.inner_list" in extracted
    assert "nested.justification" not in extracted
    assert "nested.is_axiomatic" not in extracted
    assert "nested.citation.author" not in extracted
    assert "description" not in extracted


def test_get_parameter_candidates():
    candidates = get_parameter_candidates("search_bounds.pollard_rho.batch_size")
    assert "search_bounds.pollard_rho.batch_size" in candidates
    assert "search_bounds_pollard_rho_batch_size" in candidates
    assert "pollard_rho_batch_size" in candidates
    assert "batch_size" in candidates
    assert "POLLARD_RHO_BATCH_SIZE" in candidates
    assert "BATCH_SIZE" in candidates


def test_is_parameter_documented():
    import textwrap

    doc_content = textwrap.dedent("""
        # Document
        We can document standard configurations here.
        
        ### POLLARD_RHO_BATCH_SIZE
        Description of batch size.
        
        Other parameter: `sieve_limit`.
        Some bold param: **max_exponent**.
        
        - active_prime_slots: a telemetry option.
    """)
    assert is_parameter_documented(doc_content, ["POLLARD_RHO_BATCH_SIZE"]) is True
    assert is_parameter_documented(doc_content, ["sieve_limit"]) is True
    assert is_parameter_documented(doc_content, ["max_exponent"]) is True
    assert is_parameter_documented(doc_content, ["active_prime_slots"]) is True
    assert is_parameter_documented(doc_content, ["missing_param"]) is False


def test_superscripts_mapping():
    assert SUPERSCRIPTS["⁰"] == "0"
    assert SUPERSCRIPTS["⁴"] == "4"
    assert SUPERSCRIPTS["³"] == "3"


def test_is_backtick_valid():
    repo_paths = {"sieve.rs", "ManifestConstants.lean", "README.md"}
    code_constructs = {"my_theorem", "my_fn"}
    valid_params = {"target_max_log10", "pollard_rho_batch_size"}

    # Common technology/names from SAFE_COMMON_WORDS
    assert is_backtick_valid("rust", repo_paths, code_constructs, valid_params) is True
    assert is_backtick_valid("lean", repo_paths, code_constructs, valid_params) is True
    assert is_backtick_valid("rayon", repo_paths, code_constructs, valid_params) is True

    # Real file references
    assert (
        is_backtick_valid("sieve.rs", repo_paths, code_constructs, valid_params) is True
    )
    assert (
        is_backtick_valid(
            "ManifestConstants.lean", repo_paths, code_constructs, valid_params
        )
        is True
    )

    # Active code constructs
    assert (
        is_backtick_valid("my_theorem", repo_paths, code_constructs, valid_params)
        is True
    )
    assert is_backtick_valid("my_fn", repo_paths, code_constructs, valid_params) is True

    # Active parameters
    assert (
        is_backtick_valid("target_max_log10", repo_paths, code_constructs, valid_params)
        is True
    )

    # Trailing slashes or attributes/keywords
    assert (
        is_backtick_valid("UALBF/Pure/", {"UALBF/Pure"}, code_constructs, valid_params)
        is True
    )
    assert (
        is_backtick_valid("@[export]", repo_paths, code_constructs, valid_params)
        is True
    )
    assert (
        is_backtick_valid("#[cfg(test)]", repo_paths, code_constructs, valid_params)
        is True
    )
    assert is_backtick_valid("def", repo_paths, code_constructs, valid_params) is True

    # Broken/invalid reference
    assert (
        is_backtick_valid("broken_ref", repo_paths, code_constructs, valid_params)
        is False
    )


def test_extract_backticks_with_lines(tmp_path):
    temp_file = tmp_path / "test_doc.md"
    temp_file.write_text("""
    # Header
    This is `item1` and `item2` here.
    ```rust
    ignore `item3` inside code block
    ```
    Another line with `item4`.
    """)
    items = extract_backticks_with_lines(str(temp_file))
    extracted_names = [item[0] for item in items]
    assert "item1" in extracted_names
    assert "item2" in extracted_names
    assert "item3" not in extracted_names
    assert "item4" in extracted_names


def test_extract_fqns_nested_namespaces():
    from verify_metadata import extract_fqns_from_lean_content

    lean_code = """
    namespace Outer
    namespace Inner
    def val := 1
    theorem my_thm : 1 = 1 := by rfl
    end Inner
    def top_val := 2
    end Outer
    def root_val := 3
    """
    fqns = extract_fqns_from_lean_content(lean_code)
    assert "Outer.Inner.val" in fqns
    assert "Outer.Inner.my_thm" in fqns
    assert "Outer.top_val" in fqns
    assert "root_val" in fqns
    assert "val" not in fqns


def test_find_construct_strict_and_fallback():
    lean_code = """
    namespace MyNamespace
    def my_theorem := True
    end MyNamespace
    """
    assert find_construct(lean_code, "MyNamespace.my_theorem", "test.lean") is True
    # Bypasses leaf-level fallback completely for qualified references containing '.'
    assert find_construct(lean_code, "WrongNamespace.my_theorem", "test.lean") is False
    # Preserves fallback for unqualified references
    assert find_construct(lean_code, "my_theorem", "test.lean") is True


def test_check_documentation_fqn_rules(tmp_path):
    import json
    from unittest.mock import patch, mock_open
    from auditor import check_documentation

    doc_path = tmp_path / "README.md"
    doc_path.write_text("""
    This is `MyNamespace.my_theorem` which is valid.
    This is `unrelated_theorem` which is unqualified and valid because it's in a namespace.
    This is `WrongNamespace.my_theorem` which is invalid and must be flagged!
    This is `MyNamespace.deleted_theorem` which is invalid and must be flagged!
    """)

    with patch("auditor.CORE_THEOREMS", []), patch(
        "auditor.os.walk", return_value=[]
    ), patch("auditor.check_lean_environment", return_value=True):

        with patch(
            "auditor.CORE_THEOREMS",
            ["MyNamespace.my_theorem", "Outer.unrelated_theorem"],
        ):
            docs_manifest_content = json.dumps({"README.md": "authoritative"})

            original_open = open

            def custom_open(file, *args, **kwargs):
                if "docs_manifest.json" in str(file):
                    return mock_open(read_data=docs_manifest_content)()
                if "README.md" in str(file):
                    return original_open(doc_path, *args, **kwargs)
                return original_open(file, *args, **kwargs)

            with patch("builtins.open", custom_open), patch(
                "auditor.os.path.exists", return_value=True
            ):
                with patch("sys.stderr") as mock_stderr:
                    result = check_documentation({"verus_hashes": {}})

                    assert result is False

                    error_calls = [
                        call[0][0]
                        for call in mock_stderr.write.call_args_list
                        if call[0]
                    ]
                    full_error_output = "".join(error_calls)

                    assert (
                        "Invalid code symbol: 'WrongNamespace.my_theorem'"
                        in full_error_output
                    )
                    assert (
                        "Invalid code symbol: 'MyNamespace.deleted_theorem'"
                        in full_error_output
                    )
                    assert (
                        "Invalid code symbol: 'MyNamespace.my_theorem'"
                        not in full_error_output
                    )
                    assert (
                        "Invalid code symbol: 'unrelated_theorem'"
                        not in full_error_output
                    )


def test_path_normalization():
    from verify_metadata import normalize_repo_path, get_canonical_path_variants

    assert normalize_repo_path("./bounds_manifest.json") == "bounds_manifest.json"
    assert normalize_repo_path("ualbf-project/TUNING.md") == "ualbf-project/TUNING.md"

    variants = get_canonical_path_variants("bounds_manifest.json")
    assert "bounds_manifest.json" in variants
    assert "ualbf-project/bounds_manifest.json" in variants


def test_check_reverse_dependencies_missing_docs_fails():
    from verify_metadata import check_reverse_dependencies

    # Scenario: Modifying UALBF.lean without updating lean4-proofs/README.md and TCB.md
    modified = ["ualbf-project/lean4-proofs/UALBF.lean"]
    is_valid, errors, spec_modified = check_reverse_dependencies(modified)

    assert is_valid is False
    assert spec_modified is True
    assert len(errors) > 0
    full_error = " ".join(errors)
    assert "lean4-proofs/README.md" in full_error
    assert "TCB.md" in full_error


def test_check_reverse_dependencies_with_docs_passes():
    from verify_metadata import check_reverse_dependencies

    # Scenario: Modifying UALBF.lean WITH all mapped documentation files
    modified = [
        "ualbf-project/lean4-proofs/UALBF.lean",
        "ualbf-project/lean4-proofs/README.md",
        "ualbf-project/TCB.md",
    ]
    is_valid, errors, spec_modified = check_reverse_dependencies(modified)

    assert is_valid is True
    assert spec_modified is True
    assert len(errors) == 0


def test_check_reverse_dependencies_bounds_manifest_missing_docs_fails():
    from verify_metadata import check_reverse_dependencies

    # Scenario: Modifying bounds_manifest.json without TUNING.md / README.md
    modified = ["ualbf-project/bounds_manifest.json"]
    is_valid, errors, spec_modified = check_reverse_dependencies(modified)

    assert is_valid is False
    assert spec_modified is True
    assert len(errors) > 0
    full_error = " ".join(errors)
    assert "TUNING.md" in full_error


def test_check_reverse_dependencies_bounds_manifest_with_docs_passes():
    from verify_metadata import check_reverse_dependencies

    # Scenario: Modifying bounds_manifest.json WITH TUNING.md and README.md
    modified = [
        "ualbf-project/bounds_manifest.json",
        "ualbf-project/TUNING.md",
        "README.md",
    ]
    is_valid, errors, spec_modified = check_reverse_dependencies(modified)

    assert is_valid is True
    assert spec_modified is True
    assert len(errors) == 0


def test_check_reverse_dependencies_unrelated_files_passes():
    from verify_metadata import check_reverse_dependencies

    modified = ["ualbf-project/TODO.md"]
    is_valid, errors, spec_modified = check_reverse_dependencies(modified)

    assert is_valid is True
    assert spec_modified is False
    assert len(errors) == 0
