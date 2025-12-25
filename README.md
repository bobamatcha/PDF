# PDF Monolith

> **UX Principle**: The interface must work FOR users, not make users work. Design for clarity over flexibility. Elderly users should never need to learn workarounds—if they must, the UI is broken.

> **Development Guidelines**: See [CLAUDE.md](./CLAUDE.md) for test-first development practices.

Consolidating agentPDF-server, agentPDF-web, corpus-server, and docsign-web into a unified workspace with three deployable web applications.

<!-- BENCHMARK_RESULTS_START -->
## Performance Benchmarks

> **Last updated**: 2025-12-21 19:36 UTC
>
> Run `./scripts/update-benchmarks.sh` to refresh these results.

### WASM Bundle Sizes

| App | Bundle Size | Description |
|-----|-------------|-------------|
| **pdfjoin-web** | **327 KB** | PDF split/merge only |
| docsign-web | 3.0 MB | Document signing + crypto |
| agentpdf-web | 75 MB | Full Typst template engine |

### Core Web Vitals

| App | LCP (p95) | CLS | INP | Status |
|-----|-----------|-----|-----|--------|
| **pdfjoin-web** | **80ms** | 0.000 | 24ms | PASS |
| agentpdf-web | 396ms | 0.050 | — | PASS |
| docsign-web | — | — | — | Not tested |

**Thresholds**: LCP < 500ms, CLS < 0.1, INP < 100ms

### Running Benchmarks

```bash
# Build all apps first
cd apps/agentpdf-web && trunk build --release && cd ../..
cd apps/docsign-web && trunk build --release && cd ../..
cd apps/pdfjoin-web && trunk build --release && cd ../..

# Start servers (in separate terminals)
cd apps/agentpdf-web && trunk serve --port 8080
cd apps/docsign-web && trunk serve --port 8081
cd apps/pdfjoin-web && trunk serve --port 8082

# Update README with latest results
./scripts/update-benchmarks.sh

# Or run individual benchmarks manually
cargo run -p benchmark-harness --example run_benchmark -- crates/benchmark-harness/scenarios/pdfjoin.toml
cargo run -p benchmark-harness --example run_benchmark -- crates/benchmark-harness/scenarios/agentpdf.toml
```

<!-- BENCHMARK_RESULTS_END -->

## Documentation

- [CLAUDE.md](./CLAUDE.md) - Development guidelines and test-first flow
- [PLAN.md](./PLAN.md) - Integration plan and implementation phases
- [STRATEGY.md](./STRATEGY.md) - Market positioning and go-to-market
- [RESEARCH.md](./RESEARCH.md) - Architectural research

## Quick Start

```bash
# Install pre-commit hook
cp scripts/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

# Run all checks
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --workspace
```

## Deployable Sites

| Domain | Purpose | Bundle Size | Source |
|--------|---------|-------------|--------|
| **agentPDF.org** | Compliance checking + template population | 75 MB | agentpdf-web |
| **getsignatures.org** | Standalone digital signatures | 3.0 MB | docsign-web |
| **pdfjoin** | PDF split & merge tools | 327 KB | pdfjoin-web |

## Crate Structure

```
crates/                     # Shared libraries
├── shared-types/           # Document, Violation, ComplianceReport
├── shared-pdf/             # PDF parsing, coordinate transforms, PAdES signer
├── shared-crypto/          # ECDSA P-256, CMS/PKCS#7, certificates, TSA
├── compliance-engine/      # 16-state landlord-tenant rules (227 tests)
├── docsign-core/           # PAdES signing, audit chain
├── pdfjoin-core/           # PDF split/merge algorithms
├── typst-engine/           # Typst rendering + 3 embedded templates
└── benchmark-harness/      # Performance benchmarking framework

apps/                       # Deployable applications
├── agentpdf-web/           # agentPDF.org (WASM 75MB - full Typst engine)
│   ├── wasm/               # WASM bindings
│   └── www/                # Static assets
├── docsign-web/            # getsignatures.org (WASM 3MB - signing + crypto)
│   ├── wasm/               # WASM bindings
│   └── www/                # Static assets
├── pdfjoin-web/            # PDF tools (WASM 327KB - split/merge only)
│   ├── wasm/               # WASM bindings (session, validation, page_info)
│   └── www/                # Static assets (SPA with tabs)
└── mcp-server/             # Claude Desktop MCP (stdio + HTTP transport)
```

