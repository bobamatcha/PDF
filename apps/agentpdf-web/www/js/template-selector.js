/**
 * Template Selector Module
 *
 * Local-first template rendering using WASM (Typst compiler runs in browser).
 * Falls back to API if WASM unavailable.
 */

const TemplateSelector = {
    // Configuration
    config: {
        // API base URL - fallback only (local-first prefers WASM)
        apiBaseUrl: window.location.hostname === 'localhost'
            ? 'http://localhost:3000'
            : window.location.hostname === 'agentpdf.org'
                ? 'https://api.agentpdf.org'
                : 'http://localhost:3000',

        // Cached templates
        templates: null,

        // Modal element ID
        modalId: 'template-modal',

        // Whether WASM rendering is available
        wasmAvailable: false
    },

    /**
     * Check if WASM template rendering is available
     * @returns {boolean}
     */
    isWasmAvailable() {
        const wasm = window.wasmBindings || window.wasm;
        return wasm && typeof wasm.render_template === 'function';
    },

    /**
     * Set custom API URL (for fallback)
     * @param {string} url - The API base URL
     */
    setApiUrl(url) {
        this.config.apiBaseUrl = url;
    },

    /**
     * Fetch available templates - LOCAL-FIRST from WASM
     * @returns {Promise<Array>} - List of templates
     */
    async fetchTemplates() {
        if (this.config.templates) {
            return this.config.templates;
        }

        // Try WASM first (local-first)
        try {
            const wasm = window.wasmBindings || window.wasm;
            if (wasm && typeof wasm.list_templates === 'function') {
                const templatesJson = wasm.list_templates();
                const templates = JSON.parse(templatesJson);
                // Normalize field names for UI compatibility
                this.config.templates = templates.map(t => ({
                    ...t,
                    id: t.name,
                    required_fields: t.required_inputs || [],
                    optional_fields: t.optional_inputs || []
                }));
                this.config.wasmAvailable = true;
                console.log('TemplateSelector: Loaded', this.config.templates.length, 'templates from WASM (local-first)');
                return this.config.templates;
            }
        } catch (err) {
            console.warn('TemplateSelector: WASM list_templates failed:', err);
        }

        // Fallback to API
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/api/templates`);
            if (!response.ok) {
                throw new Error(`API error: ${response.status}`);
            }
            const data = await response.json();
            this.config.templates = data.templates || [];
            return this.config.templates;
        } catch (err) {
            console.error('Failed to fetch templates:', err);
            // Return hardcoded fallback for offline/demo mode
            return this._getFallbackTemplates();
        }
    },

    /**
     * Render a template with given inputs - LOCAL-FIRST using WASM
     * @param {string} templateName - The template name (e.g., "florida_lease")
     * @param {Object} inputs - The template variables
     * @param {string} format - Output format (pdf, svg, png) - only pdf supported locally
     * @returns {Promise<{success: boolean, data?: string, error?: string}>}
     */
    async renderTemplate(templateName, inputs, format = 'pdf') {
        // Try WASM first (local-first) - $0 server cost!
        try {
            const wasm = window.wasmBindings || window.wasm;
            if (wasm && typeof wasm.render_template === 'function' && format === 'pdf') {
                console.log('TemplateSelector: Rendering locally via WASM...');
                const startTime = performance.now();

                const inputsJson = JSON.stringify(inputs);
                const pdfBase64 = wasm.render_template(templateName, inputsJson);

                const endTime = performance.now();
                console.log(`TemplateSelector: Rendered in ${Math.round(endTime - startTime)}ms (local, $0 cost)`);

                return {
                    success: true,
                    data: pdfBase64,
                    local: true,
                    renderTimeMs: Math.round(endTime - startTime)
                };
            }
        } catch (err) {
            console.warn('TemplateSelector: WASM render failed, falling back to API:', err);
        }

        // Fallback to API
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/api/render`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    template: templateName,
                    is_template: true,
                    inputs: inputs,
                    format: format
                })
            });

            const data = await response.json();
            return { ...data, local: false };
        } catch (err) {
            console.error('Failed to render template:', err);
            return {
                success: false,
                error: err.message
            };
        }
    },

    /**
     * Show the template selector modal
     * @param {Function} onSelect - Callback when a template is selected and rendered
     */
    async showModal(onSelect) {
        const templates = await this.fetchTemplates();

        // Create or get modal
        let modal = document.getElementById(this.config.modalId);
        if (!modal) {
            modal = this._createModal();
            document.body.appendChild(modal);
        }

        // Populate templates
        const templateList = modal.querySelector('.template-list');
        templateList.innerHTML = '';

        templates.forEach(template => {
            const card = this._createTemplateCard(template, onSelect);
            templateList.appendChild(card);
        });

        // Show modal
        modal.classList.remove('hidden');
        modal.classList.add('visible');
    },

    /**
     * Hide the template selector modal
     */
    hideModal() {
        const modal = document.getElementById(this.config.modalId);
        if (modal) {
            modal.classList.remove('visible');
            modal.classList.add('hidden');
        }
    },

    /**
     * Show the template form modal for a specific template
     * @param {Object} template - The template metadata
     * @param {Function} onSubmit - Callback when form is submitted
     */
    showFormModal(template, onSubmit) {
        let modal = document.getElementById('template-form-modal');
        if (!modal) {
            modal = this._createFormModal();
            document.body.appendChild(modal);
        }

        const title = modal.querySelector('.modal-title');
        title.textContent = `Fill: ${template.name}`;

        const form = modal.querySelector('form');
        form.innerHTML = '';

        // Support both API naming (required_inputs) and fallback naming (required_fields)
        const reqFields = template.required_inputs || template.required_fields || [];
        const optFields = template.optional_inputs || template.optional_fields || [];

        // Add required fields
        if (reqFields.length > 0) {
            const reqSection = document.createElement('div');
            reqSection.className = 'form-section';
            reqSection.innerHTML = '<h4>Required Fields</h4>';

            reqFields.forEach(field => {
                reqSection.appendChild(this._createFormField(field, true));
            });
            form.appendChild(reqSection);
        }

        // Add optional fields (collapsed by default)
        if (optFields.length > 0) {
            const optSection = document.createElement('details');
            optSection.className = 'form-section';
            optSection.innerHTML = `<summary>Optional Fields (${optFields.length})</summary>`;

            optFields.forEach(field => {
                optSection.appendChild(this._createFormField(field, false));
            });
            form.appendChild(optSection);
        }

        // Add submit button
        const submitBtn = document.createElement('button');
        submitBtn.type = 'submit';
        submitBtn.className = 'btn btn-primary';
        submitBtn.textContent = 'Generate Document';
        form.appendChild(submitBtn);

        // Handle form submission
        form.onsubmit = async (e) => {
            e.preventDefault();
            const formData = new FormData(form);
            const inputs = {};
            formData.forEach((value, key) => {
                if (value) inputs[key] = value;
            });

            submitBtn.disabled = true;
            submitBtn.textContent = 'Generating...';

            try {
                const result = await this.renderTemplate(template.id || template.name, inputs);
                if (result.success && result.data) {
                    this.hideFormModal();
                    onSubmit(result.data, template.name);
                } else {
                    alert('Failed to generate: ' + (result.error || 'Unknown error'));
                }
            } catch (err) {
                alert('Error: ' + err.message);
            } finally {
                submitBtn.disabled = false;
                submitBtn.textContent = 'Generate Document';
            }
        };

        modal.classList.remove('hidden');
        modal.classList.add('visible');
    },

    /**
     * Hide the template form modal
     */
    hideFormModal() {
        const modal = document.getElementById('template-form-modal');
        if (modal) {
            modal.classList.remove('visible');
            modal.classList.add('hidden');
        }
    },

    // Private methods

    _createModal() {
        const modal = document.createElement('div');
        modal.id = this.config.modalId;
        modal.className = 'modal hidden';
        modal.innerHTML = `
            <div class="modal-backdrop" onclick="TemplateSelector.hideModal()"></div>
            <div class="modal-content">
                <div class="modal-header">
                    <h3 class="modal-title">Select a Template</h3>
                    <button class="modal-close" onclick="TemplateSelector.hideModal()">&times;</button>
                </div>
                <div class="modal-body">
                    <div class="template-list"></div>
                </div>
            </div>
        `;
        return modal;
    },

    _createFormModal() {
        const modal = document.createElement('div');
        modal.id = 'template-form-modal';
        modal.className = 'modal hidden';
        modal.innerHTML = `
            <div class="modal-backdrop" onclick="TemplateSelector.hideFormModal()"></div>
            <div class="modal-content modal-large">
                <div class="modal-header">
                    <h3 class="modal-title">Fill Template</h3>
                    <button class="modal-close" onclick="TemplateSelector.hideFormModal()">&times;</button>
                </div>
                <div class="modal-body">
                    <form class="template-form"></form>
                </div>
            </div>
        `;
        return modal;
    },

    _createTemplateCard(template, onSelect) {
        const card = document.createElement('div');
        card.className = 'template-card';
        // Support both API naming (required_inputs) and fallback naming (required_fields)
        const reqFields = template.required_inputs || template.required_fields || [];
        const optFields = template.optional_inputs || template.optional_fields || [];
        card.innerHTML = `
            <h4>${template.name}</h4>
            <p>${template.description || 'No description'}</p>
            <div class="template-meta">
                <span class="badge">${reqFields.length} required</span>
                <span class="badge secondary">${optFields.length} optional</span>
            </div>
        `;
        card.onclick = () => {
            this.hideModal();
            this.showFormModal(template, onSelect);
        };
        return card;
    },

    _createFormField(fieldName, required) {
        const div = document.createElement('div');
        div.className = 'form-field';

        const label = document.createElement('label');
        const friendlyLabel = this._getFriendlyLabel(fieldName);
        label.textContent = friendlyLabel;
        label.htmlFor = fieldName;
        if (required) label.className = 'required';

        // Check if this is a tristate flood disclosure field (3 neutral options)
        // Per scrivener adherence - must not lead user to any particular answer
        if (this._isTristateFloodField(fieldName)) {
            div.appendChild(label);
            div.appendChild(this._createTristateRadio(fieldName));
            return div;
        }

        // Check if this is a simple boolean field (lead paint, etc)
        if (this._isBooleanField(fieldName)) {
            div.appendChild(label);
            div.appendChild(this._createYesNoRadio(fieldName));
            return div;
        }

        // NOTE: email_consent is NOT shown in template form
        // Per HB 615, tenant must actively consent during SIGNATURE CEREMONY
        // The template generates unchecked boxes that tenant fills during signing

        const input = document.createElement('input');
        input.type = this._getInputType(fieldName);
        input.name = fieldName;
        input.id = fieldName;
        input.required = required;
        input.placeholder = this._getPlaceholder(fieldName);

        div.appendChild(label);
        div.appendChild(input);
        return div;
    },

    /**
     * Check if field is a tristate flood disclosure field
     * Per scrivener adherence, these MUST offer 3 neutral options
     */
    _isTristateFloodField(fieldName) {
        const tristateFields = [
            'flood_history_status',
            'flood_claims_status',
            'flood_fema_status'
        ];
        return tristateFields.includes(fieldName);
    },

    /**
     * Check if field is a simple boolean (lead paint, etc)
     */
    _isBooleanField(fieldName) {
        // Only is_pre_1978 remains as simple boolean
        // Flood fields are now tristate per scrivener adherence
        return fieldName === 'is_pre_1978' || fieldName.startsWith('is_');
    },

    /**
     * Create tristate radio buttons with 3 neutral options
     * Per STRATEGY.md scrivener adherence requirements
     */
    _createTristateRadio(fieldName) {
        const container = document.createElement('div');
        container.className = 'radio-group tristate';

        // Option 1: Yes
        const yesLabel = document.createElement('label');
        yesLabel.className = 'radio-label';
        const yesInput = document.createElement('input');
        yesInput.type = 'radio';
        yesInput.name = fieldName;
        yesInput.value = 'yes';
        yesLabel.appendChild(yesInput);
        yesLabel.appendChild(document.createTextNode(' Yes'));

        // Option 2: No
        const noLabel = document.createElement('label');
        noLabel.className = 'radio-label';
        const noInput = document.createElement('input');
        noInput.type = 'radio';
        noInput.name = fieldName;
        noInput.value = 'no';
        noLabel.appendChild(noInput);
        noLabel.appendChild(document.createTextNode(' No'));

        // Option 3: Unknown - DEFAULT per scrivener neutrality
        const unknownLabel = document.createElement('label');
        unknownLabel.className = 'radio-label';
        const unknownInput = document.createElement('input');
        unknownInput.type = 'radio';
        unknownInput.name = fieldName;
        unknownInput.value = 'unknown';
        unknownInput.checked = true; // Default to unknown - neutral, not leading
        unknownLabel.appendChild(unknownInput);
        unknownLabel.appendChild(document.createTextNode(' I don\'t know'));

        container.appendChild(yesLabel);
        container.appendChild(noLabel);
        container.appendChild(unknownLabel);
        return container;
    },

    /**
     * Create Yes/No radio for simple boolean fields (like lead paint)
     */
    _createYesNoRadio(fieldName) {
        const container = document.createElement('div');
        container.className = 'radio-group';

        const yesLabel = document.createElement('label');
        yesLabel.className = 'radio-label';
        const yesInput = document.createElement('input');
        yesInput.type = 'radio';
        yesInput.name = fieldName;
        yesInput.value = 'true';
        yesLabel.appendChild(yesInput);
        yesLabel.appendChild(document.createTextNode(' Yes'));

        const noLabel = document.createElement('label');
        noLabel.className = 'radio-label';
        const noInput = document.createElement('input');
        noInput.type = 'radio';
        noInput.name = fieldName;
        noInput.value = 'false';
        noInput.checked = true; // Default to No for simple booleans
        noLabel.appendChild(noInput);
        noLabel.appendChild(document.createTextNode(' No'));

        container.appendChild(yesLabel);
        container.appendChild(noLabel);
        return container;
    },

    _getFriendlyLabel(fieldName) {
        const friendlyLabels = {
            // Flood Disclosure (§ 83.512 / SB 948) - NEUTRAL PHRASING
            // Per scrivener adherence: questions must not lead the user
            'flood_history_status': '§ 83.512 Flood Disclosure: Property flooding history',
            'flood_claims_status': '§ 83.512 Flood Disclosure: Flood insurance claims',
            'flood_fema_status': '§ 83.512 Flood Disclosure: Federal flood assistance (FEMA)',
            'flood_status_unknown': 'Include "unknown" option for flood disclosure',
            'flooding_description': 'Describe flooding details (if applicable)',
            // Lead Paint
            'is_pre_1978': 'Was this property built before 1978? (Lead paint disclosure required)',
            'year_built': 'Year Built',
            // Standard fields
            'landlord_name': 'Landlord Name',
            'tenant_name': 'Tenant Name',
            'property_address': 'Property Address',
            'monthly_rent': 'Monthly Rent ($)',
            'lease_start': 'Lease Start Date',
            'lease_end': 'Lease End Date',
            'security_deposit': 'Security Deposit ($)',
            'deposit_details': 'Security Deposit Details',
            'landlord_email': 'Landlord Email',
            'landlord_address': 'Landlord Address',
            'tenant_email': 'Tenant Email'
        };
        return friendlyLabels[fieldName] || this._formatFieldName(fieldName);
    },

    _formatFieldName(name) {
        return name
            .replace(/_/g, ' ')
            .replace(/\b\w/g, c => c.toUpperCase());
    },

    _getInputType(fieldName) {
        if (fieldName.includes('email')) return 'email';
        if (fieldName.includes('phone')) return 'tel';
        // Use word boundaries to avoid matching 'sender' -> 'end'
        if (fieldName.includes('date') || fieldName.match(/\b(start|end)\b/) || fieldName.endsWith('_start') || fieldName.endsWith('_end')) return 'date';
        if (fieldName.includes('rent') || fieldName.includes('fee') || fieldName.includes('deposit') || fieldName.includes('amount')) return 'number';
        return 'text';
    },

    _getPlaceholder(fieldName) {
        const placeholders = {
            'landlord_name': 'John Smith',
            'tenant_name': 'Jane Doe',
            'property_address': '123 Main St, Miami, FL 33101',
            'monthly_rent': '2000',
            'security_deposit': '2000',
            'lease_start': '',
            'lease_end': ''
        };
        return placeholders[fieldName] || '';
    },

    _getFallbackTemplates() {
        // Hardcoded templates for offline/demo mode
        // NOTE: email_consent removed - belongs in signature ceremony, not template form
        // Flood fields use neutral tristate names per scrivener adherence
        return [
            {
                name: 'Florida Lease',
                id: 'florida_lease',
                state: 'FL',
                description: 'Florida residential lease (F.S. Chapter 83) with § 83.512 Flood Disclosure',
                required_fields: ['landlord_name', 'tenant_name', 'property_address', 'monthly_rent', 'lease_start', 'lease_end'],
                optional_fields: [
                    'landlord_address', 'landlord_email', 'tenant_email',
                    'year_built', 'is_pre_1978', 'deposit_details',
                    // § 83.512 Flood Disclosure - tristate fields (yes/no/unknown)
                    'flood_history_status', 'flood_claims_status', 'flood_fema_status',
                    'flood_status_unknown', 'flooding_description'
                ]
            },
            {
                name: 'Texas Lease',
                id: 'texas_lease',
                state: 'TX',
                description: 'Texas residential lease agreement (Tex. Prop. Code Ch. 92 compliant)',
                required_fields: ['landlord_name', 'tenant_name', 'property_address', 'monthly_rent', 'lease_start', 'lease_end'],
                optional_fields: ['landlord_phone', 'landlord_email', 'security_deposit', 'late_fee', 'application_fee', 'year_built']
            },
            {
                name: 'Invoice',
                id: 'invoice',
                description: 'Professional invoice template',
                required_fields: ['company_name', 'client_name', 'invoice_number', 'amount'],
                optional_fields: ['company_address', 'client_address', 'due_date', 'notes']
            },
            {
                name: 'Letter',
                id: 'letter',
                description: 'Formal business letter template',
                required_fields: ['sender_name', 'recipient_name', 'subject', 'body'],
                optional_fields: ['sender_address', 'recipient_address', 'date']
            }
        ];
    }
};

