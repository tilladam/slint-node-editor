# Changelog

## Unreleased — initial 0.1.0

- Distribute Slint components through the `@nodeeditor` library module.
  Consumers enable `experimental-module-builds` on slint-build and require
  Rust 1.92. Dependencies currently use the documented git revisions.
- Routes update after programmatic geometry changes and drag commits; removal
  retires node and pin geometry. Picking uses the rendered world-space curve.
- Normalize connection direction before topology validation and choose the
  nearest eligible pin deterministically.
- Make layout input ordering and disconnected-component packing deterministic.
- Test public consumer gestures and configured marquee behavior.

### Migration from earlier git snapshots

`Link.path-commands` is replaced by `Link.geometry: LinkPath`, containing commands
relative to its bounding box. Custom routes must supply that box.
`compute_link_path_callback` now takes a LinkPath constructor.

Use NodeEditorComputations for computations and NodeEditorEvents for BaseNode
double-clicks. Use the current quick-start macros and generated global exports;
removed component callbacks are not compatibility aliases. Selection remains
host-owned and must be projected synchronously before a drag proceeds.

### Release limitations

Publication is blocked until the required Slint library-module support is
available from crates.io and actual archive verification succeeds. Independent
editors in one window, complete keyboard accessibility, and measured large-graph
performance remain future work. See CONTRIBUTING.md for configuration coverage.
