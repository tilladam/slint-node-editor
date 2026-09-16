# Changelog

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
- Reconcile rejected and snapped drag commits to the synchronous host model.
- Reject invalid controller viewport updates and require both normalization
  endpoints to exist. Add LinkData::new for explicitly colored links and
  standard Error support for ValidationError.

### Migration from earlier git snapshots

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
