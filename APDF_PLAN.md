# agentPDF Strategic Plan

> Florida-First Document Generation, Completion & Signature Platform

**Document Version**: 4.0
**Last Updated**: December 31, 2025
**Status**: Active Development - Unified Platform

---

## Vision: One Platform, Complete Workflow

**agentPDF = Template Generation + Field Completion + Signature Dispatch**

Users should never leave agentPDF to get signatures. The complete workflow:

```
Template Selection → Form Wizard → PDF Generation → Field Placement → Signature Dispatch → Download
                                                                    ↓
                                                           (Optional: Download without signatures)
```

### Core Principles

1. **Florida Excellence** - Every Florida document type must be legally compliant
2. **Scrivener Standard** - Present options, never recommend (prevents UPL)
3. **Self-Contained Experience** - NO external redirects, everything in one app
4. **Signature Dispatch Default** - Last step is always "Send for Signatures" (with download-only option)
5. **Shared Infrastructure** - Reuse docsign-web signing components, email-proxy Lambda

---

## Part I: Unified Architecture

### 1.1 The Integration Strategy

agentPDF merges functionality from two sources:

| Source | What We Take | How We Use It |
|--------|--------------|---------------|
| **agentpdf-server** | Typst template rendering | Server-side PDF generation |
| **docsign-web** | Signing WASM, session management, email dispatch | Client-side signature flow |
| **email-proxy** | AWS SES Lambda | Replace Resend API everywhere |

### 1.2 Complete User Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        agentPDF UNIFIED FLOW                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. TEMPLATE SELECTION                                                       │
│     └─ User picks: Lease, Purchase Contract, Bill of Sale, etc.             │
│                                                                              │
│  2. FORM WIZARD (Scrivener-Compliant)                                       │
│     └─ User fills template fields (names, dates, amounts)                   │
│     └─ Presents options with definitions, never recommendations             │
│                                                                              │
│  3. SERVER-SIDE GENERATION                                                  │
│     └─ agentpdf-server renders Typst template → PDF                         │
│     └─ Returns base PDF ready for field placement                           │
│                                                                              │
│  4. FIELD PLACEMENT (Template Completion Engine)                            │
│     └─ User places: Text, Signature, Initials, Checkbox, Date fields       │
│     └─ Optional: Split/Merge pages                                          │
│     └─ Uses pdfjoin-wasm for PDF operations                                 │
│                                                                              │
│  5. SIGNATURE DISPATCH (Default Last Step) ← NEW INTEGRATION               │
│     ├─ "Send for Signatures" button (PRIMARY action)                        │
│     │   └─ Add recipients (name + email)                                    │
│     │   └─ Assign fields to recipients                                      │
│     │   └─ Send via email-proxy Lambda (AWS SES)                           │
│     │   └─ Recipients sign via magic link                                   │
│     │                                                                        │
│     └─ "Download PDF Only" link (secondary action)                          │
│         └─ For users who will collect signatures offline                    │
│                                                                              │
│  6. SIGNATURE COLLECTION (docsign-web components)                           │
│     └─ Recipients open magic link                                           │
│     └─ Sign using docsign-wasm (local-first)                               │
│     └─ Audit trail, timestamps                                              │
│                                                                              │
│  7. COMPLETION                                                              │
│     └─ All parties signed → Sender gets final PDF                          │
│     └─ Email notifications via email-proxy Lambda                          │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.3 Shared Components Map

```
m3-agentpdfmvp/                      m3-getsigsmvp/
├── apps/agentpdf-web/               ├── apps/docsign-web/
│   ├── src/ts/                      │   ├── src/ts/
│   │   ├── template-editor.ts       │   │   ├── signature-capture.ts    ← SHARE
│   │   ├── pdf-bridge.ts            │   │   ├── typed-signature.ts      ← SHARE
│   │   └── app.ts                   │   │   ├── local-session-manager.ts← SHARE
│   │                                │   │   └── sync-manager.ts         ← SHARE
│   └── wasm/                        │   └── wasm/
│       └── (pdfjoin-based)          │       └── (docsign signing)       ← SHARE
│                                    │
├── apps/agentpdf-server/            ├── crates/email-proxy/             ← USE THIS
│   └── (Typst rendering)            │   └── (AWS SES Lambda)
│                                    │
└── crates/                          └── crates/
    ├── pdfjoin-core/                    ├── docsign-core/               ← SHARE
    ├── compliance-engine/               └── shared-crypto/              ← SHARE
    └── typst-engine/
```

---

## Part II: Email Infrastructure (Replacing Resend)

### 2.1 Migration: Resend → email-proxy Lambda

**Current State (Resend):**
- docsign-web worker calls `https://api.resend.com/emails`
- Requires `RESEND_API_KEY` secret
- Limited to Resend free tier: 100/day, 3000/month
- Cost scales with volume

**Target State (email-proxy Lambda):**
- Call email-proxy Lambda (AWS SES backend)
- Uses `EMAIL_PROXY_URL` + `EMAIL_PROXY_API_KEY`
- AWS SES: $0.10 per 1000 emails
- Better deliverability (dedicated IP possible)

### 2.2 email-proxy Lambda Details

**Endpoint**: `https://5wbbpgjw7acyu4sgjqksmsqtvq0zajks.lambda-url.us-east-2.on.aws`

