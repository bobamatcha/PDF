// PDF.js bridge for Rust/WASM integration
// Uses lazy loading to avoid bloating the main bundle

// Lazy loading state
let pdfJsLoaded = false;
let pdfJsLoadPromise = null;

export async function ensurePdfJsLoaded() {
    if (pdfJsLoaded) return;
    if (pdfJsLoadPromise) return pdfJsLoadPromise;

    pdfJsLoadPromise = new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = './js/vendor/pdf.min.js';
        script.onload = () => {
            window.pdfjsLib.GlobalWorkerOptions.workerSrc = './js/vendor/pdf.worker.min.js';
            pdfJsLoaded = true;
            console.log('PDF.js loaded (lazy)');
            resolve();
        };
        script.onerror = (e) => {
            pdfJsLoadPromise = null;
            reject(new Error('Failed to load PDF.js'));
        };
        document.head.appendChild(script);
    });

    return pdfJsLoadPromise;
}

export const PdfBridge = {
    currentDoc: null,
    pageCanvases: new Map(),

    async loadDocument(data) {
        // Lazy load PDF.js on first use
        await ensurePdfJsLoaded();

        const typedArray = new Uint8Array(data);
        this.currentDoc = await pdfjsLib.getDocument(typedArray).promise;
        return this.currentDoc.numPages;
    },

    async renderPage(pageNum, canvas, scale = 1.5) {
        if (!this.currentDoc) throw new Error('No document loaded');

        const page = await this.currentDoc.getPage(pageNum);
        const viewport = page.getViewport({ scale });

        canvas.width = viewport.width;
        canvas.height = viewport.height;

        const ctx = canvas.getContext('2d');
        await page.render({
            canvasContext: ctx,
            viewport: viewport
        }).promise;

        this.pageCanvases.set(pageNum, { canvas, viewport, page });

        return {
            width: viewport.width,
            height: viewport.height,
            originalWidth: viewport.width / scale,
            originalHeight: viewport.height / scale,
            pdfWidth: page.view[2],
            pdfHeight: page.view[3]
        };
    },

    getPageDimensions(pageNum) {
        const cached = this.pageCanvases.get(pageNum);
        if (cached) {
            return {
                width: cached.viewport.width,
                height: cached.viewport.height
            };
        }
        return null;
    },

    getPageInfo(pageNum) {
        return this.pageCanvases.get(pageNum);
    },

    async extractText(pageNum) {
        if (!this.currentDoc) throw new Error('No document loaded');

        const page = await this.currentDoc.getPage(pageNum);
        const textContent = await page.getTextContent();
        return textContent.items.map(item => item.str).join(' ');
    },

    async extractTextWithPositions(pageNum) {
        if (!this.currentDoc) throw new Error('No document loaded');

        const page = await this.currentDoc.getPage(pageNum);
        const textContent = await page.getTextContent();
        const cached = this.pageCanvases.get(pageNum);
        const viewport = cached?.viewport;

        // Get font styles map (fontName -> { fontFamily, ascent, descent, vertical })
        const styles = textContent.styles || {};

        return textContent.items.map((item, index) => {
            // PDF.js transform: [scaleX, skewX, skewY, scaleY, x, y]
            const pdfX = item.transform[4];
            const pdfY = item.transform[5];
            const pdfWidth = item.width || 0;
            const pdfHeight = item.height || 12; // Default font height

            // Font size is the absolute value of the scaleY component (transform[3])
            // This represents how tall the font renders in PDF points
            const fontSize = Math.abs(item.transform[3]) || item.height || 12;

            // Get font family from styles (e.g., "serif", "sans-serif", "monospace")
            const fontStyle = styles[item.fontName];
            const fontFamily = fontStyle?.fontFamily || 'sans-serif';

            // Detect italic/bold from font name
            // Font names often contain style info: "Times-Italic", "Helvetica-Bold", etc.
            const fontNameLower = (item.fontName || '').toLowerCase();
            const isItalic = fontNameLower.includes('italic') || fontNameLower.includes('oblique');
            const isBold = fontNameLower.includes('bold');

            // Convert PDF coords to DOM coords if viewport available
            let domBounds = null;
            let domFontSize = fontSize; // DOM-scaled font size
            if (viewport) {
                // PDF origin is bottom-left, viewport is top-left
                const [domX, domY] = viewport.convertToViewportPoint(pdfX, pdfY);
                const [domX2, domY2] = viewport.convertToViewportPoint(pdfX + pdfWidth, pdfY + pdfHeight);
                domBounds = {
                    x: Math.min(domX, domX2),
                    y: Math.min(domY, domY2),
                    width: Math.abs(domX2 - domX) || pdfWidth * viewport.scale,
                    height: Math.abs(domY2 - domY) || pdfHeight * viewport.scale
                };
                // Scale font size to match viewport
                domFontSize = fontSize * viewport.scale;
            }

            return {
                index,
                str: item.str,
                pdfX,
                pdfY,
                pdfWidth,
                pdfHeight,
                fontSize,      // PDF font size in points
                domFontSize,   // Font size scaled to viewport (pixels)
                fontName: item.fontName,
                fontFamily, // "serif", "sans-serif", or "monospace"
                isItalic,   // true if font name contains "italic" or "oblique"
                isBold,     // true if font name contains "bold"
                domBounds
            };
        });
    },

    async extractAllText() {
        if (!this.currentDoc) throw new Error('No document loaded');

        const texts = [];
        for (let i = 1; i <= this.currentDoc.numPages; i++) {
            texts.push(await this.extractText(i));
        }
        return texts;
    },

    cleanup() {
        if (this.currentDoc) {
            this.currentDoc.destroy();
            this.currentDoc = null;
        }
        this.pageCanvases.clear();
    }
};

// Also expose on window for backwards compatibility
window.ensurePdfJsLoaded = ensurePdfJsLoaded;
window.PdfBridge = PdfBridge;
