/**
 * Geriatric-Friendly Error Messages
 *
 * DOCSIGN_PLAN: Error handling designed for users 65+ with:
 * - Clear, reassuring language (no technical jargon)
 * - "What happened" + "What to do next" format
 * - No raw error messages or stack traces
 * - Plain language that reduces anxiety
 */

/**
 * User-friendly error structure
 */
export interface UserError {
  /** Short, clear title describing the problem */
  title: string;
  /** Detailed explanation in plain language */
  message: string;
  /** Suggested action the user can take */
  action: string;
  /** Icon identifier for visual context */
  icon: ErrorIcon;
}

/**
 * Error icon types for visual feedback
 */
export type ErrorIcon =
  | "wifi-off" // Network/connection issues
  | "lock" // Password/encryption issues
  | "signature" // Signature-related issues
  | "clock" // Timeout/expiry issues
  | "file" // Document/file issues
  | "alert" // Generic/unknown issues
  | "user"; // Authentication/identity issues

/**
 * Error category for internal classification
 */
export type ErrorCategory =
  | "network"
  | "password-protected"
  | "signature-invalid"
  | "session-expired"
  | "file-corrupt"
  | "authentication"
  | "generic";

/**
 * Map of error patterns to categories
 */
const ERROR_PATTERNS: Array<{
  patterns: RegExp[];
  category: ErrorCategory;
}> = [
  {
    category: "network",
    patterns: [
      /network/i,
      /fetch/i,
      /offline/i,
      /connection/i,
      /internet/i,
      /failed to load/i,
      /timeout/i,
      /ECONNREFUSED/i,
      /ENOTFOUND/i,
      /ERR_NETWORK/i,
      /net::/i,
    ],
  },
  {
    category: "password-protected",
    patterns: [
      /password/i,
      /encrypted/i,
      /protected/i,
      /decrypt/i,
      /locked/i,
      /access denied.*pdf/i,
    ],
  },
  {
    category: "signature-invalid",
    patterns: [
      /signature.*invalid/i,
      /invalid.*signature/i,
      /signature.*failed/i,
      /failed.*signature/i,
      /signature.*error/i,
      /sign.*failed/i,
      /could not sign/i,
      /signing.*error/i,
    ],
  },
  {
    category: "session-expired",
    patterns: [
      /session.*expired/i,
      /expired.*session/i,
      /link.*expired/i,
      /expired.*link/i,
      /session.*not found/i,
      /invalid.*session/i,
      /401/i,
      /403/i,
      /unauthorized/i,
    ],
  },
  {
    category: "file-corrupt",
    patterns: [
      /corrupt/i,
      /invalid.*pdf/i,
      /pdf.*invalid/i,
      /malformed/i,
      /cannot.*read/i,
      /parse.*error/i,
      /invalid.*document/i,
      /damaged/i,
    ],
  },
  {
    category: "authentication",
    patterns: [
      /authentication/i,
      /login/i,
      /credentials/i,
      /identity/i,
      /verification.*failed/i,
    ],
  },
];

/**
 * Friendly error messages for each category
 */
const FRIENDLY_ERRORS: Record<ErrorCategory, UserError> = {
  network: {
    title: "Connection Problem",
    message:
      "We could not connect to the internet right now. Your document is completely safe and has not been lost. Please check your internet connection and try again when you are back online.",
    action: "Try Again",
    icon: "wifi-off",
  },
  "password-protected": {
    title: "This PDF is Password-Protected",
    message:
      "This document has a password that prevents us from opening it. Please contact the person who sent you this document and ask them for the password, or request an unprotected version.",
    action: "Enter Password",
    icon: "lock",
  },
  "signature-invalid": {
    title: "Signature Problem",
    message:
      "We had trouble adding your signature to the document. This sometimes happens if the signature was drawn too quickly. Please try drawing your signature again, taking your time with each stroke.",
    action: "Try Again",
    icon: "signature",
  },
  "session-expired": {
    title: "Signing Link Has Expired",
    message:
      "The link you used to sign this document is no longer active. This can happen if some time has passed since you received the email. Please contact the sender to request a new signing link.",
    action: "Request New Link",
    icon: "clock",
  },
  "file-corrupt": {
    title: "Document Problem",
    message:
      "We could not open this document because it may be damaged or in an unsupported format. Please contact the sender and ask them to send the document again.",
    action: "Contact Sender",
    icon: "file",
  },
  authentication: {
    title: "Identity Verification Problem",
    message:
      "We could not verify your identity to access this document. Please make sure you are using the correct signing link from your email.",
    action: "Check Link",
    icon: "user",
  },
  generic: {
    title: "Something Went Wrong",
    message:
      "We ran into an unexpected problem, but your document is safe. If this keeps happening, please contact the person who sent you this document for help.",
    action: "Go Back",
    icon: "alert",
  },
};

