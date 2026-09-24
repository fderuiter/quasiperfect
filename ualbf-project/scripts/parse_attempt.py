import json
import os
import re
import urllib.request


def get_repository():
    """Extract owner/repo string from GITHUB_REPOSITORY or REPO environment variables."""
    repo = os.environ.get("GITHUB_REPOSITORY") or os.environ.get("REPO")
    if repo:
        repo = repo.strip()
        if "/" in repo and len(repo.split("/")) == 2 and all(repo.split("/")):
            return repo
    return None


def get_issue_number():
    """Extract PR/issue number from ISSUE_NUMBER, PR_NUMBER, or GITHUB_EVENT_PATH payload."""
    issue_num = os.environ.get("ISSUE_NUMBER") or os.environ.get("PR_NUMBER")
    if issue_num:
        issue_num_str = str(issue_num).strip()
        if issue_num_str.isdigit():
            return issue_num_str

    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if event_path and os.path.exists(event_path):
        try:
            with open(event_path, "r", encoding="utf-8") as f:
                event_data = json.load(f)
            num = None
            if isinstance(event_data, dict):
                num = (
                    event_data.get("issue", {}).get("number")
                    or event_data.get("pull_request", {}).get("number")
                    or event_data.get("number")
                )
            if num is not None:
                num_str = str(num).strip()
                if num_str.isdigit():
                    return num_str
        except Exception:
            pass

    return None


def get_parsed_attempt(repo=None, issue_number=None):
    """
    Dynamically queries GitHub comments for the resolved repo and issue_number.
    Returns the maximum attempt number found as a string, or None if missing env/API error.
    """
    if repo is None:
        repo = get_repository()
    if issue_number is None:
        issue_number = get_issue_number()

    if not repo or not issue_number:
        return None

    attempts = []
    # Fetch comments up to 5 pages (up to 500 comments)
    for page in [1, 2, 3, 4, 5]:
        url = (
            f"https://api.github.com/repos/{repo}/"
            f"issues/{issue_number}/comments?per_page=100&page={page}"
        )
        headers = {"User-Agent": "Mozilla/5.0"}
        token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
        if token:
            headers["Authorization"] = f"token {token}"

        req = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=10) as response:
                status = getattr(response, "status", 200)
                if status != 200:
                    break
                data = json.loads(response.read().decode("utf-8"))
                if not isinstance(data, list):
                    break
                for comment in data:
                    if isinstance(comment, dict):
                        body = comment.get("body", "")
                        match = re.search(r"\[CI/CD Fix Attempt\s+(\d+)\]", body)
                        if match:
                            attempts.append(int(match.group(1)))
        except Exception:
            pass

    if attempts:
        return str(max(attempts))
    return None


def get_fallback_attempt(file_path="/app/comment_count.txt", default="3"):
    """Reads attempt count from local fallback file or returns default."""
    try:
        if os.path.exists(file_path):
            with open(file_path, "r", encoding="utf-8") as f:
                val = f.read().strip()
                if val:
                    return val
    except Exception:
        pass
    return default


if __name__ == "__main__":
    attempt = get_parsed_attempt()
    if not attempt:
        attempt = get_fallback_attempt()
    print(attempt)
