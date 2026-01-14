/**
 * Performance Monitoring Utilities
 *
 * Provides timing markers, performance measurement, and loading indicators
 * for critical paths in the DocSign application.
 *
 * Target: < 200ms time-to-interactive for core features.
 *
 * Usage:
 *   import { perf, PERF_MARKS } from './perf';
 *
 *   perf.mark(PERF_MARKS.BUNDLE_LOADED);
 *   perf.start('pdf-render');
 *   await renderPdf();
 *   perf.end('pdf-render');
 *
 *   const metrics = perf.getMetrics();
 */

/**
 * Performance mark identifiers for critical application events
 */
export const PERF_MARKS = {
  /** Main bundle started loading */
  BUNDLE_START: "docsign:bundle:start",
  /** Main bundle finished loading */
  BUNDLE_LOADED: "docsign:bundle:loaded",
  /** DocSign namespace initialized */
  NAMESPACE_INIT: "docsign:namespace:init",
  /** PDF.js started lazy loading */
  PDFJS_LOAD_START: "docsign:pdfjs:load:start",
  /** PDF.js finished loading */
  PDFJS_LOADED: "docsign:pdfjs:loaded",
  /** Signature modal opened */
  SIG_MODAL_OPEN: "docsign:sig:modal:open",
  /** Signature modal rendered */
  SIG_MODAL_RENDERED: "docsign:sig:modal:rendered",
  /** Signature capture initialized */
  SIG_CAPTURE_INIT: "docsign:sig:capture:init",
  /** PDF render started */
  PDF_RENDER_START: "docsign:pdf:render:start",
  /** PDF render completed */
  PDF_RENDER_END: "docsign:pdf:render:end",
  /** Signature applied to PDF */
  SIG_APPLIED: "docsign:sig:applied",
  /** Document fully interactive */
  INTERACTIVE: "docsign:interactive",
} as const;

export type PerfMarkId = (typeof PERF_MARKS)[keyof typeof PERF_MARKS];

/**
 * Performance timing entry
 */
interface TimingEntry {
  name: string;
  startTime: number;
  endTime?: number;
  duration?: number;
}

/**
 * Performance metrics summary
 */
export interface PerformanceMetrics {
  /** Time from bundle start to interactive (if available) */
  timeToInteractive?: number;
  /** Time to load PDF.js (lazy load time) */
  pdfJsLoadTime?: number;
  /** Time to render signature modal */
  signatureModalRenderTime?: number;
  /** Time to render PDF page */
  pdfRenderTime?: number;
  /** All custom timings */
  timings: Record<string, number>;
  /** All marks with timestamps */
  marks: Record<string, number>;
  /** Browser performance entries (if available) */
  entries: PerformanceEntry[];
}

/**
 * Loading indicator state
 */
interface LoadingIndicator {
  element: HTMLElement;
  message: string;
  startTime: number;
}

/**
 * Performance Monitor Class
 *
 * Singleton that tracks performance marks, measures durations,
 * and provides loading indicators for heavy operations.
 */
class PerformanceMonitor {
  private static instance: PerformanceMonitor;

  private marks: Map<string, number> = new Map();
  private timings: Map<string, TimingEntry> = new Map();
  private loadingIndicators: Map<string, LoadingIndicator> = new Map();
  private enabled: boolean;

  private constructor() {
    // Disable in production by default, enable via URL param or config
    this.enabled = this.determineEnabled();
  }

  /**
   * Determine if performance monitoring should be enabled
   */
  private determineEnabled(): boolean {
    if (typeof window === "undefined") {
      return false;
    }

    try {
      // Check URL param
      if (window.location?.search?.includes("perf=1")) {
        return true;
      }

      // Check localStorage (safely)
      if (typeof localStorage !== "undefined" && localStorage.getItem("docsign:perf") === "1") {
        return true;
      }
    } catch {
      // Ignore errors (e.g., in restricted contexts)
    }

    return process.env.NODE_ENV !== "production";
  }

  static getInstance(): PerformanceMonitor {
    if (!PerformanceMonitor.instance) {
      PerformanceMonitor.instance = new PerformanceMonitor();
    }
    return PerformanceMonitor.instance;
  }