/**
 * State Selector for Compliance Checking
 * Allows users to select which state's laws to use for compliance checking
 *
 * State data is loaded from WASM (compliance-engine) to avoid duplication.
 * Falls back to cached data if WASM is unavailable.
 */
const StateSelector = {
    // States loaded from WASM - no more hardcoded duplication!
    states: [],

    // Whether states have been loaded from WASM
    _loaded: false,

    // Currently selected state
    currentState: 'FL',

    /**
     * Load states from WASM module
     * @returns {Promise<Array>} List of states from compliance engine
     */
    async loadStates() {
        if (this._loaded && this.states.length > 0) {
            return this.states;
        }

        try {
            // Try to get states from WASM
            // Check both window.wasmBindings (trunk) and window.wasm (wasm-pack direct)
            const wasm = window.wasmBindings || window.wasm;
            if (wasm && typeof wasm.get_supported_states === 'function') {
                const statesJson = wasm.get_supported_states();
                this.states = JSON.parse(statesJson);
                this._loaded = true;
                console.log('StateSelector: Loaded', this.states.length, 'states from WASM');
                return this.states;
            }
        } catch (err) {
            console.warn('StateSelector: Failed to load states from WASM:', err);
        }

        // Fallback: use cached/default states if WASM unavailable
        if (this.states.length === 0) {
            this.states = this._getFallbackStates();
        }
        return this.states;
    },

    /**
     * Fallback states when WASM is unavailable (e.g., during initial load)
     * This should match the Rust State::implemented_states() + statute_citation()
     */
    _getFallbackStates() {
        return [
            { code: 'FL', name: 'Florida', implemented: true, statutes: 'F.S. Chapter 83' },
            { code: 'TX', name: 'Texas', implemented: true, statutes: 'Tex. Prop. Code Ch. 92' },
            { code: 'CA', name: 'California', implemented: true, statutes: 'CA Civil Code 1940-1954' },
            { code: 'NY', name: 'New York', implemented: true, statutes: 'NY RPL Article 7' },
            { code: 'GA', name: 'Georgia', implemented: true, statutes: 'GA Code Title 44 Ch. 7' },
            { code: 'IL', name: 'Illinois', implemented: true, statutes: '765 ILCS + Chicago RLTO' },
            { code: 'PA', name: 'Pennsylvania', implemented: true, statutes: '68 P.S. § 250.501 et seq.' },
            { code: 'NJ', name: 'New Jersey', implemented: true, statutes: 'N.J.S.A. 46:8 et seq.' },
            { code: 'VA', name: 'Virginia', implemented: true, statutes: 'VA Code § 55.1-1200 et seq.' },
            { code: 'MA', name: 'Massachusetts', implemented: true, statutes: 'M.G.L. c. 186' },
            { code: 'OH', name: 'Ohio', implemented: true, statutes: 'O.R.C. Chapter 5321' },
            { code: 'MI', name: 'Michigan', implemented: true, statutes: 'M.C.L. 554.601 et seq.' },
            { code: 'WA', name: 'Washington', implemented: true, statutes: 'RCW 59.18' },
            { code: 'AZ', name: 'Arizona', implemented: true, statutes: 'A.R.S. Title 33 Ch. 10' },
            { code: 'NC', name: 'North Carolina', implemented: true, statutes: 'N.C.G.S. Chapter 42' },
            { code: 'TN', name: 'Tennessee', implemented: true, statutes: 'T.C.A. Title 66 Ch. 28' }
        ];
    },

    /**
     * Get the currently selected state
     * @returns {string} State code
     */
    getState() {
        return this.currentState;
    },

    /**
     * Set the current state
     * @param {string} stateCode - Two-letter state code
     */
    setState(stateCode) {
        const state = this.states.find(s => s.code === stateCode);
        if (state) {
            this.currentState = stateCode;
            this._updateUI();
            this._dispatchChange();
        }
    },

    /**
     * Create and inject the state selector UI
     * @param {string} containerId - ID of the container element
     */
    async init(containerId) {
        const container = document.getElementById(containerId);
        if (!container) {
            console.warn('StateSelector: Container not found:', containerId);
            return;
        }

        // Load states from WASM before creating UI
        await this.loadStates();

        container.innerHTML = this._createSelectorHTML();
        this._attachEventListeners(container);
    },

    /**
     * Get list of implemented states
     * @returns {Array} List of implemented state objects
     */
    getImplementedStates() {
        return this.states.filter(s => s.implemented);
    },

    _createSelectorHTML() {
        const implementedStates = this.states.filter(s => s.implemented);
        const comingSoon = this.states.filter(s => !s.implemented);

        let optgroupsHTML = `
            <optgroup label="Available (${implementedStates.length} states)">
                ${implementedStates.map(s => `
                    <option value="${s.code}" ${s.code === this.currentState ? 'selected' : ''}>
                        ${s.name} (${s.statutes})
                    </option>
                `).join('')}
            </optgroup>
        `;

        // Only show "Coming Soon" if there are states pending
        if (comingSoon.length > 0) {
            optgroupsHTML += `
                <optgroup label="Coming Soon" disabled>
                    ${comingSoon.map(s => `
                        <option value="${s.code}" disabled>
                            ${s.name} (${s.statutes})
                        </option>
                    `).join('')}
                </optgroup>
            `;
        }

        return `
            <div class="state-selector">
                <label for="state-select">Check compliance for:</label>
                <select id="state-select" class="state-select">
                    ${optgroupsHTML}
                </select>
                <span class="state-badge" id="state-badge">${this.currentState}</span>
            </div>
        `;
    },

    _attachEventListeners(container) {
        const select = container.querySelector('#state-select');
        if (select) {
            select.addEventListener('change', (e) => {
                this.setState(e.target.value);
            });
        }
    },

    _updateUI() {
        const badge = document.getElementById('state-badge');
        if (badge) {
            badge.textContent = this.currentState;
        }

        const select = document.getElementById('state-select');
        if (select) {
            select.value = this.currentState;
        }
    },

    _dispatchChange() {
        const event = new CustomEvent('statechange', {
            detail: {
                state: this.currentState,
                stateInfo: this.states.find(s => s.code === this.currentState)
            }
        });
        document.dispatchEvent(event);
    }
};

