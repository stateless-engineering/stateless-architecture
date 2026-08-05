//! End-to-end test: hibernate and restore a blob via the ServiceBus.
//!
//! Proves the core loop works:
//! 1. Create a blob
//! 2. Register a service on the bus
//! 3. Invoke through the bus (with capability token)
//! 4. Hibernate (serialize)
//! 5. Restore (deserialize + progressive)
//! 6. Verify state is intact

use stateless_architecture::{
    restore, Blob, CapabilityToken, HtmlRenderer, MockMesh, OpType, Operation, Service, ServiceBus,
    ServiceError,
};

/// A test service that increments a counter.
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
        let new_blob = blob
            .child(new_payload)
            .map_err(|e| ServiceError::Invocation {
                name: "counter".into(),
                reason: e.to_string(),
            })?;
        Ok((new_blob, serde_json::json!({ "result": new_count })))
    }
}

#[test]
fn hibernate_and_restore_via_bus() {
    // 1. Create a blob.
    let blob = Blob::new("test://app", serde_json::json!({ "count": 0 }));

    // 2. Register a service on the bus.
    let mut bus = ServiceBus::new();
    bus.register("counter", Box::new(CounterService));

    // Issue a capability token.
    let token = CapabilityToken::issue(
        "user:alice",
        "service:counter",
        vec!["invoke".into()],
        None,
        None,
    );

    // 3. Invoke through the bus.
    let (transformed, _) = bus
        .invoke(
            "counter",
            &blob,
            &serde_json::json!({ "delta": 42 }),
            &token,
        )
        .unwrap();
    assert_eq!(transformed.payload["count"], 42);

    // 4. Hibernate (serialize).
    let bytes = transformed.serialize().unwrap();

    // 5. Restore (deserialize).
    let restored = Blob::deserialize(&bytes).unwrap();
    assert_eq!(restored.payload["count"], 42);
    assert_eq!(restored.parent, transformed.parent);

    // 6. Progressive restoration.
    let final_blob = restore(&restored).unwrap();
    assert_eq!(final_blob.payload["count"], 42);
}

#[test]
fn capability_enforcement() {
    let mut bus = ServiceBus::new();
    bus.register("counter", Box::new(CounterService));

    // Token with wrong scope.
    let token = CapabilityToken::issue(
        "user:alice",
        "service:counter",
        vec!["stream".into()],
        None,
        None,
    );

    let blob = Blob::new("test://app", serde_json::json!({ "count": 0 }));

    assert!(bus
        .invoke("counter", &blob, &serde_json::json!({}), &token)
        .is_err());
}

#[test]
fn mesh_sync_roundtrip() {
    let mut mesh = MockMesh::new();

    mesh.broadcast(Operation {
        op_type: OpType::Insert,
        target: "/doc/title".into(),
        value: Some(serde_json::json!("Hello Mesh")),
        lamport: 1,
        origin: "peer-1".into(),
    });

    let op = mesh.log.first().unwrap();
    assert_eq!(op.target, "/doc/title");
    assert_eq!(op.origin, "peer-1");
    assert_eq!(op.lamport, 1);
}

#[test]
fn hibernate_passphrase_restore_and_render() {
    // 1. Create a blob with app data.
    let blob = Blob::new(
        "test://app",
        serde_json::json!({ "title": "My App", "count": 7 }),
    );

    // 2. Hibernate with a passphrase.
    let sealed = blob.hibernate("correct-horse-battery-staple").unwrap();

    // 3. Restore with the same passphrase.
    let restored = Blob::restore(&sealed, "correct-horse-battery-staple").unwrap();

    // 4. App data is identical.
    assert_eq!(restored.payload, blob.payload);
    assert_eq!(restored.schema, blob.schema);

    // 5. Render output matches what we'd get from the original.
    let original_html = HtmlRenderer::render(&blob);
    let restored_html = HtmlRenderer::render(&restored);
    assert_eq!(original_html, restored_html);
    assert!(restored_html.contains("<title>My App</title>"));
    assert!(restored_html.contains("<p>"));
}
