# RFC-0002: State Blob Design Proposal

**Status:** Draft
**Author:** Stateless Engineering
**Created:** 2026-08-05
**Supersedes:** RFC-0001 (webshell lifecycle)

## Abstract

This document presents the formal design for the **State Blob** — the
serialized representation of an entire system's state. It synthesizes prior
art from Solid Pods, Redux/Elm architecture, the Web App Manifest, JSON-LD,
and the structured clone algorithm into a unified structure with versioning,
schema evolution, and encryption compartments.

## 1. Prior Art Analysis

### 1.1 Solid Pods

Solid (Social Linked Data) stores user data in **Pods** — decentralized,
RDF-based data stores. Key insights:

- **RDF triples** (subject-predicate-object) as the universal data model
- **Content-addressed** via URIs — every resource has a unique identifier
- **Shape validation** via SHACL (Shapes Constraint Language) and ShEx
- **Access control** at the resource level via WAC (Web Access Control)
- **Linked data** — pods reference each other via URIs, forming a graph

**What we adopt:** Content-addressing, schema validation, resource-level
access control, URI-based identification.

**What we diverge from:** RDF's triple-based model is too granular for a
state blob. We need a tree-structured document, not a flat triple store.
RDF's verbosity (every property is a separate triple) bloats serialization.

### 1.2 Redux / Elm Architecture

Redux (and its inspiration, the Elm Architecture) pioneered serializable
application state:

- **Single state tree** — the entire app state is one immutable value
- **Pure reducers** — `(state, action) -> state` is a pure function
- **Time-travel debugging** — every state transition is recorded and replayable
- **Serializable actions** — every state change is a plain object

**What we adopt:** Single state tree, pure transitions, action journal for
time-travel debugging, serializable operations.

**What we diverge from:** Redux actions are application-specific. We need a
generic operation envelope (`insert`, `delete`, `update`, `move`) that works
across all domains.

### 1.3 Web App Manifest

The Web App Manifest provides a declarative descriptor for installable apps:

```json
{
  "name": "My App",
  "short_name": "App",
  "start_url": "/",
  "display": "standalone",
  "icons": [...]
}
```

**What we adopt:** Declarative metadata envelope, icon/theme resources,
capability declarations.

**What we diverge from:** The manifest describes an app's *chrome*, not its
*state*. We need a structure that carries both metadata and payload.

### 1.4 JSON-LD / Schema.org

JSON-LD (JSON for Linked Data) adds semantic context to JSON:

```json
{
  "@context": "https://schema.org",
  "@type": "Person",
  "name": "Alice",
  "knows": { "@type": "Person", "name": "Bob" }
}
```

**What we adopt:** Semantic context via `$schema`, typed data, linked
references between blobs, schema.org vocabulary for common types.

**What we diverge from:** JSON-LD's `@context` resolution requires network
access. Our schema URIs must be resolvable offline (bundled schemas).

### 1.5 Structured Clone Algorithm

The structured clone algorithm is the browser's native deep-copy mechanism:

- **Preserves:** Date, Map, Set, ArrayBuffer, TypedArrays, RegExp, Blob, File,
  ImageBitmap, DOM exceptions
- **Rejects:** Functions, DOM nodes, prototype chains, getters/setters,
  property descriptors
- **Handles:** Circular references, shared references within a single clone

**What we adopt:** As the baseline serialization format for JS environments.
Its type system defines what can live in a blob.

**Limitation:** Functions and closures cannot be serialized. This is a
fundamental constraint — a blob captures *state*, not *behavior*. Services
provide behavior.

### 1.6 seL4 Capability System

seL4 is a formally-verified capability-based microkernel. Key concepts:

- **Capabilities** are unforgeable tokens of authority
- **All operations** require capability invocation
- **No ambient authority** — you can only do what your capabilities allow
- **Capabilities can be delegated** (passed between threads)

