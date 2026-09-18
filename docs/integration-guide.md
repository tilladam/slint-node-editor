# Integration guide

[README](../README.md) · [Component reference](component-reference.md)

The quick start is a complete downstream crate, compiled outside this workspace
by [`smoke/run.sh packaged`](../smoke/run.sh). Copy its four application files:

- [`Cargo.toml`](../smoke/downstream/Cargo.toml)
- [`build.rs`](../smoke/downstream/build.rs)
- [`ui/app.slint`](../smoke/downstream/ui/app.slint)
- [`src/main.rs`](../smoke/downstream/src/main.rs)

It starts with two nodes. Click a node to select it, drag its body to move it,
drag from one pin to the other to connect the nodes, and use the toolbar to add
or delete nodes.

## 1. Add dependencies

Slint Node Editor 1.0.1 requires Slint 1.18.0 or newer and Rust 1.92 or newer.
The manifest below uses the published crates.io package.
The software renderer avoids a native graphics SDK dependency.

```toml
[workspace]

[package]
name = "node-editor-quick-start"
version = "1.0.0"
edition = "2021"

[dependencies]
slint = { version = "1.18.0", default-features = false, features = ["std", "compat-1-18", "backend-winit", "renderer-software"] }
slint-node-editor = "1.0.1"

[build-dependencies]
slint-build = { version = "1.18.0", features = ["experimental-module-builds"] }
```

The standalone fixture also has a test-only backend dependency; it is
unnecessary in an application.

## 2. Compile the Slint UI

`experimental-module-builds` lets `@nodeeditor` resolve from dependency
metadata. The complete `build.rs` is:

```rust
fn main() {
    slint_build::compile("ui/app.slint").unwrap();
}
```

No library paths or environment variables are required.

## 3. Compose the editor

The complete compiled UI is [`ui/app.slint`](../smoke/downstream/ui/app.slint).
This excerpt shows the required integration surface:

```slint
// Excerpt — copy the complete linked file above.
import {
    NodeEditor, BaseNode, Pin, PinTypes,
    NodeEditorInternalCallbacks, NodeEditorComputations, LinkData,
} from "@nodeeditor";

export { PinTypes, NodeEditorInternalCallbacks, NodeEditorComputations }

component QuickNode inherits BaseNode {
    node-width: 180px;
    node-height: 80px;

    Pin {
        pin-id: root.node-id * 2;
        node-id: root.node-id;
        pin-type: PinTypes.input;
        node-screen-x: root.screen-x;
        node-screen-y: root.screen-y;
    }
}

export component App inherits Window {
    in property <[NodeData]> nodes;
    in property <[LinkData]> links <=> editor.links;

    // Required names used by wire_node_editor! for grid generation.
    in-out property <string> grid-commands <=> editor.grid-commands;
    out property <float> width_: editor.width / 1px;
    out property <float> height_: editor.height / 1px;

    // Required by wire_node_editor! for synchronous focus on pointer presses.
    public function focus-editor() {
        editor.focus();
    }

    // Required by wire_selection! and the host link policy.
    callback link-requested <=> editor.link-requested;
    callback node-selected <=> editor.node-selected;
    callback select-link <=> editor.select-link;
    callback selection-cleared <=> editor.selection-cleared;
    callback box-selection-committed <=> editor.box-selection-committed;
    callback compute-link-at <=> editor.compute-link-at;

    editor := NodeEditor {
        has-link-selection: true;
        for node in root.nodes: QuickNode {
            node-id: node.id;
            world-x: node.x * 1px;
            world-y: node.y * 1px;
            selected: node.selected;
        }
    }
}
```

Each pin needs a unique positive `pin-id`, its owning `node-id` and type, and
the node's screen position. The complete example supplies both an input and
an output pin.

## 4. Wire the Rust side

The complete compiled implementation is
[`src/main.rs`](../smoke/downstream/src/main.rs). The central setup is:

```rust
// Excerpt — NodeData implements MovableNode in the complete linked file.
let setup = NodeEditorSetup::new({
    let nodes = nodes.clone();
    move |dragged, dx, dy| GraphLogic::commit_drag(&nodes, dragged, dx, dy)
});

wire_node_editor!(window, setup);
wire_selection!(window, setup, nodes, links);
```

Keep `LinkPath` plus the two macros and setup types in Rust scope. The generated
`NodeEditorInternalCallbacks` and `NodeEditorComputations` types come from the
Slint exports shown above. `wire_node_editor!` installs geometry,
route, pin-picking, viewport, grid, and press-focus handlers. `wire_selection!`
resolves each selection intent immediately and writes the absolute result into row
`selected` flags before a drag continues.

