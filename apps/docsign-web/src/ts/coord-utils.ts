/**
 * Coordinate conversion utilities using PDF.js viewport methods
 *
 * This module provides proper coordinate transformations between:
 * - DOM/viewport coordinates (origin top-left, Y increases downward)
 * - PDF coordinates (origin bottom-left, Y increases upward)
 *
 * Uses PDF.js native viewport methods which handle:
 * - Y-axis flip
 * - Scale transformations
 * - Page rotation
 */

import type { PDFJSViewport, CachedPageInfo } from './types/pdf-types';

/**
 * Represents a rectangle in either coordinate system
 */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Convert a DOM rectangle (relative to canvas) to PDF coordinates
 * Uses viewport.convertToPdfPoint() for accurate transformation
 */
export function domRectToPdf(
  viewport: PDFJSViewport,
  domX: number,
  domY: number,
  domWidth: number,
  domHeight: number
): Rect {
  // Convert top-left and bottom-right corners
  const [pdfX1, pdfY1] = viewport.convertToPdfPoint(domX, domY);
  const [pdfX2, pdfY2] = viewport.convertToPdfPoint(domX + domWidth, domY + domHeight);

  // In PDF coords, y1 > y2 because PDF Y increases upward
  return {
    x: Math.min(pdfX1, pdfX2),
    y: Math.min(pdfY1, pdfY2),
    width: Math.abs(pdfX2 - pdfX1),
    height: Math.abs(pdfY2 - pdfY1),
  };
}

/**
 * Convert a single DOM point to PDF coordinates
 */
export function domPointToPdf(
  viewport: PDFJSViewport,
  domX: number,
  domY: number
): [number, number] {
  return viewport.convertToPdfPoint(domX, domY);
}

/**
 * Convert a PDF rectangle to DOM coordinates (relative to canvas)
 * Uses viewport.convertToViewportRectangle() for accurate transformation
 */
export function pdfRectToDom(
  viewport: PDFJSViewport,
  pdfX: number,
  pdfY: number,
  pdfWidth: number,
  pdfHeight: number
): Rect {
  // convertToViewportRectangle expects [x1, y1, x2, y2]
  const pdfRect: [number, number, number, number] = [
    pdfX,
    pdfY,
    pdfX + pdfWidth,
    pdfY + pdfHeight,
  ];

  const [domX1, domY1, domX2, domY2] = viewport.convertToViewportRectangle(pdfRect);

  return {
    x: Math.min(domX1, domX2),
    y: Math.min(domY1, domY2),
    width: Math.abs(domX2 - domX1),
    height: Math.abs(domY2 - domY1),
  };
}

/**
 * Convert a single PDF point to DOM coordinates
 */
export function pdfPointToDom(
  viewport: PDFJSViewport,
  pdfX: number,
  pdfY: number
): [number, number] {
  return viewport.convertToViewportPoint(pdfX, pdfY);
}

/**
 * Get canvas-relative coordinates from a client rect and canvas bounding rect
 * This is the first step before converting to PDF coordinates
 */
export function clientRectToCanvasRelative(
  clientRect: DOMRect,
  canvasRect: DOMRect
): Rect {
  return {
    x: clientRect.left - canvasRect.left,
    y: clientRect.top - canvasRect.top,
    width: clientRect.width,
    height: clientRect.height,
  };
}

/**
 * Convert selection client rects to PDF coordinates
 * This is the main function for highlight/underline operations
 */
export function selectionRectsToPdf(
  rects: DOMRectList,
  canvasRect: DOMRect,
  viewport: PDFJSViewport
): Rect[] {
  const pdfRects: Rect[] = [];

  for (let i = 0; i < rects.length; i++) {
    const rect = rects[i];

    // Skip tiny rects (artifacts from text selection)
    if (rect.width < 2 || rect.height < 2) continue;

    // Convert to canvas-relative coordinates
    const domX = rect.left - canvasRect.left;
    const domY = rect.top - canvasRect.top;

    // Convert to PDF coordinates using viewport method
    const pdfRect = domRectToPdf(viewport, domX, domY, rect.width, rect.height);
    pdfRects.push(pdfRect);
  }

  return pdfRects;
}

/**
 * Get the canvas and viewport for a page, throwing if not found
 */
export function getPageRenderInfo(
  pageInfo: CachedPageInfo | undefined,
  pageDiv: HTMLElement | null
): { canvas: HTMLCanvasElement; canvasRect: DOMRect; viewport: PDFJSViewport } | null {
  if (!pageInfo) return null;

  const canvas = pageDiv?.querySelector('canvas') as HTMLCanvasElement | null;
  if (!canvas) return null;

  return {
    canvas,
    canvasRect: canvas.getBoundingClientRect(),
    viewport: pageInfo.viewport,
  };
}
