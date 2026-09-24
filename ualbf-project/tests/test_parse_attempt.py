import json
import os
import sys
from unittest.mock import MagicMock, patch

# Ensure scripts directory is on sys.path so parse_attempt can be imported
SCRIPTS_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "scripts"))
if SCRIPTS_DIR not in sys.path:
    sys.path.insert(0, SCRIPTS_DIR)

import parse_attempt


def test_get_repository():
    with patch.dict(
        os.environ, {"GITHUB_REPOSITORY": "fderuiter/quasiperfect"}, clear=True
    ):
        assert parse_attempt.get_repository() == "fderuiter/quasiperfect"

    with patch.dict(os.environ, {"REPO": "owner/repo-name"}, clear=True):
        assert parse_attempt.get_repository() == "owner/repo-name"

    with patch.dict(os.environ, {"GITHUB_REPOSITORY": "invalid_no_slash"}, clear=True):
        assert parse_attempt.get_repository() is None

    with patch.dict(os.environ, {}, clear=True):
        assert parse_attempt.get_repository() is None


def test_get_issue_number_direct_env():
    with patch.dict(os.environ, {"ISSUE_NUMBER": "123"}, clear=True):
        assert parse_attempt.get_issue_number() == "123"

    with patch.dict(os.environ, {"PR_NUMBER": "456"}, clear=True):
        assert parse_attempt.get_issue_number() == "456"

    with patch.dict(os.environ, {"ISSUE_NUMBER": "abc"}, clear=True):
        assert parse_attempt.get_issue_number() is None


def test_get_issue_number_event_path(tmp_path):
    event_file = tmp_path / "event.json"

    # Issue event payload
    event_file.write_text(json.dumps({"issue": {"number": 789}}))
    with patch.dict(os.environ, {"GITHUB_EVENT_PATH": str(event_file)}, clear=True):
        assert parse_attempt.get_issue_number() == "789"

    # PR event payload
    event_file.write_text(json.dumps({"pull_request": {"number": 101}}))
    with patch.dict(os.environ, {"GITHUB_EVENT_PATH": str(event_file)}, clear=True):
        assert parse_attempt.get_issue_number() == "101"

    # Generic number payload
    event_file.write_text(json.dumps({"number": 202}))
    with patch.dict(os.environ, {"GITHUB_EVENT_PATH": str(event_file)}, clear=True):
        assert parse_attempt.get_issue_number() == "202"

    # Non-existent event file
    with patch.dict(
        os.environ,
        {"GITHUB_EVENT_PATH": str(tmp_path / "non_existent.json")},
        clear=True,
    ):
        assert parse_attempt.get_issue_number() is None


def test_get_parsed_attempt_missing_context():
    with patch.dict(os.environ, {}, clear=True):
        assert parse_attempt.get_parsed_attempt() is None


def test_get_parsed_attempt_success_and_auth_header():
    mock_comments = [
        {"body": "This is a regular comment"},
        {"body": "Fix applied [CI/CD Fix Attempt 2]"},
        {"body": "Another fix [CI/CD Fix Attempt 5]"},
        {"body": "Outdated fix [CI/CD Fix Attempt 3]"},
    ]
    response_body = json.dumps(mock_comments).encode("utf-8")

    captured_requests = []

    def mock_urlopen(req, timeout=10):
        captured_requests.append(req)
        mock_response = MagicMock()
        mock_response.status = 200
        mock_response.read.return_value = response_body
        mock_response.__enter__.return_value = mock_response
        return mock_response

    env_vars = {
        "GITHUB_REPOSITORY": "testowner/testrepo",
        "ISSUE_NUMBER": "42",
        "GITHUB_TOKEN": "secret_token_123",
    }
    with patch.dict(os.environ, env_vars, clear=True):
        with patch("urllib.request.urlopen", side_effect=mock_urlopen):
            attempt = parse_attempt.get_parsed_attempt()

    assert attempt == "5"
    assert len(captured_requests) == 5
    first_req = captured_requests[0]
    assert first_req.full_url.startswith(
        "https://api.github.com/repos/testowner/testrepo/issues/42/comments"
    )
    assert first_req.headers.get("Authorization") == "token secret_token_123"


def test_get_parsed_attempt_http_error_fallback():
    env_vars = {
        "GITHUB_REPOSITORY": "testowner/testrepo",
        "ISSUE_NUMBER": "42",
    }
    with patch.dict(os.environ, env_vars, clear=True):
        with patch("urllib.request.urlopen", side_effect=Exception("Network error")):
            attempt = parse_attempt.get_parsed_attempt()

    assert attempt is None


def test_get_fallback_attempt(tmp_path):
    comment_file = tmp_path / "comment_count.txt"

    # Test file with content
    comment_file.write_text("  7 \n")
    assert parse_attempt.get_fallback_attempt(str(comment_file)) == "7"

    # Test empty file
    comment_file.write_text("   \n")
    assert parse_attempt.get_fallback_attempt(str(comment_file)) == "3"

    # Test missing file
    assert parse_attempt.get_fallback_attempt(str(tmp_path / "missing.txt")) == "3"
