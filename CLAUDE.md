# Claude Development Guidelines

This file defines development practices for Claude Code when working in this repository.

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

### 4. Confirm Tests Pass

- Run tests again: `cargo test --all-features --workspace`
- All new tests should now pass
- Existing tests should still pass (no regressions)

### 5. Verify with Puppeteer MCP

- Use Puppeteer MCP to verify the fix in the actual UI
- Navigate to the affected pages and test the functionality
- Take screenshots if helpful

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
