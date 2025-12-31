//! DocSign - Local-First Document Signing
//!
//! A zero-knowledge, client-side PDF signing solution.

use js_sys::{Function, Uint8Array};
use wasm_bindgen::prelude::*;

pub mod coords;
pub mod crypto;
pub mod pdf;
pub mod session;
pub mod storage;

use crypto::cert::CertificateIdentity;
use crypto::keys::{EphemeralIdentity, SigningIdentity};
use pdf::audit::{hash_document, AuditAction, AuditChain};
use pdf::parser::PdfDocument;
use pdf::signer::{PdfSigner, SignatureField};

/// Identity type - either ephemeral or certificate-based
enum IdentityType {
    Ephemeral(EphemeralIdentity),
    Certificate(CertificateIdentity),
}

impl SigningIdentity for IdentityType {
    fn public_key_der(&self) -> Vec<u8> {
        match self {
            IdentityType::Ephemeral(id) => id.public_key_der(),
            IdentityType::Certificate(id) => id.public_key_der(),
        }
    }

    fn sign(&self, data: &[u8]) -> Vec<u8> {
        match self {
            IdentityType::Ephemeral(id) => id.sign(data),
            IdentityType::Certificate(id) => id.sign(data),
        }
    }

    fn sign_prehashed(&self, hash: &[u8; 32]) -> Vec<u8> {
        match self {
            IdentityType::Ephemeral(id) => id.sign_prehashed(hash),
            IdentityType::Certificate(id) => id.sign_prehashed(hash),
        }
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        match self {
            IdentityType::Ephemeral(id) => id.verify(data, signature),
            IdentityType::Certificate(id) => id.verify(data, signature),
        }
    }

    fn certificate_der(&self) -> Option<&[u8]> {
        match self {
            IdentityType::Ephemeral(_) => None,
            IdentityType::Certificate(id) => Some(id.certificate_der()),
        }
    }

    fn signer_name(&self) -> Option<&str> {
        match self {
            IdentityType::Ephemeral(_) => None,
            IdentityType::Certificate(id) => id.signer_name(),
        }
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"DocSign WASM initialized".into());
}

/// Main DocSign application state
#[wasm_bindgen]
pub struct DocSign {
    document: Option<PdfDocument>,
    identity: Option<IdentityType>,
    audit_chain: AuditChain,
    document_id: String,
    signer_email: String,
    has_certificate: bool,
}

impl Default for DocSign {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl DocSign {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let document_id = uuid::Uuid::new_v4().to_string();
        Self {
            document: None,
            identity: None,
            audit_chain: AuditChain::new(&document_id),
            document_id,
            signer_email: String::new(),
            has_certificate: false,
        }
    }

    /// Set the signer's email address
    #[wasm_bindgen]
    pub fn set_signer_email(&mut self, email: &str) {
        self.signer_email = email.to_string();
    }

    /// Import a CA-issued certificate for signing
    /// Returns certificate info (subject, issuer, expiration) on success
    #[wasm_bindgen]
    pub fn import_certificate(
        &mut self,
        cert_pem: &str,
        key_pem: &str,
    ) -> Result<JsValue, JsValue> {
        let cert_identity =
            CertificateIdentity::from_pem(cert_pem, key_pem).map_err(|e| JsValue::from_str(&e))?;

        // Build certificate info for JS
        let info = js_sys::Object::new();
        js_sys::Reflect::set(
            &info,
            &"subject".into(),
            &cert_identity.subject_name().into(),
        )?;
        js_sys::Reflect::set(&info, &"issuer".into(), &cert_identity.issuer_name().into())?;
        js_sys::Reflect::set(
            &info,
            &"serialNumber".into(),
            &cert_identity.serial_number_hex().into(),
        )?;
        js_sys::Reflect::set(
            &info,
            &"notBefore".into(),
            &cert_identity.not_before().into(),
        )?;
        js_sys::Reflect::set(&info, &"notAfter".into(), &cert_identity.not_after().into())?;
        js_sys::Reflect::set(&info, &"isValid".into(), &cert_identity.is_valid().into())?;

        self.identity = Some(IdentityType::Certificate(cert_identity));
        self.has_certificate = true;

        Ok(info.into())
    }

    /// Check if a certificate is currently loaded
    #[wasm_bindgen]
    pub fn has_certificate(&self) -> bool {
        self.has_certificate
    }

