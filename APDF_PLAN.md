# agentPDF Strategic Plan

> Comprehensive Architecture and Roadmap for Florida Real Estate Contract Generation Platform

**Document Version**: 1.1
**Last Updated**: December 30, 2025
**Status**: Active Development

---

## Implementation Progress

### Overall Status

| Phase | Name | Status | Progress |
|-------|------|--------|----------|
| 0 | Infrastructure | COMPLETE | 5/7 tasks |
| 1 | Slim Frontend | NOT STARTED | 0/4 tasks |
| 2 | Residential Lease MVP | NOT STARTED | 0/5 tasks |
| 3 | Desktop App (Tauri) | NOT STARTED | 0/5 tasks |
| 4 | Purchase Contracts | NOT STARTED | 0/4 tasks |
| 5 | Listing Agreements | NOT STARTED | 0/4 tasks |

### Phase 0: Infrastructure - COMPLETE

| Task | Status | Notes |
|------|--------|-------|
| Create agentpdf-server with Axum | DONE | `apps/agentpdf-server/` |
| Define API endpoints for typst-engine | DONE | `/api/render`, `/api/templates`, `/api/compliance` |
| Add property tests for rendering API | DONE | 15 tests passing (proptest + regression) |
| Configure rate limiting with tower-governor | DONE | Configurable per-IP limiting |
| Integrate compliance-engine | DONE | 16-state support, locality detection |
| Add authentication middleware | PENDING | JWT/API key auth planned |
| CI/CD for server deployment | PENDING | |

### Server Implementation Summary

The `agentpdf-server` is now operational with:

```
apps/agentpdf-server/
├── Cargo.toml          # Dependencies including tower-governor, axum
├── src/
│   ├── main.rs         # CLI + server bootstrap (configurable port, rate limit, timeout)
│   ├── api.rs          # REST handlers (/health, /templates, /render, /compliance)
│   ├── error.rs        # Error handling with proper HTTP status codes
│   └── tests.rs        # 15 property tests + regression tests
```

**API Endpoints:**
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check with version info |
| `/api/templates` | GET | List all 10 available templates |
| `/api/render` | POST | Render template to PDF/SVG/PNG |
| `/api/compliance` | POST | Check document compliance (16 states) |

**CLI Options:**
```bash
cargo run -p agentpdf-server -- \
  --port 3000 \
  --host 0.0.0.0 \
  --rate-limit 10 \
  --timeout-ms 10000 \
  --verbose
```

**Test Coverage (15 tests):**
- Property tests for template validation (4 tests)
- Property tests for compliance engine states (3 tests)
- Regression tests for Florida lease/purchase rendering (2 tests)
- Regression tests for locality detection - Chicago RLTO, NYC (4 tests)
- Edge case tests for empty/long text (2 tests)

---

## Executive Summary

agentPDF.org is a specialized platform for generating legally compliant Florida real estate contracts. This document outlines the strategic architecture, implementation roadmap, and technical decisions for building a production-ready system by Q1 2026.

### Core Philosophy: The Scrivener Standard

agentPDF operates under the **Scrivener Doctrine** - functioning as an intelligent typewriter, not a legal counselor. This approach:

- **Presents options** for user selection without recommending specific choices
- **Derives contracts from statute** (Florida Statutes, case law) rather than copying bar association templates
- **Validates bar association documents privately** to ensure no critical provisions are missed
- **Rephrases and relocates** any similar concepts to avoid IP infringement

> **Critical Distinction**: Advisory logic ("Based on your assets, we recommend...") = UPL violation. Scrivener logic ("Do you want to include X? (Definition: X is...)") = Permitted.

### MVP Priority Order

| Priority | Use Case | Target Date | Key Statute |
|----------|----------|-------------|-------------|
| **P0** | Florida Residential Leases | Q1 2026 | Chapter 83 Part II |
| **P1** | Purchase Contracts (As-Is) | Q2 2026 | Chapter 689, 718, 720 |
| **P2** | Listing Agreements | Q2 2026 | Chapter 475 |

---

## Part I: Architecture Decision - The Hybrid Microservices Model

### 1.1 The Problem with Local-First WASM

The current agentpdf-web bundles the full typst-engine into WASM, resulting in:

