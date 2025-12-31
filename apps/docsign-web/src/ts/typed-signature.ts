/**
 * Typed Signature Component
 *
 * Provides a typed signature option for users who have difficulty drawing
 * with mouse/finger. Renders text in cursive fonts for a natural signature look.
 *
 * Designed with geriatric UX principles:
 * - Large touch targets (60px minimum)
 * - High contrast text
 * - Clear visual font previews
 * - Real-time feedback
 *
 * Legally valid under ESIGN Act and UETA.
 */

/**
 * Available signature fonts (Google Fonts, pre-loaded)
 */
export const SIGNATURE_FONTS = [
  { name: "Dancing Script", label: "Classic Cursive", style: "flowing" },
  { name: "Great Vibes", label: "Elegant Script", style: "formal" },
  { name: "Pacifico", label: "Casual Handwriting", style: "casual" },
  { name: "Sacramento", label: "Flowing Script", style: "flowing" },
  { name: "Allura", label: "Formal Calligraphy", style: "calligraphy" },
] as const;

export type SignatureFontName = (typeof SIGNATURE_FONTS)[number]["name"];

/**
 * Configuration options for TypedSignature
 */
export interface TypedSignatureOptions {
  /** Container element to render into */
  container: HTMLElement;
  /** Available fonts (defaults to all SIGNATURE_FONTS) */
  fonts?: string[];
  /** Default font family */
  defaultFont?: string;
  /** Font size in pixels for rendering */
  fontSize?: number;
  /** Text color (default: navy #000080 for legibility) */
  textColor?: string;
  /** Background color for canvas (default: white) */
  backgroundColor?: string;
  /** Placeholder text for input */
  placeholder?: string;
  /** Callback when signature changes */
  onChange?: (text: string, font: string) => void;
}

/**
 * TypedSignature class
 *
 * Creates a typed signature input with font selection and live preview.
 * Renders text to canvas for export as PNG data URL.
 */
export class TypedSignature {
  private container: HTMLElement;
  private fonts: string[];
  private currentFont: string;
  private fontSize: number;
  private textColor: string;
  private backgroundColor: string;
  private placeholder: string;
  private onChange?: (text: string, font: string) => void;

  // DOM elements
  private inputElement: HTMLInputElement | null = null;
  private fontSelectorContainer: HTMLElement | null = null;
  private previewCanvas: HTMLCanvasElement | null = null;
  private previewContainer: HTMLElement | null = null;

  // State
  private text: string = "";
  private destroyed: boolean = false;

  constructor(options: TypedSignatureOptions) {
    this.container = options.container;
    this.fonts = options.fonts || SIGNATURE_FONTS.map((f) => f.name);
    this.currentFont = options.defaultFont || "Dancing Script";
    this.fontSize = options.fontSize || 48;
    this.textColor = options.textColor || "#000080";
    this.backgroundColor = options.backgroundColor || "#ffffff";
    this.placeholder = options.placeholder || "Type your name";
    this.onChange = options.onChange;

    this.render();
  }

  /**
   * Set the signature text
   */
  setText(text: string): void {
    this.text = text;
    if (this.inputElement && this.inputElement.value !== text) {
      this.inputElement.value = text;
    }
    this.updatePreview();
    this.onChange?.(this.text, this.currentFont);
  }

  /**
   * Get the current signature text
   */
  getText(): string {
    return this.text;
  }

  /**
   * Set the current font
   */
  setFont(fontFamily: string): void {
    if (this.fonts.includes(fontFamily)) {
      this.currentFont = fontFamily;
      this.updateFontSelection();
      this.updatePreview();
      this.onChange?.(this.text, this.currentFont);
    }
  }

  /**
   * Get the current font
   */
  getFont(): string {
    return this.currentFont;
  }

  /**
   * Check if signature is empty
   */
  isEmpty(): boolean {
    return !this.text || this.text.trim() === "";
  }

  /**
   * Export signature as data URL
   */
  toDataURL(): string {
    const canvas = this.toCanvas();
    return canvas.toDataURL("image/png");
  }

  /**
   * Export signature as canvas
   */
  toCanvas(): HTMLCanvasElement {
    // Create high-resolution canvas for crisp output
    const dpr = window.devicePixelRatio || 1;
    const width = 400;
    const height = 100;

    const canvas = document.createElement("canvas");
    canvas.width = width * dpr;
    canvas.height = height * dpr;

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Could not get canvas 2D context");
    }

    // Scale for high DPI
    ctx.scale(dpr, dpr);

    // Clear with background color
    ctx.fillStyle = this.backgroundColor;
    ctx.fillRect(0, 0, width, height);

    if (this.text.trim()) {
      // Configure text rendering
      ctx.font = `${this.fontSize}px '${this.currentFont}', cursive`;
      ctx.fillStyle = this.textColor;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";

      // Add slight baseline adjustment for better centering
      const textMetrics = ctx.measureText(this.text);
      const actualHeight =
        textMetrics.actualBoundingBoxAscent + textMetrics.actualBoundingBoxDescent;
      const yOffset = (height - actualHeight) / 2 + textMetrics.actualBoundingBoxAscent;

      // Draw the text centered
      ctx.fillText(this.text, width / 2, yOffset || height / 2, width - 20);
    }

