//! IndexedDB wrapper for persistent local storage of documents and keys

use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbOpenDbRequest, IdbRequest, IdbTransactionMode};

const DB_NAME: &str = "agentpdf_local";
const DB_VERSION: u32 = 1;

const STORE_DOCUMENTS: &str = "documents";
const STORE_KEYS: &str = "keys";
const STORE_DRAFTS: &str = "drafts";

/// IndexedDB storage manager
#[wasm_bindgen]
pub struct Storage {
    db: IdbDatabase,
}

/// Initialize the database - call this before using Storage
#[wasm_bindgen]
pub async fn init_storage() -> Result<Storage, JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let idb = window.indexed_db()?.ok_or("IndexedDB not available")?;

    let request = idb.open_with_u32(DB_NAME, DB_VERSION)?;

    // Set up upgrade handler using a Promise
    let db = JsFuture::from(open_db_promise(&request)).await?;
    let db: IdbDatabase = db.unchecked_into();

    Ok(Storage { db })
}

/// Create a promise that handles database opening with upgrade
fn open_db_promise(request: &IdbOpenDbRequest) -> js_sys::Promise {
    let request = request.clone();

    js_sys::Promise::new(&mut move |resolve, reject| {
        let request_for_upgrade = request.clone();
        let request_for_success = request.clone();
        let resolve_clone = resolve.clone();
        let reject_clone = reject.clone();

        // Handle upgrade needed
        let onupgradeneeded = Closure::once(Box::new(move |_event: web_sys::Event| {
            if let Ok(result) = request_for_upgrade.result() {
                let db: IdbDatabase = result.unchecked_into();
                // Create object stores (ignore errors if they already exist)
                let _ = db.create_object_store(STORE_DOCUMENTS);
                let _ = db.create_object_store(STORE_KEYS);
                let _ = db.create_object_store(STORE_DRAFTS);
            }
        }) as Box<dyn FnOnce(_)>);

        let onsuccess = Closure::once(Box::new(move |_event: web_sys::Event| {
            if let Ok(result) = request_for_success.result() {
                let _ = resolve_clone.call1(&JsValue::NULL, &result);
            }
        }) as Box<dyn FnOnce(_)>);

        let onerror = Closure::once(Box::new(move |_event: web_sys::Event| {
            let _ = reject_clone.call1(
                &JsValue::NULL,
                &JsValue::from_str("Failed to open database"),
            );
        }) as Box<dyn FnOnce(_)>);

        request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
        request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        onupgradeneeded.forget();
        onsuccess.forget();
        onerror.forget();
    })
}

#[wasm_bindgen]
impl Storage {
    /// Store a document blob
    #[wasm_bindgen]
    pub async fn store_document(
        &self,
        id: &str,
        data: Vec<u8>,
        filename: &str,
    ) -> Result<(), JsValue> {
        let tx = self
            .db
            .transaction_with_str_and_mode(STORE_DOCUMENTS, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_DOCUMENTS)?;

        let obj = Object::new();
        let uint8_array = Uint8Array::from(data.as_slice());
        Reflect::set(&obj, &"data".into(), &uint8_array)?;
        Reflect::set(&obj, &"filename".into(), &filename.into())?;
        Reflect::set(&obj, &"created_at".into(), &js_sys::Date::now().into())?;

        let request = store.put_with_key(&obj, &id.into())?;
        JsFuture::from(request_to_promise(&request)).await?;

        Ok(())
    }

    /// Retrieve a document blob
    #[wasm_bindgen]
    pub async fn get_document(&self, id: &str) -> Result<JsValue, JsValue> {
        let tx = self.db.transaction_with_str(STORE_DOCUMENTS)?;
        let store = tx.object_store(STORE_DOCUMENTS)?;

        let request = store.get(&id.into())?;
        let result = JsFuture::from(request_to_promise(&request)).await?;

        if result.is_undefined() || result.is_null() {
            return Ok(JsValue::NULL);
        }

        let data = Reflect::get(&result, &"data".into())?;
        Ok(data)
    }

    /// Delete a document
    #[wasm_bindgen]
    pub async fn delete_document(&self, id: &str) -> Result<(), JsValue> {
        let tx = self
            .db
            .transaction_with_str_and_mode(STORE_DOCUMENTS, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_DOCUMENTS)?;

        let request = store.delete(&id.into())?;
        JsFuture::from(request_to_promise(&request)).await?;

        Ok(())
    }

    /// Store an ephemeral key
    #[wasm_bindgen]
    pub async fn store_key(&self, document_id: &str, key_data: Vec<u8>) -> Result<(), JsValue> {
        let tx = self
            .db
            .transaction_with_str_and_mode(STORE_KEYS, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_KEYS)?;

        let obj = Object::new();
        let uint8_array = Uint8Array::from(key_data.as_slice());
        Reflect::set(&obj, &"key_data".into(), &uint8_array)?;
        Reflect::set(&obj, &"created_at".into(), &js_sys::Date::now().into())?;

        let request = store.put_with_key(&obj, &document_id.into())?;
        JsFuture::from(request_to_promise(&request)).await?;

        Ok(())
    }

