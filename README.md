# Slint Node Editor Library

A flexible, **generic** Slint component library for building visual graph editors. Supports data flow diagrams, state machines, shader graphs, and any visual node-based interface.

https://github.com/user-attachments/assets/f0e8d69c-19da-4acf-b3e1-ea7e6c1324d8

## Key Features

- ✅ **Generic Design** - Works with any node and link data structures
- ✅ **Trait-Based Architecture** - Zero coupling via `NodeGeometry` and `LinkModel` traits
- ✅ **Callback-Based Computation** - Delegates expensive operations to Rust for performance
- ✅ **Opaque Pin IDs** - Library never prescribes pin encoding; applications choose
- ✅ **Extensible** - Easy to customize pins, styling, node types, and behaviors
- ✅ **Zero Dependencies** - Library has no external Rust dependencies beyond Slint

## Architecture Overview

```
┌─────────────────────────────────────┐
│  Your Application (any node types)  │
│  - Custom node data structures      │
│  - Callback implementations         │
│  - UI composition                   │
└──────────────┬──────────────────────┘
               │ depends on
               │
┌──────────────▼──────────────────────┐
│  Slint Node Editor Library          │
│  - NodeEditor (main component)       │
│  - BaseNode, Pin, Link, Minimap      │
│  - Geometry traits for hit-testing   │
│  - Selection & cache management      │
│  - Grid and link path generation    │
└──────────────┬──────────────────────┘
               │ depends on
               │
┌──────────────▼──────────────────────┐
│  Slint Framework                    │
└─────────────────────────────────────┘
```

**Key Principle:** The library depends *downward* only (on Slint), never upward on application code. This ensures the library remains generic and reusable.

## Quick Start

### 1. Add to Your Project

```toml
# In your Cargo.toml
[dependencies]
slint = "1.18"
slint-node-editor = "0.1"

[build-dependencies]
# `experimental-module-builds` is what lets the Slint compiler resolve the
# `@nodeeditor` import below. You get it transitively today, because cargo
# unifies build-dependency features and slint-node-editor enables it — but
# declare it anyway, so your build does not depend on that.
slint-build = { version = "1.18", features = ["experimental-module-builds"] }
```

Requires Rust 1.92, which is Slint 1.18's minimum, not ours.

### 2. Import Core Components

```slint
// In your main .slint file
import { NodeEditor, BaseNode, Pin, Link, LinkData, PinTypes } from "@nodeeditor";
```

### 3. Configure build.rs

Nothing special — `@nodeeditor` resolves from the dependency's own metadata:

```rust
fn main() {
    slint_build::compile("ui/main.slint").unwrap();
}
```

