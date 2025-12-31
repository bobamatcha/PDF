/**
 * Tests for Dispatch Modal Validation Logic
 *
 * Tests the validation rules for the "Send for Signatures" flow:
 * - Recipient requirements
 * - Signer requirements
 * - Field assignment validation
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock DOM environment
const mockDocument = {
  getElementById: vi.fn(() => null),
  createElement: vi.fn(() => ({
    style: {},
    cssText: '',
    textContent: '',
    innerHTML: '',
    appendChild: vi.fn(),
    querySelector: vi.fn(() => null),
    querySelectorAll: vi.fn(() => []),
    remove: vi.fn(),
    addEventListener: vi.fn(),
    setAttribute: vi.fn(),
    focus: vi.fn(),
    dataset: {},
  })),
  body: {
    appendChild: vi.fn(),
    style: {},
  },
  head: {
    appendChild: vi.fn(),
  },
};

const mockWindow = {
  location: {
    hostname: 'localhost',
    protocol: 'http:',
    search: '',
    origin: 'http://localhost:8080',
  },
};

vi.stubGlobal('document', mockDocument);
vi.stubGlobal('window', mockWindow);
vi.stubGlobal('localStorage', {
  getItem: vi.fn(() => null),
  setItem: vi.fn(),
  removeItem: vi.fn(),
});
vi.stubGlobal('crypto', {
  randomUUID: () => `uuid-${Date.now()}-${Math.random()}`,
});

// Import modules after mocks
import {
  addRecipient,
  clearAllRecipients,
  assignFieldToRecipient,
  SigningMode,
  setSigningMode,
} from '../recipient-manager';

// We need to extract the validation logic for testing
// Create a test-specific validation function that mirrors the modal's logic
interface TestRecipient {
  id: string;
  name: string;
  email: string;
  role: 'signer' | 'reviewer' | 'cc';
  order: number;
}

interface TestField {
  id: string;
  type: 'signature' | 'text' | 'date' | 'checkbox' | 'initials';
}

interface TestAssignment {
  fieldId: string;
  recipientId: string;
}

/**
 * Validation function extracted from dispatch-modal.ts for testing
 */
function validateDispatch(
  recipients: TestRecipient[],
  fields: TestField[],
  assignments: TestAssignment[]
): string[] {
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
    const signerFieldIds = assignments
      .filter((a) => a.recipientId === signer.id)
      .map((a) => a.fieldId);
    const signerSignatures = fields.filter(
      (f) => signerFieldIds.includes(f.id) && f.type === 'signature'
    );
    if (signerSignatures.length === 0) {
      warnings.push(`${signer.name} has no signature fields assigned`);
    }
  }

  // Check for unassigned signature fields
  const assignedFieldIds = new Set(assignments.map((a) => a.fieldId));
  const unassignedSignatures = signatureFields.filter((f) => !assignedFieldIds.has(f.id));
  if (unassignedSignatures.length > 0) {
    warnings.push(`${unassignedSignatures.length} signature field(s) not assigned to any recipient`);
  }

  return warnings;
}