**API (Resend-compatible)**:
```json
POST /send
{
  "from": "agentPDF <noreply@agentpdf.org>",
  "to": ["recipient@example.com"],
  "subject": "Sign: Florida Purchase Contract",
  "html": "<html>...</html>"
}
```

**Deliverability Features** (from email-proxy):
- DKIM signing (AWS SES)
- SPF alignment
- Unsubscribe headers
- Bounce handling
- 10x cheaper than Resend at scale

### 2.3 Migration Tasks

| File | Change |
|------|--------|
| `apps/docsign-web/worker/src/lib.rs` | Replace `RESEND_API_URL` with `EMAIL_PROXY_URL` |
| `apps/docsign-web/worker/wrangler.toml` | Add `EMAIL_PROXY_URL`, `EMAIL_PROXY_API_KEY` secrets |
| `DEPLOY.md` | Update deployment docs |
| `README.md` | Update setup instructions |
| `apps/docsign-web/UX006_*.md` | Update email references |

---

## Part III: Document Type Hierarchy

### Phase 1.0 - Core Florida Documents (Complete)

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
├── Contractor::
│   ├── Invoice                # Standard contractor invoice
│   ├── NoticeOfCommencement   # Ch. 713.13
│   ├── NoticeToOwner          # Ch. 713.06
│   ├── ClaimOfLien            # Ch. 713.08
│   └── ReleaseOfLien          # Ch. 713.20/21
│
└── BillOfSale::
    ├── Car                    # Motor vehicle (HSMV 82050)
    ├── Boat                   # Vessel (HSMV 87002)
    ├── Trailer                # Ch. 319/320
    ├── JetSki                 # Ch. 328
    └── MobileHome             # Ch. 319/723
```

---

## Part IV: Implementation Phases

### Phase 1: Signature Dispatch Integration (Priority 1) 🚧 IN PROGRESS

**Goal**: Add signature dispatch as default last step in agentpdf-web

- [x] Copy from docsign-web to agentpdf-web:
  - `src/ts/signature-capture.ts`
  - `src/ts/typed-signature.ts`
  - `src/ts/local-session-manager.ts`
  - `src/ts/sync-manager.ts`
  - `src/ts/sync-events.ts`
  - `src/ts/mobile-signature-modal.ts`
  - `src/ts/signature-modal.ts`
  - `src/ts/error-messages.ts`
  - `src/ts/error-ui.ts`
- [ ] Add recipient management UI:
  - `src/ts/recipient-manager.ts` (NEW)
  - `src/ts/dispatch-modal.ts` (NEW)
  - Field-to-recipient assignment
  - Signing order (parallel/sequential)
- [ ] Add "Send for Signatures" button as PRIMARY action
- [ ] Add "Download PDF Only" as secondary link

### Phase 2: Signing Flow Integration (Priority 2) 🚧 IN PROGRESS

**Goal**: Enable recipients to sign within agentpdf ecosystem

- [ ] Add `www/sign.html` route to agentpdf-web
- [ ] Copy signing page components from docsign-web
- [ ] Integrate docsign-wasm signing operations into agentpdf-wasm
- [ ] Session management via Cloudflare D1
- [ ] Magic link authentication (session, recipient, key params)

### Phase 3: WASM Integration (Priority 3) 🚧 IN PROGRESS

**Goal**: Add signing capabilities to agentpdf-wasm

- [ ] Add PDF signing functions (sign_document, sign_document_with_progress)
- [ ] Add signature field placement
- [ ] Add text field, checkbox, date field operations
- [ ] Add audit chain for compliance
- [ ] Add session validation

### Phase 4: Shared Crate Extraction (Priority 4)

**Goal**: Create shared crates for both apps

- [ ] Create `crates/signing-session/`:
  - Session data structures
  - Validation logic
  - Status management
- [ ] Create `crates/email-templates/`:
  - Signing invitation
  - Completion notification
  - Reminder emails
- [ ] Update both apps to use shared crates

### Phase 5: Email Infrastructure Migration (LAST)

**Goal**: Replace all Resend usage with email-proxy Lambda

- [ ] Update `apps/docsign-web/worker/src/lib.rs`:
  - Replace `RESEND_API_URL` constant
  - Replace `RESEND_API_KEY` with `EMAIL_PROXY_API_KEY`
  - Update `send_email_notification()` function
  - Update `send_admin_warning()` function
  - Update `handle_invite()` function
- [ ] Update `apps/docsign-web/worker/wrangler.toml`:
  - Remove RESEND references
  - Add EMAIL_PROXY_URL binding
- [ ] Update documentation:
  - DEPLOY.md
  - README.md
  - UX006_*.md files
- [ ] Test email delivery end-to-end

---

## Part V: Scrivener Standard (Legal Compliance)

### 5.1 The Doctrine

agentPDF is a **scrivener** (intelligent typewriter), not a legal advisor.

| Allowed | Not Allowed |
|---------|-------------|
| "Do you want to include X?" | "We recommend X" |
| "X is defined as..." | "Based on your situation, you should..." |
| Present options with definitions | Apply law to user's specific facts |

### 5.2 Required Disclaimers

Every generated document must include:

```
DISCLAIMER: This document was prepared using agentPDF.org, a document
preparation service. No attorney-client relationship is created. This
is not legal advice. For complex matters, consult a Florida attorney.
```

---

## Part VI: Template Completion Engine

### 6.1 Allowed Field Types

```typescript
enum FieldType {
  TextField = 'text',       // Fill in names, dates, amounts
  Signature = 'signature',  // Mark signature locations
  Initials = 'initials',    // Contract revision acknowledgment
  Checkbox = 'checkbox',    // Yes/No selections
  DateField = 'date',       // Auto-formatted date entry
}

