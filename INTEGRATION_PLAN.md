# Integration Plan: agentPDF + Web + Corpus + DocSign

## Executive Summary

This document outlines the integration strategy for four interconnected projects in the BobaMatchSolutions PDF ecosystem, centered around the Florida residential lease document workflow:

1. **agentPDF** (this repo) - Typst MCP server with lease generation and verification
2. **web** (../web) - Local-first compliance checking website → **agentPDF.org**
3. **corpus** (../corpus) - Template storage and semantic search
4. **docsign** (../../docsign) - Standalone digital signature platform → **getsignatures.org**

**Goal**: Enable users to:
1. Select an empty Florida lease template from corpus
2. Populate fields with relevant information (names, dates, amounts)
3. Verify the populated document against Florida law
4. Download the PDF or dispatch for signatures via DocSign

---

## Deployment Strategy

### Domain Ownership

Both production domains have been purchased and are ready for deployment:

| Project | Domain | Status |
|---------|--------|--------|
| web (agentpdf.org frontend) | **agentPDF.org** | ✅ Purchased |
| docsign (signature platform) | **getsignatures.org** | ✅ Purchased |

### Service Independence

**Critical Architectural Principle**: DocSign (getsignatures.org) is a **standalone, independent service**. Users can:

1. **Use DocSign directly** at getsignatures.org without ever touching agentPDF.org
2. **Upload any PDF** for signing (not just Florida leases)
3. **Arrive via referral** from agentPDF.org with a pre-loaded document

DocSign is NOT embedded into agentPDF.org. The integration is a **handoff model**:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SERVICE INDEPENDENCE MODEL                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   agentPDF.org                              getsignatures.org               │
│   ┌─────────────────────┐                   ┌─────────────────────┐        │
│   │ • Template selection │                   │ • Upload any PDF    │        │
│   │ • Field population   │                   │ • Add recipients    │        │
│   │ • Compliance verify  │                   │ • Place sig fields  │        │
│   │ • Download PDF       │                   │ • Send for signing  │        │
│   │                      │    ───────────►   │ • Track status      │        │
│   │ [Send for Signatures]│    Redirect with  │ • Collect sigs      │        │
│   │                      │    PDF in URL     │                     │        │
│   └─────────────────────┘                   └─────────────────────┘        │
│                                                                             │
│   Users who start here                      Users can start here           │
│   can hand off to DocSign                   independently too!             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Why Separate Services?

1. **Different user personas**: Landlords creating leases vs. anyone needing signatures
2. **Independent value**: Each product useful on its own
3. **Simpler deployment**: Separate CI/CD, scaling, monitoring
4. **Cleaner architecture**: No tight coupling between codebases
5. **Marketing flexibility**: Can promote each service independently

---

## Project Overview

### Current State

| Project | Status | Primary Function |
|---------|--------|------------------|
| agentPDF | ✅ Complete | Typst rendering + 8-rule Florida verifier |
| web | 🟡 Partial | WASM compliance UI (4 rule modules, 116 tests) |
| corpus | ✅ Complete | 8 Florida templates in markdown |
| docsign | ✅ Complete | PAdES-B signing + email dispatch |

### Architecture Comparison

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CURRENT ARCHITECTURE                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  agentPDF (Rust)              web (Rust WASM)           corpus (Rust)       │
│  ┌────────────────┐           ┌────────────────┐        ┌──────────────┐   │
│  │ Typst Compiler │           │ PDF.js Viewer  │        │ LanceDB      │   │
│  │ MCP Server     │           │ Field Placer   │        │ BGE-M3       │   │
│  │ Lease Verifier │           │ Compliance     │        │ Templates    │   │
│  │ PDF Extraction │           │ IndexedDB      │        │ REST API     │   │
│  └────────────────┘           └────────────────┘        └──────────────┘   │
│         │                            │                         │           │
│         └────────────────────────────┴─────────────────────────┘           │
│                                                                             │
│  ════════════════════════════════════════════════════════════════════════  │
│                        SEPARATE STANDALONE SERVICE                          │
│  ════════════════════════════════════════════════════════════════════════  │
│                                                                             │
│                              ┌───────────────┐                              │
│                              │    docsign    │  ← getsignatures.org        │
│                              │ WASM Signing  │    (independent service)    │
│                              │ Email Relay   │                              │
│                              │ Own Frontend  │                              │
│                              └───────────────┘                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Part 1: Verifier Logic Analysis

