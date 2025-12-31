# agentPDF Strategic Plan

> Florida-First Document Generation & Compliance Platform

**Document Version**: 3.1
**Last Updated**: December 31, 2025
**Status**: Active Development - Florida MVP

---

## Scope Philosophy

**Perfect for Florida first, then expand.**

This plan focuses exclusively on Florida documents until the platform is production-ready. Multi-state support (TX, CA, NY, etc.) is archived for future phases but deliberately excluded from MVP to ensure quality.

### Core Principles

1. **Florida Excellence** - Every Florida document type must be legally compliant and user-tested before expanding scope
2. **Scrivener Standard** - Present options, never recommend (see Part IV)
3. **Self-Contained Experience** - Template completion engine embedded directly; NO external redirects
4. **Template Strategy** - Server-side Typst rendering + client-side form completion
5. **No Contract Drafting** - Users complete templates, not write contracts (prevents UPL concerns)

---

## Part I: Document Type Hierarchy

### Phase 1.0 - Core Florida Documents

```
Florida::
├── Lease::
│   ├── Agreement              # Ch. 83 Part II residential lease
│   ├── TerminationNotice      # 7/15/30-day notices (83.57)
│   └── Eviction               # 3-day notice, eviction complaint
│
├── Purchase::
│   ├── Contract               # Standard + As-Is variants
│   ├── OptionalContingencies::
│   │   ├── Inspection         # Inspection period addendum
│   │   └── Financing          # Mortgage contingency
│   └── Addendum::
│       └── Escalation         # Escalation clause addendum
│
├── Listing::
│   └── Exclusive              # Exclusive right to sell (Ch. 475)
│
└── Contractor::
    ├── Invoice                # Standard contractor invoice
    ├── CostOfMaterialsBill    # Materials cost breakdown
    ├── NoticeOfCommencement   # Ch. 713.13 - starts lien period
    ├── NoticeOfLien           # Ch. 713 - notice to owner
    ├── DisputeLien            # Contest/dispute filed lien
    └── FraudulentLienReporting # Report fraudulent lien filing
```

### Phase 1.1 - Bill of Sale (High-Volume Florida Transactions)

```
Florida::
└── BillOfSale::
    ├── Car                    # Motor vehicle (HSMV 82050)
    ├── Boat                   # Vessel (HSMV 87002)
    ├── Trailer                # Trailer title transfer
    ├── JetSki                 # PWC/watercraft
    └── MobileHome             # Mobile home (Ch. 319)
```

---

## Part II: Architecture Decision - Template Generation Strategy

### 2.1 The WASM Size Question

**Problem**: Full typst-engine WASM bundle is ~75MB (8.3MB brotli). This is too large for web delivery.

**Two Approaches**:

| Approach | Pros | Cons |
|----------|------|------|
| **A: Server-side Typst** | Small frontend (<500KB), fast renders (~100ms), caching | Requires server, ~$0.001/doc |
| **B: Pre-generated PDFs** | Zero server cost, works offline, users familiar with PDF editing | Less flexible, harder to update |

### 2.2 Template Completion Engine (Self-Contained)