    /// Get certificate info if one is loaded
    #[wasm_bindgen]
    pub fn get_certificate_info(&self) -> Result<JsValue, JsValue> {
        match &self.identity {
            Some(IdentityType::Certificate(cert)) => {
                let info = js_sys::Object::new();
                js_sys::Reflect::set(&info, &"subject".into(), &cert.subject_name().into())?;
                js_sys::Reflect::set(&info, &"issuer".into(), &cert.issuer_name().into())?;
                js_sys::Reflect::set(
                    &info,
                    &"serialNumber".into(),
                    &cert.serial_number_hex().into(),
                )?;
                js_sys::Reflect::set(&info, &"notBefore".into(), &cert.not_before().into())?;
                js_sys::Reflect::set(&info, &"notAfter".into(), &cert.not_after().into())?;
                js_sys::Reflect::set(&info, &"isValid".into(), &cert.is_valid().into())?;
                Ok(info.into())
            }
            _ => Err(JsValue::from_str("No certificate loaded")),
        }
    }

    /// Clear loaded certificate and revert to ephemeral keys
    #[wasm_bindgen]
    pub fn clear_certificate(&mut self) {
        self.identity = None;
        self.has_certificate = false;
    }

    /// Get the document ID
    #[wasm_bindgen]
    pub fn document_id(&self) -> String {
        self.document_id.clone()
    }

    /// Load a PDF from bytes
    #[wasm_bindgen]
    pub fn load_pdf(&mut self, bytes: Vec<u8>) -> Result<JsValue, JsValue> {
        let doc = PdfDocument::from_bytes(bytes).map_err(|e| JsValue::from_str(&e))?;

        // Generate ephemeral identity only if no certificate is loaded
        if !self.has_certificate {
            self.identity = Some(IdentityType::Ephemeral(EphemeralIdentity::generate()));
        }

        // Calculate document hash and log upload event
        let doc_hash = hash_document(doc.bytes());
        self.audit_chain.append(
            AuditAction::Upload,
            &self.signer_email,
            &doc_hash,
            Some(format!("Document loaded, {} pages", doc.page_count())),
        );

        let page_count = doc.page_count();
        self.document = Some(doc);

        // Return document info as JS object
        let info = js_sys::Object::new();
        js_sys::Reflect::set(&info, &"pageCount".into(), &(page_count as u32).into())?;
        js_sys::Reflect::set(
            &info,
            &"documentId".into(),
            &self.document_id.clone().into(),
        )?;
        Ok(info.into())
    }

    /// Get page count
    #[wasm_bindgen]
    pub fn page_count(&self) -> u32 {
        self.document
            .as_ref()
            .map(|d| d.page_count() as u32)
            .unwrap_or(0)
    }

    /// Get page dimensions for a given page number (1-indexed)
    #[wasm_bindgen]
    pub fn page_dimensions(&self, page_num: u32) -> Result<JsValue, JsValue> {
        let doc = self
            .document
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No document loaded"))?;

        let dims = doc
            .page_dimensions(page_num)
            .map_err(|e| JsValue::from_str(&e))?;

        let result = js_sys::Object::new();
        js_sys::Reflect::set(&result, &"x".into(), &dims[0].into())?;
        js_sys::Reflect::set(&result, &"y".into(), &dims[1].into())?;
        js_sys::Reflect::set(&result, &"width".into(), &dims[2].into())?;
        js_sys::Reflect::set(&result, &"height".into(), &dims[3].into())?;
        Ok(result.into())
    }

    /// Add a signature field (returns field index)
    #[wasm_bindgen]
    pub fn add_signature_field(
        &mut self,
        page: u32,
        _x: f64,
        _y: f64,
        _width: f64,
        _height: f64,
        _reason: &str,
    ) -> Result<(), JsValue> {
        // Log the field addition
        let doc_hash = self
            .document
            .as_ref()
            .map(|d| hash_document(d.bytes()))
            .unwrap_or_default();

        self.audit_chain.append(
            AuditAction::FieldAdded,
            &self.signer_email,
            &doc_hash,
            Some(format!("Signature field on page {}", page)),
        );

        Ok(())
    }

    /// Sign the document with a signature field
    #[wasm_bindgen]
    pub fn sign_document(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        reason: &str,
    ) -> Result<Uint8Array, JsValue> {
        let doc = self
            .document
            .as_mut()
            .ok_or_else(|| JsValue::from_str("No document loaded"))?;

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No identity generated"))?;

        // Create signature field
        let field = SignatureField::new(page, self.signer_email.clone(), reason.to_string())
            .with_rect([x, y, x + width, y + height]);

        // Sign the document
        let mut signer = PdfSigner::new(doc, identity);
        let signed_bytes = signer.sign(&field).map_err(|e| JsValue::from_str(&e))?;

        // Log the signing event
        let doc_hash = hash_document(&signed_bytes);
        self.audit_chain.append(
            AuditAction::Sign,
            &self.signer_email,
            &doc_hash,
            Some(format!("Document signed by {}", self.signer_email)),
        );

        // Update the stored document
        self.document =
            Some(PdfDocument::from_bytes(signed_bytes.clone()).map_err(|e| JsValue::from_str(&e))?);

        Ok(Uint8Array::from(signed_bytes.as_slice()))
    }

