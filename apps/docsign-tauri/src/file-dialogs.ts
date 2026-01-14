/**
 * Native file dialog bindings for the docsign Tauri app.
 *
 * These functions wrap Tauri commands to provide type-safe access to native
 * OS file dialogs. Designed with geriatric UX considerations:
 * - Clear file type descriptions ("PDF Documents")
 * - Defaults to user's Documents folder
 * - User-friendly error messages
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Error thrown when a file dialog operation fails.
 */
export class FileDialogError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FileDialogError";
  }
}

/**
 * Opens a native file picker dialog for selecting a single PDF file.
 *
 * @returns The file contents as Uint8Array, or null if the user cancelled.
 * @throws FileDialogError if the file cannot be read.
 *
 * @example
 * ```typescript
 * const pdfBytes = await openPdfFile();
 * if (pdfBytes) {
 *   // User selected a file - process the bytes
 *   console.log(`Loaded ${pdfBytes.length} bytes`);
 * } else {
 *   // User cancelled the dialog
 *   console.log('No file selected');
 * }
 * ```
 */
export async function openPdfFile(): Promise<Uint8Array | null> {
  try {
    const result = await invoke<number[] | null>("open_pdf_file");

    if (result === null) {
      return null;
    }

    // Convert the number array to Uint8Array
    return new Uint8Array(result);
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Could not open the PDF file.";
    throw new FileDialogError(message);
  }
}

/**
 * Opens a native save dialog for saving a signed PDF file.
 *
 * @param pdfBytes - The PDF file contents to save.
 * @param suggestedName - A suggested filename (e.g., "contract_signed.pdf").
 * @returns The path where the file was saved, or null if the user cancelled.
 * @throws FileDialogError if the file cannot be saved.
 *
 * @example
 * ```typescript
 * const savedPath = await saveSignedPdf(signedPdfBytes, 'contract_signed.pdf');
 * if (savedPath) {
 *   console.log(`Saved to: ${savedPath}`);
 * } else {
 *   console.log('Save cancelled');
 * }
 * ```
 */
export async function saveSignedPdf(
  pdfBytes: Uint8Array,
  suggestedName: string
): Promise<string | null> {
  try {
    // Convert Uint8Array to regular array for Tauri serialization
    const bytesArray = Array.from(pdfBytes);

    const result = await invoke<string | null>("save_signed_pdf", {
      pdfBytes: bytesArray,
      suggestedName,
    });

    return result;
  } catch (error) {
    const message =
      error instanceof Error
        ? error.message
        : "Could not save the PDF file. Please try again.";
    throw new FileDialogError(message);
  }
}

/**
 * Represents a PDF file loaded from the file system.
 */
export interface LoadedPdfFile {
  /** The filename (not full path) of the PDF */
  name: string;
  /** The raw PDF file contents */
  data: Uint8Array;
}

/**
 * Opens a native file picker dialog for selecting multiple PDF files.
 *
 * @returns An array of loaded PDF files, or an empty array if the user cancelled.
 * @throws FileDialogError if any files cannot be read.
 *
 * @example
 * ```typescript
 * const pdfs = await openMultiplePdfs();
 * if (pdfs.length > 0) {
 *   console.log(`Loaded ${pdfs.length} files:`);
 *   for (const pdf of pdfs) {
 *     console.log(`  - ${pdf.name}: ${pdf.data.length} bytes`);
 *   }
 * } else {
 *   console.log('No files selected');
 * }
 * ```
 */
export async function openMultiplePdfs(): Promise<LoadedPdfFile[]> {
  try {
    const result = await invoke<[string, number[]][]>("open_multiple_pdfs");

    // Convert each (name, bytes) tuple to a LoadedPdfFile
    return result.map(([name, bytes]) => ({
      name,
      data: new Uint8Array(bytes),
    }));
  } catch (error) {
    const message =
      error instanceof Error
        ? error.message
        : "Could not open the selected PDF files.";
    throw new FileDialogError(message);
  }
}

/**
 * Utility function to check if running in a Tauri environment.
 *
 * @returns True if running inside a Tauri app, false otherwise.
 */
export function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

/**
 * Opens a PDF file using native dialog if in Tauri, otherwise falls back
 * to web file input.
 *
 * This provides a unified API that works in both desktop and web contexts.
 *
 * @returns The file contents and name, or null if cancelled.
 */
export async function openPdfFileUnified(): Promise<{
  name: string;
  data: Uint8Array;
} | null> {
  if (isTauriEnvironment()) {
    const data = await openPdfFile();
    if (!data) return null;
    // Note: Native dialog doesn't return filename for single file
    // We use a generic name since the actual filename isn't available
    return { name: "document.pdf", data };
  }

  // Web fallback using file input
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".pdf,application/pdf";

    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }

      const arrayBuffer = await file.arrayBuffer();
      resolve({
        name: file.name,
        data: new Uint8Array(arrayBuffer),
      });
    };

    input.oncancel = () => resolve(null);
    input.click();
  });
}