```
Template Completion Flow
========================

1. TYPST SERVER (agentpdf-server)
   └─ Generates base PDF from template + user inputs
   └─ Returns PDF ready for field placement

2. TEMPLATE COMPLETION ENGINE (in agentpdf-web)
   └─ Self-contained, NOT a redirect to pdfjoin
   └─ LIMITED field types (prevents contract drafting):

   ALLOWED FIELDS:
   ┌────────────────────────────────────────────────┐
   │ 📝 Text Field    - Fill in blanks (names, $)  │
   │ ✍️  Signature     - Mark where to sign         │
   │ 🔤 Initials      - Contract revision marks    │
   │ ☑️  Checkbox      - Yes/No selections          │
   │ 📅 Date Field    - Auto-format date entry     │
   └────────────────────────────────────────────────┘

   ALLOWED OPERATIONS:
   ┌────────────────────────────────────────────────┐
   │ ✂️  Split Pages   - Extract specific pages     │
   │ 📎 Merge PDFs    - Combine with another doc   │
   │ 🔡 Font Controls - Size, type, bold/italic    │
   │ 👁️  Visual Preview - See document as you work │
   └────────────────────────────────────────────────┘

   EXPLICITLY NOT ALLOWED (prevents UPL):
   ┌────────────────────────────────────────────────┐
   │ ❌ Free-form text editing on existing content │
   │ ❌ Whiteout/blackout tools (could hide terms) │
   │ ❌ Text replacement (could alter clauses)     │
   │ ❌ Adding paragraphs/sections                 │
   └────────────────────────────────────────────────┘

3. SIGNATURE COLLECTION (in agentpdf-web)
   └─ Signature placement and collection
   └─ Same interface, no redirect
```

**Key Insight**: This is a "form filler" not a "document editor". Users can complete pre-generated templates but cannot draft their own contract language.

### 2.3 Size Benchmarking Results

**Benchmark Date**: December 30, 2025

| Component | Raw Size | Gzipped | Notes |
|-----------|----------|---------|-------|
| pdfjoin-wasm | 759 KB | 260 KB | PDF editing - acceptable |
| typst-engine | **N/A** | **N/A** | **Cannot compile to WASM** |

**typst-engine WASM Status**: BLOCKED
- Dependency `mio` does not support `wasm32-unknown-unknown` target
- 49 compilation errors related to network/IO primitives
- Server-side rendering is **required** - no client-side option

**Decision**: Server-side Typst rendering confirmed as only viable approach.

```
Architecture (Confirmed)
========================

1. agentpdf-server (Rust/Axum)
   └─ Runs typst-engine natively
   └─ Renders PDF from template + inputs
   └─ ~100ms render time

2. agentpdf-web (React + pdfjoin-wasm)
   └─ 260KB gzipped WASM bundle (acceptable)
   └─ Embedded PDF editing
   └─ Embedded signature collection
```

---

## Part III: Implementation Progress

### Overall Status

| Phase | Name | Status | Progress |
|-------|------|--------|----------|
| 0 | Infrastructure | ✅ COMPLETE | 7/7 tasks |
| 1.0 | Florida Core Documents | ✅ COMPLETE | 24 templates |
| 1.1 | Bill of Sale | ✅ COMPLETE | 5/5 types + Ch. 319 compliance |
| 2 | Compliance Validation | ✅ COMPLETE | 4/4 tasks (incl. Ch. 319) |
| 3 | Template Completion Engine | ✅ COMPLETE | 5/5 features |
| 4 | Browser Test Coverage | 🚧 IN PROGRESS | 4 regression tests |

### Phase 0: Infrastructure (7/7 Complete)

| Task | Status | Notes |
|------|--------|-------|
| Create agentpdf-server with Axum | DONE | `apps/agentpdf-server/` |
| Define API endpoints | DONE | `/api/render`, `/api/templates`, `/api/compliance`, `/api/document-types` |
| Add comprehensive test suite | DONE | **40 tests** (proptest + HTTP + regression) |
| Configure rate limiting | DONE | tower-governor, per-IP |
| Integrate compliance-engine | DONE | Florida-focused, all document types |
| Expand DocumentType enum | DONE | **26 document types** across 5 categories |
| Feature flags for FL-only mode | DONE | `florida-only` / `all-states` features |

### Phase 1.0: Florida Core Documents ✅ TEMPLATES COMPLETE

| Category | Documents | Status | Statute |
|----------|-----------|--------|---------|
| Lease | Agreement, Termination, Eviction | ✅ COMPLETE | Ch. 83 |
| Purchase | Contract, Contingencies, Escalation | ✅ COMPLETE | Ch. 689, 718, 720 |
| Listing | Exclusive | ✅ COMPLETE | Ch. 475 |
| Contractor | Invoice, NOC, Lien notices | ✅ COMPLETE | Ch. 713 |

