/**
 * Recipient Manager for agentPDF
 *
 * Manages recipients for document signing workflows.
 * Supports both parallel (all sign at once) and sequential (sign in order) modes.
 *
 * GERIATRIC-FRIENDLY: Large touch targets, clear visual hierarchy
 */

import type { PlacedField } from './template-editor';

// ============================================================================
// TYPES
// ============================================================================

/**
 * Recipient role in the signing process
 */
export type RecipientRole = 'signer' | 'reviewer' | 'cc';

/**
 * A document recipient
 */
export interface Recipient {
  id: string;
  name: string;
  email: string;
  role: RecipientRole;
  order: number; // Order for sequential signing (1-based)
  color: string; // Color for visual field assignment
}

/**
 * Field assignment linking a field to a recipient
 */
export interface FieldAssignment {
  fieldId: string;
  recipientId: string;
}

/**
 * Signing mode
 */
export enum SigningMode {
  Parallel = 'parallel',   // All recipients sign simultaneously
  Sequential = 'sequential', // Recipients sign in order
}

// ============================================================================
// CONSTANTS
// ============================================================================

// Geriatric-friendly colors with good contrast
const RECIPIENT_COLORS = [
  '#0066CC', // Blue
  '#CC6600', // Orange
  '#00CC66', // Green
  '#CC0066', // Magenta
  '#6600CC', // Purple
  '#00CCCC', // Teal
];

// ============================================================================
// STATE
// ============================================================================

interface RecipientState {
  recipients: Map<string, Recipient>;
  assignments: Map<string, string>; // fieldId -> recipientId
  signingMode: SigningMode;
  nextOrder: number;
}

const state: RecipientState = {
  recipients: new Map(),
  assignments: new Map(),
  signingMode: SigningMode.Parallel,
  nextOrder: 1,
};

// ============================================================================
// RECIPIENT OPERATIONS
// ============================================================================

/**
 * Generate unique recipient ID
 */