    /// Sign document with progress callback for large PDFs
    /// Callback receives (stage: string, percent: number)
    #[wasm_bindgen]
    #[allow(clippy::too_many_arguments)]
    pub fn sign_document_with_progress(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        reason: &str,
        progress_callback: &Function,
    ) -> Result<Uint8Array, JsValue> {
        let report_progress = |stage: &str, percent: u32| {
            let _ = progress_callback.call2(
                &JsValue::NULL,
                &JsValue::from_str(stage),
                &JsValue::from(percent),
            );
        };

        report_progress("Preparing signature", 5);

        let doc = self
            .document
            .as_mut()
            .ok_or_else(|| JsValue::from_str("No document loaded"))?;

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No identity generated"))?;

        report_progress("Creating signature field", 10);

        let field = SignatureField::new(page, self.signer_email.clone(), reason.to_string())
            .with_rect([x, y, x + width, y + height]);

        report_progress(
            "Signing document (this may take a moment for large PDFs)",
            20,
        );

        let mut signer = PdfSigner::new(doc, identity);
        let signed_bytes = signer.sign(&field).map_err(|e| JsValue::from_str(&e))?;

        report_progress("Updating audit log", 80);

        let doc_hash = hash_document(&signed_bytes);
        self.audit_chain.append(
            AuditAction::Sign,
            &self.signer_email,
            &doc_hash,
            Some(format!("Document signed by {}", self.signer_email)),
        );

        report_progress("Finalizing", 90);

        self.document =
            Some(PdfDocument::from_bytes(signed_bytes.clone()).map_err(|e| JsValue::from_str(&e))?);

        report_progress("Complete", 100);

        Ok(Uint8Array::from(signed_bytes.as_slice()))
    }

