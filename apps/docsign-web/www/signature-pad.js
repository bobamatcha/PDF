/**
 * Signature Pad Module
 *
 * Provides both drawing and typing modes for capturing signatures.
 * Exports signature as PNG data URL.
 */

export class SignaturePad {
    constructor(canvasElement, options = {}) {
        this.canvas = canvasElement;
        this.ctx = canvasElement.getContext('2d');

        // Options
        this.penColor = options.penColor || '#000000';
        this.penWidth = options.penWidth || 2;
        this.backgroundColor = options.backgroundColor || 'transparent';

        // State
        this.isDrawing = false;
        this.lastX = 0;
        this.lastY = 0;
        this.points = []; // For bezier smoothing

        this._initCanvas();
        this._bindEvents();
    }

    _initCanvas() {
        // Set canvas size to match display size
        const rect = this.canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        this.canvas.width = rect.width * dpr;
        this.canvas.height = rect.height * dpr;
        this.ctx.scale(dpr, dpr);

        // Style
        this.ctx.strokeStyle = this.penColor;
        this.ctx.lineWidth = this.penWidth;
        this.ctx.lineCap = 'round';
        this.ctx.lineJoin = 'round';

        this.clear();
    }

    _bindEvents() {
        // Mouse events
        this.canvas.addEventListener('mousedown', this._onPointerDown.bind(this));
        this.canvas.addEventListener('mousemove', this._onPointerMove.bind(this));
        this.canvas.addEventListener('mouseup', this._onPointerUp.bind(this));
        this.canvas.addEventListener('mouseleave', this._onPointerUp.bind(this));

        // Touch events
        this.canvas.addEventListener('touchstart', this._onTouchStart.bind(this));
        this.canvas.addEventListener('touchmove', this._onTouchMove.bind(this));
        this.canvas.addEventListener('touchend', this._onPointerUp.bind(this));

        // Prevent scrolling on touch
        this.canvas.addEventListener('touchstart', e => e.preventDefault());
        this.canvas.addEventListener('touchmove', e => e.preventDefault());
    }

    _getPointerPos(e) {
        const rect = this.canvas.getBoundingClientRect();
        return {
            x: e.clientX - rect.left,
            y: e.clientY - rect.top
        };
    }

    _getTouchPos(e) {
        const rect = this.canvas.getBoundingClientRect();
        const touch = e.touches[0];
        return {
            x: touch.clientX - rect.left,
            y: touch.clientY - rect.top
        };
    }

    _onPointerDown(e) {
        this.isDrawing = true;
        const pos = this._getPointerPos(e);
        this.lastX = pos.x;
        this.lastY = pos.y;
        this.points = [pos];

        // Draw a dot for single clicks
        this.ctx.beginPath();
        this.ctx.arc(pos.x, pos.y, this.penWidth / 2, 0, Math.PI * 2);
        this.ctx.fill();
    }

    _onPointerMove(e) {
        if (!this.isDrawing) return;

        const pos = this._getPointerPos(e);
        this.points.push(pos);

        // Use quadratic bezier for smooth curves
        if (this.points.length >= 3) {
            const p1 = this.points[this.points.length - 3];
            const p2 = this.points[this.points.length - 2];
            const p3 = this.points[this.points.length - 1];

            const midX = (p2.x + p3.x) / 2;
            const midY = (p2.y + p3.y) / 2;

            this.ctx.beginPath();
            this.ctx.moveTo(p1.x, p1.y);
            this.ctx.quadraticCurveTo(p2.x, p2.y, midX, midY);
            this.ctx.stroke();
        } else {
            // Simple line for first points
            this.ctx.beginPath();
            this.ctx.moveTo(this.lastX, this.lastY);
            this.ctx.lineTo(pos.x, pos.y);
            this.ctx.stroke();
        }

        this.lastX = pos.x;
        this.lastY = pos.y;
    }

