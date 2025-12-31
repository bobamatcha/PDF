#!/bin/bash
# verify-dist.sh - Regression test for Trunk build output
# Ensures all required files are present in www/dist after build
#
# Usage: ./scripts/verify-dist.sh [--build]
#   --build: Run trunk build --release before verification

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$APP_DIR/www/dist"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build if requested
if [[ "$1" == "--build" ]]; then
    echo -e "${YELLOW}Building with trunk...${NC}"
    cd "$APP_DIR"
    trunk build --release
    echo ""
fi

echo "Verifying dist directory: $DIST_DIR"
echo "================================================"

# Track failures
FAILED=0

# Function to check if file exists
check_file() {
    local file="$1"
    local desc="$2"
    if [[ -f "$DIST_DIR/$file" ]]; then
        local size=$(du -h "$DIST_DIR/$file" | cut -f1)
        echo -e "${GREEN}✓${NC} $file ($size) - $desc"
    else
        echo -e "${RED}✗${NC} $file - MISSING - $desc"
        FAILED=$((FAILED + 1))
    fi
}

# Function to check if directory exists
check_dir() {
    local dir="$1"
    local desc="$2"
    if [[ -d "$DIST_DIR/$dir" ]]; then
        local count=$(find "$DIST_DIR/$dir" -type f | wc -l | tr -d ' ')
        echo -e "${GREEN}✓${NC} $dir/ ($count files) - $desc"
    else
        echo -e "${RED}✗${NC} $dir/ - MISSING - $desc"
        FAILED=$((FAILED + 1))
    fi
}

# Function to check WASM files exist (with hash in filename)
check_wasm() {
    local count=$(find "$DIST_DIR" -maxdepth 1 -name "*.wasm" | wc -l | tr -d ' ')
    if [[ "$count" -gt 0 ]]; then
        local wasm_file=$(find "$DIST_DIR" -maxdepth 1 -name "*.wasm" | head -1)
        local size=$(du -h "$wasm_file" | cut -f1)
        echo -e "${GREEN}✓${NC} *.wasm ($size) - WASM module"
    else
        echo -e "${RED}✗${NC} *.wasm - MISSING - WASM module"
        FAILED=$((FAILED + 1))
    fi

    # Also check for JS bindings
    count=$(find "$DIST_DIR" -maxdepth 1 -name "docsign-wasm-*.js" | wc -l | tr -d ' ')
    if [[ "$count" -gt 0 ]]; then
        echo -e "${GREEN}✓${NC} docsign-wasm-*.js - WASM JS bindings"
    else
        echo -e "${RED}✗${NC} docsign-wasm-*.js - MISSING - WASM JS bindings"
        FAILED=$((FAILED + 1))
    fi
}

echo ""
echo "Required files for signing flow:"
echo "---------------------------------"

# HTML pages
check_file "index.html" "Main sender page"
check_file "sign.html" "Recipient signing page"

# CSS
check_file "geriatric.css" "Accessibility CSS (60px buttons, 18px fonts)"

# TypeScript bundle
check_dir "js" "TypeScript bundle directory"
check_file "js/bundle.js" "TypeScript bundle"

# WASM
check_wasm

# JavaScript files used by sign.html
check_file "sign.js" "Signing page logic"
check_file "signature-pad.js" "Signature capture"
check_file "sw.js" "Service worker for offline"

echo ""
echo "Optional files:"
echo "---------------"

# Optional files (don't fail if missing)
for file in "ltv-timestamp.js" "guided-flow.js" "js/bundle.js.map"; do
    if [[ -f "$DIST_DIR/$file" ]]; then
        size=$(du -h "$DIST_DIR/$file" 2>/dev/null | cut -f1)
        echo -e "${GREEN}✓${NC} $file ($size)"
    else
        echo -e "${YELLOW}○${NC} $file (optional, not present)"
    fi
done

echo ""
echo "================================================"

if [[ $FAILED -eq 0 ]]; then
    echo -e "${GREEN}All required files present!${NC}"

    # Print total size
    total_size=$(du -sh "$DIST_DIR" | cut -f1)
    echo "Total dist size: $total_size"
    exit 0
else
    echo -e "${RED}$FAILED required file(s) missing!${NC}"
    exit 1
fi
