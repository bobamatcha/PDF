# Deployment Guide

Simple step-by-step instructions to deploy agentPDF.org and getsignatures.org.

## Prerequisites

You need:
- A Cloudflare account (free tier works)
- Node.js installed (for wrangler CLI)
- Rust installed (for building)
- Trunk installed: `cargo install trunk`

## Step 1: Install Wrangler CLI

```bash
npm install -g wrangler
```

## Step 2: Login to Cloudflare

```bash
wrangler login
```

This opens a browser. Click "Allow" to authorize.

## Step 3: Get Your Account ID

```bash
wrangler whoami
```

Copy your Account ID (looks like `a1b2c3d4e5f6...`). You'll need it later.

## Step 4: Create Cloudflare Pages Projects

Go to https://dash.cloudflare.com and:

1. Click **Pages** in the left sidebar
2. Click **Create a project** → **Direct Upload**
3. Name it `agentpdf-org` → Click **Create project**
4. Repeat for `getsignatures-org`

## Step 5: Create KV Namespaces (for docsign worker)

```bash
cd apps/docsign-web/worker

# Create the namespaces
wrangler kv:namespace create "SESSIONS"
wrangler kv:namespace create "RATE_LIMITS"
```

Copy the IDs from the output. They look like:
```
id = "abc123def456..."
```

## Step 6: Update wrangler.toml

Edit `apps/docsign-web/worker/wrangler.toml`:

```toml
[[kv_namespaces]]
binding = "SESSIONS"
id = "paste-your-sessions-id-here"

[[kv_namespaces]]
binding = "RATE_LIMITS"
id = "paste-your-rate-limits-id-here"
```

## Step 7: Set Worker Secrets

```bash
cd apps/docsign-web/worker

# Required: Get a Resend API key from https://resend.com
wrangler secret put RESEND_API_KEY
# Paste your Resend API key when prompted

# Optional: API protection (leave blank for open access)
wrangler secret put DOCSIGN_API_KEY
# Press Enter for no key (public access with rate limiting)
```

## Step 8: Deploy agentPDF.org

### Using Trunk (Recommended)

Trunk handles WASM compilation, bundling, and copies static files via `Trunk.toml` hooks.

```bash
cd apps/agentpdf-web

# Build for production (output: www/dist/)
trunk build --release

# Deploy to Cloudflare Pages
npx wrangler pages deploy www/dist --project-name agentpdf-org
```

**Note**: The Tampa landing page (`tampa.html`) is automatically copied to `www/dist/` via a post-build hook in `Trunk.toml`.

### Alternative: Manual wasm-pack Build

```bash
cd apps/agentpdf-web/wasm
wasm-pack build --target web --release --out-dir ../www/pkg

cd ../www
cp tampa.html dist/ 2>/dev/null || true  # Copy landing pages manually
npx wrangler pages deploy . --project-name agentpdf-org
```

## Step 9: Deploy getsignatures.org (Static Site)

### Using Trunk (Recommended)

```bash
cd apps/docsign-web

# Build for production (output: www/dist/)
trunk build --release

# Deploy to Cloudflare Pages
npx wrangler pages deploy www/dist --project-name getsignatures-org
```

### Alternative: Manual wasm-pack Build

```bash
cd apps/docsign-web/wasm
wasm-pack build --target web --release --out-dir ../www/pkg

cd ../www
npx wrangler pages deploy . --project-name getsignatures-org
```

## Step 10: Deploy getsignatures.org (Worker API)

```bash
cd apps/docsign-web/worker
cargo install worker-build  # First time only
worker-build --release
npx wrangler deploy
```

## Step 11: Configure Custom Domains (Optional)

In Cloudflare dashboard:

1. Go to **Pages** → your project
2. Click **Custom domains** tab
3. Add your domain (e.g., `agentpdf.org`)
4. Follow DNS instructions

For the worker API subdomain (`api.getsignatures.org`):
1. Go to **Workers & Pages** → your worker
2. Click **Triggers** tab
3. Add route: `api.getsignatures.org/*`

## Quick Deploy Scripts

After initial setup, use these for future deploys:

```bash
# Deploy agentPDF.org
./scripts/deploy-agentpdf.sh

# Deploy getsignatures.org (static + worker)
./scripts/deploy-docsign.sh
```

## GitHub Actions (Automatic Deploys)

To enable automatic deploys on push to main:

1. Go to your GitHub repo → Settings → Secrets
2. Add these secrets:
   - `CF_API_TOKEN`: Create at https://dash.cloudflare.com/profile/api-tokens
   - `CF_ACCOUNT_ID`: Your account ID from Step 3

Now every push to `main` will auto-deploy both sites.

## Troubleshooting

### "Project not found"
Make sure project names match exactly: `agentpdf-org` and `getsignatures-org`

### "KV namespace not found"
Run `wrangler kv:namespace list` to see your namespaces and verify IDs in wrangler.toml

### "Unauthorized"
Run `wrangler login` again

### Build fails with wasm-opt error
Already fixed - wasm-opt is disabled in Cargo.toml

### Emails not sending
Check that RESEND_API_KEY is set: `wrangler secret list`

### Trunk "Is a directory" error

**Cause**: `target` in `Trunk.toml` was pointing to the output directory instead of the input HTML file.

**Fix**: Ensure `Trunk.toml` has:
```toml
[build]
target = "www/index.html"   # Input HTML file (NOT "www/dist")
dist = "www/dist"           # Output directory
```

### Tampa landing page 404 after deploy

**Cause**: Static HTML files not copied to `www/dist/` during build.

**Fix**: Add a post-build hook in `Trunk.toml`:
```toml
[[hooks]]
stage = "post_build"
command = "sh"
command_arguments = ["-c", "cp www/tampa.html www/dist/ 2>/dev/null || true"]
```

### WASM not loading / stale build

**Fix**: Clean and rebuild:
```bash
rm -rf www/dist www/pkg
trunk build --release
```

## Verify Deployment

After deploying:

| URL | What to Check |
|-----|---------------|
| `agentpdf.org` | PDF upload, compliance check, template generation |
| `agentpdf.org/tampa.html` | Tampa landing page loads, 4 compliance tools visible |
| `getsignatures.org` | 4-step signing wizard works |

### Quick Verification Steps

1. **agentPDF.org**: Click "Use a Template" → Select "florida_lease" → Verify 11 optional fields including HB 615 and flood disclosure
2. **agentPDF.org/tampa.html**: Verify landing page loads with Flood Disclosure, Email Consent, and other cards
3. **getsignatures.org**: Upload PDF → Add recipient → Place signature → Complete signing flow