// EXPLICITLY NOT ALLOWED (prevents contract drafting):
// - WhiteoutTool (could hide contract terms)
// - TextReplaceTool (could alter clauses)
// - FreeformTextTool (could add paragraphs)
```

### 6.2 Field-to-Recipient Assignment

New feature for signature dispatch:

```typescript
interface FieldAssignment {
  fieldId: string;
  recipientId: string;
  required: boolean;
  order?: number; // For sequential signing
}

interface Recipient {
  id: string;
  name: string;
  email: string;
  role?: string; // "Buyer", "Seller", "Witness"
}
```

---

## Part VII: API Endpoints

### 7.1 agentpdf-server (Typst Rendering)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/render` | POST | Render Typst template → PDF |
| `/api/templates` | GET | List available templates |
| `/api/compliance` | POST | Check document compliance |
| `/api/document-types` | GET | List supported document types |

### 7.2 Cloudflare Worker (Session Management)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/session` | POST | Create signing session |
| `/session/:id` | GET | Get session data |
| `/session/:id/signed` | POST | Submit signature |
| `/session/:id/status` | GET | Check completion status |
| `/invite` | POST | Send signing invitations |

---

## Part VIII: Success Metrics

### Florida MVP Launch Criteria

| Metric | Target | Status |
|--------|--------|--------|
| All Phase 1.0 documents complete | 100% | ✅ |
| Compliance test coverage | >90% | ✅ |
| Signature dispatch integrated | 100% | 🚧 |
| Email via email-proxy | 100% | ✅ |
| Resend references removed | 100% | ✅ |

---

## Part IX: File Structure After Integration

```
m3-agentpdfmvp/
├── apps/
│   ├── agentpdf-web/           # Main web app
│   │   ├── src/ts/
│   │   │   ├── app.ts
│   │   │   ├── template-editor.ts
│   │   │   ├── pdf-bridge.ts
│   │   │   ├── signature-capture.ts    ← FROM docsign
│   │   │   ├── typed-signature.ts      ← FROM docsign
│   │   │   ├── local-session-manager.ts← FROM docsign
│   │   │   ├── recipient-manager.ts    ← NEW
│   │   │   └── dispatch-modal.ts       ← NEW
│   │   ├── wasm/
│   │   │   └── (pdfjoin + docsign signing)
│   │   └── www/
│   │       ├── index.html
│   │       └── sign.html               ← FROM docsign
│   │
│   ├── agentpdf-server/        # Typst rendering
│   │   └── src/
│   │
│   └── docsign-web/            # Legacy, to be merged
│       └── worker/             # Keep for now, migrate to agentpdf
│
├── crates/
│   ├── compliance-engine/
│   ├── pdfjoin-core/
│   ├── typst-engine/
│   ├── email-proxy/            ← FROM getsigsmvp (or reference)
│   └── signing-session/        ← NEW shared crate
│
└── output/                     # Test PDFs
```

---

## Part X: Security & DDoS Mitigation

### 10.1 Attack Surface Analysis

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         agentPDF ATTACK VECTORS                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. TYPST RENDERING (agentpdf-server) - HIGH RISK                           │
│     └─ Attack: Malformed templates, infinite loops, memory bombs            │
│     └─ Impact: Server CPU/memory exhaustion, denial of service              │
│     └─ Vector: POST /api/render with crafted template data                  │
│                                                                              │
│  2. PDF UPLOAD/PROCESSING - HIGH RISK                                       │
│     └─ Attack: Malicious PDFs, ZIP bombs, billion laughs                    │
│     └─ Impact: WASM memory exhaustion, browser crashes                      │
│     └─ Vector: File upload in template editor                               │
│                                                                              │
│  3. EMAIL DISPATCH (email-proxy Lambda) - MEDIUM RISK                       │
│     └─ Attack: Mass email flooding, recipient enumeration                   │
│     └─ Impact: AWS SES quota exhaustion, abuse complaints                   │
│     └─ Vector: POST /invite with many recipients                            │
│                                                                              │
│  4. SESSION MANAGEMENT (Cloudflare Worker) - MEDIUM RISK                    │
│     └─ Attack: Session flooding, magic link brute force                     │
│     └─ Impact: D1 database overwhelm, unauthorized access                   │
│     └─ Vector: POST /session spam, GET /session/:id enumeration             │
│                                                                              │
│  5. SIGNATURE COLLECTION (Cloudflare Worker) - LOW-MEDIUM                   │
│     └─ Attack: Signature submission flooding, replay attacks                │
│     └─ Impact: D1 writes exhausted, invalid signatures stored               │
│     └─ Vector: POST /session/:id/signed with fake signatures                │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 10.2 Rate Limiting Strategy

