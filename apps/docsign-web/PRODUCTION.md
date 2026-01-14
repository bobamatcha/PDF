# Production Deployment Checklist

Step-by-step guide to deploy GetSignatures to production.

## Prerequisites

- [ ] Cloudflare account with domain `getsignatures.org` configured
- [ ] AWS account with SES configured (see `crates/email-proxy/README.md`)
- [ ] GitHub repository secrets configured

## Step 1: Create Cloudflare KV Namespaces

Run these commands in the `worker` directory:

```bash
cd apps/docsign-web/worker

# Create production namespaces
wrangler kv:namespace create "SESSIONS" --env production
wrangler kv:namespace create "RATE_LIMITS" --env production
wrangler kv:namespace create "VERIFICATIONS" --env production

# Create staging namespaces (optional)
wrangler kv:namespace create "SESSIONS" --env staging
wrangler kv:namespace create "RATE_LIMITS" --env staging
wrangler kv:namespace create "VERIFICATIONS" --env staging

# Create dev namespaces (optional)
wrangler kv:namespace create "SESSIONS" --env dev
wrangler kv:namespace create "RATE_LIMITS" --env dev
wrangler kv:namespace create "VERIFICATIONS" --env dev
```

Copy the namespace IDs from the output and update `wrangler.toml`.

## Step 2: Configure Secrets

Set the required secrets:

```bash
# Production secrets (optional - for API protection)
wrangler secret put DOCSIGN_API_KEY --env production

# Staging secrets (optional)
wrangler secret put DOCSIGN_API_KEY --env staging
```

When prompted, enter:
- **DOCSIGN_API_KEY**: A random secret for API authentication (generate with `openssl rand -hex 32`)

**Note**: Email sending is handled by the `email-proxy` Lambda. See `crates/email-proxy/README.md` for deployment.

## Step 3: Configure DNS

In Cloudflare Dashboard:

1. Go to DNS settings for `getsignatures.org`
2. Add CNAME record:
   - Name: `api`
   - Target: `docsign-worker.<your-subdomain>.workers.dev`
   - Proxy: ON (orange cloud)

Alternatively, the worker routes in `wrangler.toml` will handle this automatically.

## Step 4: Configure GitHub Secrets

Add these secrets to your GitHub repository (Settings > Secrets > Actions):

| Secret | Description |
|--------|-------------|
| `CLOUDFLARE_API_TOKEN` | Cloudflare API token with Workers/Pages permissions |
| `CLOUDFLARE_ACCOUNT_ID` | Your Cloudflare account ID |

To create a Cloudflare API token:
1. Go to Cloudflare Dashboard > My Profile > API Tokens
2. Create Token > Custom Token
3. Permissions needed:
   - Account > Cloudflare Pages > Edit
   - Account > Workers Scripts > Edit
   - Account > Workers KV Storage > Edit
   - Zone > Zone > Read (for getsignatures.org)

## Step 5: Deploy Manually (First Time)

For the first deployment, run manually:

```bash
# Deploy worker
cd apps/docsign-web/worker
wrangler deploy --env production

# Build and deploy static site
cd apps/docsign-web
trunk build --release
wrangler pages deploy www/dist --project-name=getsignatures
```

## Step 6: Verify Deployment

Check these endpoints:

```bash
# Health check
curl https://api.getsignatures.org/health

# Should return:
# {"status":"healthy","timestamp":"..."}

# Main site
curl -I https://getsignatures.org
```

## CI/CD Pipeline

After initial setup, the GitHub Actions workflow handles deployments automatically:

- **On push to main**: Deploys to production
- **On pull request**: Runs tests only (no deploy)
- **On push to getsigsmvp**: Runs tests only (no deploy)

## Environment Summary

| Environment | Worker URL | Site URL | Rate Limits |
|-------------|------------|----------|-------------|
| Development | `*.workers.dev` | localhost:8081 | 10/day, 100/month |
| Staging | `*.workers.dev` | localhost:8081 | 50/day, 500/month |
| Production | `api.getsignatures.org` | `getsignatures.org` | 100/day, 3000/month |

## Monitoring

Check worker logs:
```bash
wrangler tail --env production
```

Check KV storage:
```bash
wrangler kv:key list --binding SESSIONS --env production
```

## Rollback

To rollback to a previous version:

```bash
# List deployments
wrangler deployments list

# Rollback worker
wrangler rollback --env production

# For Pages, use Cloudflare Dashboard
```

## Security Checklist

- [ ] DOCSIGN_API_KEY is a secret (not in code)
- [ ] AWS credentials for email-proxy Lambda are secure
- [ ] HTTPS enforced on all endpoints
- [ ] Rate limiting enabled
- [ ] KV namespace IDs not committed to public repo
- [ ] API token has minimal required permissions
