#!/usr/bin/env bash
set -euo pipefail

# ualbf-project/scripts/check-code-quality.sh
# Automated Repository Quality Check Script
# Scans Lean files for prompt breakdown tags and synthetic tactic macros.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -d "$SCRIPT_DIR/../lean4-proofs" ]; then
    LEAN_DIR="$SCRIPT_DIR/../lean4-proofs"
elif [ -d "$SCRIPT_DIR/lean4-proofs" ]; then
    LEAN_DIR="$SCRIPT_DIR/lean4-proofs"
else
    LEAN_DIR="."
fi

echo "Scanning Lean source files in: $LEAN_DIR"

VIOLATIONS=0

# Pattern 1: Prompt breakdown tags & prompt artifact patterns
PROMPT_TAG_PATTERN='\b5[a-z]\b|\b5[a-z]_[0-9]+\b|\[source[0-9]+\]'

# Pattern 2: Synthetic tactic macro definitions in proof files
TACTIC_MACRO_PATTERN='^\s*(macro|elab)\s+"[^"]+"'

while IFS= read -r file; do
    filename=$(basename "$file")

    # Check for prompt breakdown tags
    if grep -n -E "$PROMPT_TAG_PATTERN" "$file" > /dev/null 2>&1; then
        echo "[QUALITY ERROR] Prompt artifact/breakdown tag detected in $file:"
        grep -n -E "$PROMPT_TAG_PATTERN" "$file" | sed 's/^/  /'
        VIOLATIONS=$((VIOLATIONS + 1))
    fi

    # Check for synthetic tactic macros in proof files
    if [ "$filename" != "FFI_generated.lean" ]; then
        if grep -n -E "$TACTIC_MACRO_PATTERN" "$file" > /dev/null 2>&1; then
            echo "[QUALITY ERROR] Tactic macro definition detected in $file:"
            grep -n -E "$TACTIC_MACRO_PATTERN" "$file" | sed 's/^/  /'
            VIOLATIONS=$((VIOLATIONS + 1))
        fi
    fi
done < <(find "$LEAN_DIR" -type d -name ".lake" -prune -o -type f -name "*.lean" -print)

if [ $VIOLATIONS -ne 0 ]; then
    echo "=================================================="
    echo "FAILED: Found $VIOLATIONS code quality violation(s) in Lean proof files."
    echo "=================================================="
    exit 1
fi

echo "=================================================="
echo "PASSED: All Lean proof files satisfied code quality standards."
echo "=================================================="
exit 0