// Add CSS styles for state selector
(function() {
    const style = document.createElement('style');
    style.textContent += `
        .state-selector {
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 8px 12px;
            background: #f8f9fa;
            border-radius: 6px;
            margin-bottom: 16px;
        }
        .state-selector label {
            font-weight: 500;
            color: #495057;
        }
        .state-select {
            padding: 6px 12px;
            border: 1px solid #ced4da;
            border-radius: 4px;
            background: white;
            font-size: 0.9rem;
            cursor: pointer;
        }
        .state-select:focus {
            outline: none;
            border-color: #007bff;
            box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
        }
        .state-badge {
            background: #007bff;
            color: white;
            padding: 4px 10px;
            border-radius: 12px;
            font-size: 0.8rem;
            font-weight: 600;
        }
        /* Dark mode */
        .dark-mode .state-selector {
            background: #2d2d2d;
        }
        .dark-mode .state-selector label {
            color: #adb5bd;
        }
        .dark-mode .state-select {
            background: #1e1e1e;
            border-color: #444;
            color: #fff;
        }
    `;
    document.head.appendChild(style);
})();

// Add CSS styles for the modal
(function() {
    const style = document.createElement('style');
    style.textContent = `
        .modal {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            z-index: 1000;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .modal.hidden {
            display: none;
        }
        .modal-backdrop {
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.5);
        }
        .modal-content {
            position: relative;
            background: white;
            border-radius: 8px;
            max-width: 600px;
            width: 90%;
            max-height: 80vh;
            overflow-y: auto;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
        }
        .modal-large {
            max-width: 800px;
        }
        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 16px 20px;
            border-bottom: 1px solid #eee;
        }
        .modal-title {
            margin: 0;
            font-size: 1.25rem;
        }
        .modal-close {
            background: none;
            border: none;
            font-size: 1.5rem;
            cursor: pointer;
            color: #666;
        }
        .modal-body {
            padding: 20px;
        }
        .template-list {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
            gap: 16px;
        }
        .template-card {
            border: 1px solid #ddd;
            border-radius: 8px;
            padding: 16px;
            cursor: pointer;
            transition: all 0.2s;
        }
        .template-card:hover {
            border-color: #007bff;
            box-shadow: 0 2px 8px rgba(0, 123, 255, 0.2);
        }
        .template-card h4 {
            margin: 0 0 8px 0;
        }
        .template-card p {
            margin: 0 0 12px 0;
            color: #666;
            font-size: 0.9rem;
        }
        .template-meta {
            display: flex;
            gap: 8px;
        }
        .badge {
            background: #007bff;
            color: white;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 0.75rem;
        }
        .badge.secondary {
            background: #6c757d;
        }
        .template-form {
            display: flex;
            flex-direction: column;
            gap: 16px;
        }
        .form-section {
            border: 1px solid #eee;
            border-radius: 8px;
            padding: 16px;
        }
        .form-section h4 {
            margin: 0 0 16px 0;
        }
        .form-section summary {
            cursor: pointer;
            font-weight: 500;
        }
        .form-field {
            display: flex;
            flex-direction: column;
            gap: 4px;
            margin-bottom: 12px;
        }
        .form-field label {
            font-weight: 500;
            font-size: 0.9rem;
        }
        .form-field label.required::after {
            content: ' *';
            color: #dc3545;
        }
        .form-field input {
            padding: 8px 12px;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 1rem;
        }
        .form-field input:focus {
            outline: none;
            border-color: #007bff;
        }
        .radio-group {
            display: flex;
            gap: 12px;
            padding: 8px 0;
            flex-wrap: wrap;
        }
        .radio-group.tristate {
            /* Tristate groups have 3 options - allow wrapping on mobile */
            gap: 10px;
        }
        .radio-label {
            display: flex;
            align-items: center;
            gap: 6px;
            cursor: pointer;
            padding: 8px 14px;
            border: 1px solid #ddd;
            border-radius: 4px;
            transition: all 0.2s;
            font-size: 0.9rem;
        }
        .radio-label:hover {
            border-color: #007bff;
            background: #f8f9fa;
        }
        .radio-label input[type="radio"] {
            margin: 0;
        }
        /* Style for checked radio labels */
        .radio-label:has(input:checked) {
            border-color: #007bff;
            background: #e7f1ff;
        }
        .checkbox-field {
            display: flex;
            align-items: flex-start;
            gap: 12px;
            padding: 12px;
            background: #e8f4fd;
            border: 1px solid #b6d4fe;
            border-radius: 6px;
        }
        .checkbox-field input[type="checkbox"] {
            margin-top: 3px;
            width: 18px;
            height: 18px;
        }
        .checkbox-label {
            flex: 1;
            font-size: 0.9rem;
            line-height: 1.4;
        }
        /* Dark mode support */
        .dark-mode .modal-content {
            background: #1e1e1e;
            color: #fff;
        }
        .dark-mode .modal-header {
            border-color: #333;
        }
        .dark-mode .template-card {
            border-color: #333;
            background: #2d2d2d;
        }
        .dark-mode .template-card:hover {
            border-color: #007bff;
        }
        .dark-mode .template-card p {
            color: #aaa;
        }
        .dark-mode .form-section {
            border-color: #333;
        }
        .dark-mode .form-field input {
            background: #2d2d2d;
            border-color: #444;
            color: #fff;
        }
        .dark-mode .radio-label {
            border-color: #444;
            background: #2d2d2d;
        }
        .dark-mode .radio-label:hover {
            border-color: #007bff;
            background: #333;
        }
        .dark-mode .checkbox-field {
            background: #1a3a5c;
            border-color: #2563eb;
        }
    `;
    document.head.appendChild(style);
})();

// Export for ES modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = TemplateSelector;
}
