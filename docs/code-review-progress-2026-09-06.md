# Code review progress — 2026-09-06

This document tracks work against the findings in
[code-review-2026-09-06.md](code-review-2026-09-06.md). Update it when a finding
is started, completed, reopened, or intentionally deferred.

Last updated: 2026-09-07 through R10.

## Status

- **Complete:** implementation and acceptance coverage are in place.
- **In progress:** work has started but the finding's acceptance criteria are
  not fully met.
- **Open:** no corrective implementation has started.
- **Deferred:** intentionally postponed, with the reason recorded here.

| ID | Priority | Status | Delivery batch | Commit(s) | Progress |
|---|---|---|---|---|---|
| R1 | P1 | Complete | 1. Correctness | `83f7145`, `b097948` | Geometry changes now invalidate link bindings after updating the cache. Programmatic movement, sizing, pointer dragging, drag commits, and batched updates have regression coverage. |
| R2 | P1 | Complete | 1. Correctness | `1363ce7` | Added explicit node, pin, and graph-reset lifecycle operations; identity replacement; cache and registered-link cleanup; interaction-state cleanup; advanced-example deletion wiring; and a tested hidden-pin policy. |
| R3 | P1 | Complete | 1. Correctness | `ff274c9` | Default picking now uses the rendered world curve and converts screen tolerance once. Custom routes can supply their rendered geometry; the orthogonal example shares one route with its picker. |
| R4 | P1 | Complete | 2. Interaction and public contract | `0b454a9` | Globals are canonical for computations and building-block events; component configuration synchronizes with the controller; public geometry functions use the lifecycle; obsolete members were removed or documented as host conveniences. |
| R5 | P1 | Complete | 2. Interaction and public contract | `5a4b483` | The quick start is a tested downstream crate using exact git dependencies; its docs cover generated members, ownership, callback replacement, units, IDs, and lifecycle. |
| R6 | P2 | Complete | 1. Correctness | `28eeaa3` | Link validation now returns canonical output/input endpoints before topology rules run; the advanced and pin-compatibility examples create links from those validated endpoints. |
| R7 | P2 | Complete | 2. Interaction and public contract | `e6e9aa5` | A modifier-only editor overlay owns forced marquees across every surface; node and pin fallbacks share the configuration, and all transient interactions have cancellation paths. |
| R8 | P2 | Complete | 1. Correctness | `b63767e` | Pin picking scans all eligible candidates for the nearest and resolves exact distance ties by lowest pin ID, independent of insertion order. |
| R9 | P2 | Complete | 2. Interaction and public contract | `0fe0b7e` | Raw and cache layouts canonicalize unique nodes and valid edges, return node-ID order, and pack deterministically ordered components without overlap on the perpendicular axis. |
| R10 | P1 | Complete | 2. Interaction and public contract | `f235b25` | The external `@nodeeditor` consumer drives real drag, link creation, edge selection, and double-click gestures through standard macros at transformed coordinates; self-confirming interaction tests were removed or replaced. |
| R11 | P2 | In progress | 5. Scale and optional UX | `1542652` | Unselected nodes no longer subscribe to the global drag start/end toggle through their world-position bindings; frame baselines and broader update localization remain open. |
| R12 | P1 roadmap | Open | 4. Embeddability | — | Implement structural accessibility and configurable keyboard policy. |
| R13 | P2 roadmap | Open | 4. Embeddability | — | Introduce instance-scoped editor context. |
| R14 | P2 | In progress | 3/4 | This change | Reconcile rejected drag commits, guard controller viewport updates, check both normalization endpoints, add colored LinkData constructor and Error support, and clarify ID/geometry/selection contracts. Broader API consolidation remains deferred. |
| R15 | P1 release | In progress | 3. Release hardening | This change | Release verification workflow added; actual packaging remains blocked on published Slint 1.18 library-module support. |
| R16 | P3 | In progress | 3. Release hardening | This change | Fixed pin example sizing and target feedback, added drag regression, fmt/clippy/all-feature CI, changelog, contributor guidance and bug template. Fixture separation and broader example coverage remain. |

Overall: **10 of 16 findings complete**. Correctness batch: **5 of 5
findings complete**. Interaction and public contract batch: **5 of 5 findings
complete**. The next active finding by review order is **R11**.

## Completed work

