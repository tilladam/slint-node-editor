# AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the **Slint Node Editor** library (`slint-node-editor`), a generic Slint component library for building visual graph editors. It includes examples demonstrating how to use the library.

## Build Commands

```bash
# Build and run the full-featured example
cargo run -p advanced

# Build and run the minimal example
cargo run -p minimal

# Build and run the custom shapes example
cargo run -p custom-shapes

# Build and run the zoom stress test (LOD system demo)
cargo run -p zoom-stress-test

# Check compilation without running
cargo check
```

## Key Files

| File | Purpose |
|------|---------|
| `examples/advanced/src/main.rs` | Application entry point (Full Example) |
| `src/lib.rs` | Library entry point (Rust) |
| `node-editor.slint` | Generic Slint components (NodeEditor, Pin, Link, Minimap) |
| `examples/advanced/ui/pin_encoding.slint` | Application-specific pin ID encoding scheme |
| `examples/advanced/ui/ui.slint` | Application-specific UI (Node component, data structs, main window) |
| `examples/advanced/ui/filter_node.slint` | Complex node example with multiple widgets |
| `examples/zoom-stress-test/ui/*.slint` | LOD implementation patterns for zoom scaling |

## Architecture

**Three-layer rendering** (back to front):
1. **Background** - Grid and link paths (SVG-based)
2. **Children** - Node components (Slint components)
3. **Overlay** - Selection box, link preview, input handling

**Coordinate systems**:
- **World coordinates**: Graph-space positions (`world_x`, `world_y` on nodes)
- **Screen coordinates**: After pan/zoom transformation
- Conversion: `screen_x = world_x * zoom + pan_x`

**Pin ID encoding**: Application-specific. The generic library treats pin IDs as opaque integers. Examples:
- minimal: `pin_id = node_id * 2 + pin_type` (0=input, 1=output)
- advanced: `pin_id = node_id * 1000 + pin_type`

**Callback-based computation**: The Slint UI delegates expensive operations to Rust via callbacks (e.g., `compute-pin-at`).

**Level of Detail (LOD)**: NodeEditor supports configurable LOD thresholds for zoom-dependent rendering:
- `lod-full-threshold: 0.5` - Zoom above which nodes render full detail
- `lod-simplified-threshold: 0.25` - Zoom above which nodes render simplified detail
- Below simplified threshold: nodes render as minimal colored boxes
- See `examples/zoom-stress-test` for implementation patterns

## Data Models (in examples/advanced/src/main.rs)

The application maintains separate models for different node types but unifies operations via helper functions:

```rust
VecModel<NodeData>       // Simple nodes
VecModel<FilterNodeData> // Complex nodes
VecModel<LinkData>       // Logical connections
```

**Selection State**:
- **Source of Truth**: `selected_node_ids` and `selected_link_ids` (VecModels shared with Slint).
- **Performance Cache**: Rust maintains `HashSet<i32>` caches (`selection_set`, `link_selection_set`) for O(1) lookups.
- **View Binding**: UI components bind `selected` property to the `is-node-selected` callback (reactive via `selection-version`).

## Library Helpers

The library provides Rust helpers to reduce boilerplate:

**GeometryTracker** - Convenience wrapper for geometry cache setup:
```rust
let tracker = GeometryTracker::new();
window.on_node_rect_changed(tracker.node_rect_callback());
window.on_pin_position_changed(tracker.pin_position_callback());
let cache = tracker.cache(); // Use for hit testing, etc.
```

**Trait-based design** mirrors node handling:
- Nodes: `NodeGeometry` trait + `GeometryCache<N>`
- Links: `LinkModel` trait + `LinkManager<L, N>`

## Helper Functions (advanced example)
`examples/advanced/src/main.rs` defines local helpers to handle multiple node models:

- `remove_selected_items`: Generic function to remove items from a `VecModel` based on selection.
- `compute_graph_bounds`: Iterates over all node models to calculate the bounding box of the graph.
- `build_minimap_nodes`: construct a model for the minimap from all node models.