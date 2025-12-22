#!/bin/bash
# Check that all HTML files in www/ are properly configured for trunk build
# This prevents the common mistake of HTML files not being copied to dist/
#
# Usage: ./scripts/check-trunk-html.sh [app-dir]
# Example: ./scripts/check-trunk-html.sh apps/docsign-web

set -e

APP_DIR="${1:-.}"
WWW_DIR="$APP_DIR/www"
INDEX_HTML="$WWW_DIR/index.html"

if [ ! -f "$INDEX_HTML" ]; then
    echo "ERROR: $INDEX_HTML not found"
    exit 1
fi

MISSING=()

for html_file in "$WWW_DIR"/*.html; do
    [ -e "$html_file" ] || continue

    filename=$(basename "$html_file")

    # Skip index.html - it's the trunk target
    if [ "$filename" = "index.html" ]; then
        continue
    fi

    # Check if this file has a copy-file link in index.html
    if ! grep -q "data-trunk.*rel=\"copy-file\".*href=\"$filename\"" "$INDEX_HTML"; then
        MISSING+=("$filename")
    fi
done

if [ ${#MISSING[@]} -gt 0 ]; then
    echo "ERROR: The following HTML files are NOT configured to be copied by trunk:"
    for f in "${MISSING[@]}"; do
        echo "  - $f"
    done
    echo ""
    echo "FIX: Add these lines to $INDEX_HTML inside <head>:"
    for f in "${MISSING[@]}"; do
        echo "  <link data-trunk rel=\"copy-file\" href=\"$f\" />"
    done
    echo ""
    echo "See https://trunkrs.dev/assets/ for documentation on trunk asset copying."
    exit 1
fi

echo "✓ All HTML files in $WWW_DIR are properly configured for trunk build"
exit 0