**All Templates** (24 total, registered in typst-engine):
- `florida_lease.typ` - Residential lease agreement
- `florida_lease_termination.typ` - 7/15/30-day notices ✅ NEW
- `florida_eviction_notice.typ` - 3-day, 7-day, 15-day notices ✅ NEW
- `florida_purchase_contract.typ` - Standard purchase
- `florida_purchase_as_is.typ` - As-is purchase
- `florida_inspection_contingency.typ` - Inspection addendum ✅ NEW
- `florida_financing_contingency.typ` - Financing addendum ✅ NEW
- `florida_listing_agreement.typ` - Exclusive right to sell
- `florida_escalation_addendum.typ` - Escalation clause
- `florida_flood_disclosure.typ` - Flood zone disclosure
- `florida_notice_of_commencement.typ` - Ch. 713.13 ✅ NEW
- `florida_notice_to_owner.typ` - Ch. 713.06 ✅ NEW
- `florida_claim_of_lien.typ` - Ch. 713.08 ✅ NEW
- `florida_release_of_lien.typ` - Ch. 713.20/21 ✅ NEW
- `florida_contractor_invoice.typ` - Standard/progress billing ✅ NEW

### Phase 1.1: Bill of Sale ✅ COMPLETE

| Type | Statute/Form | Status |
|------|--------------|--------|
| Car | Ch. 319, HSMV 82050 | ✅ `florida_bill_of_sale_car.typ` |
| Boat | Ch. 328, HSMV 87002 | ✅ `florida_bill_of_sale_boat.typ` |
| Trailer | Ch. 319/320 | ✅ `florida_bill_of_sale_trailer.typ` |
| JetSki | Ch. 328 | ✅ `florida_bill_of_sale_jetski.typ` |
| MobileHome | Ch. 319/723 | ✅ `florida_bill_of_sale_mobile_home.typ` |

---

## Part IV: Scrivener Standard (Legal Compliance)

### 4.1 The Doctrine

agentPDF is a **scrivener** (intelligent typewriter), not a legal advisor.

| Allowed | Not Allowed |
|---------|-------------|
| "Do you want to include X?" | "We recommend X" |
| "X is defined as..." | "Based on your situation, you should..." |
| "Properties built before 1978 require lead paint disclosure" | "You need lead paint disclosure" |
| Present options with definitions | Apply law to user's specific facts |

### 4.2 UI Pattern

```
Form Wizard Pattern (Scrivener-Compliant)
=========================================

GOOD:
┌─────────────────────────────────────────────┐
│ Inspection Contingency                       │
│                                              │
│ [ ] Include inspection contingency           │
│                                              │
│ ℹ️ An inspection contingency allows the      │
│   buyer to terminate if significant defects  │
│   are found during the inspection period.    │
│                                              │
│ If included, inspection period: [15] days    │
└─────────────────────────────────────────────┘

BAD:
┌─────────────────────────────────────────────┐
│ We recommend including an inspection         │
│ contingency to protect yourself.             │  ← UPL VIOLATION
└─────────────────────────────────────────────┘
```

### 4.3 Required Disclaimers

Every generated document must include:

```
DISCLAIMER: This document was prepared using agentPDF.org, a document
preparation service. No attorney-client relationship is created. This
is not legal advice. For complex matters, consult a Florida attorney.
```

---

## Part V: Compliance Validation Engine

### 5.1 Florida-Only Focus

The compliance engine validates uploaded documents against Florida law.