| Endpoint | Rate Limit | Window | Per-IP Tier | Tier Requests/Window | Key | Action on Exceed |
|----------|-----------|--------|-------------|---------------------|-----|------------------|
| `/api/render` | 10 req | 1 min | SessionWrite | 10/min | IP | 429 + 60s block |
| `/api/templates` | 60 req | 1 min | SessionRead | 60/min | IP | 429 |
| `/session` (POST) | 5 req | 1 min | HealthCheck | 5/min | IP | 429 + CAPTCHA |
| `/session/:id` (GET) | 30 req | 1 min | SessionRead | 30/min | session | 429 |
| `/session/:id/signed` | 3 req | 10 min | SessionWrite | 3/10min | recipient | 429 + warn |
| `/invite` | 10 req | 1 hour | RequestLink | 10/hour | sender | 429 + queue |
| File upload | 5 uploads | 10 min | SessionWrite | 5/10min | IP | 429 |

#### Rate Limit Tier Definitions

| Tier | Purpose | Requests/min | Requests/hour | Requests/day |
|------|---------|--------------|----------------|--------------|
| HealthCheck | Session health probes | 30 | 1800 | 43200 |
| SessionRead | Read-only operations (GET) | 60 | 3600 | 86400 |
| SessionWrite | Write operations (POST/PUT) | 10 | 600 | 14400 |
| RequestLink | Link dispatch (email invites) | 10/hour | 10 | 240 |

### 10.3 Implementation Details

#### Cloudflare Worker Rate Limiting (D1-backed)

```typescript
// Rate limiter using Cloudflare D1 for distributed state
interface RateLimitEntry {
  key: string;           // IP or user identifier
  endpoint: string;      // Endpoint pattern
  count: number;         // Request count in window
  window_start: number;  // Unix timestamp
  blocked_until: number; // Block expiry (0 = not blocked)
}

// D1 table schema
// CREATE TABLE rate_limits (
//   key TEXT NOT NULL,
//   endpoint TEXT NOT NULL,
//   count INTEGER DEFAULT 1,
//   window_start INTEGER NOT NULL,
//   blocked_until INTEGER DEFAULT 0,
//   PRIMARY KEY (key, endpoint)
// );

async function checkRateLimit(
  db: D1Database,
  key: string,
  endpoint: string,
  limit: number,
  windowSec: number,
  blockSec: number
): Promise<{ allowed: boolean; retryAfter?: number }> {
  const now = Math.floor(Date.now() / 1000);

  // Check for active block
  const entry = await db.prepare(`
    SELECT count, window_start, blocked_until
    FROM rate_limits
    WHERE key = ? AND endpoint = ?
  `).bind(key, endpoint).first<RateLimitEntry>();

  if (entry?.blocked_until && entry.blocked_until > now) {
    return { allowed: false, retryAfter: entry.blocked_until - now };
  }

  // Check/update rate limit
  if (!entry || (now - entry.window_start) > windowSec) {
    // New window
    await db.prepare(`
      INSERT OR REPLACE INTO rate_limits (key, endpoint, count, window_start, blocked_until)
      VALUES (?, ?, 1, ?, 0)
    `).bind(key, endpoint, now).run();
    return { allowed: true };
  }

  if (entry.count >= limit) {
    // Rate exceeded - block
    const blockedUntil = now + blockSec;
    await db.prepare(`
      UPDATE rate_limits SET blocked_until = ? WHERE key = ? AND endpoint = ?
    `).bind(blockedUntil, key, endpoint).run();
    return { allowed: false, retryAfter: blockSec };
  }

  // Increment counter
  await db.prepare(`
    UPDATE rate_limits SET count = count + 1 WHERE key = ? AND endpoint = ?
  `).bind(key, endpoint).run();
  return { allowed: true };
}
```

#### agentpdf-server Rate Limiting (In-Memory)

```rust
// For agentpdf-server (Typst rendering) - in-memory rate limiter
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    limit: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: u32, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            limit,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check(&self, key: &str) -> Result<(), Duration> {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        if let Some((count, start)) = requests.get_mut(key) {
            if now.duration_since(*start) > self.window {
                // Reset window
                *count = 1;
                *start = now;
                Ok(())
            } else if *count >= self.limit {
                // Rate exceeded
                Err(self.window - now.duration_since(*start))
            } else {
                *count += 1;
                Ok(())
            }
        } else {
            requests.insert(key.to_string(), (1, now));
            Ok(())
        }
    }
}
```

### 10.4 PDF Processing Mitigations

```rust
// Maximum sizes for PDF operations
const MAX_PDF_SIZE: usize = 50 * 1024 * 1024;  // 50 MB
const MAX_PAGE_COUNT: usize = 500;
const MAX_OBJECT_COUNT: usize = 100_000;
const MAX_STREAM_SIZE: usize = 10 * 1024 * 1024; // 10 MB per stream
const RENDER_TIMEOUT_MS: u64 = 30_000;  // 30 second timeout

// Validation before processing
fn validate_pdf(bytes: &[u8]) -> Result<(), PdfError> {
    if bytes.len() > MAX_PDF_SIZE {
        return Err(PdfError::TooLarge);
    }

    let doc = lopdf::Document::load_mem(bytes)?;

    if doc.get_pages().len() > MAX_PAGE_COUNT {
        return Err(PdfError::TooManyPages);
    }

    if doc.objects.len() > MAX_OBJECT_COUNT {
        return Err(PdfError::TooManyObjects);
    }

    // Check for stream bombs (compressed content that expands massively)
    for (_, obj) in &doc.objects {
        if let lopdf::Object::Stream(stream) = obj {
            if stream.content.len() > MAX_STREAM_SIZE {
                return Err(PdfError::StreamTooLarge);
            }
        }
    }

    Ok(())
}
```

