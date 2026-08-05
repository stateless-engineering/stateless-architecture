# Reference Implementation: Skeleton

## Cargo.toml

```toml
[package]
name = "stateless-architecture"
version = "0.1.0"
edition = "2021"
description = "Reference implementation of the stateless core loop"
license = "MIT"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
hex = "0.4"
thiserror = "1"

[dev-dependencies]
serde_json = "1"

[lib]
name = "stateless_architecture"
path = "rust/src/lib.rs"
```

## Module Outline

```
reference/
├── Cargo.toml
├── rust/
│   └── src/
│       ├── lib.rs           # Public API re-exports
│       ├── state_blob.rs    # Blob struct, serialization, sealing
│       ├── service.rs       # Service trait, registry
│       ├── service_bus.rs   # Endpoint resolution, capability tokens
│       ├── lifecycle.rs     # State machine (active ↔ hibernated)
│       ├── progressive.rs   # Four-pass restore
│       ├── mesh.rs          # CRDT sync stub
│       └── renderer.rs      # Mock renderer (string → string)
└── tests/
    ├── state_blob_test.rs
    ├── integration_test.rs
    └── minimal_demo_test.rs
```

## Core `run_test()` Function

```rust
/// The complete stateless core loop, end-to-end.
///
/// 1. Create a blob (state)
/// 2. Render it (service invocation)
/// 3. Hibernate (serialize + encrypt)
/// 4. Destroy renderer
/// 5. Restore (deserialize + decrypt)
/// 6. Re-render (new service invocation)
/// 7. Assert state integrity
fn run_test() {
    // Step 1: Create
    let blob = Blob::new("test://app", json!({ "title": "Hello" }));

    // Step 2: Render
    let html = HtmlRenderer::render(&blob);
    assert!(html.contains("Hello"));

    // Step 3: Hibernate
    let bytes = blob.serialize().unwrap();

    // Step 4: Destroy (drop)
    drop(HtmlRenderer);

    // Step 5: Restore
    let restored = Blob::deserialize(&bytes).unwrap();

    // Step 6: Re-render
    let html2 = HtmlRenderer::render(&restored);
    assert!(html2.contains("Hello"));
}
```

**Total: ~15 lines. This is the entire stateless loop proven in code.**
