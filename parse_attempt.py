import urllib.request
import json
import re

def get_parsed_attempt():
    attempts = []
    # Fetch both pages of comments (up to 200 comments)
    for page in [1, 2]:
        url = f"https://api.github.com/repos/fderuiter/quasipolynomials/issues/401/comments?per_page=100&page={page}"
        req = urllib.request.Request(
            url,
            headers={"User-Agent": "Mozilla/5.0"}
        )
        try:
            with urllib.request.urlopen(req) as response:
                data = json.loads(response.read().decode('utf-8'))
                for comment in data:
                    body = comment.get("body", "")
                    match = re.search(r"\[CI/CD Fix Attempt\s+(\d+)\]", body)
                    if match:
                        attempts.append(int(match.group(1)))
        except Exception:
            pass
            
    if attempts:
        return str(max(attempts))
    return None

if __name__ == "__main__":
    attempt = get_parsed_attempt()
    if attempt:
        print(attempt)
    else:
        # Fallback to the value from comment_count.txt
        try:
            with open("/app/comment_count.txt", "r") as f:
                val = f.read().strip()
                if val:
                    print(val)
                else:
                    print("3")
        except Exception:
            print("3")