function generateRecipientId(): string {
  return `recipient-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Get next available color for a recipient
 */
function getNextColor(): string {
  const usedColors = Array.from(state.recipients.values()).map((r) => r.color);
  const availableColor = RECIPIENT_COLORS.find((c) => !usedColors.includes(c));
  return availableColor || RECIPIENT_COLORS[state.recipients.size % RECIPIENT_COLORS.length];
}

/**
 * Add a new recipient
 *
 * @param name - Recipient's full name
 * @param email - Recipient's email address
 * @param role - Role in signing process (default: 'signer')
 * @returns The created recipient
 */
export function addRecipient(
  name: string,
  email: string,
  role: RecipientRole = 'signer'
): Recipient {
  const id = generateRecipientId();
  const recipient: Recipient = {
    id,
    name: name.trim(),
    email: email.trim().toLowerCase(),
    role,
    order: state.nextOrder++,
    color: getNextColor(),
  };

  state.recipients.set(id, recipient);
  renderRecipientList();
  return recipient;
}

/**
 * Remove a recipient and their field assignments
 *
 * @param id - Recipient ID to remove
 */
export function removeRecipient(id: string): void {
  state.recipients.delete(id);

  // Remove all field assignments for this recipient
  for (const [fieldId, recipientId] of state.assignments.entries()) {
    if (recipientId === id) {
      state.assignments.delete(fieldId);
      updateFieldVisual(fieldId, null);
    }
  }

  // Reorder remaining recipients
  reorderRecipients();
  renderRecipientList();
}

/**
 * Update a recipient's information
 *
 * @param id - Recipient ID
 * @param updates - Partial recipient data to update
 */
export function updateRecipient(id: string, updates: Partial<Omit<Recipient, 'id'>>): void {
  const recipient = state.recipients.get(id);
  if (!recipient) return;

  if (updates.name !== undefined) recipient.name = updates.name.trim();
  if (updates.email !== undefined) recipient.email = updates.email.trim().toLowerCase();
  if (updates.role !== undefined) recipient.role = updates.role;
  if (updates.order !== undefined) recipient.order = updates.order;

  renderRecipientList();
}

/**
 * Reorder recipients after removal
 */
function reorderRecipients(): void {
  const sorted = Array.from(state.recipients.values()).sort((a, b) => a.order - b.order);
  sorted.forEach((r, i) => {
    r.order = i + 1;
  });
  state.nextOrder = sorted.length + 1;
}

/**
 * Move a recipient up in the signing order
 */
export function moveRecipientUp(id: string): void {
  const recipient = state.recipients.get(id);
  if (!recipient || recipient.order <= 1) return;

  // Find recipient with order - 1
  for (const r of state.recipients.values()) {
    if (r.order === recipient.order - 1) {
      r.order = recipient.order;
      recipient.order = recipient.order - 1;
      break;
    }
  }
  renderRecipientList();
}

/**
 * Move a recipient down in the signing order
 */
export function moveRecipientDown(id: string): void {
  const recipient = state.recipients.get(id);
  if (!recipient || recipient.order >= state.recipients.size) return;

  // Find recipient with order + 1
  for (const r of state.recipients.values()) {
    if (r.order === recipient.order + 1) {
      r.order = recipient.order;
      recipient.order = recipient.order + 1;
      break;
    }
  }
  renderRecipientList();
}

/**
 * Get all recipients
 */
export function getAllRecipients(): Recipient[] {
  return Array.from(state.recipients.values()).sort((a, b) => a.order - b.order);
}

/**
 * Get a recipient by ID
 */
export function getRecipient(id: string): Recipient | undefined {
  return state.recipients.get(id);
}

// ============================================================================
// FIELD ASSIGNMENT OPERATIONS
// ============================================================================

/**
 * Assign a field to a recipient
 *
 * @param fieldId - The field ID
 * @param recipientId - The recipient ID
 */
export function assignFieldToRecipient(fieldId: string, recipientId: string): void {
  const recipient = state.recipients.get(recipientId);
  if (!recipient) return;

  state.assignments.set(fieldId, recipientId);
  updateFieldVisual(fieldId, recipient);
}

/**
 * Unassign a field from any recipient
 *
 * @param fieldId - The field ID to unassign
 */
export function unassignField(fieldId: string): void {
  state.assignments.delete(fieldId);
  updateFieldVisual(fieldId, null);
}

/**
 * Get the recipient assigned to a field
 *
 * @param fieldId - The field ID
 * @returns The assigned recipient or undefined
 */
export function getFieldRecipient(fieldId: string): Recipient | undefined {
  const recipientId = state.assignments.get(fieldId);
  if (!recipientId) return undefined;
  return state.recipients.get(recipientId);
}

/**
 * Get all field assignments for a recipient
 *
 * @param recipientId - The recipient ID
 * @returns Array of field IDs assigned to this recipient
 */
export function getRecipientFields(recipientId: string): string[] {
  const fields: string[] = [];
  for (const [fieldId, rId] of state.assignments.entries()) {
    if (rId === recipientId) {
      fields.push(fieldId);
    }
  }
  return fields;
}

/**
 * Get all field assignments
 */
export function getAllAssignments(): FieldAssignment[] {
  return Array.from(state.assignments.entries()).map(([fieldId, recipientId]) => ({
    fieldId,
    recipientId,
  }));
}

/**
 * Update the visual appearance of a field based on its recipient
 */
function updateFieldVisual(fieldId: string, recipient: Recipient | null): void {
  const el = document.getElementById(fieldId);
  if (!el) return;

  if (recipient) {
    el.style.borderColor = recipient.color;
    el.style.boxShadow = `0 0 0 3px ${recipient.color}33`;

    // Add or update recipient badge
    let badge = el.querySelector('.recipient-badge') as HTMLElement;
    if (!badge) {
      badge = document.createElement('div');
      badge.className = 'recipient-badge';
      badge.style.position = 'absolute';
      badge.style.top = '-24px';
      badge.style.left = '0';
      badge.style.fontSize = '12px';
      badge.style.fontWeight = 'bold';
      badge.style.padding = '2px 6px';
      badge.style.borderRadius = '4px';
      badge.style.whiteSpace = 'nowrap';
      el.appendChild(badge);
    }
    badge.textContent = recipient.name;
    badge.style.backgroundColor = recipient.color;
    badge.style.color = '#FFFFFF';
  } else {
    el.style.borderColor = '#0066cc';
    el.style.boxShadow = 'none';

    // Remove recipient badge
    const badge = el.querySelector('.recipient-badge');
    if (badge) badge.remove();
  }
}

// ============================================================================
// SIGNING MODE
// ============================================================================

/**
 * Set the signing mode
 *
 * @param mode - Parallel or Sequential
 */
export function setSigningMode(mode: SigningMode): void {
  state.signingMode = mode;
  renderRecipientList();
}

/**
 * Get the current signing mode
 */
export function getSigningMode(): SigningMode {
  return state.signingMode;
}

// ============================================================================
// UI RENDERING
// ============================================================================

/**
 * Render the recipient list UI
 * Geriatric-friendly: 60px buttons, 18px fonts, clear labels
 */
export function renderRecipientList(): void {
  const container = document.getElementById('recipient-list');
  if (!container) return;

  container.innerHTML = '';

  // Header with signing mode toggle
  const header = document.createElement('div');
  header.className = 'recipient-header';
  header.style.marginBottom = '20px';
  header.innerHTML = `
    <h3 style="font-size: 20px; margin: 0 0 12px 0; color: #333;">Recipients</h3>
    <div style="display: flex; gap: 12px; margin-bottom: 16px;">
      <button id="mode-parallel" class="mode-btn ${state.signingMode === SigningMode.Parallel ? 'active' : ''}"
              style="flex: 1; height: 48px; font-size: 16px; border-radius: 8px; border: 2px solid #0066CC; cursor: pointer;
                     background: ${state.signingMode === SigningMode.Parallel ? '#0066CC' : '#fff'};
                     color: ${state.signingMode === SigningMode.Parallel ? '#fff' : '#0066CC'};">
        All at Once
      </button>
      <button id="mode-sequential" class="mode-btn ${state.signingMode === SigningMode.Sequential ? 'active' : ''}"
              style="flex: 1; height: 48px; font-size: 16px; border-radius: 8px; border: 2px solid #0066CC; cursor: pointer;
                     background: ${state.signingMode === SigningMode.Sequential ? '#0066CC' : '#fff'};
                     color: ${state.signingMode === SigningMode.Sequential ? '#fff' : '#0066CC'};">
        In Order
      </button>
    </div>
  `;
  container.appendChild(header);

  // Add event listeners for mode buttons
  header.querySelector('#mode-parallel')?.addEventListener('click', () => setSigningMode(SigningMode.Parallel));
  header.querySelector('#mode-sequential')?.addEventListener('click', () => setSigningMode(SigningMode.Sequential));

  // Recipient list
  const recipients = getAllRecipients();

  if (recipients.length === 0) {
    const emptyMsg = document.createElement('p');
    emptyMsg.style.cssText = 'font-size: 16px; color: #666; text-align: center; padding: 20px;';
    emptyMsg.textContent = 'No recipients added yet';
    container.appendChild(emptyMsg);
  } else {
    recipients.forEach((recipient) => {
      const row = createRecipientRow(recipient);
      container.appendChild(row);
    });
  }

  // Add recipient button (geriatric-friendly: 60px height)
  const addBtn = document.createElement('button');
  addBtn.id = 'add-recipient-btn';
  addBtn.className = 'add-recipient-btn';
  addBtn.style.cssText = `
    width: 100%;
    height: 60px;
    font-size: 18px;
    font-weight: bold;
    border: 2px dashed #0066CC;
    border-radius: 12px;
    background: #f0f8ff;
    color: #0066CC;
    cursor: pointer;
    margin-top: 16px;
  `;
  addBtn.textContent = '+ Add Recipient';
  addBtn.addEventListener('click', showAddRecipientForm);
  container.appendChild(addBtn);
}

/**
 * Create a recipient row element
 */
function createRecipientRow(recipient: Recipient): HTMLElement {
  const row = document.createElement('div');
  row.className = 'recipient-row';
  row.dataset.recipientId = recipient.id;
  row.style.cssText = `
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    margin-bottom: 12px;
    background: #fff;
    border: 2px solid ${recipient.color};
    border-radius: 12px;
    border-left: 8px solid ${recipient.color};
  `;

  // Order number (shown in sequential mode)
  if (state.signingMode === SigningMode.Sequential) {
    const orderNum = document.createElement('div');
    orderNum.className = 'recipient-order';
    orderNum.style.cssText = `
      width: 36px;
      height: 36px;
      border-radius: 50%;
      background: ${recipient.color};
      color: #fff;
      font-size: 18px;
      font-weight: bold;
      display: flex;
      align-items: center;
      justify-content: center;
    `;
    orderNum.textContent = recipient.order.toString();
    row.appendChild(orderNum);
  }

  // Recipient info
  const info = document.createElement('div');
  info.className = 'recipient-info';
  info.style.cssText = 'flex: 1; min-width: 0;';
  info.innerHTML = `
    <div style="font-size: 18px; font-weight: bold; color: #333; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
      ${escapeHtml(recipient.name)}
    </div>
    <div style="font-size: 14px; color: #666; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
      ${escapeHtml(recipient.email)}
    </div>
    <div style="font-size: 12px; color: #999; margin-top: 4px;">
      ${getRecipientFields(recipient.id).length} fields assigned
    </div>
  `;
  row.appendChild(info);

  // Action buttons
  const actions = document.createElement('div');
  actions.className = 'recipient-actions';
  actions.style.cssText = 'display: flex; gap: 8px; flex-shrink: 0;';

  // Order buttons (sequential mode only)
  if (state.signingMode === SigningMode.Sequential) {
    const upBtn = createActionButton('\u25B2', 'Move Up', () => moveRecipientUp(recipient.id));
    upBtn.disabled = recipient.order <= 1;
    actions.appendChild(upBtn);

    const downBtn = createActionButton('\u25BC', 'Move Down', () => moveRecipientDown(recipient.id));
    downBtn.disabled = recipient.order >= state.recipients.size;
    actions.appendChild(downBtn);
  }

  // Delete button
  const deleteBtn = createActionButton('\u00D7', 'Remove', () => {
    if (confirm(`Remove ${recipient.name} from recipients?`)) {
      removeRecipient(recipient.id);
    }
  });
  deleteBtn.style.backgroundColor = '#ffebeb';
  deleteBtn.style.color = '#cc0000';
  deleteBtn.style.borderColor = '#cc0000';
  actions.appendChild(deleteBtn);

  row.appendChild(actions);
  return row;
}

/**
 * Create an action button with geriatric-friendly sizing
 */
function createActionButton(text: string, title: string, onClick: () => void): HTMLButtonElement {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.title = title;
  btn.textContent = text;
  btn.style.cssText = `
    width: 44px;
    height: 44px;
    font-size: 18px;
    border: 2px solid #ccc;
    border-radius: 8px;
    background: #fff;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  `;
  btn.addEventListener('click', onClick);
  return btn;
}