## Features

### Compliance Engine
- **16-State Coverage** - FL, TX, CA, NY, GA, IL, PA, NJ, VA, MA, OH, MI, WA, AZ, NC, TN
- **268 Tests** - Comprehensive rule validation with property-based testing (including Florida real estate)
- **Violation Detection** - Pattern matching with severity levels (Critical, Warning, Info)
- **State-specific Rules** - Security deposits, notice periods, prohibited provisions, disclosures

### Document Templates
- **7 Embedded Typst Templates**: `invoice`, `letter`, `florida_lease`, `florida_purchase_contract`, `florida_escalation_addendum`, `florida_listing_agreement`, `texas_lease`
- **florida_lease.typ**: Comprehensive Florida residential lease (F.S. Chapter 83 compliant)
- **florida_purchase_contract.typ**: Full residential purchase contract with all mandatory disclosures
- **florida_escalation_addendum.typ**: Competitive offer escalation with max price cap
- **florida_listing_agreement.typ**: Exclusive listing with § 475.278 brokerage disclosure
- **texas_lease.typ**: Texas residential lease with Ch. 92 compliance (lockout, deposit, repair, parking)
- **Dynamic Field Population**: 60+ customizable fields via JSON inputs
- **HB 615 Email Consent**: Addendum G - Electronic notice consent per § 83.56
- **SB 948 Flood Disclosure**: Addendum H - Mandatory flood history disclosure per § 83.512
- **HB 1417 (2023)**: 30-day month-to-month termination notice (updated from 15 days)

### Florida Real Estate Compliance
- **§ 404.056** - Radon Gas Disclosure
- **§ 689.261** - Property Tax Disclosure
- **§ 689.302** - Flood Disclosure (SB 948, October 2025)
- **§ 720.401** - HOA Disclosure
- **§ 553.996** - Energy Efficiency Disclosure
- **§ 475.278** - Brokerage Relationship Disclosure
- **§ 475.25** - Definite Expiration Date (Listing Agreements)
- **42 U.S.C. § 4852d** - Lead Paint Disclosure (pre-1978)
- **Johnson v. Davis (1985)** - Material Defect Disclosure
- **HB 1417 (2023)** - 30-day month-to-month termination (updated from 15 days)

### Texas Property Code Compliance (NEW)
- **§ 92.0081** - Lockout policy (no self-help lockout without court order)
- **§ 92.103-109** - Security deposit rules (30-day return, itemization)
- **§ 92.201** - Landlord disclosure requirements (owner/agent identity)
- **§ 92.056** - Repair procedures (tenant request, landlord response)
- **§ 92.0131** - Parking addendum with towing disclosure
- **§ 92.3515** - Tenant screening fee limits
- **42 U.S.C. § 4852d** - Lead Paint Disclosure (pre-1978)

### Digital Signatures
- **PAdES-B Signatures**: PDF Advanced Electronic Signatures
- **ECDSA P-256**: Industry-standard elliptic curve cryptography
- **CMS/PKCS#7**: RFC 5652 compliant signature format
- **TSA Integration**: RFC 3161 timestamp authority support

### MCP Integration
- **Claude Desktop Compatible**: JSON-RPC over stdio
- **HTTP Transport**: Optional REST API mode (feature flag)
- **Tools**: `render_document`, `validate_syntax`, `list_fonts`, `list_templates`
- **Resources**: Template discovery via `typst://templates/*` URIs

### REST API (Web Clients)
- **GET /api/templates**: List available document templates with metadata
- **POST /api/render**: Render templates to PDF/SVG/PNG with custom inputs
- **CORS Enabled**: Cross-origin requests supported for web clients
- **Base64 Output**: PDF data returned as base64 for browser consumption

### Cross-Site Integration
- **agentPDF → DocSign Handoff**: Transfer documents via sessionStorage
- **Template Selector UI**: Modal interface for template selection and form filling
- **Automatic Document Loading**: DocSign detects incoming documents from agentPDF

## Development

### Build & Serve with Trunk (Recommended)

Trunk handles WASM compilation, bundling, and serving with hot reload.