### 10.5 Email Dispatch Protections

```rust
// email-proxy Lambda protections
const MAX_RECIPIENTS_PER_REQUEST: usize = 10;
const MAX_EMAILS_PER_SENDER_HOUR: usize = 50;
const MAX_EMAILS_PER_SENDER_DAY: usize = 200;
const MAX_EMAIL_SIZE_KB: usize = 500;

// DynamoDB table for sender rate tracking
// PK: sender_email, SK: timestamp

async fn validate_email_request(
    sender: &str,
    recipients: &[String],
    body_size: usize,
) -> Result<(), EmailError> {
    if recipients.len() > MAX_RECIPIENTS_PER_REQUEST {
        return Err(EmailError::TooManyRecipients);
    }

    if body_size > MAX_EMAIL_SIZE_KB * 1024 {
        return Err(EmailError::BodyTooLarge);
    }

    // Check hourly/daily limits (DynamoDB query)
    let hour_count = get_sender_count(sender, Duration::hours(1)).await?;
    if hour_count >= MAX_EMAILS_PER_SENDER_HOUR {
        return Err(EmailError::HourlyLimitExceeded);
    }

    let day_count = get_sender_count(sender, Duration::days(1)).await?;
    if day_count >= MAX_EMAILS_PER_SENDER_DAY {
        return Err(EmailError::DailyLimitExceeded);
    }

    Ok(())
}
```

### 10.6 Session Security

```typescript
// Magic link security
const MAGIC_LINK_EXPIRY_HOURS = 72;  // 3 days
const MAX_SIGNING_ATTEMPTS = 5;
const SESSION_KEY_BYTES = 32;  // 256-bit session keys

interface SessionSecurity {
  // URL parameters (encrypted)
  session: string;    // Session UUID
  recipient: string;  // Recipient UUID
  key: string;        // Signing key (HKDF-derived)

  // Server-side validation
  ip_binding?: string;      // Optional IP lock
  user_agent_hash?: string; // Browser fingerprint
  created_at: number;       // For expiry check
  attempts: number;         // Failed attempt counter
}

// Brute force protection for magic links
async function validateMagicLink(
  db: D1Database,
  sessionId: string,
  recipientId: string,
  key: string,
  ip: string
): Promise<ValidationResult> {
  const session = await getSession(db, sessionId);

  if (!session) {
    return { valid: false, error: 'SESSION_NOT_FOUND' };
  }

  // Check expiry
  const ageHours = (Date.now() - session.created_at) / (1000 * 60 * 60);
  if (ageHours > MAGIC_LINK_EXPIRY_HOURS) {
    return { valid: false, error: 'LINK_EXPIRED' };
  }

  // Check attempts
  if (session.attempts >= MAX_SIGNING_ATTEMPTS) {
    return { valid: false, error: 'TOO_MANY_ATTEMPTS' };
  }

  // Verify key (constant-time comparison)
  const expectedKey = await deriveSigningKey(sessionId, recipientId);
  if (!constantTimeEqual(key, expectedKey)) {
    await incrementAttempts(db, sessionId);
    return { valid: false, error: 'INVALID_KEY' };
  }

  return { valid: true };
}
```

### 10.7 Cloudflare WAF Rules (Pre-Deployment)

Configure in Cloudflare Dashboard before launch:

