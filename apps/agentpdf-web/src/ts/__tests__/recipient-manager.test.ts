/**
 * Tests for RecipientManager
 *
 * These tests verify the core logic of recipient management:
 * - Adding/removing recipients
 * - Field assignments
 * - Order management
 * - Export/import functionality
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the DOM environment
const mockDocument = {
  getElementById: vi.fn(() => null),
  createElement: vi.fn(() => ({
    style: {},
    textContent: '',
    innerHTML: '',
    appendChild: vi.fn(),
    querySelector: vi.fn(() => null),
    remove: vi.fn(),
  })),
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
  randomUUID: vi.fn(() => `uuid-${Date.now()}-${Math.random()}`),
});

// Import after mocks are set up
import {
  addRecipient,
  removeRecipient,
  updateRecipient,
  getRecipient,
  getAllRecipients,
  moveRecipientUp,
  moveRecipientDown,
  assignFieldToRecipient,
  unassignField,
  getFieldRecipient,
  getRecipientFields,
  getAllAssignments,
  setSigningMode,
  getSigningMode,
  SigningMode,
  exportRecipientData,
  importRecipientData,
  clearAllRecipients,
  type Recipient,
} from '../recipient-manager';

describe('RecipientManager', () => {
  beforeEach(() => {
    // Clear all recipients before each test
    clearAllRecipients();
    vi.clearAllMocks();
  });

  describe('addRecipient', () => {
    it('should add a recipient with default signer role', () => {
      const recipient = addRecipient('John Doe', 'john@example.com');

      expect(recipient.name).toBe('John Doe');
      expect(recipient.email).toBe('john@example.com');
      expect(recipient.role).toBe('signer');
      expect(recipient.order).toBe(1);
      expect(recipient.id).toBeDefined();
      expect(recipient.color).toBeDefined();
    });

    it('should add a recipient with specified role', () => {
      const recipient = addRecipient('Jane Doe', 'jane@example.com', 'reviewer');

      expect(recipient.role).toBe('reviewer');
    });

    it('should trim whitespace from name and email', () => {
      const recipient = addRecipient('  John Doe  ', '  JOHN@EXAMPLE.COM  ');

      expect(recipient.name).toBe('John Doe');
      expect(recipient.email).toBe('john@example.com');
    });

    it('should lowercase the email', () => {
      const recipient = addRecipient('Test', 'TeSt@ExAmPlE.cOm');

      expect(recipient.email).toBe('test@example.com');
    });

    it('should assign sequential order numbers', () => {
      const r1 = addRecipient('First', 'first@example.com');
      const r2 = addRecipient('Second', 'second@example.com');
      const r3 = addRecipient('Third', 'third@example.com');

      expect(r1.order).toBe(1);
      expect(r2.order).toBe(2);
      expect(r3.order).toBe(3);
    });

    it('should assign different colors to recipients', () => {
      const colors = new Set<string>();

      for (let i = 0; i < 6; i++) {
        const r = addRecipient(`Recipient ${i}`, `r${i}@example.com`);
        colors.add(r.color);
      }

      // All 6 default colors should be used
      expect(colors.size).toBe(6);
    });
  });

  describe('removeRecipient', () => {
    it('should remove a recipient by id', () => {
      const r1 = addRecipient('First', 'first@example.com');
      const r2 = addRecipient('Second', 'second@example.com');

      removeRecipient(r1.id);

      const remaining = getAllRecipients();
      expect(remaining.length).toBe(1);
      expect(remaining[0].id).toBe(r2.id);
    });

    it('should reorder remaining recipients after removal', () => {
      addRecipient('First', 'first@example.com');
      const r2 = addRecipient('Second', 'second@example.com');
      const r3 = addRecipient('Third', 'third@example.com');

      removeRecipient(r2.id);

      const remaining = getAllRecipients();
      expect(remaining[0].order).toBe(1);
      expect(remaining[1].order).toBe(2);
    });

    it('should remove field assignments for deleted recipient', () => {
      const r1 = addRecipient('First', 'first@example.com');
      assignFieldToRecipient('field-1', r1.id);
      assignFieldToRecipient('field-2', r1.id);

      removeRecipient(r1.id);

      expect(getFieldRecipient('field-1')).toBeUndefined();
      expect(getFieldRecipient('field-2')).toBeUndefined();
    });
  });

  describe('updateRecipient', () => {
    it('should update recipient name', () => {
      const r = addRecipient('Original', 'original@example.com');

      updateRecipient(r.id, { name: 'Updated Name' });

      const updated = getRecipient(r.id);
      expect(updated?.name).toBe('Updated Name');
      expect(updated?.email).toBe('original@example.com'); // Unchanged
    });

    it('should update recipient email', () => {
      const r = addRecipient('Original', 'original@example.com');

      updateRecipient(r.id, { email: 'updated@example.com' });

      const updated = getRecipient(r.id);
      expect(updated?.email).toBe('updated@example.com');
    });

    it('should update recipient role', () => {
      const r = addRecipient('Original', 'original@example.com', 'signer');

      updateRecipient(r.id, { role: 'cc' });

      const updated = getRecipient(r.id);
      expect(updated?.role).toBe('cc');
    });

    it('should ignore update for non-existent recipient', () => {
      // Should not throw
      updateRecipient('non-existent-id', { name: 'Test' });

      expect(getAllRecipients().length).toBe(0);
    });
  });

  describe('getRecipient', () => {
    it('should return recipient by id', () => {
      const r = addRecipient('Test', 'test@example.com');

      const found = getRecipient(r.id);

      expect(found).toEqual(r);
    });

    it('should return undefined for non-existent id', () => {
      const found = getRecipient('non-existent');

      expect(found).toBeUndefined();
    });
  });

  describe('getAllRecipients', () => {
    it('should return empty array when no recipients', () => {
      expect(getAllRecipients()).toEqual([]);
    });

    it('should return recipients sorted by order', () => {
      const r3 = addRecipient('Third', 'third@example.com');
      const r1 = addRecipient('First', 'first@example.com');
      const r2 = addRecipient('Second', 'second@example.com');

      // Manually adjust orders to test sorting
      moveRecipientUp(r1.id);
      moveRecipientUp(r1.id);

      const all = getAllRecipients();
      expect(all[0].id).toBe(r1.id);
    });
  });

  describe('moveRecipientUp', () => {
    it('should move recipient up in order', () => {
      addRecipient('First', 'first@example.com');
      const r2 = addRecipient('Second', 'second@example.com');

      moveRecipientUp(r2.id);

      const updated = getRecipient(r2.id);
      expect(updated?.order).toBe(1);
    });

    it('should not move first recipient up', () => {
      const r1 = addRecipient('First', 'first@example.com');
      addRecipient('Second', 'second@example.com');

      moveRecipientUp(r1.id);

      const updated = getRecipient(r1.id);
      expect(updated?.order).toBe(1);
    });
  });

  describe('moveRecipientDown', () => {
    it('should move recipient down in order', () => {
      const r1 = addRecipient('First', 'first@example.com');
      addRecipient('Second', 'second@example.com');

      moveRecipientDown(r1.id);

      const updated = getRecipient(r1.id);
      expect(updated?.order).toBe(2);
    });

    it('should not move last recipient down', () => {
      addRecipient('First', 'first@example.com');
      const r2 = addRecipient('Second', 'second@example.com');

      moveRecipientDown(r2.id);

      const updated = getRecipient(r2.id);
      expect(updated?.order).toBe(2);
    });
  });

  describe('Field Assignments', () => {
    describe('assignFieldToRecipient', () => {
      it('should assign a field to a recipient', () => {
        const r = addRecipient('Test', 'test@example.com');

        assignFieldToRecipient('field-1', r.id);

        expect(getFieldRecipient('field-1')).toEqual(r);
      });

      it('should not assign to non-existent recipient', () => {
        assignFieldToRecipient('field-1', 'non-existent');

        expect(getFieldRecipient('field-1')).toBeUndefined();
      });

      it('should reassign field if already assigned', () => {
        const r1 = addRecipient('First', 'first@example.com');
        const r2 = addRecipient('Second', 'second@example.com');

        assignFieldToRecipient('field-1', r1.id);
        assignFieldToRecipient('field-1', r2.id);

        expect(getFieldRecipient('field-1')).toEqual(r2);
      });
    });

    describe('unassignField', () => {
      it('should remove field assignment', () => {
        const r = addRecipient('Test', 'test@example.com');
        assignFieldToRecipient('field-1', r.id);

        unassignField('field-1');

        expect(getFieldRecipient('field-1')).toBeUndefined();
      });
    });

    describe('getRecipientFields', () => {
      it('should return all fields assigned to a recipient', () => {
        const r = addRecipient('Test', 'test@example.com');
        assignFieldToRecipient('field-1', r.id);
        assignFieldToRecipient('field-2', r.id);
        assignFieldToRecipient('field-3', r.id);

        const fields = getRecipientFields(r.id);

        expect(fields).toContain('field-1');
        expect(fields).toContain('field-2');
        expect(fields).toContain('field-3');
        expect(fields.length).toBe(3);
      });

      it('should return empty array for recipient with no fields', () => {
        const r = addRecipient('Test', 'test@example.com');

        expect(getRecipientFields(r.id)).toEqual([]);
      });
    });

    describe('getAllAssignments', () => {
      it('should return all field assignments', () => {
        const r1 = addRecipient('First', 'first@example.com');
        const r2 = addRecipient('Second', 'second@example.com');
        assignFieldToRecipient('field-1', r1.id);
        assignFieldToRecipient('field-2', r2.id);

        const assignments = getAllAssignments();

        expect(assignments.length).toBe(2);
        expect(assignments).toContainEqual({ fieldId: 'field-1', recipientId: r1.id });
        expect(assignments).toContainEqual({ fieldId: 'field-2', recipientId: r2.id });
      });
    });
  });

  describe('Signing Mode', () => {
    it('should default to Parallel mode', () => {
      expect(getSigningMode()).toBe(SigningMode.Parallel);
    });

    it('should set signing mode to Sequential', () => {
      setSigningMode(SigningMode.Sequential);

      expect(getSigningMode()).toBe(SigningMode.Sequential);
    });

    it('should set signing mode to Parallel', () => {
      setSigningMode(SigningMode.Sequential);
      setSigningMode(SigningMode.Parallel);

      expect(getSigningMode()).toBe(SigningMode.Parallel);
    });
  });

  describe('Export/Import', () => {
    it('should export all recipient data', () => {
      const r1 = addRecipient('First', 'first@example.com');
      const r2 = addRecipient('Second', 'second@example.com');
      assignFieldToRecipient('field-1', r1.id);
      setSigningMode(SigningMode.Sequential);

      const data = exportRecipientData();

      expect(data.recipients.length).toBe(2);
      expect(data.assignments.length).toBe(1);
      expect(data.signingMode).toBe(SigningMode.Sequential);
    });

    it('should import recipient data', () => {
      const r1 = addRecipient('First', 'first@example.com');
      const data = exportRecipientData();

      clearAllRecipients();
      expect(getAllRecipients().length).toBe(0);

      importRecipientData(data);

      expect(getAllRecipients().length).toBe(1);
      expect(getAllRecipients()[0].name).toBe('First');
    });

    it('should preserve field assignments on import', () => {
      const r1 = addRecipient('First', 'first@example.com');
      assignFieldToRecipient('field-1', r1.id);
      const data = exportRecipientData();

      clearAllRecipients();
      importRecipientData(data);

      expect(getAllAssignments().length).toBe(1);
    });
  });

  describe('clearAllRecipients', () => {
    it('should remove all recipients', () => {
      addRecipient('First', 'first@example.com');
      addRecipient('Second', 'second@example.com');

      clearAllRecipients();

      expect(getAllRecipients().length).toBe(0);
    });

    it('should remove all assignments', () => {
      const r1 = addRecipient('First', 'first@example.com');
      assignFieldToRecipient('field-1', r1.id);

      clearAllRecipients();

      expect(getAllAssignments().length).toBe(0);
    });
  });
});

describe('RecipientManager Properties', () => {
  beforeEach(() => {
    clearAllRecipients();
  });

  it('property: adding N recipients results in N recipients', () => {
    const n = Math.floor(Math.random() * 10) + 1;

    for (let i = 0; i < n; i++) {
      addRecipient(`Recipient ${i}`, `r${i}@example.com`);
    }

    expect(getAllRecipients().length).toBe(n);
  });

  it('property: recipient order is always 1..N after any operation', () => {
    const r1 = addRecipient('A', 'a@example.com');
    const r2 = addRecipient('B', 'b@example.com');
    const r3 = addRecipient('C', 'c@example.com');

    // Perform random operations
    moveRecipientUp(r2.id);
    removeRecipient(r1.id);

    const recipients = getAllRecipients();
    const orders = recipients.map((r) => r.order).sort((a, b) => a - b);

    // Should be [1, 2] (contiguous starting from 1)
    for (let i = 0; i < orders.length; i++) {
      expect(orders[i]).toBe(i + 1);
    }
  });

  it('property: each recipient has a unique color', () => {
    for (let i = 0; i < 6; i++) {
      addRecipient(`R${i}`, `r${i}@example.com`);
    }

    const recipients = getAllRecipients();
    const colors = new Set(recipients.map((r) => r.color));

    expect(colors.size).toBe(recipients.length);
  });

  it('property: field can only be assigned to one recipient', () => {
    const r1 = addRecipient('A', 'a@example.com');
    const r2 = addRecipient('B', 'b@example.com');

    assignFieldToRecipient('field-1', r1.id);
    assignFieldToRecipient('field-1', r2.id);

    // Should only appear in r2's fields
    expect(getRecipientFields(r1.id)).not.toContain('field-1');
    expect(getRecipientFields(r2.id)).toContain('field-1');
    expect(getAllAssignments().length).toBe(1);
  });

  it('property: export then import is identity', () => {
    const r1 = addRecipient('A', 'a@example.com');
    addRecipient('B', 'b@example.com');
    assignFieldToRecipient('field-1', r1.id);
    setSigningMode(SigningMode.Sequential);

    const before = exportRecipientData();
    clearAllRecipients();
    importRecipientData(before);
    const after = exportRecipientData();

    expect(after.recipients.length).toBe(before.recipients.length);
    expect(after.assignments.length).toBe(before.assignments.length);
    expect(after.signingMode).toBe(before.signingMode);
  });
});