```bash
# Install trunk (if not already installed)
cargo install trunk

# agentPDF.org (port 8080) - 75MB WASM
cd apps/agentpdf-web
trunk serve --port 8080

# getsignatures.org (port 8081) - 3MB WASM
cd apps/docsign-web
trunk serve --port 8081

# PDF Tools (port 8082) - 327KB WASM
cd apps/pdfjoin-web
trunk serve --port 8082
```

**Note**: Trunk reads `Trunk.toml` for configuration. The `target` field points to the input HTML file (`www/index.html`), and `dist` specifies the output directory (`www/dist`). Static files like `tampa.html` are copied via post-build hooks.

### Build for Production

```bash
# agentPDF.org (75MB WASM)
cd apps/agentpdf-web
trunk build --release
# Output in www/dist/ (includes tampa.html landing page)

# getsignatures.org (3MB WASM)
cd apps/docsign-web
trunk build --release
# Output in www/dist/

# PDF Tools (327KB WASM)
cd apps/pdfjoin-web
trunk build --release
# Output in www/dist/
```

### Alternative: Manual wasm-pack Build

If you need to build WASM separately:

```bash
# agentPDF.org
cd apps/agentpdf-web/wasm
wasm-pack build --target web --out-dir ../www/pkg

# getsignatures.org
cd apps/docsign-web/wasm
wasm-pack build --target web --out-dir ../www/pkg
```

### Run MCP Server (HTTP Mode)

```bash
# Start MCP server with HTTP transport for template API
cargo run -p mcp-server --features http -- --transport http --http-addr 0.0.0.0:3000

# Test the API
curl http://localhost:3000/api/templates
curl -X POST http://localhost:3000/api/render \
  -H "Content-Type: application/json" \
  -d '{"template":"letter","is_template":true,"inputs":{"sender_name":"John","recipient_name":"Jane","body":"Hello!"}}'
```

### Run Tests

```bash
# All workspace tests
cargo test --all-features --workspace

# Specific crate
cargo test -p compliance-engine
cargo test -p shared-crypto
```

## Demo Verification

Both web applications can be verified using browser automation. The demos use chromiumoxide (Rust CDP bindings) to drive a headless Chrome browser.

### Demo New Florida Compliance Features

The florida_lease template now includes HB 615 and SB 948 compliance:

**1. Tampa Landing Page (NEW)**
```bash
# Start dev server
cd apps/agentpdf-web && trunk serve --port 8080

# Open Tampa landing page
open http://localhost:8080/tampa.html
```
- Shows 4 compliance tools: Flood Disclosure, Email Consent, Complete Lease, Compliance Check
- Local Tampa REIA event info
- Links to main app

**2. Florida Lease with HB 615 + SB 948**
```bash
# Open main app
open http://localhost:8080

# Click "Use a Template" → Select "florida_lease"
# Expand "Optional Fields (11)" to see:
#   - email_consent (HB 615)
#   - has_prior_flooding, has_flood_claims, has_fema_assistance (§ 83.512)
```

**3. Compliance Check for HB 615 / § 83.512**
- Upload any PDF lease
- Select "Florida (F.S. Chapter 83)" from state selector
- System checks for proper HB 615 consent language and § 83.512 flood disclosure

**4. Tampa REIA Demo Script**
```bash
# Interactive walkthrough for Tampa REIA meetings
./scripts/tampa-demo.sh
```
- Starts dev server automatically
- Opens Tampa landing page
- Step-by-step talking points for each feature
- Covers flood disclosure, email consent, compliance check
- Includes upcoming REIA meeting dates

### Prerequisites

```bash
# Install Chrome/Chromium
# macOS
brew install --cask google-chrome

# Linux
apt-get install chromium-browser
```

### agentPDF.org Demo

The agentPDF demo verifies:
- PDF upload via drag-drop or file picker
- PDF.js rendering with page navigation
- Multi-state compliance checking (16 states)
- Violation highlighting with positions
- IndexedDB persistence

```bash
# Start trunk dev server (builds WASM automatically)
cd apps/agentpdf-web
trunk serve www/index.html --port 8080 &

# Run demo (if demo binary exists)
cargo run -p agentpdf-demo --release
```

### getsignatures.org Demo

