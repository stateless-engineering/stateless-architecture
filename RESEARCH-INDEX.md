# Research & Design Output Index

This document indexes all research prompts and their outputs in the
stateless-architecture repository.

## Prompt 1: State Blob Semantics

**Question:** How do we define a precise, portable state blob? What prior art
exists for serializable application state with schema evolution?

**Sources:**
- [Solid Pods](https://solidproject.org) — RDF triples, content-addressed URIs, SHACL validation
- [Redux](https://redux.js.org) — Single state tree, serializable actions, time-travel debugging
- [Web App Manifest](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest) — Declarative metadata envelope
- [JSON-LD](https://json-ld.org) — Semantic context, schema.org vocabulary
- [Structured Clone Algorithm](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Structured_clone_algorithm) — Browser serialization constraints

**Outputs:**
- `rfcs/0002-state-blob-design.md` — Full design proposal
- `spec/state-blob.schema.json` — JSON Schema (draft-2020-12)
- `book/03-state-blob.md` — Chapter with prior art section

## Prompt 2: Service Bus & Capability Protocol

**Question:** Design a minimal protocol connecting a webshell to stateless
services with async request/response, streaming, capability tokens, and
multi-endpoint support.

**Sources:**
- [seL4 Microkernel IPC](https://docs.sel4.systems/Tutorials/ipc.html) — Capability-based access, synchronous message passing
- [Cloudflare Workers RPC](https://developers.cloudflare.com/workers/runtime-apis/rpc/) — JS-native calling, structured clone
- [WIT / WASI Preview 2](https://component-model.bytecodealliance.org/design/wit.html) — Cross-language interface types
- [gRPC vs Cap'n Proto](https://github.com/LesnyRumcajs/grpc_bench) — Performance benchmarks

**Outputs:**
- `rfcs/0003-service-bus-protocol.md` — Protocol design
- `spec/service-bus.proto` — Protobuf 3 definition
- `reference/rust/src/bus.rs` — ServiceBus implementation
- `book/04-service-bus.md` — Chapter with prior art section

## Prompt 3: Progressive Restoration Timing

**Question:** How do we achieve <16ms shell, <50ms structure, <200ms
interactivity, <1s full state?

**Sources:**
- [OS Compositor Comparison](https://www.lexo.ch/blog/2025/08/comparison-of-modern-display-systems) — 8–16ms presentation
- [React Concurrent Mode](https://react.dev/reference/react/Suspense) — Time slicing, selective hydration
- [Game Level Streaming](https://dev.epicgames.com/documentation/unreal-engine/level-streaming) — Async activation
- [Serialization Benchmarks](https://github.com/kcchu/buffer-benchmarks) — FlatBuffers zero-copy

**Outputs:**
- `rfcs/0004-progressive-restore-timing.md` — Timing analysis
- `spec/progressive-restore.md` — Engine specification
- `reference/tests/minimal_demo_test.rs` — Minimal core loop demo

## Prompt 4: Minimal Reference Implementation

**Question:** What's the smallest amount of code that demonstrates the core loop?

**Output:** `reference/tests/minimal_demo_test.rs` — 7-step demo:
1. Create blob → 2. Render → 3. Service transform → 4. Hibernate (serialize) →
5. Destroy renderer → 6. Restore (deserialize) → 7. Re-render

**Total: 33 lines of test code proving the entire loop.**
