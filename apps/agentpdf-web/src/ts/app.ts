/**
 * agentPDF Template Completion Engine
 *
 * Main entry point for the TypeScript bundle.
 * This module initializes the template editor and exports all public APIs.
 */

// Re-export all modules
export * from './pdf-bridge';
export * from './pdf-loader';
export * from './coord-utils';
export * from './template-editor';
export * from './page-operations';

// Import for side effects (window assignments)
import './pdf-bridge';
import './pdf-loader';
import './template-editor';
import './page-operations';

// Export types
export type * from './types/pdf-types';
export type * from './types/index';

console.log('agentPDF Template Completion Engine loaded');
