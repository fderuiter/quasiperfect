import sys
import os
from pathlib import Path
import tempfile
import subprocess

# Add scripts directory to sys.path
sys.path.append(str(Path(__file__).parent.parent / "scripts"))
from parse_latex_log import parse_log, unwrap_log_content


def test_unwrap_log_content():
    # Simulated wrapped log content where first line has exactly 79 characters
    content = (
        "This is a line that has exactly 79 characters of content and should be wrapped!\n"
        "onto the next line of output as a single merged line because it was split."
    )
    # The first line has length 79 (without newline)
    first_line = content.splitlines()[0]
    assert len(first_line) == 79

    unwrapped = unwrap_log_content(content)
    # They should be merged without newline
    assert (
        unwrapped
        == "This is a line that has exactly 79 characters of content and should be wrapped!onto the next line of output as a single merged line because it was split."
    )


def test_parse_log_fully_resolved():
    mock_log = """
This is a standard LaTeX log file with no errors or warnings.
Output written on main.pdf (10 pages, 305890 bytes).
Transcript written on main.log.
"""
    refs, cits = parse_log(mock_log)
    assert len(refs) == 0
    assert len(cits) == 0


def test_parse_log_third_party_warnings_ignored():
    mock_log = """
LaTeX Warning: Font shape `OT1/cmss/m/it' in size <10> not available
(Font)              size <10.95> substituted on input line 50.
LaTeX Warning: Underfull \\hbox (badness 10000) in paragraph at lines 12--15
LaTeX Warning: Overfull \\vbox (2.5pt too high) has occurred while \\output is active
Package microtype Warning: Character `a' is not trackable.
"""
    refs, cits = parse_log(mock_log)
    assert len(refs) == 0
    assert len(cits) == 0


def test_parse_log_undefined_references():
    mock_log = """
LaTeX Warning: Reference `sec:intro' on page 3 undefined on input line 45.
LaTeX Warning: Reference 'fig:results' undefined.
"""
    refs, cits = parse_log(mock_log)
    assert refs == ["fig:results", "sec:intro"]
    assert len(cits) == 0


def test_parse_log_undefined_citations():
    mock_log = """
LaTeX Warning: Citation `smith99' on page 10 undefined on input line 120.
Package natbib Warning: Citation 'jones2020' on page 15 undefined.
Package biblatex Warning: Citation 'doe2021' undefined.
"""
    refs, cits = parse_log(mock_log)
    assert len(refs) == 0
    assert cits == ["doe2021", "jones2020", "smith99"]


def test_parse_log_wrapped_keys():
    # Simulating a citation warning where the warning and key wrap at column 79
    # "LaTeX Warning: Citation `very_long_citation_key_that_spans_multiple_lines_to_be" is exactly 79 characters.
    line1 = "LaTeX Warning: Citation `very_long_citation_key_that_spans_multiple_lines_to_be"
    line2 = "test_the_unwrap_feature' on page 5 undefined."
    mock_log = f"{line1}\n{line2}"

    assert len(line1) == 79

    refs, cits = parse_log(mock_log)
    assert len(refs) == 0
    assert cits == [
        "very_long_citation_key_that_spans_multiple_lines_to_betest_the_unwrap_feature"
    ]


def test_parse_log_multiple_keys_in_one_warning():
    mock_log = """
LaTeX Warning: Citation `key1, key2, key3' on page 2 undefined on input line 5.
"""
    refs, cits = parse_log(mock_log)
    assert len(refs) == 0
    assert cits == ["key1", "key2", "key3"]


def test_script_execution_success():
    # Test script returning 0 for a clean log
    script_path = str(Path(__file__).parent.parent / "scripts" / "parse_latex_log.py")
    with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
        f.write("A clean LaTeX log file with no warnings.\n")
        log_name = f.name
    try:
        res = subprocess.run(
            [sys.executable, script_path, log_name], capture_output=True, text=True
        )
        assert res.returncode == 0
        assert "LaTeX LOG VALIDATION SUCCESS" in res.stdout
    finally:
        os.remove(log_name)


def test_script_execution_failure():
    # Test script returning 1 for a log with warnings
    script_path = str(Path(__file__).parent.parent / "scripts" / "parse_latex_log.py")
    with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
        f.write(
            "LaTeX Warning: Reference `sec:broken' on page 4 undefined on input line 56.\n"
        )
        log_name = f.name
    try:
        res = subprocess.run(
            [sys.executable, script_path, log_name], capture_output=True, text=True
        )
        assert res.returncode == 1
        assert "LaTeX LOG VALIDATION FAILURE" in res.stdout
        assert "sec:broken" in res.stdout
    finally:
        os.remove(log_name)