| Rule | Action | Description |
|------|--------|-------------|
| Block known bad bots | Block | Bot Fight Mode enabled |
| Challenge high threat scores | Challenge | Threat score > 10 |
| Rate limit /api/* | Block | >100 req/min per IP |
| Block non-browser User-Agents | Challenge | API endpoints |
| Geographic restrictions | Allow | US, CA, UK, EU only (initial) |
| Under Attack Mode | Auto | Enable during active attacks |

### 10.8 Monitoring & Alerting

```yaml
# CloudWatch Alarms (AWS)
alarms:
  - name: EmailProxyHighVolume
    metric: Invocations
    threshold: 1000
    period: 300  # 5 min
    action: SNS -> PagerDuty

  - name: SESBounceRate
    metric: Bounce
    threshold: 5%  # of sends
    action: SNS -> Slack + throttle

  - name: LambdaErrors
    metric: Errors
    threshold: 10
    period: 60
    action: SNS -> PagerDuty

# Cloudflare Analytics (Edge)
dashboards:
  - RPS by endpoint
  - 4xx/5xx rates
  - WAF blocked requests
  - Origin latency P50/P95/P99
```

### 10.10 Session Token Security

Borrowed from m3-getsigsmvp, token security prevents unauthorized session access and forgery attacks.

#### Token Generation

```typescript
// HMAC-SHA256 signing for session tokens
import { createHmac } from 'crypto';

const HMAC_SIGNING_SECRET = process.env.HMAC_SIGNING_SECRET || 'dev-secret';

interface SessionToken {
  sessionId: string;    // UUID
  recipientId: string;  // UUID
  timestamp: number;    // Creation time (Unix ms)
  signature: string;    // HMAC-SHA256(payload)
}

function generateSessionToken(
  sessionId: string,
  recipientId: string
): SessionToken {
  const timestamp = Date.now();
  const payload = `${sessionId}|${recipientId}|${timestamp}`;

  const signature = createHmac('sha256', HMAC_SIGNING_SECRET)
    .update(payload)
    .digest('hex');

  return {
    sessionId,
    recipientId,
    timestamp,
    signature,
  };
}

function verifySessionToken(token: SessionToken): boolean {
  const payload = `${token.sessionId}|${token.recipientId}|${token.timestamp}`;

  const expectedSignature = createHmac('sha256', HMAC_SIGNING_SECRET)
    .update(payload)
    .digest('hex');

  // Constant-time comparison to prevent timing attacks
  return constantTimeEqual(token.signature, expectedSignature);
}
```

#### Per-Sender Session Limits

```typescript
// Maximum concurrent sessions per sender prevents abuse
const MAX_SESSIONS_PER_SENDER = 100;
const MAX_RECIPIENTS_PER_SESSION = 50;

interface SenderQuota {
  senderId: string;
  sessionCount: number;
  recipientCount: number;
  lastSessionTime: number;
}

async function checkSenderQuota(
  db: D1Database,
  senderId: string
): Promise<{ allowed: boolean; message?: string }> {
  const quota = await db.prepare(`
    SELECT sessionCount, recipientCount, lastSessionTime
    FROM sender_quotas
    WHERE senderId = ?
  `).bind(senderId).first<SenderQuota>();

  if (!quota) {
    return { allowed: true };
  }

  if (quota.sessionCount >= MAX_SESSIONS_PER_SENDER) {
    return {
      allowed: false,
      message: `Exceeded maximum active sessions (${MAX_SESSIONS_PER_SENDER}). Complete or delete a session to create a new one.`
    };
  }

  return { allowed: true };
}

async function createSenderSession(
  db: D1Database,
  senderId: string,
  recipientCount: number
): Promise<{ allowed: boolean; error?: string }> {
  const quota = await checkSenderQuota(db, senderId);
  if (!quota.allowed) {
    return { allowed: false, error: quota.message };
  }

  if (recipientCount > MAX_RECIPIENTS_PER_SESSION) {
    return {
      allowed: false,
      error: `Too many recipients (${recipientCount}). Maximum is ${MAX_RECIPIENTS_PER_SESSION}.`
    };
  }

  // Increment session counter
  await db.prepare(`
    INSERT INTO sender_quotas (senderId, sessionCount, recipientCount, lastSessionTime)
    VALUES (?, 1, ?, ?)
    ON CONFLICT(senderId) DO UPDATE SET
      sessionCount = sessionCount + 1,
      recipientCount = recipientCount + ?,
      lastSessionTime = ?
  `).bind(senderId, recipientCount, Date.now(), recipientCount, Date.now()).run();

  return { allowed: true };
}
```

### 10.11 Input Validation

All user inputs must be validated before processing to prevent injection attacks, buffer overflows, and resource exhaustion.

#### File Size Limits

```rust
// Maximum sizes for various file operations
pub const MAX_PDF_SIZE: usize = 50 * 1024 * 1024;           // 50 MB
pub const MAX_SIGNATURE_SIZE: usize = 20 * 1024;             // 20 KB
pub const MAX_INITIALS_SIZE: usize = 10 * 1024;              // 10 KB
pub const MAX_METADATA_SIZE: usize = 100 * 1024;             // 100 KB
pub const MAX_FIELD_COUNT: usize = 1000;                     // 1000 fields per document
pub const MAX_FIELD_NAME_LEN: usize = 255;
pub const MAX_RECIPIENT_NAME_LEN: usize = 255;
pub const MAX_EMAIL_LEN: usize = 254;
pub const MAX_FILENAME_LEN: usize = 255;

pub fn validate_pdf_upload(bytes: &[u8]) -> Result<(), ValidationError> {
    if bytes.len() > MAX_PDF_SIZE {
        return Err(ValidationError::FileTooLarge {
            size: bytes.len(),
            max: MAX_PDF_SIZE,
        });
    }
    if bytes.len() == 0 {
        return Err(ValidationError::EmptyFile);
    }
    if !bytes.starts_with(b"%PDF") {
        return Err(ValidationError::InvalidPdfHeader);
    }
    Ok(())
}

pub fn validate_signature(bytes: &[u8]) -> Result<(), ValidationError> {
    if bytes.len() > MAX_SIGNATURE_SIZE {
        return Err(ValidationError::SignatureTooLarge {
            size: bytes.len(),
            max: MAX_SIGNATURE_SIZE,
        });
    }
    Ok(())
}
```

#### Session Parameter Validation

```typescript
// Validate session parameters on the server
interface SessionParams {
  sessionId: string;
  recipientId: string;
  key: string;
  senderId?: string;
}

function validateSessionParams(params: SessionParams): ValidationResult {
  const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

  if (!uuidPattern.test(params.sessionId)) {
    return { valid: false, error: 'Invalid sessionId format' };
  }

  if (!uuidPattern.test(params.recipientId)) {
    return { valid: false, error: 'Invalid recipientId format' };
  }

  if (!/^[a-f0-9]{64}$/.test(params.key)) {
    return { valid: false, error: 'Invalid signing key format' };
  }

  if (params.senderId && !uuidPattern.test(params.senderId)) {
    return { valid: false, error: 'Invalid senderId format' };
  }

  return { valid: true };
}

function validateEmail(email: string): ValidationResult {
  if (email.length > MAX_EMAIL_LEN) {
    return { valid: false, error: 'Email too long' };
  }

  // Simple email validation
  const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailPattern.test(email)) {
    return { valid: false, error: 'Invalid email format' };
  }

  return { valid: true };
}

function validateFieldName(name: string): ValidationResult {
  if (name.length === 0 || name.length > MAX_FIELD_NAME_LEN) {
    return { valid: false, error: 'Field name length invalid' };
  }

  // Allow alphanumeric, underscores, hyphens
  if (!/^[a-zA-Z0-9_\-]+$/.test(name)) {
    return { valid: false, error: 'Field name contains invalid characters' };
  }

  return { valid: true };
}
```

#### Filename Sanitization

```typescript
// Prevent path traversal and other filename attacks
function sanitizeFilename(filename: string): string {
  // Remove null bytes
  let sanitized = filename.replace(/\0/g, '');

  // Remove path traversal patterns
  sanitized = sanitized.replace(/\.\./g, '');
  sanitized = sanitized.replace(/[\/\\]/g, '_');

  // Remove control characters
  sanitized = sanitized.replace(/[\x00-\x1f]/g, '');

  // Limit length
  if (sanitized.length > MAX_FILENAME_LEN) {
    sanitized = sanitized.substring(0, MAX_FILENAME_LEN - 4) + '.pdf';
  }

  // Ensure .pdf extension
  if (!sanitized.toLowerCase().endsWith('.pdf')) {
    sanitized += '.pdf';
  }

  return sanitized;
}

function validateFilename(filename: string): ValidationResult {
  const original = filename;
  const sanitized = sanitizeFilename(filename);

  if (sanitized.length === 0) {
    return { valid: false, error: 'Filename is empty after sanitization' };
  }

  // Check for suspicious patterns
  if (sanitized.includes('..') || sanitized.includes('/') || sanitized.includes('\\')) {
    return { valid: false, error: 'Filename contains path traversal patterns' };
  }

  return { valid: true, sanitized };
}
```

#### XSS Prevention

```typescript
// Escape HTML content to prevent XSS attacks
function escapeHtml(text: string): string {
  const map: { [key: string]: string } = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#039;',
  };
  return text.replace(/[&<>"']/g, (char) => map[char]);
}

// Sanitize user-provided HTML (email templates, notifications)
function sanitizeHtmlContent(html: string): string {
  // Remove script tags and event handlers
  let sanitized = html.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '');
  sanitized = sanitized.replace(/on\w+\s*=\s*"[^"]*"/gi, '');
  sanitized = sanitized.replace(/on\w+\s*=\s*'[^']*'/gi, '');

  return sanitized;
}

// Render recipient names safely
function renderRecipientName(name: string): string {
  return escapeHtml(name);
}

// Render email safely in templates
function renderEmail(email: string): string {
  return escapeHtml(email);
}
```

### 10.12 Client-Side Security

Client-side UI must provide clear feedback on rate limiting and resource usage to prevent poor user experience.

#### Rate Limit Visibility

```typescript
// Show rate limit headers from server
interface RateLimitInfo {
  remaining: number;
  limit: number;
  resetTime: number;  // Unix seconds
  threshold: 'warning' | 'critical' | 'normal';
}

function getRateLimitInfo(response: Response): RateLimitInfo | null {
  const remaining = parseInt(response.headers.get('X-RateLimit-Remaining') || '-1');
  const limit = parseInt(response.headers.get('X-RateLimit-Limit') || '-1');
  const reset = parseInt(response.headers.get('X-RateLimit-Reset') || '0');

  if (remaining === -1 || limit === -1) {
    return null;
  }

  const ratio = remaining / limit;
  let threshold: 'warning' | 'critical' | 'normal' = 'normal';

  if (ratio < 0.1) threshold = 'critical';
  else if (ratio < 0.3) threshold = 'warning';

  return { remaining, limit, resetTime: reset, threshold };
}

function displayRateLimitWarning(info: RateLimitInfo): void {
  const message = `${info.remaining} of ${info.limit} requests remaining`;

  if (info.threshold === 'critical') {
    console.warn(`CRITICAL: ${message}`);
    showUIWarning('You are near your request limit. Please wait before trying again.', 'critical');
  } else if (info.threshold === 'warning') {
    console.warn(`WARNING: ${message}`);
    showUIWarning(`You have used ${info.limit - info.remaining} of ${info.limit} requests.`, 'warning');
  }
}
```

#### Warning Thresholds

```typescript
// User feedback thresholds for rate limiting
const RATE_LIMIT_THRESHOLDS = {
  // Daily email sending limits
  emailDaily: {
    warning: 10,     // Show warning at 10 emails/day
    critical: 3,     // Show critical at 3 remaining
    max: 200,
  },
  // Monthly sending limits
  emailMonthly: {
    warning: 100,    // Show warning at 100 emails/month
    critical: 20,    // Show critical at 20 remaining
    max: 2000,
  },
  // API request limits
  apiDaily: {
    warning: 100,
    critical: 10,
    max: 1000,
  },
};

function shouldShowWarning(
  current: number,
  threshold: 'warning' | 'critical'
): boolean {
  return current <= RATE_LIMIT_THRESHOLDS[threshold];
}

function getWarningMessage(
  endpoint: string,
  remaining: number,
  threshold: 'warning' | 'critical'
): string {
  if (threshold === 'critical') {
    return `⚠️ CRITICAL: You have only ${remaining} requests remaining today.`;
  } else {
    return `⚠️ You have ${remaining} requests remaining today.`;
  }
}
```

#### User Feedback UI

```typescript
// Display rate limit feedback in UI
class RateLimitFeedback {
  private warningElement: HTMLElement | null = null;

  constructor() {
    this.createWarningElement();
  }

  private createWarningElement(): void {
    const div = document.createElement('div');
    div.id = 'rate-limit-warning';
    div.className = 'rate-limit-warning hidden';
    div.style.cssText = `
      position: fixed;
      bottom: 20px;
      right: 20px;
      padding: 12px 16px;
      border-radius: 4px;
      font-size: 14px;
      font-weight: 600;
      max-width: 300px;
      z-index: 9999;
    `;
    document.body.appendChild(div);
    this.warningElement = div;
  }

  showWarning(message: string, severity: 'warning' | 'critical' = 'warning'): void {
    if (!this.warningElement) return;

    this.warningElement.textContent = message;
    this.warningElement.className = `rate-limit-warning ${severity}`;

    if (severity === 'critical') {
      this.warningElement.style.backgroundColor = '#fee';
      this.warningElement.style.color = '#a00';
      this.warningElement.style.border = '1px solid #fcc';
    } else {
      this.warningElement.style.backgroundColor = '#ffc';
      this.warningElement.style.color = '#880';
      this.warningElement.style.border = '1px solid #ff9';
    }
  }

  hideWarning(): void {
    if (this.warningElement) {
      this.warningElement.className = 'rate-limit-warning hidden';
    }
  }
}
```

### 10.9 Pre-Deployment Checklist

#### Core Security Infrastructure
- [ ] Cloudflare WAF rules configured
- [ ] Rate limit D1 table created
- [ ] DynamoDB sender tracking table created
- [ ] CloudWatch alarms configured
- [ ] Bot Fight Mode enabled
- [ ] Under Attack Mode tested

#### Session Token Security
- [ ] HMAC signing secret configured (HMAC_SIGNING_SECRET env var)
- [ ] Per-IP rate limit KV namespace created
- [ ] Session token generation verified
- [ ] Per-sender session limits enforced (max 100 sessions/sender)
- [ ] Session quota database schema created
- [ ] Token verification with timing attack protection

#### Input Validation
- [ ] File size limits enforced in WASM (MAX_PDF_SIZE = 50MB)
- [ ] File header validation (PDF %PDF signature check)
- [ ] Session parameter validation implemented
- [ ] Email format validation on all recipient inputs
- [ ] Field name validation (alphanumeric + underscore/hyphen only)
- [ ] Filename sanitization prevents path traversal

#### Email & PDF Protections
- [ ] Magic link expiry implemented (72 hours)
- [ ] Email dispatch limits in Lambda (50/hour, 200/day per sender)
- [ ] PDF processing timeouts configured (30s timeout)
- [ ] Stream validation prevents zip bombs (MAX_STREAM_SIZE = 10MB)
- [ ] Brute force protection on magic links (MAX_SIGNING_ATTEMPTS = 5)

#### XSS & Content Security
- [ ] XSS prevention via HTML escaping on all user inputs
- [ ] Script tag removal in email templates
- [ ] Event handler stripping (onload, onclick, etc.)
- [ ] Recipient names escaped in all outputs
- [ ] Email addresses escaped in all templates
- [ ] Content-Security-Policy headers configured

#### Client-Side Security
- [ ] Rate limit visibility implemented (X-RateLimit-* headers)
- [ ] Warning thresholds configured (daily < 10, monthly < 100)
- [ ] Critical thresholds configured (daily < 3, monthly < 20)
- [ ] UI feedback for rate limit warnings
- [ ] Color-coded severity indicators (warning yellow, critical red)
- [ ] Rate limit status displayed prominently before actions

#### Testing & Validation
- [ ] Security test suite passes (rate limiting, validation, XSS)
- [ ] CAPTCHA integration tested on rate limit exceeded
- [ ] Magic link brute force protection verified
- [ ] Filename sanitization tested with path traversal attempts
- [ ] HTML escaping verified with special character payloads
- [ ] PDF bomb detection tested with malicious files

---

## References

### Florida Statutes
- [Chapter 83 - Landlord and Tenant](https://www.flsenate.gov/Laws/Statutes/2025/Chapter83)
- [Chapter 475 - Real Estate Brokers](https://www.flsenate.gov/Laws/Statutes/2025/Chapter475)
- [Chapter 689 - Conveyances](https://www.flsenate.gov/Laws/Statutes/2025/Chapter689)
- [Chapter 713 - Construction Liens](https://www.flsenate.gov/Laws/Statutes/2025/Chapter713)
- [Chapter 319 - Motor Vehicle Titles](https://www.flsenate.gov/Laws/Statutes/2025/Chapter319)

### Related Documents
- `DOCSIGN_PLAN.md` (in m3-getsigsmvp) - Signing platform details
- `crates/email-proxy/README.md` - AWS SES Lambda docs
- `FL_LEASE.md`, `FL_PURCHASE.md` - Florida law research
