# Monolith Integration Plan - Part 2 of 4

> **This is Part 2 of 4. See also:** [PLAN0.md](./PLAN0.md) (Index), [PLAN2.md](./PLAN2.md), [PLAN3.md](./PLAN3.md)

---

## Implementation Roadmap

### Week 1: Template Gap Fixes

| Task | File | Priority |
|------|------|----------|
| Add HOA/Condo Association Addendum | `florida_lease.typ` | P0 |
| Add CDD Disclosure Addendum | `florida_lease.typ` | P0 |
| Add Liquidated Damages Addendum (§ 83.595) | `florida_lease.typ` | P1 |
| Add explicit 30-day notice statutory reference | `florida_lease.typ` | P1 |
| Add Jury Trial Waiver clause | `florida_lease.typ` | P2 |
| Add Mold Prevention Addendum | `florida_lease.typ` | P2 |
| Add HB 621 Squatter Language | `florida_lease.typ` | P2 |

### Week 2: Purchase Template Creation

| Task | File | Priority |
|------|------|----------|
| Create base template structure | `florida_purchase_as_is.typ` | P0 |
| Implement As-Is inspection clause | `florida_purchase_as_is.typ` | P0 |
| Add Flood Disclosure integration | `florida_purchase_as_is.typ` | P0 |
| Add SB 264 Foreign Ownership section | `florida_purchase_as_is.typ` | P0 |
| Add Condo Rider (SIRS/Milestone) | `florida_purchase_as_is.typ` | P1 |
| Add Appraisal Gap clause | `florida_purchase_as_is.typ` | P2 |
| Add Kick-Out clause | `florida_purchase_as_is.typ` | P2 |

### Week 3: Listing Template Creation

| Task | File | Priority |
|------|------|----------|
| Create base template with Four Pillars | `florida_listing.typ` | P0 |
| Implement NAR-compliant commission structure | `florida_listing.typ` | P0 |
| Add fee negotiability disclosure | `florida_listing.typ` | P0 |
| Add pre-listing disclosure collection | `florida_listing.typ` | P0 |
| Add CDD/HOA/Condo conditional riders | `florida_listing.typ` | P1 |
| Add protection period clause | `florida_listing.typ` | P1 |
| Add Coastal/CCCL rider | `florida_listing.typ` | P2 |

### Week 4: Metro Detection & Integration

| Task | Location | Priority |
|------|----------|----------|
| Create FL zip → metro JSON mapping | `wasm/src/data/` | P0 |
| Implement military base proximity check | `wasm/src/geo.rs` | P0 |
| Add metro detection to template selector | `www/js/template-selector.js` | P0 |
| Wire up conditional addenda based on metro | `www/js/template-selector.js` | P1 |
| Add "Coming Soon" state selector UI | `www/index.html` | P2 |

### Week 5: Demo Polish & Testing

| Task | Priority |
|------|----------|
| Create Demo 1 script (overview) | P0 |
| Create Demo 2 script (lease deep dive) | P0 |
| Create Demo 3 script (purchase deep dive) | P0 |
| Create Demo 4 script (listing deep dive) | P0 |
| Update Tampa landing page with new features | P1 |
| Test all flows end-to-end | P0 |
| Create sample documents for each contract type | P1 |

---

## 16-STATE COMPLIANCE ENGINE (Coming Soon Display)

The existing compliance engine supports 16 states with 227 tests. For the Tampa demo, display these as "Coming Soon":

