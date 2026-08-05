//! State blob: the serializable representation of an entire system's state.
//!
//! The blob is the **only** thing that crosses the eviction boundary.
//! Everything else is a throwaway projection.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Current blob format version.
pub const BLOB_VERSION: &str = "0.1.0";

/// A state blob — the single source of truth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blob {
    /// Format version (semver).
    pub version: String,
    /// Schema URI this blob conforms to.
    pub schema: String,
    /// Creation timestamp (ISO 8601).
    pub timestamp: String,
    /// Application-specific state.
    pub payload: serde_json::Value,
    /// Optional seal (encrypted at rest).
    pub seal: Option<BlobSeal>,
    /// Hash of the parent blob (for versioning).
    pub parent: Option<String>,
}

/// Encryption envelope for a sealed blob.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlobSeal {
    /// Encryption algorithm (e.g. "AES-256-GCM").
    pub algorithm: String,
    /// Ciphertext (hex-encoded).
    pub ciphertext: String,
    /// Nonce/IV (hex-encoded).
    pub nonce: String,
}

/// Errors that can occur when working with blobs.
#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("deserialization failed: {0}")]
    Deserialization(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("seal error: {0}")]
    Seal(String),
}

impl Blob {
    /// Create a new state blob.
    pub fn new(schema: impl Into<String>, payload: serde_json::Value) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            version: BLOB_VERSION.to_string(),
            schema: schema.into(),
            timestamp: format!("{}T00:00:00Z", timestamp), // simplified
            payload,
            seal: None,
            parent: None,
        }
    }

    /// Serialize the blob to JSON bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, BlobError> {
        serde_json::to_vec(self).map_err(|e| BlobError::Serialization(e.to_string()))
    }

    /// Deserialize a blob from JSON bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, BlobError> {
        serde_json::from_slice(bytes).map_err(|e| BlobError::Deserialization(e.to_string()))
    }

    /// Compute the content hash of this blob (SHA-256, hex-encoded).
    pub fn hash(&self) -> Result<String, BlobError> {
        let bytes = self.serialize()?;
        let hash = Sha256::digest(&bytes);
        Ok(hex::encode(hash))
    }

    /// Validate the blob against basic structural rules.
    pub fn validate(&self) -> Result<(), BlobError> {
        if self.version.is_empty() {
            return Err(BlobError::Validation("version is required".into()));
        }
        if self.schema.is_empty() {
            return Err(BlobError::Validation("schema is required".into()));
        }
        Ok(())
    }

    /// Create a new blob that descends from this one.
    pub fn child(&self, payload: serde_json::Value) -> Result<Self, BlobError> {
        let mut blob = Blob::new(self.schema.clone(), payload);
        blob.parent = Some(self.hash()?);
        Ok(blob)
    }

    /// Hibernate: serialize and seal with encryption.
    ///
    /// This is the high-level API for putting a blob to sleep.
    /// It serializes the blob to JSON, then seals the payload with
    /// the given passphrase (using a simplified XOR cipher for the
    /// reference implementation — production would use AES-256-GCM).
    pub fn hibernate(&self, passphrase: &str) -> Result<Vec<u8>, BlobError> {
        let mut sealed = self.clone();
        let payload_bytes = serde_json::to_vec(&sealed.payload)
            .map_err(|e| BlobError::Serialization(e.to_string()))?;
        let key = Self::derive_key(passphrase);
        let ciphertext: Vec<u8> = payload_bytes
            .iter()
            .zip(key.iter().cycle())
            .map(|(p, k)| p ^ k)
            .collect();
        sealed.seal = Some(BlobSeal {
            algorithm: "XOR-REF".into(),
            ciphertext: hex::encode(&ciphertext),
            nonce: hex::encode(&key[..12]),
        });
        sealed.payload = serde_json::json!(null);
        sealed.serialize()
    }

    /// Restore: deserialize and unseal with decryption.
    ///
    /// This is the high-level API for waking a blob up.
    /// It deserializes the blob from JSON, then unseals the payload
    /// with the given passphrase.
    pub fn restore(data: &[u8], passphrase: &str) -> Result<Self, BlobError> {
        let mut blob = Self::deserialize(data)?;
        if let Some(seal) = blob.seal.take() {
            let key = Self::derive_key(passphrase);
            let ciphertext =
                hex::decode(&seal.ciphertext).map_err(|e| BlobError::Seal(e.to_string()))?;
            let payload_bytes: Vec<u8> = ciphertext
                .iter()
                .zip(key.iter().cycle())
                .map(|(c, k)| c ^ k)
                .collect();
            let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
                .map_err(|e| BlobError::Deserialization(e.to_string()))?;
            blob.payload = payload;
        }
        Ok(blob)
    }

    /// Derive a 32-byte key from a passphrase using SHA-256.
    fn derive_key(passphrase: &str) -> Vec<u8> {
        Sha256::digest(passphrase.as_bytes()).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_roundtrip() {
        let payload = serde_json::json!({ "counter": 42 });
        let blob = Blob::new("test://schema", payload.clone());
        let bytes = blob.serialize().unwrap();
        let restored = Blob::deserialize(&bytes).unwrap();
        assert_eq!(restored.payload, payload);
        assert_eq!(restored.version, BLOB_VERSION);
    }

    #[test]
    fn blob_hash_is_deterministic() {
        let payload = serde_json::json!({ "x": 1 });
        let blob = Blob::new("test://schema", payload);
        let h1 = blob.hash().unwrap();
        let h2 = blob.hash().unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn blob_child_tracks_parent() {
        let parent = Blob::new("test://schema", serde_json::json!({ "a": 1 }));
        let child = parent.child(serde_json::json!({ "a": 2 })).unwrap();
        assert_eq!(child.parent, Some(parent.hash().unwrap()));
    }

    #[test]
    fn blob_validation_rejects_empty_version() {
        let mut blob = Blob::new("test://schema", serde_json::json!({}));
        blob.version.clear();
        assert!(blob.validate().is_err());
    }

    #[test]
    fn hibernate_and_restore_roundtrip() {
        let blob = Blob::new(
            "test://app",
            serde_json::json!({ "title": "Hello", "count": 42 }),
        );

        // Hibernate with passphrase.
        let sealed = blob.hibernate("my-secret-passphrase").unwrap();

        // Sealed blob should have null payload and a seal.
        let sealed_blob = Blob::deserialize(&sealed).unwrap();
        assert!(sealed_blob.seal.is_some());
        assert_eq!(sealed_blob.payload, serde_json::json!(null));

        // Restore with correct passphrase.
        let restored = Blob::restore(&sealed, "my-secret-passphrase").unwrap();
        assert_eq!(restored.payload["title"], "Hello");
        assert_eq!(restored.payload["count"], 42);
        assert!(restored.seal.is_none());
    }

    #[test]
    fn restore_with_wrong_passphrase_fails() {
        let blob = Blob::new("test://app", serde_json::json!({ "secret": "data" }));
        let sealed = blob.hibernate("correct-passphrase").unwrap();

        // Wrong passphrase produces garbage, which fails to parse as JSON.
        assert!(Blob::restore(&sealed, "wrong-passphrase").is_err());
    }
}