    /// Retrieve an ephemeral key
    #[wasm_bindgen]
    pub async fn get_key(&self, document_id: &str) -> Result<JsValue, JsValue> {
        let tx = self.db.transaction_with_str(STORE_KEYS)?;
        let store = tx.object_store(STORE_KEYS)?;

        let request = store.get(&document_id.into())?;
        let result = JsFuture::from(request_to_promise(&request)).await?;

        if result.is_undefined() || result.is_null() {
            return Ok(JsValue::NULL);
        }

        let key_data = Reflect::get(&result, &"key_data".into())?;
        Ok(key_data)
    }

    /// Delete an ephemeral key
    #[wasm_bindgen]
    pub async fn delete_key(&self, document_id: &str) -> Result<(), JsValue> {
        let tx = self
            .db
            .transaction_with_str_and_mode(STORE_KEYS, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_KEYS)?;

        let request = store.delete(&document_id.into())?;
        JsFuture::from(request_to_promise(&request)).await?;

        Ok(())
    }

    /// Store a draft document state (auto-save)
    #[wasm_bindgen]
    pub async fn store_draft(&self, id: &str, state_json: &str) -> Result<(), JsValue> {
        let tx = self
            .db
            .transaction_with_str_and_mode(STORE_DRAFTS, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_DRAFTS)?;

        let obj = Object::new();
        Reflect::set(&obj, &"state".into(), &state_json.into())?;
        Reflect::set(&obj, &"updated_at".into(), &js_sys::Date::now().into())?;

        let request = store.put_with_key(&obj, &id.into())?;
        JsFuture::from(request_to_promise(&request)).await?;

        Ok(())
    }

    /// Retrieve a draft document state
    #[wasm_bindgen]
    pub async fn get_draft(&self, id: &str) -> Result<JsValue, JsValue> {
        let tx = self.db.transaction_with_str(STORE_DRAFTS)?;
        let store = tx.object_store(STORE_DRAFTS)?;

        let request = store.get(&id.into())?;
        let result = JsFuture::from(request_to_promise(&request)).await?;

        if result.is_undefined() || result.is_null() {
            return Ok(JsValue::NULL);
        }

        let state = Reflect::get(&result, &"state".into())?;
        Ok(state)
    }

    /// List all draft IDs
    #[wasm_bindgen]
    pub async fn list_drafts(&self) -> Result<Array, JsValue> {
        let tx = self.db.transaction_with_str(STORE_DRAFTS)?;
        let store = tx.object_store(STORE_DRAFTS)?;

        let request = store.get_all_keys()?;
        let result = JsFuture::from(request_to_promise(&request)).await?;

        Ok(result.unchecked_into())
    }

    /// Delete a draft
    #[wasm_bindgen]
    pub async fn delete_draft(&self, id: &str) -> Result<(), JsValue> {
        let tx = self
            .db
            .transaction_with_str_and_mode(STORE_DRAFTS, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(STORE_DRAFTS)?;

        let request = store.delete(&id.into())?;
        JsFuture::from(request_to_promise(&request)).await?;

        Ok(())
    }

    /// Clear all data (for testing/reset)
    #[wasm_bindgen]
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let stores = Array::of3(
            &STORE_DOCUMENTS.into(),
            &STORE_KEYS.into(),
            &STORE_DRAFTS.into(),
        );

        let tx = self
            .db
            .transaction_with_str_sequence_and_mode(&stores, IdbTransactionMode::Readwrite)?;

        for store_name in [STORE_DOCUMENTS, STORE_KEYS, STORE_DRAFTS] {
            let store = tx.object_store(store_name)?;
            store.clear()?;
        }

        Ok(())
    }
}

/// Convert an IdbRequest to a Promise for async/await
fn request_to_promise(request: &IdbRequest) -> js_sys::Promise {
    let request = request.clone();

    js_sys::Promise::new(&mut |resolve, reject| {
        let request_success = request.clone();
        let resolve_clone = resolve.clone();
        let reject_clone = reject.clone();

        let onsuccess = Closure::once(Box::new(move |_event: web_sys::Event| {
            let result = request_success.result().unwrap_or(JsValue::NULL);
            let _ = resolve_clone.call1(&JsValue::NULL, &result);
        }) as Box<dyn FnOnce(_)>);

        let onerror = Closure::once(Box::new(move |_event: web_sys::Event| {
            let _ = reject_clone.call1(
                &JsValue::NULL,
                &JsValue::from_str("IndexedDB request failed"),
            );
        }) as Box<dyn FnOnce(_)>);

        request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        onsuccess.forget();
        onerror.forget();
    })
}

/// Helper to convert Uint8Array to Vec<u8>
#[wasm_bindgen]
pub fn uint8_array_to_vec(arr: &Uint8Array) -> Vec<u8> {
    arr.to_vec()
}

/// Helper to convert Vec<u8> to Uint8Array
#[wasm_bindgen]
pub fn vec_to_uint8_array(data: Vec<u8>) -> Uint8Array {
    Uint8Array::from(data.as_slice())
}