    _onPointerUp() {
        this.isDrawing = false;
        this.points = [];
    }

    _onTouchStart(e) {
        this.isDrawing = true;
        const pos = this._getTouchPos(e);
        this.lastX = pos.x;
        this.lastY = pos.y;
        this.points = [pos];
    }

    _onTouchMove(e) {
        if (!this.isDrawing) return;

        const pos = this._getTouchPos(e);
        this.points.push(pos);

        if (this.points.length >= 3) {
            const p1 = this.points[this.points.length - 3];
            const p2 = this.points[this.points.length - 2];
            const p3 = this.points[this.points.length - 1];

            const midX = (p2.x + p3.x) / 2;
            const midY = (p2.y + p3.y) / 2;

            this.ctx.beginPath();
            this.ctx.moveTo(p1.x, p1.y);
            this.ctx.quadraticCurveTo(p2.x, p2.y, midX, midY);
            this.ctx.stroke();
        } else {
            this.ctx.beginPath();
            this.ctx.moveTo(this.lastX, this.lastY);
            this.ctx.lineTo(pos.x, pos.y);
            this.ctx.stroke();
        }

        this.lastX = pos.x;
        this.lastY = pos.y;
    }

    clear() {
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        if (this.backgroundColor !== 'transparent') {
            this.ctx.fillStyle = this.backgroundColor;
            this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
        }
        this.ctx.fillStyle = this.penColor;
    }

    isEmpty() {
        const data = this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height).data;
        // Check if any pixel has non-zero alpha
        for (let i = 3; i < data.length; i += 4) {
            if (data[i] !== 0) return false;
        }
        return true;
    }

    toDataURL(type = 'image/png') {
        return this.canvas.toDataURL(type);
    }

    toBlob(callback, type = 'image/png') {
        this.canvas.toBlob(callback, type);
    }

    setPenColor(color) {
        this.penColor = color;
        this.ctx.strokeStyle = color;
        this.ctx.fillStyle = color;
    }

    setPenWidth(width) {
        this.penWidth = width;
        this.ctx.lineWidth = width;
    }
}

/**
 * Typed Signature Generator
 * Renders text in cursive font to canvas
 */
export class TypedSignature {
    constructor(previewElement, options = {}) {
        this.preview = previewElement;
        this.fonts = options.fonts || [
            'Dancing Script',
            'Great Vibes',
            'Allura',
            'Sacramento',
            'Pacifico'
        ];
        this.currentFont = this.fonts[0];
        this.fontSize = options.fontSize || 48;
        this.color = options.color || '#000000';
        this.text = '';
    }

    setText(text) {
        this.text = text;
        this._updatePreview();
    }

    setFont(fontName) {
        if (this.fonts.includes(fontName)) {
            this.currentFont = fontName;
            this._updatePreview();
        }
    }

    _updatePreview() {
        this.preview.textContent = this.text;
        this.preview.style.fontFamily = `'${this.currentFont}', cursive`;
        this.preview.style.fontSize = `${this.fontSize}px`;
        this.preview.style.color = this.color;
    }

    toCanvas(width = 400, height = 100) {
        const canvas = document.createElement('canvas');
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext('2d');

        // Clear with transparent background
        ctx.clearRect(0, 0, width, height);

        // Draw text
        ctx.font = `${this.fontSize}px '${this.currentFont}', cursive`;
        ctx.fillStyle = this.color;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(this.text, width / 2, height / 2);

        return canvas;
    }

    toDataURL(width = 400, height = 100) {
        return this.toCanvas(width, height).toDataURL('image/png');
    }

    isEmpty() {
        return !this.text || this.text.trim() === '';
    }
}

/**
 * Signature Modal Controller
 * Manages the signature capture modal UI
 */
