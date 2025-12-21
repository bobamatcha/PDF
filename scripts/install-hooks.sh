#!/bin/bash
# Install git hooks for this repository
# Run: ./scripts/install-hooks.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

echo "Installing git hooks..."

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
cp "$SCRIPT_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"
echo "  ✓ Installed pre-commit hook"

# Make test-browser.sh executable
chmod +x "$SCRIPT_DIR/test-browser.sh"
echo "  ✓ Made test-browser.sh executable"

echo ""
echo "Done! The pre-commit hook will run:"
echo "  1. cargo fmt --check"
echo "  2. cargo clippy"
echo "  3. cargo test (unit tests)"
echo "  4. parity tests (config consistency)"
echo "  5. browser tests (starts servers automatically)"
echo ""
echo "To skip browser tests on a commit:"
echo "  SKIP_BROWSER_TESTS=1 git commit -m 'message'"