- **75MB WASM bundle** (8.3MB brotli-compressed)
- **~650ms render time** per template
- **Browser constraints** on memory and CPU
- **No caching** between sessions

While local-first is ideal for offline scenarios, contract generation is fundamentally an **online workflow** where users:
1. Fill out forms (needs connectivity for autocomplete, validation)
2. Generate PDFs (can be server-side)
3. Download for editing (final step before signing)

### 1.2 The Proposed Architecture: Slim Frontend + Proxy Server

```
Architecture Overview
=====================

+------------------+      HTTPS       +----------------------+
|   agentPDF.org   |  <----------->   |   Proxy Server       |
|   (Slim Frontend)|                  |   (typst-engine)     |
+------------------+                  +----------------------+
        |                                     |
        | Final Edit                          | Render
        v                                     v
+------------------+                  +----------------------+
|   pdfjoin-web    |                  |   Typst Templates    |
|   Edit Component |                  |   + FL Compliance    |
+------------------+                  +----------------------+
```

### 1.3 Component Responsibilities

#### Slim Frontend (agentPDF.org)
- **Form wizard** for contract configuration
- **State management** for multi-step workflows
- **Preview iframe** for rendered PDFs
- **Download handler** with optional pdfjoin-web edit step
- **Bundle size target**: <500KB

#### Proxy Server (typst-engine)
- **Typst rendering** via native Rust (not WASM)
- **Template caching** and hot-reload
- **Compliance validation** before render
- **Rate limiting** and authentication
- **Horizontal scaling** for high volume

#### pdfjoin-web Integration
- **Final step only** - after template generation
- **Edit capabilities**: TextBox, Whiteout, Checkbox, Highlight
- **Flatten on export** - burns edits into PDF content stream
- **Optional** - users can download without editing

### 1.4 Benefits of This Architecture

| Aspect | WASM-Only | Hybrid Model |
|--------|-----------|--------------|
| Bundle Size | 75MB | <500KB frontend |
| Render Time | 650ms | ~100ms (server) |
| Memory | Browser-limited | Server-scaled |
| Caching | None | Multi-layer |
| Offline | Full | Download-only |
| Cost | $0/document | ~$0.001/document |

The trade-off is acceptable because contract generation is not an offline-critical workflow.

---

## Part II: Tauri 2.0 Desktop Application

### 2.1 Why Tauri for agentPDF Desktop