export class SignatureModal {
    constructor(modalElement, options = {}) {
        this.modal = modalElement;
        this.onApply = options.onApply || (() => {});
        this.onCancel = options.onCancel || (() => {});

        // Get elements
        this.drawTab = modalElement.querySelector('[data-tab="draw"]');
        this.typeTab = modalElement.querySelector('[data-tab="type"]');
        this.drawPanel = modalElement.querySelector('#draw-tab');
        this.typePanel = modalElement.querySelector('#type-tab');
        this.canvas = modalElement.querySelector('#signature-pad');
        this.typedInput = modalElement.querySelector('#typed-name');
        this.fontSelector = modalElement.querySelector('#font-selector');
        this.preview = modalElement.querySelector('#cursive-preview');
        this.clearBtn = modalElement.querySelector('#clear-signature');
        this.applyBtn = modalElement.querySelector('#apply-signature');
        this.cancelBtn = modalElement.querySelector('#cancel-signature');

        // Initialize components
        if (this.canvas) {
            this.signaturePad = new SignaturePad(this.canvas);
        }
        if (this.preview) {
            this.typedSignature = new TypedSignature(this.preview);
        }

        // Current mode
        this.mode = 'draw';
        this.currentFieldId = null;

        this._bindEvents();
    }

    _bindEvents() {
        // Tab switching
        this.drawTab?.addEventListener('click', () => this._switchTab('draw'));
        this.typeTab?.addEventListener('click', () => this._switchTab('type'));

        // Clear button
        this.clearBtn?.addEventListener('click', () => this.signaturePad?.clear());

        // Typed input
        this.typedInput?.addEventListener('input', (e) => {
            this.typedSignature?.setText(e.target.value);
        });

        // Font selector
        this.fontSelector?.addEventListener('change', (e) => {
            this.typedSignature?.setFont(e.target.value);
        });

        // Apply button
        this.applyBtn?.addEventListener('click', () => this._apply());

        // Cancel button
        this.cancelBtn?.addEventListener('click', () => this.hide());

        // Close on backdrop click
        this.modal?.addEventListener('click', (e) => {
            if (e.target === this.modal) this.hide();
        });

        // Close on Escape
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && !this.modal.classList.contains('hidden')) {
                this.hide();
            }
        });
    }

    _switchTab(tab) {
        this.mode = tab;

        if (tab === 'draw') {
            this.drawTab?.classList.add('active');
            this.typeTab?.classList.remove('active');
            this.drawPanel?.classList.remove('hidden');
            this.typePanel?.classList.add('hidden');
        } else {
            this.typeTab?.classList.add('active');
            this.drawTab?.classList.remove('active');
            this.typePanel?.classList.remove('hidden');
            this.drawPanel?.classList.add('hidden');
        }
    }

    _apply() {
        let signatureData = null;

        if (this.mode === 'draw') {
            if (this.signaturePad?.isEmpty()) {
                alert('Please draw your signature');
                return;
            }
            signatureData = this.signaturePad.toDataURL();
        } else {
            if (this.typedSignature?.isEmpty()) {
                alert('Please type your name');
                return;
            }
            signatureData = this.typedSignature.toDataURL();
        }

        this.onApply({
            fieldId: this.currentFieldId,
            signatureData,
            mode: this.mode,
            text: this.mode === 'type' ? this.typedSignature.text : null,
            font: this.mode === 'type' ? this.typedSignature.currentFont : null
        });

        this.hide();
    }

    show(fieldId) {
        this.currentFieldId = fieldId;
        this.modal.classList.remove('hidden');
        this._switchTab('draw');
        this.signaturePad?.clear();
        if (this.typedInput) this.typedInput.value = '';
        if (this.typedSignature) this.typedSignature.setText('');
    }

    hide() {
        this.modal.classList.add('hidden');
        this.onCancel();
    }
}

// Export for window access in non-module contexts
if (typeof window !== 'undefined') {
    window.SignaturePad = SignaturePad;
    window.TypedSignature = TypedSignature;
    window.SignatureModal = SignatureModal;
}