### 1.1 Coverage Comparison

Both agentPDF and web implement Florida lease compliance checking. Here's the detailed comparison:

#### agentPDF Verifier (src/verifier/)

| Rule | Statute | Severity | Implementation |
|------|---------|----------|----------------|
| RadonDisclosureRule | F.S. § 404.056 | Critical | Keyword search: "radon", "radioactive", "gas" |
| SecurityDepositDisclosureRule | F.S. § 83.49 | Critical | Bank info, return timeline (15-60 days) |
| LeadPaintDisclosureRule | 24 CFR Part 35 | High | Pre-1978 check, EPA pamphlet |
| ProhibitedTermsRule | F.S. § 83.47 | Critical | 7 regex patterns for illegal clauses |
| SecurityDepositLimitRule | Best Practice | Warning | Deposit > 2x/3x rent warning |
| LateFeeRule | F.S. § 83.56 | Medium | > 5% warning, > 10% fail |
| GracePeriodRule | Best Practice | Warning | Grace period presence check |
| BedBugDisclosureRule | F.S. § 83.50 | Warning | Known history disclosure |

**Additional Features**:
- Anomaly detection (10 suspicious language patterns)
- Canonical structure validation (14 expected sections)
- Section length analysis
- Unknown section flagging

#### web Verifier (agentpdf-compliance/)

| Rule | Statute | Severity | Implementation |
|------|---------|----------|----------------|
| Prohibited Provisions | F.S. § 83.47(1)(a) | Critical | Semantic clustering for waiver detection |
| Property Disposal | F.S. § 83.47(1)(b) | Critical | Property disposal clause detection |
| AS-IS Waivers | F.S. § 83.51(2)(a) | Critical | AS-IS + structural items |
| Security Deposit Return | F.S. § 83.49(2-3) | Critical | 15-day/30-day return validation |
| Attorney Fees | F.S. § 83.48 | Critical | Reciprocity requirement |
| Notice Requirements | F.S. § 83.56(2-3) | Critical | 3-day/7-day minimum validation |
| Month-to-Month Notice | F.S. § 83.57 | Critical | 30-day notice requirement (2024 update) |

**Implementation Differences**:
- Uses semantic clustering (multiple keyword groups)
- Numeric extractors for day counts
- Business day calendar for Florida
- 116 passing unit tests

### 1.2 Coverage Gap Analysis

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                        RULE COVERAGE MATRIX                               │
├───────────────────────────────────────────────────────────────────────────┤
│ Statute            │ agentPDF │  web   │ Notes                           │
├───────────────────────────────────────────────────────────────────────────┤
│ § 83.47 Prohibited │    ✓     │   ✓    │ web has better semantic cluster │
│ § 83.48 Atty Fees  │    ✗     │   ✓    │ MISSING in agentPDF             │
│ § 83.49 Deposit    │    ✓     │   ✓    │ agentPDF has bank location      │
│ § 83.50 Bed Bugs   │    ✓     │   ✗    │ MISSING in web                  │
│ § 83.51 AS-IS      │    ✗     │   ✓    │ MISSING in agentPDF             │
│ § 83.56 Late Fees  │    ✓     │   ✓    │ Different thresholds            │
│ § 83.56 Notices    │    ✗     │   ✓    │ MISSING in agentPDF             │
│ § 83.57 Month-Month│    ✗     │   ✓    │ MISSING in agentPDF (2024 law)  │
│ § 404.056 Radon    │    ✓     │   ✗    │ MISSING in web                  │
│ 24 CFR Lead Paint  │    ✓     │   ✗    │ MISSING in web                  │
│ Anomaly Detection  │    ✓     │   ✗    │ MISSING in web                  │
├───────────────────────────────────────────────────────────────────────────┤
│ Total Rules        │    8     │   10   │ web: 10 sub-rules across 4 mod  │
└───────────────────────────────────────────────────────────────────────────┘
```

### 1.3 Recommended Unified Verifier

The optimal architecture consolidates both implementations:

```rust
// Unified rule registry (proposed: agentpdf-compliance-unified/)
pub enum FloridaRule {
    // From agentPDF
    RadonDisclosure,         // § 404.056 - Critical
    SecurityDepositBank,     // § 83.49(2) - bank location
    LeadPaintDisclosure,     // 24 CFR Part 35
    BedBugDisclosure,        // § 83.50
    LateFeeReasonableness,   // § 83.56
    GracePeriodPresence,     // Best practice