Interactive link selection also requires the application-owned
`compute-link-at` callback. The complete quick start resolves the current link
rows against the controller's geometry cache and converts the fixed screen
tolerance to world units before calling `find_bezier_link_at_world`.

Slint callbacks have one handler. Install application overrides after the
macros; the last `on_*` handler replaces the earlier one. Replacing a
computation or lifecycle handler also takes responsibility for the behavior the
macro supplied.

The window's `focus-editor()` function must call `editor.focus()` synchronously.
Presses on the canvas, nodes, pins, minimap, and reserved box-selection overlay
give the editor keyboard focus. Embedded text fields keep focus when they accept
the press. A host callback may move focus elsewhere in response to that press;
the editor does not take it back afterward. Manual integrations must connect
`NodeEditorInternalCallbacks.take-editor-focus` to the same function.

The host handles `link-requested`: validate and normalize the two pins, then add
a `LinkData` row. The fixture rejects same-node, same-type and duplicate links.
Keyboard and toolbar policy is also host-owned. Its delete handler removes
connected logical links and selected node rows, then invokes
`NodeEditorInternalCallbacks.remove-node(id)` to retire cached geometry and
interaction state.

## 5. Run the application

```sh
cargo run
```

## Integration contracts

### Geometry and IDs

`BaseNode` and `Pin` automatically publish geometry after they render. Model
positions, drag deltas, cached rectangles, pin offsets, link picking and box
selection use world units. Screen positions are `world * zoom + pan`; pointer
hit tolerances are converted to world units internally.

IDs are integers with these current domains:

| Kind | Contract |
|---|---|
| Node | Unique while live; `0` is reserved for “no node”; negative IDs are supported |
| Pin | Unique and positive while live; otherwise opaque to the library |
| Link | Unique and nonnegative while live; `-1` means “no link” in picking |

Pin IDs are opaque: choose any scheme that satisfies the table above. Geometry
reports carry `pin-id`, `node-id`, and `pin-type` separately, so the library
never decodes a pin ID.

### Drag commits

Host drag callbacks must project their final position synchronously before
returning. BaseNode then discards its gesture offset and uses the model even
when the host rejects the move or snaps one or both axes back to unchanged
coordinates. Asynchronous hosts may apply a later model update, but the editor
shows the current authoritative position while waiting.

### Valid geometry

Coordinates and pin offsets must be finite; node dimensions and zoom must be
positive and finite. Keep Slint viewport bindings within the configured zoom
limits. Rust `NodeEditorController::set_viewport` ignores an invalid update
atomically, preserving the previous transform; it does not repair invalid
values in the host's Slint properties. Low-level geometry maps and pure helpers
remain caller-validated APIs.

### Viewport size and transform

`NodeEditorComputations.viewport-changed(zoom, pan-x, pan-y)` reports pan and
zoom changes. `NodeEditorComputations.viewport-resized(width, height)` reports
changes to the editor's own dimensions in logical pixels. Listen to both when
tracking the visible world rectangle; `request-grid-update` is only a request
to regenerate the grid and is unnecessary for a host without one.

`wire_node_editor!` leaves the resize callback available for application code:

```rust
window.global::<NodeEditorComputations>().on_viewport_resized({
    let window = window.as_weak();
    move |width, height| {
        if let Some(window) = window.upgrade() {
            let center_x = (width / 2.0 - window.get_pan_x()) / window.get_zoom();
            let center_y = (height / 2.0 - window.get_pan_y()) / window.get_zoom();
            // Update the application's visible centre or culling bounds here.
        }
    }
});
```

Expose the editor's `pan-x`, `pan-y`, and `zoom` on the window when using this
snippet. Resize notifications do not replace the macro's transform handler.
If overriding `viewport-changed` to track pan and zoom as well, preserve its
call to `setup.controller().set_viewport(zoom, pan_x, pan_y)`.

### Link defaults and selection

`LinkData::default()` has Slint's zero-valued status (idle), which overrides its
color. Use `LinkData::new(id, output, input, color)` for a 2px colored link with
no status override. Link box selection uses endpoint inclusion: either endpoint
inside or on the box selects the link; a crossing curve whose endpoints are
both outside is omitted.

### Replacing and seeding a graph

When replacing the whole graph, install the new models and invoke
`NodeEditorInternalCallbacks.reset-graph()`. When seeding geometry without live
components, call `NodeEditor.report-node-rect` and
`NodeEditor.report-pin-position`; those functions update the same cache and
invalidation lifecycle used by `BaseNode` and `Pin`.

See the [component reference](component-reference.md) for properties, callbacks,
selection policy, and geometry lifecycle helpers.
