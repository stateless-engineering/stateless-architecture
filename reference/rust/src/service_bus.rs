//! Service contract: a service is a pure function `(blob, input) → (blob, output)`.
//!
//! Services are stateless — no ambient state, no config files, no hidden inputs.
//! Same inputs always produce same outputs.

use crate::state_blob::{Blob, BlobError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A stateless service: transforms a blob and produces output.
///
/// Implementors must ensure:
/// - **Pure**: same `(blob, input)` → same `(new_blob, output)`, always.
/// - **No ambient state**: no config files, no global mutable state.
/// - **Deterministic**: fixed ordering, no hidden clock dependencies.
pub trait Service: Send + Sync {
    /// The service name.
    fn name(&self) -> &str;

    /// The service version (semver).
    fn version(&self) -> &str;

    /// Execute the service: `(blob, input) → (new_blob, output)`.
    fn invoke(
        &self,
        blob: &Blob,
        input: &serde_json::Value,
    ) -> Result<(Blob, serde_json::Value), ServiceError>;
}

/// Errors that can occur during service invocation.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("service {name} failed: {reason}")]
    Invocation { name: String, reason: String },
    #[error("blob error: {0}")]
    Blob(#[from] BlobError),
    #[error("unknown service: {0}")]
    UnknownService(String),
}

/// A service manifest — declares a service's interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub capabilities: Vec<String>,
}

/// A registry of available services.
///
/// Discovery is path-based: a service exists if it's in the registry.
/// No ambient config, no network discovery.
pub struct ServiceRegistry {
    services: HashMap<String, Box<dyn Service>>,
    manifests: HashMap<String, ServiceManifest>,
}

impl ServiceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            manifests: HashMap::new(),
        }
    }

    /// Register a service.
    pub fn register(&mut self, service: Box<dyn Service>, manifest: ServiceManifest) {
        self.services.insert(manifest.name.clone(), service);
        self.manifests.insert(manifest.name.clone(), manifest);
    }

    /// Look up a service by name.
    pub fn get(&self, name: &str) -> Option<&dyn Service> {
        self.services.get(name).map(|s| s.as_ref())
    }

    /// List all registered service manifests.
    pub fn list(&self) -> Vec<&ServiceManifest> {
        self.manifests.values().collect()
    }

    /// Invoke a service by name.
    pub fn invoke(
        &self,
        name: &str,
        blob: &Blob,
        input: &serde_json::Value,
    ) -> Result<(Blob, serde_json::Value), ServiceError> {
        let service = self
            .services
            .get(name)
            .ok_or_else(|| ServiceError::UnknownService(name.into()))?;
        service
            .invoke(blob, input)
            .map_err(|e| ServiceError::Invocation {
                name: name.into(),
                reason: e.to_string(),
            })
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test service that increments a counter in the blob payload.
    struct CounterService;

    impl Service for CounterService {
        fn name(&self) -> &str {
            "counter"
        }
        fn version(&self) -> &str {
            "0.1.0"
        }
        fn invoke(
            &self,
            blob: &Blob,
            input: &serde_json::Value,
        ) -> Result<(Blob, serde_json::Value), ServiceError> {
            let delta = input.get("delta").and_then(|v| v.as_i64()).unwrap_or(1);
            let current = blob
                .payload
                .get("count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let new_count = current + delta;
            let new_payload = serde_json::json!({ "count": new_count });
            let new_blob = blob.child(new_payload).map_err(ServiceError::Blob)?;
            Ok((new_blob, serde_json::json!({ "result": new_count })))
        }
    }

    #[test]
    fn counter_service_increments() {
        let mut reg = ServiceRegistry::new();
        reg.register(
            Box::new(CounterService),
            ServiceManifest {
                name: "counter".into(),
                version: "0.1.0".into(),
                description: "test counter".into(),
                input_schema: None,
                output_schema: None,
                capabilities: vec![],
            },
        );

        let blob = Blob::new("test://schema", serde_json::json!({ "count": 0 }));
        let (new_blob, output) = reg
            .invoke("counter", &blob, &serde_json::json!({ "delta": 5 }))
            .unwrap();

        assert_eq!(new_blob.payload["count"], 5);
        assert_eq!(output["result"], 5);
    }

    #[test]
    fn registry_returns_unknown_service_error() {
        let reg = ServiceRegistry::new();
        let blob = Blob::new("test://schema", serde_json::json!({}));
        match reg.invoke("nonexistent", &blob, &serde_json::json!({})) {
            Err(ServiceError::UnknownService(_)) => (),
            other => panic!("expected UnknownService, got {:?}", other),
        }
    }

    #[test]
    fn service_is_deterministic() {
        let mut reg = ServiceRegistry::new();
        reg.register(
            Box::new(CounterService),
            ServiceManifest {
                name: "counter".into(),
                version: "0.1.0".into(),
                description: "test".into(),
                input_schema: None,
                output_schema: None,
                capabilities: vec![],
            },
        );

        let blob = Blob::new("test://schema", serde_json::json!({ "count": 10 }));
        let (b1, o1) = reg
            .invoke("counter", &blob, &serde_json::json!({ "delta": 1 }))
            .unwrap();
        let (b2, o2) = reg
            .invoke("counter", &blob, &serde_json::json!({ "delta": 1 }))
            .unwrap();

        assert_eq!(b1.payload, b2.payload);
        assert_eq!(o1, o2);
    }
}
