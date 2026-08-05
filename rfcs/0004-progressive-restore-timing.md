# RFC-0004: Progressive Restoration Timing & Guarantees

**Status:** Draft
**Author:** Stateless Engineering
**Created:** 2026-08-05

## Abstract

This document analyzes the timing requirements for progressive tab restoration
and the technical mechanisms that make each pass achievable. It covers OS
compositor behavior, serialization format performance, and rendering engine
scheduling.

## 1. Timing Budgets

| Pass | Target | Must Have | User Perceives |
|------|--------|-----------|----------------|
| 0 — Shell | < 16 ms | Static screenshot painted | "My tab is back" |
| 1 — Structure | < 50 ms | Real text + layout | "I can read it" |
| 2 — Interactivity | < 200 ms | Event handlers, forms | "I can click/type" |
| 3 — Full State | < 1 s | JS re-animation, network | "It's alive" |

These targets are **perceptual thresholds**, not averages. A user notices
jank above 100ms; they notice delay above 300ms. The 16ms target for Pass 0
is the frame budget — one vsync at 60Hz.

## 2. Compositor Latency Research

### 2.1 How Fast Can an OS Paint a Cached Image?

| OS | Compositor | Image-to-Screen | Notes |
|----|-----------|-----------------|-------|
| macOS | Quartz Compositor | 8–16 ms | Core Animation layers are GPU-cached; presenting a CALayer is one compositor pass |
| Windows | DWM (Desktop Window Manager) | 10–16 ms | DWM composes from DXGI surfaces; a pre-rendered surface is one Present() call |
| Linux (Wayland) | Mutter/KWin/Sway | 8–24 ms | Depends on compositor; direct scanout can achieve 1 frame |
| Linux (X11) | X.Org + compositor | 16–33 ms | Extra copy through X server; indirect rendering adds latency |

**Key insight:** All modern compositors can present a pre-rendered surface
within one frame (16.67ms at 60Hz). The screenshot for Pass 0 should be
stored as a GPU texture (or a compressed bitmap that can be uploaded to
the GPU in < 1ms).

### 2.2 Achieving < 16ms for Pass 0

1. **Pre-render the screenshot** at hibernation time (not restore time).
2. **Store as GPU texture** (DXGI surface on Windows, IOSurface on macOS,
   dmabuf on Linux) — zero upload cost on restore.
3. **Compositor presents the texture** directly — no layout, no paint, no JS.
4. **Fallback:** If GPU texture unavailable, store as JPEG/WebP (decodes in
   2–5 ms on modern hardware).

### 2.3 Compositor-Only Animations

The cheapest visual update is a **compositor-only** transform: opacity,
scale, translate. These run on the GPU without re-rendering. Pass 0 can
use a crossfade from screenshot to real content — the screenshot fades
out while the real page fades in, both via compositor.

## 3. Serialization Format Performance

### 3.1 Comparison for Layout Tree Restoration

| Format | Serialize | Deserialize | Size | Zero-copy | Schema |
|--------|-----------|-------------|------|-----------|--------|
| JSON | Slow | Slow | Large | No | None |
| Protobuf | Fast | Fast | Small | No | Required |
| FlatBuffers | Very fast | Very fast (zero-copy) | Large | Yes | Required |
| Cap'n Proto | Very fast | Very fast (zero-copy) | Medium | Yes | Required |
| CBOR | Medium | Medium | Medium | No | Optional |

### 3.2 Recommendation by Pass

| Pass | Format | Rationale |
|------|--------|-----------|
| 0 — Shell | GPU texture / JPEG | Not serialized — pre-rendered |
| 1 — Structure | FlatBuffers or Cap'n Proto | Zero-copy deserialization; layout tree is ready immediately |
| 2 — Interactivity | JSON (structured clone) | JS values need native types; structured clone is the browser's native format |
| 3 — Full State | JSON + binary blobs | Mixed: JSON for state, binary for media/CRDT ops |

### 3.3 Why FlatBuffers/Cap'n Proto for Pass 1?

A layout tree has many nodes (thousands). Deserializing JSON requires parsing
+ allocation per node. FlatBuffers and Cap'n Proto store data in a flat
binary buffer with pointers — deserialization is **pointer validation only**,
no parsing, no allocation. A 10,000-node layout tree deserializes in < 1ms.

**Trade-off:** FlatBuffers/Cap'n Proto require a schema and code generation.
JSON is flexible but 10–100x slower to parse.

