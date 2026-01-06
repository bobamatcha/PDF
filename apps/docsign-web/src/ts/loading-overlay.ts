/**
 * Loading Overlay with Jokes
 *
 * Shows a fullscreen overlay with a joke while waiting for API responses.
 * Designed for geriatric-friendly UX with large text and clear visuals.
 */

import { getRandomJoke, Joke } from "./jokes";
import { createLogger } from "./logger";

const log = createLogger("LoadingOverlay");

/** Default delay before showing punchline (ms) */
const DEFAULT_PUNCHLINE_DELAY = 1500;

/** Minimum time to show overlay to avoid flashing (ms) */
const MIN_DISPLAY_TIME = 800;

/** Overlay state */
let overlayElement: HTMLElement | null = null;
let currentJoke: Joke | null = null;
let showStartTime: number = 0;
let punchlineTimeout: ReturnType<typeof setTimeout> | null = null;

/**
 * Create the overlay DOM element
 */
function createOverlayElement(): HTMLElement {
  const overlay = document.createElement("div");
  overlay.id = "loading-overlay";
  overlay.className = "loading-overlay";
  overlay.innerHTML = `
    <div class="loading-content">
      <div class="loading-spinner"></div>
      <div class="loading-joke">
        <p class="joke-setup"></p>
        <p class="joke-punchline"></p>
      </div>
      <p class="loading-status">Loading...</p>
    </div>
  `;
  return overlay;
}

/**
 * Initialize the loading overlay
 * Call this once on page load
 */
export function initLoadingOverlay(): void {
  // Don't create if already exists
  if (document.getElementById("loading-overlay")) {
    overlayElement = document.getElementById("loading-overlay");
    return;
  }

  overlayElement = createOverlayElement();
  document.body.appendChild(overlayElement);

  // Add styles if not already present
  if (!document.getElementById("loading-overlay-styles")) {
    const style = document.createElement("style");
    style.id = "loading-overlay-styles";
    style.textContent = OVERLAY_STYLES;
    document.head.appendChild(style);
  }

  log.debug("Loading overlay initialized");
}

/**
 * Show the loading overlay with a joke
 * @param statusText - Optional status text (default: "Loading...")
 */
export function showLoadingOverlay(statusText: string = "Loading..."): void {
  if (!overlayElement) {
    initLoadingOverlay();
  }

  if (!overlayElement) return;

  showStartTime = Date.now();
  currentJoke = getRandomJoke();

  // Update content
  const setupEl = overlayElement.querySelector(".joke-setup");
  const punchlineEl = overlayElement.querySelector(".joke-punchline");
  const statusEl = overlayElement.querySelector(".loading-status");

  if (setupEl) setupEl.textContent = currentJoke.setup;
  if (punchlineEl) {
    punchlineEl.textContent = "";
    punchlineEl.classList.remove("visible");
  }
  if (statusEl) statusEl.textContent = statusText;

  // Show overlay
  overlayElement.classList.add("visible");

  // Schedule punchline reveal
  const delay = currentJoke.delay ?? DEFAULT_PUNCHLINE_DELAY;
  punchlineTimeout = setTimeout(() => {
    if (punchlineEl && currentJoke) {
      punchlineEl.textContent = currentJoke.punchline;
      punchlineEl.classList.add("visible");
    }
  }, delay);

  log.debug("Showing loading overlay");
}

/**
 * Hide the loading overlay
 * Will wait for minimum display time to avoid flashing
 */
export function hideLoadingOverlay(): void {
  if (!overlayElement) return;

  // Clear punchline timeout
  if (punchlineTimeout) {
    clearTimeout(punchlineTimeout);
    punchlineTimeout = null;
  }

  // Calculate remaining time to meet minimum display
  const elapsed = Date.now() - showStartTime;
  const remaining = Math.max(0, MIN_DISPLAY_TIME - elapsed);

  setTimeout(() => {
    if (overlayElement) {
      overlayElement.classList.remove("visible");
    }
    currentJoke = null;
    log.debug("Hiding loading overlay");
  }, remaining);
}

/**
 * Update the loading status text
 */
export function updateLoadingStatus(statusText: string): void {
  if (!overlayElement) return;

  const statusEl = overlayElement.querySelector(".loading-status");
  if (statusEl) {
    statusEl.textContent = statusText;
  }
}

/**
 * Wrap a fetch call with loading overlay
 * @param fetchFn - The fetch function to wrap
 * @param statusText - Optional status text
 * @returns The fetch result
 */
export async function withLoadingOverlay<T>(
  fetchFn: () => Promise<T>,
  statusText: string = "Loading..."
): Promise<T> {
  showLoadingOverlay(statusText);
  try {
    return await fetchFn();
  } finally {
    hideLoadingOverlay();
  }
}

/** CSS styles for the loading overlay */
const OVERLAY_STYLES = `
.loading-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.95);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  opacity: 0;
  visibility: hidden;
  transition: opacity 0.3s ease, visibility 0.3s ease;
}

.loading-overlay.visible {
  opacity: 1;
  visibility: visible;
}

.loading-content {
  text-align: center;
  max-width: 500px;
  padding: 2rem;
}

.loading-spinner {
  width: 60px;
  height: 60px;
  margin: 0 auto 2rem;
  border: 4px solid #e0e0e0;
  border-top-color: #0056b3;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-joke {
  margin-bottom: 1.5rem;
  min-height: 100px;
}

.joke-setup {
  font-size: 1.25rem;
  color: #333;
  margin-bottom: 1rem;
  line-height: 1.5;
}

.joke-punchline {
  font-size: 1.25rem;
  color: #0056b3;
  font-weight: 600;
  opacity: 0;
  transform: translateY(10px);
  transition: opacity 0.5s ease, transform 0.5s ease;
}

.joke-punchline.visible {
  opacity: 1;
  transform: translateY(0);
}

.loading-status {
  font-size: 1rem;
  color: #666;
  margin-top: 1rem;
}

/* Geriatric-friendly: larger text on smaller screens */
@media (max-width: 600px) {
  .joke-setup,
  .joke-punchline {
    font-size: 1.1rem;
  }

  .loading-content {
    padding: 1.5rem;
  }
}
`;

// Export for window.DocSign namespace
export function initLoadingNamespace(): void {
  if (typeof window !== "undefined" && (window as any).DocSign) {
    const docSign = (window as any).DocSign;
    docSign.showLoadingOverlay = showLoadingOverlay;
    docSign.hideLoadingOverlay = hideLoadingOverlay;
    docSign.updateLoadingStatus = updateLoadingStatus;
    docSign.withLoadingOverlay = withLoadingOverlay;
    log.debug("Loading overlay added to window.DocSign");
  }
}