```javascript
const STATE_STATUS = {
  available: {
    FL: { name: "Florida", tests: 31, templates: ["lease", "purchase", "listing"] }
  },
  coming_soon: {
    TX: { name: "Texas", tests: 15, eta: "Q2 2026" },
    CA: { name: "California", tests: 18, eta: "Q2 2026" },
    NY: { name: "New York", tests: 12, eta: "Q2 2026" },
    GA: { name: "Georgia", tests: 10, eta: "Q2 2026" },
    IL: { name: "Illinois", tests: 11, eta: "Q2 2026" },
    PA: { name: "Pennsylvania", tests: 8, eta: "Q3 2026" },
    NJ: { name: "New Jersey", tests: 9, eta: "Q3 2026" },
    VA: { name: "Virginia", tests: 8, eta: "Q3 2026" },
    MA: { name: "Massachusetts", tests: 7, eta: "Q3 2026" },
    OH: { name: "Ohio", tests: 6, eta: "Q3 2026" },
    MI: { name: "Michigan", tests: 7, eta: "Q3 2026" },
    WA: { name: "Washington", tests: 8, eta: "Q3 2026" },
    AZ: { name: "Arizona", tests: 6, eta: "Q3 2026" },
    NC: { name: "North Carolina", tests: 5, eta: "Q3 2026" },
    TN: { name: "Tennessee", tests: 5, eta: "Q3 2026" }
  }
};
```

**UI Treatment:**
- Florida: Green checkmark, fully clickable
- Others: Lock icon with state outline, "Coming Q2 2026" tooltip
- Clicking locked state shows: "Join waitlist for [State] launch notification"

---

## Quick Reference

| Domain | Purpose | Source Microservice | Priority |
|--------|---------|---------------------|----------|
| **agentPDF.org** | Compliance checking + template population | agentPDF-web + corpus-server | High |
| **getsignatures.org** | Standalone digital signatures | docsign-web | High |

**Goal:** Deploy simple working versions to both domains ASAP, then iterate.

**Strategic Context:** See [STRATEGY.md](./STRATEGY.md) for market positioning, vertical targeting, and go-to-market approach.

---

## PRIORITY: Florida Compliance Features

> **Local-First Template Generation: ✅ COMPLETE** - Template rendering runs entirely in browser via WASM. Zero server cost per document.
>
> **Next Priority: Florida Regulatory Compliance** - The local-first architecture enables $0 marginal cost, which unlocks the "Free Local, Paid Cloud" business model. Now we need the Florida-specific features that create market urgency.

### Florida Regulatory Deadlines

| Feature | Statute | Priority | Status |
|---------|---------|----------|--------|
| **Email Consent Addendum** | HB 615 | SHORT-TERM | ✅ COMPLETE |
| **Flood Disclosure Wizard** | SB 948 / § 83.512 | MEDIUM-TERM | ✅ COMPLETE |
| **Tampa Landing Page** | Marketing | SHORT-TERM | ✅ COMPLETE |
| **Tampa Demo Script** | Marketing | SHORT-TERM | ✅ COMPLETE |
| **30-Day Termination Notice** | § 83.57 | SHORT-TERM | ⚠️ Template needs update |

### § 83.512 Flood Disclosure (MEDIUM-TERM PRIORITY) - ✅ COMPLETE

**Risk**: Landlords who fail to provide this disclosure can face:
- Tenant can terminate lease immediately
- Tenant can demand full rent refund
- Creates "voidability risk" for every lease that lacks this addendum

**Implementation**: Neutral tristate wizard that generates compliant form:

```
Step 1: "Property flooding history"          → [Yes] [No] [I don't know]
Step 2: "Flood insurance claims"             → [Yes] [No] [I don't know]
Step 3: "Federal flood assistance (FEMA)"    → [Yes] [No] [I don't know]
                                               ↓
                    [Generate § 83.512 Compliant Disclosure Form]
```

