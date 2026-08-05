# RFC-0001: Webshell Lifecycle

**Status:** Draft
**Author:** Stateless Engineering
**Created:** 2026-08-05

## Abstract

This RFC defines the lifecycle contract for a **webshell** — a peer in the
stateless mesh that holds a state blob and invokes stateless services. It
covers creation, hibernation, restoration, and the progressive rendering
pipeline that makes a tab useful in <16ms.

## Motivation

A webshell is the primary consumer of the stateless architecture. It needs a
well-defined lifecycle so that:

1. A user can close a tab and reopen it with zero data loss.
2. Background tabs consume zero CPU and minimal memory.
3. Restoration is progressive — the user sees content before it's interactive.
4. Multiple webshells can synchronize state over the mesh.

## Specification

### Lifecycle States

```
Active → Freezing → Hibernated → Restoring → Active
```

Defined in `spec/lifecycle.fsm` and enforced by `reference/src/lifecycle.rs`.

### Hibernation

When a webshell becomes inactive (background tab, minimized window):

1. **Freeze:** Stop all timers, cancel pending network requests, snapshot JS heap.
2. **Serialize:** Encode the full state to a `StateBlob` (see `spec/state-blob.schema.json`).
3. **Seal:** Optionally encrypt the blob with AES-256-GCM.
4. **Store:** Write to local blob store (disk) or replicate to mesh peers.
5. **Evict:** Release all memory — renderer, JS runtime, connections.

### Restoration

When the webshell becomes active again:

1. **Load:** Fetch the blob from store, verify hash, decrypt if sealed.
2. **Shell (< 16ms):** Paint static capture (bitmap or DOM skeleton).
3. **Structure (< 100ms):** Rebuild DOM tree, compute styles, layout.
4. **Interactivity (< 300ms):** Attach event handlers, restore form state.
5. **Full State (< 1s):** Reconnect network, fetch incremental updates.

Defined in `spec/progressive-restore.md`.

### Mesh Synchronization

Webshells can replicate their blob to peers via CRDT operations
(`spec/service-bus.proto`, `Operation` message). A hibernated webshell's blob
survives on its peers — resurrection can come from any connected node.

## Reference Implementation

`reference/rust/src/` provides a minimal but executable implementation:

- `lifecycle.rs` — state machine transitions
- `state_blob.rs` — serialization, hashing, validation
- `progressive.rs` — four-stage restoration
- `mesh.rs` — in-memory mock for testing

## Open Questions

1. **Engine base:** What JS/WASM runtime executes services? (SpiderMonkey? V8? Custom?)
2. **Heap serialization:** Record-replay vs full snapshot? (Research gate — see `stateless-platform`.)
3. **Blob size limits:** 1 MiB soft cap? Larger for media?
4. **Cross-origin blobs:** Can an origin read another origin's hibernated state? (Security boundary question.)