**What we adopt:** Capability tokens for service invocation, delegation,
revocation. This informs the Service Bus security model (RFC-0003).

## 2. State Blob Structure

### 2.1 Envelope

```json
{
  "$schema": "https://stateless-architecture.org/schemas/state-blob/v0.1",
  "id": "sha256:abc123...",
  "version": "0.1.0",
  "timestamp": "2026-08-05T20:00:00Z",
  "application": "com.example.chat",
  "parent": "sha256:def456...",
  "seal": {
    "algorithm": "AES-256-GCM",
    "ciphertext": "...",
    "nonce": "..."
  },
  "payload": { ... }
}
```

| Field | Required | Purpose |
|-------|----------|---------|
| `$schema` | Yes | Schema URI for validation |
| `id` | Yes | Content hash (SHA-256, hex) — the blob's identity |
| `version` | Yes | Format version (semver) |
| `timestamp` | Yes | Creation time (ISO 8601) |
| `application` | Yes | Reverse-domain app identifier |
| `parent` | No | Hash of previous blob (version DAG) |
| `seal` | No | Encryption envelope (absent = plaintext) |
| `payload` | Yes | Application-specific state |

### 2.2 Content Addressing

The blob's `id` is `SHA-256(canonical_json(envelope_without_id))`. This means:

- A blob's identity is its content — identical content → identical id
- Tampering is detectable — any change produces a different hash
- Deduplication is automatic — identical blobs collapse to one
- Verification is local — no CA, no trust anchor needed

**Canonicalization:** Keys sorted lexicographically, no whitespace,
UTF-8 encoding, no trailing zeros in numbers.

### 2.3 Schema Evolution Rules

1. **Forward-only:** New fields may be added; existing fields may not be
   removed or renamed.
2. **No-skip:** Migrations must be applied sequentially (v0.1 → v0.2 → v0.3,
   never v0.1 → v0.3 directly).
3. **Idempotent:** Applying the same migration twice produces the same result.
4. **Lossless:** Migration must preserve all data (unknown fields pass through).
5. **Default values:** New fields with defaults are assumed absent in old
   blobs.

### 2.4 Encryption Compartments

A sealed blob encrypts its `payload`. But not everything needs the same
protection level. We support **compartments**:

```json
{
  "seal": {
    "algorithm": "AES-256-GCM",
    "ciphertext": "...",
    "nonce": "...",
    "compartments": [
      { "path": "/credentials", "algorithm": "AES-256-GCM" },
      { "path": "/preferences", "algorithm": "none" }
    ]
  }
}
```

| Compartment | Algorithm | Use case |
|-------------|-----------|----------|
| `none` | None | Public metadata, shareable state |
| `AES-256-GCM` | Symmetric | User data at rest |
| `ECDH-AES-256-GCM` | Asymmetric | Data shared between specific peers |
| `HMAC-SHA256` | Integrity-only | Tamper detection without secrecy |

**Key hierarchy:**
- **Master key** → encrypts the seal envelope
- **Compartment keys** → derived from master via HKDF, one per compartment
- **Peer keys** → derived from ECDH shared secret, for shared blobs

### 2.5 Blob Hierarchy

Large blobs are split into a tree:

```
Root Blob (metadata + pointers)
├── Subsystem Blob: /ui (DOM tree, scroll positions)
├── Subsystem Blob: /data (records, collections)
├── Subsystem Blob: /credentials (tokens, keys)
└── Leaf Blob: /attachments/{id} (binary data, media)
```

Each subsystem blob is content-addressed independently. The root blob's
payload contains pointers (hashes) to subsystem blobs. This enables:
- **Partial restore** — only load what's needed now
- **Sharing** — share `/data` without exposing `/credentials`
- **Dedup** — identical attachments collapse across apps

## 3. Example: Minimal Chat Application