[Tauri 2.0](https://v2.tauri.app/) (released October 2024) offers:

- **95% smaller binaries** than Electron (~3MB vs 100MB+)
- **30-40MB idle memory** vs Electron's 150MB+
- **Unified desktop + mobile** from single codebase
- **Rust security** with system WebView integration
- **Native system access** via permission-gated APIs

### 2.2 Desktop-Specific Advantages

```
Tauri Desktop Benefits
======================

+------------------------------------------+
|  agentPDF Desktop (Tauri 2.0)            |
+------------------------------------------+
|  - Local template caching                |
|  - Offline form completion               |
|  - Native file system integration        |
|  - System keychain for credentials       |
|  - Auto-update via Tauri updater         |
|  - macOS notarization / Windows signing  |
+------------------------------------------+
        |
        | Shares backend with web
        v
+------------------------------------------+
|  Proxy Server (same as web)              |
|  - PDF generation                        |
|  - Compliance checking                   |
|  - Template updates                      |
+------------------------------------------+
```

### 2.3 Tauri Architecture Pattern

Following [Tauri 2.0 best practices](https://v2.tauri.app/concept/architecture/):

```rust
// Tauri command example for agentPDF
#[tauri::command]
async fn generate_contract(
    config: ContractConfig,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    // Option 1: Call remote proxy server
    let pdf = state.client
        .post(&state.server_url)
        .json(&config)
        .send()
        .await?
        .bytes()
        .await?;

    // Option 2: Local typst-engine (for offline)
    // let pdf = typst_engine::render(&config)?;

    Ok(pdf.to_vec())
}
```

### 2.4 Desktop Roadmap

| Phase | Feature | Implementation |
|-------|---------|----------------|
| D1 | Basic shell | Tauri + same React frontend |
| D2 | Offline forms | Local state persistence |
| D3 | Local rendering | Embed typst-engine sidecar |
| D4 | Auto-update | Tauri updater + code signing |

---

## Part III: MVP - Florida Residential Leases

### 3.1 Statutory Foundation

Florida residential leases are governed by **Chapter 83, Part II** of the Florida Statutes. The template must enforce:

| Requirement | Statute | Implementation |
|-------------|---------|----------------|
| Radon Disclosure | 404.056(5) | Exact statutory text, non-editable |
| Lead Paint | 42 U.S.C. 4852d | Conditional on Year Built < 1978 |
| Flood Disclosure | 83.512 / SB 948 | Wizard-driven, Oct 2025 expanded |
| Security Deposit | 83.49 | Bank details + 30-day notice language |
| Electronic Notice | HB 615 | Consent checkbox with statutory cite |
| 30-Day Termination | HB 1417 | Month-to-month notice period |
| Service Member | 83.682 | 35-mile military base termination |
| Squatter Removal | HB 621 / SB 1084 | Unauthorized occupant clause |

### 3.2 Template Modules (Scrivener Approach)

Following the scrivener standard, the UI presents options without recommendations:

```
Form Wizard Structure (Residential Lease)
=========================================

Step 1: Property Information
----------------------------
- Property Type: [ ] Single Family [ ] Condo/Townhome [ ] Apartment
- Year Built: [____]
  └─ System note: "Properties built before 1978 require lead paint disclosure"
- In HOA/Condo Association: [ ] Yes [ ] No
  └─ If Yes: Attach Association Addendum (720.401)

Step 2: Flood History (SB 948 Compliant)
----------------------------------------
"Has this property experienced flooding during your ownership?"
  [ ] Yes, the property has flooded
  [ ] No known flooding events
  [ ] I don't know / Property recently acquired

"Have you filed any flood-related insurance claims?"
  [ ] Yes, claims have been filed
  [ ] No claims filed

"Has this property received FEMA or federal flood assistance?"
  [ ] Yes, federal assistance received
  [ ] No federal assistance

→ Generates: Section 83.512 Compliant Flood Disclosure Addendum

Step 3: Notice Preferences (HB 615)
-----------------------------------
"Does tenant consent to receive legal notices via email?"
  [ ] Yes - Email: [________________]
      └─ System inserts: "pursuant to Florida Statute 83.56 as amended"
  [ ] No - Tenant requires postal mail only

Step 4: Security Deposit
------------------------
"How will security be handled?"
  [ ] Traditional security deposit
      └─ Bank Name: [________________]
      └─ Account Type: [ ] Interest-bearing [ ] Non-interest
  [ ] Monthly fee in lieu of deposit (83.491)
      └─ System inserts: Non-refundable fee language

Step 5: Optional Addenda
------------------------
[ ] Liquidated Damages (83.595) - Max 2 months rent
[ ] Mold Prevention Addendum
[ ] Pet/ESA Addendum (citing SB 1084)
[ ] Jury Trial Waiver
```

### 3.3 Typst Template Structure

```typst
// florida_lease.typ - Module structure

#let florida_lease(
  // Property
  property_address,
  property_type,
  year_built,
  in_association: false,

  // Parties
  landlord_name,
  tenant_name,

  // Terms
  rent_amount,
  lease_start,
  lease_end,
  security_deposit,

  // Compliance flags
  flood_history: "unknown",
  flood_claims: false,
  fema_assistance: false,
  email_notice_consent: false,
  tenant_email: none,

  // Optional addenda
  include_liquidated_damages: false,
  include_mold_addendum: false,
  include_pet_addendum: false,
  include_jury_waiver: false,
) = {
  // Document generation with conditional sections
}
```

### 3.4 Compliance Engine Integration

The existing `compliance-engine` crate (268 tests) validates contracts:

```rust
// Pre-render validation
pub fn validate_lease_config(config: &LeaseConfig) -> Vec<Violation> {
    let mut violations = Vec::new();

    // § 83.49 - Security deposit bank required
    if config.security_deposit.is_some()
       && config.bank_name.is_none() {
        violations.push(Violation {
            rule: "fl-83.49-bank",
            severity: Critical,
            message: "Security deposit requires bank disclosure",
        });
    }

    // § 83.512 - Flood disclosure required
    if config.flood_history == FloodHistory::Unknown
       && config.effective_date >= date!(2025-10-01) {
        violations.push(Violation {
            rule: "fl-83.512-flood",
            severity: Critical,
            message: "Flood disclosure required after Oct 1, 2025",
        });
    }

    violations
}
```

---

## Part IV: Phase 2 - Purchase Contracts (As-Is)

### 4.1 Strategic Approach

Purchase contracts are governed by multiple statutes and common law. The "As-Is" contract form provides buyers with **maximum discretion** (unilateral termination power) during inspection.

Key requirements:

| Requirement | Source | Implementation |
|-------------|--------|----------------|
| Property Tax Disclosure | 689.261 | Exact statutory warning |
| Flood Disclosure | 689.302 | Oct 2024+ expanded requirements |
| Foreign Ownership | SB 264 | Affidavit for countries of concern |
| SIRS/Milestone (Condo) | SB 4-D | 718.503 disclosure + 7-day review |
| Johnson v. Davis | Case Law | Material defect disclosure duty |
| Radon | 404.056 | Same as lease |
| Lead Paint | Federal | Same as lease |

### 4.2 As-Is vs Standard Contract Logic

```
Contract Selection (Scrivener Approach)
=======================================

"Which contract type do you want to use?"

[ ] As-Is Contract
    └─ "Buyer has unilateral right to terminate during inspection"
    └─ "Seller has no obligation to make repairs"
    └─ "15-day inspection period (negotiable)"

[ ] Standard Contract
    └─ "Seller obligated to repair defects up to repair limits"
    └─ "Buyer loses termination right if repairs within limits"
    └─ "Default repair limits: 1.5% of purchase price per category"

System does NOT recommend - presents factual differences only.
```

### 4.3 Addenda Modules

```
Purchase Contract Addenda
=========================

Mandatory (Auto-Attached):
- Radon Gas Disclosure
- Property Tax Disclosure
- Flood Disclosure (689.302)

Conditional:
- Lead Paint (if Year Built < 1978)
- HOA Disclosure (if in association)
- CDD Disclosure (if in CDD - 190.048)
- Foreign Interest Affidavit (SB 264)
- Condo Safety Rider (if condo - SIRS/Milestone)

Optional (User Selection):
- Escalation Clause Addendum
- Appraisal Gap Guarantee
- Kick-Out Clause (Rider X)
- Post-Occupancy Agreement
```

---

## Part V: Phase 3 - Listing Agreements

### 5.1 Chapter 475 Requirements

Listing agreements must satisfy the "Four Pillars" of validity:

| Pillar | Requirement | Enforcement |
|--------|-------------|-------------|
| 1 | Definite expiration date | Hard-coded date field, no auto-renewal |
| 2 | Legal description | Parcel ID + full legal from deed |
| 3 | Price and terms | Commission structure clearly stated |
| 4 | Fee structure | Listing fee separate from buyer concessions |

### 5.2 NAR Settlement Integration (Aug 2024)

Post-NAR settlement requirements:

- **Decoupled compensation** - No pre-determined buyer-broker splits
- **Negotiability disclosure** - "Fees are fully negotiable" in bold
- **Concession authorization** - Seller authorizes willingness to consider

```
Listing Agreement Structure (Post-NAR)
======================================

Section A: Listing Broker Compensation
--------------------------------------
"Seller agrees to pay Listing Broker: ___% or $___"

Section B: Buyer Concessions (Optional)
---------------------------------------
"Does Seller authorize communication of willingness to
consider buyer concessions?"
  [ ] Yes - Up to ___% or $___ toward buyer costs
  [ ] No - No concession communication authorized

Section C: Steering Defense Acknowledgment
------------------------------------------
"Seller acknowledges that failing to offer concessions
may limit the pool of potential buyers who cannot pay
for their own representation."
```

### 5.3 Brokerage Relationship Disclosure

Required under 475.278:

```
Brokerage Relationship Type
===========================

[ ] Single Agent
    └─ Fiduciary duties: Loyalty, Confidentiality, Obedience,
       Full Disclosure, Accounting, Skill/Care/Diligence
    └─ First sentence in UPPERCASE BOLD

[ ] Transaction Broker
    └─ Limited duties: Deal honestly, Account for funds,
       Use skill/care/diligence, Disclose material facts
```

---

## Part VI: pdfjoin-web Integration

### 6.1 Integration Point

pdfjoin-web is used **only as the final step** before download:

```
User Flow
=========

1. User completes form wizard
2. System generates PDF via proxy server
3. User previews PDF
4. [Optional] User clicks "Edit Before Download"
   └─ Opens pdfjoin-web Edit component
   └─ Available tools: TextBox, Whiteout, Checkbox, Highlight
5. User downloads PDF
   └─ If edited: Flattens edits into content stream
```

### 6.2 Technical Integration

```typescript
// agentPDF integration with pdfjoin-web

interface EditableDocument {
  pdfBase64: string;
  filename: string;
  metadata: ContractMetadata;
}

async function openForEditing(doc: EditableDocument) {
  // Load pdfjoin-web Edit component
  const editModule = await import('@pdfjoin/edit');

  // Initialize with generated PDF
  const session = editModule.createSession({
    pdf: doc.pdfBase64,
    tools: ['textbox', 'whiteout', 'checkbox', 'highlight'],
    onSave: async (editedPdf) => {
      // Flatten and download
      const flattened = await editModule.flatten(editedPdf);
      downloadPdf(flattened, doc.filename);
    }
  });
}
```

### 6.3 Bundle Strategy

pdfjoin-web edit components are **dynamically imported** only when needed:

```
Bundle Sizes
============

agentPDF core:     ~300KB (forms, wizard, preview)
pdfjoin-web edit:  ~327KB (loaded on demand)
                   -------
Total (if editing): ~627KB

Compare to current: 75MB WASM bundle
```

---

## Part VII: Implementation Roadmap

### Phase 0: Infrastructure (Week 1-2) - COMPLETE

- [x] Set up proxy server with Axum (`apps/agentpdf-server/`)
- [x] Deploy typst-engine as native Rust service (integrated via workspace)
- [x] Configure rate limiting (tower-governor, configurable per-IP)
- [x] Add property tests (15 tests: proptest + regression)
- [x] Add compliance API endpoint (`/api/compliance`)
- [ ] Add authentication middleware (JWT/API key - planned)
- [ ] Set up CI/CD for server deployment

**Server running with:**
```bash
cargo run -p agentpdf-server -- --port 3000 --rate-limit 10 --timeout-ms 10000
```

### Phase 1: Slim Frontend (Week 3-4)

- [ ] Extract form wizard from current agentpdf-web
- [ ] Remove WASM dependencies
- [ ] Implement API client for proxy server
- [ ] Build preview iframe with PDF.js

### Phase 2: Residential Lease MVP (Week 5-8)

- [ ] Implement all mandatory disclosures
- [ ] Build SB 948 Flood Disclosure wizard
- [ ] Add HB 615 Electronic Notice consent
- [ ] Integrate pdfjoin-web edit step
- [ ] Complete end-to-end testing

### Phase 3: Desktop App (Week 9-10)

- [ ] Scaffold Tauri 2.0 application
- [ ] Wrap existing React frontend
- [ ] Add native file save dialogs
- [ ] Configure auto-updater
- [ ] Code signing for macOS/Windows

### Phase 4: Purchase Contracts (Week 11-14)

- [ ] Implement As-Is contract template
- [ ] Add all conditional addenda
- [ ] Build SB 264 Foreign Interest affidavit
- [ ] Add condo SIRS/Milestone disclosure

### Phase 5: Listing Agreements (Week 15-16)

- [ ] Implement Chapter 475 compliant template
- [ ] Add NAR settlement compensation structure
- [ ] Build brokerage relationship disclosure
- [ ] Add protection period clause generator

---

## Part VIII: Legal Compliance Framework

### 8.1 Scrivener Standard Enforcement

Every UI element must pass the scrivener test:

| Pattern | Allowed? | Example |
|---------|----------|---------|
| Present options | YES | "Do you want X?" with tooltip definition |
| State facts | YES | "Properties over $184,500 typically require probate" |
| Recommend | NO | "We recommend X based on your situation" |
| Apply law to facts | NO | "Since you have $500K, you should use a trust" |

### 8.2 Disclaimer Requirements

Terms of Service must include:

```
REQUIRED DISCLAIMERS
====================

1. NO ATTORNEY-CLIENT RELATIONSHIP
   "agentPDF is a document preparation service. No attorney-client
   relationship is created by use of this service."

2. PRO SE REPRESENTATION
   "By using this service, you acknowledge that you are representing
   yourself and bear full responsibility for your legal decisions."

3. DATA ACCURACY
   "You are solely responsible for the accuracy of all information
   entered. agentPDF does not verify the truthfulness of your inputs."

4. NOT LEGAL ADVICE
   "This service does not provide legal advice. For complex situations,
   consult a licensed Florida attorney."
```

### 8.3 Template Sourcing Protocol

To avoid IP issues with bar association forms:

1. **Derive from statute** - Primary source is Florida Statutes
2. **Private validation** - Compare to FAR/BAR forms internally
3. **Rephrase provisions** - Use different language for similar concepts
4. **Relocate clauses** - Place sections in different order
5. **Document sourcing** - Maintain audit trail of statutory basis

---

## Part IX: Success Metrics

### 9.1 Tampa Demo Targets (Q1 2026)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Lease generation time | <2 seconds | Server logs |
| Compliance pass rate | 100% | Automated validation |
| User completion rate | >80% | Analytics |
| Edit step adoption | Track only | Analytics |

### 9.2 Market Entry Goals

| Milestone | Target Date | Goal |
|-----------|-------------|------|
| Beta launch | Jan 2026 | 100 landlords using system |
| REIA presentation | Feb 2026 | Tampa REIA demo |
| Paid launch | Mar 2026 | First paying customers |
| Purchase contracts | May 2026 | Full contract suite |

---

## References

### Statutory Sources
- [Florida Statutes Chapter 83](https://www.flsenate.gov/Laws/Statutes/2025/Chapter83)
- [Florida Statutes Chapter 475](https://www.flsenate.gov/Laws/Statutes/2025/Chapter475)
- [Florida Statutes Chapter 689](https://www.flsenate.gov/Laws/Statutes/2025/Chapter689)

### Technical Resources
- [Tauri 2.0 Architecture](https://v2.tauri.app/concept/architecture/)
- [Tauri Process Model](https://v2.tauri.app/concept/process-model/)
- [Typst Automated PDF Generation](https://typst.app/blog/2025/automated-generation/)

### Related Plan Documents
- `FL_LEASE.md` - Detailed Florida lease law analysis
- `FL_PURCHASE.md` - Purchase contract architecture
- `FL_LIST.md` - Listing agreement compliance
- `STRATEGY.md` - Market positioning and business model
- `RESEARCH.md` - Local-first architectural decisions

---

## Appendix A: Existing Template Status

| Template | File | Status | Compliance |
|----------|------|--------|------------|
| Florida Lease | `florida_lease.typ` | Ready | 31 FL rules |
| Florida Purchase | `florida_purchase_contract.typ` | Ready | Multi-statute |
| Florida As-Is | `florida_purchase_as_is.typ` | Ready | As-Is specific |
| Florida Listing | `florida_listing_agreement.typ` | Ready | Ch 475 + NAR |
| Escalation Addendum | `florida_escalation_addendum.typ` | Ready | Best practices |
| Flood Disclosure | `florida_flood_disclosure.typ` | Ready | SB 948 |
| Commercial Lease | `florida_commercial_lease.typ` | Ready | Part I |
| Texas Lease | `texas_lease.typ` | Ready | Ch 92 |

## Appendix B: Compliance Engine Coverage

The `compliance-engine` crate provides:

- **268 tests** across 16 states
- **31 Florida rules** with statutory citations
- **Property-based fuzz testing** for edge cases
- **Pattern matching** for prohibited provisions

Key Florida rules enforced:
- 83.47 - Prohibited lease provisions
- 83.48 - Attorney fee reciprocity
- 83.49 - Security deposit requirements
- 83.56 - Electronic notice consent
- 83.57 - Termination periods (30 days)
- 83.512 - Flood disclosure
- 83.682 - Service member rights