    /// Get the audit log as JSON
    #[wasm_bindgen]
    pub fn get_audit_log(&self) -> Result<String, JsValue> {
        self.audit_chain
            .to_json()
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Get audit log summary (for display)
    #[wasm_bindgen]
    pub fn get_audit_summary(&self) -> Vec<JsValue> {
        self.audit_chain
            .summary()
            .into_iter()
            .map(|s| JsValue::from_str(&s))
            .collect()
    }

    /// Verify the audit chain integrity
    #[wasm_bindgen]
    pub fn verify_audit_chain(&self) -> Result<bool, JsValue> {
        self.audit_chain
            .verify()
            .map(|_| true)
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Get the current document bytes
    #[wasm_bindgen]
    pub fn get_document_bytes(&self) -> Result<Uint8Array, JsValue> {
        let doc = self
            .document
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No document loaded"))?;

        Ok(Uint8Array::from(doc.bytes()))
    }

    /// Get public key (for verification purposes)
    #[wasm_bindgen]
    pub fn get_public_key(&self) -> Result<String, JsValue> {
        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No identity generated"))?;

        Ok(identity.public_key_hex())
    }

    /// Log a view event
    #[wasm_bindgen]
    pub fn log_view(&mut self) {
        let doc_hash = self
            .document
            .as_ref()
            .map(|d| hash_document(d.bytes()))
            .unwrap_or_default();

        self.audit_chain
            .append(AuditAction::View, &self.signer_email, &doc_hash, None);
    }

    /// Add a text field (date, text, or initials) to the document
    /// This creates a stamp annotation with the specified text
    #[wasm_bindgen]
    #[allow(clippy::too_many_arguments)]
    pub fn add_text_field(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        text: &str,
        field_type: &str,
    ) -> Result<(), JsValue> {
        let doc = self
            .document
            .as_mut()
            .ok_or_else(|| JsValue::from_str("No document loaded"))?;

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No identity generated"))?;

        // Choose background color based on field type
        let bg_color = match field_type {
            "date" => Some([1.0, 0.95, 0.9]),     // Light orange
            "initials" => Some([0.9, 0.95, 1.0]), // Light blue
            _ => Some([1.0, 1.0, 0.9]),           // Light yellow for text
        };

        let mut signer = PdfSigner::new(doc, identity);
        signer
            .add_text_stamp(page, x, y, width, height, text, bg_color)
            .map_err(|e| JsValue::from_str(&e))?;

        // Update stored document bytes
        let updated_bytes = doc.save_to_bytes().map_err(|e| JsValue::from_str(&e))?;
        self.document =
            Some(PdfDocument::from_bytes(updated_bytes).map_err(|e| JsValue::from_str(&e))?);

        // Log the field addition
        let doc_hash = self
            .document
            .as_ref()
            .map(|d| hash_document(d.bytes()))
            .unwrap_or_default();

        self.audit_chain.append(
            AuditAction::FieldAdded,
            &self.signer_email,
            &doc_hash,
            Some(format!("{} field on page {}: {}", field_type, page, text)),
        );

        Ok(())
    }

    /// Add a checkbox field to the document
    #[wasm_bindgen]
    pub fn add_checkbox_field(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        size: f64,
        checked: bool,
    ) -> Result<(), JsValue> {
        let doc = self
            .document
            .as_mut()
            .ok_or_else(|| JsValue::from_str("No document loaded"))?;

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| JsValue::from_str("No identity generated"))?;

        let mut signer = PdfSigner::new(doc, identity);
        signer
            .add_checkbox(page, x, y, size, checked)
            .map_err(|e| JsValue::from_str(&e))?;

        // Update stored document bytes
        let updated_bytes = doc.save_to_bytes().map_err(|e| JsValue::from_str(&e))?;
        self.document =
            Some(PdfDocument::from_bytes(updated_bytes).map_err(|e| JsValue::from_str(&e))?);

        // Log the field addition
        let doc_hash = self
            .document
            .as_ref()
            .map(|d| hash_document(d.bytes()))
            .unwrap_or_default();

        self.audit_chain.append(
            AuditAction::FieldAdded,
            &self.signer_email,
            &doc_hash,
            Some(format!(
                "Checkbox on page {}: {}",
                page,
                if checked { "checked" } else { "unchecked" }
            )),
        );

        Ok(())
    }
}

/// Convert DOM coordinates to PDF coordinates
#[wasm_bindgen]
pub fn dom_to_pdf_coords(
    dom_x: f64,
    dom_y: f64,
    container_width: f64,
    container_height: f64,
    pdf_width: f64,
    pdf_height: f64,
) -> Result<JsValue, JsValue> {
    let media_box = [0.0, 0.0, pdf_width, pdf_height];
    let (pdf_x, pdf_y) =
        coords::dom_to_pdf(dom_x, dom_y, container_width, container_height, media_box);

    let result = js_sys::Object::new();
    js_sys::Reflect::set(&result, &"x".into(), &pdf_x.into())?;
    js_sys::Reflect::set(&result, &"y".into(), &pdf_y.into())?;
    Ok(result.into())
}

/// Convert PDF coordinates to DOM coordinates
#[wasm_bindgen]
pub fn pdf_to_dom_coords(
    pdf_x: f64,
    pdf_y: f64,
    container_width: f64,
    container_height: f64,
    pdf_width: f64,
    pdf_height: f64,
) -> Result<JsValue, JsValue> {
    let media_box = [0.0, 0.0, pdf_width, pdf_height];
    let (dom_x, dom_y) =
        coords::pdf_to_dom(pdf_x, pdf_y, container_width, container_height, media_box);

    let result = js_sys::Object::new();
    js_sys::Reflect::set(&result, &"x".into(), &dom_x.into())?;
    js_sys::Reflect::set(&result, &"y".into(), &dom_y.into())?;
    Ok(result.into())
}

// Re-export storage functions
pub use storage::indexeddb::{init_storage, uint8_array_to_vec, vec_to_uint8_array, Storage};

// Re-export session management
pub use session::{
    get_offline_queue_length, is_navigator_online, sync_offline_queue, validate_session_params,
    SessionValidation, SigningSession,
};

// TSA (Timestamp Authority) exports for LTV support
use crypto::tsa;

/// Build an RFC 3161 timestamp request for the given signature bytes
/// Returns DER-encoded TimeStampReq to send to a TSA server
#[wasm_bindgen]
pub fn build_tsa_request(signature_bytes: &[u8]) -> Vec<u8> {
    tsa::build_timestamp_request(signature_bytes)
}

/// Parse a TSA response and extract the TimeStampToken
/// Returns the token bytes or throws an error
#[wasm_bindgen]
pub fn parse_tsa_response(response_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    tsa::parse_timestamp_response(response_bytes).map_err(|e| JsValue::from_str(&e))
}

/// Validate basic structure of a timestamp token
#[wasm_bindgen]
pub fn validate_timestamp_token(token_bytes: &[u8]) -> Result<(), JsValue> {
    tsa::validate_timestamp_token(token_bytes).map_err(|e| JsValue::from_str(&e))
}