/**
 * Determine error category from error message
 */
export function categorizeError(error: Error | string): ErrorCategory {
  const message = typeof error === "string" ? error : error.message;

  for (const { patterns, category } of ERROR_PATTERNS) {
    for (const pattern of patterns) {
      if (pattern.test(message)) {
        return category;
      }
    }
  }

  return "generic";
}

/**
 * Convert a raw error into a user-friendly error message
 *
 * This function NEVER exposes raw error messages or stack traces.
 * All output is designed to be clear and reassuring for elderly users.
 *
 * @param error - The original error (Error object or string)
 * @returns A UserError with friendly title, message, action, and icon
 *
 * @example
 * ```ts
 * try {
 *   await fetchDocument();
 * } catch (err) {
 *   const friendly = getUserFriendlyError(err);
 *   showErrorModal(friendly);
 * }
 * ```
 */
export function getUserFriendlyError(error: Error | string): UserError {
  const category = categorizeError(error);
  return { ...FRIENDLY_ERRORS[category] };
}

/**
 * Create a custom user-friendly error
 *
 * Use this when you need to create a specific error message
 * that doesn't fit the standard categories.
 *
 * @param title - Short, clear title
 * @param message - Detailed explanation
 * @param action - Suggested action text
 * @param icon - Icon identifier
 */
export function createUserError(
  title: string,
  message: string,
  action: string,
  icon: ErrorIcon = "alert"
): UserError {
  return { title, message, action, icon };
}

/**
 * Get a friendly error for offline state
 */
export function getOfflineError(): UserError {
  return {
    title: "You Are Offline",
    message:
      "Your device is not connected to the internet. Your work is saved locally and will be sent automatically when you reconnect. You can continue working offline.",
    action: "Continue",
    icon: "wifi-off",
  };
}

/**
 * Get a friendly error for file too large
 * Default is 100MB (MAX_PDF_SIZE_BYTES from docsign-core)
 */
export function getFileTooLargeError(maxSizeMb: number = 100): UserError {
  return {
    title: "File Is Too Large",
    message: `This PDF is larger than ${maxSizeMb} MB, which is the maximum size we can handle. Please ask the sender to compress the document or split it into smaller files.`,
    action: "Choose Different File",
    icon: "file",
  };
}

/**
 * Get a friendly error for too many recipients
 */
export function getTooManyRecipientsError(maxRecipients: number = 10): UserError {
  return {
    title: "Too Many Recipients",
    message: `You can add up to ${maxRecipients} people to sign a document. Please remove some recipients or create separate signing requests for additional signers.`,
    action: "Edit Recipients",
    icon: "user",
  };
}

/**
 * Get a friendly error for field extending past page boundary
 */
export function getFieldOutOfBoundsError(direction: string): UserError {
  const directionText = direction === "right" || direction === "left"
    ? "side"
    : direction === "top" ? "top" : "bottom";

  return {
    title: "Field Outside Page",
    message: `This field extends past the ${directionText} of the page. Please move it or make it smaller so it fits entirely on the page.`,
    action: "Adjust Field",
    icon: "file",
  };
}

/**
 * Get a friendly error for unsupported file type
 */
export function getUnsupportedFileError(): UserError {
  return {
    title: "Unsupported File Type",
    message:
      "We can only work with PDF documents. If you received a different type of file, please ask the sender to convert it to PDF format.",
    action: "Go Back",
    icon: "file",
  };
}
