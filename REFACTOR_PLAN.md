# Global Callbacks Refactoring Plan

## Goal

Remove all callback boilerplate from examples. Examples should only need:

```rust
let window = MainWindow::new().unwrap();
let ctrl = NodeEditorController::new();
ctrl.wire_to_window(&window); // ONE LINE
window.run().unwrap();
```

## Current Problems

1. **Examples must define callbacks**: Each example's MainWindow must declare callbacks like `compute-link-path`
2. **Examples must forward callbacks**: Each example must wire NodeEditor callbacks to MainWindow callbacks
3. **Nodes don't update during drag**: Links don't follow nodes because geometry-version doesn't increment during drag
4. **Too much boilerplate**: Minimal example has ~30 lines of callback wiring

## Solution Architecture

### Slint Side Changes

#### 1. Create `NodeEditorComputations` Global

Location: `node-editor-building-blocks.slint`

```slint
/// Computational callbacks wired by Rust - examples don't touch these!
export global NodeEditorComputations {
    // Link path computation
    pure callback compute-link-path(int, int, int, float, float, float) -> string;

    // Viewport updates
    callback viewport-changed(float, float, float);

    // Selection checking
    pure callback is-node-selected(int, int) -> bool;
    pure callback is-link-selected(int, int) -> bool;

    // Other computational callbacks as needed
}
```

#### 2. Update NodeEditor to Call Global

Location: `node-editor.slint`

**BEFORE:**

```slint
export component NodeEditor {
    pure callback compute-link-path(...) -> string;

    for link in links: Link {
        path-commands: root.compute-link-path(...)  // Calls own callback
    }
}
```

**AFTER:**

```slint
export component NodeEditor {
    // NO callbacks defined here!

    for link in links: Link {
        path-commands: NodeEditorComputations.compute-link-path(...)  // Calls global
    }
}
```

#### 3. Simplify Examples

Location: `examples/minimal/ui/minimal.slint`

**BEFORE:**

```slint
export component MainWindow {
    callback update-viewport(...);
    pure callback compute-link-path(...) -> string;

    editor := NodeEditor {
        compute-link-path(start, end, ver, z, px, py) => {
            root.compute-link-path(start, end, ver, z, px, py)
        }
        viewport-changed => {
            root.update-viewport(self.zoom, self.pan-x / 1px, self.pan-y / 1px);
        }
    }
}
```

**AFTER:**

```slint
export component MainWindow {
    // NO callbacks!

    editor := NodeEditor {
        // NO callback wiring!
        for data in root.nodes: SimpleNode { ... }
    }
}
```

### Rust Side Changes

#### 4. Add `wire_to_window()` Method

Location: `src/controller.rs`

```rust
impl NodeEditorController {
    /// Wire all callbacks in one call - examples use this
    pub fn wire_to_window<W: HasGlobals>(&self, window: &W) {
        // Wire GeometryCallbacks
        self.wire_geometry_callbacks(window);

        // Wire NodeEditorComputations
        self.wire_computations(window);
    }

    fn wire_geometry_callbacks<W: HasGlobals>(&self, window: &W) {
        let geometry_callbacks = window.global::<GeometryCallbacks>();

        geometry_callbacks.on_report_node_rect({
            let ctrl = self.clone();
            move |id, x, y, w, h| ctrl.handle_node_rect(id, x, y, w, h)
        });

        geometry_callbacks.on_report_pin_position({
            let ctrl = self.clone();
            move |pid, nid, ptype, x, y| ctrl.handle_pin_position(pid, nid, ptype, x, y)
        });

        // ... other geometry callbacks
    }

    fn wire_computations<W: HasGlobals>(&self, window: &W) {
        let computations = window.global::<NodeEditorComputations>();

        computations.on_compute_link_path(self.compute_link_path_callback());

        computations.on_viewport_changed({
            let ctrl = self.clone();
            move |z, px, py| ctrl.set_viewport(z, px, py)
        });

        // ... other computational callbacks
    }
}
```

## Implementation Steps

1. ✅ **TEST**: Verify Slint supports `pure callback` on globals (CONFIRMED)
2. ⏳ **Create NodeEditorComputations global** in `node-editor-building-blocks.slint`
3. ⏳ **Update NodeEditor** to call `NodeEditorComputations` instead of own callbacks
4. ⏳ **Simplify minimal.slint** - remove all callbacks
5. ⏳ **Update minimal/src/main.rs** - use `ctrl.wire_to_window(&window)`
6. ⏳ **Test minimal example** compiles and runs
7. ⏳ **Fix drag updates** - ensure geometry-version increments during drag
8. ⏳ **Update other examples** (advanced, zoom-stress-test, etc.)
9. ⏳ **Update documentation**

## Current Issues to Fix

### Issue 1: Links Don't Update During Drag

**Problem**: `changed drag-offset-x/y` handlers don't fire because they're computed properties
**Solution**: Already implemented - BaseNode's TouchArea moved handler calls:

```slint
GeometryVersion.version += 1;
GeometryCallbacks.report-node-rect(node-id, world-x + current-drag-offset-x, ...);
```

### Issue 2: There are leftover broken changed handlers

**Location**: `node-editor-building-blocks.slint:452`
**Error**: `changed DragState.drag-offset-x =>` (invalid syntax)
**Solution**: Remove these - they were from a failed attempt

## Benefits After Refactoring

1. ✅ **Zero callback boilerplate** in examples
2. ✅ **One-line Rust wiring**: `ctrl.wire_to_window(&window)`
3. ✅ **Automatic geometry updates**: Links follow nodes during drag
4. ✅ **Consistent architecture**: All globals work the same way
5. ✅ **Easy to extend**: New examples just copy the pattern

## Files Modified

### Library Files

- `node-editor-building-blocks.slint` - Add NodeEditorComputations global
- `node-editor.slint` - Remove callbacks, call globals instead
- `src/controller.rs` - Add wire_to_window() method
- `src/lib.rs` - Export new types if needed

### Example Files

- `examples/minimal/ui/minimal.slint` - Remove all callbacks
- `examples/minimal/src/main.rs` - Simplify to one-line wiring
- Other examples: Update similarly

## Testing Checklist

- [ ] minimal example compiles
- [ ] minimal example runs
- [ ] Can drag nodes
- [ ] Links follow nodes during drag
- [ ] Links update when panning
- [ ] Selection works
- [ ] Double-click works
- [ ] Link creation works
- [ ] All tests pass
