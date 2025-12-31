/**
 * Dispatch Modal for agentPDF
 *
 * Modal for "Send for Signatures" flow.
 * Lists recipients with their assigned fields and provides send functionality.
 *
 * GERIATRIC-FRIENDLY:
 * - 60px minimum button heights
 * - 18px minimum font sizes
 * - High contrast colors
 * - Clear visual hierarchy
 * - Large touch targets
 */

import {
  RecipientManager,
  SigningMode,
  type Recipient,
  type FieldAssignment,
} from './recipient-manager';
import { getAllFields, type PlacedField } from './template-editor';

// ============================================================================
// TYPES
// ============================================================================

/**
 * Dispatch status for tracking send progress
 */
export type DispatchStatus = 'pending' | 'sending' | 'sent' | 'error';

/**
 * Dispatch result from email API
 */
export interface DispatchResult {
  success: boolean;
  recipientId: string;
  email: string;
  error?: string;
  timestamp?: Date;
}

/**
 * Email API request payload
 */
export interface EmailPayload {
  to: string;
  recipientName: string;
  documentName: string;
  signingUrl: string;
  order: number;
  isSequential: boolean;
  previousSigners?: string[];
}

// ============================================================================
// CONSTANTS (Geriatric-friendly sizing)
// ============================================================================

const BUTTON_HEIGHT = 60;
const BUTTON_FONT_SIZE = 18;
const LABEL_FONT_SIZE = 18;
const BODY_FONT_SIZE = 16;
const BORDER_RADIUS = 12;

// ============================================================================
// STATE
// ============================================================================

interface ModalState {
  isOpen: boolean;
  modalElement: HTMLElement | null;
  dispatchStatus: Map<string, DispatchStatus>;
  documentName: string;
  pdfBytes: Uint8Array | null;
  onSendCallback: ((payload: EmailPayload) => Promise<boolean>) | null;
}

const state: ModalState = {
  isOpen: false,
  modalElement: null,
  dispatchStatus: new Map(),
  documentName: 'Document',
  pdfBytes: null,
  onSendCallback: null,
};

// ============================================================================
// MODAL LIFECYCLE
// ============================================================================

/**
 * Open the dispatch modal
 *
 * @param documentName - Name of the document being sent
 * @param pdfBytes - The PDF bytes to send (optional, for preview)
 */
export function openDispatchModal(documentName?: string, pdfBytes?: Uint8Array): void {
  if (state.isOpen) return;

  if (documentName) state.documentName = documentName;
  if (pdfBytes) state.pdfBytes = pdfBytes;

  // Reset dispatch status
  state.dispatchStatus.clear();

  createModalElement();
  state.isOpen = true;
  document.body.style.overflow = 'hidden';

  // Focus the modal for accessibility
  state.modalElement?.focus();
}

/**
 * Close the dispatch modal
 */
export function closeDispatchModal(): void {
  if (!state.isOpen || !state.modalElement) return;

  state.modalElement.remove();
  state.modalElement = null;
  state.isOpen = false;
  document.body.style.overflow = '';
}

/**
 * Check if the modal is currently open
 */
export function isModalOpen(): boolean {
  return state.isOpen;
}

/**
 * Set the callback function for sending emails
 * This allows integration with any email API
 */
export function setEmailCallback(callback: (payload: EmailPayload) => Promise<boolean>): void {
  state.onSendCallback = callback;
}

// ============================================================================
// MODAL CREATION
// ============================================================================

/**
 * Create the modal DOM element
 */
