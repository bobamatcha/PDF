/**
 * DocSign TypeScript Entry Point
 *
 * Main entry point for the docsign-web TypeScript codebase.
 * Exposes all functionality via the `window.DocSign` namespace.
 *
 * @packageDocumentation
 * @module docsign-web
 *
 * ## Architecture
 *
 * Preview-only PDF rendering (no editing):
 * - PDF.js renders pages to canvas (read-only)
 * - Signature fields overlay on top (not embedded until signing)
 * - All signing happens locally in WASM
 *
 * ## window.DocSign Namespace
 *
 * All public APIs are exposed on `window.DocSign`:
 *
 * ### PDF Operations
 * - `loadPdf(data)` - Load PDF from bytes or base64
 * - `renderAllPages(config)` - Render all pages to container
 * - `renderPage(pageNum, canvas, scale)` - Render single page
 * - `getPageCount()` - Get loaded document page count
 * - `cleanup()` - Release PDF resources
 *
 * ### Session Management
 * - `LocalSessionManager` - Static class for IndexedDB operations
 * - `localSessionManager` - Singleton instance
 * - `createSession(doc, recipients)` - Create new signing session
 * - `getSession(id)` - Get session by ID
 * - `recordSignature(sessionId, fieldId, data)` - Save signature
 *
 * ### Signature Capture
 * - `SignatureCapture` - Canvas drawing with undo/redo
 * - `TypedSignature` - Font-based signature generation
 * - `SignatureCaptureModal` - Full modal wrapper
 * - `MobileSignatureModal` - Full-screen mobile modal
 *
 * ### Sync
 * - `SyncManager` - Background sync with exponential backoff
 * - `initSyncManager(config)` - Initialize sync
 * - `SYNC_EVENTS` - Event type constants
 * - `onSyncStarted/Completed/Failed/Progress` - Event listeners
 *
 * ### Error Handling
 * - `showErrorModal(options)` - Display error modal
 * - `showConfirmDialog(options)` - Confirmation dialog
 * - `getUserFriendlyError(error)` - Convert to user message
 *
 * @example
 * ```typescript
 * // Load and render PDF
 * const result = await DocSign.loadPdf(pdfBytes);
 * await DocSign.renderAllPages({ container: document.getElementById('viewer') });
 *
 * // Create signature
 * const capture = new DocSign.SignatureCapture({ container: sigContainer });
 * capture.onchange = (isEmpty) => submitBtn.disabled = isEmpty;
 *
 * // Save signature
 * await DocSign.recordSignature(sessionId, fieldId, capture.toDataURL());
 * ```
 */

// Import core modules
import { ensurePdfJsLoaded, isPdfJsLoaded } from "./pdf-loader";
import { PdfPreviewBridge, previewBridge } from "./pdf-preview";
import { createLogger } from "./logger";

const log = createLogger('DocSign');
import {
  domRectToPdf,
  pdfRectToDom,
  domPointToPdf,
  pdfPointToDom,
  getPageRenderInfo,
} from "./coord-utils";
import type { IPdfPreviewBridge, PageDimensions, TextItem } from "./types/pdf-types";

// Import sign-pdf-bridge and initialize DocSign namespace
import {
  initDocSignNamespace,
  docSignPdfBridge,
  // Re-export error handling functions from sign-pdf-bridge
  getUserFriendlyError,
  categorizeError,
  createUserError,
  getOfflineError,
  getFileTooLargeError,
  getUnsupportedFileError,
  showErrorModal,
  hideErrorModal,
  showErrorToast,
  hideErrorToast,
  showConfirmDialog,
  // Sync manager
  SyncManager,
  getSyncManager,
  initSyncManager,
  SYNC_EVENTS,
  onSyncStarted,
  onSyncCompleted,
  onSyncFailed,
  onSyncProgress,
  onOnlineStatusChanged,
} from "./sign-pdf-bridge";

// Import local session management
import {
  LocalSessionManager,
  localSessionManager,
  initLocalSessionNamespace,
} from "./local-session-manager";
import type {
  Session,
  SessionSummary,
  Recipient,
  RecipientStatus,
  SignatureData,
} from "./local-session-manager";

// Import typed signature component
import {
  TypedSignature,
  createTypedSignature,
  SIGNATURE_FONTS,
} from "./typed-signature";
import type { TypedSignatureOptions, SignatureFontName } from "./typed-signature";