### R1 — invalidate links when geometry changes

Completed in `83f7145` and `b097948`.

- Geometry reports update the Rust cache before requesting route invalidation.
- `NodeEditor` coalesces geometry requests into one link-version update per
  event-loop turn.
- `BaseNode` covers programmatic world-position changes, size changes, live
  dragging, and the final model projection after a drag commit.
- Hosts that seed geometry directly retain an explicit refresh contract.
- Tests cover independent x/y/size changes, a non-unit zoom and pan pointer
  drag, drag commit, and a batched layout-style update.

### R2 — add an explicit removal and topology lifecycle

Completed in `1363ce7`.

- `GeometryCache` and `NodeEditorController` expose pin removal, cascading node
  removal, and graph reset operations.
- Node removal prunes controller-registered hit-test links and clears matching
  drag state. The Slint lifecycle also clears relevant hover and active
  link-creation state.
- `BaseNode` and `Pin` retire their previously reported identity before
  publishing a replacement. Pins also republish changes to owner and type.
- Graph reset clears disposable geometry and republishes live component
  geometry, including reused IDs.
- The advanced example resolves connected logical links while ownership is
  still available, retires projected state, then removes model rows.
- Hidden pins remain valid topology and routing endpoints but are excluded from
  pin hit testing; making them visible restores picking.
- Tests cover deletion and re-addition, ID reuse, pin removal, retyping,
  reparenting, graph reset, cache counts, validation, picking, logical-link
  cascade, registered-link cleanup, and interaction-state cleanup.

### R3 — use the rendered world curve for picking

Completed in `ff274c9`.

- Default link picking constructs the same zoom-independent world-space
  `CubicBezier` as rendering.
- The screen-space facade converts both the pointer and its pixel tolerance to
  world space exactly once; lower-level APIs name and document their units.
- `LinkRoute`, `BezierLinkRoute`, and `PolylineLinkRoute` let custom renderers
  expose the route that was actually drawn instead of falling back to the
  default Bézier shape.
- The advanced example and integration harness use the corrected world picker.
  The custom-shapes example derives rendering and picking from the same
  Manhattan vertices and enables interactive link hit testing.
- Regression tests cover points on the rendered curve and nearby misses at
  zoom 0.1, 0.25, 1, and 3 with nonzero pan, plus short, vertical, reversed,
  and orthogonal routes.

### R4 — reconcile public component APIs with globals

Completed in `0b454a9`.

- `NodeEditorComputations` is the canonical computation surface. It now owns
  grid update requests and the synchronized grid-spacing and Bézier settings
  consumed by `wire_node_editor!` and `NodeEditorController`.
- The component `geometry-version` aliases the global version used by link
  bindings. Public node and pin reporting functions call the same internal
  lifecycle as BaseNode and Pin before requesting invalidation.
- BaseNode double-clicks use the new public `NodeEditorEvents` global. Obsolete
  component computation/report callbacks and unused internal callbacks were
  removed.
- The unused BaseNode viewport width/height inputs were removed. Node minimums
  and LOD thresholds are documented as values hosts may pass to custom node
  delegates, rather than behavior supplied by NodeEditor.
- The minimal example acts as an external consumer fixture. It sets nondefault
  configuration, applies the documented host-side LOD and minimum-size values,
  observes the resulting controller/grid/path/node geometry, exercises public
  reporting and pin computation, and receives a real BaseNode double-click
  through the public event global.

### R5 — replace the quick start with a compiled consumer

Completed in `5a4b483`.

- The downstream smoke crate is now the complete quick-start application. It
  resolves `@nodeeditor` through dependency metadata and pins the currently
  usable node-editor and Slint git revisions.
- Its UI and Rust host create nodes, project selection synchronously, commit
  movement, validate and normalize connections, reject duplicates, remove
  connected links, retire geometry, and add nodes through visible controls.
- A headless downstream test exercises selection, movement, connection in both
  directions, deletion with link/cache cleanup, and node creation.
- The README links the four tested files and documents macro imports and
  generated members, single-handler replacement semantics, coordinate units,
  integer ID domains, host ownership, and graph/geometry lifecycle.
- Obsolete component wiring was removed from the controller, tracker, and link
  manager documentation. Complete Rust examples are executable doctests;
  generated-UI fragments are labeled and linked to the downstream fixture.