**Current Coverage** (florida.rs + florida_realestate.rs + florida_contractor.rs):
- Ch. 83 Part II - Residential leases
- Ch. 689 - Conveyances
- Ch. 718 - Condominiums
- Ch. 720 - HOAs
- Ch. 475 - Real estate brokers
- **Ch. 713 - Construction liens** (NEW - 9 document types)
  - § 713.13 - Notice of Commencement
  - § 713.06 - Notice to Owner
  - § 713.04 - Lien Rights Disclosure
  - § 713.08 - Claim of Lien
  - § 713.21 - Release of Lien
  - § 713.22 - Contest of Lien
  - § 713.31 - Fraudulent Liens
- 42 U.S.C. 4852d - Lead paint (federal)
- F.S. 404.056 - Radon disclosure

**New Coverage** (Phase 1.1 - COMPLETE):
- **Ch. 319 - Motor vehicle titles** (`florida_billofsale.rs`)
  - § 319.22 - Title transfer requirements (VIN, seller/buyer info, signatures)
  - § 319.23 - Odometer disclosure requirements
  - § 319.261 - Mobile home title requirements
  - § 327.02 - Vessel (boat/jet ski) title requirements
- Bill of Sale type detection (Car, Boat, Trailer, JetSki, MobileHome)
- 10 unit tests for bill of sale compliance

### 5.2 Validation API

**GET /api/document-types** - List all supported document types

```json
{
  "success": true,
  "categories": [
    {"category": "Lease", "chapter": "Chapter 83", "types": [...]},
    {"category": "Real Estate Purchase", "chapter": "Chapter 475, 689", "types": [...]},
    {"category": "Listing", "chapter": "Chapter 475", "types": [...]},
    {"category": "Contractor", "chapter": "Chapter 713", "types": [...]}
  ],
  "total_types": 20
}
```

**POST /api/compliance** - Check document compliance

```json
Request:
{
  "text": "...",           // Extracted document text
  "document_type": "lease" | "purchase" | "listing" | "notice_of_commencement" | "claim_of_lien" | ...,
  "year_built": 1970,      // Optional: for lead paint
  "state": "FL",           // Always FL for MVP
  "zip_code": "33101"      // Optional: for local ordinances
}

Response:
{
  "success": true,
  "compliant": false,
  "violations": [
    {
      "statute": "F.S. § 713.13",
      "message": "Notice of Commencement may be incomplete...",
      "severity": "Warning",
      "page": 1,
      "text_snippet": "..."
    }
  ],
  "violation_count": 1
}
```

**Supported Document Types (20 total)**:
- Lease: `lease`, `lease_termination`, `eviction`
- Purchase: `purchase`, `purchase_as_is`, `inspection_contingency`, `financing_contingency`, `escalation`, `appraisal_contingency`
- Listing: `listing`
- Contractor: `notice_of_commencement`, `notice_to_owner`, `claim_of_lien`, `release_of_lien`, `dispute_lien`, `fraudulent_lien`, `contractor_invoice`, `cost_of_materials`, `final_payment_affidavit`

### 5.3 Test Cases Needed

| Document Type | Test Case | Expected |
|---------------|-----------|----------|
| Lease | Missing radon disclosure | Violation |
| Lease | Security deposit > 2 months | Warning |
| Lease | Pre-1978 without lead paint | Violation |
| Purchase | Missing property tax disclosure | Violation |
| Purchase | Condo without SIRS/Milestone | Violation |
| Listing | No expiration date | Violation |
| Contractor | NOC missing property description | Violation |
| BillOfSale | Car without VIN | Violation |

---

## Part VI: Template Completion Engine (Self-Contained)

### 6.1 Architecture

The Template Completion Engine is **self-contained** within agentpdf-web. It reuses
pdfjoin-core Rust crates but has its own TypeScript UI with intentionally limited features.

```
agentpdf-web/
├── src/ts/
│   ├── template-editor.ts    # LIMITED field placement (NEW)
│   ├── pdf-bridge.ts         # PDF.js wrapper (copied from pdfjoin)
│   ├── coord-utils.ts        # Coordinate conversion (copied)
│   └── types/                # Type definitions (copied)
├── wasm/                     # Uses pdfjoin-core for PDF ops
└── www/
    └── index.html            # Unified interface
```

