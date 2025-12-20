// PDF.js bridge for Rust/WASM integration
window.PdfBridge = {
    currentDoc: null,
    pageCanvases: new Map(),

    async loadDocument(data) {
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

        this.pageCanvases.set(pageNum, { canvas, viewport });

        return {
            width: viewport.width,
            height: viewport.height,
            originalWidth: viewport.width / scale,
            originalHeight: viewport.height / scale
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

    async extractText(pageNum) {
        if (!this.currentDoc) throw new Error('No document loaded');

        const page = await this.currentDoc.getPage(pageNum);
        const textContent = await page.getTextContent();
        return textContent.items.map(item => item.str).join(' ');
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