function createModalElement(): void {
  // Overlay
  const overlay = document.createElement('div');
  overlay.id = 'dispatch-modal-overlay';
  overlay.style.cssText = `
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    padding: 20px;
  `;

  // Close on overlay click
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) {
      closeDispatchModal();
    }
  });

  // Modal container
  const modal = document.createElement('div');
  modal.id = 'dispatch-modal';
  modal.setAttribute('role', 'dialog');
  modal.setAttribute('aria-labelledby', 'dispatch-modal-title');
  modal.setAttribute('tabindex', '-1');
  modal.style.cssText = `
    background: #fff;
    border-radius: ${BORDER_RADIUS}px;
    max-width: 600px;
    width: 100%;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  `;

  // Build modal content
  modal.appendChild(createModalHeader());
  modal.appendChild(createModalBody());
  modal.appendChild(createModalFooter());

  overlay.appendChild(modal);
  document.body.appendChild(overlay);
  state.modalElement = overlay;

  // Keyboard handling
  overlay.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeDispatchModal();
    }
  });
}

/**
 * Create modal header
 */
function createModalHeader(): HTMLElement {
  const header = document.createElement('div');
  header.className = 'dispatch-modal-header';
  header.style.cssText = `
    padding: 24px 24px 16px 24px;
    border-bottom: 2px solid #eee;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  `;

  // Title and document name
  const titleGroup = document.createElement('div');
  titleGroup.innerHTML = `
    <h2 id="dispatch-modal-title" style="margin: 0 0 8px 0; font-size: 24px; color: #333;">
      Send for Signatures
    </h2>
    <p style="margin: 0; font-size: ${BODY_FONT_SIZE}px; color: #666;">
      ${escapeHtml(state.documentName)}
    </p>
  `;
  header.appendChild(titleGroup);

  // Close button (large touch target)
  const closeBtn = document.createElement('button');
  closeBtn.type = 'button';
  closeBtn.setAttribute('aria-label', 'Close modal');
  closeBtn.style.cssText = `
    width: 48px;
    height: 48px;
    font-size: 28px;
    border: none;
    background: #f5f5f5;
    border-radius: 50%;
    cursor: pointer;
    color: #666;
    display: flex;
    align-items: center;
    justify-content: center;
  `;
  closeBtn.textContent = '\u00D7';
  closeBtn.addEventListener('click', closeDispatchModal);
  header.appendChild(closeBtn);

  return header;
}

/**
 * Create modal body with recipient list
 */
function createModalBody(): HTMLElement {
  const body = document.createElement('div');
  body.className = 'dispatch-modal-body';
  body.style.cssText = 'padding: 24px;';

  const recipients = RecipientManager.getAllRecipients();
  const signingMode = RecipientManager.getSigningMode();
  const fields = getAllFields();

  // Signing mode indicator
  const modeIndicator = document.createElement('div');
  modeIndicator.style.cssText = `
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: ${signingMode === SigningMode.Sequential ? '#fff8e6' : '#e6f4ff'};
    border-radius: ${BORDER_RADIUS}px;
    margin-bottom: 20px;
  `;
  modeIndicator.innerHTML = `
    <span style="font-size: 28px;">${signingMode === SigningMode.Sequential ? '\u23F3' : '\u2B1C'}</span>
    <div>
      <div style="font-size: ${LABEL_FONT_SIZE}px; font-weight: bold; color: #333;">
        ${signingMode === SigningMode.Sequential ? 'Sequential Signing' : 'Parallel Signing'}
      </div>
      <div style="font-size: 14px; color: #666;">
        ${signingMode === SigningMode.Sequential
          ? 'Recipients will sign one after another in order'
          : 'All recipients will receive the document at the same time'}
      </div>
    </div>
  `;
  body.appendChild(modeIndicator);

  // Validation warnings
  const warnings = validateDispatch(recipients, fields);
  if (warnings.length > 0) {
    const warningBox = document.createElement('div');
    warningBox.style.cssText = `
      padding: 16px;
      background: #fff3e6;
      border: 2px solid #ff9800;
      border-radius: ${BORDER_RADIUS}px;
      margin-bottom: 20px;
    `;
    warningBox.innerHTML = `
      <div style="font-size: ${LABEL_FONT_SIZE}px; font-weight: bold; color: #e65100; margin-bottom: 8px;">
        Please Review
      </div>
      <ul style="margin: 0; padding-left: 20px; font-size: ${BODY_FONT_SIZE}px; color: #333;">
        ${warnings.map((w) => `<li>${escapeHtml(w)}</li>`).join('')}
      </ul>
    `;
    body.appendChild(warningBox);
  }

  // Recipient list heading
  const heading = document.createElement('h3');
  heading.style.cssText = `font-size: ${LABEL_FONT_SIZE}px; margin: 0 0 16px 0; color: #333;`;
  heading.textContent = `Recipients (${recipients.length})`;
  body.appendChild(heading);

  // Empty state
  if (recipients.length === 0) {
    const emptyState = document.createElement('div');
    emptyState.style.cssText = `
      text-align: center;
      padding: 40px 20px;
      background: #f9f9f9;
      border-radius: ${BORDER_RADIUS}px;
    `;
    emptyState.innerHTML = `
      <div style="font-size: 48px; margin-bottom: 16px;">
        <span role="img" aria-label="No recipients">&#128100;</span>
      </div>
      <div style="font-size: ${LABEL_FONT_SIZE}px; color: #666;">
        No recipients added yet
      </div>
      <div style="font-size: 14px; color: #999; margin-top: 8px;">
        Add recipients before sending the document
      </div>
    `;
    body.appendChild(emptyState);
    return body;
  }

  // Recipient cards
  const recipientList = document.createElement('div');
  recipientList.className = 'dispatch-recipient-list';
  recipientList.style.cssText = 'display: flex; flex-direction: column; gap: 12px;';

  recipients.forEach((recipient) => {
    const card = createRecipientCard(recipient, fields);
    recipientList.appendChild(card);
  });

  body.appendChild(recipientList);

  return body;
}

