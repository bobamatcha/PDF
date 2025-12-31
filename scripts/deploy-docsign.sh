#!/bin/bash
set -e

# Deploy getsignatures.org to Cloudflare Pages + Worker
# Usage: ./scripts/deploy-docsign.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Deploying getsignatures.org ==="

# Build WASM
echo "Building WASM..."
cd "$PROJECT_ROOT/apps/docsign-web/wasm"
wasm-pack build --target web --release --out-dir ../www/pkg

# Deploy static site to Cloudflare Pages
echo "Deploying static site to Cloudflare Pages..."
cd "$PROJECT_ROOT/apps/docsign-web/www"

if command -v npx &> /dev/null; then
    npx wrangler pages deploy . --project-name getsignatures-org
else
    echo "Error: npx not found. Please install Node.js and npm."
    exit 1
fi

# Build and deploy Worker
echo "Building Worker..."
cd "$PROJECT_ROOT/apps/docsign-web/worker"

if ! command -v worker-build &> /dev/null; then
    echo "Installing worker-build..."
    cargo install worker-build
fi

worker-build --release

echo "Deploying Worker..."
npx wrangler deploy --env production

echo "=== getsignatures.org deployment complete ==="
echo ""
echo "Don't forget to set Worker secrets:"
echo "  wrangler secret put DOCSIGN_API_KEY  (optional, leave unset for open access)"
echo ""
echo "Note: Email sending is handled by email-proxy Lambda (not Resend)"
