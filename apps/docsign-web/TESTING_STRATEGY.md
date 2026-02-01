# Testing Strategy for getsignatures.org

*Created: 2026-02-01*
*Problem: Testing in production is painful and slow*

---

## The Core Problem

The signing flow is **multi-step and involves external services**:

```
Upload PDF → Add Recipients → Place Fields → Generate Links → Send Email → Open Link → Sign → Complete
    ↓            ↓               ↓              ↓              ↓           ↓        ↓
 (WASM)     (IndexedDB)      (Canvas)      (Worker API)    (Resend)    (KV)    (Crypto)
```

Testing this in production means:
- Wasting Resend email credits
- Polluting KV with test data
- Slow feedback loop (deploy → wait → test → find bug → repeat)
- Users hitting bugs before we do

---

## Proposed Solution: Three-Layer Testing

### Layer 1: Local Development (Fast, Free)

**What:** Run everything locally with mocks

**Setup:**
```bash
# Terminal 1: Local worker with Miniflare
cd apps/docsign-web/worker
npx wrangler dev --local

# Terminal 2: Frontend with Trunk
cd apps/docsign-web
trunk serve --port 8080
```

**Mocks needed:**
- `MOCK_EMAIL=true` → Skip Resend, log email to console
- `MOCK_KV=true` → Use in-memory KV (Miniflare does this)
- Test signing links work without real email delivery

**Test mode URL parameter:**
```
http://localhost:8080?test_mode=true
```

When `test_mode=true`:
- Skip email sending, show link directly in modal
- Add "Copy Test Link" button
- Log all API calls to console

---

### Layer 2: Staging Environment (Real infra, isolated)

**What:** Separate Cloudflare project that mirrors production

**Setup:**

| Component | Production | Staging |
|-----------|------------|---------|
| Frontend | getsignatures.org | staging.getsignatures.org |
| Worker | docsign-worker-production | docsign-worker-staging |
| KV: USERS | production namespace | staging namespace |
| KV: SESSIONS | production namespace | staging namespace |
| Resend | Production API key | Test mode / separate key |

**Create staging worker:**
```bash
cd apps/docsign-web/worker

# Create wrangler-staging.toml
cp wrangler.toml wrangler-staging.toml
# Edit: change name, KV namespace IDs

# Deploy to staging
wrangler deploy --config wrangler-staging.toml
```

**Staging benefits:**
- Test real Cloudflare behavior (KV latency, worker limits)
- Isolated from production data
- Can break things without affecting users

---

### Layer 3: Automated E2E Tests (chromiumoxide)

**What:** Headless browser tests that verify the full flow

**Framework:** chromiumoxide (Rust CDP bindings) — already in the codebase, much faster than Playwright/Puppeteer

**Why not Playwright?** Heavy Node.js dependency, slower execution. chromiumoxide is native Rust, fits the stack.

**Test scenarios (Rust with chromiumoxide):**

```rust
// tests/e2e/signing_flow.rs
use chromiumoxide::Browser;

#[tokio::test]
async fn test_complete_signing_flow() -> Result<()> {
    let browser = Browser::launch(BrowserConfig::builder().build()?).await?;
    let page = browser.new_page("https://staging.getsignatures.org?test_mode=true").await?;
    
    // 1. Upload PDF
    let input = page.find_element("input[type='file']").await?;
    input.upload_file("tests/fixtures/test.pdf").await?;
    
    // 2. Wait for preview
    page.wait_for_selector(".pdf-preview").await?;
    
    // 3. Add recipient
    page.type_text("#recipient-first-name", "Test").await?;
    page.type_text("#recipient-last-name", "User").await?;
    page.type_text("#recipient-email", "test@example.com").await?;
    page.click("button:has-text('Add')").await?;
    
    // 4. Verify page count (Bug #10)
    let indicator = page.find_element(".page-indicator").await?;
    let text = indicator.inner_text().await?;
    assert!(text.contains("/ 1"), "Page count should show total");
    
    Ok(())
}
```

**Run on:**
- Every PR (against staging with `?test_mode=true`)
- Before production deploy
- Nightly (catch regressions)

---

## Implementation Plan

### Phase 1: Test Mode (1-2 hours)

Add `?test_mode=true` support to frontend:

```javascript
// In index.html, near the top
const TEST_MODE = new URLSearchParams(window.location.search).has('test_mode');

// In generateSigningLinks(), change email section:
if (TEST_MODE) {
  // Don't send emails, show links directly
  showTestLinksModal(links);
  return;
}
```

**Files to modify:**
- `www/index.html` - Add TEST_MODE check
- Add `showTestLinksModal()` function

### Phase 2: Staging Environment (2-3 hours)

1. Create `wrangler-staging.toml`:
```toml
name = "docsign-worker-staging"
main = "src/index.ts"
compatibility_date = "2024-01-01"

[vars]
ENVIRONMENT = "staging"

[[kv_namespaces]]
binding = "USERS"
id = "STAGING_USERS_KV_ID"  # Create new namespace

[[kv_namespaces]]
binding = "SESSIONS"
id = "STAGING_SESSIONS_KV_ID"  # Create new namespace
```

2. Create staging KV namespaces:
```bash
wrangler kv:namespace create "USERS_STAGING"
wrangler kv:namespace create "SESSIONS_STAGING"
```

3. Deploy staging worker:
```bash
wrangler deploy --config wrangler-staging.toml
```

4. Create Cloudflare Pages project for staging frontend

### Phase 3: E2E Test Suite (3-4 hours)

1. Create test crate (chromiumoxide already in workspace):
```bash
mkdir -p apps/docsign-web/tests/e2e
```

2. Add to `apps/docsign-web/Cargo.toml`:
```toml
[dev-dependencies]
chromiumoxide = "0.5"
tokio = { version = "1", features = ["full"] }
```

3. Write initial test suite in `tests/e2e/mod.rs`:
   - Upload flow
   - Recipient management
   - Page count verification (Bug #10)
   - Signing link generation (test mode)

4. Add to CI:
```yaml
# .github/workflows/e2e.yml
name: E2E Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test --package docsign-e2e
```

---

## Quick Wins (Do Today)

### 1. Add console logging for debugging

In the worker, add debug logging:
```rust
console_log!("[DEBUG] Creating session: {:?}", session_id);
console_log!("[DEBUG] Field positions: {:?}", fields);
```

### 2. Add error tracking (self-hosted only)

**No external services** — avoid vendor lock-in traps like Sentry that go paid later.

Option: Simple error endpoint on the worker
```javascript
// Frontend
window.onerror = (msg, url, line) => {
  fetch('https://api.getsignatures.org/error', {
    method: 'POST',
    body: JSON.stringify({ msg, url, line, userAgent: navigator.userAgent, ts: Date.now() })
  });
};

// Worker: Store in KV with TTL, view in admin dashboard
```

**Future work:** Add `/admin/errors` page to view recent client errors.

### 3. Create test PDF fixtures

Put these in `tests/fixtures/`:
- `1-page.pdf` - Simple single page
- `14-page.pdf` - Multi-page (for Bug #10)
- `100mb.pdf` - Large file (for limit testing)
- `corrupted.pdf` - Invalid PDF (for error handling)

---

## Testing Checklist (Before Any Deploy)

```markdown
## Pre-Deploy Checklist

### Unit Tests
- [ ] `cargo test -p docsign-worker` passes
- [ ] `npm test` in worker/ passes

### Local Testing
- [ ] Upload PDF works
- [ ] Add recipient works
- [ ] Place field works
- [ ] Page count is correct (Bug #10)
- [ ] Generate links works (or shows test modal in test_mode)

### Staging Testing
- [ ] Deploy to staging first
- [ ] Full signing flow works on staging
- [ ] No JS console errors
- [ ] Check mobile view (responsive)

### E2E Tests (when set up)
- [ ] All Playwright tests pass against staging
```

---

## Cost

| Component | Monthly Cost |
|-----------|-------------|
| Staging KV namespaces | $0 (free tier) |
| Staging worker | $0 (free tier: 100k req/day) |
| Staging Pages | $0 (free tier) |
| E2E tests (chromiumoxide) | $0 (GitHub Actions free tier) |

**Total: $0/month** — no external service dependencies

---

## TL;DR

1. **Today:** Add `?test_mode=true` to skip emails during testing
2. **This week:** Set up staging environment
3. **Next week:** Add Playwright E2E tests
4. **Ongoing:** Run E2E on every PR, never deploy untested code to prod

The goal: **Never find a bug in production again.**