    // From web
    ProhibitedProvisions,    // § 83.47(1)(a-b)
    AsIsStructural,          // § 83.51(2)(a)
    SecurityDepositReturn,   // § 83.49(3)
    AttorneyFeeReciprocity,  // § 83.48
    NoticeRequirements,      // § 83.56(2-3)
    MonthToMonthNotice,      // § 83.57 (2024 update)

    // New combined
    AnomalyDetection,        // agentPDF's 10 patterns
}
```

**Recommended Location**: Create `agentpdf-compliance-unified` crate that:
1. Lives in `web/agentpdf/crates/agentpdf-compliance-unified/`
2. Imports rules from both sources
3. Provides single `UnifiedComplianceEngine::check()` method
4. Exports to both WASM (web) and native (agentPDF MCP)

---

## Part 2: Template Selection from Corpus

### 2.1 Available Templates

The corpus contains 8 Florida templates:

| Template | File | Use Case |
|----------|------|----------|
| Single Family Lease | `residential_lease_single_family.md` | Houses, duplexes |
| Multifamily Lease | `residential_lease_multifamily.md` | Apartments, condos |
| 3-Day Notice | `3_day_notice_nonpayment.md` | Non-payment of rent |
| 7-Day Notice | `7_day_notice_noncompliance.md` | Lease violations |
| 30-Day Termination | `30_day_termination_notice.md` | Month-to-month end |
| Lease Renewal | `lease_renewal_notice.md` | Renewal/non-renewal |
| Security Deposit Claim | `security_deposit_claim_notice.md` | Deposit deductions |
| Move-In/Out Checklist | `move_in_move_out_checklist.md` | Property inspection |

### 2.2 Template Field Schema

All templates use consistent field patterns. Here's the proposed schema for the Florida lease:

```typescript
interface FloridaLeaseFields {
  // Landlord/Agent
  landlord: {
    name: string;
    company?: string;
    address: Address;
    phone: string;
    email: string;
  };

  // Tenant(s)
  tenants: Array<{
    name: string;
    phone?: string;
    email?: string;
    isAdult: boolean;
  }>;

  // Property
  property: {
    address: Address;
    unit?: string;
    type: 'single_family' | 'duplex' | 'apartment' | 'condo' | 'townhouse' | 'mobile_home';
    bedrooms: number;
    bathrooms: number;
    yearBuilt?: number;
    squareFeet?: number;
  };

  // Lease Terms
  terms: {
    startDate: Date;
    endDate?: Date;
    leaseType: 'fixed' | 'month_to_month';
    monthlyRent: number;
    rentDueDay: number;
    securityDeposit: number;
    lateFee?: number;
    lateFeeGracePeriod?: number;
  };

  // Utilities
  utilities: {
    electric: 'landlord' | 'tenant';
    gas: 'landlord' | 'tenant';
    water: 'landlord' | 'tenant';
    trash: 'landlord' | 'tenant';
    internet: 'landlord' | 'tenant';
  };

  // Optional sections
  pets?: {
    allowed: boolean;
    deposit?: number;
    monthlyRent?: number;
    types?: string[];
    weightLimit?: number;
  };

