//! WASM bindings for the stateless-architecture reference implementation.
//!
//! Exposes the core loop to JavaScript: blob creation, rendering, and
//! progressive restoration — all callable from a browser or Node.js.

use wasm_bindgen::prelude::*;

pub mod blob;
pub mod render;
pub mod restore;
pub mod service;

use blob::Blob;

/// Create a new state blob (WASM-exported).
#[wasm_bindgen]
pub fn create_blob(schema: &str, payload_json: &str) -> Result<JsValue, JsValue> {
    let payload: serde_json::Value = serde_json::from_str(payload_json)
        .map_err(|e| JsValue::from_str(&format!("invalid payload JSON: {}", e)))?;
    let blob = Blob::new(schema, payload);
    let bytes = blob.serialize()
        .map_err(|e| JsValue::from_str(&format!("serialization failed: {}", e)))?;
    Ok(JsValue::from_str(&String::from_utf8_lossy(&bytes)))
}

/// Compute the hash of a blob (WASM-exported).
#[wasm_bindgen]
pub fn blob_hash(blob_json: &str) -> Result<String, JsValue> {
    let blob = Blob::deserialize(blob_json.as_bytes())
        .map_err(|e| JsValue::from_str(&format!("deserialization failed: {}", e)))?;
    blob.hash()
        .map_err(|e| JsValue::from_str(&format!("hash failed: {}", e)))
}
