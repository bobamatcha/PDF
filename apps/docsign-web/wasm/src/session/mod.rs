//! Signing session management
//!
//! Handles session data, signature collection, offline queueing, and submission.
//!
//! ## Role Separation
//! - SENDER flow (index.html): Upload, add recipients, place fields, send
//! - SIGNER flow (sign.html): Only accessible via valid session link
//!
//! This module validates session parameters and prevents mock data fallback.

use js_sys::{Array, Object, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Session validation result
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct SessionValidation {
    valid: bool,
    error_message: Option<String>,
}

#[wasm_bindgen]
impl SessionValidation {
    /// Check if session params are valid
    #[wasm_bindgen(getter)]
    pub fn valid(&self) -> bool {
        self.valid
    }

    /// Get error message if invalid
    #[wasm_bindgen(getter)]
    pub fn error_message(&self) -> Option<String> {
        self.error_message.clone()
    }
}

/// Validate session URL parameters (called from JS)
/// Returns validation result - NEVER falls back to mock data
#[wasm_bindgen]
pub fn validate_session_params(
    session_id: Option<String>,
    recipient_id: Option<String>,
    signing_key: Option<String>,
) -> SessionValidation {
    // Check all required params exist
    let session_id = match session_id {
        Some(s) if !s.is_empty() => s,
        _ => {
            return SessionValidation {
                valid: false,
                error_message: Some("Missing required parameter: session".to_string()),
            }
        }
    };

    let recipient_id = match recipient_id {
        Some(r) if !r.is_empty() => r,
        _ => {
            return SessionValidation {
                valid: false,
                error_message: Some("Missing required parameter: recipient".to_string()),
            }
        }
    };

    let signing_key = match signing_key {
        Some(k) if !k.is_empty() => k,
        _ => {
            return SessionValidation {
                valid: false,
                error_message: Some("Missing required parameter: key".to_string()),
            }
        }
    };

    // Basic format validation
    if session_id.len() < 3 {
        return SessionValidation {
            valid: false,
            error_message: Some("Invalid session ID format".to_string()),
        };
    }

    if recipient_id.is_empty() {
        return SessionValidation {
            valid: false,
            error_message: Some("Invalid recipient ID format".to_string()),
        };
    }

    if signing_key.len() < 3 {
        return SessionValidation {
            valid: false,
            error_message: Some("Invalid signing key format".to_string()),
        };
    }

    SessionValidation {
        valid: true,
        error_message: None,
    }
}

/// A signing session representing a document sent for signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningSessionData {
    pub session_id: String,
    pub recipient_id: String,
    pub signing_key: String,
    pub document_name: String,
    pub fields: Vec<SigningField>,
    pub completed_fields: Vec<String>,
    pub created_at: f64,
}

/// A field that needs to be signed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningField {
    pub id: String,
    pub field_type: String,
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub required: bool,
    pub recipient_id: String,
}

/// Result of a signature submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResult {
    pub success: bool,
    pub all_signed: bool,
    pub download_url: Option<String>,
    pub error: Option<String>,
}

/// Offline queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedSubmission {
    pub session_id: String,
    pub recipient_id: String,
    pub signing_key: String,
    pub signatures: String, // JSON serialized HashMap
    pub completed_at: String,
    pub timestamp: f64,
}

/// Signing session manager - handles session state and submission
#[wasm_bindgen]
pub struct SigningSession {
    session_id: String,
    recipient_id: String,
    signing_key: String,
    fields: Vec<SigningField>,
    signatures: std::collections::HashMap<String, String>,
    completed_fields: std::collections::HashSet<String>,
    is_online: bool,
}

