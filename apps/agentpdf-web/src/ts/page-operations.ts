/**
 * Page Operations for Template Completion Engine
 *
 * Provides split/merge functionality using pdfjoin-core.
 * This module wraps the pdfjoin-wasm bindings for use in agentPDF.
 */

import { SessionMode, type PdfJoinSessionInstance } from './types/index';

// ============================================================================
// PAGE SPLIT
// ============================================================================

export interface SplitResult {
  success: boolean;
  data?: Uint8Array;
  error?: string;
}

/**
 * Split a PDF, extracting specific pages
 *
 * @param pdfBytes - The source PDF bytes
 * @param pageRanges - Page ranges to extract (e.g., "1-3, 5, 8-10")
 * @returns The extracted pages as a new PDF
 */
export async function splitPdf(
  pdfBytes: Uint8Array,
  pageRanges: string
): Promise<SplitResult> {
  try {
    const PdfJoinSession = window.wasmBindings?.PdfJoinSession;
    if (!PdfJoinSession) {
      return { success: false, error: 'pdfjoin-wasm not loaded' };
    }

    const session = new PdfJoinSession(SessionMode.Split);
    try {
      session.addDocument('source.pdf', pdfBytes);
      session.setPageSelection(pageRanges);
      const result = session.execute();
      return { success: true, data: result };
    } finally {
      session.free();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { success: false, error: message };
  }
}

/**
 * Parse page ranges string into array of page numbers
 * Validates ranges against total page count
 *
 * @param rangeStr - Range string like "1-3, 5, 8-10"
 * @param totalPages - Total pages in the document
 * @returns Array of page numbers (1-indexed)
 */
export function parsePageRanges(rangeStr: string, totalPages: number): number[] {
  const pages = new Set<number>();
  const parts = rangeStr.split(',').map((p) => p.trim());

  for (const part of parts) {
    if (part.includes('-')) {
      const [start, end] = part.split('-').map((n) => parseInt(n.trim(), 10));
      if (!isNaN(start) && !isNaN(end)) {
        for (let i = Math.max(1, start); i <= Math.min(totalPages, end); i++) {
          pages.add(i);
        }
      }
    } else {
      const num = parseInt(part, 10);
      if (!isNaN(num) && num >= 1 && num <= totalPages) {
        pages.add(num);
      }
    }
  }

  return Array.from(pages).sort((a, b) => a - b);
}

// ============================================================================
// PAGE MERGE
// ============================================================================

export interface MergeDocument {
  name: string;
  bytes: Uint8Array;
}

export interface MergeResult {
  success: boolean;
  data?: Uint8Array;
  error?: string;
}

/**
 * Merge multiple PDFs into one
 *
 * @param documents - Array of documents to merge (in order)
 * @returns The merged PDF
 */
export async function mergePdfs(documents: MergeDocument[]): Promise<MergeResult> {
  if (documents.length === 0) {
    return { success: false, error: 'No documents to merge' };
  }

  if (documents.length === 1) {
    return { success: true, data: documents[0].bytes };
  }

  try {
    const PdfJoinSession = window.wasmBindings?.PdfJoinSession;
    if (!PdfJoinSession) {
      return { success: false, error: 'pdfjoin-wasm not loaded' };
    }

    const session = new PdfJoinSession(SessionMode.Merge);
    try {
      for (const doc of documents) {
        session.addDocument(doc.name, doc.bytes);
      }
      const result = session.execute();
      return { success: true, data: result };
    } finally {
      session.free();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { success: false, error: message };
  }
}

/**
 * Merge a new document with an existing one
 *
 * @param existingPdf - The existing PDF bytes
 * @param existingName - Name of the existing PDF
 * @param newPdf - The new PDF bytes to append
 * @param newName - Name of the new PDF
 * @param prepend - If true, add new PDF before existing (default: false)
 * @returns The merged PDF
 */
export async function appendPdf(
  existingPdf: Uint8Array,
  existingName: string,
  newPdf: Uint8Array,
  newName: string,
  prepend = false
): Promise<MergeResult> {
  const documents: MergeDocument[] = prepend
    ? [
        { name: newName, bytes: newPdf },
        { name: existingName, bytes: existingPdf },
      ]
    : [
        { name: existingName, bytes: existingPdf },
        { name: newName, bytes: newPdf },
      ];

  return mergePdfs(documents);
}

// ============================================================================
// EXPORT
// ============================================================================

export const PageOperations = {
  // Split
  splitPdf,
  parsePageRanges,

  // Merge
  mergePdfs,
  appendPdf,
};

// Expose on window
(window as unknown as { PageOperations: typeof PageOperations }).PageOperations = PageOperations;
