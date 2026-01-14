#!/bin/bash
set -e

# Deploy getsignatures.org
#
# Frontend: getsigs (Cloudflare Pages) -> www/dist
# API: docsign-worker-production -> api.getsignatures.org

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== getsignatures.org deployment ==="

# Step 1: Build frontend
echo ""
echo "[1/4] Building frontend with Trunk..."
cd "$PROJECT_ROOT/apps/docsign-web"
trunk build --release

# Step 2: Verify build (catches the "deployed source instead of dist" bug)
echo ""
echo "[2/4] Verifying build output..."
./scripts/verify-dist.sh

# Step 3: Deploy frontend to Cloudflare Pages
echo ""
echo "[3/4] Deploying frontend to getsigs..."
npx wrangler pages deploy www/dist --project-name getsigs

# Step 4: Deploy API worker
echo ""
echo "[4/4] Deploying API worker..."
cd "$PROJECT_ROOT/apps/docsign-web/worker"
npx wrangler deploy --env production

echo ""
echo "=== Deployment complete ==="
echo "Frontend: https://getsignatures.org"
echo "API: https://api.getsignatures.org"
