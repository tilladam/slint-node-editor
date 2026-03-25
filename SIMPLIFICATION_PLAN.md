# Node Editor Simplification Plan

Goal: Eliminate all node editor mechanics from examples, leaving only domain-specific logic.

## Phase 1: Auto-Grid Management

**Goal:** Grid updates happen automatically inside the library

**Changes:**

1. NodeEditor internally watches `changed zoom`, `changed pan-x`, `changed pan-y`
2. Calls internal grid generation function
3. Updates own `grid-commands` property
4. Remove from examples: `on_request_grid_update`, `on_viewport_changed` (for grid), `generate_grid()` calls

**Impact:** Removes 2-3 callbacks from every example

---

## Phase 2: Default Link Path Computation

**Goal:** Standard bezier path is automatic

**Changes:**

1. Add `use-default-link-paths: bool` property to NodeEditor (default: true)
2. When true, NodeEditor internally sets own `compute-link-path` callback
3. Uses library's `generate_bezier_path()` with standard offset
4. Examples with custom paths (orthogonal, animated) set property to false and provide callback
5. Remove from simple examples: `on_compute_link_path` entirely

**Impact:** Removes 1 callback from minimal, sugiyama, zoom-stress-test

---

## Phase 3: Default Pin Hit Testing

**Goal:** Standard circular pin hit testing is automatic

**Changes:**

1. Add `use-default-pin-hit-test: bool` property to NodeEditor (default: true)
2. When true, internally wires `compute-pin-at` to cache lookup
3. Examples with custom hit areas (rectangular pins, etc.) override
4. Remove from simple examples: `on_compute_pin_at`

**Impact:** Removes 1 callback from examples with pin dragging

---

## Phase 4: Default Link Preview Path

**Goal:** Link creation preview uses standard bezier automatically

**Changes:**

1. Add `use-default-link-preview: bool` property to NodeEditor (default: true)
2. When true, internally wires `compute-link-preview-path`
3. Custom examples (animated, custom shapes) override
4. Remove from simple examples: `on_compute_link-preview-path`

**Impact:** Removes 1 callback from examples with link creation

---

## Phase 5: Geometry Tracking Consolidation ✓ COMPLETED

**Goal:** NodeEditorSetup (or new helper) handles all geometry callbacks automatically

**Changes:**

1. ✓ Pin component now supports `parent-offset-x/y` properties for wrapped pins
   - Wrapped pins (e.g., ValidatedPin) can pass their offset to report correct position
   - Pin calculates `center-x/y` including parent offset for geometry reporting
   - Examples with wrapped pins set these properties to the wrapper's position
2. ✓ `wire_node_editor!` macro automatically wires geometry callbacks
   - `on_report_node_rect` and `on_report_pin_position` to controller
   - `on_start_node_drag` and `on_end_node_drag` for node movement
   - `on_compute_link_path` for link path generation
3. ✓ All examples converted: minimal, sugiyama, animated-links, custom-shapes, zoom-stress-test, pin-compatibility, advanced

**Impact:** Removes 5 explicit callback setups (2 geometry + 2 drag + 1 path) from all examples, enables pin wrapping without coordinate math

---

## Phase 6: Node Drag Consolidation

**Goal:** Drag start/end handled by setup helper

**Changes:**

1. NodeEditorSetup already has drag callback
2. Extend `wire_node_editor!` to also wire `on_node_drag_started`
3. Move model update logic into NodeEditorSetup (already there)
4. Remove from examples: manual `on_node_drag_started` and `on_node_drag_ended` setup

**Impact:** Removes 2 callback setups, examples only provide model update closure to setup

---

## Phase 7: Optional Default Hit Testing for Links/Boxes

**Goal:** Standard link and box selection hit testing automatic

**Changes:**

1. Add properties: `use-default-link-hit-test`, `use-default-box-selection`
2. When true, wire callbacks internally
3. Advanced example overrides for custom logic
4. Remove from simple examples: `on_compute_link_at`, `on_compute_box_selection`

**Impact:** Removes 2 callbacks from examples with selection

---

## Target: Minimal Example After All Phases

```rust
let nodes = Rc::new(VecModel::from([...]));
window.set_nodes(ModelRc::from(nodes.clone()));
window.set_links(ModelRc::from([...]));

let setup = NodeEditorSetup::new(move |node_id, dx, dy| {
    // Update node position in model
});

wire_node_editor!(window, setup);
window.run().unwrap();
```

**~10 lines total** for a working node editor with connections, dragging, zoom/pan, grid.

---

## Implementation Order

Priority by impact/simplicity ratio:

1. ✅ **Phase 5** (Geometry + Drag + Link Path) - COMPLETED, affects all examples, 5 callbacks removed
2. **Phase 1** (Grid) - Easy, affects all examples, 2-3 callbacks removed
3. **Phase 2** (Link paths) - ~~Medium~~ DEFERRED (already handled by Phase 5's wire_node_editor!)
4. **Phase 6** (Drag) - ~~Easy~~ DEFERRED (already handled by Phase 5's wire_node_editor!)
5. **Phase 3** (Pin hit test) - Easy, affects some examples
6. **Phase 4** (Link preview) - Easy, affects some examples
7. **Phase 7** (Advanced hit testing) - Last, only benefits complex examples
