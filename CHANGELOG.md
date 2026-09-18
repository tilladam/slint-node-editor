# Changelog

## Unreleased

- Add `NodeEditorComputations.viewport-resized(width, height)` so hosts can
  track editor dimensions independently of grid updates, preserving the
  existing `viewport-changed` signature (#6).
- Share cancellation and gesture globals across the `@nodeeditor` module
  boundary so public removal and reset hooks cancel consumer interactions (#7).

## 1.0.1 — 2026-09-17

Documentation-only patch. The library API and implementation are unchanged.

- Replace the broken README demo attachment with the verified GitHub release
  video and use explicit, versioned screenshot URLs on crates.io and docs.rs.
- Remove pre-publication setup instructions from the published README and guide.

## 1.0.0 — 2026-09-16

First stable release. Requires Slint 1.18.0 or newer and Rust 1.92 or newer.

- Distribute Slint components through the `@nodeeditor` library module.
  Consumers enable `experimental-module-builds` on slint-build and require
  Rust 1.92. Slint 1.18.0 is the minimum supported registry release.
- Routes update after programmatic geometry changes and drag commits; removal
  retires node and pin geometry. Picking uses the rendered world-space curve.
- Normalize connection direction before topology validation and choose the
  nearest eligible pin deterministically.
- Make layout input ordering and disconnected-component packing deterministic.
- Test public consumer gestures and configured marquee behavior.
- Focus the editor on canvas, node, pin, minimap, and reserved marquee presses,
  while preserving embedded text input and application-directed focus changes.
  Adapted from Olivier de Gaalon's focus fix.
- Reconcile rejected and snapped drag commits to the synchronous host model.
- Reject invalid controller viewport updates and require both normalization
  endpoints to exist. Add LinkData::new for explicitly colored links and
  standard Error support for ValidationError.

### Migration from earlier git snapshots

Windows using `wire_node_editor!` must expose
`public function focus-editor() { editor.focus(); }`. Manual integrations must
connect `NodeEditorInternalCallbacks.take-editor-focus` synchronously to that
function. See the integration guide for the complete window interface.

`Link.path-commands` is replaced by `Link.geometry: LinkPath`, containing commands
relative to its bounding box. Custom routes must supply that box.
`compute_link_path_callback` now takes a LinkPath constructor.

Use NodeEditorComputations for computations and NodeEditorEvents for BaseNode
double-clicks. Use the current quick-start macros and generated global exports;
removed component callbacks are not compatibility aliases. Selection remains
host-owned and must be projected synchronously before a drag proceeds.

### Release limitations

Independent editors in one window, complete keyboard accessibility, and
measured large-graph performance remain future work. See CONTRIBUTING.md for configuration coverage.