### 6.2 Allowed Field Types

```typescript
// template-editor.ts - ONLY these field types are allowed

enum FieldType {
  TextField = 'text',       // Fill in names, dates, amounts
  Signature = 'signature',  // Mark signature locations
  Initials = 'initials',    // Contract revision acknowledgment
  Checkbox = 'checkbox',    // Yes/No selections
  DateField = 'date',       // Auto-formatted date entry
}

// EXPLICITLY NOT IMPLEMENTED (prevents contract drafting):
// - WhiteoutTool (could hide contract terms)
// - BlackoutTool (could hide contract terms)
// - TextReplaceTool (could alter clauses)
// - HighlightTool (not needed for form completion)
// - UnderlineTool (not needed for form completion)
```

### 6.3 Page Operations

```typescript
// Split/merge reuse pdfjoin-core directly
import { PdfJoinSession, SessionMode } from 'pdfjoin-wasm';

// Split pages
const split = new PdfJoinSession(SessionMode.Split);
split.addDocument("contract.pdf", bytes);
split.setPageSelection("1-3, 5");
const extracted = split.execute();

// Merge documents
const merge = new PdfJoinSession(SessionMode.Merge);
merge.addDocument("addendum.pdf", addendumBytes);
merge.addDocument("contract.pdf", contractBytes);
const combined = merge.execute();
```

### 6.4 User Flow

```
User Flow (Self-Contained)
==========================

1. [Form Wizard] User selects template, fills required fields
         ↓
2. [Server] Typst generates base PDF from template
         ↓
3. [Template Completion Engine] User places fields:
   ├─ 📝 Text fields for names, dates, amounts
   ├─ ✍️  Signature placeholders
   ├─ 🔤 Initials for revisions
   ├─ ☑️  Checkboxes for options
   └─ 📅 Date fields
         ↓
4. [Page Operations] Optional split/merge
         ↓
5. [Signature Collection] Collect actual signatures
         ↓
6. [Download] Final PDF

All steps in same interface. NO external redirects.
```

### 6.5 Files Copied from pdfjoin-web