The docsign demo verifies the complete signing workflow:
1. **Upload** - PDF upload via drag-drop
2. **Recipients** - Add signer with email
3. **Fields** - Place signature field on document
4. **Review** - Generate PAdES digital signature

```bash
# Start trunk dev server (builds WASM automatically)
cd apps/docsign-web
trunk serve www/index.html --port 8081 &

# Run demo (if demo binary exists)
cargo run -p docsign-demo --release
```

### Manual Browser Verification

For manual testing, start trunk servers and open in Chrome:

```bash
# Start agentPDF.org (in one terminal)
cd apps/agentpdf-web && trunk serve www/index.html --port 8080

# Start getsignatures.org (in another terminal)
cd apps/docsign-web && trunk serve www/index.html --port 8081

# Open in browser
open http://localhost:8080  # agentPDF.org
open http://localhost:8081  # getsignatures.org
```

**agentPDF.org Checklist:**
- [ ] Upload sample PDF (drag-drop or file picker)
- [ ] Verify PDF renders with PDF.js
- [ ] Run compliance check
- [ ] See violation list with severity
- [ ] Navigate between pages
- [ ] Refresh page - document persists (IndexedDB)

**getsignatures.org Checklist:**
- [ ] Upload sample PDF
- [ ] Add recipient (name + email)
- [ ] Click to place signature field
- [ ] Open signature capture pad
- [ ] Draw signature
- [ ] Complete signing flow
- [ ] Download signed PDF
- [ ] Verify audit chain in PDF

## MCP Server

The MCP server provides Claude Desktop integration for document generation and compliance checking.

### Available Tools

| Tool | Description |
|------|-------------|
| `render_document` | Render Typst templates to PDF/SVG/PNG |
| `validate_syntax` | Check Typst syntax without rendering |
| `list_fonts` | List available fonts |
| `list_templates` | Discover embedded templates |

### Available Resources

| URI | Description |
|-----|-------------|
| `typst://templates/invoice` | Invoice template |
| `typst://templates/letter` | Business letter template |
| `typst://templates/florida_lease` | Florida residential lease |
| `typst://templates/texas_lease` | Texas residential lease (Ch. 92 compliant) |
| `typst://fonts` | Available fonts list |

### Run MCP Server

```bash
# Stdio transport (default for Claude Desktop)
cargo run -p mcp-server --release

# With custom timeout (default 30s)
cargo run -p mcp-server --release -- --timeout 60000

# HTTP transport (feature flag)
cargo run -p mcp-server --release --features http -- --http --addr 127.0.0.1:3000
```

### Claude Desktop Configuration

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "pdf-monolith": {
      "command": "/path/to/monolith/target/release/mcp-server"
    }
  }
}
```

### Example: Render Florida Lease

```json
{
  "method": "tools/call",
  "params": {
    "name": "render_document",
    "arguments": {
      "source": "typst://templates/florida_lease",
      "format": "pdf",
      "inputs": {
        "landlord_name": "John Smith",
        "tenant_name": "Jane Doe",
        "property_address": "123 Main St, Miami, FL 33101",
        "monthly_rent": "2000",
        "lease_start": "2025-01-01",
        "lease_end": "2025-12-31"
      }
    }
  }
}
```

## CI/CD

The GitHub Actions workflow runs on every push and PR:

1. **Format** - `cargo fmt --all -- --check`
2. **Clippy** - `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests** - `cargo test --all-features --workspace`
4. **WASM Build** - Builds both WASM packages

See `.github/workflows/ci.yml` for details.

## Deployment

### Prerequisites

1. **Cloudflare Account** with Pages and Workers enabled
2. **GitHub Secrets** configured:
   - `CF_API_TOKEN` - Cloudflare API token with Pages/Workers permissions
   - `CF_ACCOUNT_ID` - Your Cloudflare account ID
3. **Cloudflare Pages Projects** created:
   - `agentpdf-org` for agentPDF.org
   - `getsignatures-org` for getsignatures.org

### Automated Deployment (GitHub Actions)

Push to `main` triggers deployment of both sites:

```bash
git push origin main
```

Or manually deploy a specific site via GitHub Actions:
1. Go to Actions → Deploy
2. Click "Run workflow"
3. Select site: `agentpdf`, `docsign`, or `both`

### Manual Deployment

