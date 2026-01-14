/**
 * Native printing module for Tauri docsign app.
 *
 * This module provides TypeScript bindings for the native printing commands
 * implemented in Rust. It handles communication with the Tauri backend to
 * access platform-specific printing functionality.
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Information about an available printer on the system.
 */
export interface PrinterInfo {
  /** The printer name as known to the operating system */
  name: string;
  /** Whether this printer is set as the system default */
  is_default: boolean;
}

/**
 * Result of a print operation.
 */
export interface PrintResult {
  success: boolean;
  error?: string;
}

/**
 * Opens the system print dialog with the provided PDF.
 *
 * On macOS, this opens Preview with the print dialog.
 * On Windows, this uses the native print command.
 * On Linux, this uses lpr or xdg-open as a fallback.
 *
 * @param pdfBytes - The PDF file contents as a Uint8Array
 * @returns Promise resolving to true if the print dialog was opened successfully
 * @throws Error with user-friendly message if printing fails
 *
 * @example
 * ```typescript
 * const pdfBytes = new Uint8Array(pdfArrayBuffer);
 * try {
 *   const success = await printPdf(pdfBytes);
 *   if (success) {
 *     console.log('Print dialog opened');
 *   }
 * } catch (error) {
 *   console.error('Failed to print:', error);
 * }
 * ```
 */
export async function printPdf(pdfBytes: Uint8Array): Promise<boolean> {
  try {
    // Convert Uint8Array to regular array for Tauri serialization
    const bytesArray = Array.from(pdfBytes);
    return await invoke<boolean>("print_pdf", { pdfBytes: bytesArray });
  } catch (error) {
    throw new Error(formatPrintError(error));
  }
}

/**
 * Lists all available printers on the system.
 *
 * @returns Promise resolving to an array of PrinterInfo objects
 * @throws Error with user-friendly message if printer enumeration fails
 *
 * @example
 * ```typescript
 * try {
 *   const printers = await getAvailablePrinters();
 *   console.log('Available printers:', printers);
 *
 *   const defaultPrinter = printers.find(p => p.is_default);
 *   if (defaultPrinter) {
 *     console.log('Default printer:', defaultPrinter.name);
 *   }
 * } catch (error) {
 *   console.error('Failed to get printers:', error);
 * }
 * ```
 */
export async function getAvailablePrinters(): Promise<PrinterInfo[]> {
  try {
    return await invoke<PrinterInfo[]>("get_available_printers");
  } catch (error) {
    throw new Error(formatPrintError(error));
  }
}

/**
 * Prints a PDF directly to a specified printer without showing a dialog.
 *
 * @param pdfBytes - The PDF file contents as a Uint8Array
 * @param printerName - The name of the printer to print to
 * @returns Promise resolving to true if the print job was sent successfully
 * @throws Error with user-friendly message if printing fails
 *
 * @example
 * ```typescript
 * const pdfBytes = new Uint8Array(pdfArrayBuffer);
 * try {
 *   const success = await printToPrinter(pdfBytes, 'HP LaserJet Pro');
 *   if (success) {
 *     console.log('Document sent to printer');
 *   }
 * } catch (error) {
 *   console.error('Failed to print:', error);
 * }
 * ```
 */
export async function printToPrinter(
  pdfBytes: Uint8Array,
  printerName: string
): Promise<boolean> {
  try {
    // Convert Uint8Array to regular array for Tauri serialization
    const bytesArray = Array.from(pdfBytes);
    return await invoke<boolean>("print_to_printer", {
      pdfBytes: bytesArray,
      printerName,
    });
  } catch (error) {
    throw new Error(formatPrintError(error));
  }
}

/**
 * Checks if the current environment supports native printing.
 *
 * This checks if we're running in a Tauri context where native printing
 * commands are available.
 *
 * @returns true if native printing is available, false otherwise
 */
export function isNativePrintingAvailable(): boolean {
  // Check if we're in a Tauri environment
  return typeof window !== "undefined" && "__TAURI__" in window;
}

/**
 * Gets the default printer, if one is configured.
 *
 * @returns Promise resolving to the default PrinterInfo, or null if no default is set
 */
export async function getDefaultPrinter(): Promise<PrinterInfo | null> {
  try {
    const printers = await getAvailablePrinters();
    return printers.find((p) => p.is_default) || null;
  } catch {
    return null;
  }
}

/**
 * Prints using the system default printer without showing a dialog.
 *
 * @param pdfBytes - The PDF file contents as a Uint8Array
 * @returns Promise resolving to a PrintResult object
 */
export async function printToDefaultPrinter(
  pdfBytes: Uint8Array
): Promise<PrintResult> {
  try {
    const defaultPrinter = await getDefaultPrinter();
    if (!defaultPrinter) {
      return {
        success: false,
        error: "No default printer configured",
      };
    }

    const success = await printToPrinter(pdfBytes, defaultPrinter.name);
    return { success };
  } catch (error) {
    return {
      success: false,
      error: formatPrintError(error),
    };
  }
}

/**
 * Formats an error from the print backend into a user-friendly message.
 */
function formatPrintError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "An unexpected error occurred while printing";
}
