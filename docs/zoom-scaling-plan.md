# Zoom Scaling Issues: Research Summary & Implementation Plan

## Current Problem Analysis

The current implementation scales everything linearly by zoom factor:
- At **0.15x**: 14px font → 2.1px (unreadable), buttons become ~5px tall (unusable)
- At **5.0x**: widgets become 5x normal size, potential rendering artifacts

## Research Findings

| Source | Strategy | Key Insight |
|--------|----------|-------------|
| [Rete.js LOD](https://retejs.org/examples/lod/) | Replace nodes with simplified versions at zoom thresholds | Viewport culling + LOD combined |
| [yWorks Diagrams](https://www.yworks.com/pages/level-of-detail-for-large-diagrams) | Content-aware simplification | Hide text "as soon as it becomes unreadable" |
| [Qt Elastic Nodes](https://doc.qt.io/qt-6/qtwidgets-graphicsview-elasticnodes-example.html) | Scale limits + crisp rendering | `setMinimumSize()` + scale bounds checking |
| [Blender](https://blenderartists.org/t/node-editor-zoom-level/1478930) | 50 discrete zoom levels | Hardcoded min/max limits |
| [High DPI Canvas](https://web.dev/articles/canvas-hidipi) | devicePixelRatio scaling | Scale up then CSS scale down for crispness |

---

## Proposed Implementation Plan

### Phase 1: Minimum Node Size with LOD Transition

**Goal:** Ensure nodes remain visible and identifiable at all zoom levels.

**Key Insight:** Clamping internal element sizes (fonts, buttons) without clamping the node container causes overflow. Instead, we must:
1. Define a minimum node size (the entire node stops shrinking below a threshold)
2. Accept that nodes will overlap at very low zoom ("bird's eye view" behavior)
3. Use this as the trigger for LOD transitions

**Implementation:**
```slint
// Minimum visual size for the entire node
property <length> min-node-width: 80px;
property <length> min-node-height: 40px;

// Effective size with floor
property <length> node-width: max(base-width * zoom, min-node-width);
property <length> node-height: max(base-height * zoom, min-node-height);

// Detect when we've hit the floor (triggers LOD switch)
property <bool> at-minimum-size: base-width * zoom <= min-node-width;
```

**Behavior at low zoom:**
- Nodes stop shrinking at minimum size
- Nodes will overlap (acceptable for overview/navigation)
- This naturally triggers the LOD system (Phase 2) to show simplified views
- User understands they need to zoom in to interact

### Phase 2: Level of Detail (LOD) System

**Goal:** Switch node rendering based on zoom thresholds. This is the **highest priority** optimization for both performance and usability.

**Define LOD levels:**

| LOD | Zoom Range | Rendering | Interactivity |
|-----|------------|-----------|---------------|
| **Full** | > 0.5x | All widgets | Full editing |
| **Simplified** | 0.25x - 0.5x | Title + pins only | Drag, select, connect |
| **Minimal** | < 0.25x | Colored rectangle + title | Drag, select only |

**Implementation approach using component composition:**

```slint
// Shared node shell (used by all LOD levels)
component NodeShell inherits Rectangle {
    in property <string> title;
    in property <color> node-color;
    in property <bool> selected;
    in property <float> zoom;

    background: selected ? node-color.brighter(0.3) : node-color;
    border-radius: 4px * zoom;
    border-width: selected ? 2px : 1px;
    border-color: selected ? #fff : #666;
}

// LOD wrapper - handles switching logic so individual nodes don't need to
component LODNode inherits BaseNode {
    in property <float> lod-full-threshold: 0.5;
    in property <float> lod-simplified-threshold: 0.25;

    property <int> lod-level: zoom > lod-full-threshold ? 2
                            : zoom > lod-simplified-threshold ? 1
                            : 0;

    // Slots for each LOD level (provided by concrete node types)
    in property <component> full-content;
    in property <component> simplified-content;
    in property <component> minimal-content;

    if lod-level == 2: @full-content
    if lod-level == 1: @simplified-content
    if lod-level == 0: @minimal-content
}
```

**Note:** Slint doesn't support component slots directly, so the actual implementation will use conditional `if` blocks within each node type, but sharing style components (NodeShell, PinRow, etc.) to minimize duplication.

**Simplified content shows:**
- Node background with title (using NodeShell)
- Input/output pins in fixed positions (no labels)
- Node type icon (optional)

**Minimal content shows:**
- Solid colored rectangle (NodeShell only)
- Title text at fixed minimum font size (8-10px)
- No pins visible

### Phase 3: Hard Zoom Limits

**Goal:** Prevent unusable zoom extremes with simple hard limits.

~~Hybrid approach (decoupled position/visual zoom)~~ **REJECTED:** Decoupling visual-zoom from position-zoom creates an "exploding diagram" effect where whitespace grows but nodes don't, which is confusing to users.

**Simple approach - hard clamp:**

| Limit | Value | Rationale |
|-------|-------|-----------|
| **min-zoom** | 0.1x | Below this, even minimal LOD is illegible |
| **max-zoom** | 3.0x | Above this, rendering artifacts appear; nodes exceed screen size |
| **default-zoom** | 1.0x | Standard working zoom |

**Implementation:**
```slint
in property <float> min-zoom: 0.1;
in property <float> max-zoom: 3.0;

// In zoom handling:
root.zoom = clamp(new-zoom, min-zoom, max-zoom);
```

**Zoom presets for UX:**
- "Fit all" - Calculate zoom to show all nodes with padding
- "Fit selected" - Zoom to selected nodes
- "100%" - Reset to 1.0x

### Phase 4: Configurable Thresholds

**Goal:** Allow applications to tune LOD behavior for their use case.

**NodeEditor-level configuration:**
```slint
// In NodeEditor component
in property <float> lod-full-threshold: 0.5;
in property <float> lod-simplified-threshold: 0.25;
in property <length> min-node-width: 80px;
in property <length> min-node-height: 40px;
```

**Per-node-type overrides** (optional, for advanced use):
```slint
// In specific node types that need different behavior
in property <bool> use-custom-lod: false;
in property <float> custom-lod-full-threshold: 0.5;
```

---

## Implementation Order

1. **Phase 3** (Hard zoom limits) - ✅ DONE - Simplest, prevents worst-case scenarios immediately
2. **Phase 2** (LOD system) - ✅ DONE - Highest impact for both performance and usability
3. **Phase 1** (Minimum sizes) - ✅ DONE - Integrated with LOD transitions
4. **Phase 4** (Configuration) - ✅ DONE - Polish/extensibility

### Implementation Notes

All phases have been implemented in the `zoom-stress-test` example:

- **NodeEditor** now exposes `min-zoom`, `max-zoom`, `lod-full-threshold`, `lod-simplified-threshold`, `min-node-width`, and `min-node-height` properties
- **Node components** (`InputNode`, `ControlNode`, `DisplayNode`) accept LOD configuration as `in` properties with sensible defaults
- **LOD levels**: Full (>0.5x zoom), Simplified (0.25x-0.5x), Minimal (<0.25x)
- **Minimum node sizes** prevent nodes from becoming unusable at low zoom
- **Pins are hidden** at minimal LOD since connections don't make sense at bird's eye view

---

## Design Decisions

### Resolved Questions

| Question | Decision | Rationale |
|----------|----------|-----------|
| LOD transitions animated or instant? | **Instant** | Animation adds complexity; instant is standard in Blender, Unreal |
| Simplified nodes draggable/selectable? | **Yes** | Essential for navigation; only editing is disabled |
| Links at minimal LOD? | **Straight lines** | Bezier control points don't make sense at this scale |
| Zoom limits hard or soft? | **Hard clamp** | Soft limits (spring/resistance) add UX confusion |

### Node Overlap at Low Zoom

At very low zoom levels (< 0.25x), nodes will overlap when they hit minimum size. This is **acceptable** because:
1. User is in "overview/navigation mode", not editing
2. Clicking on overlapped nodes can use z-order (most recently selected on top)
3. This matches behavior in Blender, Figma, and other professional tools
4. Zooming in resolves overlap naturally

---

## Performance Considerations

| Phase | Impact | Notes |
|-------|--------|-------|
| Phase 2 (LOD) | **High positive** | Removes complex widgets at low zoom; fewer vertices, layouts, draw calls |
| Phase 1 (min sizes) | **Low negative** | Additional `max()` bindings; negligible vs rendering savings |
| Phase 3 (limits) | **Neutral** | Simple clamp operation |

---

## References

- [Rete.js LOD Example](https://retejs.org/examples/lod/)
- [yWorks Level of Detail for Large Diagrams](https://www.yworks.com/pages/level-of-detail-for-large-diagrams)
- [Qt Elastic Nodes Example](https://doc.qt.io/qt-6/qtwidgets-graphicsview-elasticnodes-example.html)
- [Blender Node Editor Zoom Discussion](https://blenderartists.org/t/node-editor-zoom-level/1478930)
- [Canvas High DPI Rendering](https://web.dev/articles/canvas-hidipi)
- [Slint Positioning & Layout](https://docs.slint.dev/latest/docs/slint/guide/language/coding/positioning-and-layouts/)
