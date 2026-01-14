/**
 * Geriatric-Friendly Error UI Components
 *
 * DOCSIGN_PLAN: UI components for displaying errors designed for users 65+:
 * - Large, readable text (uses geriatric.css variables)
 * - Clear visual hierarchy
 * - Accessible (ARIA labels, focus management)
 * - Touch-friendly buttons (60px+ targets)
 */

import type { UserError, ErrorIcon } from "./error-messages";

/**
 * Toast notification types
 */
export type ToastType = "error" | "warning" | "success";

/**
 * Icon SVG paths for each error type
 */
const ICON_SVGS: Record<ErrorIcon, string> = {
  "wifi-off": `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <line x1="1" y1="1" x2="23" y2="23"></line>
    <path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55"></path>
    <path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39"></path>
    <path d="M10.71 5.05A16 16 0 0 1 22.58 9"></path>
    <path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88"></path>
    <path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path>
    <line x1="12" y1="20" x2="12.01" y2="20"></line>
  </svg>`,
  lock: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
    <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
  </svg>`,
  signature: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20 17.5a2.5 2.5 0 0 1-2.5 2.5H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h5.5L14 6.5H20a2 2 0 0 1 2 2v9"></path>
    <path d="M18 17l-3 3 1-6 5-5-3-3-5 5-6 1 3 3z"></path>
  </svg>`,
  clock: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <polyline points="12 6 12 12 16 14"></polyline>
  </svg>`,
  file: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path>
    <polyline points="13 2 13 9 20 9"></polyline>
  </svg>`,
  alert: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="12" y1="8" x2="12" y2="12"></line>
    <line x1="12" y1="16" x2="12.01" y2="16"></line>
  </svg>`,
  user: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
    <circle cx="12" cy="7" r="4"></circle>
  </svg>`,
};

/**
 * Toast icon SVGs
 */
const TOAST_ICONS: Record<ToastType, string> = {
  error: `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="15" y1="9" x2="9" y2="15"></line>
    <line x1="9" y1="9" x2="15" y2="15"></line>
  </svg>`,
  warning: `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
    <line x1="12" y1="9" x2="12" y2="13"></line>
    <line x1="12" y1="17" x2="12.01" y2="17"></line>
  </svg>`,
  success: `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
    <polyline points="22 4 12 14.01 9 11.01"></polyline>
  </svg>`,
};

/**
 * Currently displayed modal (for cleanup)
 */
let currentModal: HTMLElement | null = null;

/**
 * Currently displayed toast (for replacement)
 */
let currentToast: HTMLElement | null = null;

/**
 * Toast timeout handle
 */
let toastTimeout: ReturnType<typeof setTimeout> | null = null;

/**
 * Create the modal overlay element
 */
function createModalOverlay(): HTMLDivElement {
  const overlay = document.createElement("div");
  overlay.className = "modal-overlay error-modal-overlay";
  overlay.setAttribute("role", "dialog");
  overlay.setAttribute("aria-modal", "true");
  return overlay;
}

/**
 * Create the modal content container
 */
function createModalContent(error: UserError): HTMLDivElement {
  const content = document.createElement("div");
  content.className = "modal-content error-modal-content confirm-dialog";
  content.setAttribute("role", "alertdialog");
  content.setAttribute("aria-labelledby", "error-modal-title");
  content.setAttribute("aria-describedby", "error-modal-message");

  content.innerHTML = `
    <div class="confirm-icon error-icon" aria-hidden="true">
      ${ICON_SVGS[error.icon]}
    </div>
    <h2 id="error-modal-title" class="confirm-title error-title">${escapeHtml(error.title)}</h2>
    <p id="error-modal-message" class="confirm-message error-modal-message">${escapeHtml(error.message)}</p>
    <div class="confirm-actions error-modal-actions">
      <button type="button" class="btn-primary btn-large error-modal-action" data-action="primary">
        ${escapeHtml(error.action)}
      </button>
      <button type="button" class="btn-secondary error-modal-dismiss" data-action="dismiss">
        Close
      </button>
    </div>
  `;

  return content;
}

/**
 * Display a modal error dialog
 *
 * Uses geriatric.css classes for styling:
 * - .modal-overlay, .modal-content for layout
 * - .confirm-dialog, .confirm-icon, .confirm-title, .confirm-message for structure
 * - .btn-primary, .btn-secondary, .btn-large for buttons
 *
 * @param error - The UserError to display
 * @param onAction - Optional callback when the primary action button is clicked
 * @param onDismiss - Optional callback when the modal is dismissed
 *
 * @example
 * ```ts
 * const error = getUserFriendlyError(new Error('network failed'));
 * showErrorModal(error, () => {
 *   // Retry the operation
 *   retryOperation();
 * });
 * ```
 */
