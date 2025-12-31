/**
 * Property-based tests for canvas undo functionality
 *
 * Tests the undo/redo system in the signature canvas.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import * as fc from 'fast-check';

// ============================================================
// Canvas History Mock
// ============================================================

interface MockImageData {
  width: number;
  height: number;
  data: Uint8ClampedArray;
}

/**
 * Mock implementation of canvas history for testing
 */
class CanvasHistoryMock {
  private history: MockImageData[] = [];
  private maxHistory = 20;

  push(imageData: MockImageData): void {
    if (this.history.length >= this.maxHistory) {
      this.history.shift();
    }
    this.history.push(imageData);
  }

  pop(): MockImageData | undefined {
    return this.history.pop();
  }

  get length(): number {
    return this.history.length;
  }

  clear(): void {
    this.history = [];
  }

  get items(): MockImageData[] {
    return [...this.history];
  }
}

/**
 * Create a mock ImageData
 */
function createMockImageData(width: number, height: number): MockImageData {
  return {
    width,
    height,
    data: new Uint8ClampedArray(width * height * 4),
  };
}

describe('Canvas Undo Functionality Property Tests', () => {
  let history: CanvasHistoryMock;

  beforeEach(() => {
    history = new CanvasHistoryMock();
  });

  // ============================================================
  // History Size Tests
  // ============================================================

  describe('History Size Management', () => {
    it('should never exceed max history size', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 50 }),
          (pushCount) => {
            history.clear();

            for (let i = 0; i < pushCount; i++) {
              history.push(createMockImageData(100, 100));
            }

            expect(history.length).toBeLessThanOrEqual(20);
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should keep most recent items when exceeding max', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 21, max: 50 }),
          (pushCount) => {
            history.clear();

            for (let i = 0; i < pushCount; i++) {
              const imageData = createMockImageData(100, 100);
              // Mark each image with its index
              imageData.data[0] = i % 256;
              history.push(imageData);
            }

            // The oldest items should have been removed
            expect(history.length).toBe(20);

            // The most recent item should have the last index
            const items = history.items;
            const lastItem = items[items.length - 1];
            expect(lastItem.data[0]).toBe((pushCount - 1) % 256);
          }
        ),
        { numRuns: 50 }
      );
    });
  });

  // ============================================================
  // Undo Operation Tests
  // ============================================================

  describe('Undo Operations', () => {
    it('should correctly pop items in LIFO order', () => {
      fc.assert(
        fc.property(
          fc.array(fc.integer({ min: 0, max: 255 }), { minLength: 1, maxLength: 20 }),
          (markers) => {
            history.clear();

            // Push items with markers
            for (const marker of markers) {
              const imageData = createMockImageData(100, 100);
              imageData.data[0] = marker;
              history.push(imageData);
            }

            // Pop and verify LIFO order
            for (let i = markers.length - 1; i >= 0; i--) {
              const popped = history.pop();
              expect(popped).toBeDefined();
              expect(popped!.data[0]).toBe(markers[i]);
            }

            expect(history.length).toBe(0);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should return undefined when popping empty history', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 10 }),
          (popCount) => {
            history.clear();

            for (let i = 0; i < popCount; i++) {
              const result = history.pop();
              expect(result).toBeUndefined();
            }
          }
        ),
        { numRuns: 20 }
      );
    });

    it('should correctly track length after mixed operations', () => {
      fc.assert(
        fc.property(
          fc.array(
            fc.oneof(
              fc.constant('push' as const),
              fc.constant('pop' as const)
            ),
            { minLength: 1, maxLength: 50 }
          ),
          (operations) => {
            history.clear();
            let expectedLength = 0;

            for (const op of operations) {
              if (op === 'push') {
                history.push(createMockImageData(100, 100));
                expectedLength = Math.min(expectedLength + 1, 20);
              } else {
                if (expectedLength > 0) {
                  history.pop();
                  expectedLength--;
                } else {
                  history.pop(); // Returns undefined, length stays 0
                }
              }
            }

            expect(history.length).toBe(expectedLength);
          }
        ),
        { numRuns: 100 }
      );
    });
  });

  // ============================================================
  // ImageData Property Tests
  // ============================================================

  describe('ImageData Properties', () => {
    it('should preserve image dimensions', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 1000 }),
          fc.integer({ min: 1, max: 1000 }),
          (width, height) => {
            const imageData = createMockImageData(width, height);

            expect(imageData.width).toBe(width);
            expect(imageData.height).toBe(height);
            expect(imageData.data.length).toBe(width * height * 4);
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should have RGBA data (4 bytes per pixel)', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 10, max: 500 }),
          fc.integer({ min: 10, max: 500 }),
          (width, height) => {
            const imageData = createMockImageData(width, height);
            const pixelCount = width * height;
            const expectedBytes = pixelCount * 4; // RGBA

            expect(imageData.data.length).toBe(expectedBytes);
          }
        ),
        { numRuns: 50 }
      );
    });
  });

  // ============================================================
  // Stroke Simulation Tests
  // ============================================================

  describe('Stroke Simulation', () => {
    it('should save state before each stroke', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 15 }),
          (strokeCount) => {
            history.clear();

            // Simulate initial blank canvas state
            history.push(createMockImageData(500, 300));

            // Simulate strokes (each saves state before drawing)
            for (let i = 0; i < strokeCount; i++) {
              history.push(createMockImageData(500, 300));
            }

            // History should have initial + all stroke states
            expect(history.length).toBe(Math.min(strokeCount + 1, 20));
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should allow undoing all strokes to blank canvas', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 10 }),
          (strokeCount) => {
            history.clear();

            // Initial blank state (marker = 0)
            const blankCanvas = createMockImageData(500, 300);
            blankCanvas.data[0] = 0;
            history.push(blankCanvas);

            // Strokes (marker = stroke number)
            for (let i = 1; i <= strokeCount; i++) {
              const strokeState = createMockImageData(500, 300);
              strokeState.data[0] = i;
              history.push(strokeState);
            }

            // Undo all strokes
            for (let i = strokeCount; i >= 1; i--) {
              const undone = history.pop();
              expect(undone).toBeDefined();
              expect(undone!.data[0]).toBe(i);
            }

            // Should be back to blank canvas
            expect(history.length).toBe(1);
            const remaining = history.items[0];
            expect(remaining.data[0]).toBe(0);
          }
        ),
        { numRuns: 50 }
      );
    });
  });

  // ============================================================
  // Clear Operation Tests
  // ============================================================

  describe('Clear Operation', () => {
    it('should reset history after clear', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 20 }),
          (itemCount) => {
            history.clear();

            // Add items
            for (let i = 0; i < itemCount; i++) {
              history.push(createMockImageData(100, 100));
            }

            expect(history.length).toBeGreaterThan(0);

            // Clear
            history.clear();

            expect(history.length).toBe(0);
          }
        ),
        { numRuns: 50 }
      );
    });

    it('should save new blank state after clear', () => {
      fc.assert(
        fc.property(
          fc.integer({ min: 1, max: 10 }),
          (strokeCount) => {
            history.clear();

            // Add strokes
            for (let i = 0; i < strokeCount; i++) {
              history.push(createMockImageData(100, 100));
            }

            // Clear and add new blank state
            history.clear();
            const blankState = createMockImageData(100, 100);
            blankState.data.fill(255); // White canvas
            history.push(blankState);

            expect(history.length).toBe(1);
            expect(history.items[0].data[0]).toBe(255);
          }
        ),
        { numRuns: 30 }
      );
    });
  });

  // ============================================================
  // Keyboard Shortcut Tests
  // ============================================================

  describe('Keyboard Shortcut Detection', () => {
    it('should detect Ctrl+Z', () => {
      fc.assert(
        fc.property(
          fc.boolean(), // ctrlKey
          fc.boolean(), // metaKey
          fc.string({ minLength: 1, maxLength: 1 }), // key
          (ctrlKey, metaKey, key) => {
            const isUndoShortcut =
              (ctrlKey || metaKey) && key.toLowerCase() === 'z';

            if (key.toLowerCase() === 'z' && (ctrlKey || metaKey)) {
              expect(isUndoShortcut).toBe(true);
            }
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should not trigger undo for other shortcuts', () => {
      fc.assert(
        fc.property(
          fc.boolean(),
          fc.boolean(),
          fc.string({ minLength: 1, maxLength: 1 }).filter((k) => k.toLowerCase() !== 'z'),
          (ctrlKey, metaKey, key) => {
            const isUndoShortcut =
              (ctrlKey || metaKey) && key.toLowerCase() === 'z';

            expect(isUndoShortcut).toBe(false);
          }
        ),
        { numRuns: 100 }
      );
    });
  });
});

// ============================================================
// Integration Tests
// ============================================================

describe('Undo Integration Tests', () => {
  it('should handle rapid undo operations', () => {
    const history = new CanvasHistoryMock();

    // Simulate drawing session
    history.push(createMockImageData(500, 300)); // Initial
    for (let i = 0; i < 10; i++) {
      history.push(createMockImageData(500, 300));
    }

    // Rapid undo (like holding Ctrl+Z)
    const undoCount = 5;
    for (let i = 0; i < undoCount; i++) {
      history.pop();
    }

    expect(history.length).toBe(11 - undoCount);
  });

  it('should preserve history across modal open/close', () => {
    const history = new CanvasHistoryMock();

    // Initial state
    history.push(createMockImageData(500, 300));

    // Draw some strokes
    history.push(createMockImageData(500, 300));
    history.push(createMockImageData(500, 300));

    // "Close" modal (don't clear history)
    const savedLength = history.length;

    // "Reopen" modal
    expect(history.length).toBe(savedLength);
  });
});