#[wasm_bindgen]
impl SigningSession {
    /// Create a new signing session from URL parameters
    #[wasm_bindgen(constructor)]
    pub fn new(session_id: &str, recipient_id: &str, signing_key: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            recipient_id: recipient_id.to_string(),
            signing_key: signing_key.to_string(),
            fields: Vec::new(),
            signatures: std::collections::HashMap::new(),
            completed_fields: std::collections::HashSet::new(),
            is_online: true,
        }
    }

    /// Get session ID
    #[wasm_bindgen(getter)]
    pub fn session_id(&self) -> String {
        self.session_id.clone()
    }

    /// Get recipient ID
    #[wasm_bindgen(getter)]
    pub fn recipient_id(&self) -> String {
        self.recipient_id.clone()
    }

    /// Set online status
    #[wasm_bindgen]
    pub fn set_online(&mut self, online: bool) {
        self.is_online = online;
    }

    /// Check if online
    #[wasm_bindgen]
    pub fn is_online(&self) -> bool {
        self.is_online
    }

    /// Load fields from JSON array
    #[wasm_bindgen]
    pub fn load_fields(&mut self, fields_json: &str) -> Result<(), JsValue> {
        let fields: Vec<SigningField> =
            serde_json::from_str(fields_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.fields = fields;
        Ok(())
    }

    /// Get field count
    #[wasm_bindgen]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Get required field count
    #[wasm_bindgen]
    pub fn required_field_count(&self) -> usize {
        self.fields.iter().filter(|f| f.required).count()
    }

    /// Get completed field count
    #[wasm_bindgen]
    pub fn completed_field_count(&self) -> usize {
        self.completed_fields.len()
    }

    /// Record a signature for a field
    #[wasm_bindgen]
    pub fn record_signature(&mut self, field_id: &str, signature_data: &str) {
        self.signatures
            .insert(field_id.to_string(), signature_data.to_string());
        self.completed_fields.insert(field_id.to_string());
    }

    /// Check if a field is completed
    #[wasm_bindgen]
    pub fn is_field_completed(&self, field_id: &str) -> bool {
        self.completed_fields.contains(field_id)
    }

    /// Check if all required fields are completed
    #[wasm_bindgen]
    pub fn can_finish(&self) -> bool {
        self.fields
            .iter()
            .filter(|f| f.required)
            .all(|f| self.completed_fields.contains(&f.id))
    }

    /// Get signatures as JSON
    #[wasm_bindgen]
    pub fn get_signatures_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.signatures).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Submit signatures to the Worker API
    #[wasm_bindgen]
    pub async fn submit(&self, api_base: &str) -> Result<JsValue, JsValue> {
        if !self.can_finish() {
            return Err(JsValue::from_str(
                "Cannot submit: not all required fields are completed",
            ));
        }

        // Check if we're online
        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();
        let online = navigator.on_line();

        if !online {
            // Queue for offline submission
            self.queue_for_offline().await?;
            return self.create_result(true, false, None, Some("Queued for offline sync"));
        }

        // Build the request
        let url = format!("{}/session/{}/signed", api_base, self.session_id);

        let body = serde_json::json!({
            "recipient_id": self.recipient_id,
            "signatures": self.signatures,
            "completed_at": js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default()
        });

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        let body_str =
            serde_json::to_string(&body).map_err(|e| JsValue::from_str(&e.to_string()))?;
        opts.set_body(&JsValue::from_str(&body_str));

        let request = Request::new_with_str_and_init(&url, &opts)?;
        request.headers().set("Content-Type", "application/json")?;
        request
            .headers()
            .set("X-Recipient-Id", &self.recipient_id)?;
        request.headers().set("X-Signing-Key", &self.signing_key)?;

        let response = JsFuture::from(window.fetch_with_request(&request)).await?;
        let response: Response = response.dyn_into()?;

        if !response.ok() {
            return Err(JsValue::from_str(&format!(
                "Submission failed: {}",
                response.status()
            )));
        }

        let json = JsFuture::from(response.json()?).await?;

        // Extract response data
        let all_signed = Reflect::get(&json, &"all_signed".into())
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let download_url = Reflect::get(&json, &"download_url".into())
            .ok()
            .and_then(|v| v.as_string());

        self.create_result(true, all_signed, download_url.as_deref(), None)
    }

    /// Queue submission for offline sync
    async fn queue_for_offline(&self) -> Result<(), JsValue> {
        let queued = QueuedSubmission {
            session_id: self.session_id.clone(),
            recipient_id: self.recipient_id.clone(),
            signing_key: self.signing_key.clone(),
            signatures: serde_json::to_string(&self.signatures)
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
            completed_at: js_sys::Date::new_0()
                .to_iso_string()
                .as_string()
                .unwrap_or_default(),
            timestamp: js_sys::Date::now(),
        };

        // Store in localStorage (simpler than IndexedDB for queue)
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let queue_json = storage
            .get_item("offline_queue")?
            .unwrap_or_else(|| "[]".to_string());

        let mut queue: Vec<QueuedSubmission> =
            serde_json::from_str(&queue_json).unwrap_or_else(|_| Vec::new());

        queue.push(queued);

        let updated_json =
            serde_json::to_string(&queue).map_err(|e| JsValue::from_str(&e.to_string()))?;

        storage.set_item("offline_queue", &updated_json)?;

        web_sys::console::log_1(&"Queued submission for offline sync".into());

        Ok(())
    }

    /// Create a result object for JS
    fn create_result(
        &self,
        success: bool,
        all_signed: bool,
        download_url: Option<&str>,
        message: Option<&str>,
    ) -> Result<JsValue, JsValue> {
        let result = Object::new();
        Reflect::set(&result, &"success".into(), &success.into())?;
        Reflect::set(&result, &"allSigned".into(), &all_signed.into())?;

        if let Some(url) = download_url {
            Reflect::set(&result, &"downloadUrl".into(), &url.into())?;
        }

        if let Some(msg) = message {
            Reflect::set(&result, &"message".into(), &msg.into())?;
        }

        Ok(result.into())
    }

    /// Get fields as JavaScript array
    #[wasm_bindgen]
    pub fn get_fields(&self) -> Result<JsValue, JsValue> {
        let arr = Array::new();
        for field in &self.fields {
            let obj = Object::new();
            Reflect::set(&obj, &"id".into(), &field.id.clone().into())?;
            Reflect::set(&obj, &"type".into(), &field.field_type.clone().into())?;
            Reflect::set(&obj, &"page".into(), &field.page.into())?;
            Reflect::set(&obj, &"x".into(), &field.x.into())?;
            Reflect::set(&obj, &"y".into(), &field.y.into())?;
            Reflect::set(&obj, &"width".into(), &field.width.into())?;
            Reflect::set(&obj, &"height".into(), &field.height.into())?;
            Reflect::set(&obj, &"required".into(), &field.required.into())?;
            Reflect::set(
                &obj,
                &"recipientId".into(),
                &field.recipient_id.clone().into(),
            )?;
            Reflect::set(
                &obj,
                &"completed".into(),
                &self.completed_fields.contains(&field.id).into(),
            )?;
            arr.push(&obj);
        }
        Ok(arr.into())
    }
}