### R6 — normalize endpoints before duplicate checking

Completed in `28eeaa3`.

- `validate_and_normalize_link` performs basic endpoint checks, normalizes the
  gesture to output/input order, and only then calls topology validators.
- Successful validation returns a `NormalizedLink` with named output and input
  fields, so hosts create the exact logical edge their policies approved.
- The advanced example uses the operation for link creation. Its real callback
  test creates a connection in one direction, retries it in reverse, and
  verifies that exactly one canonical link exists.
- The pin-compatibility example now applies type compatibility and duplicate
  checks to normalized endpoints and uses the returned endpoints for creation.

### R7 — arbitrate gestures consistently across surfaces

Completed in `e6e9aa5`.

- A topmost TouchArea is enabled only while the configured marquee modifier is
  held. It gives forced box selection one owner over the background, links,
  nodes, pins, and controls embedded inside consumer nodes.
- `BaseNode` and `Pin` use the same published modifier configuration as a
  fallback when focus was outside the editor during the modifier key press.
  Control and Command/Meta share the `ctrl` setting.
- With no reserved gesture, normal link selection, node dragging, pin linking,
  and embedded-control input continue unchanged.
- Pointer cancellation, outside release, focus loss, hiding, disabling, and
  the public pre-removal hook reset marquee, drag, and link state. Cancellation
  does not commit a marquee or allow the remaining pointer sequence to restart
  a node drag.
- Interaction tests cover all four settings over all five surfaces and each
  cancellation path.

### R8 — pick the nearest pin deterministically

Completed in `b63767e`.

- `find_pin_at` now scans every candidate within the radius and compares
  squared distances, avoiding unnecessary square roots.
- Exact distance ties choose the lowest pin ID, so results do not depend on
  iterator or `HashMap` insertion order.
- Cache and controller pickers exclude non-hit-testable pins. `Pin.enabled`
  now controls both its pointer input and its cache eligibility, matching the
  existing hidden-pin behavior while retaining both as link route endpoints.
- Regression tests cover overlapping hit radii, reversed input order,
  equidistant pins, hidden pins, disabled pins, and screen-space lookup.

### R9 — normalize layout inputs and make ordering intentional

Completed in this commit.

- A single first-wins node table now supplies IDs, validated dimensions,
  algorithm vertices, reverse lookup, and component bounds. Invalid dimensions
  are omitted before edges are resolved.
- Nodes and retained edges are sorted by ID, while duplicate edges, self-loops,
  and unknown endpoints are ignored consistently by raw and cache entry points.
- Components are ordered by their lowest node ID. Top-to-bottom layouts pack
  components along x; left-to-right layouts pack them along y. Returned
  positions are sorted by node ID.
- Regression tests cover conflicting duplicate sizes, invalid dimensions,
  noisy edges, equivalent input permutations, cache insertion order, isolated
  nodes, and non-overlap of disconnected components in both directions.

### R10 — test through the promised interfaces

Completed in this commit.

- The out-of-workspace quick-start fixture imports the library through
  `@nodeeditor` and retains only `wire_node_editor!` and `wire_selection!` for
  standard editor wiring.
- Its testing-backend suite sends real pointer events for node dragging, pin
  completion, edge selection, and BaseNode double-click forwarding. Drag and
  link creation assert final host-model state at zoom 1.5, nonzero pan, and the
  toolbar-offset editor origin.
- The fixture enables link selection and supplies the documented host-owned
  link picker, so the edge gesture exercises the same public callback boundary
  as a consumer application.
- Level 2 drag tests now render nodes and commit movement through pointer input.
  No-op keyboard dispatch checks and tests that manually pushed values into
  callback trackers were removed; command policy remains host-owned and is
  tracked separately under R12.
- Harness node and pin helpers now honor the editor origin, zoom, and pan when
  returning the screen coordinates their API promised.
- CI explicitly compiles and runs all executable documentation examples with
  all features before generating documentation, while the downstream smoke
  step compiles the generated-UI quick-start fixture.

## In-progress work

### R14 — focused contract correctness

- Drag commits reconcile to the synchronous host model even for rejected moves
  or unchanged snapped axes; pointer tests cover both and the next gesture.
- Controller viewport updates reject nonfinite or nonpositive zoom and nonfinite
  pan atomically. Raw geometry helpers remain caller-validated, as documented.