```bash
# Deploy agentPDF.org
./scripts/deploy-agentpdf.sh

# Deploy getsignatures.org (static + worker)
./scripts/deploy-docsign.sh
```

### Worker Configuration

After deploying the docsign worker, set secrets:

```bash
cd apps/docsign-web/worker

# Required: Resend API key for email sending
wrangler secret put RESEND_API_KEY

# Optional: API key for endpoint protection
# Leave unset for open access (rate limiting still applies)
wrangler secret put DOCSIGN_API_KEY

# Create KV namespaces (first time only)
wrangler kv:namespace create "SESSIONS"
wrangler kv:namespace create "RATE_LIMITS"
# Then update wrangler.toml with the returned IDs
```

### Magic Link / Email Flow

The signing flow uses end-to-end encryption:

1. **Sender** encrypts PDF with AES-256-GCM key
2. **Worker** stores encrypted document in KV (7-day TTL)
3. **Magic link** format: `#sign={sessionId}:{recipientId}:{keyBase64}`
4. **Signer** receives email with link, decrypts locally
5. **Signature** captured and re-encrypted for storage

Rate limits (free tier):
- 100 emails/day
- 3,000 emails/month

## Current Status

See [PLAN.md](./PLAN.md#current-progress) for detailed progress tracking.

### Implementation Phases

| Phase | Status | Description |
|-------|--------|-------------|
| **Phase 0** | ✅ Complete | ASAP Deployment - all crates compile, tests pass |
| **Phase 1** | ✅ Complete | Shared Foundation - shared-types, shared-pdf, shared-crypto |
| **Phase 2** | ✅ Complete | Unified Compliance Engine - 10 Florida rules |
| **Phase 3** | ✅ Complete | Full Integration - templates, REST API, cross-site handoff |

### Quality Status

| Check | Status |
|-------|--------|
| **Tests** | ✅ 540+ passing (including property tests) |
| **Clippy** | ✅ Clean (`-D warnings`) |
| **Format** | ✅ Formatted |
| **WASM** | ✅ All 3 apps compile (agentpdf 75MB, docsign 3MB, pdfjoin 327KB) |
| **Worker** | ✅ docsign-worker compiles (worker 0.7) |
| **Benchmarks** | ✅ All apps pass Core Web Vitals thresholds |
| **Parity Tests** | ✅ wasm-opt config, viewport meta, optimization level |

### Test Results: 540+ Tests Passing

| Crate | Tests | Description |
|-------|-------|-------------|
| compliance-engine | 268 | 16-state rules + Florida real estate compliance (with property tests) |
| agentpdf-wasm | 82 | WASM bindings + compliance integration |
| docsign-wasm | 63 | WASM bindings + signing workflow |
| typst-engine | 59 | Document rendering, 6 templates, verifier, registry tests |
| shared-crypto | 33 | ECDSA P-256, CMS/PKCS#7, certificates, TSA |
| docsign-worker | 31 | Cloudflare Worker + session/magic link property tests |
| shared-pdf | 30 | PDF parsing, coordinate transforms, signer |
| mcp-server | 29 | MCP protocol, HTTP transport, REST API property tests |
| pdfjoin-wasm | 24 | Session management, validation, split/merge regression tests |
| shared-types | 22 | Document, Violation, ComplianceReport types |
| pdfjoin-core | 6 | PDF split/merge algorithms |
| docsign-core | 2 | PAdES signing, audit chain |
| **Total** | **540+** | All tests passing (including property/fuzz tests) |

### All Components Compiling

- **Shared Crates**: shared-types, shared-pdf, shared-crypto, compliance-engine, docsign-core, pdfjoin-core, typst-engine, benchmark-harness
- **WASM Apps**: agentpdf-wasm (75MB), docsign-wasm (3MB), pdfjoin-wasm (327KB)
- **MCP Server**: stdio + HTTP transport (with `http` feature flag)
- **Cloudflare Worker**: docsign-worker (upgraded to worker crate 0.7)

### Deferred

- **corpus-core**: Blocked due to version conflicts between candle-core 0.8.x, rand 0.9.x, and half 2.7.x. Options: wait for candle 0.9 stable, rewrite with fastembed, or use remote embedding API.
- **corpus-api**: Blocked, depends on corpus-core