/**
 * Show the add recipient form
 */
function showAddRecipientForm(): void {
  const container = document.getElementById('recipient-list');
  if (!container) return;

  // Hide add button
  const addBtn = container.querySelector('#add-recipient-btn') as HTMLElement;
  if (addBtn) addBtn.style.display = 'none';

  // Show form
  const form = document.createElement('div');
  form.id = 'add-recipient-form';
  form.className = 'add-recipient-form';
  form.style.cssText = `
    padding: 20px;
    background: #f0f8ff;
    border: 2px solid #0066CC;
    border-radius: 12px;
    margin-top: 16px;
  `;
  form.innerHTML = `
    <h4 style="font-size: 18px; margin: 0 0 16px 0; color: #333;">Add New Recipient</h4>
    <div style="margin-bottom: 16px;">
      <label style="font-size: 16px; display: block; margin-bottom: 8px; color: #333;">Full Name</label>
      <input type="text" id="new-recipient-name" placeholder="John Smith"
             style="width: 100%; height: 48px; font-size: 18px; padding: 8px 12px; border: 2px solid #ccc; border-radius: 8px; box-sizing: border-box;">
    </div>
    <div style="margin-bottom: 16px;">
      <label style="font-size: 16px; display: block; margin-bottom: 8px; color: #333;">Email Address</label>
      <input type="email" id="new-recipient-email" placeholder="john@example.com"
             style="width: 100%; height: 48px; font-size: 18px; padding: 8px 12px; border: 2px solid #ccc; border-radius: 8px; box-sizing: border-box;">
    </div>
    <div style="margin-bottom: 20px;">
      <label style="font-size: 16px; display: block; margin-bottom: 8px; color: #333;">Role</label>
      <select id="new-recipient-role"
              style="width: 100%; height: 48px; font-size: 18px; padding: 8px 12px; border: 2px solid #ccc; border-radius: 8px; box-sizing: border-box;">
        <option value="signer">Signer - Must sign the document</option>
        <option value="reviewer">Reviewer - Views and approves</option>
        <option value="cc">CC - Receives copy only</option>
      </select>
    </div>
    <div style="display: flex; gap: 12px;">
      <button id="save-recipient-btn" type="button"
              style="flex: 1; height: 60px; font-size: 18px; font-weight: bold; background: #0066CC; color: #fff; border: none; border-radius: 12px; cursor: pointer;">
        Add Recipient
      </button>
      <button id="cancel-recipient-btn" type="button"
              style="width: 100px; height: 60px; font-size: 16px; background: #fff; color: #666; border: 2px solid #ccc; border-radius: 12px; cursor: pointer;">
        Cancel
      </button>
    </div>
  `;

  container.appendChild(form);

  // Focus name input
  const nameInput = document.getElementById('new-recipient-name') as HTMLInputElement;
  nameInput?.focus();

  // Event listeners
  document.getElementById('save-recipient-btn')?.addEventListener('click', () => {
    const name = (document.getElementById('new-recipient-name') as HTMLInputElement).value;
    const email = (document.getElementById('new-recipient-email') as HTMLInputElement).value;
    const role = (document.getElementById('new-recipient-role') as HTMLSelectElement).value as RecipientRole;

    if (!name.trim()) {
      alert('Please enter a name');
      return;
    }
    if (!email.trim() || !isValidEmail(email)) {
      alert('Please enter a valid email address');
      return;
    }

    addRecipient(name, email, role);
    form.remove();
  });

  document.getElementById('cancel-recipient-btn')?.addEventListener('click', () => {
    form.remove();
    if (addBtn) addBtn.style.display = 'block';
  });
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
 * Basic email validation
 */
function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

// ============================================================================
// EXPORT / IMPORT
// ============================================================================

/**
 * Export recipient data for saving/dispatch
 */
export function exportRecipientData(): {
  recipients: Recipient[];
  assignments: FieldAssignment[];
  signingMode: SigningMode;
} {
  return {
    recipients: getAllRecipients(),
    assignments: getAllAssignments(),
    signingMode: state.signingMode,
  };
}

/**
 * Import recipient data
 */
export function importRecipientData(data: {
  recipients: Recipient[];
  assignments: FieldAssignment[];
  signingMode: SigningMode;
}): void {
  // Clear current state
  state.recipients.clear();
  state.assignments.clear();

  // Import recipients
  data.recipients.forEach((r) => {
    state.recipients.set(r.id, r);
  });
  state.nextOrder = data.recipients.length + 1;

  // Import assignments
  data.assignments.forEach((a) => {
    state.assignments.set(a.fieldId, a.recipientId);
    const recipient = state.recipients.get(a.recipientId);
    if (recipient) {
      updateFieldVisual(a.fieldId, recipient);
    }
  });

  // Import signing mode
  state.signingMode = data.signingMode;

  renderRecipientList();
}

/**
 * Clear all recipient data
 */
export function clearAllRecipients(): void {
  // Clear field visuals
  for (const fieldId of state.assignments.keys()) {
    updateFieldVisual(fieldId, null);
  }

  state.recipients.clear();
  state.assignments.clear();
  state.nextOrder = 1;
  renderRecipientList();
}

// ============================================================================
// PUBLIC API
// ============================================================================

export const RecipientManager = {
  // Recipient operations
  addRecipient,
  removeRecipient,
  updateRecipient,
  getRecipient,
  getAllRecipients,
  moveRecipientUp,
  moveRecipientDown,

  // Field assignments
  assignFieldToRecipient,
  unassignField,
  getFieldRecipient,
  getRecipientFields,
  getAllAssignments,

  // Signing mode
  setSigningMode,
  getSigningMode,
  SigningMode,

  // UI
  renderRecipientList,

  // Import/Export
  exportRecipientData,
  importRecipientData,
  clearAllRecipients,
};

// Expose on window for use from HTML (browser only)
if (typeof window !== 'undefined') {
  (window as unknown as { RecipientManager: typeof RecipientManager }).RecipientManager = RecipientManager;
}
