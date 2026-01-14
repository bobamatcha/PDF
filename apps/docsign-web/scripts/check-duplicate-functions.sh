#!/bin/bash
# Check for duplicate function declarations in JS/HTML files
# This prevents deploying code that will fail at runtime with:
# "SyntaxError: Cannot declare a function that shadows a let/const/class/function variable"
#
# Note: This checks for duplicates at the same indentation level (likely same scope).
# Nested functions in different parent functions are valid JavaScript.

set -e

cd "$(dirname "$0")/.."

echo "Checking for duplicate function declarations..."

ERRORS=0

# Check each file that contains inline JavaScript
for file in www/index.html www/sign.html www/auth.html www/admin.html www/sign.js www/guided-flow.js; do
    if [ ! -f "$file" ]; then
        continue
    fi

    # Extract function declarations that are at the TOP level (minimal indentation)
    # This matches "function name(" with 0-8 spaces of indentation (top-level in script blocks)
    # Functions with more indentation are nested and can have duplicate names in different scopes
    DUPLICATES=$(grep -E '^[ ]{0,8}function [a-zA-Z_][a-zA-Z0-9_]*\s*\(' "$file" 2>/dev/null | \
        sed 's/^[ ]*//' | \
        sed 's/function //' | sed 's/\s*(.*//' | \
        sort | uniq -d)

    if [ -n "$DUPLICATES" ]; then
        echo ""
        echo "ERROR: Duplicate top-level function declarations in $file:"
        for func in $DUPLICATES; do
            echo "  - $func (declared multiple times at top level)"
            # Show line numbers for top-level declarations only
            grep -n "^[ ]\{0,8\}function $func\s*(" "$file" | sed 's/^/    line /'
        done
        ERRORS=$((ERRORS + 1))
    fi
done

if [ $ERRORS -gt 0 ]; then
    echo ""
    echo "FAILED: Found duplicate function declarations in $ERRORS file(s)"
    echo "Fix: Remove duplicate function declarations before deploying"
    exit 1
fi

echo "OK: No duplicate top-level function declarations found"
exit 0