The components ship as a [Slint library
module](https://github.com/slint-ui/slint/issues/154). That mechanism is
marked experimental upstream and may change; if it does, this crate needs a new
release, and pinning `slint-node-editor` to an exact version is the safe move.

Data types declared in the `.slint` sources — `LinkData`, `MinimapNode`,
`MinimapPosition`, `LinkCreationState`, `BoxSelectionModifier` — are re-exported
from the Rust crate, so reach for them there:

```rust
use slint_node_editor::LinkData;
```

### 4. Create a Simple Node

```slint
component Node inherits BaseNode {
    Text { text: "My Node"; color: white; }
    Pin {
        pin-type: PinTypes.input;
        node-id: root.node-id;
        // Pass required context for drag handling
        zoom: root.zoom;
        node-screen-x: root.screen-x;
        node-screen-y: root.screen-y;
    }
}
```

### 5. Wire Up the NodeEditor Component

```slint
export component MainWindow inherits Window {
    width: 1200px;
    height: 800px;

    in property <[LinkData]> links; // Provided by your Rust backend

    NodeEditor {
        id: editor;
        links: root.links; // Bind links model

        // Provide your node data as @children
        for node in nodes: Node {
            node-id: node.id;
            world-x: node.x * 1px; // Convert float to length
            world-y: node.y * 1px;
            
            // Pass viewport state for local coordinate calculation
            zoom: editor.zoom;
            pan-x: editor.pan-x;
            pan-y: editor.pan-y;
        }

        // Implement callbacks (see Callbacks section below)
        // Note: Use NodeEditorController in Rust to handle these easily
    }
}
```

## Core Concepts

### Pin ID Encoding (Your Choice!)

The library treats pin IDs as opaque integers. **You decide how to encode them:**

**Example 1: Dense Encoding**
```rust
// Pin ID = node_id * 10 + pin_type
// node_id=5, pin_type=1 → pin_id=51
pub fn make_pin_id(node_id: i32, pin_type: i32) -> i32 {
    node_id * 10 + pin_type
}
```

**Example 2: Sparse Encoding**
```rust
// Pin ID = node_id * 1000 + pin_type
// Allows node IDs up to ~1,000,000
pub fn make_pin_id(node_id: i32, pin_type: i32) -> i32 {
    node_id * 1000 + pin_type
}
```

The geometry lifecycle reports `pin-id`, `node-id`, and `pin-type` separately,
so the library never needs to decode an application's pin IDs.

### Coordinate Systems

The editor uses two coordinate systems:

1. **World Coordinates** - Graph space (where nodes live)
   - Property: `world-x`, `world-y` on `BaseNode`
   - Range: Unbounded (can be negative, very large)

2. **Screen Coordinates** - After pan/zoom transformation
   - Computed: `screen_x = world_x * zoom + pan_x`
   - Used for: Hit-testing, rendering, mouse interaction

The library handles all transformations transparently.

### Zoom & Pan Controls

Built-in input handling:
- **Ctrl+Scroll**: Zoom in/out centered on mouse position
- **Scroll**: Pan the viewport
- **Middle-click drag**: Pan the viewport

Zoom is automatically clamped to `min-zoom` (default 0.1) and `max-zoom` (default 3.0).

## Component Reference

### NodeEditor (Main Component)

**Properties:**
```slint
in-out property <length> pan-x;          // Pan offset (x)
in-out property <length> pan-y;          // Pan offset (y)
in-out property <float> zoom;            // Zoom factor (1.0 = 100%)
in property <float> min-zoom: 0.1;       // Minimum zoom level
in property <float> max-zoom: 3.0;       // Maximum zoom level

// Host-side convenience values: pass these to custom node delegates that
// implement LOD or minimum sizing. NodeEditor does not resize nodes itself.
in property <float> lod-full-threshold: 0.5;
in property <float> lod-simplified-threshold: 0.25;
in property <length> min-node-width: 80px;
in property <length> min-node-height: 40px;

in property <length> grid-spacing: 24px;       // Grid cell size
in property <color> grid-color: #404040;       // Grid line color
in property <brush> background-color: #1a1a1a; // Background color

in property <length> link-hover-distance: 8px; // Click tolerance for links
in property <length> pin-hit-radius: 10px;     // Hit radius for pins
in property <int> link-hit-samples: 20;        // Bezier samples for hit-testing
in property <float> bezier-min-offset: 50.0;   // Min horizontal offset for curves

// Minimap
in property <bool> minimap-enabled: false;
in property <MinimapPosition> minimap-position: bottom-right;

// Selection lives in the application — see "Callbacks (Selection)" below.
// The editor stores none of it; `selected` is per-row data on your node models
// and on LinkData.

// Geometry & Rendering
in-out property <string> grid-commands;         // SVG path generated by wire_node_editor!
in-out property <int> geometry-version <=> GeometryVersion.version;
in property <[MinimapNode]> minimap-nodes: [];  // Minimap data
in-out property <length> graph-min-x;           // Graph bounds
in-out property <length> graph-max-x;
in-out property <length> graph-min-y;
in-out property <length> graph-max-y;

// State outputs (read-only)
out property <bool> is-selecting;               // User is dragging selection box
out property <length> selection-x;              // Selection box position
out property <length> selection-y;
out property <length> selection-width;          // Selection box size
out property <length> selection-height;

out property <bool> is-creating-link;           // User is dragging to create link
out property <length> link-start-x;             // Link preview start
out property <length> link-start-y;
out property <length> link-end-x;               // Link preview end
out property <length> link-end-y;
out property <int> link-start-pin-id;           // Which pin started the link

out property <length> context-menu-x;           // Right-click position
out property <length> context-menu-y;

out property <int> hovered-link-id;             // Link under mouse
```

**Callbacks (Computation):**

Link picking remains a host callback because it needs the host's link model:

```slint
/// Compute which link is at world position (x, y)
callback compute-link-at(x: length, y: length) -> int;
```

The standard Rust computations live on the exported
`NodeEditorComputations` global. `wire_node_editor!` installs their default
implementations and synchronizes `grid-spacing` and `bezier-min-offset` with
the controller. Applications that replace a handler do so on this global; the
last installed handler wins.

```slint
NodeEditorComputations.compute-pin-at(world-x, world-y, world-radius);
NodeEditorComputations.compute-link-path(start-pin, end-pin, geometry-version);
NodeEditorComputations.compute-link-preview-path(start-x, start-y, end-x, end-y, zoom, offset);
NodeEditorComputations.request-grid-update();
```

BaseNode double-clicks are emitted through the exported public event global:

```slint
NodeEditorEvents.node-double-clicked(node-id) => { /* handle event */ }
```

**Callbacks (Selection):**

The editor keeps **no selection state**. It reports gestures; your application
decides what they mean and puts the answer in the `selected` field of your node
rows and of `LinkData`, which is what the editor renders. Those rows can be the
only record you keep — there is nothing to hold on the side.

```slint
/// A node was clicked (or dragged while unselected):
/// replace the selection with it, or toggle it when shift-held
callback node-selected(node-id: int, shift-held: bool);

/// A link was clicked — same semantics
callback select-link(link-id: int, shift-held: bool);

/// The background was clicked: drop the whole selection
callback selection-cleared();

/// A marquee was released over this world-coordinate rectangle:
/// hit-test it and select the hits, extending the selection when shift-held
callback box-selection-committed(x: length, y: length, w: length, h: length, shift-held: bool);
```

Handlers for selection intents that can affect a subsequent drag must update
the rows' `selected` flags before returning; the live multi-node preview and
drag commit read those flags. `wire_selection!` provides this behavior.
Applications that defer selection projection must also own their drag policy
and commit. Selection changes during an active drag are otherwise unspecified.

On the Rust side the `selection` module holds the policy — `resolve_click` and
`resolve_box` turn a gesture and the current set into the new set, and
`project_selection` writes a set into the rows. Every intent resolves to an
**absolute set**, never a delta, and both resolvers are order-stable.

`wire_selection!` composes them for the common case — rows with `id` and
`selected` fields — and keeps no state of its own:

```rust
let setup = NodeEditorSetup::new({
    let nodes = nodes.clone();
    move |dragged, dx, dy| GraphLogic::commit_drag(&nodes, dragged, dx, dy)
});

wire_node_editor!(window, setup);
wire_selection!(window, setup, nodes);          // nodes only
wire_selection!(window, setup, nodes, links);   // …or nodes and links
```

Applications with several node models or their own gesture semantics wire the
four callbacks by hand out of the `selection` module — see the `advanced`
example, which spans two node models sharing one id space.

Multi-node drag needs no wiring: `GraphLogic::commit_drag` moves the dragged
node plus every row the model shows as selected, which is the same data the
editor renders.

**Callbacks (Events):**

```slint
/// User completed a link (dragged from one pin to another)
callback link-requested(start-pin-id: int, end-pin-id: int);

/// The host cancelled link creation through cancel-link-creation()
callback link-cancelled();

/// User hovered over a link
callback link-hovered();

/// User right-clicked (context menu)
callback context-menu-requested();
```

**Functions (Helper API):**

```slint
/// Report node geometry through the canonical lifecycle and schedule a refresh.
function report-node-rect(id: int, x: length, y: length, w: length, h: length);

/// Report pin geometry through the canonical lifecycle and schedule a refresh.
function report-pin-position(pin-id: int, node-id: int, pin-type: int, rel-x: length, rel-y: length);

/// Start link creation from a pin
function start-link-from-pin(pin_id: int, x: length, y: length);

/// Update link end position during creation
function update-link-end(x: length, y: length);

/// Complete link creation (checks for pin at end pos)
function complete-link-creation();

/// Force re-computation of all link paths
function refresh-links();
```

### BaseNode

Base component for creating custom nodes. Provides drag handling and selection.

**Properties:**
```slint
in property <int> node-id;             // Unique node ID
in property <length> world-x;          // X position in graph space
in property <length> world-y;          // Y position in graph space
in property <bool> selected: false;    // Selection state — bind it from your model row
in property <float> zoom;              // Required for view calculation
in property <length> pan-x;            // Required for view calculation
in property <length> pan-y;            // Required for view calculation
```

### Pin

Represents a connection point on a node.

**Properties:**
```slint
in property <int> pin-id;           // Pin ID (encoded by application)
in property <int> node-id;          // Parent node ID
in property <int> pin-type;         // PinTypes.input or .output (or custom)
in property <color> base-color: #888;
in property <color> hover-color: #aaa;
in property <float> zoom;           // Required for scaling
in property <length> node-screen-x; // Required for drag handling
in property <length> node-screen-y; // Required for drag handling
```

### Link

Renders a Bezier curve between two pins. Used internally by `NodeEditor` but can be used for custom rendering.

The element is sized to the curve, not to the canvas. That is what lets a
partial renderer repaint one link's strip instead of the whole graph — so a
`LinkPath` whose box does not contain what its `commands` draw leaves stale
pixels. `Link` pads the box for the stroke itself; the box is the centreline's.

**Properties:**
```slint
in property <LinkPath> geometry;           // commands relative to x/y, plus the box
in property <color> link-color: #888;
in property <length> line-width: 2px;
in property <bool> selected: false;
in property <bool> hovered: false;
```

**`LinkPath`:**
```slint
struct LinkPath {
    commands: string,   // SVG path, relative to x/y (e.g., "M 0 0 C 50 50 100 100")
    x: length, y: length,
    width: length, height: length,
}
```

Building one in Rust from a curve — `CubicBezier::to_link_path` takes the
constructor rather than naming the type, because a consumer importing
`@nodeeditor` through cargo metadata gets the crate's `LinkPath` while one
passing a library path gets their own generated copy:

```rust
use slint_node_editor::path::CubicBezier;

let curve = CubicBezier::from_endpoints(sx, sy, ex, ey, 1.0, 50.0);
curve.to_link_path(|commands, x, y, width, height| LinkPath {
    commands: commands.into(),
    x,
    y,
    width,
    height,
})
```

## Convenience Helpers

The library provides Rust helpers to reduce boilerplate.

### NodeEditorController

The `NodeEditorController` is a high-level helper that manages geometry tracking, zoom state, and link path computation. It provides ready-to-use callback implementations.

```rust
use slint_node_editor::NodeEditorController;

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();

    // 1. Hook up computation callbacks
    window.on_compute_link_path(ctrl.compute_link_path_callback(
        |commands, x, y, width, height| LinkPath { commands: commands.into(), x, y, width, height },
    ));
    window.on_node_drag_started(ctrl.node_drag_started_callback());

    // 2. Hook up geometry tracking
    window.on_node_rect_changed({
        let ctrl = ctrl.clone();
        move |id, x, y, w, h| ctrl.handle_node_rect(id, x, y, w, h)
    });
    
    window.on_pin_position_changed({
        let ctrl = ctrl.clone();
        move |pid, nid, ptype, x, y| ctrl.handle_pin_position(pid, nid, ptype, x, y)
    });

    // 3. Handle grid updates
    window.on_request_grid_update({
        let ctrl = ctrl.clone();
        let w = window.as_weak();
        move || {
            if let Some(w) = w.upgrade() {
                w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
            }
        }
    });

    window.invoke_request_grid_update();
    window.run().unwrap();
}
```

Node and pin geometry is a projection of the application's graph model. Retire
that projection whenever the model removes an object, and reset it after
replacing the whole graph:

```rust
let lifecycle = window.global::<NodeEditorInternalCallbacks>();

// Remove connected logical links in the host model, then:
lifecycle.invoke_remove_pin(pin_id);
lifecycle.invoke_remove_node(node_id); // also retires pins owned by the node

// After installing another graph model (including one that reuses IDs):
lifecycle.invoke_reset_graph();
```

These functions also clear editor interactions that refer to retired objects.
Call the controller's `remove_pin`, `remove_node`, or `reset_graph` methods when
there is no Slint component instance. A hidden `Pin` remains available to route
existing links, but it is excluded from pin hit testing until visible again.

### GeometryTracker

For lower-level control, `GeometryTracker` simplifies just the geometry cache setup.

```rust
use slint_node_editor::GeometryTracker;

let tracker = GeometryTracker::new();
window.on_node_rect_changed(tracker.node_rect_callback());
window.on_pin_position_changed(tracker.pin_position_callback());
let cache = tracker.cache(); // Use for hit testing
```

## Examples

All examples are located in the `examples/` directory and can be run from the root using `cargo run -p <name>`:

- **minimal:** A simple example using `NodeEditorController` with basic nodes and links.
  - Run: `cargo run -p minimal`
- **advanced:** A comprehensive example demonstrating custom nodes, widgets inside nodes, minimap, selection logic, link validation, and manual callback implementation.
  - Run: `cargo run -p advanced`
- **animated-links:** Demonstrates creative link animations (growing/snake effect) using de Casteljau's algorithm and glow effects.
  - Run: `cargo run -p animated-links`
- **custom-shapes:** Shows how to implement custom link routing (e.g., orthogonal) and reactive styling.
  - Run: `cargo run -p custom-shapes`
- **pin-compatibility:** Demonstrates type-safe connections with a compatibility matrix, visual validation feedback, and custom pin behaviors.
  - Run: `cargo run -p pin-compatibility`
- **zoom-stress-test:** Tests widget scaling at various zoom levels with Level of Detail (LOD) rendering. Shows how to implement LOD transitions for complex nodes.
  - Run: `cargo run -p zoom-stress-test`

## License

This library is licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

### A note on Slint's licensing

That choice covers **this library's own code only**. It does not, and cannot,
relicense Slint.

Slint is distributed by SixtyFPS GmbH under its own terms, and you pick one of
three: **GPLv3**, a **royalty-free** license, or a **commercial** license.
Anything you build with this library links against Slint, so those terms apply
to your application independently of the permissive license above.

Two conditions are easy to miss if you assume the royalty-free option is
unconditional:

- It requires **disclosing that you use Slint** — via the `AboutSlint` widget or
  the Slint badge. Without that disclosure, Slint's terms direct you to the
  commercial license.
- It **excludes embedded systems**, which need a commercial license regardless
  of disclosure.

In short: a permissive license here does not make the resulting application
unencumbered. Confirm which Slint license you are relying on and comply with it.
The authoritative statement is Slint's
[LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md); the
available plans are at [slint.dev/pricing](https://slint.dev/pricing).