  parking?: {
    included: boolean;
    spaces?: number;
    location?: string;
    fee?: number;
  };
}
```

### 2.3 Template Selection UI Flow

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                     TEMPLATE SELECTION WORKFLOW                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Step 1: Choose Template Type                                           │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │  ○ Residential Lease                                               │ │
│  │    ├─ Single Family / Duplex                                       │ │
│  │    └─ Multifamily (Apartment, Condo, etc.)                         │ │
│  │  ○ Notices                                                         │ │
│  │    ├─ 3-Day Notice (Non-Payment)                                   │ │
│  │    ├─ 7-Day Notice (Violation)                                     │ │
│  │    └─ 30-Day Termination                                           │ │
│  │  ○ Other Documents                                                 │ │
│  │    ├─ Lease Renewal                                                │ │
│  │    ├─ Security Deposit Claim                                       │ │
│  │    └─ Move-In/Out Checklist                                        │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                              ↓                                          │
│  Step 2: Fill Required Fields                                           │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │  Landlord Information                                              │ │
│  │  ├─ Name: ________________________                                 │ │
│  │  ├─ Phone: ______________________                                  │ │
│  │  └─ Email: ______________________                                  │ │
│  │                                                                    │ │
│  │  Property Address                                                  │ │
│  │  ├─ Street: _____________________                                  │ │
│  │  ├─ City: _______ County: _______                                  │ │
│  │  └─ ZIP: ________                                                  │ │
│  │                                                                    │ │
│  │  [Continue to Tenant Info →]                                       │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                              ↓                                          │
│  Step 3: Generate & Verify                                              │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │  ✓ Generating PDF via Typst...                                     │ │
│  │  ✓ Running compliance verification...                              │ │
│  │                                                                    │ │
│  │  Results:                                                          │ │
│  │  ✓ PASS: Radon Disclosure (§ 404.056)                              │ │
│  │  ✓ PASS: Security Deposit Terms (§ 83.49)                          │ │
│  │  ✓ PASS: Lead Paint N/A (built 2010)                               │ │
│  │  ⚠ WARN: Consider adding grace period                              │ │
│  │                                                                    │ │
│  │  [Download PDF]  [Send for Signatures →]                           │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Part 3: Integration Architecture

### 3.1 Proposed Architecture (Two Independent Services)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    agentPDF.org (Compliance Platform)                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          ┌──────────────────────┐                           │
│                          │   agentPDF.org UI    │                           │
│                          │   (web/www/)         │                           │
│                          └──────────┬───────────┘                           │
│                                     │                                       │
│         ┌───────────────────────────┼───────────────────────────┐           │
│         │                           │                           │           │
│         ▼                           ▼                           ▼           │
│  ┌──────────────┐          ┌───────────────┐          ┌──────────────┐     │
│  │   Template   │          │   Document    │          │   Handoff    │     │
│  │   Selector   │          │   Verifier    │          │   to DocSign │     │
│  │              │          │              │          │              │     │
│  │  ○ Corpus    │          │  ○ Unified   │          │  ○ Redirect  │     │
│  │    REST API  │          │    Engine    │          │    with PDF  │     │
│  │  ○ Search    │          │  ○ 12+ Rules │          │  ○ Deep link │     │
│  │  ○ Preview   │          │  ○ Anomaly   │          │    params    │     │
│  └──────┬───────┘          └──────┬───────┘          └──────┬───────┘     │
│         │                         │                         │             │
│         ▼                         ▼                         │             │
│  ┌───────────────────────────────────────────────┐          │             │
│  │              SERVICES (agentPDF.org)          │          │             │
│  ├───────────────────────────────────────────────┤          │             │
│  │  ┌─────────────┐  ┌─────────────┐  ┌────────┐ │          │             │
│  │  │ agentPDF    │  │ corpus      │  │ WASM   │ │          │             │
│  │  │ MCP Server  │  │ REST API    │  │ comply │ │          │             │
│  │  └─────────────┘  └─────────────┘  └────────┘ │          │             │
│  └───────────────────────────────────────────────┘          │             │
│                                                             │             │
└─────────────────────────────────────────────────────────────┼─────────────┘
                                                              │
                            Redirect / Deep Link              │
                            (PDF passed via URL or temp link) │
                                                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                  getsignatures.org (Standalone Signing)                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          ┌──────────────────────┐                           │
│                          │  getsignatures.org   │                           │
│                          │   (docsign/www/)     │                           │
│                          └──────────┬───────────┘                           │
│                                     │                                       │
│         ┌───────────────────────────┼───────────────────────────┐           │
│         │                           │                           │           │
│         ▼                           ▼                           ▼           │
│  ┌──────────────┐          ┌───────────────┐          ┌──────────────┐     │
│  │   Upload     │          │   Recipient   │          │   Signing    │     │
│  │   Any PDF    │          │   Management  │          │   Workflow   │     │
│  │              │          │              │          │              │     │
│  │  ○ Drag/drop │          │  ○ Add users  │          │  ○ PAdES-B   │     │
│  │  ○ Or arrive │          │  ○ Set order  │          │  ○ Email     │     │
│  │    with PDF  │          │  ○ Assign flds│          │  ○ Track     │     │
│  └──────────────┘          └──────────────┘          └──────────────┘     │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────┐     │
│  │                    SERVICES (getsignatures.org)                   │     │
│  ├───────────────────────────────────────────────────────────────────┤     │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐               │     │
│  │  │ docsign     │  │ Cloudflare  │  │ Resend      │               │     │
│  │  │ WASM        │  │ Worker API  │  │ Email       │               │     │
│  │  │ (signing)   │  │ (sessions)  │  │ (invites)   │               │     │
│  │  └─────────────┘  └─────────────┘  └─────────────┘               │     │
│  └───────────────────────────────────────────────────────────────────┘     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Data Flow

```text
User Flow (agentPDF.org):