## 4. Rendering Engine Scheduling

### 4.1 React Concurrent Mode

React 18+ concurrent rendering provides:

- **Time slicing:** Work is broken into 5ms chunks, yielding to the browser
  between chunks. This prevents jank during large renders.
- **Priority scheduling:** User interactions (clicks, typing) interrupt
  rendering. Pass 2 (interactivity) can preempt Pass 3 (full state).
- **Suspense boundaries:** Each restoration pass can be a Suspense boundary.
  The shell renders immediately; structure, interactivity, and full state
  stream in as data becomes available.

### 4.2 Restoration as a Concurrent Render

```jsx
function RestoredTab({ blob }) {
  return (
    <Suspense fallback={<ShellScreenshot blob={blob} />}>
      <Pass0Shell blob={blob} />     {/* < 16ms — static image */}
      <Suspense fallback={<ShellShell />}>
        <Pass1Structure blob={blob} /> {/* < 50ms — layout tree */}
        <Suspense fallback={<StructureShell />}>
          <Pass2Interactive blob={blob} /> {/* < 200ms — event handlers */}
          <Suspense fallback={<InteractiveShell />}>
            <Pass3FullState blob={blob} /> {/* < 1s — JS re-animation */}
          </Suspense>
        </Suspense>
      </Suspense>
    </Suspense>
  );
}
```

Each Suspense boundary yields to the event loop, ensuring the browser stays
responsive. The scheduler prioritizes Pass 0 → 1 → 2 → 3 in that order.

### 4.3 Web Worker Offloading

Pass 1 (structure deserialization) and Pass 3 (JS re-execution) can run in
a Web Worker to avoid blocking the main thread:

```
Main Thread:    Pass 0 (compositor) → Pass 2 (event handlers)
Web Worker:     Pass 1 (layout tree) → Pass 3 (JS re-execution)
```

The worker posts the layout tree to the main thread via transferable
(SharedArrayBuffer) for zero-copy handoff.

## 5. Detailed Pass Breakdown

### Pass 0 — Shell (< 16ms)

```
Hibernation:
  1. Capture compositor frame as GPU texture
  2. Store texture handle in blob.seal.shell_texture

Restore:
  1. Compositor presents texture (1 frame, ~8ms)
  2. Crossfade begins (compositor-only animation)
```

**Guarantee:** The user sees their tab within one frame. No network, no
parsing, no JS.

### Pass 1 — Structure (< 50ms)

```
Restore:
  1. Read layout tree from blob (FlatBuffers, zero-copy) — 1ms
  2. Build DOM tree from layout — 10ms
  3. Apply computed styles — 5ms
  4. Layout (reflow) — 15ms
  5. Paint — 10ms
  6. Composite — 5ms
  Total: ~46ms
```

**Guarantee:** The user sees real text and layout. They can read the page.

### Pass 2 — Interactivity (< 200ms)

```
Restore:
  1. Hydrate event handlers — 20ms
  2. Restore form state (scroll, input values) — 10ms
  3. Resume timers — 5ms
  4. Re-establish WebSocket/EventSource — 50ms (network)
  5. First interactive paint — 10ms
  Total: ~95ms (without network), ~195ms (with)
```

**Guarantee:** The user can click, type, scroll. The page responds.

### Pass 3 — Full State (< 1s)

```
Restore:
  1. Re-execute JS (module evaluation) — 100ms
  2. Re-fetch dynamic data — 200ms (network)
  3. Re-animate (CSS transitions, requestAnimationFrame) — 50ms
  4. Spawn Web Workers — 50ms
  5. Subscribe to real-time updates — 100ms
  Total: ~500ms
```

**Guarantee:** The page is indistinguishable from a live page.

## 6. Failure Modes

| Failure | Behavior | User Impact |
|---------|----------|-------------|
| Screenshot corrupt | Fall back to solid color + spinner | Slightly degraded, still fast |
| Layout tree corrupt | Reload from network | 1–3s delay, no data loss |
| JS re-execution fails | Page is interactive but static | Degraded but usable |
| Network unavailable | Page works offline (blob is source of truth) | Full functionality for cached content |

## 7. Relationship to the Spec

The timing requirements in this RFC are encoded in
`spec/progressive-restore.md`. The reference implementation's
`progressive.rs` module models these stages. The actual browser
implementation (in `stateless-platform`) must meet these targets.