**Scrivener Adherence**: Per strict neutrality requirements, the wizard:
- Offers 3 options including "I don't know / Property recently acquired"
- Defaults to "I don't know" (doesn't lead user either way)
- Uses neutral phrasing without implying a "correct" answer
- Complies with form generation best practices for legal documents

### HB 615 Email Consent (SHORT-TERM PRIORITY) - ✅ COMPLETE

**Value Prop**: "Stop paying for Certified Mail. Get the free form to make Email Legal in Florida."

**Implementation**: The TENANT signs consent during signature ceremony (not pre-filled by landlord):

```
Template generates:
┌────────────────────────────────────────────────────────────────┐
│  TENANT'S ELECTION (HB 615)                                    │
│  (Tenant: Please check ONE option below during signing)        │
│                                                                │
│  ☐ I CONSENT to receive notices via email                      │
│  ☐ I DECLINE and require postal mail                           │
│                                                                │
│  Email: [tenant@email.com]                                     │
│  Signature: ________________________                           │
└────────────────────────────────────────────────────────────────┘
```

**Scrivener Adherence**:
- Both options unchecked by default (tenant must actively choose)
- Tenant signs during getsignatures.org ceremony
- Not pre-filled by landlord in template form

---

## ✅ COMPLETE: Local-First Template Generation

### Architecture Change

```
BEFORE (Server-Side):
Browser → HTTP API → MCP Server (Typst) → PDF → Browser
         ~~~~~~~~~~~~~~~~~~~~~~~~~~~~
         Server cost per request

AFTER (Local-First):
Browser → WASM (Typst) → PDF
          ~~~~~~~~~~~~
          $0 marginal cost, runs entirely client-side
```

### Implementation Summary

| Component | Status | Notes |
|-----------|--------|-------|
| `compile_document_sync()` | ✅ Done | Sync version of Typst compilation for WASM |
| Feature flags (`server`/`wasm`) | ✅ Done | tokio optional via feature flag |
| `render_template()` WASM export | ✅ Done | Exposed in agentpdf-wasm |
| `render_typst()` WASM export | ✅ Done | Raw Typst source rendering |
| `list_templates()` WASM export | ✅ Done | Template discovery |
| `validate_typst_syntax()` WASM | ✅ Done | Syntax validation |

### WASM API (agentpdf-wasm)

```javascript
// Render embedded template to PDF (returns base64)
const pdfBase64 = wasm.render_template("florida_lease", JSON.stringify({
  landlord_name: "John Smith",
  tenant_name: "Jane Doe",
  property_address: "123 Main St, Miami, FL 33101",
  monthly_rent: "2000"
}));

// Render raw Typst source
const pdfBase64 = wasm.render_typst("Hello, *World*!", "{}");

// List available templates
const templates = JSON.parse(wasm.list_templates());

// Validate syntax before rendering
const errors = JSON.parse(wasm.validate_typst_syntax("#let x = "));
```

### Cost Comparison

| Model | Server Cost | UX |
|-------|-------------|-----|
| Server-side (MCP) | Per-request | Fast first render |
| **Local-first (WASM)** | **$0** | ~20-40MB initial download, then instant |
| Hybrid (cache) | $0 after first load | Best of both worlds |

### Trade-offs

**WASM Bundle Size** (mitigated by lazy loading):
- Typst compiler: ~15-20 MB
- typst-assets fonts: ~25-30 MB
- Application code: ~2-3 MB
- **Total: ~45-55 MB** (lazy-loaded on first template render)

**Mitigation**: Compliance checking (small WASM) loads immediately. Template rendering (large) loads on-demand when user clicks "Generate".

### UI Integration

| Component | Status | Notes |
|-----------|--------|-------|
| Template selector modal | ✅ Done | Loads templates from WASM |
| Form generation | ✅ Done | Dynamic form from template fields |
| Local-first rendering | ✅ Done | Falls back to API if WASM unavailable |
| PDF viewer integration | ✅ Done | Generated PDF loads in viewer |

### Performance (Measured)

- **WASM bundle size**: 26 MB uncompressed, 8.3 MB brotli
- **Template render time**: ~650ms (florida_lease, 14 pages)
- **Server cost**: $0 per document

### Files Changed

- `crates/typst-engine/Cargo.toml` - Added `server`/`wasm` feature flags
- `crates/typst-engine/src/compiler/render.rs` - Added `compile_document_sync()`
- `crates/typst-engine/src/lib.rs` - Feature-gated exports
- `apps/agentpdf-web/wasm/src/lib.rs` - Added template rendering exports
- `apps/agentpdf-web/www/js/template-selector.js` - Local-first rendering logic
- `apps/agentpdf-web/www/index.html` - Template callback integration

---

## Current Progress

### ✅ Phase 0: ASAP Deployment - COMPLETE

| Component | Status | Tests | Notes |
|-----------|--------|-------|-------|
| **Workspace Structure** | ✅ Complete | - | Cargo.toml with all crates and shared dependencies |
| **shared-types** | ✅ Tests Pass | 22 | Document, Violation, ComplianceReport types |
| **shared-pdf** | ✅ Tests Pass | 30 | PDF parsing, coordinate transforms, signer |
| **shared-crypto** | ✅ Tests Pass | 33 | ECDSA P-256, CMS/PKCS#7, certificates, TSA |
| **compliance-engine** | ✅ Tests Pass | 268 | 16 states + Florida real estate (property tests) |
| **docsign-core** | ✅ Tests Pass | 2 | PAdES signing, audit chain |
| **typst-engine** | ✅ Tests Pass | 59 | Document rendering, 6 templates, verifier, registry tests |
| **mcp-server** | ✅ Tests Pass | 29 | Claude Desktop MCP with HTTP transport, REST API, property tests |
| **agentpdf-wasm** | ✅ Tests Pass | 82 | WASM bindings + compliance integration |
| **docsign-wasm** | ✅ Tests Pass | 63 | WASM bindings + signing workflow |
| **docsign-worker** | ✅ Tests Pass | 31 | Cloudflare Worker + session property tests |
| **CI/CD** | ✅ Set up | - | GitHub Actions for fmt, clippy, tests, WASM |
| **Pre-commit Hook** | ✅ Installed | - | Runs fmt, clippy, tests before commit |
| **Demo Verification** | ✅ Complete | - | Both demos verified with Puppeteer |

### ✅ Phase 1: Shared Foundation - COMPLETE

| Component | Status | Notes |
|-----------|--------|-------|
| **shared-types** | ✅ Complete | Document, Violation, Severity, ComplianceReport |
| **shared-pdf** | ✅ Complete | PDF parsing, coordinate transforms, PAdES signer |
| **shared-crypto** | ✅ Complete | ECDSA P-256, CMS/PKCS#7, TSA integration |

### ✅ Phase 2: Unified Compliance Engine - COMPLETE

| Component | Status | Notes |
|-----------|--------|-------|
| **compliance-engine** | ✅ Complete | 10 Florida Chapter 83 rules |
| **Rules: prohibited** | ✅ Complete | § 83.47 prohibited provisions |
| **Rules: deposit** | ✅ Complete | § 83.49 security deposit requirements |
| **Rules: attorney_fees** | ✅ Complete | § 83.48 attorney fee reciprocity |
| **Rules: notices** | ✅ Complete | § 83.56, § 83.57 notice requirements |
| **Pattern matching** | ✅ Complete | Regex-based violation detection |

### ✅ Phase 3: Full Integration - COMPLETE

| Component | Status | Notes |
|-----------|--------|-------|
| **typst-engine templates** | ✅ Complete | 6 templates (invoice, letter, florida_lease, florida_purchase_contract, florida_escalation_addendum, florida_listing_agreement) |
| **MCP render_document** | ✅ Complete | Template rendering via MCP protocol |
| **MCP list_templates** | ✅ Complete | Template discovery |
| **REST API /api/templates** | ✅ Complete | HTTP endpoint for web clients (with CORS) |
| **REST API /api/render** | ✅ Complete | HTTP endpoint for template rendering |
| **agentpdf handoff (sender)** | ✅ Complete | DocsignHandoff module with sessionStorage |
| **docsign handoff (receiver)** | ✅ Complete | Auto-loads documents from agentpdf |
| **Template selector UI** | ✅ Complete | Modal UI for template selection + form filling |
| **Deep link parsing** | ✅ Complete | Signing links + agentpdf integration |

### ✅ Phase 3.5: Florida Real Estate Templates - COMPLETE

Added comprehensive Florida real estate transaction support:

| Component | Status | Notes |
|-----------|--------|-------|
| **florida_purchase_contract.typ** | ✅ Complete | Residential purchase contract with all mandatory disclosures |
| **florida_escalation_addendum.typ** | ✅ Complete | Competitive offer escalation clause with max price cap |
| **florida_listing_agreement.typ** | ✅ Complete | Exclusive listing with § 475.278 brokerage disclosure |
| **florida_realestate.rs compliance** | ✅ Complete | 9 check functions for real estate document compliance |
| **Property tests (proptest)** | ✅ Complete | Fuzz testing for all compliance rules |
| **Template integration tests** | ✅ Complete | Registry tests for new templates |

**Real Estate Compliance Coverage:**
- § 404.056 - Radon Gas Disclosure
- § 689.261 - Property Tax Disclosure
- § 689.302 - Flood Disclosure (SB 948, October 2025)
- § 720.401 - HOA Disclosure
- § 553.996 - Energy Efficiency Disclosure
- § 475.278 - Brokerage Relationship Disclosure
- § 475.25 - Definite Expiration Date (Listing Agreements)
- 42 U.S.C. § 4852d - Lead Paint Disclosure (pre-1978)
- Johnson v. Davis (1985) - Material Defect Disclosure

**Bug Fix (Test-First):** Fixed lead paint disclosure incorrectly triggering for properties built exactly in 1978. The law applies to pre-1978 properties only.

**Total Tests: 510+ passing** (including new property tests for Florida real estate compliance)

### ✅ Quality Checks

| Check | Status |
|-------|--------|
| **cargo test --workspace --all-features** | ✅ 510+ tests passing |
| **cargo clippy --workspace --all-features -- -D warnings** | ✅ Clean (compliance-engine, typst-engine) |
| **cargo fmt --all -- --check** | ✅ Formatted |
| **WASM Compilation (agentpdf-wasm)** | ✅ Compiles (wasm-opt disabled) |
| **WASM Compilation (docsign-wasm)** | ✅ Compiles (wasm-opt disabled) |
| **docsign-worker** | ✅ Compiles | Upgraded to worker 0.7 |
| **Demo Verification (Puppeteer)** | ✅ Both apps working |
| **Trunk Build System** | ✅ Migrated | Both apps use `trunk serve/build` |
| **Property Testing (proptest)** | ✅ Complete | Fuzz testing for Florida real estate compliance |

### ✅ Dev Tooling: Trunk Migration

Migrated from Python `http.server` to **Trunk** for local development:
- **Single command**: `trunk serve www/index.html` builds WASM + serves with hot reload
- **Production build**: `trunk build www/index.html --release` outputs to `www/dist/`
- **No manual wasm-pack**: Trunk handles wasm-bindgen and bundling automatically
- **State data from Rust**: StateSelector now loads states + statute citations from WASM (no JS duplication)

### ⏸️ Blocked/Deferred

| Component | Status | Reason |
|-----------|--------|--------|
| **corpus-core** | ⏸️ Blocked | Version conflicts between candle-core, rand, and half crates |
| **corpus-api** | ⏸️ Blocked | Depends on corpus-core |

**corpus-core Details:**
- Code uses `candle-core`, `candle-nn`, `tokenizers`, `hf-hub` for BGE-M3 embeddings
- Candle 0.8.x has compatibility issues with rand 0.9.x and half 2.7.x
- Options: (1) Wait for candle 0.9 stable release, (2) Rewrite with fastembed, (3) Use remote embedding API
- Not critical for MVP - semantic search is an advanced feature

### 📋 Next Steps (Post Phase 3) - UPDATED FOR LOCAL-FIRST PIVOT

#### ✅ COMPLETED (December 2025)

| Priority | Task | Status |
|----------|------|--------|
| P0 | **HB 615 Email Consent Addendum** - Added to florida_lease.typ (Addendum G) | ✅ Done |
| P0 | **§ 83.512 Flood Disclosure** - Added to florida_lease.typ (Addendum H) + compliance check | ✅ Done |
| P0 | **agentPDF.org/tampa landing page** - Tampa Bay landlord-focused landing page | ✅ Done |
| P0 | **Template metadata update** - 11 optional fields including HB 615 & flood disclosure | ✅ Done |

#### SHORT-TERM (Next)

| Priority | Task | Owner |
|----------|------|-------|
| P0 | **Deploy to production** - Push to agentpdf.org and getsignatures.org | Engineering |
| P1 | **30-day termination update** - Updated notices.rs + florida_lease.typ for HB 1417 (2023) | ✅ Done |
| P1 | **Tampa REIA outreach** - Demo at January 2026 meetings | Human/Marketing |
| P1 | **Texas Lease Template** - Created texas_lease.typ with Ch. 92 compliance | ✅ Done |

#### MEDIUM-TERM

| Priority | Task | Owner |
|----------|------|-------|
| P1 | **NARPM Florida Chapter sponsorship** - $200-500 lunch sponsor | Human/Marketing |
| P2 | **Florida Landlord Network webinar** - "Is Your Lease Compliant?" pitch | Human/Marketing |
| P2 | **Standalone Flood Disclosure form** - Quick-generate just the SB 948 form | Engineering |

#### Deferred (Not Needed for MVP)

| Feature | Reason |
|---------|--------|
| OAuth between sites | Not needed for free tier |
| Cloud Sync | Paid feature - defer until paying users |
| Medical Mode | Phase 2 after FL RE validation |
| corpus-core semantic search | Advanced feature, not critical for launch |

### 📍 Human Action Items (Marketing Ground Game)

> These tasks require physical presence in Tampa and cannot be automated.

#### January 2026 Target Events

| Event | Date | Location | Action | Expected Outcome |
|-------|------|----------|--------|------------------|
| **Tampa REIA Main Meeting** | Thu Jan 9, 2026 | Tampa (check venue) | Demo app in "Kiosk Mode", hand out cards | 5 beta testers |
| **Beach REIA (Pinellas)** | Thu Jan 16, 2026 | Clearwater area | QR code → agentpdf.org/tampa | Email list growth |
| **NARPM Tampa Chapter** | Check calendar | TBD | Sponsor lunch ($200-500), pitch "Offline Reliability" | Property manager trials |

#### Recurring Monthly Events

| Event | When | Action |
|-------|------|--------|
| **Tampa REIA** | 2nd Thursday monthly | Demo flood disclosure wizard |
| **Beach REIA** | 3rd Thursday monthly | QR code → landing page |
| **Florida Landlord Network** | Newsletter sponsorship | "Compliance" educational content |

**The Script (for REIA meetings):**
> "I'm a local developer here in Tampa. I was reviewing the new statutes on flood disclosures and noticed most free forms online are outdated. I built a free tool to generate the new mandatory SB 948 form so we don't get sued. I'm not selling anything; I just want to make sure the local community has the right docs."

**Demo Checklist:**
1. Open agentpdf.org/tampa on phone/tablet
2. Show "Flood Disclosure" card → "Generate Form"
3. Fill quick form → Download PDF in seconds
4. "Works offline - perfect for showings"

---

## Phase 4: Nationwide Template Expansion

> **Full Research**: See [LEASE_RESEARCH.md](./LEASE_RESEARCH.md) for comprehensive legal analysis.

### Strategic Template Use Cases

| Use Case | Description | Priority | Research |
|----------|-------------|----------|----------|
| **Residential Lease** | 50-state lease automation with compliance | High | [LEASE_RESEARCH.md](./LEASE_RESEARCH.md) |
| **Tax Preparation** | TurboTax competitor with IRS-compliant forms | High | [TAX_RESEARCH.md](./TAX_RESEARCH.md) |
| **Estate Planning** | Wills, trusts, POA with UPL-safe statutory forms | High | [ESTATE_RESEARCH.md](./ESTATE_RESEARCH.md) |
| **Commercial Lease** | Office, retail, industrial leases | Medium | - |
| **Property Management** | Notices, addendums, disclosures | Medium | - |

### The "Layer Cake" Architecture

Templates must support hierarchical compliance:

```
┌─────────────────────────────────────────────────────────────┐
│  VARIABLE LAYER - User inputs (rent, dates, parties)       │
├─────────────────────────────────────────────────────────────┤
│  LOCAL LAYER - City ordinances (Chicago RLTO, SF rent ctrl) │
├─────────────────────────────────────────────────────────────┤
│  STATE LAYER - Statutory requirements (deposits, notices)   │
├─────────────────────────────────────────────────────────────┤
│  FEDERAL LAYER - Lead paint, Fair Housing (baseline)        │
└─────────────────────────────────────────────────────────────┘
```

### Rollout Strategy: Volume/Complexity Matrix

| Tier | States | Strategy | Status |
|------|--------|----------|--------|
| **Tier 0** | FL | ✅ Complete (10 rules in compliance-engine) | ✅ 31 tests |
| **Tier 1: Big Five** | TX, CA, NY, GA, IL | High volume, prove platform capability | ✅ Complete (76 tests) |
| **Tier 2: Growth** | PA, NJ, VA, MA, OH, MI, WA, AZ, NC, TN | Regional importance | ✅ Complete (111 tests) |
| **Tier 3: URLTA Block** | AK, KS, KY, NE, NM, OR, RI + others | Clone master template | Pending |
| **Tier 4: Long Tail** | Remaining states | Complete coverage | Pending |

### Phase 4 Implementation Plan

**Short Term: Foundation** ✅ COMPLETE
- [x] Extend compliance-engine for multi-jurisdiction support
- [x] Implement Federal layer (lead paint, Fair Housing)
- [x] Add Texas and Georgia rules (Tier 1)
- [x] Add California (with AB 12 deposit cap, SB 611 junk fees)
- [x] Add Illinois (with Chicago RLTO support)
- [x] Add New York (with NYC rent stabilization, late fee caps)

**Medium Term: Growth Hubs** ✅ COMPLETE
- [x] Add Pennsylvania (Plain Language Act, 2-month deposit cap)
- [x] Add New Jersey (Truth in Renting, Anti-Eviction Act)
- [x] Add Virginia (HB 2430 Fee Transparency, mold disclosure)
- [x] Add Massachusetts (Broker Fee Reform, 1-month deposit cap)
- [x] Add Ohio (30-day deposit return, itemized deductions)
- [x] Add Michigan (Source of Income Protection, inventory checklist)
- [x] Add Washington (90-day rent increase notice, Just Cause cities)
- [x] Add Arizona (Bed bug disclosure, pool safety, 1.5-month deposit cap)
- [x] Add North Carolina (Pet Fee vs Pet Deposit distinction, trust account)
- [x] Add Tennessee (URLTA county applicability based on population)

**Long Term: Scale & Coverage**
- [ ] Roll out URLTA block (AK, KS, KY, NE, NM, OR, RI)
- [ ] Complete 50-state coverage
- [ ] Build zip code → municipality mapping for local ordinances
- [ ] Real-time legislative monitoring

---

## Phase 5: Tax Preparation Platform

> **Full Research**: See [TAX_RESEARCH.md](./TAX_RESEARCH.md) for comprehensive IRS compliance analysis.

### Tax Form Architecture

The tax product uses a hierarchical form structure similar to the Layer Cake:

```
┌─────────────────────────────────────────────────────────────┐
│  FORM 1040 - Master Return (anchors all schedules)          │
├─────────────────────────────────────────────────────────────┤
│  NUMBERED SCHEDULES (1, 2, 3) - Aggregate categories        │
├─────────────────────────────────────────────────────────────┤
│  LETTERED SCHEDULES (A-SE) - Specific tax situations        │
├─────────────────────────────────────────────────────────────┤
│  WORKSHEETS - Intermediate calculations (not filed)         │
├─────────────────────────────────────────────────────────────┤
│  SOURCE DOCUMENTS - W-2, 1099s (import/display)             │
└─────────────────────────────────────────────────────────────┘
```

### Form Priority Matrix

| Priority | Forms | Rationale |
|----------|-------|-----------|
| **P0: Core** | 1040, 1040-SR | Required for all returns |
| **P1: Income** | W-2, 1099-NEC, 1099-MISC | Gig economy, self-employed |
| **P2: Deductions** | Schedule C, SE | Sole proprietor focus |
| **P3: Itemized** | Schedule A, B, D | Investment income, itemizers |
| **P4: Complex** | Schedule E, K-1 | Rental, partnerships |

### Phase 5 Implementation Plan

**Short Term: Foundation**
- [ ] Implement Form 1040 / 1040-SR PDF generation (IRS Pub 1167 compliant)
- [ ] Build Schedule C engine for self-employed/gig workers
- [ ] Create W-2 and 1099-NEC import and display
- [ ] Build calculation engine for basic tax math
- [ ] Add interview-based data collection flow

**Medium Term: Expansion**
- [ ] Add Schedule SE (self-employment tax)
- [ ] Add Schedules A, B, D for itemizers and investors
- [ ] Implement California FTB 540 integration
- [ ] Build state-aware routing logic
- [ ] Add MeF XML generation for e-file

**Long Term: Full Platform**
- [ ] Add Schedule E (rental income)
- [ ] Add K-1 passthrough support
- [ ] Complete 1099 family (INT, DIV, B, R, G)
- [ ] Add New York IT-201 with 2D barcode
- [ ] Implement IRS e-file transmission
- [ ] Add tax planning and projection features

---

## Phase 6: Estate Planning Platform

> **Full Research**: See [ESTATE_RESEARCH.md](./ESTATE_RESEARCH.md) for comprehensive UPL analysis and statutory form research.

### Market Context: The Great Wealth Transfer

The US is undergoing an unprecedented wealth transfer of **$16-84 trillion** from Baby Boomers to Gen X/Millennials. Traditional legal services fail the middle class due to prohibitive costs.

**Opportunity**: Statutory-compliant PDF builder operating in "safe harbor" of validity.

### The UPL Firewall

**Critical Risk**: Unauthorized Practice of Law (UPL) is a crime in most states.

| Requirement | Implementation |
|-------------|----------------|
| **Verbatim Input** | Software populates forms exactly as entered, no interpretation |
| **Statutory Forms** | Use state-promulgated forms, not proprietary instruments |
| **Scrivener Logic** | Present options for selection, never recommend |
| **Explicit Disclaimers** | Clear notice: not a lawyer, no legal advice |

**The Scrivener Doctrine**:
```
PROHIBITED (Advisory): "Based on your $5M assets, we recommend a Credit Shelter Trust"
PERMITTED (Scrivener): "Do you want a Credit Shelter Trust? [Tooltip: defined as...]"
```

### Tier 1 Markets (Big Four)

| State | Wealth Index | Primary Need | Statutory Forms Available |
|-------|-------------|--------------|---------------------------|
| **California** | 2.71 | Revocable Living Trusts (probate avoidance) | Probate Code § 6240, § 4701 |
| **New York** | 1.95 | Updated POA (2021 overhaul) | GOL § 5-1513 |
| **Texas** | 0.62 | Independent Administration Wills | Supreme Court approved forms |
| **Florida** | 0.33 | Advance Directives | F.S. Chapter 765 |

### Phase 6 Implementation Plan

**Short Term: Foundation**
- [ ] Implement California Statutory Will (Probate Code § 6240)
- [ ] Implement Texas Supreme Court approved wills (all 4 variants)
- [ ] Build California Advance Health Care Directive
- [ ] Build Texas Statutory Durable POA
- [ ] Create state-specific Signing Instruction Sheet generator
- [ ] Implement UPL-compliant Terms of Service
- [ ] Add scrivener-style tooltips (factual, not advisory)

**Medium Term: Full Big Four Coverage**
- [ ] Add New York Statutory Short Form POA (2021 version)
- [ ] Add New York Health Care Proxy and Living Will
- [ ] Add Florida Health Care Surrogate and Living Will
- [ ] Build California Revocable Living Trust engine
- [ ] Implement Trust Certification generator (CA § 18100.5)
- [ ] Integrate Remote Online Notarization APIs for TX/FL

**Long Term: Platform Expansion**
- [ ] Add secondary markets (MA, WA - high home values)
- [ ] Build Attorney Assist network (Tier 3 upsell)
- [ ] Implement e-Will support (NV, IN, FL)
- [ ] Add audit trail infrastructure for electronic signatures
- [ ] Build B2B2C partner portal for financial advisors
- [ ] Complete document suite with Self-Proving Affidavits

---

**Continue to [PLAN2.md](./PLAN2.md) for Phase 7: Web Performance Benchmarking and Architecture Overview.**