1. [SELECT TEMPLATE]
   User → agentPDF.org → corpus REST API
   ← Template list with metadata
   User selects "Florida Single Family Lease"

2. [FILL FIELDS]
   User fills form → Browser validates
   → agentPDF MCP `render_document` with inputs
   ← PDF bytes (base64)

3. [VERIFY COMPLIANCE]
   PDF bytes → unified compliance engine (WASM)
   ← ComplianceReport { violations: [], passed: 10/12, ... }

4. [CHOOSE ACTION]
   Option A: Download PDF
   → Browser downloads directly

   Option B: Send for Signatures (HANDOFF TO getsignatures.org)
   → agentPDF.org stores PDF temporarily (or encodes in URL)
   → User redirected to: getsignatures.org?doc=<temp_id> or #doc=<base64>
   → DocSign loads PDF automatically
   → User continues in DocSign's own UI (add recipients, place fields, send)
   → Signing workflow handled entirely by getsignatures.org

─────────────────────────────────────────────────────────────────────────────

User Flow (getsignatures.org - Independent):

1. [UPLOAD PDF]
   User → getsignatures.org (directly, no referral needed)
   → Drag/drop any PDF
   → DocSign WASM loads document

2. [ADD RECIPIENTS]
   User adds signers → Sets signing order
   → Assigns signature fields to recipients

3. [SEND FOR SIGNING]
   → DocSign WASM encrypts document
   → Cloudflare Worker creates session
   → Resend sends email invitations
   → Signers receive links, sign in browser
   → Signed PDF returned to sender

```

### 3.3 API Integration Points

#### Corpus → Web Integration

```typescript
// New endpoint in web/agentpdf/crates/agentpdf-server/
// GET /api/templates
async function listTemplates(): Promise<Template[]> {
  // Call corpus REST API
  const response = await fetch('http://corpus-server:8080/api/templates?state=florida');
  return response.json();
}

