#!/bin/bash
# Test: Verify all HTML files from www/ are present in www/dist/ after build
# This prevents 404 errors in production when new HTML pages are added
#
# Run this after `trunk build --release` to verify the build is complete

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
WWW_DIR="$APP_DIR/www"
DIST_DIR="$APP_DIR/www/dist"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "Verifying all HTML files are in dist..."
echo "Source: $WWW_DIR"
echo "Dist:   $DIST_DIR"
echo ""

if [ ! -d "$DIST_DIR" ]; then
    echo -e "${RED}FAIL: dist directory does not exist. Run 'trunk build --release' first.${NC}"
    exit 1
fi

MISSING=0
FOUND=0

for html_file in "$WWW_DIR"/*.html; do
    filename=$(basename "$html_file")
    if [ -f "$DIST_DIR/$filename" ]; then
        echo -e "${GREEN}✓${NC} $filename"
        ((FOUND++))
    else
        echo -e "${RED}✗ MISSING: $filename${NC}"
        ((MISSING++))
    fi
done

echo ""
echo "Found: $FOUND, Missing: $MISSING"

if [ $MISSING -gt 0 ]; then
    echo ""
    echo -e "${RED}FAIL: $MISSING HTML file(s) missing from dist/${NC}"
    echo "Fix: Add [[copy]] entries in Trunk.toml for missing files"
    exit 1
else
    echo -e "${GREEN}PASS: All HTML files present in dist/${NC}"
    exit 0
fi