**Minimal set** (only what's needed, ~500 lines):

| File | Lines | Purpose |
|------|-------|---------|
| `pdf-bridge.ts` | 178 | PDF.js wrapper |
| `pdf-loader.ts` | 66 | Lazy load PDF.js |
| `coord-utils.ts` | 164 | Coordinate conversion |
| `types/pdf-types.ts` | 156 | Type definitions |

**NOT copied** (intentionally excluded):
- `edit.ts` lines for whiteout/blackout
- `edit.ts` lines for text replacement
- `edit.ts` lines for highlight/underline

---

## Part VII: Archived Code (Non-Florida States)

### 7.1 Archived for Future Use

The following state modules exist but are **not active in MVP**:

```
crates/compliance-engine/src/states/
├── florida.rs           # ACTIVE
├── florida_realestate.rs # ACTIVE
├── arizona.rs           # ARCHIVED
├── california.rs        # ARCHIVED
├── georgia.rs           # ARCHIVED
├── illinois.rs          # ARCHIVED
├── massachusetts.rs     # ARCHIVED
├── michigan.rs          # ARCHIVED
├── new_jersey.rs        # ARCHIVED
├── new_york.rs          # ARCHIVED
├── north_carolina.rs    # ARCHIVED
├── ohio.rs              # ARCHIVED
├── pennsylvania.rs      # ARCHIVED
├── tennessee.rs         # ARCHIVED
├── texas.rs             # ARCHIVED
├── virginia.rs          # ARCHIVED
└── washington.rs        # ARCHIVED

crates/typst-engine/templates/
├── florida_*.typ        # ACTIVE
├── texas_lease.typ      # ARCHIVED
├── invoice.typ          # KEEP (generic)
└── letter.typ           # KEEP (generic)
```

### 7.2 Feature Flag Strategy

```rust
// compliance-engine/src/lib.rs

#[cfg(feature = "florida-only")]
pub fn supported_states() -> Vec<State> {
    vec![State::FL]
}

#[cfg(not(feature = "florida-only"))]
pub fn supported_states() -> Vec<State> {
    vec![State::FL, State::TX, State::CA, /* ... */]
}
```

Default: `florida-only` feature enabled for MVP.

---

## Part VIII: Implementation Roadmap

### Phase 0: Infrastructure (COMPLETE)

- [x] agentpdf-server with Axum
- [x] API endpoints (render, templates, compliance, document-types)
- [x] 40+ tests passing (proptest + HTTP + regression)
- [x] Rate limiting (tower-governor)
- [x] Florida compliance integration
- [x] DocumentType enum with 26 types across 5 categories
- [x] Feature flags (`florida-only` / `all-states`)
- [x] WASM bindings for all document types
- [ ] Authentication middleware (future)
- [ ] CI/CD deployment (future)

### Phase 1.0: Florida Core Documents

**Milestone 1.0.1 - Lease Documents**
- [ ] Review/update florida_lease.typ for Ch. 83 compliance
- [ ] Create termination notice templates (7/15/30-day)
- [ ] Create eviction notice template
- [ ] Add compliance tests for each

**Milestone 1.0.2 - Purchase Documents**
- [ ] Review florida_purchase_contract.typ
- [ ] Review florida_purchase_as_is.typ
- [ ] Create inspection contingency addendum
- [ ] Create financing contingency addendum
- [ ] Review florida_escalation_addendum.typ

**Milestone 1.0.3 - Listing Documents**
- [ ] Review florida_listing_agreement.typ for Ch. 475
- [ ] Ensure NAR settlement compliance (Aug 2024)

**Milestone 1.0.4 - Contractor Documents** ✅ COMPLIANCE COMPLETE

- [x] Ch. 713 compliance module (`florida_contractor.rs`)
  - [x] Notice of Commencement (§ 713.13)
  - [x] Notice to Owner (§ 713.06)
  - [x] Claim of Lien (§ 713.08)
  - [x] Release of Lien (§ 713.21)
  - [x] Dispute of Lien (§ 713.22)
  - [x] Fraudulent Lien Report (§ 713.31)
  - [x] Final Payment Affidavit (§ 713.06)
  - [x] Contractor Invoice validation
  - [x] Cost of Materials Bill validation
- [ ] Create Typst templates for contractor documents

### Phase 1.1: Bill of Sale

- [ ] Car bill of sale (HSMV 82050 format)
- [ ] Boat bill of sale (HSMV 87002 format)
- [ ] Trailer bill of sale
- [ ] JetSki/PWC bill of sale
- [ ] Mobile home bill of sale (Ch. 319)

### Phase 2: Compliance Validation ✅ COMPLETE

- [x] Add Ch. 713 contractor lien rules (`florida_contractor.rs` - 21 tests)
- [x] Unified DocumentType enum (26 types)
- [x] Unified compliance API (`check_document_compliance`)
- [x] WASM bindings for all document types
- [x] Server API `/api/document-types` endpoint
- [x] Add Ch. 319 motor vehicle rules (`florida_billofsale.rs` - 10 tests)

### Phase 3: Template Completion Engine ✅ COMPLETE

**Milestone 3.1 - Core Infrastructure**
- [x] TypeScript build setup with esbuild (`apps/agentpdf-web/`)
- [x] Template editor implementation (`src/ts/template-editor.ts`)
- [x] PDF coordinate transformation utilities
- [x] WASM bindings for field flattening (`field_export.rs`)

**Milestone 3.2 - Field Types**
- [x] Text field placement with font controls (font family, size, bold, italic, color)
- [x] Signature field placement with canvas capture
- [x] Initials field placement
- [x] Checkbox field placement (toggle yes/no)
- [x] Date field placement (auto-format)

**Milestone 3.3 - Page Operations**
- [x] Split pages modal with page range input
- [x] Merge PDFs modal with file selection
- [x] Visual page picker UI
- [x] Integration with pdfjoin-core via WASM

**Milestone 3.4 - Signature Collection**
- [x] Signature capture canvas modal
- [x] Clear/save signature functionality
- [x] Signature embedding into PDF via AddImage operation

**Milestone 3.5 - Field Flattening (Export)**
- [x] `export_pdf_with_fields` WASM function
- [x] `validate_fields_for_export` WASM function
- [x] AddImage operation in pdfjoin-core for signatures
- [x] PNG/JPEG image embedding support

### Phase 4: Browser Test Coverage 🚧 IN PROGRESS

**Regression Tests** (chromiumoxide, parallel execution):
- [x] Field type buttons exist test
- [x] Font controls panel test
- [x] Page operations buttons test
- [x] Signature capture modal test
- [ ] E2E: Template generation → field placement → download
- [ ] E2E: Text field: place, type, resize, font change
- [ ] E2E: Signature field: place, capture, embed
- [ ] E2E: Checkbox field: place, toggle, export
- [ ] E2E: Page split: select pages, extract, verify
- [ ] E2E: Page merge: add documents, combine

**Property Tests** (`proptest`):
- [x] Page range parsing (27 tests in `page_ranges.rs`)
- [x] Field dimension validation (WCAG compliance)
- [x] Font size validation

**Smart Pre-commit Hook** ✅ COMPLETE:
- [x] Detect which app was edited (agentpdf-web, pdfjoin-web, docsign-web)
- [x] Only run browser tests for edited app
- [x] Uses cargo-nextest for parallel test execution

---

## Part IX: Success Metrics

### Florida MVP Launch Criteria

| Metric | Target |
|--------|--------|
| All Phase 1.0 documents complete | 100% |
| Compliance test coverage | >90% |
| WASM size decision made | Done |
| Integration (pdfjoin/docsign) working | Done |
| User testing with FL landlords | 10+ users |
| Zero critical compliance bugs | 0 |

### Post-MVP Expansion Order

1. Florida perfected (MVP)
2. Texas (similar landlord-friendly laws)
3. Georgia (Southeast expansion)
4. California (high volume, complex)
5. New York (highest complexity)

---

## References

### Florida Statutes

- [Chapter 83 - Landlord and Tenant](https://www.flsenate.gov/Laws/Statutes/2025/Chapter83)
- [Chapter 475 - Real Estate Brokers](https://www.flsenate.gov/Laws/Statutes/2025/Chapter475)
- [Chapter 689 - Conveyances](https://www.flsenate.gov/Laws/Statutes/2025/Chapter689)
- [Chapter 713 - Construction Liens](https://www.flsenate.gov/Laws/Statutes/2025/Chapter713)
- [Chapter 319 - Motor Vehicle Titles](https://www.flsenate.gov/Laws/Statutes/2025/Chapter319)
- [Chapter 718 - Condominiums](https://www.flsenate.gov/Laws/Statutes/2025/Chapter718)
- [Chapter 720 - HOAs](https://www.flsenate.gov/Laws/Statutes/2025/Chapter720)

### HSMV Forms

- [HSMV 82050 - Certificate of Title](https://www.flhsmv.gov/pdf/forms/82050.pdf)
- [HSMV 87002 - Vessel Registration](https://www.flhsmv.gov/pdf/forms/87002.pdf)

### Related Plan Documents

- `FL_LEASE.md` - Detailed Florida lease law analysis
- `FL_PURCHASE.md` - Purchase contract architecture
- `FL_LIST.md` - Listing agreement compliance
