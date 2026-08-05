//! Unit tests for the state blob data structure.

use stateless_architecture::{Blob, BlobSeal};

#[test]
fn blob_creation() {
    let blob = Blob::new("test://schema", serde_json::json!({ "x": 1 }));
    assert_eq!(blob.version, "0.1.0");
    assert_eq!(blob.schema, "test://schema");
    assert_eq!(blob.payload["x"], 1);
    assert!(blob.seal.is_none());
    assert!(blob.parent.is_none());
}

#[test]
fn blob_roundtrip_serialization() {
    let payload = serde_json::json!({ "counter": 42, "name": "test" });
    let blob = Blob::new("test://schema", payload.clone());
    let bytes = blob.serialize().unwrap();
    let restored = Blob::deserialize(&bytes).unwrap();
    assert_eq!(restored.payload, payload);
    assert_eq!(restored.version, blob.version);
    assert_eq!(restored.schema, blob.schema);
}

#[test]
fn blob_hash_is_deterministic() {
    let blob = Blob::new("test://schema", serde_json::json!({ "x": 1 }));
    let h1 = blob.hash().unwrap();
    let h2 = blob.hash().unwrap();
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64); // SHA-256 hex
}

#[test]
fn blob_hash_changes_with_payload() {
    let b1 = Blob::new("test://schema", serde_json::json!({ "x": 1 }));
    let b2 = Blob::new("test://schema", serde_json::json!({ "x": 2 }));
    assert_ne!(b1.hash().unwrap(), b2.hash().unwrap());
}

#[test]
fn blob_child_tracking() {
    let parent = Blob::new("test://schema", serde_json::json!({ "a": 1 }));
    let child = parent.child(serde_json::json!({ "a": 2 })).unwrap();
    assert_eq!(child.parent, Some(parent.hash().unwrap()));
}

#[test]
fn blob_with_seal() {
    let seal = BlobSeal {
        algorithm: "AES-256-GCM".into(),
        ciphertext: "deadbeef".into(),
        nonce: "cafebabe".into(),
    };
    let mut blob = Blob::new("test://schema", serde_json::json!({}));
    blob.seal = Some(seal.clone());
    let bytes = blob.serialize().unwrap();
    let restored = Blob::deserialize(&bytes).unwrap();
    assert_eq!(restored.seal, Some(seal));
}

#[test]
fn blob_validation_rejects_empty_fields() {
    let mut blob = Blob::new("test://schema", serde_json::json!({}));
    blob.version.clear();
    assert!(blob.validate().is_err());
}

#[test]
fn blob_inequality() {
    let b1 = Blob::new("s", serde_json::json!({ "x": 1 }));
    let b2 = Blob::new("s", serde_json::json!({ "x": 2 }));
    assert_ne!(b1, b2);
}