```json
{
  "$schema": "https://stateless-architecture.org/schemas/state-blob/v0.1",
  "id": "sha256:a1b2c3...",
  "version": "0.1.0",
  "timestamp": "2026-08-05T20:00:00Z",
  "application": "org.stateless.chat",
  "payload": {
    "user": {
      "id": "user-alice",
      "displayName": "Alice",
      "avatar": "sha256:img-hash..."
    },
    "conversations": [
      {
        "id": "conv-1",
        "participants": ["user-alice", "user-bob"],
        "messages": [
          {
            "id": "msg-1",
            "author": "user-alice",
            "timestamp": "2026-08-05T19:58:00Z",
            "body": "Hello!",
            "crdt": {
              "lamport": 1,
              "origin": "user-alice"
            }
          }
        ],
        "lastRead": { "user-alice": "msg-1", "user-bob": null }
      }
    ],
    "drafts": {
      "conv-1": { "body": "Hi there", "cursorPos": 8 }
    },
    "preferences": {
      "theme": "dark",
      "fontSize": 14,
      "notifications": true
    }
  },
  "seal": {
    "algorithm": "AES-256-GCM",
    "ciphertext": "...",
    "nonce": "..."
  }
}
```

## 4. Example: Complex Document Editor

```json
{
  "$schema": "https://stateless-architecture.org/schemas/state-blob/v0.1",
  "id": "sha256:d4e5f6...",
  "version": "0.1.0",
  "timestamp": "2026-08-05T20:00:00Z",
  "application": "org.stateless.editor",
  "payload": {
    "document": {
      "title": "Stateless Architecture RFC",
      "schemaVersion": "doc-v2",
      "blocks": [
        {
          "id": "blk-1",
          "type": "heading",
          "level": 1,
          "text": "Abstract",
          "annotations": []
        },
        {
          "id": "blk-2",
          "type": "paragraph",
          "text": "This document presents...",
          "annotations": [
            { "start": 0, "end": 4, "style": "bold" }
          ]
        }
      ],
      "selection": {
        "anchor": { "block": "blk-2", "offset": 5 },
        "focus": { "block": "blk-2", "offset": 12 }
      },
      "crdt": {
        "siteId": "editor-alice",
        "clock": 42,
        "operations": "sha256:op-log-hash..."
      }
    },
    "undoStack": [
      { "op": "insert", "block": "blk-1", "pos": 0 },
      { "op": "insert", "block": "blk-2", "pos": 1 }
    ],
    "collaborators": [
      { "id": "user-bob", "cursor": { "block": "blk-1", "offset": 3 },
        "color": "#4287f5" }
    ],
    "attachments": {
      "img-1": { "hash": "sha256:img-hash...", "mime": "image/png" }
    }
  }
}
```

## 5. CRDT vs Flat KV: Trade-off Analysis

### 5.1 Flat Key-Value Model

```
state = { "count": 42, "name": "Alice", "theme": "dark" }
```

**Pros:**
- Simple to implement and debug
- Small payload for small state
- Direct access: `state["count"]`
- Works with any storage backend

**Cons:**
- **No merge semantics:** Concurrent updates to different keys conflict at the
  blob level, not the key level. Last-write-wins loses data.
- **No operation history:** Can't reconstruct how state evolved.
- **Granularity mismatch:** Locking/-syncing the whole blob for one key change.
- **No offline support:** Offline edits require full-blob conflict resolution.

**Best for:** Single-user apps, small state, settings/preferences, read-heavy
workloads.

### 5.2 Nested CRDT-Aware Model

```
state = {
  "doc": Y.Doc,           // Yjs document with typed sub-types
  "text": Y.Text,         // Collaborative text
  "map": Y.Map,           // Collaborative key-value
  "array": Y.Array        // Collaborative list
}
```

**Pros:**
- **Conflict-free merges:** Concurrent edits merge without data loss
- **Operation history:** Full audit log of every change
- **Fine-grained sync:** Only transmit changed operations, not whole state
- **Offline-first:** Edit offline, merge on reconnect
- **Presence awareness:** Know what others are doing