  /**
   * Enable or disable performance monitoring
   */
  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    try {
      if (typeof localStorage !== "undefined") {
        if (enabled) {
          localStorage.setItem("docsign:perf", "1");
        } else {
          localStorage.removeItem("docsign:perf");
        }
      }
    } catch {
      // Ignore errors (e.g., in restricted contexts or Node.js)
    }
  }

  /**
   * Check if monitoring is enabled
   */
  isEnabled(): boolean {
    return this.enabled;
  }

  /**
   * Record a performance mark at the current time
   */
  mark(markId: PerfMarkId | string): void {
    const timestamp = performance.now();
    this.marks.set(markId, timestamp);

    if (this.enabled) {
      // Also use native Performance API for devtools
      try {
        performance.mark(markId);
      } catch {
        // Ignore if mark already exists
      }
    }
  }

  /**
   * Start a named timing measurement
   */
  start(name: string): void {
    const startTime = performance.now();
    this.timings.set(name, {
      name,
      startTime,
    });

    if (this.enabled) {
      try {
        performance.mark(`${name}:start`);
      } catch {
        // Ignore
      }
    }
  }

  /**
   * End a named timing measurement
   * @returns Duration in milliseconds, or undefined if not started
   */
  end(name: string): number | undefined {
    const entry = this.timings.get(name);
    if (!entry) return undefined;

    const endTime = performance.now();
    entry.endTime = endTime;
    entry.duration = endTime - entry.startTime;

    if (this.enabled) {
      try {
        performance.mark(`${name}:end`);
        performance.measure(name, `${name}:start`, `${name}:end`);
      } catch {
        // Ignore
      }

      // Log slow operations (> 100ms)
      if (entry.duration > 100) {
        console.warn(`[perf] Slow operation: ${name} took ${entry.duration.toFixed(1)}ms`);
      }
    }

    return entry.duration;
  }

  /**
   * Get duration of a completed timing
   */
  getDuration(name: string): number | undefined {
    return this.timings.get(name)?.duration;
  }

  /**
   * Get timestamp of a mark
   */
  getMark(markId: string): number | undefined {
    return this.marks.get(markId);
  }

  /**
   * Calculate duration between two marks
   */
  measureBetween(startMark: string, endMark: string): number | undefined {
    const start = this.marks.get(startMark);
    const end = this.marks.get(endMark);
    if (start === undefined || end === undefined) return undefined;
    return end - start;
  }

  /**
   * Get all collected performance metrics
   */
  getMetrics(): PerformanceMetrics {
    // Calculate key metrics
    const timeToInteractive = this.measureBetween(
      PERF_MARKS.BUNDLE_START,
      PERF_MARKS.INTERACTIVE
    );

    const pdfJsLoadTime = this.measureBetween(
      PERF_MARKS.PDFJS_LOAD_START,
      PERF_MARKS.PDFJS_LOADED
    );

    const signatureModalRenderTime = this.measureBetween(
      PERF_MARKS.SIG_MODAL_OPEN,
      PERF_MARKS.SIG_MODAL_RENDERED
    );

    const pdfRenderTime = this.measureBetween(
      PERF_MARKS.PDF_RENDER_START,
      PERF_MARKS.PDF_RENDER_END
    );

    // Convert maps to objects
    const timings: Record<string, number> = {};
    this.timings.forEach((entry, name) => {
      if (entry.duration !== undefined) {
        timings[name] = entry.duration;
      }
    });

    const marks: Record<string, number> = {};
    this.marks.forEach((timestamp, name) => {
      marks[name] = timestamp;
    });

    // Get browser performance entries
    let entries: PerformanceEntry[] = [];
    if (typeof performance !== "undefined" && performance.getEntriesByType) {
      try {
        entries = [
          ...performance.getEntriesByType("mark"),
          ...performance.getEntriesByType("measure"),
        ].filter((e) => e.name.startsWith("docsign:"));
      } catch {
        // Ignore
      }
    }

    return {
      timeToInteractive,
      pdfJsLoadTime,
      signatureModalRenderTime,
      pdfRenderTime,
      timings,
      marks,
      entries,
    };
  }

  /**
   * Log metrics summary to console
   */
  logMetrics(): void {
    if (!this.enabled) return;

    const metrics = this.getMetrics();

    console.group("[DocSign Performance]");

    if (metrics.timeToInteractive !== undefined) {
      const status = metrics.timeToInteractive < 200 ? "OK" : "SLOW";
      console.log(`Time to Interactive: ${metrics.timeToInteractive.toFixed(1)}ms [${status}]`);
    }

    if (metrics.pdfJsLoadTime !== undefined) {
      console.log(`PDF.js Load Time: ${metrics.pdfJsLoadTime.toFixed(1)}ms`);
    }

    if (metrics.signatureModalRenderTime !== undefined) {
      console.log(`Signature Modal Render: ${metrics.signatureModalRenderTime.toFixed(1)}ms`);
    }

    if (metrics.pdfRenderTime !== undefined) {
      console.log(`PDF Render Time: ${metrics.pdfRenderTime.toFixed(1)}ms`);
    }

    if (Object.keys(metrics.timings).length > 0) {
      console.log("Custom Timings:", metrics.timings);
    }

    console.groupEnd();
  }

  /**
   * Clear all collected metrics
   */
  clear(): void {
    this.marks.clear();
    this.timings.clear();

    if (typeof performance !== "undefined") {
      try {
        performance.clearMarks();
        performance.clearMeasures();
      } catch {
        // Ignore
      }
    }
  }

  // ========================================
  // Loading Indicators (Geriatric UX)
  // ========================================

  /**
   * Show a loading indicator for a heavy operation
   * Returns an ID to use when hiding the indicator
   */
  showLoading(message: string, containerId?: string): string {
    const id = `loading-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

    // Create loading overlay
    const overlay = document.createElement("div");
    overlay.id = id;
    overlay.className = "docsign-loading-overlay";
    overlay.setAttribute("role", "alert");
    overlay.setAttribute("aria-busy", "true");
    overlay.setAttribute("aria-live", "polite");

    overlay.innerHTML = `
      <div class="docsign-loading-content">
        <div class="docsign-loading-spinner" aria-hidden="true">
          <svg viewBox="0 0 50 50" width="50" height="50">
            <circle cx="25" cy="25" r="20" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round">
              <animate attributeName="stroke-dasharray" values="1,150;90,150;90,150" dur="1.5s" repeatCount="indefinite"/>
              <animate attributeName="stroke-dashoffset" values="0;-35;-125" dur="1.5s" repeatCount="indefinite"/>
            </circle>
          </svg>
        </div>
        <p class="docsign-loading-message">${this.escapeHtml(message)}</p>
      </div>
    `;

    // Inject styles if not present
    this.injectLoadingStyles();

    // Append to container or body
    const container = containerId
      ? document.getElementById(containerId)
      : document.body;

    if (container) {
      container.appendChild(overlay);
    }

    this.loadingIndicators.set(id, {
      element: overlay,
      message,
      startTime: performance.now(),
    });

    return id;
  }

  /**
   * Hide a loading indicator
   * @returns Duration the indicator was shown (ms)
   */
  hideLoading(id: string): number {
    const indicator = this.loadingIndicators.get(id);
    if (!indicator) return 0;

    const duration = performance.now() - indicator.startTime;

    // Add fade-out animation
    indicator.element.classList.add("docsign-loading-fade-out");

    // Remove after animation
    setTimeout(() => {
      indicator.element.remove();
    }, 200);

    this.loadingIndicators.delete(id);

    if (this.enabled && duration > 500) {
      console.log(`[perf] Loading "${indicator.message}" took ${duration.toFixed(0)}ms`);
    }

    return duration;
  }

  /**
   * Update loading indicator message
   */
  updateLoadingMessage(id: string, message: string): void {
    const indicator = this.loadingIndicators.get(id);
    if (!indicator) return;

    const msgEl = indicator.element.querySelector(".docsign-loading-message");
    if (msgEl) {
      msgEl.textContent = message;
    }
    indicator.message = message;
  }

  /**
   * Inject loading indicator styles
   */
  private injectLoadingStyles(): void {
    if (document.getElementById("docsign-loading-styles")) return;

    const style = document.createElement("style");
    style.id = "docsign-loading-styles";
    style.textContent = `
      .docsign-loading-overlay {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100000;
        animation: docsign-loading-fade-in 0.2s ease;
      }

      .docsign-loading-fade-out {
        animation: docsign-loading-fade-out 0.2s ease forwards;
      }

      @keyframes docsign-loading-fade-in {
        from { opacity: 0; }
        to { opacity: 1; }
      }

      @keyframes docsign-loading-fade-out {
        from { opacity: 1; }
        to { opacity: 0; }
      }

      .docsign-loading-content {
        background-color: var(--color-bg-primary, #ffffff);
        padding: 32px 48px;
        border-radius: 16px;
        text-align: center;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
        max-width: 90%;
      }

      .docsign-loading-spinner {
        color: var(--color-action-bg, #0056b3);
        margin-bottom: 16px;
      }

      .docsign-loading-spinner svg {
        display: block;
        margin: 0 auto;
      }

      .docsign-loading-message {
        font-size: var(--font-size-lg, 22px);
        color: var(--color-text-primary, #1a1a1a);
        margin: 0;
        font-weight: 500;
      }

      /* Reduced motion preference */
      @media (prefers-reduced-motion: reduce) {
        .docsign-loading-spinner svg circle {
          animation: none;
          stroke-dasharray: 45, 150;
        }
      }
    `;

    document.head.appendChild(style);
  }

  /**
   * Escape HTML for safe insertion
   */
  private escapeHtml(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }
}

// Export singleton instance
export const perf = PerformanceMonitor.getInstance();

// Export class for testing
export { PerformanceMonitor };

/**
 * Decorator/wrapper for measuring async function execution time
 */
export async function withTiming<T>(
  name: string,
  fn: () => Promise<T>
): Promise<T> {
  perf.start(name);
  try {
    return await fn();
  } finally {
    perf.end(name);
  }
}

/**
 * Decorator/wrapper for measuring sync function execution time
 */
export function withTimingSync<T>(name: string, fn: () => T): T {
  perf.start(name);
  try {
    return fn();
  } finally {
    perf.end(name);
  }
}

/**
 * Wrapper for operations with loading indicator
 */
export async function withLoading<T>(
  message: string,
  fn: () => Promise<T>,
  containerId?: string
): Promise<T> {
  const loadingId = perf.showLoading(message, containerId);
  try {
    return await fn();
  } finally {
    perf.hideLoading(loadingId);
  }
}

// Mark bundle loaded when this module is imported
if (typeof window !== "undefined") {
  perf.mark(PERF_MARKS.BUNDLE_LOADED);

  // Expose on window for debugging
  (window as unknown as { DocSignPerf: typeof perf }).DocSignPerf = perf;
}
