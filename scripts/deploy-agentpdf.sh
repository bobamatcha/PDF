#!/bin/bash
set -e

# Deploy agentPDF.org to Cloudflare Pages
# Usage: ./scripts/deploy-agentpdf.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Deploying agentPDF.org ==="

# Build WASM
echo "Building WASM..."
cd "$PROJECT_ROOT/apps/agentpdf-web/wasm"
wasm-pack build --target web --release --out-dir ../www/pkg

# Deploy to Cloudflare Pages
echo "Deploying to Cloudflare Pages..."
cd "$PROJECT_ROOT/apps/agentpdf-web/www"

if command -v npx &> /dev/null; then
    npx wrangler pages deploy . --project-name agentpdf-org
else
    echo "Error: npx not found. Please install Node.js and npm."
    exit 1
fi

echo "=== agentPDF.org deployment complete ==="
