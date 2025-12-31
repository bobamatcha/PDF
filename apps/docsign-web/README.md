# docsign-web

Geriatric-friendly document signing web application. Part of the GetSignatures platform.

## Overview

docsign-web is a local-first web application for signing PDF documents. Designed with users 65+ in mind:

- **Large touch targets** (60px minimum)
- **High contrast** (AAA level, 7:1 ratio)
- **Large fonts** (18px base, 24px actions)
- **No time limits** - sessions never expire
- **Works offline** - sign documents without internet
- **Privacy-focused** - documents stay on your device

## Quick Start

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build TypeScript only
npm run build

# Type check
npm run typecheck

# Run tests
npm test
```

## Available Scripts

| Script | Description |
|--------|-------------|
| `npm run build` | Build TypeScript bundle (esbuild) |
| `npm run build:watch` | Watch mode for TypeScript |
| `npm run dev` | Run TypeScript watch + trunk serve |
| `npm run typecheck` | Type-check without emitting |
| `npm test` | Run vitest tests |
| `npm run test:watch` | Run tests in watch mode |

## Architecture

### Local-First Design

```
User Device
├── IndexedDB (sessions, PDFs, signatures)
├── TypeScript Bundle (141KB)
├── PDF.js (lazy loaded)
└── WASM Module (signing)

Optional Server (sync only)
└── Signature sync when online
```

### Key Components

| Component | File | Purpose |
|-----------|------|---------|
| `LocalSessionManager` | `local-session-manager.ts` | IndexedDB session storage |
| `SignatureCapture` | `signature-capture.ts` | Canvas-based signature drawing |
| `TypedSignature` | `typed-signature.ts` | Font-based signatures |
| `PdfPreviewBridge` | `pdf-preview.ts` | PDF.js rendering (preview-only) |
| `SyncManager` | `sync-manager.ts` | Background sync with retry |
| `MobileSignatureModal` | `mobile-signature-modal.ts` | Full-screen mobile signing |

### Directory Structure

```
src/ts/
├── main.ts                    # Entry point, exports DocSign namespace
├── pdf-loader.ts              # Lazy PDF.js loading
├── pdf-preview.ts             # PDF rendering (preview-only)
├── coord-utils.ts             # DOM <-> PDF coordinate transforms
├── sign-pdf-bridge.ts         # Bridge for legacy sign.js
├── local-session-manager.ts   # IndexedDB session storage
├── sync-manager.ts            # Background sync
├── sync-events.ts             # Custom sync events
├── signature-capture.ts       # Canvas signature with undo/redo
├── typed-signature.ts         # Font-based signatures
├── mobile-signature-modal.ts  # Full-screen mobile modal
├── signature-modal.ts         # Modal wrapper
├── error-messages.ts          # User-friendly error messages
├── error-ui.ts                # Modal dialogs, toasts
├── session.ts                 # Session validation
├── types/
│   └── pdf-types.ts           # TypeScript definitions
└── __tests__/
    ├── session.test.ts
    ├── local-session-manager.test.ts
    ├── error-messages.test.ts
    └── signature-capture.test.ts

www/
├── index.html                 # Landing page
├── sign.html                  # Signing interface
├── sign.js                    # Legacy signing logic
├── geriatric.css              # Accessibility-first CSS
└── js/
    ├── bundle.js              # Compiled TypeScript
    └── vendor/
        ├── pdf.min.js         # PDF.js (lazy loaded)
        └── pdf.worker.min.js  # PDF.js worker
```

## TypeScript Modules

### window.DocSign Namespace

All TypeScript functionality is exposed on `window.DocSign`:

```typescript
// PDF Loading
await DocSign.loadPdf(pdfBytes);           // Load PDF from bytes
await DocSign.renderAllPages({ container }); // Render to container
DocSign.getPageCount();                    // Get page count
DocSign.cleanup();                         // Release resources

// Sessions
await DocSign.createSession(document, recipients);
await DocSign.getSession(sessionId);
await DocSign.recordSignature(sessionId, fieldId, data);

// Signatures
const capture = new DocSign.SignatureCapture({ container });
const typed = new DocSign.TypedSignature({ container, name: 'John Doe' });

// Sync
DocSign.initSyncManager({ syncEndpoint: '/api/sync' });
DocSign.onSyncCompleted(({ syncedCount }) => { ... });

// Errors
DocSign.showErrorModal({ title, message, action });
DocSign.showConfirmDialog({ title, message });
```

### Sync Events

Listen for sync status changes:

```typescript
// Event types
DocSign.SYNC_EVENTS.STARTED       // 'docsign:sync-started'
DocSign.SYNC_EVENTS.COMPLETED     // 'docsign:sync-completed'
DocSign.SYNC_EVENTS.FAILED        // 'docsign:sync-failed'
DocSign.SYNC_EVENTS.PROGRESS      // 'docsign:sync-progress'

// Typed listeners
const unsubscribe = DocSign.onSyncCompleted((detail) => {
  console.log(`Synced ${detail.syncedCount} items`);
});

// Cleanup
unsubscribe();
```

## Testing

### Run All Tests

```bash
npm test
```

### Test Coverage

| Area | Tests |
|------|-------|
| Session validation | 35 property tests |
| LocalSessionManager | 55 property tests |
| SignatureCapture | 64 property tests |
| Error messages | Property tests |

### Test Files

- `src/ts/__tests__/session.test.ts`
- `src/ts/__tests__/local-session-manager.test.ts`
- `src/ts/__tests__/signature-capture.test.ts`
- `src/ts/__tests__/error-messages.test.ts`

## Geriatric UX Guidelines

### Touch Targets

All interactive elements: **60px minimum** height/width.

```css
button, a, input[type="checkbox"] {
  min-width: 60px;
  min-height: 60px;
}
```

### Typography

- Base font: **18px** (Atkinson Hyperlegible)
- Action buttons: **24px**
- Line height: **1.6**

### Colors

AAA contrast (7:1 minimum):

| Element | Color |
|---------|-------|
| Primary text | `#1a1a1a` |
| Background | `#ffffff` |
| Action button | `#0056b3` |
| Error | `#b30000` |
| Success | `#006644` |

## Related Documentation

- [DOCSIGN_PLAN.md](/DOCSIGN_PLAN.md) - Full architectural plan
- [USER_GUIDE.md](./USER_GUIDE.md) - End-user guide
- [CLAUDE.md](/CLAUDE.md) - Development guidelines
