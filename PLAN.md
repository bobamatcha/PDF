# Monolith Integration Plan

> Consolidating agentPDF-server, agentPDF-web, corpus-server, and docsign-web into a unified workspace with two deployable web applications.

## Quick Reference

| Domain | Purpose | Source Microservice | Priority |
|--------|---------|---------------------|----------|
| **agentPDF.org** | Compliance checking + template population | agentPDF-web + corpus-server | High |
| **getsignatures.org** | Standalone digital signatures | docsign-web | High |

**Goal:** Deploy simple working versions to both domains ASAP, then iterate.

**Strategic Context:** See [STRATEGY.md](./STRATEGY.md) for market positioning, vertical targeting, and go-to-market approach.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture Overview](#2-architecture-overview)
3. [Existing Assets Inventory](#3-existing-assets-inventory)
4. [Shared Components Strategy](#4-shared-components-strategy)
5. [Directory Structure](#5-directory-structure)
6. [Migration Plan](#6-migration-plan)
7. [Dual-Site Deployment Strategy](#7-dual-site-deployment-strategy)
8. [Phase 0: ASAP Deployment](#8-phase-0-asap-deployment)
9. [Phase 1: Shared Foundation](#9-phase-1-shared-foundation)
10. [Phase 2: Unified Compliance Engine](#10-phase-2-unified-compliance-engine)
11. [Phase 3: Full Integration](#11-phase-3-full-integration)
12. [Test Coverage Strategy](#12-test-coverage-strategy)
13. [Demo Functionality Preservation](#13-demo-functionality-preservation)
14. [Future Considerations](#14-future-considerations)

---

## 1. Executive Summary

This plan consolidates four microservices into a **modular monolith** using Cargo Workspaces, enabling:

1. **Code Reuse**: Extract ~25,000 lines of production Rust code
2. **Shared Components**: Common PDF handling, types, and utilities
3. **Dual Deployment**: Two independent web applications from one repository
4. **Test Preservation**: Maintain 150+ existing tests across all components
5. **ASAP Launch**: Phased approach prioritizing working deployments

### Key Principle: Prefer Existing Code

The microservices contain battle-tested implementations. This plan prioritizes **copying and adapting existing code** over rewriting:

| Component | Existing Code | Action |
|-----------|---------------|--------|
| Florida compliance rules | agentPDF-web (90 tests) | Copy directly |
| PDF signing (PAdES) | docsign-web (63 tests) | Copy directly |
| Typst rendering | agentPDF-server | Copy directly |
| Semantic search | corpus-server | Copy directly |
| Coordinate transforms | docsign-web | Extract to shared |
| PDF parsing | Both web services | Unify into shared |

### Insights from Research

The [RESEARCH.md](./RESEARCH.md) document provides architectural guidance. Key applicable ideas:

| Research Concept | Application |
|------------------|-------------|
| **Modular Monolith** | Use Cargo Workspaces for logical separation |
| **Shared Dependencies** | Unify tokio, serde, axum versions workspace-wide |
| **Local-First** | Preserve IndexedDB storage in web apps |
| **Type Safety** | Consider rspc for future Tauri desktop version |
| **Tantivy Integration** | Already implemented in corpus-server |

> **Note:** The research proposes a Tauri desktop app. For ASAP web deployment, we defer Tauri but preserve the architecture for future desktop builds.

### Insights from Strategy

The [STRATEGY.md](./STRATEGY.md) document provides market positioning and go-to-market guidance. Key strategic priorities that inform this plan:

| Strategic Priority | Implementation Impact | Timeline |
|--------------------|----------------------|----------|
| **Florida RE Dogfooding** | Phase 0 targets Florida real estate agents & property managers | **Short-Term** |
| **Offline-First Competitive Moat** | Preserve existing local-first architecture in both apps | Short-Term |
| **Medical Mode** | Plan for HIPAA-compliant local encryption in docsign-web | Medium-Term |
| **Field Ops Mode** | Add GPS/photo evidence capture to getsignatures.org | Medium-Term |
| **MCP as AI Infrastructure** | Ensure mcp-server app is production-ready for enterprise | Medium-Term |
| **Government Micro-Purchase** | Keep annual pricing under $10K threshold | Long-Term |

> **Why Florida RE First:** The corpus already contains Florida residential lease templates. Targeting landlords and property managers first allows focused dogfooding before pivoting to other verticals. Regulatory pressure (§ 83.512, HB 615) creates natural market urgency.

---

## 2. Architecture Overview

### Current State: Four Microservices

```
microservices/
├── agentPDF-server/    # Typst MCP, 5,642 lines
├── agentPDF-web/       # WASM compliance, 10,207 lines
├── corpus-server/      # Search engine, 4,450 lines
└── docsign-web/        # Signatures, 5,080 lines
```

### Target State: Modular Monolith

```
monolith/
├── crates/                     # Shared Rust libraries
│   ├── shared-types/           # Common types (Document, Violation, etc.)
│   ├── shared-pdf/             # PDF parsing, coordinate transforms
│   ├── shared-crypto/          # Crypto primitives for signing
│   ├── compliance-engine/      # Unified Florida Chapter 83 rules
│   ├── corpus-core/            # Search & embeddings (from corpus-server)
│   ├── docsign-core/           # Signing logic (from docsign-web)
│   └── typst-engine/           # Document rendering (from agentPDF-server)
│
├── apps/
│   ├── agentpdf-web/           # agentPDF.org WASM + frontend
│   │   ├── wasm/               # WASM bindings
│   │   └── www/                # Static site
│   │
│   ├── docsign-web/            # getsignatures.org WASM + frontend
│   │   ├── wasm/               # WASM bindings
│   │   └── www/                # Static site
│   │
│   ├── corpus-api/             # Optional: Shared search API server
│   └── mcp-server/             # Claude Desktop MCP server
│
├── Cargo.toml                  # Workspace manifest
├── PLAN.md                     # This file
├── RESEARCH.md                 # Architectural research
└── STRATEGY.md                 # Market positioning & GTM
```

### Deployment Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ONE GITHUB REPOSITORY                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────┐    ┌─────────────────────────┐        │
│  │    apps/agentpdf-web    │    │    apps/docsign-web     │        │
│  │                         │    │                         │        │
│  │  Build: wasm-pack       │    │  Build: wasm-pack       │        │
│  │  Output: www/pkg/       │    │  Output: www/pkg/       │        │
│  │  Deploy: Cloudflare     │    │  Deploy: Cloudflare     │        │
│  └───────────┬─────────────┘    └───────────┬─────────────┘        │
│              │                              │                       │
│              ▼                              ▼                       │
│       agentPDF.org                  getsignatures.org               │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                      SHARED CRATES                           │   │
│  │  shared-types │ shared-pdf │ compliance-engine │ corpus-core │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Existing Assets Inventory

### 3.1 agentPDF-server (5,642 lines)

**Location:** `../microservices/agentPDF-server/`

| Component | Lines | Destination | Notes |
|-----------|-------|-------------|-------|
| `src/compiler/` | ~800 | `crates/typst-engine/` | Typst compilation with timeout |
| `src/world/` | ~600 | `crates/typst-engine/` | Virtual filesystem, font cache |
| `src/mcp/` | ~1,200 | `apps/mcp-server/` | MCP protocol implementation |
| `src/verifier/` | ~500 | `crates/compliance-engine/` | Florida rules (subset) |
| `templates/` | ~1,200 | `apps/agentpdf-web/templates/` | Typst templates |
| `tests/` | ~500 | Preserve location | 5 test files |

**Key Files to Copy:**
- `src/compiler/render.rs` - Core Typst compilation
- `src/world/virtual_world.rs` - World trait implementation
- `src/world/fonts.rs` - Embedded font handling
- `templates/florida_lease.typ` - 1,100-line production template

### 3.2 agentPDF-web (10,207 lines)

**Location:** `../microservices/agentPDF-web/`

| Crate | Lines | Destination | Notes |
|-------|-------|-------------|-------|
| `agentpdf-core` | ~1,500 | `crates/shared-types/` | Document, Violation types |
| `agentpdf-compliance` | ~2,000 | `crates/compliance-engine/` | 10 Florida rules, 90 tests |
| `agentpdf-wasm` | ~3,000 | `apps/agentpdf-web/wasm/` | WASM entry point |
| `agentpdf-server` | ~1,200 | `apps/corpus-api/` | Axum server + MCP |
| `agentpdf-test` | ~1,500 | `apps/agentpdf-web/tests/` | E2E framework |
| `www/` | ~1,000 | `apps/agentpdf-web/www/` | Static assets |

**Key Files to Copy:**
- `crates/agentpdf-compliance/src/rules/` - All rule implementations
- `crates/agentpdf-compliance/src/patterns.rs` - Violation patterns
- `crates/agentpdf-core/src/types.rs` - LeaseDocument, ComplianceReport
- `www/index.html` - Production UI

### 3.3 corpus-server (4,450 lines)

**Location:** `../microservices/corpus-server/`

| Crate | Lines | Destination | Notes |
|-------|-------|-------------|-------|
| `corpus-core` | ~1,500 | `crates/corpus-core/` | Document, embeddings, storage |
| `corpus-server` | ~500 | `apps/corpus-api/` | HTTP handlers |
| `corpus-verify` | ~800 | `crates/compliance-engine/` | Verification traits |
| `corpus-ingest` | ~500 | `apps/corpus-api/` | Ingestion pipeline |
| `corpus-bench` | ~650 | `apps/corpus-api/benches/` | Criterion benchmarks |

**Key Files to Copy:**
- `corpus-core/src/embeddings.rs` - BGE-M3 model integration
- `corpus-core/src/storage.rs` - LanceDB + Arrow
- `corpus-core/src/search/` - Hybrid search implementation
- `corpus-server/src/handlers/` - All HTTP handlers

### 3.4 docsign-web (5,080 lines)

**Location:** `../microservices/docsign-web/`

| Component | Lines | Destination | Notes |
|-----------|-------|-------------|-------|
| `docsign-wasm/src/pdf/` | ~1,000 | `crates/docsign-core/` | Parser, signer, audit |
| `docsign-wasm/src/crypto/` | ~800 | `crates/shared-crypto/` | ECDSA, CMS, TSA |
| `docsign-wasm/src/coords.rs` | ~200 | `crates/shared-pdf/` | Coordinate transforms |
| `docsign-wasm/src/storage/` | ~300 | `apps/docsign-web/wasm/` | IndexedDB |
| `www/` | ~1,500 | `apps/docsign-web/www/` | Multi-step wizard UI |
| `docsign-server/` | ~800 | `apps/docsign-web/worker/` | Cloudflare Worker |
| `e2e-tests/` | ~500 | `apps/docsign-web/tests/` | Puppeteer tests |

**Key Files to Copy:**
- `docsign-wasm/src/pdf/signer.rs` - PAdES signature injection
- `docsign-wasm/src/crypto/keys.rs` - ECDSA P-256 identity
- `docsign-wasm/src/crypto/cms.rs` - PKCS#7/CMS SignedData
- `www/index.html` - 4-step wizard UI
- `www/sign.js` - Signing workflow

---

## 4. Shared Components Strategy

### 4.1 Extraction Priority

Create shared crates **before** copying application code. This ensures proper dependency flow:

```
1. shared-types      (no deps on other crates)
2. shared-pdf        (depends on: shared-types)
3. shared-crypto     (depends on: shared-types)
4. compliance-engine (depends on: shared-types, shared-pdf)
5. corpus-core       (depends on: shared-types)
6. docsign-core      (depends on: shared-types, shared-pdf, shared-crypto)
7. typst-engine      (depends on: shared-types)
```

### 4.2 shared-types

**Purpose:** Common types used across all applications.

**Source Files:**
- `agentPDF-web/agentpdf/crates/agentpdf-core/src/types.rs`
- `corpus-server/corpus-core/src/document.rs`

**Unified Types:**
```rust
// crates/shared-types/src/lib.rs
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
}

pub struct Violation {
    pub rule_id: String,
    pub statute: String,
    pub severity: Severity,
    pub message: String,
    pub position: Option<TextPosition>,
}

pub struct ComplianceReport {
    pub document_id: String,
    pub violations: Vec<Violation>,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
}

pub enum Severity {
    Critical,
    High,
    Medium,
    Warning,
    Info,
}
```

### 4.3 shared-pdf

**Purpose:** PDF parsing and coordinate transformation.

**Source Files:**
- `docsign-web/docsign-wasm/src/coords.rs` - DOM↔PDF mapping
- `docsign-web/docsign-wasm/src/pdf/parser.rs` - lopdf wrapper
- `agentPDF-web/agentpdf/crates/agentpdf-wasm/src/extraction/` - Text extraction

**Unified Interface:**
```rust
// crates/shared-pdf/src/lib.rs
pub trait PdfDocument {
    fn load(bytes: &[u8]) -> Result<Self>;
    fn page_count(&self) -> usize;
    fn extract_text(&self, page: usize) -> Result<String>;
    fn get_dimensions(&self, page: usize) -> (f64, f64);
}

pub struct CoordinateTransform {
    // From docsign-web/coords.rs
    pub fn dom_to_pdf(&self, x: f64, y: f64) -> (f64, f64);
    pub fn pdf_to_dom(&self, x: f64, y: f64) -> (f64, f64);
}
```

### 4.4 shared-crypto

**Purpose:** Cryptographic primitives for signing.

**Source Files:**
- `docsign-web/docsign-wasm/src/crypto/keys.rs` - ECDSA P-256
- `docsign-web/docsign-wasm/src/crypto/cms.rs` - CMS/PKCS#7

**Interface:**
```rust
// crates/shared-crypto/src/lib.rs
pub trait SigningKey {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn public_key(&self) -> &[u8];
}

pub struct EcdsaP256Key { /* from docsign-web */ }
pub struct CmsSignedData { /* from docsign-web */ }
```

### 4.5 compliance-engine

**Purpose:** Unified Florida Chapter 83 compliance checking.

**Source Files (to merge):**

| From agentPDF-server | From agentPDF-web | Combined |
|---------------------|-------------------|----------|
| RadonDisclosureRule | - | ✓ |
| SecurityDepositBank | SecurityDepositReturn | Merge both |
| LeadPaintDisclosure | - | ✓ |
| BedBugDisclosure | - | ✓ |
| ProhibitedTermsRule | ProhibitedProvisions | Use web (better) |
| LateFeeRule | - | ✓ |
| GracePeriodRule | - | ✓ |
| - | AsIsStructural | ✓ |
| - | AttorneyFeeReciprocity | ✓ |
| - | NoticeRequirements | ✓ |
| - | MonthToMonthNotice | ✓ |
| AnomalyDetection | - | ✓ |

**Unified Engine:**
```rust
// crates/compliance-engine/src/lib.rs
pub struct UnifiedComplianceEngine {
    rules: Vec<Box<dyn ComplianceRule>>,
}

impl UnifiedComplianceEngine {
    pub fn florida() -> Self {
        Self {
            rules: vec![
                // From agentPDF-server
                Box::new(RadonDisclosureRule),
                Box::new(LeadPaintDisclosureRule),
                Box::new(BedBugDisclosureRule),
                Box::new(AnomalyDetectionRule),

                // From agentPDF-web (better implementations)
                Box::new(ProhibitedProvisionsRule),
                Box::new(SecurityDepositRule),  // Merged
                Box::new(AsIsStructuralRule),
                Box::new(AttorneyFeeRule),
                Box::new(NoticeRequirementsRule),
                Box::new(MonthToMonthRule),
                Box::new(LateFeeRule),
                Box::new(GracePeriodRule),
            ],
        }
    }

    pub fn check(&self, text: &str) -> ComplianceReport { /* ... */ }
}
```

---

## 5. Directory Structure

### Complete Monolith Structure

```
monolith/
├── Cargo.toml                      # Workspace manifest
├── Cargo.lock
├── PLAN.md                         # This file
├── RESEARCH.md                     # Architectural research
├── STRATEGY.md                     # Market positioning & GTM
├── Makefile                        # Top-level build commands
│
├── crates/
│   ├── shared-types/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── document.rs         # From agentpdf-core + corpus-core
│   │       ├── violation.rs        # From agentpdf-core
│   │       └── report.rs           # From agentpdf-core
│   │
│   ├── shared-pdf/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs           # From docsign-wasm/pdf/parser.rs
│   │       ├── coords.rs           # From docsign-wasm/coords.rs
│   │       └── extraction.rs       # From agentpdf-wasm/extraction/
│   │
│   ├── shared-crypto/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── keys.rs             # From docsign-wasm/crypto/keys.rs
│   │       ├── cms.rs              # From docsign-wasm/crypto/cms.rs
│   │       └── tsa.rs              # From docsign-wasm/crypto/tsa.rs
│   │
│   ├── compliance-engine/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              # UnifiedComplianceEngine
│   │       ├── rules/
│   │       │   ├── mod.rs
│   │       │   ├── radon.rs        # From agentPDF-server
│   │       │   ├── lead_paint.rs   # From agentPDF-server
│   │       │   ├── bed_bug.rs      # From agentPDF-server
│   │       │   ├── prohibited.rs   # From agentPDF-web
│   │       │   ├── deposit.rs      # Merged from both
│   │       │   ├── attorney.rs     # From agentPDF-web
│   │       │   ├── notices.rs      # From agentPDF-web
│   │       │   ├── as_is.rs        # From agentPDF-web
│   │       │   ├── late_fee.rs     # From agentPDF-server
│   │       │   ├── grace_period.rs # From agentPDF-server
│   │       │   └── anomaly.rs      # From agentPDF-server
│   │       ├── patterns.rs         # From agentPDF-web
│   │       ├── extractors.rs       # From agentPDF-web
│   │       └── calendar.rs         # From agentPDF-web
│   │
│   ├── corpus-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── document.rs         # From corpus-server/corpus-core
│   │       ├── storage.rs          # From corpus-server/corpus-core
│   │       ├── embeddings.rs       # From corpus-server/corpus-core
│   │       └── search/             # From corpus-server/corpus-core
│   │
│   ├── docsign-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── signer.rs           # From docsign-wasm/pdf/signer.rs
│   │       ├── audit.rs            # From docsign-wasm/pdf/audit.rs
│   │       └── session.rs          # From docsign-wasm/session/
│   │
│   └── typst-engine/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── render.rs           # From agentPDF-server/compiler/
│           ├── world.rs            # From agentPDF-server/world/
│           ├── fonts.rs            # From agentPDF-server/world/
│           └── templates/          # Embedded templates
│
├── apps/
│   ├── agentpdf-web/               # → agentPDF.org
│   │   ├── Cargo.toml
│   │   ├── Makefile
│   │   ├── wasm/
│   │   │   ├── Cargo.toml          # WASM bindings
│   │   │   └── src/
│   │   │       └── lib.rs          # From agentPDF-web/agentpdf-wasm
│   │   ├── www/
│   │   │   ├── index.html          # From agentPDF-web/www
│   │   │   ├── florida_lease.pdf   # Sample document
│   │   │   ├── js/
│   │   │   │   ├── pdf-bridge.js   # PDF.js integration
│   │   │   │   ├── template-selector.js  # NEW: Template picker
│   │   │   │   └── docsign-handoff.js    # NEW: Redirect to getsignatures.org
│   │   │   ├── styles/
│   │   │   └── pkg/                # WASM output
│   │   ├── templates/              # Typst templates
│   │   │   └── florida_lease.typ
│   │   └── tests/
│   │       ├── e2e/                # From agentPDF-web/e2e-tests
│   │       └── rust/               # From agentPDF-web/agentpdf-test
│   │
│   ├── docsign-web/                # → getsignatures.org
│   │   ├── Cargo.toml
│   │   ├── Makefile
│   │   ├── wasm/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       └── lib.rs          # From docsign-web/docsign-wasm
│   │   ├── www/
│   │   │   ├── index.html          # From docsign-web/www
│   │   │   ├── sign.html           # Recipient signing page
│   │   │   ├── sign.js             # Signing workflow
│   │   │   ├── guided-flow.js
│   │   │   ├── signature-pad.js
│   │   │   ├── styles.css
│   │   │   └── pkg/
│   │   ├── worker/                 # Cloudflare Worker
│   │   │   ├── Cargo.toml
│   │   │   ├── wrangler.toml
│   │   │   └── src/lib.rs          # From docsign-web/docsign-server
│   │   └── tests/
│   │       └── e2e/                # From docsign-web/e2e-tests
│   │
│   ├── corpus-api/                 # Optional shared API
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs             # From corpus-server/corpus-server
│   │       └── handlers/           # HTTP endpoints
│   │
│   └── mcp-server/                 # Claude Desktop integration
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # From agentPDF-server
│           └── tools.rs
│
└── scripts/
    ├── build-all.sh                # Build all apps
    ├── deploy-agentpdf.sh          # Deploy to agentPDF.org
    ├── deploy-docsign.sh           # Deploy to getsignatures.org
    └── migrate-history.sh          # Git history preservation
```

---

## 6. Migration Plan

### 6.1 Git History Preservation

Preserve commit history from all microservices:

```bash
#!/bin/bash
# scripts/migrate-history.sh

# Initialize monolith if needed
cd /Users/amar/AG1337v2/BobaMatchSolutions/PDF/monolith

# Import each microservice with history
for service in agentPDF-server agentPDF-web corpus-server docsign-web; do
    echo "Importing $service..."

    # Add as remote
    git remote add $service ../microservices/$service
    git fetch $service

    # Merge with history preservation
    git merge $service/main --allow-unrelated-histories \
        -m "Import $service with full history"

    # Move to appropriate directory
    # (Manual restructuring follows)

    git remote remove $service
done
```

### 6.2 File Copy Script

```bash
#!/bin/bash
# scripts/copy-sources.sh

MICRO="../microservices"
MONO="."

# ===== SHARED TYPES =====
mkdir -p crates/shared-types/src
cp $MICRO/agentPDF-web/agentpdf/crates/agentpdf-core/src/types.rs \
   crates/shared-types/src/document.rs

# ===== SHARED PDF =====
mkdir -p crates/shared-pdf/src
cp $MICRO/docsign-web/docsign-wasm/src/coords.rs \
   crates/shared-pdf/src/coords.rs
cp $MICRO/docsign-web/docsign-wasm/src/pdf/parser.rs \
   crates/shared-pdf/src/parser.rs

# ===== SHARED CRYPTO =====
mkdir -p crates/shared-crypto/src
cp $MICRO/docsign-web/docsign-wasm/src/crypto/keys.rs \
   crates/shared-crypto/src/keys.rs
cp $MICRO/docsign-web/docsign-wasm/src/crypto/cms.rs \
   crates/shared-crypto/src/cms.rs

# ===== COMPLIANCE ENGINE =====
mkdir -p crates/compliance-engine/src/rules
# From agentPDF-web (preferred implementations)
cp $MICRO/agentPDF-web/agentpdf/crates/agentpdf-compliance/src/rules/*.rs \
   crates/compliance-engine/src/rules/
cp $MICRO/agentPDF-web/agentpdf/crates/agentpdf-compliance/src/patterns.rs \
   crates/compliance-engine/src/
# From agentPDF-server (additional rules)
cp $MICRO/agentPDF-server/src/verifier/rules/florida.rs \
   crates/compliance-engine/src/rules/florida_additional.rs

# ===== CORPUS CORE =====
mkdir -p crates/corpus-core/src
cp -r $MICRO/corpus-server/corpus-core/src/* crates/corpus-core/src/

# ===== DOCSIGN CORE =====
mkdir -p crates/docsign-core/src
cp $MICRO/docsign-web/docsign-wasm/src/pdf/signer.rs crates/docsign-core/src/
cp $MICRO/docsign-web/docsign-wasm/src/pdf/audit.rs crates/docsign-core/src/

# ===== TYPST ENGINE =====
mkdir -p crates/typst-engine/src
cp -r $MICRO/agentPDF-server/src/compiler/* crates/typst-engine/src/
cp -r $MICRO/agentPDF-server/src/world/* crates/typst-engine/src/

# ===== APP: agentPDF-web =====
mkdir -p apps/agentpdf-web/{wasm/src,www,templates,tests}
cp -r $MICRO/agentPDF-web/www/* apps/agentpdf-web/www/
cp $MICRO/agentPDF-server/templates/florida_lease.typ apps/agentpdf-web/templates/

# ===== APP: docsign-web =====
mkdir -p apps/docsign-web/{wasm/src,www,worker/src,tests}
cp -r $MICRO/docsign-web/www/* apps/docsign-web/www/
cp -r $MICRO/docsign-web/docsign-server/src/* apps/docsign-web/worker/src/
```

---

## 7. Dual-Site Deployment Strategy

### 7.1 Build Configuration

**Workspace Cargo.toml:**
```toml
[workspace]
resolver = "2"
members = [
    "crates/*",
    "apps/agentpdf-web/wasm",
    "apps/docsign-web/wasm",
    "apps/docsign-web/worker",
    "apps/corpus-api",
    "apps/mcp-server",
]

[workspace.dependencies]
# Unified versions across all crates
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
wasm-bindgen = "0.2"
web-sys = "0.3"
lopdf = "0.33"
axum = "0.7"
```

### 7.2 CI/CD Pipeline

**.github/workflows/deploy.yml:**
```yaml
name: Deploy Sites

on:
  push:
    branches: [main]
  workflow_dispatch:
    inputs:
      site:
        description: 'Site to deploy'
        required: true
        default: 'both'
        type: choice
        options:
          - agentpdf
          - docsign
          - both

jobs:
  build-agentpdf:
    if: github.event.inputs.site != 'docsign'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Build WASM
        run: |
          cd apps/agentpdf-web/wasm
          wasm-pack build --target web --out-dir ../www/pkg

      - name: Deploy to Cloudflare Pages
        uses: cloudflare/pages-action@v1
        with:
          apiToken: ${{ secrets.CF_API_TOKEN }}
          accountId: ${{ secrets.CF_ACCOUNT_ID }}
          projectName: agentpdf-org
          directory: apps/agentpdf-web/www

  build-docsign:
    if: github.event.inputs.site != 'agentpdf'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Build WASM
        run: |
          cd apps/docsign-web/wasm
          wasm-pack build --target web --out-dir ../www/pkg

      - name: Deploy to Cloudflare Pages
        uses: cloudflare/pages-action@v1
        with:
          apiToken: ${{ secrets.CF_API_TOKEN }}
          accountId: ${{ secrets.CF_ACCOUNT_ID }}
          projectName: getsignatures-org
          directory: apps/docsign-web/www

      - name: Deploy Worker
        run: |
          cd apps/docsign-web/worker
          npx wrangler deploy
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CF_API_TOKEN }}
```

### 7.3 Local Development

**Makefile:**
```makefile
.PHONY: dev-agentpdf dev-docsign dev-all test build

# Development servers
dev-agentpdf:
	cd apps/agentpdf-web/wasm && wasm-pack build --target web --out-dir ../www/pkg
	cd apps/agentpdf-web/www && python3 -m http.server 8080

dev-docsign:
	cd apps/docsign-web/wasm && wasm-pack build --target web --out-dir ../www/pkg
	cd apps/docsign-web/www && python3 -m http.server 8081

dev-all:
	make dev-agentpdf & make dev-docsign

# Testing
test:
	cargo test --workspace

test-agentpdf:
	cargo test -p agentpdf-wasm
	cd apps/agentpdf-web/tests/e2e && npm test

test-docsign:
	cargo test -p docsign-wasm
	cd apps/docsign-web/tests/e2e && npm test

# Build
build:
	cargo build --workspace --release
	cd apps/agentpdf-web/wasm && wasm-pack build --target web --release
	cd apps/docsign-web/wasm && wasm-pack build --target web --release
```

---

## 8. Phase 0: ASAP Deployment

**Goal:** Get working versions deployed to both domains within days, not weeks.

**Strategy:** Copy existing microservices with minimal modifications.

### Strategic Alignment (from [STRATEGY.md](./STRATEGY.md))

Phase 0 aligns with **Florida Real Estate Dogfooding**—launching first to landlords and property managers:

| Strategic Priority | Phase 0 Action | Why First |
|--------------------|----------------|-----------|
| § 83.512 Flood Disclosure | Add Flood Disclosure Wizard to agentPDF.org | Regulatory urgency creates demand |
| HB 615 Email Consent | Hardcode consent checkbox in signature flow | Cost savings pitch to property managers |
| 30-Day Termination | Update templates (already in existing code) | Templates already in corpus |
| Offline-First Moat | Preserve existing IndexedDB architecture | Foundation for all verticals |

> **Dogfooding Strategy:** The corpus already contains Florida residential lease templates and related documents. Targeting this vertical first validates the product with a focused user persona before expanding to healthcare and legal field operations.

### 8.0.1 agentPDF.org - Immediate Deployment

The existing `agentPDF-web` is **already deployable**. Steps:

1. Copy `agentPDF-web/www/` to `apps/agentpdf-web/www/`
2. Copy WASM crate with dependencies
3. Build and deploy

```bash
# Quick deploy script
cd apps/agentpdf-web
cp -r ../../microservices/agentPDF-web/www/* www/
cp -r ../../microservices/agentPDF-web/agentpdf/crates/agentpdf-wasm wasm/

# Update Cargo.toml paths to use workspace dependencies
# Build
cd wasm && wasm-pack build --target web --out-dir ../www/pkg

# Deploy (Cloudflare Pages, Vercel, or Netlify)
cd ../www
# Upload to hosting provider
```

**Result:** agentPDF.org live with:
- PDF upload and viewing
- Florida compliance checking (10 rules)
- Field placement
- IndexedDB storage

**Florida RE Dogfooding Enhancements** (short-term priority):
- § 83.512 Flood Disclosure Wizard (interview-based form generation)
- HB 615 Email Consent checkbox (digitally verifiable audit trail)
- Updated 30-day termination language in templates
- Target users: Florida landlords, property managers, real estate agents

### 8.0.2 getsignatures.org - Immediate Deployment

The existing `docsign-web` is **already deployable**. Steps:

1. Copy `docsign-web/www/` to `apps/docsign-web/www/`
2. Copy WASM crate and worker
3. Build and deploy

```bash
# Quick deploy script
cd apps/docsign-web
cp -r ../../microservices/docsign-web/www/* www/
cp -r ../../microservices/docsign-web/docsign-wasm wasm/
cp -r ../../microservices/docsign-web/docsign-server worker/

# Build WASM
cd wasm && wasm-pack build --target web --out-dir ../www/pkg

# Deploy worker
cd ../worker && npx wrangler deploy

# Deploy static site
cd ../www
# Upload to hosting provider
```

**Result:** getsignatures.org live with:
- 4-step signing wizard
- PDF upload
- Recipient management
- PAdES digital signatures
- Email dispatch via Cloudflare Worker

**Field Ops Mode Enhancements** (medium-term, after Florida RE validation):
- GPS/photo evidence capture for process servers
- Medical Mode with HIPAA-compliant local encryption
- Offline-first sync for rural healthcare
- Target users: Mobile notaries, visiting nurses, process servers

### 8.0.3 Handoff Link

Add redirect from agentPDF.org to getsignatures.org:

```javascript
// apps/agentpdf-web/www/js/docsign-handoff.js
export function sendForSignatures(pdfBytes, filename) {
    const pdfBase64 = btoa(String.fromCharCode(...pdfBytes));
    const url = `https://getsignatures.org/#doc=${encodeURIComponent(pdfBase64)}&name=${encodeURIComponent(filename)}&source=agentpdf`;
    window.location.href = url;
}
```

---

## 9. Phase 1: Shared Foundation

**Timeline:** After Phase 0 deployment
**Goal:** Extract shared crates without breaking deployed sites

### 9.1 Create shared-types

```bash
mkdir -p crates/shared-types/src
```

**crates/shared-types/Cargo.toml:**
```toml
[package]
name = "shared-types"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
```

**crates/shared-types/src/lib.rs:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created_at: Option<i64>,
    pub page_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub rule_id: String,
    pub statute: String,
    pub severity: Severity,
    pub message: String,
    pub context: Option<String>,
    pub position: Option<TextPosition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPosition {
    pub page: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub document_id: String,
    pub violations: Vec<Violation>,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub checked_at: i64,
}
```

### 9.2 Create shared-pdf

Extract coordinate transformation and PDF parsing:

```rust
// crates/shared-pdf/src/coords.rs
// Copied from docsign-web/docsign-wasm/src/coords.rs

/// Transforms coordinates between DOM pixels and PDF user space
pub struct CoordinateTransform {
    pdf_width: f64,
    pdf_height: f64,
    dom_width: f64,
    dom_height: f64,
    scale: f64,
}

impl CoordinateTransform {
    pub fn new(pdf_width: f64, pdf_height: f64, dom_width: f64, dom_height: f64) -> Self {
        let scale = dom_width / pdf_width;
        Self { pdf_width, pdf_height, dom_width, dom_height, scale }
    }

    pub fn dom_to_pdf(&self, x: f64, y: f64) -> (f64, f64) {
        (x / self.scale, self.pdf_height - (y / self.scale))
    }

    pub fn pdf_to_dom(&self, x: f64, y: f64) -> (f64, f64) {
        (x * self.scale, (self.pdf_height - y) * self.scale)
    }
}
```

### 9.3 Update App Dependencies

Update `apps/agentpdf-web/wasm/Cargo.toml`:
```toml
[dependencies]
shared-types = { path = "../../../crates/shared-types" }
shared-pdf = { path = "../../../crates/shared-pdf" }
```

---

## 10. Phase 2: Unified Compliance Engine

**Goal:** Merge all Florida rules into single authoritative source.

### 10.1 Rule Inventory

| Rule | Source | Priority | Tests |
|------|--------|----------|-------|
| Radon Disclosure | agentPDF-server | Keep | Add |
| Lead Paint | agentPDF-server | Keep | Add |
| Bed Bug | agentPDF-server | Keep | Add |
| Prohibited Provisions | agentPDF-web | **Preferred** | 15 |
| Property Disposal | agentPDF-web | Keep | 8 |
| AS-IS Waivers | agentPDF-web | Keep | 12 |
| Security Deposit | **Merge both** | Combine | 20 |
| Attorney Fees | agentPDF-web | Keep | 10 |
| Notice Requirements | agentPDF-web | Keep | 15 |
| Month-to-Month | agentPDF-web | Keep | 10 |
| Late Fee | agentPDF-server | Keep | Add |
| Grace Period | agentPDF-server | Keep | Add |
| Anomaly Detection | agentPDF-server | Keep | Add |

### 10.2 Implementation

```rust
// crates/compliance-engine/src/lib.rs
use shared_types::{ComplianceReport, Violation, Severity};

pub trait ComplianceRule: Send + Sync {
    fn id(&self) -> &'static str;
    fn statute(&self) -> &'static str;
    fn check(&self, text: &str) -> Vec<Violation>;
}

pub struct FloridaComplianceEngine {
    rules: Vec<Box<dyn ComplianceRule>>,
}

impl FloridaComplianceEngine {
    pub fn new() -> Self {
        Self {
            rules: vec![
                // From agentPDF-server
                Box::new(rules::RadonDisclosureRule),
                Box::new(rules::LeadPaintRule),
                Box::new(rules::BedBugRule),
                Box::new(rules::LateFeeRule),
                Box::new(rules::GracePeriodRule),
                Box::new(rules::AnomalyDetectionRule),

                // From agentPDF-web (preferred implementations)
                Box::new(rules::ProhibitedProvisionsRule),
                Box::new(rules::PropertyDisposalRule),
                Box::new(rules::AsIsWaiversRule),
                Box::new(rules::SecurityDepositRule::unified()),
                Box::new(rules::AttorneyFeesRule),
                Box::new(rules::NoticeRequirementsRule),
                Box::new(rules::MonthToMonthRule),
            ],
        }
    }

    pub fn check(&self, text: &str) -> ComplianceReport {
        let mut violations = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut warnings = 0;

        for rule in &self.rules {
            let rule_violations = rule.check(text);
            if rule_violations.is_empty() {
                passed += 1;
            } else {
                for v in rule_violations {
                    match v.severity {
                        Severity::Critical | Severity::High => failed += 1,
                        Severity::Warning => warnings += 1,
                        _ => {}
                    }
                    violations.push(v);
                }
            }
        }

        ComplianceReport {
            document_id: String::new(),
            violations,
            passed,
            failed,
            warnings,
            checked_at: chrono::Utc::now().timestamp(),
        }
    }
}
```

### 10.3 Test Migration

Copy and merge all tests:

```bash
mkdir -p crates/compliance-engine/tests

# Copy agentPDF-web tests (90 tests)
cp ../microservices/agentPDF-web/agentpdf/crates/agentpdf-compliance/src/rules/*_test.rs \
   crates/compliance-engine/tests/

# Copy agentPDF-server verifier tests
cp ../microservices/agentPDF-server/tests/verifier_property_tests.rs \
   crates/compliance-engine/tests/
```

**Target:** 150+ tests after merging.

---

## 11. Phase 3: Full Integration

### 11.1 Template System Integration

Connect corpus templates to agentPDF.org:

```javascript
// apps/agentpdf-web/www/js/template-selector.js
const TEMPLATES = [
    { id: 'fl-lease-sf', name: 'Florida Single Family Lease', category: 'lease' },
    { id: 'fl-lease-mf', name: 'Florida Multifamily Lease', category: 'lease' },
    { id: 'fl-3day', name: '3-Day Notice (Non-Payment)', category: 'notice' },
    { id: 'fl-7day', name: '7-Day Notice (Violation)', category: 'notice' },
    { id: 'fl-30day', name: '30-Day Termination', category: 'notice' },
];

export function renderTemplateSelector(container) {
    // Render template cards
}

export async function loadTemplate(templateId) {
    // Fetch from corpus or embedded
}
```

### 11.2 Typst Rendering Integration

Add server-side rendering via corpus-api:

```rust
// apps/corpus-api/src/handlers/render.rs
use typst_engine::TypstEngine;

pub async fn render_template(
    State(engine): State<Arc<TypstEngine>>,
    Json(request): Json<RenderRequest>,
) -> Result<Json<RenderResponse>, AppError> {
    let pdf = engine.render(&request.template_id, &request.fields)?;
    Ok(Json(RenderResponse {
        pdf_base64: base64::encode(&pdf),
    }))
}
```

### 11.3 Cross-Site Deep Linking

Finalize handoff between sites:

```javascript
// apps/docsign-web/www/js/deep-link.js
export function parseIncomingDocument() {
    const hash = window.location.hash;
    if (hash.startsWith('#doc=')) {
        const params = new URLSearchParams(hash.slice(1));
        const docBase64 = params.get('doc');
        const filename = params.get('name') || 'document.pdf';
        const source = params.get('source');

        if (docBase64) {
            const bytes = Uint8Array.from(atob(docBase64), c => c.charCodeAt(0));
            loadDocumentFromBytes(bytes, filename);

            if (source === 'agentpdf') {
                showNotification('Document received from agentPDF.org');
            }
        }
    }
}
```

---

## 12. Test Coverage Strategy

### 12.1 Current Test Inventory

| Source | Test Type | Count |
|--------|-----------|-------|
| agentPDF-server | Integration | 15 |
| agentPDF-server | Property-based | 20 |
| agentPDF-web | Compliance rules | 90 |
| agentPDF-web | E2E (chromiumoxide) | 25 |
| corpus-server | Benchmarks | 10 |
| docsign-web | Property-based | 45 |
| docsign-web | Unit | 18 |
| docsign-web | E2E (Puppeteer) | 20 |
| **Total** | | **243** |

### 12.2 Post-Migration Targets

| Crate | Minimum Tests | Strategy |
|-------|---------------|----------|
| shared-types | 20 | Serialize/deserialize round-trips |
| shared-pdf | 30 | Coordinate transform properties |
| shared-crypto | 25 | Copy docsign-web crypto tests |
| compliance-engine | 150 | Merge both sources |
| corpus-core | 20 | Copy corpus-server tests |
| docsign-core | 50 | Copy docsign-web tests |
| typst-engine | 20 | Copy agentPDF-server tests |
| agentpdf-wasm | 30 | Copy agentPDF-web tests |
| docsign-wasm | 30 | Copy docsign-web tests |
| **Total** | **375** | 54% increase |

### 12.3 CI Test Configuration

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run tests
        run: cargo test --workspace

      - name: Run compliance tests
        run: cargo test -p compliance-engine -- --test-threads=1

  e2e-agentpdf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build WASM
        run: |
          cd apps/agentpdf-web/wasm
          wasm-pack build --target web --out-dir ../www/pkg
      - name: Run E2E
        run: |
          cd apps/agentpdf-web/tests/e2e
          npm install && npm test

  e2e-docsign:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build WASM
        run: |
          cd apps/docsign-web/wasm
          wasm-pack build --target web --out-dir ../www/pkg
      - name: Run E2E
        run: |
          cd apps/docsign-web/tests/e2e
          npm install && npm test
```

---

## 13. Demo Functionality Preservation

### 13.1 agentPDF.org Demo Features

From `agentPDF-web`:
- [ ] PDF upload via drag-drop or file picker
- [ ] PDF.js rendering with page navigation
- [ ] Click-to-place field editor
- [ ] Real-time compliance checking
- [ ] Violation highlighting with positions
- [ ] IndexedDB persistence
- [ ] Sample florida_lease.pdf included

### 13.2 getsignatures.org Demo Features

From `docsign-web`:
- [ ] 4-step wizard (Upload → Recipients → Fields → Review)
- [ ] Drag-to-reorder recipients
- [ ] 5 field types (signature, initials, date, text, checkbox)
- [ ] Signature capture pad
- [ ] PAdES digital signature generation
- [ ] Audit chain display
- [ ] Email dispatch via Cloudflare Worker
- [ ] Deep link support for incoming documents

### 13.3 Demo Verification Checklist

Before each deployment, verify:

```bash
#!/bin/bash
# scripts/verify-demos.sh

echo "=== agentPDF.org Demo Verification ==="
# 1. Upload sample PDF
# 2. Run compliance check
# 3. Place a field
# 4. Navigate pages
# 5. Check IndexedDB persistence

echo "=== getsignatures.org Demo Verification ==="
# 1. Upload sample PDF
# 2. Add 2 recipients
# 3. Place signature field
# 4. Complete signing
# 5. Verify audit chain
# 6. Test email dispatch
```

---

## 14. Future Considerations

### 14.1 Tauri Desktop Application

The [RESEARCH.md](./RESEARCH.md) proposes a Tauri desktop app. Once web deployment is stable:

```
apps/
├── agentpdf-web/          # Web version (current)
├── agentpdf-desktop/      # NEW: Tauri app
│   ├── src-tauri/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   └── main.rs    # Tauri entry point
│   │   └── tauri.conf.json
│   └── src/               # Shared frontend
```

Benefits from Research:
- Single binary distribution
- Native performance
- Offline-first by default
- rspc for type-safe IPC

### 14.2 Sync & Multi-Device

From Research Phase 2-3:
1. Add Operation Log table (Event Sourcing)
2. Implement CRDT-based sync
3. Optional cloud sync for cross-device

### 14.3 MCP Server Integration

The `apps/mcp-server/` provides Claude Desktop integration:
- Template selection via natural language
- Compliance checking via conversation
- Document generation via MCP tools

Per [STRATEGY.md](./STRATEGY.md), MCP is positioned as **AI Infrastructure**:
- "Trojan Horse" strategy: bundle with getsignatures.org as differentiator
- First AI-Ready e-signature platform
- OAuth2 authentication for enterprise
- List on directories: Glama, Smith.ai, MCP.so

### 14.4 Government Micro-Purchase Strategy

Per [STRATEGY.md](./STRATEGY.md) Section 5, target federal micro-purchases (<$10K):

| Requirement | Implementation |
|-------------|----------------|
| **Section 508 Compliance** | VPAT, screen reader support, keyboard nav |
| **Data Sovereignty** | Emphasize local-first, US-hosted |
| **SAM.gov Registration** | Keywords: "Disaster Response", "Rural Access" |
| **Pricing** | Annual license at $9,500 (under threshold) |

### 14.5 Vertical Mode Roadmap

Per [STRATEGY.md](./STRATEGY.md) Phase 2:

| Mode | Platform | Target Market | Key Features |
|------|----------|---------------|--------------|
| **Medical Mode** | getsignatures.org | Rural healthcare, visiting nurses | HIPAA encryption, EVV compliance, parking lot sync |
| **Field Ops Mode** | getsignatures.org | Process servers, mobile notaries | GPS stamping, photo evidence, evidentiary metadata |
| **Compliance Mode** | agentPDF.org | Florida landlords, property managers | Flood disclosure wizard, notice consent, template library |

---

## Summary

### Strategic Timeline (from [STRATEGY.md](./STRATEGY.md))

| Phase | Priority | Focus |
|-------|----------|-------|
| **Florida RE Dogfooding** | Short-Term | Landlords, property managers, real estate agents |
| **Field Ops Pivot** | Medium-Term | Medical Mode + Field Ops Mode (after FL validation) |
| **Government Scale** | Long-Term | Micro-purchase revenue |

### Short-Term: Florida RE Dogfooding

1. **Copy** existing `agentPDF-web` to `apps/agentpdf-web/`
2. **Copy** existing `docsign-web` to `apps/docsign-web/`
3. **Deploy** both sites to Cloudflare Pages
4. **Add** handoff link from agentPDF.org → getsignatures.org
5. **Add** § 83.512 Flood Disclosure Wizard to agentPDF.org
6. **Add** HB 615 Email Consent to signature flow
7. **Validate** with Florida landlords and property managers (dogfooding)

### Short-Term: Shared Foundation

8. **Extract** shared crates (shared-types, shared-pdf, shared-crypto)
9. **Unify** compliance engine with all 13 Florida rules
10. **Migrate** 243+ tests to monolith structure

### Medium-Term: Field Ops Pivot (after Florida RE validation)

11. **Integrate** template system from corpus-server
12. **Add** Typst rendering to agentPDF.org
13. **Implement** Medical Mode with HIPAA encryption
14. **Implement** Field Ops Mode with GPS/photo evidence

### Deployment URLs

| Site | Domain | Status |
|------|--------|--------|
| Compliance Platform | **agentPDF.org** | Ready for Phase 0 |
| Signature Platform | **getsignatures.org** | Ready for Phase 0 |

---

## Appendix A: Cargo.toml Templates

### Root Workspace

```toml
[workspace]
resolver = "2"
members = [
    "crates/shared-types",
    "crates/shared-pdf",
    "crates/shared-crypto",
    "crates/compliance-engine",
    "crates/corpus-core",
    "crates/docsign-core",
    "crates/typst-engine",
    "apps/agentpdf-web/wasm",
    "apps/docsign-web/wasm",
    "apps/docsign-web/worker",
    "apps/corpus-api",
    "apps/mcp-server",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["BobaMatchSolutions"]
license = "MIT"

[workspace.dependencies]
# Async
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
anyhow = "1"
thiserror = "1"

# WASM
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["console"] }
js-sys = "0.3"

# PDF
lopdf = "0.33"
pdf-extract = "0.7"

# Crypto
p256 = "0.13"
sha2 = "0.10"
base64 = "0.21"

# Web server
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "fs"] }

# Search
tantivy = "0.22"

# Database
lancedb = "0.13"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Testing
proptest = "1"
criterion = "0.5"
```

### Shared Types Crate

```toml
[package]
name = "shared-types"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
```

### Compliance Engine Crate

```toml
[package]
name = "compliance-engine"
version.workspace = true
edition.workspace = true

[dependencies]
shared-types = { path = "../shared-types" }
shared-pdf = { path = "../shared-pdf" }
serde = { workspace = true }
regex = "1"
once_cell = "1"

[dev-dependencies]
proptest = { workspace = true }
```

---

## Appendix B: Migration Checklist

### Pre-Migration

- [ ] Backup all microservice repositories
- [ ] Document current test counts per service
- [ ] Verify all microservices build successfully
- [ ] Note any pending PRs or WIP branches

### Phase 0

- [ ] Create monolith repository structure
- [ ] Copy agentPDF-web to apps/agentpdf-web
- [ ] Copy docsign-web to apps/docsign-web
- [ ] Update Cargo.toml paths
- [ ] Build WASM for both apps
- [ ] Deploy agentPDF.org
- [ ] Deploy getsignatures.org
- [ ] Verify demo functionality on both sites
- [ ] Add handoff link

### Phase 1

- [ ] Create shared-types crate
- [ ] Create shared-pdf crate
- [ ] Create shared-crypto crate
- [ ] Update app dependencies
- [ ] Run full test suite
- [ ] Deploy updates

### Phase 2

- [ ] Create compliance-engine crate
- [ ] Copy all rules from both sources
- [ ] Merge Security Deposit implementations
- [ ] Migrate all compliance tests
- [ ] Verify 150+ tests passing
- [ ] Update agentpdf-wasm to use unified engine
- [ ] Deploy updates

### Phase 3

- [ ] Copy corpus-core from corpus-server
- [ ] Create corpus-api app
- [ ] Add template selector to agentPDF.org
- [ ] Integrate Typst rendering
- [ ] Add deep link parsing to getsignatures.org
- [ ] Full E2E test of cross-site workflow
- [ ] Final deployment

### Post-Migration

- [ ] Archive microservice repositories (read-only)
- [ ] Update any documentation links
- [ ] Monitor error rates on both sites
- [ ] Gather user feedback
