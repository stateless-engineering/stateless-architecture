//! Minimal Stateless Architecture demo.
//!
//! The smallest possible code that proves the core loop works:
//! 1. Create a blob
//! 2. Render it (function that returns a string representation of UI)
//! 3. Hibernate (serialize + encrypt)
//! 4. Destroy the renderer
//! 5. Restore (deserialize + decrypt)
//! 6. Attach a new renderer, render again

use stateless_architecture::{Blob, CapabilityToken, Service, ServiceBus, ServiceError};

/// A minimal renderer: a function that renders a blob's UI section to a string.
struct HtmlRenderer;

impl HtmlRenderer {
    fn render(blob: &Blob) -> String {
        let title = blob
            .payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let body = blob
            .payload
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("(empty)");
        format!(
            "<html><head><title>{}</title></head><body>{}</body></html>",
            title, body
        )
    }
}

/// A service that updates the document title.
struct TitleService;

impl Service for TitleService {
    fn name(&self) -> &str {
        "title-updater"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn invoke(
        &self,
        blob: &Blob,
        input: &serde_json::Value,
    ) -> Result<(Blob, serde_json::Value), ServiceError> {
        let mut new_payload = blob.payload.clone();
        if let Some(title) = input.get("title").and_then(|v| v.as_str()) {
            if let Some(obj) = new_payload.as_object_mut() {
                obj.insert("title".into(), serde_json::json!(title));
            }
        }
        let new_blob = blob
            .child(new_payload)
            .map_err(|e| ServiceError::Invocation {
                name: "title-updater".into(),
                reason: e.to_string(),
            })?;
        Ok((new_blob, serde_json::json!({ "updated": true })))
    }
}

#[test]
fn minimal_core_loop() {
    // 1. Create a blob.
    let blob = Blob::new(
        "demo://minimal",
        serde_json::json!({ "title": "Hello", "body": "World" }),
    );

    // 2. Render it.
    let html1 = HtmlRenderer::render(&blob);
    assert!(html1.contains("Hello"));
    assert!(html1.contains("World"));

    // 3. Transform it through a service (simulating user interaction).
    let mut bus = ServiceBus::new();
    bus.register("title-updater", Box::new(TitleService));
    let token = CapabilityToken::issue(
        "user:alice",
        "service:title-updater",
        vec!["invoke".into()],
        None,
        None,
    );

    let (transformed, _) = bus
        .invoke(
            "title-updater",
            &blob,
            &serde_json::json!({ "title": "Goodbye" }),
            &token,
        )
        .unwrap();
    assert_eq!(transformed.payload["title"], "Goodbye");

    // 4. Hibernate (serialize to buffer).
    let bytes = transformed.serialize().unwrap();

    // 5. Destroy the renderer (drop).
    drop(HtmlRenderer);

    // 6. Restore (deserialize from buffer).
    let restored = Blob::deserialize(&bytes).unwrap();

    // 7. Attach a new renderer and render again.
    let html2 = HtmlRenderer::render(&restored);
    assert!(html2.contains("Goodbye"));
    assert!(html2.contains("World"));
}