**Cons:**
- **Complexity:** CRDTs are harder to reason about than plain JSON
- **Payload size:** Operation journals grow without bound (need GC)
- **Serialization:** CRDT state is not plain JSON — needs special encoding
- **Performance:** Merge operations have non-trivial CPU cost
- **Schema rigidity:** CRDT types (Y.Text, Y.Map) constrain data shape

**Best for:** Collaborative apps, multi-user editing, offline-first, real-time
sync.

### 5.3 Hybrid Model (Recommended)

Use **both** — flat KV for non-shared state, CRDT for shared:

```json
{
  "payload": {
    // Flat KV — single-user, no sync needed
    "preferences": { "theme": "dark", "fontSize": 14 },
    "ui": { "sidebarOpen": true, "activePanel": "editor" },

    // CRDT — shared, collaborative
    "document": {
      "$type": "crdt",
      "crdtType": "Y.Text",
      "operations": [...],
      "stateVector": {...}
    },
    "presence": {
      "$type": "crdt",
      "crdtType": "Y.Map",
      "operations": [...]
    }
  }
}
```

**Decision criteria:**

| Criterion | Flat KV | CRDT |
|-----------|---------|------|
| Single user | ✅ | Overhead |
| Multi-user, non-simultaneous | ✅ | Overhead |
| Multi-user, simultaneous | ❌ | ✅ |
| Offline edits | ❌ | ✅ |
| Small state (< 1 KB) | ✅ | Overhead |
| Large state (> 1 MB) | Granularity issues | ✅ |
| Audit trail needed | ❌ | ✅ |
| Simple debugging | ✅ | Harder |

### 5.4 Portability Between Applications

The `$schema` field enables cross-application state portability:

1. **Schema negotiation:** App B can read App A's blob by resolving its
   `$schema` URI and mapping fields.
2. **Schema.org vocabulary:** Common types (Person, Document, Event) use
   schema.org terms, enabling AI/ML consumers to understand blob semantics.
3. **Progressive enhancement:** An app that doesn't understand a field passes
   it through unchanged (forward compatibility rule 4).
4. **Linked blobs:** A blob can reference another by hash, forming a DAG of
   shared data (like Solid's linked data).

## 6. Design Decisions & Rationale

### 6.1 Why JSON, not MessagePack/CBOR?

**JSON** for the envelope (human-readable, universally parseable),
**MessagePack/CBOR** for binary payloads (attachments, CRDT operation logs).
The envelope is small; its readability aids debugging. Binary payloads are
large; their compactness matters.

### 6.2 Why SHA-256, not Blake3?

SHA-256 is universally available (WebCrypto, OpenSSL, ring). Blake3 is
faster but less widely supported. For content-addressing, speed matters less
than ubiquity.

### 6.3 Why AES-256-GCM, not ChaCha20-Poly1305?

Both are AEAD ciphers with equivalent security. AES-256-GCM has hardware
acceleration on most platforms (AES-NI). ChaCha20 is faster in software-only
environments (mobile, WASM). We support both; default to AES-256-GCM.

### 6.4 Why content-addressing over UUIDs?

Content hashes are **self-verifying**: the hash proves the content. UUIDs
require a central registry or random generation. Content hashes enable
deduplication and verification without trust.

### 6.5 Why a tree, not a flat blob?

A tree enables **progressive loading** (shell first, data later), **partial
sharing** (share one subsystem without others), and **dedup** (identical
subsystems collapse). The cost is one level of indirection.

## 7. Open Questions

1. **CRDT encoding:** Should CRDT operations be embedded as JSON arrays or
   use a binary encoding (like Yjs's lib0)?
2. **Schema distribution:** Should schemas be bundled with the app or
   fetched from a URI (requiring network)?
3. **Blob size limits:** What's the practical maximum blob size before
   splitting is required?
4. **Migration tooling:** Do we need a migration DSL, or are hand-written
   migration functions sufficient?