/**
 * Create a recipient card for the modal
 */
function createRecipientCard(recipient: Recipient, allFields: PlacedField[]): HTMLElement {
  const assignedFieldIds = RecipientManager.getRecipientFields(recipient.id);
  const assignedFields = allFields.filter((f) => assignedFieldIds.includes(f.id));
  const signingMode = RecipientManager.getSigningMode();
  const status = state.dispatchStatus.get(recipient.id) || 'pending';

  const card = document.createElement('div');
  card.className = 'dispatch-recipient-card';
  card.dataset.recipientId = recipient.id;
  card.style.cssText = `
    padding: 20px;
    background: #fff;
    border: 2px solid ${recipient.color};
    border-left: 8px solid ${recipient.color};
    border-radius: ${BORDER_RADIUS}px;
    position: relative;
  `;

  // Status indicator
  const statusBadge = document.createElement('div');
  statusBadge.className = 'dispatch-status-badge';
  statusBadge.style.cssText = `
    position: absolute;
    top: 12px;
    right: 12px;
    padding: 6px 12px;
    border-radius: 20px;
    font-size: 14px;
    font-weight: bold;
    ${getStatusStyle(status)}
  `;
  statusBadge.textContent = getStatusLabel(status);
  card.appendChild(statusBadge);

  // Recipient info
  const infoSection = document.createElement('div');
  infoSection.style.cssText = 'padding-right: 100px;';

  // Order badge (sequential mode)
  let orderHtml = '';
  if (signingMode === SigningMode.Sequential) {
    orderHtml = `
      <span style="
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 32px;
        height: 32px;
        background: ${recipient.color};
        color: #fff;
        border-radius: 50%;
        font-weight: bold;
        font-size: 16px;
        margin-right: 12px;
      ">${recipient.order}</span>
    `;
  }

  infoSection.innerHTML = `
    <div style="display: flex; align-items: center; margin-bottom: 8px;">
      ${orderHtml}
      <span style="font-size: ${LABEL_FONT_SIZE}px; font-weight: bold; color: #333;">
        ${escapeHtml(recipient.name)}
      </span>
      <span style="margin-left: 12px; font-size: 14px; color: #666; background: #f0f0f0; padding: 2px 8px; border-radius: 4px;">
        ${getRoleLabel(recipient.role)}
      </span>
    </div>
    <div style="font-size: ${BODY_FONT_SIZE}px; color: #666; margin-bottom: 12px;">
      ${escapeHtml(recipient.email)}
    </div>
  `;
  card.appendChild(infoSection);

  // Assigned fields summary
  const fieldsSection = document.createElement('div');
  fieldsSection.style.cssText = `
    padding-top: 12px;
    border-top: 1px solid #eee;
  `;

  if (assignedFields.length === 0) {
    fieldsSection.innerHTML = `
      <div style="font-size: 14px; color: #999; font-style: italic;">
        No fields assigned to this recipient
      </div>
    `;
  } else {
    const fieldsSummary = summarizeFields(assignedFields);
    fieldsSection.innerHTML = `
      <div style="font-size: 14px; color: #333; font-weight: bold; margin-bottom: 8px;">
        ${assignedFields.length} field${assignedFields.length !== 1 ? 's' : ''} to complete:
      </div>
      <div style="display: flex; flex-wrap: wrap; gap: 8px;">
        ${fieldsSummary.map((fs) => `
          <span style="
            padding: 4px 10px;
            background: ${recipient.color}20;
            color: ${recipient.color};
            border-radius: 4px;
            font-size: 13px;
            font-weight: 500;
          ">${fs.count} ${fs.label}</span>
        `).join('')}
      </div>
    `;
  }
  card.appendChild(fieldsSection);

  return card;
}

