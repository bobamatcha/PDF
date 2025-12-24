#!/bin/bash
# Run tests for a specific app or crate
# Usage: ./scripts/test-app.sh <app-name>
#
# Examples:
#   ./scripts/test-app.sh pdfjoin      # Tests pdfjoin-core and pdfjoin-wasm
#   ./scripts/test-app.sh agentpdf     # Tests agentpdf-wasm
#   ./scripts/test-app.sh docsign      # Tests docsign-core and docsign-wasm
#   ./scripts/test-app.sh shared-pdf   # Tests shared-pdf crate
#   ./scripts/test-app.sh all          # Tests everything

set -e

cd "$(dirname "$0")/.."

APP="${1:-all}"

case "$APP" in
  pdfjoin)
    echo "Running tests for pdfjoin..."
    cargo test -p pdfjoin-core --all-features
    cargo test -p pdfjoin-wasm --all-features
    ;;
  agentpdf)
    echo "Running tests for agentpdf..."
    cargo test -p agentpdf-wasm --all-features
    ;;
  docsign)
    echo "Running tests for docsign..."
    cargo test -p docsign-core --all-features
    cargo test -p docsign-wasm --all-features
    ;;
  corpus)
    echo "Running tests for corpus..."
    cargo test -p corpus-core --all-features
    cargo test -p corpus-api --all-features
    ;;
  shared-pdf)
    echo "Running tests for shared-pdf..."
    cargo test -p shared-pdf --all-features
    ;;
  shared-types)
    echo "Running tests for shared-types..."
    cargo test -p shared-types --all-features
    ;;
  compliance)
    echo "Running tests for compliance-engine..."
    cargo test -p compliance-engine --all-features
    ;;
  typst)
    echo "Running tests for typst-engine..."
    cargo test -p typst-engine --all-features
    ;;
  all)
    echo "Running all tests..."
    cargo test --all-features --workspace
    ;;
  list)
    echo "Available apps/crates:"
    echo "  pdfjoin     - pdfjoin-core, pdfjoin-wasm"
    echo "  agentpdf    - agentpdf-wasm"
    echo "  docsign     - docsign-core, docsign-wasm"
    echo "  corpus      - corpus-core, corpus-api"
    echo "  shared-pdf  - shared-pdf"
    echo "  shared-types- shared-types"
    echo "  compliance  - compliance-engine"
    echo "  typst       - typst-engine"
    echo "  all         - all workspace crates"
    ;;
  *)
    echo "Unknown app: $APP"
    echo "Run './scripts/test-app.sh list' for available options"
    exit 1
    ;;
esac

echo "Done!"
