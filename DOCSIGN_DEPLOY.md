# DocSign Deployment Guide (ELI5)

> **Target**: Deploy docsign-web to Cloudflare Pages + Workers
> **Domain**: getsignatures.org
> **Last Updated**: 2026-01-03

## Overview

DocSign has two parts:
1. **Frontend** (HTML/CSS/JS/WASM) → Cloudflare Pages
2. **Backend** (Rust Worker) → Cloudflare Workers

---

## Prerequisites

### 1. Install Required Tools

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wasm32 target for Rust
rustup target add wasm32-unknown-unknown

# Install Trunk (builds WASM frontend)
cargo install trunk

# Install wrangler (Cloudflare CLI)
npm install -g wrangler

# Login to Cloudflare
wrangler login
```

### 2. Get API Keys

You need these API keys before deploying:

| Key | Where to Get It | Purpose |
|-----|-----------------|---------|
| `RESEND_API_KEY` | [resend.com/api-keys](https://resend.com/api-keys) | Send emails |
| `JWT_SECRET` | Generate yourself (see below) | Sign auth tokens |
| `DOCSIGN_API_KEY` | Generate yourself (see below) | Protect internal APIs |

**Generate secrets:**
```bash
# Generate JWT_SECRET (32 random bytes, base64 encoded)
openssl rand -base64 32

# Generate DOCSIGN_API_KEY (same method)
openssl rand -base64 32
```

---

## Step-by-Step Deployment

### Step 1: Create KV Namespaces

KV namespaces are like databases. We need 4 of them:

```bash
cd apps/docsign-web/worker

# Create the namespaces (run each command)
wrangler kv namespace create SESSIONS
wrangler kv namespace create RATE_LIMITS
wrangler kv namespace create USERS
wrangler kv namespace create AUTH_SESSIONS

# You'll get output like:
# { binding = "SESSIONS", id = "abc123..." }
#
# Copy each ID into wrangler.toml (see Step 2)
```

### Step 2: Update wrangler.toml with Real IDs

Open `apps/docsign-web/worker/wrangler.toml` and replace the placeholder IDs:

```toml
[[kv_namespaces]]
binding = "SESSIONS"
id = "YOUR_SESSIONS_ID_HERE"  # <-- Replace this

[[kv_namespaces]]
binding = "RATE_LIMITS"
id = "YOUR_RATE_LIMITS_ID_HERE"  # <-- Replace this

[[kv_namespaces]]
binding = "USERS"
id = "YOUR_USERS_ID_HERE"  # <-- Replace this

[[kv_namespaces]]
binding = "AUTH_SESSIONS"
id = "YOUR_AUTH_SESSIONS_ID_HERE"  # <-- Replace this
```

### Step 3: Set Secrets

Secrets are like environment variables but encrypted:

```bash
cd apps/docsign-web/worker

# Set the Resend API key
wrangler secret put RESEND_API_KEY
# Paste your key when prompted, press Enter

# Set the JWT secret
wrangler secret put JWT_SECRET
# Paste your generated secret, press Enter

# Set the API key for internal endpoints
wrangler secret put DOCSIGN_API_KEY
# Paste your generated key, press Enter
```

### Step 4: Deploy the Worker (Backend)

```bash
cd apps/docsign-web/worker

# Deploy to Cloudflare
wrangler deploy

# You'll get a URL like:
# https://docsign-worker.YOUR_SUBDOMAIN.workers.dev
```

**Note the worker URL** - you'll need it for the frontend.

### Step 5: Build the Frontend

```bash
cd apps/docsign-web

# Install npm dependencies
npm install

# Build TypeScript
npm run build

# Build WASM + bundle everything
trunk build --release
```

This creates a `www/dist/` folder with all the files.

### Step 6: Deploy Frontend to Cloudflare Pages

**Option A: Via Dashboard (Easiest)**

1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com) → Pages
2. Create new project → "Upload assets"
3. Upload the `apps/docsign-web/www/dist/` folder
4. Set custom domain to `getsignatures.org`

**Option B: Via CLI**

```bash
cd apps/docsign-web

# Deploy to Pages (first time creates the project)
wrangler pages deploy www/dist --project-name=getsignatures
```

### Step 7: Configure DNS

In Cloudflare Dashboard → DNS:

1. Add CNAME record: `getsignatures.org` → your pages URL
2. Enable proxy (orange cloud)

### Step 8: Verify Deployment

Visit these URLs to test:

| URL | Expected Result |
|-----|-----------------|
| `https://getsignatures.org` | Shows login page (redirects from index) |
| `https://getsignatures.org/auth.html` | Shows auth page |
| `https://YOUR_WORKER.workers.dev/health` | Returns `{"status":"healthy",...}` |

---

## Testing Email Locally

**Email cannot be fully tested locally** because:
- Resend requires valid API calls from production
- Email verification links need real domain

### Local Testing Strategy

1. **Frontend auth flow**: Test locally with mock API responses
2. **Email sending**: Test in staging/production only
3. **Use test mode**: Resend has a test mode that doesn't send real emails

### Resend Test Mode

```bash
# In wrangler.toml, add for local dev:
[vars]
EMAIL_TEST_MODE = "true"
```

Then in code, check this var and log instead of sending.

---

## Local Development

### Start Everything

```bash
cd apps/docsign-web

# Terminal 1: Start frontend dev server
npm run dev

# Terminal 2: Start worker locally (optional)
cd worker && wrangler dev
```

Frontend runs at: `http://localhost:8080`
Worker runs at: `http://localhost:8787`

### Testing Locally

```bash
# Run TypeScript tests
npm run test

# Run Rust tests
cd worker && cargo test

# Run all docsign tests
./scripts/test-app.sh docsign
```

---

## Troubleshooting

### "KV namespace not found"

You forgot to create the namespace or the ID is wrong:
```bash
wrangler kv namespace list  # See all your namespaces
```

### "JWT_SECRET not configured"

You forgot to set the secret:
```bash
wrangler secret list  # See what secrets exist
wrangler secret put JWT_SECRET  # Set it
```

### "Email sending failed"

1. Check RESEND_API_KEY is set correctly
2. Verify your domain is configured in Resend dashboard
3. Check email quota hasn't been exceeded

### "CORS error"

The worker needs to allow your frontend domain:
- Check `cors_headers()` function in `lib.rs`
- Make sure `Access-Control-Allow-Origin` includes your domain

### Build fails with "wasm32 target not found"

```bash
rustup target add wasm32-unknown-unknown
```

---

## Rollback

If something breaks after deployment:

```bash
# Rollback worker to previous version
wrangler rollback

# For Pages, go to Dashboard → Deployments → click previous deployment → "Rollback"
```

---

## Environment Summary

| Environment | Frontend URL | Worker URL |
|-------------|--------------|------------|
| Production | getsignatures.org | docsign-worker.*.workers.dev |
| Local | localhost:8080 | localhost:8787 |

---

## Security Checklist Before Launch

- [ ] All secrets set via `wrangler secret put` (not in code)
- [ ] HTTPS enforced on custom domain
- [ ] Rate limiting enabled (already in code)
- [ ] CORS restricted to your domain only
- [ ] No console.log with sensitive data in production build

---

## Quick Reference Commands

```bash
# Deploy worker
cd apps/docsign-web/worker && wrangler deploy

# Deploy frontend
cd apps/docsign-web && trunk build --release && wrangler pages deploy www/dist

# Check worker logs
wrangler tail

# List KV namespaces
wrangler kv namespace list

# List secrets
wrangler secret list
```