// Import mobile signature modal
import {
  MobileSignatureModal,
  isMobileDevice,
  createSignatureModal,
} from "./mobile-signature-modal";
import type {
  MobileModalOptions,
  SignatureResult as MobileSignatureResult,
} from "./mobile-signature-modal";

// Import signature modal controller (integrates TypedSignature)
import {
  SignatureModal,
  initSignatureModal,
  SignatureCaptureModal,
  createSignatureCaptureModal,
} from "./signature-modal";
import type {
  SignatureResult,
  SignatureModalOptions,
  SignatureCaptureModalOptions,
} from "./signature-modal";

// Import improved signature capture component (Phase 3)
import { SignatureCapture } from "./signature-capture";
import type {
  SignatureCaptureOptions,
  Stroke,
  StrokePoint,
} from "./signature-capture";
import type {
  DocSignPdfBridge,
  LoadPdfResult,
  RenderPageResult,
  RenderConfig,
  UserError,
  ErrorIcon,
  ErrorCategory,
  ToastType,
  SyncStatus,
  SyncError,
  SyncManagerConfig,
} from "./sign-pdf-bridge";

// Import performance monitoring
import {
  perf,
  PERF_MARKS,
  withTiming,
  withTimingSync,
  withLoading,
} from "./perf";
import type { PerformanceMetrics, PerfMarkId } from "./perf";

// Import authentication module
import {
  isAuthenticated,
  getCurrentUser,
  getAccessToken,
  getDocumentsRemaining,
  register,
  login,
  logout,
  refreshToken,
  forgotPassword,
  resetPassword,
  authenticatedFetch,
  validatePassword,
  validateEmail,
  onAuthStateChange,
  initAuthNamespace,
} from "./auth";
import type {
  User,
  UserTier,
  AuthTokens,
  RegisterResponse,
  LoginResponse,
  RefreshResponse,
  AuthResponse,
  AuthStateChangeEvent,
} from "./auth";

// Re-export for backwards compatibility and external access
export {
  // PDF modules
  ensurePdfJsLoaded,
  isPdfJsLoaded,
  PdfPreviewBridge,
  previewBridge,
  domRectToPdf,
  pdfRectToDom,
  domPointToPdf,
  pdfPointToDom,
  getPageRenderInfo,
  // Sign PDF bridge exports
  initDocSignNamespace,
  docSignPdfBridge,
  // Error handling exports
  getUserFriendlyError,
  categorizeError,
  createUserError,
  getOfflineError,
  getFileTooLargeError,
  getUnsupportedFileError,
  showErrorModal,
  hideErrorModal,
  showErrorToast,
  hideErrorToast,
  showConfirmDialog,
  // Sync manager exports
  SyncManager,
  getSyncManager,
  initSyncManager,
  SYNC_EVENTS,
  onSyncStarted,
  onSyncCompleted,
  onSyncFailed,
  onSyncProgress,
  onOnlineStatusChanged,
  // Local session management exports
  LocalSessionManager,
  localSessionManager,
  initLocalSessionNamespace,
  // Typed signature exports
  TypedSignature,
  createTypedSignature,
  SIGNATURE_FONTS,
  // Mobile signature modal exports
  MobileSignatureModal,
  isMobileDevice,
  createSignatureModal,
  // Signature modal controller exports
  SignatureModal,
  initSignatureModal,
  // Phase 3: Improved signature capture
  SignatureCapture,
  SignatureCaptureModal,
  createSignatureCaptureModal,
  // Performance monitoring
  perf,
  PERF_MARKS,
  withTiming,
  withTimingSync,
  withLoading,
  // Authentication module
  isAuthenticated,
  getCurrentUser,
  getAccessToken,
  getDocumentsRemaining,
  register,
  login,
  logout,
  refreshToken,
  forgotPassword,
  resetPassword,
  authenticatedFetch,
  validatePassword,
  validateEmail,
  onAuthStateChange,
  initAuthNamespace,
};

