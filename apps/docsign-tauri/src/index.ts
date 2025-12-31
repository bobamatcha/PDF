/**
 * Tauri docsign desktop app - TypeScript bindings.
 *
 * This module exports all native functionality available in the Tauri app,
 * including printing, file dialog operations, and auto-updates.
 */

// Print functionality
export {
  printPdf,
  getAvailablePrinters,
  printToPrinter,
  isNativePrintingAvailable,
  getDefaultPrinter,
  printToDefaultPrinter,
} from "./print";
export type { PrinterInfo, PrintResult } from "./print";

// File dialog functionality
export {
  openPdfFile,
  saveSignedPdf,
  openMultiplePdfs,
  isTauriEnvironment,
  openPdfFileUnified,
  FileDialogError,
} from "./file-dialogs";
export type { LoadedPdfFile } from "./file-dialogs";

// Auto-update functionality
export {
  checkForUpdates,
  installUpdate,
  getCurrentVersion,
  isUpdateSupported,
  checkUpdateStatus,
  formatReleaseNotes,
  UpdateMessages,
  UpdateError,
} from "./updater";
export type { UpdateInfo } from "./updater";