- Direction normalization now requires both pins to exist; ValidationError
  implements Error. LinkData::new supplies explicit-color defaults.
- README states ID domains, synchronous commit timing, supported numeric inputs,
  and endpoint-only box-selection semantics.
- Broader ownership consolidation, adapters, cache encapsulation, and typed IDs
  remain deferred. This does not mark the whole R14 finding complete.


### R16 — examples and maintenance gates

- Pin-compatibility nodes now use BaseNode dimensions; a real lower-body drag
  regression passes. Target feedback is bound to the current connection gesture.
- CI runs formatting, clippy with warnings denied, and all-feature workspace
  tests. Existing Rust formatting was normalized to establish the gate.
- Added changelog, compatibility/contributor guidance, supported-configuration
  limitations, and a bug report template. README lists all nine examples and
  accurately describes the optional layout dependency.
- Corrected stale link-status comments.
- Validation: workspace tests and 14 doctests passed; the additional lower-body
  pointer regression passed. Fixture extraction from the library build script
  and broader example interaction coverage remain open. The focused R14
  contract pass has not started.


### R15 — release packaging gates

- Added `release-check.yml`, runnable manually and on version tags. It requires
  actual package verification and downstream interaction tests against the
  extracted archive, all-feature tests on Rust 1.92, and a publication dry run.
  It does not upload a crate.
- Corrected packaged-smoke version lookup to select the library by name rather
  than assuming Cargo metadata lists it first.
- Registry verification on 2026-09-07 reports Slint 1.17.1; the 1.18.0 API
  lookup returned 404. The git package's `version = "1.18.0"` is not evidence
  of publication. The existing tested git dependency remains necessary.
- `cargo package --locked` still fails because Slint has no registry version
  requirement. Adding an unavailable version would not complete this gate.
  R15 remains incomplete until the required crates are published, dependencies
  and consumer are switched together, and the release workflow passes.


### R11 — measure the whole frame and make updates local

Started in this commit.

- `BaseNode.world-pos-x/y` now read `selected` before the global drag state.
  Slint's short-circuit evaluation therefore keeps unselected nodes from
  subscribing to drag start/end changes that cannot move them.
- The selected nodes and directly dragged node retain their existing live
  geometry behavior. Frame baselines, route-local invalidation, and the other
  measured optimizations required by R11 remain open.

## Verification

After R2:

- `cargo test --workspace --all-features --locked`: **393 tests passed**; 18
  doctests remained ignored.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed before the R2 commit.

After R3:

- `cargo test --workspace --all-features --locked`: **398 tests passed**; 18
  doctests remained ignored.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed.

After R4:

- `cargo test --workspace --all-features --locked`: **402 tests passed**; 18
  doctests remained ignored.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed.

After R5:

- `cargo test --workspace --all-features --locked`: **402 tests passed** plus
  **13 executable doctests passed**, with no ignored doctests.
- `./smoke/run.sh git`: the exact documented git consumer passed its end-to-end
  edit test outside the workspace.
- Workspace and downstream `cargo clippy --all-targets ... -- -D warnings`:
  passed.
- `git diff --check`: passed.

After R6:

- `cargo test --workspace --all-features --locked`: **406 tests passed** plus
  **14 executable doctests passed**.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed.

After the initial R11 update-locality fix:

- `cargo test --workspace --all-features --locked`: **406 tests passed** plus
  **14 executable doctests passed**.
- `git diff --check`: passed.

After R7:

- `cargo test --workspace --all-features --locked`: **417 tests passed** plus
  **14 executable doctests passed**.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed.

After R8:

- `cargo test --workspace --all-features --locked`: **420 tests passed** plus
  **14 executable doctests passed**.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed.

After R9:

- `cargo test --workspace --all-features --locked`: **425 tests passed** plus
  **14 executable doctests passed**.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed.

After R10:

- `cargo test --workspace --all-features --locked`: **411 retained tests
  passed** plus **14 executable doctests passed**.
- `./smoke/run.sh included`: **2 downstream interaction tests passed** against
  only the files Cargo would include in the package.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed.
- `git diff --check`: passed.

The original review's packaging, formatting, platform, accessibility, and
performance limitations remain open unless their corresponding finding is
marked complete above.
