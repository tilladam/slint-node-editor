# Code review progress — 2026-09-06

This document tracks work against the findings in
[code-review-2026-09-06.md](code-review-2026-09-06.md). Update it when a finding
is started, completed, reopened, or intentionally deferred.

Last updated: 2026-09-06 through R6.

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
| R6 | P2 | Complete | 1. Correctness | This commit | Link validation now returns canonical output/input endpoints before topology rules run; the advanced and pin-compatibility examples create links from those validated endpoints. |
| R7 | P2 | Open | 2. Interaction and public contract | — | Apply gesture ownership and configured modifiers consistently. |
| R8 | P2 | Open | 1. Correctness | — | Pick the nearest eligible pin with a deterministic tie rule. |
| R9 | P2 | Open | 2. Interaction and public contract | — | Normalize layout inputs and make result ordering deterministic. |
| R10 | P1 | Open | 2. Interaction and public contract | — | Replace self-confirming tests with tests through promised interfaces. |
| R11 | P2 | Open | 5. Scale and optional UX | — | Measure complete frame behavior and localize expensive updates. |
| R12 | P1 roadmap | Open | 4. Embeddability | — | Implement structural accessibility and configurable keyboard policy. |
| R13 | P2 roadmap | Open | 4. Embeddability | — | Introduce instance-scoped editor context. |
| R14 | P2 | Open | 3/4 | — | Simplify ownership, typing, and the public Rust integration surface. |
| R15 | P1 release | Open | 3. Release hardening | — | Complete packaging gates and document the currently usable dependency source. |
| R16 | P3 | Open | 3. Release hardening | — | Clean up examples and make maintenance checks reliable teaching material. |

Overall: **6 of 16 findings complete**. Correctness batch: **4 of 5
findings complete**. Interaction and public contract batch: **2 of 5 findings
complete**. The next item by review order is **R7**.

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

Completed in this commit.

- `validate_and_normalize_link` performs basic endpoint checks, normalizes the
  gesture to output/input order, and only then calls topology validators.
- Successful validation returns a `NormalizedLink` with named output and input
  fields, so hosts create the exact logical edge their policies approved.
- The advanced example uses the operation for link creation. Its real callback
  test creates a connection in one direction, retries it in reverse, and
  verifies that exactly one canonical link exists.
- The pin-compatibility example now applies type compatibility and duplicate
  checks to normalized endpoints and uses the returned endpoints for creation.

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

The original review's packaging, formatting, platform, accessibility, and
performance limitations remain open unless their corresponding finding is
marked complete above.
