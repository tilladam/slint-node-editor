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
import { NodeEditor, BaseNode, Pin, Link, LinkData } from "@slint-node-editor/node-editor.slint";
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
    Text { text: "My Node"; }
    Pin {
        pin-type: PinTypes.input;
        node-id: root.id;
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
            id: node.id;
            world-x: node.x;
            world-y: node.y;
            // ... other bindings
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
in-out property <float> pan-x;           // Pan offset (x)
in-out property <float> pan-y;           // Pan offset (y)
in-out property <float> zoom;            // Zoom factor (1.0 = 100%)
in-out property <float> min-zoom: 0.1;   // Minimum zoom level
in-out property <float> max-zoom: 5.0;   // Maximum zoom level

in property <length> grid-spacing: 24px;       // Grid cell size
in property <bool> grid-snapping: true;        // Enable snap-to-grid
in property <color> grid-color: #e0e0e0;      // Grid line color
in property <color> background-color: #f5f5f5; // Background color

in property <float> link-hover-distance: 10.0;  // Click tolerance for links
in property <int> link-hit-samples: 20;         // Bezier samples for hit-testing
in property <float> bezier-min-offset: 50.0;    // Min horizontal offset for curves

// Minimap
in property <bool> minimap-enabled: false;
in property <MinimapPosition> minimap-position: top-right;

// Data bindings
in property <[LinkData]> links;                 // Links to render
in-out property <[int]> selected-node-ids;      // Selected nodes
in-out property <[int]> selected-link-ids;      // Selected links
in property <[MinimapNode]> minimap-nodes;      // Minimap data
in property <float> graph-min-x;                // Graph bounds
in property <float> graph-max-x;
in property <float> graph-min-y;
in property <float> graph-max-y;

// State outputs (read-only)
out property <bool> is-selecting;               // User is dragging selection box
out property <float> selection-x;               // Selection box position
out property <float> selection-y;
out property <float> selection-width;           // Selection box size
out property <float> selection-height;

out property <bool> is-creating-link;           // User is dragging to create link
out property <float> link-start-x;              // Link preview start
out property <float> link-start-y;
out property <float> link-end-x;                // Link preview end
out property <float> link-end-y;
out property <int> link-start-pin-id;           // Which pin started the link

out property <bool> is-dragging;                // User is dragging nodes
out property <float> drag-offset-x;             // Drag delta
out property <float> drag-offset-y;

out property <int> context-menu-x;              // Right-click position
out property <int> context-menu-y;

out property <int> hovered-link-id;             // Link under mouse
out property <int> selection-version;           // Version counter for cache invalidation
```

**Callbacks (Computation):**

All these callbacks delegate expensive operations to your Rust code:

```slint
/// Compute which pin is at screen position (x, y)
callback compute-pin-at(x: float, y: float) -> int;

/// Compute which link is at screen position (x, y)
callback compute-link-at(x: float, y: float) -> int;

/// Find all nodes in a selection box (world coordinates)
callback compute-box-selection(x: float, y: float, w: float, h: float) -> [int];

/// Find all links in a selection box (world coordinates)
callback compute-link-box-selection(x: float, y: float, w: float, h: float) -> [int];

/// Generate SVG path for link preview (during drag-to-link)
callback compute-link-preview-path(
    start-x: float, start-y: float,
    end-x: float, end-y: float
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
callback context-menu-requested(x: float, y: float);

/// User finished dragging nodes
callback node-drag-ended(delta-x: float, delta-y: float);

/// Node geometry changed (position or size)
callback node-rect-changed(id: int, x: float, y: float, w: float, h: float);

/// Pin geometry changed (position relative to node)
callback pin-position-changed(
    pin-id: int,
    node-id: int,
    pin-type: int,
    rel-x: float,
    rel-y: float
);
```

### BaseNode

Base component for creating custom nodes. Provides drag handling and selection.

**Properties:**
```slint
in property <int> id;                  // Unique node ID
in-out property <float> world-x;       // X position in graph space
in-out property <float> world-y;       // Y position in graph space
in property <float> width;             // Node width
in property <float> height;            // Node height
in property <bool> selected: false;    // Selection state
```

### Pin

Represents a connection point on a node.

**Properties:**
```slint
in property <int> id;           // Pin ID (encoded by application)
in property <int> node-id;      // Parent node ID
in property <int> pin-type;     // PinTypes.input or .output (or custom)
in property <color> color: #999;
in property <float> rel-x;      // Relative X (from node top-left)
in property <float> rel-y;      // Relative Y (from node top-left)
```

### Link

Renders a Bezier curve between two pins. Used internally by `NodeEditor` but can be used for custom rendering.

**Properties:**
```slint
in property <string> path-commands;        // SVG path (e.g., "M 0 0 C 50 50 100 100")
in property <color> color: #666;
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
- **all-features:** A comprehensive example demonstrating custom nodes, widgets inside nodes, minimap, selection logic, link validation, and manual callback implementation.
  - Path: `examples/all-features`
  - Run: `cargo run -p all-features`

## License

Same as Slint framework