export function showErrorModal(
  error: UserError,
  onAction?: () => void,
  onDismiss?: () => void
): void {
  // Close any existing modal
  hideErrorModal();

  // Create modal elements
  const overlay = createModalOverlay();
  const content = createModalContent(error);
  overlay.appendChild(content);

  // Store reference for cleanup
  currentModal = overlay;

  // Add to DOM
  document.body.appendChild(overlay);

  // Focus the primary action button
  const primaryButton = content.querySelector<HTMLButtonElement>(
    '[data-action="primary"]'
  );
  const dismissButton = content.querySelector<HTMLButtonElement>(
    '[data-action="dismiss"]'
  );

  // Focus trap - keep focus within modal
  const focusableElements = content.querySelectorAll<HTMLButtonElement>("button");
  const firstFocusable = focusableElements[0];
  const lastFocusable = focusableElements[focusableElements.length - 1];

  // Focus primary button on open
  setTimeout(() => {
    primaryButton?.focus();
  }, 100);

  // Event handlers
  const handlePrimaryClick = () => {
    hideErrorModal();
    onAction?.();
  };

  const handleDismissClick = () => {
    hideErrorModal();
    onDismiss?.();
  };

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === overlay) {
      hideErrorModal();
      onDismiss?.();
    }
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      hideErrorModal();
      onDismiss?.();
    }

    // Tab trap
    if (e.key === "Tab") {
      if (e.shiftKey && document.activeElement === firstFocusable) {
        e.preventDefault();
        lastFocusable?.focus();
      } else if (!e.shiftKey && document.activeElement === lastFocusable) {
        e.preventDefault();
        firstFocusable?.focus();
      }
    }
  };

  // Attach event listeners
  primaryButton?.addEventListener("click", handlePrimaryClick);
  dismissButton?.addEventListener("click", handleDismissClick);
  overlay.addEventListener("click", handleOverlayClick);
  document.addEventListener("keydown", handleKeydown);

  // Store cleanup function
  (overlay as HTMLElement & { _cleanup?: () => void })._cleanup = () => {
    primaryButton?.removeEventListener("click", handlePrimaryClick);
    dismissButton?.removeEventListener("click", handleDismissClick);
    overlay.removeEventListener("click", handleOverlayClick);
    document.removeEventListener("keydown", handleKeydown);
  };
}

/**
 * Hide the currently displayed error modal
 */
export function hideErrorModal(): void {
  if (currentModal) {
    // Run cleanup if available
    const cleanup = (currentModal as HTMLElement & { _cleanup?: () => void })
      ._cleanup;
    cleanup?.();

    // Remove from DOM
    currentModal.remove();
    currentModal = null;
  }
}

/**
 * Display a toast notification
 *
 * Toasts appear at the bottom of the screen and auto-dismiss after 5 seconds.
 * Only one toast can be visible at a time - new toasts replace existing ones.
 *
 * Uses geriatric.css classes for styling:
 * - .alert-error, .alert-warning, .alert-success for colors
 *
 * @param message - The message to display
 * @param type - The type of toast (error, warning, success)
 * @param duration - How long to show the toast in ms (default 5000)
 *
 * @example
 * ```ts
 * showErrorToast('Please complete all required fields', 'warning');
 * showErrorToast('Document saved successfully', 'success');
 * ```
 */
export function showErrorToast(
  message: string,
  type: ToastType = "error",
  duration: number = 5000
): void {
  // Clear existing toast
  hideErrorToast();

  // Create toast element
  const toast = document.createElement("div");
  toast.className = `error-toast error-toast-${type}`;
  toast.setAttribute("role", "alert");
  toast.setAttribute("aria-live", type === "error" ? "assertive" : "polite");

  // Get appropriate CSS class based on type
  const alertClass =
    type === "error"
      ? "alert-error"
      : type === "warning"
        ? "alert-warning"
        : "alert-success";

  toast.innerHTML = `
    <div class="error-toast-content ${alertClass}">
      <span class="error-toast-icon" aria-hidden="true">
        ${TOAST_ICONS[type]}
      </span>
      <span class="error-toast-message">${escapeHtml(message)}</span>
      <button type="button" class="error-toast-close" aria-label="Close notification">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>
  `;

  // Apply inline styles for positioning (geriatric-friendly sizing)
  toast.style.cssText = `
    position: fixed;
    bottom: var(--spacing-lg, 32px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 2000;
    max-width: calc(100% - var(--spacing-lg, 32px) * 2);
    width: 100%;
    max-width: 600px;
    animation: error-toast-slide-up 0.3s ease-out;
  `;

  // Style the content
  const content = toast.querySelector<HTMLDivElement>(".error-toast-content");
  if (content) {
    content.style.cssText = `
      display: flex;
      align-items: center;
      gap: var(--spacing-sm, 16px);
      padding: var(--spacing-md, 24px);
      border-radius: var(--border-radius-lg, 12px);
      font-size: var(--font-size-lg, 22px);
      font-weight: 500;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    `;
  }

  // Style the close button
  const closeButton =
    toast.querySelector<HTMLButtonElement>(".error-toast-close");
  if (closeButton) {
    closeButton.style.cssText = `
      background: none;
      border: none;
      cursor: pointer;
      padding: 8px;
      margin-left: auto;
      opacity: 0.7;
      transition: opacity 0.2s;
      color: inherit;
      min-width: 44px;
      min-height: 44px;
      display: flex;
      align-items: center;
      justify-content: center;
    `;
    closeButton.addEventListener("click", hideErrorToast);
    closeButton.addEventListener("mouseenter", () => {
      closeButton.style.opacity = "1";
    });
    closeButton.addEventListener("mouseleave", () => {
      closeButton.style.opacity = "0.7";
    });
  }

  // Add keyframe animation if not already added
  if (!document.getElementById("error-toast-styles")) {
    const style = document.createElement("style");
    style.id = "error-toast-styles";
    style.textContent = `
      @keyframes error-toast-slide-up {
        from {
          transform: translateX(-50%) translateY(100%);
          opacity: 0;
        }
        to {
          transform: translateX(-50%) translateY(0);
          opacity: 1;
        }
      }
      @keyframes error-toast-slide-down {
        from {
          transform: translateX(-50%) translateY(0);
          opacity: 1;
        }
        to {
          transform: translateX(-50%) translateY(100%);
          opacity: 0;
        }
      }
    `;
    document.head.appendChild(style);
  }

  // Add to DOM
  document.body.appendChild(toast);
  currentToast = toast;

  // Auto-dismiss after duration
  toastTimeout = setTimeout(() => {
    hideErrorToast();
  }, duration);
}