// GET /api/templates/:id
async function getTemplate(id: string): Promise<TemplateContent> {
  const response = await fetch(`http://corpus-server:8080/api/templates/${id}`);
  return response.json();
}
```

#### Web → agentPDF Integration

```typescript
// Render filled template via MCP
async function renderFilledTemplate(
  templateId: string,
  fields: FloridaLeaseFields
): Promise<Uint8Array> {
  // Option A: Direct MCP call (if MCP server accessible)
  const mcpResponse = await mcpClient.call('render_document', {
    source: `typst://templates/florida_lease`,
    inputs: convertFieldsToTypstInputs(fields),
    format: 'pdf'
  });
  return base64Decode(mcpResponse.artifact);

  // Option B: HTTP endpoint (add to agentPDF server)
  const response = await fetch('http://localhost:3001/render', {
    method: 'POST',
    body: JSON.stringify({ template: 'florida_lease', inputs: fields })
  });
  return new Uint8Array(await response.arrayBuffer());
}
```

#### agentPDF.org → getsignatures.org Handoff

The integration between agentPDF.org and getsignatures.org is a **redirect/handoff**, not an embedding.
DocSign remains a fully standalone service with its own frontend.

```typescript
// In web/www/index.html - handoff to getsignatures.org
async function sendForSignatures(pdfBytes: Uint8Array, filename: string) {
  // Option A: Pass PDF via URL fragment (for smaller documents < 2MB)
  const pdfBase64 = btoa(String.fromCharCode(...pdfBytes));
  const docSignUrl = `https://getsignatures.org/#doc=${encodeURIComponent(pdfBase64)}&name=${encodeURIComponent(filename)}`;

  // Option B: Store temporarily and pass reference (for larger documents)
  // const tempId = await storePdfTemporarily(pdfBytes);
  // const docSignUrl = `https://getsignatures.org/?ref=${tempId}&source=agentpdf`;

  // Redirect user to DocSign - they complete the signing workflow there
  window.location.href = docSignUrl;

  // Alternative: Open in new tab
  // window.open(docSignUrl, '_blank');
}

