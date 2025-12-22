/**
 * Guided Signing Flow Module
 *
 * Provides step-by-step navigation through signature fields.
 * Handles Next/Back buttons, progress indicator, and field highlighting.
 */

export class GuidedSigningFlow {
    constructor(options = {}) {
        // Fields for this signer
        this.fields = [];
        this.currentIndex = 0;
        this.active = false;

        // Callbacks
        this.onFieldChange = options.onFieldChange || (() => {});
        this.onComplete = options.onComplete || (() => {});
        this.onFieldClick = options.onFieldClick || (() => {});

        // UI Elements
        this.elements = {
            startBtn: document.getElementById('btn-start'),
            prevBtn: document.getElementById('btn-prev'),
            nextBtn: document.getElementById('btn-next'),
            finishBtn: document.getElementById('btn-finish'),
            currentSpan: document.getElementById('current'),
            totalSpan: document.getElementById('total'),
            progress: document.querySelector('.progress'),
            navButtons: document.querySelector('.nav-buttons')
        };

        this._bindEvents();
    }

    /**
     * Initialize with fields for a specific recipient
     */
    init(fields, recipientId) {
        // Filter fields for this recipient and sort by page, then y position
        this.fields = fields
            .filter(f => f.recipientId === recipientId)
            .sort((a, b) => {
                if (a.page !== b.page) return a.page - b.page;
                return a.y - b.y;
            });

        this.currentIndex = 0;
        this.active = false;

        // Update total count
        if (this.elements.totalSpan) {
            this.elements.totalSpan.textContent = this.fields.length;
        }

        // Set up field click handlers
        this._setupFieldClickHandlers();

        return this;
    }

    _bindEvents() {
        this.elements.startBtn?.addEventListener('click', () => this.start());
        this.elements.prevBtn?.addEventListener('click', () => this.back());
        this.elements.nextBtn?.addEventListener('click', () => this.next());
        this.elements.finishBtn?.addEventListener('click', () => this.finish());

        // Keyboard navigation
        document.addEventListener('keydown', (e) => {
            if (!this.active) return;

            if (e.key === 'ArrowRight' || e.key === 'Tab' && !e.shiftKey) {
                e.preventDefault();
                this.next();
            } else if (e.key === 'ArrowLeft' || e.key === 'Tab' && e.shiftKey) {
                e.preventDefault();
                this.back();
            } else if (e.key === 'Enter') {
                const currentField = this.getCurrentField();
                if (currentField) {
                    this.onFieldClick(currentField);
                }
            }
        });

        // Swipe gesture navigation (UX-005 mobile optimization)
        this._initSwipeGestures();
    }

    /**
     * Initialize swipe gesture navigation for mobile
     * Swipe left = next field, Swipe right = previous field
     */
    _initSwipeGestures() {
        const viewerContainer = document.querySelector('.viewer-container');
        if (!viewerContainer) return;

        let touchStartX = 0;
        let touchStartY = 0;
        let touchEndX = 0;
        let touchEndY = 0;

        const minSwipeDistance = 50; // Minimum distance for swipe
        const maxVerticalDistance = 100; // Maximum vertical movement to count as horizontal swipe

        viewerContainer.addEventListener('touchstart', (e) => {
            if (!this.active) return;

            // Ignore if touching an input or button
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'BUTTON' || e.target.tagName === 'CANVAS') {
                return;
            }

            touchStartX = e.changedTouches[0].screenX;
            touchStartY = e.changedTouches[0].screenY;
        }, { passive: true });