/**
 * Hide the currently displayed toast
 */
export function hideErrorToast(): void {
  if (toastTimeout) {
    clearTimeout(toastTimeout);
    toastTimeout = null;
  }

  if (currentToast) {
    // Animate out
    currentToast.style.animation = "error-toast-slide-down 0.3s ease-in forwards";
    const toastToRemove = currentToast;
    currentToast = null;

    setTimeout(() => {
      toastToRemove.remove();
    }, 300);
  }
}

/**
 * Escape HTML to prevent XSS
 */
function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

/**
 * Show a simple confirmation dialog
 *
 * @param title - Dialog title
 * @param message - Dialog message
 * @param confirmText - Text for confirm button
 * @param cancelText - Text for cancel button
 * @returns Promise resolving to true if confirmed, false if cancelled
 *
 * @example
 * ```ts
 * const confirmed = await showConfirmDialog(
 *   'Remove Document?',
 *   'This will remove the document from your list.',
 *   'Yes, Remove It',
 *   'No, Keep It'
 * );
 * if (confirmed) {
 *   // Proceed with removal
 * }
 * ```
 */
export function showConfirmDialog(
  title: string,
  message: string,
  confirmText: string = "Yes",
  cancelText: string = "No"
): Promise<boolean> {
  return new Promise((resolve) => {
    const userError: UserError = {
      title,
      message,
      action: confirmText,
      icon: "alert",
    };

    // We'll use showErrorModal but override the dismiss button text
    hideErrorModal();

    const overlay = createModalOverlay();
    const content = document.createElement("div");
    content.className = "modal-content error-modal-content confirm-dialog";
    content.setAttribute("role", "alertdialog");
    content.setAttribute("aria-labelledby", "confirm-modal-title");
    content.setAttribute("aria-describedby", "confirm-modal-message");

    content.innerHTML = `
      <div class="confirm-icon" aria-hidden="true">
        ${ICON_SVGS.alert}
      </div>
      <h2 id="confirm-modal-title" class="confirm-title">${escapeHtml(title)}</h2>
      <p id="confirm-modal-message" class="confirm-message">${escapeHtml(message)}</p>
      <div class="confirm-actions">
        <button type="button" class="btn-primary btn-large" data-action="confirm">
          ${escapeHtml(confirmText)}
        </button>
        <button type="button" class="btn-secondary" data-action="cancel">
          ${escapeHtml(cancelText)}
        </button>
      </div>
    `;

    overlay.appendChild(content);
    currentModal = overlay;
    document.body.appendChild(overlay);

    const confirmButton = content.querySelector<HTMLButtonElement>(
      '[data-action="confirm"]'
    );
    const cancelButton = content.querySelector<HTMLButtonElement>(
      '[data-action="cancel"]'
    );

    setTimeout(() => {
      confirmButton?.focus();
    }, 100);

    const cleanup = () => {
      hideErrorModal();
    };

    confirmButton?.addEventListener("click", () => {
      cleanup();
      resolve(true);
    });

    cancelButton?.addEventListener("click", () => {
      cleanup();
      resolve(false);
    });

    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) {
        cleanup();
        resolve(false);
      }
    });

    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        document.removeEventListener("keydown", handleKeydown);
        cleanup();
        resolve(false);
      }
    };
    document.addEventListener("keydown", handleKeydown);
  });
}
