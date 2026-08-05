# Chapter 6 — Mesh Sync: Peer-to-Peer State Synchronization

> **Every client is a server. State is truth; the mesh is a pure function of (blob, operations) → (new blob, ops to broadcast).**

---

## 6.1 Every Client Is a Server

The mesh removes the single-engine assumption: every peer holds its own blob and runs its own engine — no leader, no coordinator. Each peer is simultaneously client and server: it accepts inputs, transforms its blob, and broadcasts the resulting operations.

The webshell pattern (Chapter 4) scales naturally: a webshell is a service that is also a peer.

## 6.2 State Sync via CRDT Operations

Shipping whole blobs on every mutation is unbounded. Peers synchronize via **operations**: small descriptions of state change.

A Conflict-free Replicated Data Type (CRDT) has operations that commute — any order yields the same result. Peers apply locally, broadcast asynchronously: no locking, no consensus. **The blob is the fold of all operations.**
```rust
type PeerId = String;
type Lamport = (u64, PeerId); // (counter, origin) — total order

enum CrdtOp {
    Set { target: String, value: serde_json::Value },
    Delete { target: String },
    Increment { target: String, delta: i64 },
}

fn create_op(kind: CrdtOp, origin: &PeerId, clock: &mut u64) -> (Lamport, CrdtOp) {
    *clock += 1;
    ((*clock - 1, origin.clone()), kind)
}

fn broadcast(peers: &[PeerId], op: &(Lamport, CrdtOp), relay: &Relay) {
    for peer in peers {
        if peer.is_reachable() { peer.send(op); } else { relay.forward(peer, op); }
    }
}
```

## 6.3 The Operation Envelope

```json
{
  "op": "set | delete | increment | append",
  "target": "path within the blob",
  "value": "the new value, or null for delete",
  "clock": { "counter": 14, "origin": "peer-A" },
  "origin": "peer-A"
}
```

`op` is the kind; `target` a JSON Pointer into the blob; `value` the payload; `clock` a Lamport `(counter, origin)` pair forming a total order; `origin` the creating peer — attribution, not authority.

## 6.4 The Mesh Topology

```mermaid
flowchart LR
    P1[Peer A] <-->|ops| P2[Peer B]
    P1 <-->|ops| P3[Peer C]
    P2 <-->|ops| P3
    P1 -->|ops| R[Relay Node]
    R -->|ops| P4[Peer D\n(directly unreachable)]
```

Every peer broadcasts to every peer it can reach. Peers that cannot connect directly route through a **relay node**, which forwards operations statelessly.

## 6.5 Pseudocode: CRDT Sync Engine

```pseudocode
function apply_operation(blob, op):
    # MUST be deterministic: same blob + same op → same result.
    case op.op:
        "set"       → blob[op.target] = op.value
        "delete"    → delete blob[op.target]
        "increment" → blob[op.target] = (blob[op.target] or 0) + op.value
        "append"    → blob[op.target].append(op.value)
    return blob

function merge(blob, ops):
    sorted = sort_by_clock_then_origin(ops)  # deterministic
    for op in sorted:
        blob = apply_operation(blob, op)
    return blob

function broadcast(op, peers):
    for peer in peers:
        if peer.is_reachable():
            peer.send(op)
        else:
            relay.forward(op, peer)
```

The engine holds no state beyond the blob and the journal. Operations are ordered by clock, ties broken by origin.

## 6.6 Conflict-Free Convergence

The CRDT guarantee: **all peers that received the same operation set compute the same blob**, regardless of delivery order, duplication, or delay. Operations are designed so no two can conflict: `set` overwrites, `increment` commutes, `append` is idempotent. Convergence is a property of the data type, not a runtime negotiation.

## 6.7 Relay Nodes

NATs, firewalls, and offline peers create partitions. **Relay nodes** forward operations without originating or interpreting them; they store nothing, and the mesh routes around a failed relay. A partitioned peer catches up on reconnect: request all operations since its last clock, apply, converge.

## 6.8 Hibernation in a Mesh

A hibernating peer (Chapter 5) leaves the mesh; its blob persists and its engine is destroyed. On resurrection it locates the blob by content hash (Chapter 3), fetches the operation journal from any peer, replays missed operations (Chapter 2), and rejoins. Catch-up is journal replay — no full download.

## 6.9 The Operation Journal as Audit Log

The journal is the complete, ordered log of every applied operation. It is **state reconstruction** — any peer rebuilds the blob at any point by replaying the journal (deterministic replay, Chapter 2) — and an **audit log**: each operation carries `origin` and `clock`, an immutable, attributed record of who changed what, when. Append-only and content-addressable, it can be hashed and compared across peers to detect tampering.

## 6.10 How This Chapter Relates to the Rest of the Book

- **Chapter 3 — State Blob**: the blob is the unit of synchronization; content-addressing lets peers verify convergence by comparing hashes.
- **Chapter 2 — Core Principles**: catching up is deterministic replay of missed operations from the journal.
- **Chapter 5 — Progressive Restore**: a hibernating peer is a partitioned peer; resurrection is catch-up via the journal.

The mesh inherits every property of the single-peer model: stateless, deterministic, sealed, evictable.

---

*The mesh is the stateless model with the server boundary removed: every peer is a service, every operation a pure function, the blob the only thing that matters.*