/**
 * Create modal footer with action buttons
 */
function createModalFooter(): HTMLElement {
  const footer = document.createElement('div');
  footer.className = 'dispatch-modal-footer';
  footer.style.cssText = `
    padding: 20px 24px 24px 24px;
    border-top: 2px solid #eee;
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  `;

  const recipients = RecipientManager.getAllRecipients();
  const fields = getAllFields();
  const canSend = recipients.length > 0 && validateDispatch(recipients, fields).length === 0;

  // Cancel button
  const cancelBtn = document.createElement('button');
  cancelBtn.type = 'button';
  cancelBtn.style.cssText = `
    flex: 1;
    min-width: 140px;
    height: ${BUTTON_HEIGHT}px;
    font-size: ${BUTTON_FONT_SIZE}px;
    font-weight: bold;
    background: #fff;
    color: #666;
    border: 2px solid #ccc;
    border-radius: ${BORDER_RADIUS}px;
    cursor: pointer;
  `;
  cancelBtn.textContent = 'Cancel';
  cancelBtn.addEventListener('click', closeDispatchModal);
  footer.appendChild(cancelBtn);

  // Send button
  const sendBtn = document.createElement('button');
  sendBtn.type = 'button';
  sendBtn.id = 'dispatch-send-btn';
  sendBtn.disabled = !canSend;
  sendBtn.style.cssText = `
    flex: 2;
    min-width: 200px;
    height: ${BUTTON_HEIGHT}px;
    font-size: ${BUTTON_FONT_SIZE}px;
    font-weight: bold;
    background: ${canSend ? '#0066CC' : '#ccc'};
    color: #fff;
    border: none;
    border-radius: ${BORDER_RADIUS}px;
    cursor: ${canSend ? 'pointer' : 'not-allowed'};
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
  `;
  sendBtn.innerHTML = `
    <span style="font-size: 24px;">&#9993;</span>
    Send to ${recipients.length} Recipient${recipients.length !== 1 ? 's' : ''}
  `;

  if (canSend) {
    sendBtn.addEventListener('click', handleSend);
  }

  footer.appendChild(sendBtn);

  return footer;
}

// ============================================================================
// DISPATCH LOGIC
// ============================================================================

/**
 * Handle the send action
 */