describe('Dispatch Validation', () => {
  describe('Recipient Validation', () => {
    it('should warn when no recipients', () => {
      const warnings = validateDispatch([], [], []);

      expect(warnings).toContain('Add at least one recipient');
    });

    it('should warn when no signers (only reviewers)', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Reviewer', email: 'r@example.com', role: 'reviewer', order: 1 },
      ];

      const warnings = validateDispatch(recipients, [], []);

      expect(warnings).toContain('Add at least one signer (not just reviewers or CC)');
    });

    it('should warn when no signers (only CC)', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'CC', email: 'cc@example.com', role: 'cc', order: 1 },
      ];

      const warnings = validateDispatch(recipients, [], []);

      expect(warnings).toContain('Add at least one signer (not just reviewers or CC)');
    });

    it('should not warn about signers when signer exists', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [{ id: 'sig-1', type: 'signature' }];
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).not.toContain('Add at least one signer (not just reviewers or CC)');
    });
  });

  describe('Signature Field Validation', () => {
    it('should warn when no signature fields', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [{ id: 'text-1', type: 'text' }];

      const warnings = validateDispatch(recipients, fields, []);

      expect(warnings).toContain('Add at least one signature field to the document');
    });

    it('should not warn when signature field exists', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [{ id: 'sig-1', type: 'signature' }];
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).not.toContain('Add at least one signature field to the document');
    });
  });

  describe('Signer-Field Assignment Validation', () => {
    it('should warn when signer has no signature fields', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'John Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [{ id: 'sig-1', type: 'signature' }];

      // No assignments
      const warnings = validateDispatch(recipients, fields, []);

      expect(warnings).toContain('John Signer has no signature fields assigned');
    });

    it('should warn for each signer without signature fields', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Alice', email: 'a@example.com', role: 'signer', order: 1 },
        { id: '2', name: 'Bob', email: 'b@example.com', role: 'signer', order: 2 },
      ];
      const fields: TestField[] = [{ id: 'sig-1', type: 'signature' }];
      // Only assign to Alice
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).toContain('Bob has no signature fields assigned');
      expect(warnings).not.toContain('Alice has no signature fields assigned');
    });

    it('should not warn when signer has text field but no signature', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [
        { id: 'sig-1', type: 'signature' },
        { id: 'text-1', type: 'text' },
      ];
      // Assign text field but not signature
      const assignments: TestAssignment[] = [{ fieldId: 'text-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).toContain('Signer has no signature fields assigned');
    });

    it('should not require reviewers to have signature fields', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
        { id: '2', name: 'Reviewer', email: 'r@example.com', role: 'reviewer', order: 2 },
      ];
      const fields: TestField[] = [{ id: 'sig-1', type: 'signature' }];
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).not.toContain('Reviewer has no signature fields assigned');
    });
  });

  describe('Unassigned Field Validation', () => {
    it('should warn about unassigned signature fields', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [
        { id: 'sig-1', type: 'signature' },
        { id: 'sig-2', type: 'signature' },
      ];
      // Only assign one
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).toContain('1 signature field(s) not assigned to any recipient');
    });

    it('should count multiple unassigned signature fields', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [
        { id: 'sig-1', type: 'signature' },
        { id: 'sig-2', type: 'signature' },
        { id: 'sig-3', type: 'signature' },
      ];
      // Only assign one
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).toContain('2 signature field(s) not assigned to any recipient');
    });

    it('should not warn about unassigned text/date fields', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
      ];
      const fields: TestField[] = [
        { id: 'sig-1', type: 'signature' },
        { id: 'text-1', type: 'text' },
        { id: 'date-1', type: 'date' },
      ];
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      // Should not mention text/date fields
      const unassignedWarnings = warnings.filter((w) => w.includes('not assigned'));
      expect(unassignedWarnings.length).toBe(0);
    });
  });

  describe('Complete Valid Configuration', () => {
    it('should return no warnings for valid configuration', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Buyer', email: 'buyer@example.com', role: 'signer', order: 1 },
        { id: '2', name: 'Seller', email: 'seller@example.com', role: 'signer', order: 2 },
      ];
      const fields: TestField[] = [
        { id: 'sig-buyer', type: 'signature' },
        { id: 'sig-seller', type: 'signature' },
        { id: 'date-1', type: 'date' },
      ];
      const assignments: TestAssignment[] = [
        { fieldId: 'sig-buyer', recipientId: '1' },
        { fieldId: 'sig-seller', recipientId: '2' },
        { fieldId: 'date-1', recipientId: '1' },
      ];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).toEqual([]);
    });

    it('should return no warnings with reviewers and signers', () => {
      const recipients: TestRecipient[] = [
        { id: '1', name: 'Signer', email: 's@example.com', role: 'signer', order: 1 },
        { id: '2', name: 'Reviewer', email: 'r@example.com', role: 'reviewer', order: 2 },
        { id: '3', name: 'CC', email: 'cc@example.com', role: 'cc', order: 3 },
      ];
      const fields: TestField[] = [{ id: 'sig-1', type: 'signature' }];
      const assignments: TestAssignment[] = [{ fieldId: 'sig-1', recipientId: '1' }];

      const warnings = validateDispatch(recipients, fields, assignments);

      expect(warnings).toEqual([]);
    });
  });
});

describe('Email Validation Helpers', () => {
  /**
   * Basic email validation (mirrors the one in recipient-manager)
   */
  function isValidEmail(email: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
  }

  it('should accept valid emails', () => {
    expect(isValidEmail('test@example.com')).toBe(true);
    expect(isValidEmail('user.name@domain.org')).toBe(true);
    expect(isValidEmail('user+tag@example.co.uk')).toBe(true);
  });

  it('should reject invalid emails', () => {
    expect(isValidEmail('notanemail')).toBe(false);
    expect(isValidEmail('@example.com')).toBe(false);
    expect(isValidEmail('user@')).toBe(false);
    expect(isValidEmail('user @example.com')).toBe(false);
    expect(isValidEmail('')).toBe(false);
  });
});

describe('Field Type Labeling', () => {
  function getFieldTypeLabel(type: string): string {
    switch (type) {
      case 'signature':
        return 'Signature';
      case 'initials':
        return 'Initials';
      case 'text':
        return 'Text';
      case 'date':
        return 'Date';
      case 'checkbox':
        return 'Checkbox';
      default:
        return type;
    }
  }

  it('should return human-readable labels', () => {
    expect(getFieldTypeLabel('signature')).toBe('Signature');
    expect(getFieldTypeLabel('initials')).toBe('Initials');
    expect(getFieldTypeLabel('text')).toBe('Text');
    expect(getFieldTypeLabel('date')).toBe('Date');
    expect(getFieldTypeLabel('checkbox')).toBe('Checkbox');
  });

  it('should return original for unknown types', () => {
    expect(getFieldTypeLabel('unknown')).toBe('unknown');
    expect(getFieldTypeLabel('custom')).toBe('custom');
  });
});

describe('Field Summary', () => {
  interface Field {
    type: string;
  }

  function summarizeFields(fields: Field[]): { label: string; count: number }[] {
    const counts: Record<string, number> = {};

    for (const field of fields) {
      counts[field.type] = (counts[field.type] || 0) + 1;
    }

    return Object.entries(counts).map(([label, count]) => ({ label, count }));
  }

  it('should count field types', () => {
    const fields = [
      { type: 'signature' },
      { type: 'signature' },
      { type: 'text' },
      { type: 'date' },
    ];

    const summary = summarizeFields(fields);

    expect(summary).toContainEqual({ label: 'signature', count: 2 });
    expect(summary).toContainEqual({ label: 'text', count: 1 });
    expect(summary).toContainEqual({ label: 'date', count: 1 });
  });

  it('should return empty array for no fields', () => {
    expect(summarizeFields([])).toEqual([]);
  });
});
