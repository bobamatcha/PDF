# DOCSIGN_PLAN: Geriatric-Friendly Document Signing Platform

> **Version:** 1.1 | **Target:** Late 2025 / Early 2026
> **Related Plans:** [PLAN0.md](./PLAN0.md), [UX_IMPROVEMENT_PLAN.md](./UX_IMPROVEMENT_PLAN.md)
> **Development Guidelines:** See [CLAUDE.md](./CLAUDE.md) for test-first development practices.

---

## Progress Log

| Date | Phase | Milestone | Details |
|------|-------|-----------|---------|
| 2025-12-30 | Phase 0 | ✅ Foundation Complete | TypeScript build, PDF preview, property tests, geriatric CSS |

### Current Status: **Phase 0 Complete → Ready for Phase 1**

**What's Done:**
- ✅ TypeScript + esbuild build infrastructure (`npm run build` → 7.5KB bundle)
- ✅ PDF preview components copied from pdfjoin-web (pdf-loader, pdf-preview, coord-utils)
- ✅ 25 property-based tests (15 coordinate, 10 signing verification)
- ✅ Geriatric UX CSS foundation (60px targets, 18px fonts, AAA contrast)

**Next Step:** Phase 1 - Geriatric UX Overhaul (integrate geriatric.css into sign.html)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Vision: The Geriatric-Friendly DocuSign Competitor](#vision-the-geriatric-friendly-docusign-competitor)
3. [Architecture Critique & Recommendations](#architecture-critique--recommendations)
4. [Local-First Architecture Specification](#local-first-architecture-specification)
5. [Reusable Components from pdfjoin-web](#reusable-components-from-pdfjoin-web)
6. [Geriatric UX Design Principles](#geriatric-ux-design-principles)
7. [Tauri Desktop Application](#tauri-desktop-application)
8. [Security & Legal Considerations](#security--legal-considerations)
9. [Implementation Phases](#implementation-phases)
10. [Testing Strategy](#testing-strategy)
11. [Appendices](#appendices)

---

## Executive Summary

**DOCSIGN_PLAN** defines the comprehensive strategy for building a geriatric-friendly document signing platform that prioritizes:

1. **Local-First Architecture** — All document generation, signing, and sensitive data processing occurs on the user's device
2. **Preview-Only PDF Rendering** — No in-browser editing; users confirm documents before signing
3. **Accessibility First** — Design for users 65+ with visual impairments, reduced dexterity, and technology anxiety
4. **Legal Correctness** — PAdES-compliant signatures, audit trails, and timestamping
5. **Offline Resilience** — Full functionality without internet (sync when available)

### Core Deliverables

| Component | Description | Target |
|-----------|-------------|--------|
| **docsign-web** | Slim web app for preview + signing | Q4 2025 |
| **docsign-wasm** | WASM module for local signing | Q4 2025 |
| **docsign-tauri** | Desktop app with native capabilities | Q1 2026 |
| **docsign-worker** | Optional backend for sync/identity | Q4 2025 |

---

## Vision: The Geriatric-Friendly DocuSign Competitor

### The Problem

DocuSign and competitors are designed for tech-savvy professionals:
- Complex multi-step workflows confuse non-technical users
- Small touch targets frustrate users with reduced dexterity
- Time-limited sessions expire during slow navigation
- "Click to sign" isn't legally clear to users who expect pen-on-paper

### Our Solution

A signing experience designed for users who:
- Are 65+ years old
- May have vision impairments (cataracts, macular degeneration)
- Have reduced fine motor control
- Distrust "the cloud" with sensitive documents
- Expect physical metaphors (sign, stamp, mail)

### Key Differentiators

| DocuSign | GetSignatures (DOCSIGN_PLAN) |
|----------|------------------------------|
| Cloud-first, documents stored on servers | Local-first, documents never leave device |
| Small signature boxes | Large, forgiving touch targets (60px+) |
| Multi-step configuration | One-page guided flow |
| Session expires in 48 hours | No artificial time limits |
| Signature = click | Signature = deliberate drawing + confirmation |
| Mobile as afterthought | Mobile-first, desktop-enhanced |

---

## Architecture Critique & Recommendations

### Current docsign-web Architecture

The existing implementation (as explored) has strong foundations:

**Strengths:**
- ✅ Client-side cryptographic signing (WASM)
- ✅ P-256 ECDSA + PAdES-compliant signatures
- ✅ Tamper-evident audit chain
- ✅ IndexedDB for local persistence
- ✅ Offline queue with background sync
- ✅ RFC 3161 timestamp authority support
- ✅ Mobile-responsive design (UX-005)

**Weaknesses Requiring Attention:**

| Issue | Impact | Recommendation | Status |
|-------|--------|----------------|--------|
| PDF.js loaded in main bundle | Slow initial load | Adopt pdfjoin-web lazy loading pattern | ✅ FIXED (pdf-loader.ts) |
| Coordinate transforms duplicated | Maintenance burden | Share coord-utils.ts from pdfjoin-web | ✅ FIXED (coord-utils.ts) |
| No TypeScript, hard to maintain | Bugs, no type safety | Migrate to TypeScript + esbuild | ✅ FIXED (7.5KB bundle) |
| No geriatric UX | Bad for 65+ users | 60px targets, 18px fonts, AAA contrast | ✅ FIXED (geriatric.css) |
| Session tied to Cloudflare Worker | Server dependency | Implement pure-local session fallback | TODO (Phase 2) |
| No offline document generation | Requires network for templates | Bundle critical templates in WASM | TODO (Phase 2) |
| sign.html is 37KB of inlined JS | Hard to maintain | Migrate JS to TypeScript modules | TODO (Phase 1) |
| Certificate import requires copy-paste | Poor UX | Add file picker + QR code options | TODO (Phase 3) |

### Recommended Architecture Split

```
┌─────────────────────────────────────────────────────────────────────┐
│                         USER DEVICE                                 │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  WEB FRONTEND (docsign-web)                                 │   │
│  │  - HTML/CSS for UI shell                                    │   │
│  │  - TypeScript for interaction logic                         │   │
│  │  - PDF.js for preview rendering (lazy loaded)               │   │
│  │  - Accessibility features (ARIA, focus management)          │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                              │                                      │
│  ┌──────────────────────────▼──────────────────────────────────┐   │
│  │  LOCAL EXECUTION LAYER (docsign-wasm)                       │   │
│  │  - PDF parsing and validation                               │   │
│  │  - Signature field detection                                │   │
│  │  - Cryptographic signing (P-256, PAdES)                     │   │
│  │  - Audit chain generation                                   │   │
│  │  - Certificate management                                   │   │
│  │  - Template rendering (Typst engine)                        │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                              │                                      │
│  ┌──────────────────────────▼──────────────────────────────────┐   │
│  │  LOCAL STORAGE (IndexedDB)                                  │   │
│  │  - Session state                                            │   │
│  │  - Pending signatures queue                                 │   │
│  │  - Offline document cache                                   │   │
│  │  - User preferences                                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  TAURI DESKTOP (docsign-tauri) — OPTIONAL                   │   │
│  │  - System fonts access                                      │   │
│  │  - Native PDF viewer / printer                              │   │
│  │  - Hardware security module integration                     │   │
│  │  - File system access for bulk operations                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ OPTIONAL (sync only)
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    OPTIONAL BACKEND SERVICES                        │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐  ┌─────────────────────┐                  │
│  │ Email Relay         │  │ Identity Service    │                  │
│  │ (Cloudflare Worker) │  │ (OAuth, Magic Link) │                  │
│  └─────────────────────┘  └─────────────────────┘                  │
│  ┌─────────────────────┐  ┌─────────────────────┐                  │
│  │ Sync Service        │  │ Audit Log Archive   │                  │
│  │ (Encrypted backup)  │  │ (Compliance copy)   │                  │
│  └─────────────────────┘  └─────────────────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
```

### Critical Design Decisions

#### DOCSIGN_PLAN Decision 1: Preview-Only PDF Rendering

**Rationale:** In-browser PDF editing adds complexity and legal risk. Users should not accidentally modify documents before signing.

**Implementation:**
- PDF.js renders pages to `<canvas>` elements (read-only)
- No annotation tools, text editing, or drawing on the PDF itself
- Signature fields are overlaid on top of the canvas (not embedded until signing)
- "What you see is what gets signed" — preview exactly matches signed output

#### DOCSIGN_PLAN Decision 2: Local-First, Server-Optional

**Rationale:** Older users distrust cloud services with sensitive documents. Local processing also eliminates server costs and latency.

**Core workflow must function with:**
- No internet connection
- No backend services
- No account registration

**Server-dependent features (premium):**
- Multi-party signing coordination
- Email delivery of signing invitations
- Cloud backup of audit logs
- Device sync

#### DOCSIGN_PLAN Decision 3: Accessibility as Foundation

**Rationale:** Accessibility isn't a feature — it's the architecture. Design for screen readers, keyboard navigation, and low vision from day one.

**Non-negotiable standards:**
- WCAG 2.1 AA compliance minimum
- Touch targets: 60px × 60px minimum (not 44px)
- Font sizes: 18px base, 24px for actions
- Color contrast: 7:1 ratio (AAA level)
- No time-limited actions
- All features keyboard-accessible

---

## Local-First Architecture Specification

### Core Principle

> Every operation that doesn't inherently require a network (like sending email) must work offline.

### Data Flow (No Server)

```
User uploads PDF
       │
       ▼
┌──────────────────────┐
│ PDF.js validates     │
│ & renders preview    │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ WASM detects         │
│ signature fields     │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ User draws signature │
│ on canvas overlay    │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ WASM injects         │
│ PAdES signature      │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Signed PDF saved     │
│ to user's device     │
└──────────────────────┘
```

### Offline-First Session Management

Replace server-dependent session lookups with local-first approach:

```typescript
// Current: Requires server
const session = await fetch(`/api/session/${sessionId}`);

// DOCSIGN_PLAN: Local-first with optional sync
const session = await localSession.get(sessionId)
  ?? await tryRemoteSession(sessionId);

class LocalSessionManager {
  private db: IDBDatabase;

  async createSession(document: Uint8Array, recipients: Recipient[]): Promise<Session> {
    const session: Session = {
      id: crypto.randomUUID(),
      documentHash: await sha256(document),
      documentEncrypted: await encryptDocument(document),
      recipients,
      status: 'pending',
      createdAt: new Date().toISOString(),
      expiresAt: null, // No expiration for local sessions
      auditChain: new AuditChain(),
    };

    await this.db.put('sessions', session);
    return session;
  }

  async signDocument(sessionId: string, signature: SignatureData): Promise<SignedDocument> {
    const session = await this.db.get('sessions', sessionId);
    const document = await decryptDocument(session.documentEncrypted);

    // All signing happens locally in WASM
    const signedPdf = await wasmSign(document, signature);

    session.auditChain.append({
      action: 'Sign',
      actor: signature.signerEmail,
      documentHash: await sha256(signedPdf),
      timestamp: new Date().toISOString(),
    });

    await this.db.put('sessions', session);
    return signedPdf;
  }
}
```

### Template Bundling for Offline Generation

Bundle essential templates in the WASM module:

```rust
// In docsign-wasm/src/templates.rs
const BUNDLED_TEMPLATES: &[(&str, &[u8])] = &[
    ("florida_lease", include_bytes!("../templates/florida_lease.typ")),
    ("florida_purchase", include_bytes!("../templates/florida_purchase.typ")),
    ("generic_contract", include_bytes!("../templates/generic_contract.typ")),
];

#[wasm_bindgen]
pub fn render_template_offline(template_name: &str, data_json: &str) -> Result<Vec<u8>, JsValue> {
    let template = BUNDLED_TEMPLATES
        .iter()
        .find(|(name, _)| *name == template_name)
        .ok_or_else(|| JsValue::from_str("Template not found"))?;

    typst_engine::render_sync(template.1, data_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

---

## Reusable Components from pdfjoin-web

### Components to Reuse Directly

Based on the pdfjoin-web exploration, these components are battle-tested and should be copied to docsign-web:

| Component | Source | Purpose | Reusability |
|-----------|--------|---------|-------------|
| `pdf-loader.ts` | `pdfjoin-web/src/ts/pdf-loader.ts` | Lazy-load PDF.js | 100% |
| `pdf-bridge.ts` | `pdfjoin-web/src/ts/pdf-bridge.ts` | Render PDF pages | 95% (remove edit operations) |
| `coord-utils.ts` | `pdfjoin-web/src/ts/coord-utils.ts` | Coordinate transforms | 100% |
| `pdf-types.ts` | `pdfjoin-web/src/ts/types/pdf-types.ts` | TypeScript definitions | 100% |

### PDF.js Lazy Loading (Copy Exactly)

```typescript
// From pdfjoin-web/src/ts/pdf-loader.ts
let pdfJsLoaded = false;
let pdfJsLoadPromise: Promise<void> | null = null;

export async function ensurePdfJsLoaded(): Promise<void> {
    if (pdfJsLoaded) return;
    if (pdfJsLoadPromise) return pdfJsLoadPromise;

    pdfJsLoadPromise = new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = '/js/vendor/pdf.min.js';
        script.onload = () => {
            (window as any).pdfjsLib.GlobalWorkerOptions.workerSrc = '/js/vendor/pdf.worker.min.js';
            pdfJsLoaded = true;
            resolve();
        };
        script.onerror = () => reject(new Error('Failed to load PDF.js'));
        document.head.appendChild(script);
    });

    return pdfJsLoadPromise;
}
```

### PDF Bridge for Preview-Only (Simplified)

```typescript
// Adapted from pdfjoin-web/src/ts/pdf-bridge.ts
// Removed: all edit/annotation functionality
// Kept: loading, rendering, text extraction

export interface PreviewBridge {
    loadDocument(data: Uint8Array | ArrayBuffer): Promise<number>;
    renderPage(pageNum: number, canvas: HTMLCanvasElement, scale?: number): Promise<PageDimensions>;
    getPageDimensions(pageNum: number): PageDimensions | null;
    extractTextWithPositions(pageNum: number): Promise<TextItem[]>;
    cleanup(): void;
}

class PdfPreviewBridge implements PreviewBridge {
    private currentDoc: PDFDocumentProxy | null = null;
    private pageCache: Map<number, CachedPageInfo> = new Map();

    async loadDocument(data: Uint8Array | ArrayBuffer): Promise<number> {
        await ensurePdfJsLoaded();
        const typedArray = new Uint8Array(data);
        this.currentDoc = await (window as any).pdfjsLib.getDocument(typedArray).promise;
        return this.currentDoc.numPages;
    }

    async renderPage(pageNum: number, canvas: HTMLCanvasElement, scale = 1.5): Promise<PageDimensions> {
        if (!this.currentDoc) throw new Error('No document loaded');

        const page = await this.currentDoc.getPage(pageNum);
        const viewport = page.getViewport({ scale });

        canvas.width = viewport.width;
        canvas.height = viewport.height;

        const ctx = canvas.getContext('2d')!;
        await page.render({ canvasContext: ctx, viewport }).promise;

        this.pageCache.set(pageNum, { canvas, viewport, page });

        return {
            width: viewport.width,
            height: viewport.height,
            pdfWidth: page.view[2],
            pdfHeight: page.view[3],
        };
    }

    async extractTextWithPositions(pageNum: number): Promise<TextItem[]> {
        if (!this.currentDoc) throw new Error('No document loaded');

        const page = await this.currentDoc.getPage(pageNum);
        const textContent = await page.getTextContent();
        const viewport = page.getViewport({ scale: 1.0 });

        return textContent.items.map((item: any, index: number) => {
            const transform = item.transform;
            return {
                index,
                str: item.str,
                pdfX: transform[4],
                pdfY: transform[5],
                pdfWidth: item.width,
                pdfHeight: item.height,
                fontSize: Math.abs(transform[0]),
            };
        });
    }

    cleanup(): void {
        if (this.currentDoc) {
            this.currentDoc.destroy();
            this.currentDoc = null;
        }
        this.pageCache.clear();
    }
}

export const previewBridge = new PdfPreviewBridge();
```

### Coordinate Utilities (Copy Exactly)

```typescript
// From pdfjoin-web/src/ts/coord-utils.ts
export function pdfPointToDom(
    viewport: PDFJSViewport,
    pdfX: number,
    pdfY: number
): [number, number] {
    return viewport.convertToViewportPoint(pdfX, pdfY);
}

export function domPointToPdf(
    viewport: PDFJSViewport,
    domX: number,
    domY: number
): [number, number] {
    return viewport.convertToPdfPoint(domX, domY);
}

export function pdfRectToDom(
    viewport: PDFJSViewport,
    pdfX: number, pdfY: number, pdfWidth: number, pdfHeight: number
): { x: number; y: number; width: number; height: number } {
    const [x1, y1] = pdfPointToDom(viewport, pdfX, pdfY);
    const [x2, y2] = pdfPointToDom(viewport, pdfX + pdfWidth, pdfY + pdfHeight);
    return {
        x: Math.min(x1, x2),
        y: Math.min(y1, y2),
        width: Math.abs(x2 - x1),
        height: Math.abs(y2 - y1),
    };
}

export function domRectToPdf(
    viewport: PDFJSViewport,
    domX: number, domY: number, domWidth: number, domHeight: number
): { x: number; y: number; width: number; height: number } {
    const [x1, y1] = domPointToPdf(viewport, domX, domY);
    const [x2, y2] = domPointToPdf(viewport, domX + domWidth, domY + domHeight);
    return {
        x: Math.min(x1, x2),
        y: Math.min(y1, y2),
        width: Math.abs(x2 - x1),
        height: Math.abs(y2 - y1),
    };
}
```

---

## Geriatric UX Design Principles

### The Foundational Rule

> **"If they have to ask for help, the UI is broken."**

Every interface element must be self-explanatory to a 75-year-old using technology for the first time.

### Visual Design Standards

#### Typography

```css
:root {
  /* Base sizes - larger than typical */
  --font-size-base: 18px;
  --font-size-lg: 22px;
  --font-size-xl: 28px;
  --font-size-action: 24px;

  /* Line heights for readability */
  --line-height-body: 1.6;
  --line-height-heading: 1.3;

  /* Fonts - high x-height, clear letterforms */
  --font-family-body: 'Atkinson Hyperlegible', -apple-system, sans-serif;
  --font-family-mono: 'JetBrains Mono', monospace;
}

body {
  font-size: var(--font-size-base);
  line-height: var(--line-height-body);
  font-family: var(--font-family-body);
}

button, .action {
  font-size: var(--font-size-action);
  font-weight: 600;
}
```

#### Color Contrast

```css
:root {
  /* AAA contrast (7:1 minimum) */
  --color-text-primary: #1a1a1a;
  --color-bg-primary: #ffffff;

  /* High-visibility actions */
  --color-action-bg: #0056b3;
  --color-action-text: #ffffff;
  --color-action-bg-hover: #003d82;

  /* Status colors - distinct, not just hue */
  --color-success: #006644;
  --color-success-bg: #e6f4ed;
  --color-error: #b30000;
  --color-error-bg: #fce8e8;
  --color-warning: #8a5700;
  --color-warning-bg: #fef3cd;
}
```

#### Touch Targets

```css
/* Minimum 60x60px for all interactive elements */
button,
a,
input[type="checkbox"],
input[type="radio"],
.interactive {
  min-width: 60px;
  min-height: 60px;
  padding: 16px 24px;
}

/* Generous spacing between targets */
.button-group {
  gap: 24px;
}

/* Large checkboxes */
input[type="checkbox"] {
  width: 32px;
  height: 32px;
  accent-color: var(--color-action-bg);
}
```

### Interaction Patterns

#### No Hidden Actions

```html
<!-- BAD: Hidden action revealed on hover -->
<div class="document">
  <span class="doc-name">Contract.pdf</span>
  <button class="delete-btn" style="opacity: 0">Delete</button>
</div>

<!-- GOOD: All actions visible by default -->
<div class="document">
  <span class="doc-name">Contract.pdf</span>
  <div class="actions">
    <button class="view-btn">View</button>
    <button class="sign-btn">Sign</button>
    <button class="delete-btn">Remove</button>
  </div>
</div>
```

#### Confirmation Before Destructive Actions

```javascript
// Always confirm before deletion, no matter how "obvious"
async function deleteDocument(docId: string): Promise<void> {
  const confirmed = await showConfirmDialog({
    title: "Remove This Document?",
    message: "This will remove the document from your list. The original file on your computer will not be affected.",
    confirmText: "Yes, Remove It",
    cancelText: "No, Keep It",
    icon: "warning",
  });

  if (confirmed) {
    await documentStore.delete(docId);
    showSuccessMessage("Document removed from your list.");
  }
}
```

#### Progress Indicators for Everything

```html
<!-- Show progress for any operation that takes >200ms -->
<div id="signing-progress" class="progress-overlay">
  <div class="progress-content">
    <div class="spinner" aria-hidden="true"></div>
    <h2>Signing Your Document</h2>
    <p>This may take a few seconds. Please don't close this window.</p>
    <div class="progress-bar" role="progressbar" aria-valuenow="45" aria-valuemin="0" aria-valuemax="100">
      <div class="progress-fill" style="width: 45%"></div>
    </div>
    <p class="progress-step">Adding your signature... (Step 2 of 4)</p>
  </div>
</div>
```

### Signature Capture UX

#### Clear Instructions

```html
<div class="signature-capture">
  <h2>Draw Your Signature</h2>
  <p class="instructions">
    Use your finger or mouse to sign in the box below.
    <strong>Take your time</strong> — there's no rush.
  </p>

  <div class="signature-pad-container">
    <canvas id="signature-pad" aria-label="Signature drawing area"></canvas>

    <div class="signature-actions">
      <button id="clear-signature" class="secondary-btn">
        ✕ Start Over
      </button>
      <button id="undo-stroke" class="secondary-btn">
        ↶ Undo Last Stroke
      </button>
    </div>
  </div>

  <div class="signature-options">
    <label class="option">
      <input type="radio" name="sig-type" value="draw" checked>
      <span>Draw my signature</span>
    </label>
    <label class="option">
      <input type="radio" name="sig-type" value="type">
      <span>Type my name in cursive</span>
    </label>
  </div>

  <button id="accept-signature" class="primary-btn large">
    ✓ Use This Signature
  </button>
</div>
```

#### Typed Signature with Preview

```javascript
// For users who can't draw well
function createTypedSignature(name: string): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = 400;
  canvas.height = 120;

  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  // Use a script font that looks handwritten
  ctx.font = '48px "Dancing Script", cursive';
  ctx.fillStyle = '#000080'; // Navy blue for signature
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(name, canvas.width / 2, canvas.height / 2);

  return canvas;
}
```

### Error Handling

#### Friendly Error Messages

```typescript
// Never show raw error messages
function getUserFriendlyError(error: Error): UserError {
  const msg = error.message.toLowerCase();

  if (msg.includes('network') || msg.includes('fetch')) {
    return {
      title: "Connection Problem",
      message: "We couldn't connect to the internet. Your document is safe — you can try again when you're back online.",
      action: "Try Again",
      icon: "wifi-off",
    };
  }

  if (msg.includes('password') || msg.includes('encrypted')) {
    return {
      title: "This PDF is Password-Protected",
      message: "Please ask the sender for an unprotected version of this document, or enter the password if you know it.",
      action: "Enter Password",
      icon: "lock",
    };
  }

  if (msg.includes('signature') && msg.includes('invalid')) {
    return {
      title: "Signature Problem",
      message: "We had trouble adding your signature. Please try drawing it again — make sure to lift your finger between strokes.",
      action: "Try Again",
      icon: "signature",
    };
  }

  // Generic fallback
  return {
    title: "Something Went Wrong",
    message: "We ran into an unexpected problem. Your document is safe. If this keeps happening, please contact support.",
    action: "Go Back",
    icon: "alert",
  };
}
```

### Navigation

#### Breadcrumb Trail

```html
<!-- Always show where the user is -->
<nav class="breadcrumb" aria-label="You are here">
  <ol>
    <li><a href="/">Home</a></li>
    <li><a href="/documents">My Documents</a></li>
    <li aria-current="page">Sign: Lease Agreement.pdf</li>
  </ol>
</nav>
```

#### Persistent Exit Path

```html
<!-- Always provide a way out -->
<header class="app-header">
  <a href="/" class="home-link" aria-label="Return to home page">
    <img src="/logo.svg" alt="GetSignatures">
  </a>

  <nav class="user-nav">
    <a href="/help" class="help-link">Need Help?</a>
    <button class="exit-btn" onclick="confirmExit()">
      ✕ Exit
    </button>
  </nav>
</header>
```

---

## Tauri Desktop Application

### Research Justification

#### Why Desktop? (Business Case)

1. **Elderly user preference for "installed software"**
   - Older users trust applications more than websites
   - "An app on my computer" feels more permanent and reliable
   - No URL to remember or type

2. **Offline-first without compromise**
   - Web apps have storage limits (IndexedDB ~50-100MB)
   - Desktop app can use unlimited local storage
   - No service worker complexity

3. **Hardware access for enhanced security**
   - USB security keys (YubiKey, etc.)
   - Smart card readers for government IDs
   - Hardware security modules (HSM)
   - Local certificate stores

4. **Native printing**
   - Direct printer access without "Save as PDF" workaround
   - Print preview with actual system fonts
   - Batch printing support

5. **File system integration**
   - Drag-and-drop from file explorer
   - "Recent Documents" in system menu
   - File associations (.sig files)

#### Why Tauri? (Technical Case)

| Criteria | Electron | Tauri | Native (Swift/C#) |
|----------|----------|-------|-------------------|
| Bundle size | ~150MB | ~5-15MB | ~5MB |
| Memory usage | High (Chromium) | Low (system WebView) | Low |
| Code reuse from web | 100% | 95%+ | 0% |
| Rust integration | FFI required | Native | FFI required |
| Cross-platform | Yes | Yes | No |
| Security | Moderate | Strong (no Node.js) | Strong |
| Development speed | Fast | Fast | Slow |

**Tauri Advantages for DOCSIGN_PLAN:**

1. **Shares WASM core** — The docsign-wasm module works identically in both web and Tauri
2. **Rust backend** — Natural fit for our existing Rust crates
3. **Minimal footprint** — 10MB installer vs 150MB for Electron
4. **Security sandboxing** — IPC-based permission model

### Tauri Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      TAURI APPLICATION                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  WEBVIEW (System WebView2/WebKit)                           │   │
│  │  ┌─────────────────────────────────────────────────────┐    │   │
│  │  │  Same HTML/CSS/JS as docsign-web                    │    │   │
│  │  │  - PDF preview via PDF.js                           │    │   │
│  │  │  - Signature capture                                │    │   │
│  │  │  - Accessibility features                           │    │   │
│  │  └─────────────────────────────────────────────────────┘    │   │
│  │                          │                                   │   │
│  │                          │ Tauri IPC                         │   │
│  │                          ▼                                   │   │
│  │  ┌─────────────────────────────────────────────────────┐    │   │
│  │  │  WASM Module (loaded in WebView)                    │    │   │
│  │  │  - docsign-wasm (same as web)                       │    │   │
│  │  │  - PDF parsing, signing, audit                      │    │   │
│  │  └─────────────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│                              │ Tauri Commands                       │
│                              ▼                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  RUST BACKEND (Tauri Core)                                  │   │
│  │  - Native file system access                                │   │
│  │  - System certificate store                                 │   │
│  │  - Printer integration                                      │   │
│  │  - Hardware security module bridge                          │   │
│  │  - Auto-updates                                             │   │
│  │  - System tray integration                                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Feature Matrix: Web vs Desktop

| Feature | Web | Tauri Desktop |
|---------|-----|---------------|
| PDF preview | ✓ | ✓ |
| Signature capture | ✓ | ✓ |
| Local signing (WASM) | ✓ | ✓ |
| Audit trail | ✓ | ✓ |
| Offline mode | ✓ (limited storage) | ✓ (unlimited) |
| System fonts | ✗ | ✓ |
| Native printing | ✗ | ✓ |
| Hardware keys | ✗ | ✓ |
| File associations | ✗ | ✓ |
| Auto-updates | N/A | ✓ |
| System tray | ✗ | ✓ |

### Tauri Implementation Plan

#### Phase 1: Scaffolding (Week 1)

```bash
# Create Tauri app with shared frontend
cargo install create-tauri-app
cd apps/
npm create tauri-app@latest docsign-tauri -- --template vanilla-ts

# Structure
apps/docsign-tauri/
├── src/                    # Frontend (shared with docsign-web)
│   ├── index.html
│   ├── styles.css
│   └── main.ts
├── src-tauri/              # Rust backend
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/       # IPC handlers
│   │   ├── print.rs        # Native printing
│   │   └── security.rs     # Hardware key integration
│   └── tauri.conf.json
└── package.json
```

#### Phase 2: Shared Frontend (Week 2)

```typescript
// Detect environment and use appropriate APIs
const isDesktop = '__TAURI__' in window;

async function saveDocument(signedPdf: Uint8Array, filename: string): Promise<void> {
  if (isDesktop) {
    // Use Tauri file dialog
    const { save } = await import('@tauri-apps/api/dialog');
    const { writeBinaryFile } = await import('@tauri-apps/api/fs');

    const path = await save({
      defaultPath: filename,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });

    if (path) {
      await writeBinaryFile(path, signedPdf);
    }
  } else {
    // Web fallback: download link
    const blob = new Blob([signedPdf], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }
}
```

#### Phase 3: Native Features (Weeks 3-4)

```rust
// src-tauri/src/commands/print.rs
use tauri::command;

#[command]
pub async fn print_document(pdf_bytes: Vec<u8>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Use Windows Print API
        windows::print_pdf(&pdf_bytes)
            .map_err(|e| e.to_string())
    }

    #[cfg(target_os = "macos")]
    {
        // Use NSPrintOperation
        macos::print_pdf(&pdf_bytes)
            .map_err(|e| e.to_string())
    }

    #[cfg(target_os = "linux")]
    {
        // Use CUPS
        linux::print_pdf(&pdf_bytes)
            .map_err(|e| e.to_string())
    }
}

#[command]
pub async fn get_system_certificates() -> Result<Vec<CertificateInfo>, String> {
    // Access system certificate store for signing
    native_certs::load_native_certs()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|cert| CertificateInfo::from_der(&cert.0))
        .collect()
}
```

---

## Security & Legal Considerations

### Cryptographic Standards

| Component | Standard | Implementation |
|-----------|----------|----------------|
| Signing algorithm | ECDSA P-256 | `p256` crate |
| Hash function | SHA-256 | `sha2` crate |
| Signature format | PAdES (PDF Advanced Electronic Signatures) | Custom + `cms` crate |
| Timestamp | RFC 3161 | `shared-crypto/tsa.rs` |
| Certificate format | X.509v3 | `x509-cert` crate |

### PAdES Compliance

The signature implementation must produce Adobe-compatible PDFs:

```rust
// Signature dictionary structure
fn create_signature_dict() -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"Sig".to_vec()));
    dict.set("Filter", Object::Name(b"Adobe.PPKLite".to_vec()));
    dict.set("SubFilter", Object::Name(b"adbe.pkcs7.detached".to_vec()));
    dict.set("ByteRange", Object::Array(vec![
        Object::Integer(0),
        Object::Integer(placeholder_offset),
        Object::Integer(placeholder_end),
        Object::Integer(eof_offset),
    ]));
    dict.set("Contents", Object::String(
        vec![0u8; MAX_SIGNATURE_SIZE],
        StringFormat::Hexadecimal,
    ));
    dict.set("Reason", Object::String(
        reason.as_bytes().to_vec(),
        StringFormat::Literal,
    ));
    dict.set("M", Object::String(
        format_pdf_date(signing_time).into_bytes(),
        StringFormat::Literal,
    ));
    dict
}
```

### Audit Trail Requirements

Every signature session must maintain a tamper-evident audit log:

```rust
pub struct AuditEvent {
    pub id: String,                    // UUID
    pub timestamp: String,             // ISO 8601
    pub action: AuditAction,
    pub actor: String,                 // Email or "system"
    pub document_hash: String,         // SHA-256 of document at this point
    pub previous_hash: String,         // Hash of previous event (chain)
    pub signature: String,             // ECDSA signature of this event
    pub details: Option<String>,       // Additional context
}

pub enum AuditAction {
    DocumentUploaded,
    RecipientAdded,
    ViewedDocument,
    SignatureFieldAdded,
    SignatureApplied,
    DocumentDeclined { reason: Option<String> },
    SessionCompleted,
    SessionExpired,
}
```

### Legal Validity (US)

DocSign signatures are legally valid under:

1. **ESIGN Act (2000)** — Federal law giving electronic signatures same legal effect as handwritten
2. **UETA (1999)** — Uniform Electronic Transactions Act, adopted by 47 states
3. **State-specific laws** — Florida (F.S. § 668.50), etc.

**Key requirements we satisfy:**
- Intent to sign (deliberate signature action)
- Consent to do business electronically (consent landing page)
- Association of signature with record (audit trail)
- Record retention (PDF with embedded signature)

---

## Implementation Phases

### Phase 0: Foundation Cleanup ✅ COMPLETE

| Task | Priority | Status |
|------|----------|--------|
| Set up TypeScript + esbuild build infrastructure | P0 | ✅ DONE |
| Copy pdf-loader.ts from pdfjoin-web | P0 | ✅ DONE |
| Copy coord-utils.ts for signature placement | P0 | ✅ DONE |
| Create pdf-types.ts type definitions | P0 | ✅ DONE |
| Implement preview-only PDF bridge (pdf-preview.ts) | P0 | ✅ DONE |
| Create geriatric.css foundation (60px, 18px, 7:1) | P0 | ✅ DONE |
| Property-based tests for coordinate transforms | P0 | ✅ DONE (11 tests) |
| Property-based tests for signing verification | P0 | ✅ DONE (10 tests) |

**Phase 0 Deliverables (Dec 30, 2025):**

```
apps/docsign-web/
├── src/ts/                          # NEW TypeScript source
│   ├── main.ts                      # Entry point (imports all modules)
│   ├── pdf-loader.ts                # Lazy PDF.js loading
│   ├── pdf-preview.ts               # Preview-only bridge (no editing)
│   ├── coord-utils.ts               # DOM ↔ PDF coordinate transforms
│   └── types/pdf-types.ts           # TypeScript definitions
├── www/
│   ├── js/bundle.js                 # Compiled TypeScript (7.5KB)
│   └── geriatric.css                # Accessibility-first CSS
├── package.json                     # npm scripts (build, dev, typecheck)
└── tsconfig.json                    # TypeScript config

Test Coverage:
- docsign-wasm: 15 tests (11 property + 4 unit) for coordinates
- docsign-core: 10 property tests for signing verification
```

---

### Phase 1: Geriatric UX Overhaul ← **NEXT**

**Goal:** Integrate geriatric.css into sign.html and migrate inlined JS to TypeScript modules.

| Task | Priority | Status |
|------|----------|--------|
| Link geriatric.css in sign.html | P0 | TODO |
| Link bundle.js in sign.html | P0 | TODO |
| Migrate PDF loading logic to pdf-preview.ts | P0 | TODO |
| Apply 60px touch targets to all buttons | P0 | TODO |
| Apply 18px base typography | P0 | TODO |
| Apply AAA contrast colors | P0 | TODO |
| Create friendly error message system | P0 | TODO |
| Add progress indicators for signing flow | P0 | TODO |
| Implement confirmation dialogs | P0 | TODO |
| Remove inlined JS from sign.html (use bundle.js) | P0 | TODO |

**Phase 1 Success Criteria:**
- sign.html loads `geriatric.css` and `bundle.js`
- All buttons have 60px minimum touch targets
- All text is 18px or larger
- PDF preview uses `PdfPreviewBridge` from bundle.js
- No inlined `<script>` blocks in sign.html (all in TypeScript)

### Phase 2: Local-First Session (Weeks 5-6)

| Task | Priority | Status |
|------|----------|--------|
| Implement LocalSessionManager | P0 | TODO |
| Add offline document generation | P1 | TODO |
| Bundle critical templates in WASM | P1 | TODO |
| Remove server dependency from core flow | P0 | TODO |
| Add sync-when-available pattern | P2 | TODO |

### Phase 3: Signing UX Polish (Weeks 7-8)

| Task | Priority | Status |
|------|----------|--------|
| Improve signature capture (larger pad) | P0 | TODO |
| Add typed signature option | P1 | TODO |
| Implement undo stroke | P1 | TODO |
| Add signature preview before confirm | P0 | TODO |
| Improve mobile signature modal | P0 | TODO |

### Phase 4: Tauri Desktop (Weeks 9-12)

| Task | Priority | Status |
|------|----------|--------|
| Scaffold Tauri application | P1 | TODO |
| Share frontend code with web | P1 | TODO |
| Implement native file dialogs | P1 | TODO |
| Add native printing support | P2 | TODO |
| Implement system tray | P2 | TODO |
| Add auto-update mechanism | P2 | TODO |

### Phase 5: Testing & Polish (Weeks 13-14)

| Task | Priority | Status |
|------|----------|--------|
| Accessibility audit (screen reader) | P0 | TODO |
| Usability testing with 65+ users | P0 | TODO |
| Performance optimization | P1 | TODO |
| Security audit | P0 | TODO |
| Documentation | P1 | TODO |

---

## Testing Strategy

### Unit Tests (Rust)

```bash
# Run all docsign tests
cargo test -p docsign-wasm
cargo test -p docsign-core
cargo test -p shared-crypto
cargo test -p shared-pdf
```

### Integration Tests (Browser)

```bash
# Run Puppeteer tests for docsign-web
cargo test -p benchmark-harness --test browser_docsign
```

### Accessibility Testing

```bash
# Automated accessibility audit
npx axe-core apps/docsign-web/www/sign.html

# Screen reader testing (manual)
# - VoiceOver (macOS)
# - NVDA (Windows)
# - Orca (Linux)
```

### Usability Testing Protocol

1. **Recruit participants:** 5+ users aged 65+
2. **Task list:**
   - Upload a PDF document
   - Add your signature
   - Download the signed document
3. **Metrics:**
   - Task completion rate (target: 100%)
   - Time to complete (target: <5 minutes)
   - Errors encountered (target: 0 unrecoverable)
   - Satisfaction rating (target: 4+/5)

---

## Appendices

### A. File Structure

```
apps/docsign-web/
├── wasm/                       # WASM signing module
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # WASM exports
│       ├── coords.rs           # Coordinate transforms (with 15 property tests) ✅
│       ├── pdf/
│       │   ├── parser.rs       # PDF loading
│       │   ├── signer.rs       # PAdES injection
│       │   └── audit.rs        # Audit chain
│       ├── crypto/
│       │   ├── keys.rs         # Key management
│       │   ├── cert.rs         # Certificates
│       │   ├── cms.rs          # CMS SignedData
│       │   └── tsa.rs          # Timestamping
│       ├── session/
│       │   └── mod.rs          # Session management
│       └── storage/
│           └── indexeddb.rs    # Local storage
├── src/ts/                     # TypeScript source ✅ CREATED
│   ├── main.ts                 # Entry point (exports all modules) ✅
│   ├── pdf-preview.ts          # Preview-only bridge (no editing) ✅
│   ├── pdf-loader.ts           # Lazy PDF.js loading ✅
│   ├── coord-utils.ts          # DOM ↔ PDF coordinate transforms ✅
│   ├── signature-capture.ts    # Signature canvas (TODO Phase 3)
│   ├── session-manager.ts      # Local-first sessions (TODO Phase 2)
│   └── types/
│       └── pdf-types.ts        # Type definitions ✅
├── www/
│   ├── index.html              # Sender flow
│   ├── sign.html               # Recipient signing (37KB inlined JS → TODO migrate)
│   ├── js/
│   │   ├── bundle.js           # Compiled TypeScript (7.5KB) ✅
│   │   ├── bundle.js.map       # Source map ✅
│   │   └── vendor/
│   │       ├── pdf.min.js
│   │       └── pdf.worker.min.js
│   └── geriatric.css           # Accessibility-first CSS ✅
├── worker/                     # Cloudflare Worker (optional)
├── package.json                # npm scripts (build, dev, typecheck) ✅
├── tsconfig.json               # TypeScript config ✅
└── Trunk.toml                  # With pre_build hook for TypeScript ✅

crates/docsign-core/
└── src/
    └── lib.rs                  # With 10 property-based signing tests ✅

apps/docsign-tauri/             # Desktop application
├── src/                        # Shared frontend
├── src-tauri/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/
│   │   ├── print.rs
│   │   └── security.rs
│   └── tauri.conf.json
└── package.json
```

### B. Glossary

| Term | Definition |
|------|------------|
| **DOCSIGN_PLAN** | This document; the architectural plan for the signing platform |
| **PAdES** | PDF Advanced Electronic Signatures; standard for embedding signatures in PDFs |
| **WASM** | WebAssembly; browser runtime for compiled code |
| **Tauri** | Framework for building desktop apps with web technologies |
| **Local-first** | Architecture where all core functionality works offline |
| **Geriatric UX** | User experience designed for elderly users |

### C. Related Documents

- [CLAUDE.md](./CLAUDE.md) — Development guidelines
- [PLAN0.md](./PLAN0.md) — Overall monolith integration plan
- [UX_IMPROVEMENT_PLAN.md](./UX_IMPROVEMENT_PLAN.md) — Existing UX improvement tasks
- [PDFJOIN_EDIT_PLAN.md](./PDFJOIN_EDIT_PLAN.md) — PDFJoin editing (reference for what NOT to include)

---

**Document Identifier:** DOCSIGN_PLAN
**Version:** 1.1
**Last Updated:** December 30, 2025
**Authors:** Claude Code (AI-assisted planning)

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2025-12-30 | Phase 0 complete: TypeScript build, PDF preview, 25 property tests, geriatric CSS |
| 1.0 | 2025-12-30 | Initial plan created |