/// Process offline queue - call this when coming back online
#[wasm_bindgen]
pub async fn sync_offline_queue(api_base: &str) -> Result<u32, JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let storage = window.local_storage()?.ok_or("No localStorage")?;

    let queue_json = storage
        .get_item("offline_queue")?
        .unwrap_or_else(|| "[]".to_string());

    let queue: Vec<QueuedSubmission> =
        serde_json::from_str(&queue_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    if queue.is_empty() {
        return Ok(0);
    }

    let mut synced = 0;
    let mut remaining = Vec::new();

    for item in queue {
        let url = format!("{}/session/{}/signed", api_base, item.session_id);

        let body = serde_json::json!({
            "recipient_id": item.recipient_id,
            "signatures": serde_json::from_str::<serde_json::Value>(&item.signatures).unwrap_or_default(),
            "completed_at": item.completed_at
        });

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        let body_str = serde_json::to_string(&body).unwrap_or_default();
        opts.set_body(&JsValue::from_str(&body_str));

        let request = Request::new_with_str_and_init(&url, &opts)?;
        request.headers().set("Content-Type", "application/json")?;
        request
            .headers()
            .set("X-Recipient-Id", &item.recipient_id)?;
        request.headers().set("X-Signing-Key", &item.signing_key)?;

        match JsFuture::from(window.fetch_with_request(&request)).await {
            Ok(response) => {
                let response: Response = response.dyn_into()?;
                if response.ok() {
                    synced += 1;
                    web_sys::console::log_1(
                        &format!("Synced offline submission: {}", item.session_id).into(),
                    );
                } else {
                    remaining.push(item);
                }
            }
            Err(_) => {
                remaining.push(item);
            }
        }
    }

    // Update queue with remaining items
    let updated_json =
        serde_json::to_string(&remaining).map_err(|e| JsValue::from_str(&e.to_string()))?;
    storage.set_item("offline_queue", &updated_json)?;

    Ok(synced)
}

/// Get offline queue length
#[wasm_bindgen]
pub fn get_offline_queue_length() -> Result<u32, JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let storage = window.local_storage()?.ok_or("No localStorage")?;

    let queue_json = storage
        .get_item("offline_queue")?
        .unwrap_or_else(|| "[]".to_string());

    let queue: Vec<QueuedSubmission> =
        serde_json::from_str(&queue_json).unwrap_or_else(|_| Vec::new());

    Ok(queue.len() as u32)
}

/// Check if navigator is online
#[wasm_bindgen]
pub fn is_navigator_online() -> bool {
    web_sys::window()
        .map(|w| w.navigator().on_line())
        .unwrap_or(false)
}
