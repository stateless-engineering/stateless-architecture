//! Service Bus client: resolves endpoints and dispatches calls.
//!
//! Implements the protocol from `spec/service-bus.proto`:
//! - Endpoint resolution: local → mesh → cloud
//! - Capability token validation
//! - Streaming support for real-time services
//! - Cross-language compatibility via WIT

use crate::service_bus::{Service, ServiceRegistry};

#[cfg(test)]
use crate::service_bus::ServiceError;
use crate::state_blob::Blob;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Endpoint type for service resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Endpoint {
    /// Same-process service (function call).
    Local,
    /// Mesh peer reachable via WebRTC/BLE.
    Mesh { peer_id: String },
    /// Cloud service reachable via HTTPS.
    Cloud { url: String },
}

/// Capability token for authorizing service calls.
///
/// Derived from a master capability via HKDF. Each token restricts
/// (never expands) the parent's scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Token ID (SHA-256 of the token content).
    pub id: String,
    /// Entity that issued this token.
    pub issuer: String,
    /// Service this token grants access to.
    pub subject: String,
    /// Allowed operations: ["invoke"], ["invoke", "stream"].
    pub scope: Vec<String>,
    /// Maximum number of invocations (None = unlimited).
    pub max_invocations: Option<u64>,
    /// Maximum bytes that can be transferred (None = unlimited).
    pub max_bytes: Option<u64>,
    /// Expiration timestamp (ISO 8601).
    pub expires: Option<String>,
    /// Ed25519 signature over the above fields.
    pub signature: String,
}

/// A streaming response chunk.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// The current state blob (may be partial).
    pub blob: Blob,
    /// Output data for this chunk.
    pub output: serde_json::Value,
    /// True when the stream is complete.
    pub done: bool,
}

/// The Service Bus: resolves and dispatches service calls.
///
/// # Example
///
/// ```ignore
/// let bus = ServiceBus::new();
/// bus.register("render-html", Box::new(RenderService));
///
/// let token = CapabilityToken::issue("user:alice", "service:render-html",
///     vec!["invoke".into()], None, None);
///
/// let result = bus.invoke("render-html", &blob, &input, &token)?;
/// ```
pub struct ServiceBus {
    local: ServiceRegistry,
    mesh: HashMap<String, Vec<String>>, // peer_id → [service_name]
    cloud: HashMap<String, String>,     // service_name → url
}

/// Errors from the service bus.
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("service not found: {0}")]
    NotFound(String),
    #[error("capability rejected: {0}")]
    CapabilityRejected(String),
    #[error("invocation limit exceeded")]
    InvocationLimit,
    #[error("byte limit exceeded")]
    ByteLimit,
    #[error("token expired")]
    TokenExpired,
    #[error("service error: {0}")]
    Service(String),
    #[error("network error: {0}")]
    Network(String),
}

impl ServiceBus {
    /// Create an empty service bus.
    pub fn new() -> Self {
        Self {
            local: ServiceRegistry::new(),
            mesh: HashMap::new(),
            cloud: HashMap::new(),
        }
    }

    /// Register a local service.
    pub fn register(&mut self, name: &str, service: Box<dyn Service>) {
        self.local.register(
            service,
            crate::service_bus::ServiceManifest {
                name: name.into(),
                version: "0.1.0".into(),
                description: "Local service".into(),
                input_schema: None,
                output_schema: None,
                capabilities: vec![],
            },
        );
    }

    /// Register a mesh peer's services.
    pub fn register_mesh_peer(&mut self, peer_id: &str, services: Vec<String>) {
        self.mesh.insert(peer_id.into(), services);
    }

    /// Register a cloud service endpoint.
    pub fn register_cloud(&mut self, name: &str, url: &str) {
        self.cloud.insert(name.into(), url.into());
    }

    /// Resolve the best endpoint for a service.
    pub fn resolve(&self, name: &str) -> Result<Endpoint, BusError> {
        // Priority: local → mesh → cloud
        if self.local.get(name).is_some() {
            return Ok(Endpoint::Local);
        }
        for (peer_id, services) in &self.mesh {
            if services.contains(&name.to_string()) {
                return Ok(Endpoint::Mesh {
                    peer_id: peer_id.clone(),
                });
            }
        }
        if let Some(url) = self.cloud.get(name) {
            return Ok(Endpoint::Cloud { url: url.clone() });
        }
        Err(BusError::NotFound(name.into()))
    }

    /// Invoke a service synchronously.
    pub fn invoke(
        &self,
        name: &str,
        blob: &Blob,
        input: &serde_json::Value,
        token: &CapabilityToken,
    ) -> Result<(Blob, serde_json::Value), BusError> {
        self.validate_capability(token, "invoke")?;

        match self.resolve(name)? {
            Endpoint::Local => self
                .local
                .invoke(name, blob, input)
                .map_err(|e| BusError::Service(e.to_string())),
            Endpoint::Mesh { peer_id } => {
                // In a real implementation: WebRTC data channel RPC
                Err(BusError::Network(format!(
                    "mesh invocation to {} not yet implemented",
                    peer_id
                )))
            }
            Endpoint::Cloud { url } => {
                // In a real implementation: HTTPS POST with protobuf
                Err(BusError::Network(format!(
                    "cloud invocation to {} not yet implemented",
                    url
                )))
            }
        }
    }