    return canvas;
  }

  /**
   * Clean up and remove the component
   */
  destroy(): void {
    this.destroyed = true;
    this.container.innerHTML = "";
    this.inputElement = null;
    this.fontSelectorContainer = null;
    this.previewCanvas = null;
    this.previewContainer = null;
  }

  /**
   * Render the component into the container
   */
  private render(): void {
    // Clear container
    this.container.innerHTML = "";

    // Create wrapper with geriatric UX styling
    const wrapper = document.createElement("div");
    wrapper.className = "typed-signature-wrapper";
    wrapper.style.cssText = `
      display: flex;
      flex-direction: column;
      gap: 1.5rem;
    `;

    // Create large text input
    this.inputElement = this.createInput();
    wrapper.appendChild(this.inputElement);

    // Create font selector with visual previews
    const fontSection = document.createElement("div");
    fontSection.className = "typed-signature-font-section";

    const fontLabel = document.createElement("label");
    fontLabel.textContent = "Choose a style for your signature";
    fontLabel.style.cssText = `
      display: block;
      font-size: 18px;
      font-weight: 600;
      color: #111827;
      margin-bottom: 1rem;
    `;
    fontSection.appendChild(fontLabel);

    this.fontSelectorContainer = this.createFontSelector();
    fontSection.appendChild(this.fontSelectorContainer);
    wrapper.appendChild(fontSection);

    // Create preview section
    this.previewContainer = this.createPreviewSection();
    wrapper.appendChild(this.previewContainer);

    this.container.appendChild(wrapper);

    // Initial preview update
    this.updatePreview();
  }

  /**
   * Create the text input element
   */
  private createInput(): HTMLInputElement {
    const input = document.createElement("input");
    input.type = "text";
    input.placeholder = this.placeholder;
    input.className = "typed-signature-input";
    input.autocomplete = "name";
    input.autocapitalize = "words";

    // Geriatric UX: Large input (60px height, 24px font)
    input.style.cssText = `
      width: 100%;
      min-height: 60px;
      padding: 16px 20px;
      border: 2px solid #d1d5db;
      border-radius: 8px;
      font-size: 24px;
      font-family: inherit;
      text-align: center;
      color: #111827;
      background: #ffffff;
      transition: border-color 0.2s, box-shadow 0.2s;
      outline: none;
    `;

    // Focus states
    input.addEventListener("focus", () => {
      input.style.borderColor = "#1e40af";
      input.style.boxShadow = "0 0 0 3px rgba(30, 64, 175, 0.1)";
    });

    input.addEventListener("blur", () => {
      input.style.borderColor = "#d1d5db";
      input.style.boxShadow = "none";
    });

    // Handle input changes
    input.addEventListener("input", () => {
      this.text = input.value;
      this.updatePreview();
      this.updateFontPreviews();
      this.onChange?.(this.text, this.currentFont);
    });

    return input;
  }

  /**
   * Create the font selector with visual previews
   */
  private createFontSelector(): HTMLElement {
    const container = document.createElement("div");
    container.className = "typed-signature-fonts";
    container.setAttribute("role", "radiogroup");
    container.setAttribute("aria-label", "Signature style");
    container.style.cssText = `
      display: flex;
      flex-direction: column;
      gap: 12px;
    `;

    this.fonts.forEach((fontName, index) => {
      const fontOption = this.createFontOption(fontName, index);
      container.appendChild(fontOption);
    });

    return container;
  }

  /**
   * Create a single font option with radio button and preview
   */
  private createFontOption(fontName: string, index: number): HTMLElement {
    const fontInfo = SIGNATURE_FONTS.find((f) => f.name === fontName);
    const label = fontInfo?.label || fontName;

    const optionContainer = document.createElement("label");
    optionContainer.className = "typed-signature-font-option";
    optionContainer.style.cssText = `
      display: flex;
      align-items: center;
      gap: 16px;
      padding: 16px;
      border: 2px solid ${this.currentFont === fontName ? "#1e40af" : "#e5e7eb"};
      border-radius: 12px;
      cursor: pointer;
      background: ${this.currentFont === fontName ? "rgba(30, 64, 175, 0.05)" : "#ffffff"};
      transition: all 0.2s;
    `;

    // Hover effect
    optionContainer.addEventListener("mouseenter", () => {
      if (this.currentFont !== fontName) {
        optionContainer.style.borderColor = "#9ca3af";
        optionContainer.style.background = "#f9fafb";
      }
    });

    optionContainer.addEventListener("mouseleave", () => {
      if (this.currentFont !== fontName) {
        optionContainer.style.borderColor = "#e5e7eb";
        optionContainer.style.background = "#ffffff";
      }
    });

    // Radio button - Geriatric UX: 32px size
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "signature-font";
    radio.value = fontName;
    radio.checked = this.currentFont === fontName;
    radio.id = `font-${index}`;
    radio.style.cssText = `
      width: 32px;
      height: 32px;
      margin: 0;
      cursor: pointer;
      accent-color: #1e40af;
      flex-shrink: 0;
    `;

    radio.addEventListener("change", () => {
      if (radio.checked) {
        this.setFont(fontName);
      }
    });

    // Font preview area
    const previewArea = document.createElement("div");
    previewArea.className = "font-preview-area";
    previewArea.style.cssText = `
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 4px;
    `;

    // Font label
    const fontLabel = document.createElement("span");
    fontLabel.textContent = label;
    fontLabel.style.cssText = `
      font-size: 14px;
      color: #6b7280;
      font-weight: 500;
    `;

    // Font preview text (shows user's name in this font)
    const preview = document.createElement("span");
    preview.className = "font-preview-text";
    preview.dataset.font = fontName;
    preview.textContent = this.text || "Your Name";
    preview.style.cssText = `
      font-family: '${fontName}', cursive;
      font-size: 32px;
      color: ${this.textColor};
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    `;

    previewArea.appendChild(fontLabel);
    previewArea.appendChild(preview);

    optionContainer.appendChild(radio);
    optionContainer.appendChild(previewArea);

    return optionContainer;
  }

  /**
   * Create the preview section showing the final signature look
   */
  private createPreviewSection(): HTMLElement {
    const container = document.createElement("div");
    container.className = "typed-signature-preview-section";
    container.style.cssText = `
      margin-top: 0.5rem;
    `;

    const label = document.createElement("label");
    label.textContent = "Your signature will look like:";
    label.style.cssText = `
      display: block;
      font-size: 16px;
      color: #6b7280;
      margin-bottom: 0.75rem;
    `;
    container.appendChild(label);

    // Preview canvas container
    const canvasContainer = document.createElement("div");
    canvasContainer.style.cssText = `
      background: #ffffff;
      border: 2px solid #e5e7eb;
      border-radius: 12px;
      padding: 1.5rem;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100px;
    `;

    this.previewCanvas = document.createElement("canvas");
    this.previewCanvas.width = 400;
    this.previewCanvas.height = 100;
    this.previewCanvas.style.cssText = `
      max-width: 100%;
      height: auto;
      display: block;
    `;

    canvasContainer.appendChild(this.previewCanvas);
    container.appendChild(canvasContainer);

    return container;
  }

  /**
   * Update the preview canvas with current text and font
   */
  private updatePreview(): void {
    if (this.destroyed || !this.previewCanvas) return;

    const ctx = this.previewCanvas.getContext("2d");
    if (!ctx) return;

    const width = this.previewCanvas.width;
    const height = this.previewCanvas.height;

    // Clear canvas
    ctx.fillStyle = this.backgroundColor;
    ctx.fillRect(0, 0, width, height);

    if (this.text.trim()) {
      // Draw signature text
      ctx.font = `${this.fontSize}px '${this.currentFont}', cursive`;
      ctx.fillStyle = this.textColor;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";

      // Center text
      ctx.fillText(this.text, width / 2, height / 2, width - 20);
    } else {
      // Show placeholder
      ctx.font = "16px system-ui, sans-serif";
      ctx.fillStyle = "#9ca3af";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("Type your name above to see preview", width / 2, height / 2);
    }
  }

  /**
   * Update font previews to show user's typed name
   */
  private updateFontPreviews(): void {
    if (this.destroyed || !this.fontSelectorContainer) return;

    const previews = this.fontSelectorContainer.querySelectorAll(".font-preview-text");
    previews.forEach((preview) => {
      (preview as HTMLElement).textContent = this.text || "Your Name";
    });
  }

  /**
   * Update font selection styling
   */
  private updateFontSelection(): void {
    if (this.destroyed || !this.fontSelectorContainer) return;

    const options = this.fontSelectorContainer.querySelectorAll(".typed-signature-font-option");
    options.forEach((option) => {
      const radio = option.querySelector("input[type='radio']") as HTMLInputElement;
      const isSelected = radio?.value === this.currentFont;

      if (radio) {
        radio.checked = isSelected;
      }

      (option as HTMLElement).style.borderColor = isSelected ? "#1e40af" : "#e5e7eb";
      (option as HTMLElement).style.background = isSelected
        ? "rgba(30, 64, 175, 0.05)"
        : "#ffffff";
    });
  }
}

/**
 * Create a TypedSignature instance
 */
export function createTypedSignature(options: TypedSignatureOptions): TypedSignature {
  return new TypedSignature(options);
}

// Export for window access
if (typeof window !== "undefined") {
  (window as unknown as { TypedSignature: typeof TypedSignature }).TypedSignature =
    TypedSignature;
  (
    window as unknown as { createTypedSignature: typeof createTypedSignature }
  ).createTypedSignature = createTypedSignature;
  (window as unknown as { SIGNATURE_FONTS: typeof SIGNATURE_FONTS }).SIGNATURE_FONTS =
    SIGNATURE_FONTS;
}
