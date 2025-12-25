# Claude Development Guidelines

> **UX Principle**: The interface must work FOR users, not make users work. Design for clarity over flexibility. Elderly users should never need to learn workarounds—if they must, the UI is broken.

This file defines development practices for Claude Code when working in this repository.

## Puppeteer MCP Testing

When verifying UI functionality with Puppeteer MCP:

### Test PDF Files

**Use PDFs from `/output/*.pdf`** - These are pre-generated valid PDFs:
- `florida_purchase_contract.pdf` - Multi-page contract (~17 pages)
- `florida_listing_agreement.pdf` - Listing agreement
- `florida_escalation_addendum.pdf` - Single page addendum

**DO NOT**:
- Copy PDFs into web app bundles
- Try to create PDFs inline with JavaScript
- Embed base64 PDFs in test code

**Instead**: Serve PDFs via a simple file server or use the browser test infrastructure in `crates/benchmark-harness/tests/browser_*.rs` which has proper PDF loading helpers.

### Browser Test Infrastructure

For automated UI tests, use the existing browser test framework:

```bash
# Run browser tests (requires trunk serve running)
cargo test -p benchmark-harness --test browser_pdfjoin test_name

# Start trunk serve first
cd apps/pdfjoin-web && trunk serve --port 8082
```

The browser tests in `crates/benchmark-harness/tests/` have:
- `test_pdf_base64(num_pages)` - Generates valid test PDFs
- `florida_contract_base64()` - Returns real PDF as base64
- Proper async test patterns with chromiumoxide

## Test-First Development Flow

When fixing a bug or adding a feature to fix something broken, **always follow this flow**:

### 1. Write Failing Tests First

- Write tests that should **FAIL** (proving the bug exists or feature is missing)
- **ALWAYS** look for existing test infrastructure before creating new test utilities
- Use existing test helpers, fixtures, and patterns from the codebase
- If existing tests don't exist for the area, **confirm with the user before proceeding**

### 2. Confirm Tests Fail

- Run the tests: `cargo test --all-features --workspace`
- Verify they fail for the expected reason (not compilation errors)
- The failure should prove the bug exists or feature is missing

### 3. Fix the Code

- Implement the minimal fix or feature
- Don't over-engineer—only change what's necessary

### 4. Confirm Tests Pass (BEFORE Puppeteer!)

- Run tests again: `cargo test --all-features --workspace`
- All new tests should now pass
- Existing tests should still pass (no regressions)
- **DO NOT skip to Puppeteer MCP verification until tests pass**

### 5. Verify with Puppeteer MCP

- Use Puppeteer MCP to verify the fix in the actual UI
- Navigate to the affected pages and test the functionality
- Take screenshots if helpful
- See "Puppeteer MCP Testing" section above for PDF handling

### 6. If Puppeteer Shows Bugs Still Exist

- **The tests were wrong**—they didn't properly capture the bug
- Rewrite the tests to properly capture the failure condition
- Go back to step 2

## Quick Reference

```bash
# Run all checks (same as CI)
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --workspace

# Format code
cargo fmt --all

# Run specific test
cargo test test_name

# Run tests for a specific crate
cargo test -p crate-name

# Run tests for a specific app (recommended for faster iteration)
./scripts/test-app.sh pdfjoin    # Tests pdfjoin-core + pdfjoin-wasm
./scripts/test-app.sh agentpdf   # Tests agentpdf-wasm
./scripts/test-app.sh docsign    # Tests docsign-core + docsign-wasm
./scripts/test-app.sh list       # Show all available options
```

## App-Specific Testing

Use `./scripts/test-app.sh` for faster test runs when working on a specific app:

| App | Crates Tested | Command |
|-----|---------------|---------|
| pdfjoin-web | pdfjoin-core, pdfjoin-wasm | `./scripts/test-app.sh pdfjoin` |
| agentpdf-web | agentpdf-wasm | `./scripts/test-app.sh agentpdf` |
| docsign-web | docsign-core, docsign-wasm | `./scripts/test-app.sh docsign` |
| corpus-api | corpus-core, corpus-api | `./scripts/test-app.sh corpus` |

### pdfjoin-web Development

```bash
cd apps/pdfjoin-web

# TypeScript build (esbuild)
npm run build          # One-time build
npm run build:watch    # Watch mode
npm run typecheck      # Type check only

# Full dev server (TS watch + Trunk serve)
npm run dev

# Run Rust tests only
./scripts/test-app.sh pdfjoin
```

## Pre-commit Hook

Install the pre-commit hook to catch issues before they reach CI:

```bash
cp scripts/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

The hook runs: fmt check → clippy → tests

## CI Workflow

The CI runs on every push and PR to main:

1. **Format** - `cargo fmt --all -- --check`
2. **Clippy** - `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests** - `cargo test --all-features --workspace`
4. **WASM Build** - Builds WASM packages if they exist

## Crate Structure

```
crates/           # Shared libraries
  shared-types/   # Common types (Document, Violation, etc.)
  shared-pdf/     # PDF parsing, coordinate transforms
  shared-crypto/  # Crypto primitives
  compliance-engine/  # Florida Chapter 83 rules
  corpus-core/    # Search & embeddings
  docsign-core/   # Signing logic
  typst-engine/   # Document rendering

apps/             # Deployable applications
  agentpdf-web/   # agentPDF.org
  docsign-web/    # getsignatures.org
  corpus-api/     # Search API server
  mcp-server/     # Claude Desktop MCP
```

## Key Principles

1. **Prefer existing code** - Copy and adapt battle-tested code from microservices
2. **Keep tests** - Maintain the 150+ existing tests during migration
3. **Local-first** - Both web apps run entirely in browser with IndexedDB
4. **WASM target** - Web apps compile to wasm32-unknown-unknown

## Build Configuration Rules

**CRITICAL: Fix build commands, don't manually copy files!**

When dealing with build output location issues:
1. **NEVER** manually copy/move build artifacts between directories
2. **NEVER** create hacky symlinks to work around build issues
3. **ALWAYS** fix the build configuration (package.json, Trunk.toml, etc.) at the source
4. **ALWAYS** ensure build outputs go directly where they need to be

Bad example (DON'T DO THIS):
```bash
cp dist/js/bundle.js www/js/bundle.js  # WRONG - hacky workaround
```

Good example (DO THIS):
```json
// Fix package.json to output to correct location
"build": "esbuild ... --outfile=www/js/bundle.js"
```

**Today's date context**: When searching for documentation or solutions online, use the current year (2025) to find up-to-date information. Avoid outdated solutions from 2023 or earlier.
