/**
 * DocSign Handoff Module
 *
 * Handles transferring documents from agentPDF.org to getsignatures.org
 * using sessionStorage for same-origin transfers or URL parameters for cross-origin.
 */

const DocsignHandoff = {
    // Configuration - override these for production
    config: {
        // Development: relative path
        // Production: 'https://getsignatures.org'
        docsignBaseUrl: window.location.hostname === 'localhost'
            ? '/apps/docsign-web/www/index.html'
            : window.location.hostname === 'agentpdf.org'
                ? 'https://getsignatures.org'
                : '../../../docsign-web/www/index.html',

        // Maximum size for sessionStorage (5MB typical limit)
        maxSessionStorageSize: 4 * 1024 * 1024, // 4MB to be safe

        // Storage keys
        keys: {
            document: 'agentpdf_document',
            filename: 'agentpdf_filename',
            fields: 'agentpdf_fields',
            source: 'agentpdf_source',
            timestamp: 'agentpdf_timestamp'
        }
    },

    /**
     * Set custom DocSign URL
     * @param {string} url - The DocSign base URL
     */
    setDocsignUrl(url) {
        this.config.docsignBaseUrl = url;
    },

    /**
     * Transfer a document to DocSign
     * @param {Uint8Array} pdfBytes - The PDF file bytes
     * @param {string} filename - The document filename
     * @param {Array} fields - Optional array of placed fields
     * @returns {Promise<boolean>} - True if transfer was initiated
     */
    async transferDocument(pdfBytes, filename, fields = []) {
        if (!pdfBytes || pdfBytes.length === 0) {
            throw new Error('No PDF data to transfer');
        }

        // Convert to base64
        const base64 = this._arrayToBase64(pdfBytes);

        // Check size
        const estimatedSize = base64.length + filename.length + JSON.stringify(fields).length;

        if (estimatedSize > this.config.maxSessionStorageSize) {
            // For large files, use download + manual upload approach
            console.warn('Document too large for direct transfer, prompting download');
            return this._handleLargeDocument(pdfBytes, filename);
        }

        try {
            // Store in sessionStorage
            sessionStorage.setItem(this.config.keys.document, base64);
            sessionStorage.setItem(this.config.keys.filename, filename);
            sessionStorage.setItem(this.config.keys.fields, JSON.stringify(fields));
            sessionStorage.setItem(this.config.keys.source, 'agentpdf');
            sessionStorage.setItem(this.config.keys.timestamp, Date.now().toString());

            // Open DocSign
            const url = this._buildDocsignUrl();
            window.open(url, '_blank');

            return true;
        } catch (err) {
            console.error('Failed to store document in sessionStorage:', err);

            // Fallback to download
            if (err.name === 'QuotaExceededError') {
                return this._handleLargeDocument(pdfBytes, filename);
            }

            throw err;
        }
    },

    /**
     * Check if there's a pending document from agentPDF
     * @returns {Object|null} - The document data or null
     */
    checkForIncomingDocument() {
        const base64 = sessionStorage.getItem(this.config.keys.document);
        if (!base64) {
            return null;
        }

        // Check timestamp (expire after 5 minutes)
        const timestamp = parseInt(sessionStorage.getItem(this.config.keys.timestamp) || '0');
        const age = Date.now() - timestamp;
        if (age > 5 * 60 * 1000) {
            console.warn('Incoming document expired, clearing');
            this.clearIncomingDocument();
            return null;
        }

        try {
            const pdfBytes = this._base64ToArray(base64);
            const filename = sessionStorage.getItem(this.config.keys.filename) || 'document.pdf';
            const fieldsJson = sessionStorage.getItem(this.config.keys.fields);
            const fields = fieldsJson ? JSON.parse(fieldsJson) : [];
            const source = sessionStorage.getItem(this.config.keys.source) || 'unknown';

            return {
                pdfBytes,
                filename,
                fields,
                source,
                timestamp
            };
        } catch (err) {
            console.error('Failed to parse incoming document:', err);
            this.clearIncomingDocument();
            return null;
        }
    },

    /**
     * Clear the incoming document from sessionStorage
     */
    clearIncomingDocument() {
        Object.values(this.config.keys).forEach(key => {
            sessionStorage.removeItem(key);
        });
    },

    /**
     * Create a File object from incoming document data
     * @param {Object} docData - The document data from checkForIncomingDocument()
     * @returns {File} - A File object suitable for loadPDF()
     */
    createFileFromData(docData) {
        const blob = new Blob([docData.pdfBytes], { type: 'application/pdf' });
        return new File([blob], docData.filename, { type: 'application/pdf' });
    },

    // Private methods

    _arrayToBase64(bytes) {
        // Handle large arrays in chunks to avoid call stack issues
        const chunkSize = 8192;
        let result = '';
        for (let i = 0; i < bytes.length; i += chunkSize) {
            const chunk = bytes.slice(i, i + chunkSize);
            result += String.fromCharCode.apply(null, chunk);
        }
        return btoa(result);
    },

    _base64ToArray(base64) {
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes;
    },

    _buildDocsignUrl() {
        const base = this.config.docsignBaseUrl;
        // Add a hash parameter to indicate incoming document
        return base + '#from=agentpdf';
    },

    _handleLargeDocument(pdfBytes, filename) {
        // Create download link
        const blob = new Blob([pdfBytes], { type: 'application/pdf' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        a.click();

        URL.revokeObjectURL(url);

        // Show instructions
        alert(
            'Document is too large for direct transfer.\n\n' +
            '1. The document has been downloaded.\n' +
            '2. Open getsignatures.org\n' +
            '3. Upload the downloaded document there.'
        );

        // Still open DocSign
        window.open(this.config.docsignBaseUrl, '_blank');

        return false;
    }
};

// Export for ES modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = DocsignHandoff;
}