export type {
  IPdfPreviewBridge,
  PageDimensions,
  TextItem,
  DocSignPdfBridge,
  LoadPdfResult,
  RenderPageResult,
  RenderConfig,
  UserError,
  ErrorIcon,
  ErrorCategory,
  ToastType,
  // Sync manager types
  SyncStatus,
  SyncError,
  SyncManagerConfig,
  // Local session management types
  Session,
  SessionSummary,
  Recipient,
  RecipientStatus,
  SignatureData,
  // Typed signature types
  TypedSignatureOptions,
  SignatureFontName,
  // Mobile signature modal types
  MobileModalOptions,
  MobileSignatureResult,
  // Signature modal types
  SignatureResult,
  SignatureModalOptions,
  // Phase 3: Improved signature capture types
  SignatureCaptureOptions,
  SignatureCaptureModalOptions,
  Stroke,
  StrokePoint,
  // Performance monitoring types
  PerformanceMetrics,
  PerfMarkId,
  // Authentication types
  User,
  UserTier,
  AuthTokens,
  RegisterResponse,
  LoginResponse,
  RefreshResponse,
  AuthResponse,
  AuthStateChangeEvent,
};

/**
 * Default sync endpoint - can be overridden via window.DOCSIGN_SYNC_ENDPOINT
 */
const DEFAULT_SYNC_ENDPOINT = "https://docsign-worker.orlandodowntownhome.workers.dev/signatures/sync";

/**
 * Initialize DocSign application
 */
function init(): void {
  // Mark namespace initialization start
  perf.mark(PERF_MARKS.NAMESPACE_INIT);

  // Initialize the DocSign namespace on window for sign.js to use
  initDocSignNamespace();

  // Initialize local session management on window.DocSign
  initLocalSessionNamespace();

  // Initialize authentication module on window.DocSign
  initAuthNamespace();

  // Initialize SyncManager with configurable endpoint
  const syncEndpoint =
    (window as unknown as { DOCSIGN_SYNC_ENDPOINT?: string }).DOCSIGN_SYNC_ENDPOINT ||
    DEFAULT_SYNC_ENDPOINT;

  initSyncManager({
    syncEndpoint,
    minBackoffMs: 1000,
    maxBackoffMs: 30000,
    retryIntervalMs: 30000,
    maxRetries: 10,
  });

  // Add TypedSignature and MobileSignatureModal to window.DocSign namespace
  if (typeof window !== "undefined" && window.DocSign) {
    const docSign = window.DocSign as unknown as Record<string, unknown>;

    // TypedSignature exports
    docSign.TypedSignature = TypedSignature;
    docSign.createTypedSignature = createTypedSignature;
    docSign.SIGNATURE_FONTS = SIGNATURE_FONTS;

    // MobileSignatureModal exports
    docSign.MobileSignatureModal = MobileSignatureModal;
    docSign.isMobileDevice = isMobileDevice;
    docSign.createSignatureModal = createSignatureModal;

    // SignatureModal exports
    docSign.SignatureModal = SignatureModal;
    docSign.initSignatureModal = initSignatureModal;

    // Phase 3: Improved signature capture exports
    docSign.SignatureCapture = SignatureCapture;
    docSign.SignatureCaptureModal = SignatureCaptureModal;
    docSign.createSignatureCaptureModal = createSignatureCaptureModal;

    // Performance monitoring exports
    docSign.perf = perf;
    docSign.PERF_MARKS = PERF_MARKS;
    docSign.withTiming = withTiming;
    docSign.withLoading = withLoading;
  }

  // Mark application as interactive
  perf.mark(PERF_MARKS.INTERACTIVE);

  // Log performance metrics if enabled
  if (perf.isEnabled()) {
    perf.logMetrics();
  }

  log.info("DocSign TypeScript initialized");
  log.debug("PDF Preview Bridge available:", typeof PdfPreviewBridge !== "undefined");
  log.debug("DocSign namespace available:", typeof window.DocSign !== "undefined");
  log.debug("LocalSessionManager available:", typeof LocalSessionManager !== "undefined");
  log.debug("SyncManager available:", typeof SyncManager !== "undefined");
  log.debug("TypedSignature available:", typeof TypedSignature !== "undefined");
  log.debug("MobileSignatureModal available:", typeof MobileSignatureModal !== "undefined");
  log.debug("SignatureCapture available:", typeof SignatureCapture !== "undefined");
  log.debug("SignatureCaptureModal available:", typeof SignatureCaptureModal !== "undefined");
  log.debug("Auth module available:", typeof isAuthenticated !== "undefined");
  log.debug("User authenticated:", isAuthenticated());
}

// Initialize when DOM is ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