        viewerContainer.addEventListener('touchend', (e) => {
            if (!this.active) return;

            // Ignore if touching an input or button
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'BUTTON' || e.target.tagName === 'CANVAS') {
                return;
            }

            touchEndX = e.changedTouches[0].screenX;
            touchEndY = e.changedTouches[0].screenY;

            const horizontalDistance = touchEndX - touchStartX;
            const verticalDistance = Math.abs(touchEndY - touchStartY);

            // Only process if mostly horizontal swipe
            if (verticalDistance > maxVerticalDistance) {
                return;
            }

            // Swipe left (next)
            if (horizontalDistance < -minSwipeDistance) {
                this.next();
            }
            // Swipe right (previous)
            else if (horizontalDistance > minSwipeDistance) {
                this.back();
            }
        }, { passive: true });

        // Mark container as swipe-enabled for tests
        viewerContainer.dataset.swipeEnabled = 'true';
    }

    _setupFieldClickHandlers() {
        this.fields.forEach((field, index) => {
            const el = document.querySelector(`[data-field-id="${field.id}"]`);
            if (el) {
                el.dataset.index = index + 1;
                el.addEventListener('click', () => {
                    if (this.active) {
                        this.goToField(index);
                    }
                    this.onFieldClick(field);
                });
            }
        });
    }

    /**
     * Start the guided flow
     */
    start() {
        if (this.fields.length === 0) {
            console.warn('No fields to sign');
            return;
        }

        this.active = true;
        this.currentIndex = 0;

        // Show navigation UI
        this.elements.startBtn?.classList.add('hidden');
        this.elements.progress?.classList.remove('hidden');
        this.elements.navButtons?.classList.remove('hidden');
        this.elements.finishBtn?.classList.remove('hidden');

        this._updateUI();
        this._scrollToCurrentField();
        this._highlightCurrentField();

        // Open first field for input
        const firstField = this.getCurrentField();
        if (firstField) {
            this.onFieldClick(firstField);
        }
    }

    /**
     * Go to next field
     */
    next() {
        if (!this.active) return;
        if (this.currentIndex >= this.fields.length - 1) return;

        this.currentIndex++;
        this._updateUI();
        this._scrollToCurrentField();
        this._highlightCurrentField();

        this.onFieldChange(this.getCurrentField(), this.currentIndex);
    }

    /**
     * Go to previous field
     */
    back() {
        if (!this.active) return;
        if (this.currentIndex <= 0) return;

        this.currentIndex--;
        this._updateUI();
        this._scrollToCurrentField();
        this._highlightCurrentField();

        this.onFieldChange(this.getCurrentField(), this.currentIndex);
    }

    /**
     * Jump to specific field
     */
    goToField(index) {
        if (!this.active) return;
        if (index < 0 || index >= this.fields.length) return;

        this.currentIndex = index;
        this._updateUI();
        this._scrollToCurrentField();
        this._highlightCurrentField();

        this.onFieldChange(this.getCurrentField(), this.currentIndex);
    }

    /**
     * Get current field
     */
    getCurrentField() {
        return this.fields[this.currentIndex] || null;
    }

    /**
     * Mark field as completed
     */
    markFieldComplete(fieldId) {
        const field = this.fields.find(f => f.id === fieldId);
        if (field) {
            field.completed = true;
            const el = document.querySelector(`[data-field-id="${fieldId}"]`);
            if (el) {
                el.classList.add('completed');
                el.dataset.signed = 'true';
            }
        }

        this._updateFinishButton();
    }

    /**
     * Check if all required fields are complete
     */
    canFinish() {
        return this.fields
            .filter(f => f.required)
            .every(f => f.completed);
    }

    /**
     * Finish signing
     */
    finish() {
        if (!this.canFinish()) {
            // Find first incomplete required field
            const incomplete = this.fields.findIndex(f => f.required && !f.completed);
            if (incomplete >= 0) {
                this.goToField(incomplete);
                alert('Please complete all required fields before finishing.');
                return;
            }
        }

        this.active = false;
        this.onComplete(this.fields);
    }

    _updateUI() {
        // Update progress
        if (this.elements.currentSpan) {
            this.elements.currentSpan.textContent = this.currentIndex + 1;
        }

        // Update button states
        if (this.elements.prevBtn) {
            this.elements.prevBtn.disabled = this.currentIndex === 0;
        }
        if (this.elements.nextBtn) {
            this.elements.nextBtn.disabled = this.currentIndex >= this.fields.length - 1;
        }

        this._updateFinishButton();
    }

    _updateFinishButton() {
        if (this.elements.finishBtn) {
            this.elements.finishBtn.disabled = !this.canFinish();
        }
    }

    _scrollToCurrentField() {
        const field = this.getCurrentField();
        if (!field) return;

        const el = document.querySelector(`[data-field-id="${field.id}"]`);
        if (el) {
            el.scrollIntoView({
                behavior: 'smooth',
                block: 'center',
                inline: 'center'
            });
        }
    }

    _highlightCurrentField() {
        // Remove highlight from all fields
        document.querySelectorAll('.field-overlay').forEach(el => {
            el.classList.remove('current');
        });

        // Add highlight to current field
        const field = this.getCurrentField();
        if (field) {
            const el = document.querySelector(`[data-field-id="${field.id}"]`);
            if (el) {
                el.classList.add('current');
            }
        }
    }

    /**
     * Reset the flow
     */
    reset() {
        this.active = false;
        this.currentIndex = 0;
        this.fields.forEach(f => f.completed = false);

        // Reset UI
        this.elements.startBtn?.classList.remove('hidden');
        this.elements.progress?.classList.add('hidden');
        this.elements.navButtons?.classList.add('hidden');
        this.elements.finishBtn?.classList.add('hidden');

        // Remove highlights
        document.querySelectorAll('.field-overlay').forEach(el => {
            el.classList.remove('current', 'completed');
            el.dataset.signed = 'false';
        });
    }
}

// Export for window access
if (typeof window !== 'undefined') {
    window.GuidedSigningFlow = GuidedSigningFlow;
}