async function handleSend(): Promise<void> {
  const sendBtn = document.getElementById('dispatch-send-btn') as HTMLButtonElement;
  if (!sendBtn) return;

  const recipients = RecipientManager.getAllRecipients();
  const signingMode = RecipientManager.getSigningMode();

  // Update button to show sending state
  sendBtn.disabled = true;
  sendBtn.innerHTML = `
    <span class="spinner" style="
      width: 24px;
      height: 24px;
      border: 3px solid #ffffff40;
      border-top-color: #fff;
      border-radius: 50%;
      animation: spin 1s linear infinite;
    "></span>
    Sending...
  `;

  // Add spinner animation
  if (!document.getElementById('dispatch-spinner-style')) {
    const style = document.createElement('style');
    style.id = 'dispatch-spinner-style';
    style.textContent = '@keyframes spin { to { transform: rotate(360deg); } }';
    document.head.appendChild(style);
  }

  const results: DispatchResult[] = [];

  for (const recipient of recipients) {
    state.dispatchStatus.set(recipient.id, 'sending');
    updateRecipientCardStatus(recipient.id, 'sending');

    try {
      const payload: EmailPayload = {
        to: recipient.email,
        recipientName: recipient.name,
        documentName: state.documentName,
        signingUrl: generateSigningUrl(recipient.id),
        order: recipient.order,
        isSequential: signingMode === SigningMode.Sequential,
        previousSigners: signingMode === SigningMode.Sequential
          ? recipients.filter((r) => r.order < recipient.order).map((r) => r.name)
          : undefined,
      };

      let success = false;

      if (state.onSendCallback) {
        success = await state.onSendCallback(payload);
      } else {
        // Simulate sending (for demo/testing)
        await simulateSend();
        success = true;
      }

      state.dispatchStatus.set(recipient.id, success ? 'sent' : 'error');
      updateRecipientCardStatus(recipient.id, success ? 'sent' : 'error');

      results.push({
        success,
        recipientId: recipient.id,
        email: recipient.email,
        timestamp: new Date(),
      });
    } catch (err) {
      state.dispatchStatus.set(recipient.id, 'error');
      updateRecipientCardStatus(recipient.id, 'error');

      results.push({
        success: false,
        recipientId: recipient.id,
        email: recipient.email,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  // Update button based on results
  const allSuccess = results.every((r) => r.success);
  const someSuccess = results.some((r) => r.success);

  sendBtn.style.background = allSuccess ? '#00aa00' : (someSuccess ? '#ff9800' : '#cc0000');
  sendBtn.innerHTML = allSuccess
    ? '<span style="font-size: 24px;">&#10004;</span> Sent Successfully!'
    : `<span style="font-size: 24px;">&#9888;</span> ${results.filter((r) => r.success).length} of ${results.length} Sent`;

  // Enable close button behavior
  sendBtn.disabled = false;
  sendBtn.addEventListener('click', closeDispatchModal);
}

/**
 * Update a recipient card's status display
 */
function updateRecipientCardStatus(recipientId: string, status: DispatchStatus): void {
  const card = document.querySelector(`[data-recipient-id="${recipientId}"]`);
  if (!card) return;

  const badge = card.querySelector('.dispatch-status-badge') as HTMLElement;
  if (badge) {
    badge.style.cssText = `
      position: absolute;
      top: 12px;
      right: 12px;
      padding: 6px 12px;
      border-radius: 20px;
      font-size: 14px;
      font-weight: bold;
      ${getStatusStyle(status)}
    `;
    badge.textContent = getStatusLabel(status);
  }
}

/**
 * Simulate sending (for demo/testing)
 */
function simulateSend(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 500 + Math.random() * 1000));
}

/**
 * Generate a signing URL for a recipient
 */
function generateSigningUrl(recipientId: string): string {
  // This would be replaced with actual URL generation logic
  const baseUrl = window.location.origin;
  const documentId = crypto.randomUUID ? crypto.randomUUID() : Date.now().toString(36);
  return `${baseUrl}/sign/${documentId}/${recipientId}`;
}

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Validate the dispatch configuration
 */
function validateDispatch(recipients: Recipient[], fields: PlacedField[]): string[] {
  const warnings: string[] = [];

  // Check for recipients
  if (recipients.length === 0) {
    warnings.push('Add at least one recipient');
    return warnings;
  }

  // Check for signers
  const signers = recipients.filter((r) => r.role === 'signer');
  if (signers.length === 0) {
    warnings.push('Add at least one signer (not just reviewers or CC)');
  }

  // Check for signature fields
  const signatureFields = fields.filter((f) => f.type === 'signature');
  if (signatureFields.length === 0) {
    warnings.push('Add at least one signature field to the document');
  }

  // Check each signer has at least one signature field
  for (const signer of signers) {
    const signerFields = RecipientManager.getRecipientFields(signer.id);
    const signerSignatures = fields.filter(
      (f) => signerFields.includes(f.id) && f.type === 'signature'
    );
    if (signerSignatures.length === 0) {
      warnings.push(`${signer.name} has no signature fields assigned`);
    }
  }

  // Check for unassigned signature fields
  const assignments = RecipientManager.getAllAssignments();
  const assignedFieldIds = new Set(assignments.map((a) => a.fieldId));
  const unassignedSignatures = signatureFields.filter((f) => !assignedFieldIds.has(f.id));
  if (unassignedSignatures.length > 0) {
    warnings.push(`${unassignedSignatures.length} signature field(s) not assigned to any recipient`);
  }

  return warnings;
}

// ============================================================================
// UTILITIES
// ============================================================================

/**
 * Escape HTML to prevent XSS
 */
function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

/**
 * Get status label text
 */
function getStatusLabel(status: DispatchStatus): string {
  switch (status) {
    case 'pending': return 'Pending';
    case 'sending': return 'Sending...';
    case 'sent': return 'Sent';
    case 'error': return 'Failed';
  }
}

/**
 * Get status badge style
 */
function getStatusStyle(status: DispatchStatus): string {
  switch (status) {
    case 'pending':
      return 'background: #f0f0f0; color: #666;';
    case 'sending':
      return 'background: #e6f4ff; color: #0066CC;';
    case 'sent':
      return 'background: #e6ffe6; color: #00aa00;';
    case 'error':
      return 'background: #ffe6e6; color: #cc0000;';
  }
}

/**
 * Get human-readable role label
 */
function getRoleLabel(role: string): string {
  switch (role) {
    case 'signer': return 'Signer';
    case 'reviewer': return 'Reviewer';
    case 'cc': return 'CC';
    default: return role;
  }
}

/**
 * Summarize field types for display
 */
function summarizeFields(fields: PlacedField[]): { label: string; count: number }[] {
  const counts: Record<string, number> = {};

  for (const field of fields) {
    const label = getFieldTypeLabel(field.type);
    counts[label] = (counts[label] || 0) + 1;
  }

  return Object.entries(counts).map(([label, count]) => ({ label, count }));
}

/**
 * Get human-readable field type label
 */
function getFieldTypeLabel(type: string): string {
  switch (type) {
    case 'signature': return 'Signature';
    case 'initials': return 'Initials';
    case 'text': return 'Text';
    case 'date': return 'Date';
    case 'checkbox': return 'Checkbox';
    default: return type;
  }
}

// ============================================================================
// PUBLIC API
// ============================================================================

export const DispatchModal = {
  // Modal lifecycle
  open: openDispatchModal,
  close: closeDispatchModal,
  isOpen: isModalOpen,

  // Configuration
  setEmailCallback,

  // Types
  DispatchStatus: null as unknown as DispatchStatus, // Type export
};

// Expose on window for use from HTML (browser only)
if (typeof window !== 'undefined') {
  (window as unknown as { DispatchModal: typeof DispatchModal }).DispatchModal = DispatchModal;
}
