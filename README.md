# PDF Monolith

> **Development Guidelines**: See [CLAUDE.md](./CLAUDE.md) for test-first development practices.

Consolidating agentPDF-server, agentPDF-web, corpus-server, and docsign-web into a unified workspace with two deployable web applications.

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

| Domain | Purpose | Source |
|--------|---------|--------|
| **agentPDF.org** | Compliance checking + template population | agentPDF-web + corpus-server |
| **getsignatures.org** | Standalone digital signatures | docsign-web |

## Crate Structure

```
crates/                     # Shared libraries
├── shared-types/           # Document, Violation, ComplianceReport
├── shared-pdf/             # PDF parsing, coordinate transforms
├── shared-crypto/          # ECDSA P-256, CMS/PKCS#7, certificates
├── compliance-engine/      # Florida Chapter 83 rules (10 rules, 90+ tests)
├── docsign-core/           # PAdES signing, audit chain
└── typst-engine/           # Typst document rendering

apps/                       # Deployable applications
├── agentpdf-web/           # agentPDF.org (WASM + static site)
│   ├── wasm/               # WASM bindings
│   └── www/                # Static assets
├── docsign-web/            # getsignatures.org (WASM + static site)
│   ├── wasm/               # WASM bindings
│   └── www/                # Static assets
└── mcp-server/             # Claude Desktop MCP integration
```

## Development

### Build WASM Packages

```bash
# agentPDF.org
cd apps/agentpdf-web/wasm
wasm-pack build --target web --out-dir ../www/pkg

# getsignatures.org
cd apps/docsign-web/wasm
wasm-pack build --target web --out-dir ../www/pkg
```

### Run Local Development Server

```bash
# agentPDF.org (port 8080)
cd apps/agentpdf-web/www
python3 -m http.server 8080

# getsignatures.org (port 8081)
cd apps/docsign-web/www
python3 -m http.server 8081
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
- Florida compliance checking (10 rules)
- Violation highlighting with positions
- IndexedDB persistence

```bash
# Build WASM first
cd apps/agentpdf-web/wasm
wasm-pack build --target web --out-dir ../www/pkg

# Start server (in background)
cd ../www && python3 -m http.server 8080 &

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
# Build WASM first
cd apps/docsign-web/wasm
wasm-pack build --target web --out-dir ../www/pkg

# Start server (in background)
cd ../www && python3 -m http.server 8081 &

# Run demo (if demo binary exists)
cargo run -p docsign-demo --release
```

### Manual Browser Verification

For manual testing, open the local servers in Chrome:

```bash
# agentPDF.org
open http://localhost:8080

# getsignatures.org
open http://localhost:8081
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

The MCP server provides Claude Desktop integration for document generation:

```bash
# Run MCP server
cargo run -p mcp-server --release

# With timeout (default 30s)
cargo run -p mcp-server --release -- --timeout 60000
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

## CI/CD

The GitHub Actions workflow runs on every push and PR:

1. **Format** - `cargo fmt --all -- --check`
2. **Clippy** - `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests** - `cargo test --all-features --workspace`
4. **WASM Build** - Builds both WASM packages

See `.github/workflows/ci.yml` for details.

## Current Status

See [PLAN.md](./PLAN.md#current-progress-phase-0) for detailed progress tracking.

### Quality Status

| Check | Status |
|-------|--------|
| **Tests** | ✅ 307 passing |
| **Clippy** | ✅ Clean (no warnings) |
| **Format** | ✅ Formatted |
| **Demos** | ✅ Both verified |

### Test Results: 307 Tests Passing

| Crate | Tests | Description |
|-------|-------|-------------|
| shared-types | 82 | Document, Violation, ComplianceReport types |
| shared-pdf | 30 | PDF parsing, coordinate transforms, PAdES signer |
| shared-crypto | 33 | ECDSA P-256, CMS/PKCS#7, certificates, TSA |
| compliance-engine | 31 | Florida Chapter 83 rules |
| typst-engine | 107 | Document rendering, templates, verifier |
| docsign-core | 2 | PAdES signing, audit chain |
| **Total** | **307** | All tests passing |

### Compiling

- All shared crates (shared-types, shared-pdf, shared-crypto, compliance-engine, docsign-core, typst-engine)
- Both WASM apps (agentpdf-wasm, docsign-wasm)
- MCP server

### Deferred

- corpus-core (fastembed/ort_sys compatibility)
- corpus-api (depends on corpus-core)
- docsign-web/worker (worker-sys needs update)
