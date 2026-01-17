# Slint Node Editor Library

A flexible, **generic** Slint component library for building visual graph editors. Supports data flow diagrams, state machines, shader graphs, and any visual node-based interface.

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
slint = "1.x"  # Required

[dependencies.slint-node-editor]
path = "path/to/slint-node-editor"
```

### 2. Import Core Components

```slint
// In your main .slint file
import { NodeEditor, BaseNode, Pin, Link, LinkData, PinTypes } from "@slint-node-editor/node-editor.slint";
```

### 3. Configure build.rs

To use the `@slint-node-editor` import prefix, you must register it in your `build.rs`:

```rust
fn main() {
    let mut library_paths = std::collections::HashMap::new();
    library_paths.insert("slint-node-editor".into(), "path/to/slint-node-editor".into());

    let config = slint_build::CompilerConfiguration::default()
        .with_library_paths(library_paths);
    slint_build::compile_with_config("ui/main.slint", config).unwrap();
}
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

The library supports all approaches via the `pin-position-changed` callback, which passes `node-id` and `pin-type` separately.

### Coordinate Systems

The editor uses two coordinate systems:

1. **World Coordinates** - Graph space (where nodes live)
   - Property: `world-x`, `world-y` on `BaseNode`
   - Range: Unbounded (can be negative, very large)

2. **Screen Coordinates** - After pan/zoom transformation
   - Computed: `screen_x = world_x * zoom + pan_x`
   - Used for: Hit-testing, rendering, mouse interaction

The library handles all transformations transparently.

## Component Reference

### NodeEditor (Main Component)

**Properties:**
```slint
in-out property <length> pan-x;          // Pan offset (x)
in-out property <length> pan-y;          // Pan offset (y)
in-out property <float> zoom;            // Zoom factor (1.0 = 100%)
in property <float> min-zoom: 0.1;       // Minimum zoom level
in property <float> max-zoom: 3.0;       // Maximum zoom level

in property <length> grid-spacing: 24px;       // Grid cell size
in property <bool> grid-snapping: true;        // Enable snap-to-grid
in property <color> grid-color: #404040;       // Grid line color
in property <brush> background-color: #1a1a1a; // Background color

in property <length> link-hover-distance: 8px; // Click tolerance for links
in property <length> pin-hit-radius: 10px;     // Hit radius for pins
in property <int> link-hit-samples: 20;        // Bezier samples for hit-testing
in property <float> bezier-min-offset: 50.0;   // Min horizontal offset for curves

// Minimap
in property <bool> minimap-enabled: false;
in property <MinimapPosition> minimap-position: bottom-right;

// Data bindings
in property <[LinkData]> links;                 // Links to render
in-out property <[int]> selected-node-ids;      // Selected nodes
in-out property <[int]> selected-link-ids;      // Selected links
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

out property <bool> is-dragging;                // User is dragging nodes
out property <length> drag-offset-x;            // Drag delta
out property <length> drag-offset-y;

out property <length> context-menu-x;           // Right-click position
out property <length> context-menu-y;

out property <int> hovered-link-id;             // Link under mouse
out property <int> selection-version;           // Version counter for cache invalidation
```

**Callbacks (Computation):**

All these callbacks delegate expensive operations to your Rust code:

```slint
/// Compute which pin is at screen position (x, y)
callback compute-pin-at(x: length, y: length) -> int;

/// Compute which link is at screen position (x, y)
callback compute-link-at(x: length, y: length) -> int;

/// Find all nodes in a selection box (world coordinates)
callback compute-box-selection(x: length, y: length, w: length, h: length) -> [int];

/// Find all links in a selection box (world coordinates)
callback compute-link-box-selection(x: length, y: length, w: length, h: length) -> [int];

/// Generate SVG path for link preview (during drag-to-link)
callback compute-link-preview-path(
    start-x: length, start-y: length,
    end-x: length, end-y: length
) -> string;

/// Compute SVG path for a link between two pins
pure callback compute-link-path(
    start-pin-id: int, end-pin-id: int, version: int
) -> string;

/// Request grid update (when pan/zoom changes)
callback request-grid-update();
```

**Callbacks (Selection):**

```slint
/// User clicked a node
callback select-node(node-id: int, shift-held: bool);

/// User clicked a link
callback select-link(link-id: int, shift-held: bool);

/// User clicked background (clear selection)
callback clear-selection();

/// Sync selection state to all nodes (after box selection)
callback sync-selection-to-nodes(node-ids: [int]);

/// Sync selection state to all links (after box selection)
callback sync-selection-to-links(link-ids: [int]);

/// Pure callback: Is this node selected? (Used for reactive binding)
pure callback is-selected(node-id: int, version: int) -> bool;

/// Pure callback: Is this link selected?
pure callback is-link-selected(link-id: int, version: int) -> bool;
```

**Callbacks (Events):**

```slint
/// User completed a link (dragged from one pin to another)
callback link-requested(start-pin-id: int, end-pin-id: int);

/// User cancelled link creation (ESC or right-click)
callback link-cancelled();

/// User hovered over a link
callback link-hovered(link-id: int);

/// Viewport changed (pan/zoom)
callback viewport-changed();

/// User pressed Delete key
callback delete-selected();

/// User pressed Ctrl+N or clicked add button
callback add-node-requested();

/// User right-clicked (context menu)
callback context-menu-requested();

/// User finished dragging nodes
callback node-drag-ended(delta-x: float, delta-y: float);

/// Node geometry changed (position or size)
callback node-rect-changed(id: int, x: length, y: length, w: length, h: length);

/// Pin geometry changed (position relative to node)
callback pin-position-changed(
    pin-id: int,
    node-id: int,
    pin-type: int,
    rel-x: length,
    rel-y: length
);
```

### BaseNode

Base component for creating custom nodes. Provides drag handling and selection.

**Properties:**
```slint
in property <int> node-id;             // Unique node ID
in property <length> world-x;          // X position in graph space
in property <length> world-y;          // Y position in graph space
in property <bool> selected: false;    // Selection state
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

**Properties:**
```slint
in property <string> path-commands;        // SVG path (e.g., "M 0 0 C 50 50 100 100")
in property <color> link-color: #888;
in property <length> line-width: 2px;
in property <bool> selected: false;
in property <bool> hovered: false;
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
    window.on_compute_link_path(ctrl.compute_link_path_callback());
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

    window.run().unwrap();
}
```

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

- **minimal:** A simple example using `NodeEditorController` with basic nodes and links.
  - Path: `examples/minimal`
  - Run: `cargo run -p minimal`
- **advanced:** A comprehensive example demonstrating custom nodes, widgets inside nodes, minimap, selection logic, link validation, and manual callback implementation.
  - Path: `examples/advanced`
  - Run: `cargo run -p advanced`
- **pin-compatibility:** Demonstrates type-safe connections with a compatibility matrix, visual validation feedback, and custom pin behaviors.
  - Path: `examples/pin-compatibility`
  - Run: `cargo run -p pin-compatibility`

## License

Same as Slint framework