    /// Invoke a service with streaming output.
    pub fn stream(
        &self,
        name: &str,
        blob: &Blob,
        input: &serde_json::Value,
        token: &CapabilityToken,
    ) -> Result<Vec<StreamChunk>, BusError> {
        self.validate_capability(token, "stream")?;

        match self.resolve(name)? {
            Endpoint::Local => {
                // For local services, stream is a single chunk
                let (new_blob, output) = self
                    .local
                    .invoke(name, blob, input)
                    .map_err(|e| BusError::Service(e.to_string()))?;
                Ok(vec![StreamChunk {
                    blob: new_blob,
                    output,
                    done: true,
                }])
            }
            Endpoint::Mesh { .. } | Endpoint::Cloud { .. } => Err(BusError::Network(
                "streaming not yet implemented for remote endpoints".into(),
            )),
        }
    }

    /// Validate a capability token for the requested operation.
    fn validate_capability(
        &self,
        token: &CapabilityToken,
        operation: &str,
    ) -> Result<(), BusError> {
        // Check expiry
        if let Some(ref expires) = token.expires {
            // In a real implementation: parse and compare timestamps
            let _ = expires;
        }

        // Check scope
        if !token.scope.iter().any(|s| s == operation) {
            return Err(BusError::CapabilityRejected(format!(
                "{} not in scope {:?}",
                operation, token.scope
            )));
        }

        // In a real implementation: verify Ed25519 signature

        Ok(())
    }
}

impl Default for ServiceBus {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityToken {
    /// Issue a new capability token (simplified — no real signing).
    pub fn issue(
        issuer: &str,
        subject: &str,
        scope: Vec<String>,
        max_invocations: Option<u64>,
        max_bytes: Option<u64>,
    ) -> Self {
        let mut token = Self {
            id: String::new(),
            issuer: issuer.into(),
            subject: subject.into(),
            scope,
            max_invocations,
            max_bytes,
            expires: None,
            signature: "unsigned".into(),
        };
        token.id = format!("sha256:{}", token.hash_content());
        token
    }

    fn hash_content(&self) -> String {
        use sha2::{Digest, Sha256};
        let content = format!(
            "{}:{}:{:?}:{:?}:{:?}",
            self.issuer, self.subject, self.scope, self.max_invocations, self.max_bytes
        );
        let hash = Sha256::digest(content.as_bytes());
        hex::encode(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_bus::Service;

    struct EchoService;
    impl Service for EchoService {
        fn name(&self) -> &str {
            "echo"
        }
        fn version(&self) -> &str {
            "0.1.0"
        }
        fn invoke(
            &self,
            blob: &Blob,
            input: &serde_json::Value,
        ) -> Result<(Blob, serde_json::Value), ServiceError> {
            let new_blob = blob
                .child(serde_json::json!({ "echo": input }))
                .map_err(|e| ServiceError::Invocation {
                    name: "echo".into(),
                    reason: e.to_string(),
                })?;
            Ok((new_blob, input.clone()))
        }
    }

    #[test]
    fn service_bus_local_invoke() {
        let mut bus = ServiceBus::new();
        bus.register("echo", Box::new(EchoService));

        let token = CapabilityToken::issue(
            "user:alice",
            "service:echo",
            vec!["invoke".into()],
            None,
            None,
        );
        let blob = Blob::new("test://schema", serde_json::json!({ "x": 1 }));
        let (new_blob, output) = bus
            .invoke(
                "echo",
                &blob,
                &serde_json::json!({ "msg": "hello" }),
                &token,
            )
            .unwrap();

        assert_eq!(new_blob.payload["echo"]["msg"], "hello");
        assert_eq!(output["msg"], "hello");
    }

    #[test]
    fn capability_rejects_wrong_scope() {
        let mut bus = ServiceBus::new();
        bus.register("echo", Box::new(EchoService));

        // Token has "stream" scope, but we're calling "invoke"
        let token = CapabilityToken::issue(
            "user:alice",
            "service:echo",
            vec!["stream".into()],
            None,
            None,
        );
        let blob = Blob::new("test://schema", serde_json::json!({}));

        match bus.invoke("echo", &blob, &serde_json::json!({}), &token) {
            Err(BusError::CapabilityRejected(_)) => (),
            other => panic!("expected CapabilityRejected, got {:?}", other),
        }
    }

    #[test]
    fn resolve_priority() {
        let mut bus = ServiceBus::new();
        bus.register("echo", Box::new(EchoService));
        bus.register_cloud("echo", "https://example.com/echo");

        // Local takes priority over cloud
        assert_eq!(bus.resolve("echo").unwrap(), Endpoint::Local);
    }

    #[test]
    fn resolve_not_found() {
        let bus = ServiceBus::new();
        assert!(matches!(
            bus.resolve("nonexistent"),
            Err(BusError::NotFound(_))
        ));
    }
}