// Button handler in agentPDF.org UI
document.getElementById('send-for-signatures').addEventListener('click', () => {
  const pdfBytes = getCurrentPdfBytes();
  const filename = getCurrentFilename();
  sendForSignatures(pdfBytes, filename);
});
```

**What happens after handoff:**
1. User arrives at getsignatures.org with PDF pre-loaded
2. DocSign's own UI handles: recipient management, field placement, sending
3. User completes entire signing workflow in getsignatures.org
4. No return callback needed (DocSign is the end of the workflow)

---

## Part 4: Implementation Plan

### Phase 1: Unified Compliance Engine (Week 1-2)

**Goal**: Merge verifier logic from both projects

#### Tasks:

1. **Create unified crate**
   ```text
   web/agentpdf/crates/agentpdf-compliance-unified/
   ├── Cargo.toml
   ├── src/
   │   ├── lib.rs           # UnifiedComplianceEngine
   │   ├── rules/
   │   │   ├── mod.rs
   │   │   ├── radon.rs     # From agentPDF
   │   │   ├── lead_paint.rs # From agentPDF
   │   │   ├── bed_bug.rs   # From agentPDF
   │   │   ├── prohibited.rs # From web
   │   │   ├── deposit.rs   # Merged (both)
   │   │   ├── attorney.rs  # From web
   │   │   ├── notices.rs   # From web
   │   │   └── anomaly.rs   # From agentPDF
   │   ├── patterns.rs      # Merged keyword patterns
   │   └── extractors.rs    # Merged extractors
   └── tests/
   ```

2. **Port missing rules to each system**
   - agentPDF: Add § 83.48 (attorney fees), § 83.57 (30-day notice), § 83.51 (AS-IS)
   - web: Add § 404.056 (radon), 24 CFR (lead paint), § 83.50 (bed bugs), anomaly detection

3. **Create shared test suite**
   - 150+ unit tests covering all rules
   - Property-based tests for edge cases
   - Integration tests with sample PDFs

### Phase 2: Template Integration (Week 2-3)

**Goal**: Connect corpus templates to web UI

#### Tasks:

1. **Add template selector to web UI**
   - New React/vanilla component in `www/index.html`
   - Template cards with preview thumbnails
   - Category filtering (leases, notices, checklists)

2. **Create field form generator**
   - Dynamic form based on template schema
   - Validation per field type
   - Auto-save to IndexedDB

3. **Connect to agentPDF for rendering**
   - HTTP endpoint or direct MCP call
   - Loading state with progress indicator
   - Error handling for compilation failures

4. **Add template preview**
   - Live preview as fields are filled
   - Highlight required vs optional fields
   - Show compliance warnings in real-time

### Phase 3: DocSign Handoff Integration (Week 3-4)

**Goal**: Enable seamless handoff from agentPDF.org to getsignatures.org

#### Tasks:

1. **Add "Send for Signatures" button to agentPDF.org**
   - Appears after compliance check passes
   - Confirms user wants to leave agentPDF.org
   - Offers download as alternative

2. **Implement PDF handoff mechanism**
   - Option A: Base64 encode PDF in URL fragment (< 2MB documents)
   - Option B: Temporary storage with reference ID (larger documents)
   - Include source attribution (`?source=agentpdf`)

3. **Add deep link support to getsignatures.org**
   - Parse incoming PDF from URL fragment or reference
   - Auto-load document into DocSign UI
   - Skip upload step when document provided
   - Show "Document from agentPDF.org" attribution

4. **Test the handoff flow**
   - Verify PDF integrity after transfer
   - Test with various document sizes
   - Ensure DocSign UI works correctly with pre-loaded docs

### Phase 4: Polish & Testing (Week 4-5)

**Goal**: Production-ready integration

#### Tasks:

1. **E2E test suite**
   - Puppeteer tests for full workflow
   - Mobile viewport testing
   - Offline capability testing

2. **Performance optimization**
   - WASM size reduction
   - Lazy loading for DocSign module
   - Template caching

3. **Error handling**
   - Network failure recovery
   - Partial completion state save
   - User-friendly error messages

4. **Documentation**
   - User guide for template filling
   - API documentation
   - Deployment guide

---

## Part 5: File Changes Required

### agentPDF (this repo)

| File | Change |
|------|--------|
| `src/verifier/rules/mod.rs` | Add missing rules (attorney fees, notices, AS-IS) |
| `src/verifier/rules/attorney.rs` | NEW: Port from web |
| `src/verifier/rules/notices.rs` | NEW: Port from web |
| `src/verifier/rules/as_is.rs` | NEW: Port from web |
| `src/mcp/tools.rs` | Add `render_filled_template` tool |
| `Cargo.toml` | Add HTTP feature for render endpoint |

### web (../web) → agentPDF.org

| File | Change |
|------|--------|
| `www/index.html` | Add template selector UI |
| `www/index.html` | Add "Send for Signatures" button with handoff to getsignatures.org |
| `www/js/template-selector.js` | NEW: Template selection logic |
| `www/js/field-form.js` | NEW: Dynamic form generator |
| `www/js/docsign-handoff.js` | NEW: Redirect logic to getsignatures.org (NOT embedding) |
| `agentpdf/crates/agentpdf-compliance/` | Add missing rules from agentPDF |
| `agentpdf/crates/agentpdf-server/` | Add corpus proxy endpoints |

### corpus (../corpus)

| File | Change |
|------|--------|
| `corpus-server/src/handlers/templates.rs` | Expose template listing/retrieval |
| `corpus-bench/templates/florida/*.md` | Add field schemas as YAML frontmatter |

### docsign (../../docsign) → getsignatures.org

| File | Change |
|------|--------|
| `www/index.html` | Add deep link parsing for incoming PDFs from agentPDF.org |
| `www/index.html` | Add URL fragment/query param handling for pre-loaded docs |
| `www/index.html` | Show source attribution when document comes from referral |
| `www/sign.js` | Handle `#doc=<base64>` and `?ref=<id>` URL patterns |

**Note**: DocSign remains a standalone service. These changes only add the ability to receive documents via deep links - the core signing workflow is unchanged.

---

## Part 6: API Contracts

### Template List API

```text
GET /api/templates?state=florida&category=lease

Response:
{
  "templates": [
    {
      "id": "fl-res-lease-sf-2024",
      "name": "Florida Single Family Residential Lease",
      "category": "lease",
      "description": "Standard lease for single-family homes and duplexes",
      "fields_count": 45,
      "statutes": ["83.49", "83.50", "83.51", "404.056"],
      "updated_at": "2024-07-01"
    }
  ]
}
```

### Template Render API

```text
POST /api/render

Request:
{
  "template_id": "fl-res-lease-sf-2024",
  "fields": {
    "landlord_name": "John Smith",
    "property_address": "123 Main St, Miami, FL 33101",
    "monthly_rent": 2500,
    "security_deposit": 2500,
    ...
  }
}

Response:
{
  "pdf_base64": "JVBERi0xLjQK...",
  "compliance": {
    "status": "compliant_with_warnings",
    "passed": 10,
    "warnings": 2,
    "failed": 0,
    "violations": [
      {
        "statute": "Best Practice",
        "severity": "warning",
        "message": "Consider adding grace period for rent payment"
      }
    ]
  }
}
```

### DocSign Session API

```text
POST /session

Request:
{
  "encrypted_document": "base64...",
  "metadata": {
    "filename": "florida_lease.pdf",
    "page_count": 12,
    "created_by": "landlord@example.com"
  },
  "recipients": [
    {
      "id": "tenant-1",
      "name": "Jane Doe",
      "email": "jane@example.com",
      "role": "signer"
    }
  ],
  "fields": [
    {
      "id": "sig-1",
      "type": "signature",
      "recipient_id": "tenant-1",
      "page": 12,
      "x_percent": 0.1,
      "y_percent": 0.8
    }
  ]
}

Response:
{
  "session_id": "abc-123",
  "expires_at": "2024-12-26T00:00:00Z"
}
```

---

## Part 7: Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Template render time | < 2s | Time from submit to PDF display |
| Compliance check time | < 500ms | WASM execution time |
| DocSign session creation | < 1s | API round-trip time |
| E2E workflow | < 5 min | User test: select → fill → verify → send |
| Test coverage | > 90% | Combined unit + integration tests |
| WASM bundle size | < 2MB | gzip compressed |

---

## Part 8: Risk Mitigation

| Risk | Mitigation |
|------|------------|
| WASM size bloat in agentPDF.org | Tree-shaking, lazy loading (DocSign not embedded) |
| MCP server availability | Fallback to direct Typst WASM compilation |
| Large PDF handoff to DocSign | Option B: temporary storage with reference ID |
| DocSign rate limits | Queue emails, show user remaining quota |
| Template schema changes | Version templates, migration scripts |
| Compliance rule conflicts | Priority system, unified test suite |
| User confusion between services | Clear branding, "Powered by" attributions |
| Deep link security | Validate incoming PDFs, sanitize URL params |

---

## Appendix A: Florida Statute Quick Reference

| Statute | Summary | Compliance Level |
|---------|---------|------------------|
| F.S. § 83.47 | Prohibited lease provisions | Critical |
| F.S. § 83.48 | Attorney fees must be reciprocal | Critical |
| F.S. § 83.49 | Security deposit handling | Critical |
| F.S. § 83.50 | Disclosure requirements | Warning |
| F.S. § 83.51 | Landlord maintenance obligations | Critical |
| F.S. § 83.52 | Tenant maintenance obligations | Info |
| F.S. § 83.53 | Landlord entry (12-hour notice) | Warning |
| F.S. § 83.56 | Default and remedies (3/7 day) | Critical |
| F.S. § 83.57 | Termination (30-day since 2024) | Critical |
| F.S. § 404.056 | Radon gas disclosure | Critical |
| 24 CFR Part 35 | Lead-based paint (pre-1978) | High |

---

## Appendix B: Template Field Mapping

```text
corpus markdown field          → agentPDF Typst input
─────────────────────────────────────────────────────
_______________________        → #get("field_name")
☐ Option 1 ☐ Option 2          → #if get_bool("option") [text]
$________                      → #format_money(get_num("amount"))
___/___ /____                  → #format_date(get("date"))
```

---

## Conclusion

This integration plan provides a clear path to connect the four projects around the Florida lease workflow while maintaining service independence. The key architectural decisions are:

1. **Unified compliance engine** with 12+ rules from both sources
2. **Corpus as template source** with structured field schemas
3. **agentPDF as render engine** via MCP or HTTP
4. **DocSign as standalone signature service** at getsignatures.org (independent, not embedded)
5. **agentPDF.org as compliance frontend** with handoff to getsignatures.org for signing

### Deployment Summary

| Service | Domain | Independence |
|---------|--------|--------------|
| Compliance Platform | **agentPDF.org** | Standalone - template selection, field population, verification |
| Signature Platform | **getsignatures.org** | Standalone - any PDF, any user, full signing workflow |

The handoff model ensures:
- Each service provides independent value
- Users can discover either service separately
- No tight coupling between codebases
- Simpler deployment and scaling
- Clear product boundaries for marketing

The phased approach allows incremental delivery with testable milestones at each stage.
