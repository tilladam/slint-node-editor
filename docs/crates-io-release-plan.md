# Releasing `slint-node-editor` 0.1.0 to crates.io

Plan drafted 2026-09-05, against `aa5a12b` (slint pinned to the 1.18 pre-release tip).
Revised the same day after a Codex second-opinion review; every claim below was
re-verified against the Slint 1.18 source or by running the command shown.
Status section below refreshed 2026-09-05 against `f707f45` + the 1.3/1.4/1.5
commit.

## Executive summary

Publishing this crate is **not** primarily a dependency-pinning problem. It is a
component-distribution problem.

The crate's whole value is the `.slint` components, and **a crates.io consumer
currently has no way to import them.** `README.md:53` tells consumers to pass a
*checkout path* to `with_library_paths`; a registry consumer has no such path.
Solving that — via Slint 1.18's library-module mechanism — is the real work, and
it changes `build.rs`, `Cargo.toml`, the README, and the public import syntax.

Two corrections to the first draft of this plan, both material:

- The earlier "the publish flow works end to end" claim was **wrong**. Because
  `build.rs` is excluded from the package, `cargo publish --dry-run` compiles
  only the Rust library — **neither packaged `.slint` file is ever parsed.** A
  syntactically broken `node-editor.slint` would publish successfully today.
- "Git dependencies are the blocker" was **imprecise**. Cargo accepts
  `{ git, rev, version }` together and strips the git source when packaging;
  this was tested and packaged 21 files successfully. The blocker is the missing
  `version` key, plus the fact that 1.18 is not yet on the registry.

## Current status

Refreshed 2026-09-05. **Phase 1's component-distribution core (1.3, 1.4, 1.5)
has landed**, validated against the git pin rather than the registry — see
"Library modules are not blocked on the tag" below. Phase 2 remains blocked.

### Landed

| Commit | What |
|---|---|
| `aa5a12b` | Slint bumped to the 1.18 pre-release tip `2bb5a20` |
| `af10e02` | Slint library prefix renamed to `@nodeeditor` (dash constraint) |
| `1d584cf` | This plan |
| `fba04a8` | README states our license covers only our code |
| `761fa3b` | Imports use the bare `@nodeeditor` form |
| `eda18d8` | Minimap kept in sync with the graph |
| `ad620cf` | Regression tests for the minimap |
| `f707f45` | This plan brought up to date |
| *(this commit)* | 1.3 + 1.4 + 1.5: library modules, packaged file set, smoke fixture |

Health after 1.3/1.4/1.5: `cargo test --workspace` 354 passed / 0 failed,
`cargo clippy --workspace --all-targets` clean, `./smoke/run.sh` green,
`cargo package --list` correct, and `cargo package` still fails only on the
missing `version` key for `slint`.

Runtime behaviour was verified over the Slint MCP server against the running
`advanced` example: node drag, selection, link creation by pin drag, delete with
link cascade, in-node widgets, and the `@nodeeditor` imports resolving at all.

### Open, unblocked today

- 1.1 (metadata), 1.2 (README registry form + intra-doc links), 1.6 (CI),
  1.7 (docs.rs). 1.3, 1.4 and 1.5 are done.
- `filter_node.slint` text overlap: `'Ctrl'` x=[432,448] collides with
  `'Active'` x=[440,473]; `'In'` and `'Type:'` touch. Measured, not eyeballed.

### Open, blocked

- Phase 2 (2.1 – 2.3) and Phase 3, on the `v1.18.0` tag.

### Open, needs a reproduction first

- During MCP testing, `Node 3` moved from `(648,214)` to `(673.574,175.734)`
  without being dragged — fractional where every other node is integral. Two
  subsequent controlled drags moved only the dragged node, and six samples
  showed the position stable rather than mid-animation. A single clean run on
  1.17 is not enough to blame 1.18. Unexplained; needs a repro before it is
  worth chasing.

### Decided

- License stays `MIT OR Apache-2.0` — see the licensing section below.
- Component distribution uses Slint's experimental library modules — see
  "The central decision".

## Verified facts

Each measured, not assumed. Items marked ⚠️ corrected an earlier assumption.

| Fact | Evidence |
|---|---|
| A dry-run never parses our `.slint` files | `build.rs` excluded ⇒ no Slint compilation during `--verify` |
| ⚠️ Git deps alone do not block packaging | `{ git, rev, version = "1.17.1" }` → `cargo package` → *"Packaged 21 files, 388.0KiB"* |
| The name `slint-node-editor` is free | crates.io API → `crate does not exist` |
| slint 1.18 is not on crates.io yet | crates.io API → `max_version: 1.17.1` |
| slint 1.18 requires **Rust 1.92** | `rust-version = "1.92"`, `pre-release/1.18` workspace `Cargo.toml:83` |
| We currently *claim* Rust 1.70 | `Cargo.toml:14` |
| ⚠️ `compat-1-18` is the mandatory 1.18 baseline, not `compat-1-2` | `"Mandatory feature: required to keep the compatibility with Slint 1.18"` documents **`compat-1-18`**; `compat-1-2 = ["compat-1-18", linuxkms libseat, libinput]` |
| Library modules exist in 1.18 | `as_library()` at `api/rs/build/lib.rs:249`; consumer lookup at `internal/compiler/typeloader.rs:1141-1173` |
| Both sides are experimental | `#[cfg(feature = "experimental-module-builds")]` (publisher), `#[cfg(feature = "experimental-library-module")]` (consumer). Tracking issue slint-ui/slint#154 |
| ⚠️ **A hyphenated library name can never resolve** | Cargo: `links = "my-lib-name"` → `DEP_MY_LIB_NAME_*` (measured). Compiler: `format!("DEP_{}_SLINT_LIBRARY_NAME", name.to_uppercase())` — no dash handling ⇒ looks for `DEP_MY-LIB-NAME_*`, which never exists |
| ⚠️ `cargo package --list` is not a packaging check | Same tree: `--list` → exit 0, `cargo package` → exit 101 |
| Post-PR-#5 code still compiles against 1.17.1 | `cargo publish --dry-run` against 1.17.1 → exit 0 |
| `categories` slugs are valid | crates.io API → `GUI`, `Visualization` |
| ⚠️ Library modules work at the **git pin**, not only at the tag | `experimental-module-builds` at `api/rs/build/Cargo.toml:45`, `as_library` at `api/rs/build/lib.rs:249`, both at `2bb5a20`. 1.3/1.4/1.5 were built and validated against it |
| ⚠️ `as_library` needs MSRV ≥ 1.77 | It emits `cargo::metadata=…`; with `rust-version = "1.70"` cargo refuses: *"the `cargo::` syntax … was added in Rust 1.77.0"*. Forced the MSRV bump early |
| ⚠️ The publisher must expose the generated code as a Rust module | Consumer codegen emits `pub use slint_node_editor::nodeeditor::NodeEditor` (`generator/rust.rs:181-190`), so `src/lib.rs` needs `include!` under a module matching `rust_module` |
| ⚠️ Library-imported **structs and enums** are not re-exported to consumers | `type_exports` covers local types only; `LinkData`/`MinimapNode` land in the consumer's private `slint_generated*` module. Worked around by re-exporting them from our crate root |
| ⚠️ Consumers get `experimental-module-builds` by feature unification | Fixture with the feature *removed* still resolved `@nodeeditor`: build-dep features unify, so our build-dep enables it for them. Reliable but implicit — the fixture still declares it explicitly |
| The smoke fixture actually fails when the mechanism breaks | Negative control: dropping `links = "nodeeditor"` → fixture build script fails. Restored, green again |

## The central decision: library modules

**Chosen approach: Slint 1.18 experimental library modules** (`as_library` +
`links`). This matches upstream's intended design and requires no `build.rs`
from consumers — they just write `import { NodeEditor } from "@nodeeditor";`.

The cost, stated plainly: **every consumer must enable an experimental Slint
compiler feature**, and the API is documented as *"experimental and may change
or be removed in the future."* If it changes, our published crate breaks for
downstream users and needs a new release.

### The library name must not contain dashes

This is the single most important implementation detail, and it is not
documented upstream. Cargo mangles `links` to `DEP_<UPPERCASE, dashes→underscores>_*`,
but `typeloader.rs` builds the lookup key with `.to_uppercase()` only. A name
containing `-` produces a key Cargo never sets, and the import fails to resolve
with no useful diagnostic.

**Use a dash-free library name: `nodeeditor`.** ✅ **Applied 2026-09-05** —
the `with_library_paths` key and every `@`-import were renamed across the root
`build.rs`, all 9 example `build.rs` files, 14 `.slint` files, `README.md`, and
`src/lib.rs`. Verified: 349 tests pass, 0 warnings.

Imports use the bare `@nodeeditor` form. An earlier draft of this plan claimed
the suffix could only drop once `as_library` landed — that was **wrong**.
`find_file_in_library_path` (`typeloader.rs:1755`) handles both shapes:

```rust
// "@library/file.slint" -> "/path/to/library/" + "file.slint"
Some(file) => library_path.join(file),
// "@library"            -> the mapped path IS the file
None => library_path.clone(),
```

Upstream's own `test_library_import` maps `"libfile.slint"` to `lib.slint` and
imports `from "@libfile.slint"`. So mapping the name to the *file*
(`node-editor.slint`) rather than the directory gives the bare-`@name` form on
**stable** `with_library_paths`, today. Applied 2026-09-05.

This is worth noting for Phase 2: the consumer-visible import syntax is already
the one `as_library` produces, so adopting library modules becomes a `build.rs`
change with no further churn in consumer code.

```toml
# Cargo.toml
[package]
links = "nodeeditor"          # NOT "slint-node-editor" — would never resolve
```

```rust
// build.rs
let config = slint_build::CompilerConfiguration::new()
    .as_library("nodeeditor");
slint_build::compile_with_config("node-editor.slint", config).unwrap();
```

```slint
// consumer
import { NodeEditor, BaseNode, Pin, Link, PinTypes } from "@nodeeditor";
```

✅ **The public import syntax is already migrated.** Library modules resolve a
bare `@name` against a single entry file — `typeloader.rs` compares
`library_name == import.strip_prefix('@')`, so a path suffix would not match.
The README, `src/lib.rs`, `tests/ui/test.slint` and all nine examples were moved
to the bare form ahead of time, so this is no longer a blocker for Phase 2.

`node-editor.slint` is a suitable single entry point: line 55 already re-exports
`PinTypes`, `BaseNode`, `Pin`, `Link`, `Minimap`, `LinkData` and the rest, and it
declares `NodeEditor` and `BoxSelectionModifier` itself.

## Phase 1 — do now (not blocked on 1.18)

### 1.1 Fix the wrong metadata

`Cargo.toml` points at upstream Slint's repo, not this one. Both fields render
prominently on the crates.io page.

```toml
repository = "https://github.com/tilladam/slint-node-editor"  # was: slint-ui/slint
homepage   = "https://github.com/tilladam/slint-node-editor"  # was: https://slint.dev
```

### 1.2 Fix the documentation inconsistencies

- ✅ **Done:** `src/lib.rs:69` linked to `github.com/slint-ui/slint/tree/master/…`
  — the wrong repository entirely; now points at `tilladam/slint-node-editor`.
- ✅ **Done:** `src/lib.rs:18` wrote the import **without** the `@` prefix while
  `README.md` wrote it **with**; both now read `@nodeeditor/node-editor.slint`.
- `README.md:46-53` documents a `path = "path/to/slint-node-editor"` dependency.
  Replace with the registry form.
- The `NodeEditor` / `BaseNode` / `Pin` / `Link` / `Minimap` intra-doc links in
  `src/lib.rs` name no Rust symbols and will not resolve. Either point them at
  real items or drop the brackets.

Gate this with `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`.

### 1.3 Restructure `build.rs` for dual duty — ✅ done

Landed, and it needed four things the first draft did not list:

1. **`links = "nodeeditor"` in `[package]`.** Cargo emits `DEP_*` only for
   packages that declare `links`; without it `as_library`'s metadata goes
   nowhere and the lookup at `typeloader.rs:1141` finds nothing.
2. **`experimental-module-builds` on the `slint-build` build-dependency.**
3. **`src/lib.rs` must expose the generated code.** Consumer codegen resolves to
   `slint_node_editor::nodeeditor::<Component>`, so the crate carries
   `pub mod nodeeditor { include!(concat!(env!("OUT_DIR"), "/node-editor.rs")); }`
   matching `rust_module("nodeeditor")` in `build.rs`. Note the explicit
   `include!` rather than `slint::include_modules!()`: both `compile_with_config`
   calls set `SLINT_INCLUDE_GENERATED` and the last one wins, which the test
   harness relies on.
4. **The MSRV had to move to 1.92 now**, not in 2.2 — see the verified-facts
   table. 1.70 made `as_library` fail outright.

The test-UI compile is guarded on `tests/ui/test.slint` existing, so the
packaged crate does not try to build a test UI it does not ship.

**Two consequences worth knowing about.** Both are behaviour changes, not
cosmetics:

- *Library structs and enums do not reach consumers.* Slint 1.18 forwards
  library-imported structs and enums only into the consumer's private
  `slint_generated*` module, so `LinkData` and `MinimapNode` were suddenly
  unnameable. The crate root now re-exports `BoxSelectionModifier`,
  `LinkCreationState`, `LinkData`, `MinimapNode` and `MinimapPosition`. This is
  an upstream gap; if it is fixed, the re-exports become redundant but harmless.
- *`impl LinkModel for LinkData` moved into the library.* Both the type and the
  trait are ours now, so the orphan rule forbids consumers writing it. Two
  examples had a copy each; the canonical impl in `src/graph.rs` reads `color`
  and `status` off the row, which the example copies did not. Nothing observes
  the difference today — those two accessors are only read by `LinkManager`,
  and no example pairs it with `LinkData` — but a consumer that does will now
  get the row's own colour and status instead of white and "no status".

### 1.4 Stop shipping the test UI — ✅ done

`include` is root-anchored (`/*.slint`) and ships `build.rs`, which 1.3 requires.
`cargo package --list` confirms: `tests/ui/test.slint` gone, both root `.slint`
files and `build.rs` present, `smoke/` excluded.

### 1.5 Build the downstream smoke fixture — ✅ done

`smoke/downstream` is a crate outside the workspace; `./smoke/run.sh` builds it.
It compiles `import { NodeEditor, BaseNode, Pin, PinTypes, LinkData } from
"@nodeeditor";` and constructs a `LinkData` in Rust, so it covers both halves of
the contract. Verified to fail when `links` is removed.

Two modes. `path` (the default) builds against this checkout and works today.
`packaged` extracts the `cargo package` tarball first and is blocked on 2.1 —
`cargo package` needs the `version` key on `slint`. **Path mode does not prove
the packaged file set is sufficient**; run `packaged` before publishing.

All nine examples were also stripped of their `with_library_paths` plumbing.
They now resolve `@nodeeditor` exactly as an outside consumer does, which makes
the whole example suite a second, broader smoke test.

### 1.6 Fix the CI coverage

`.github/workflows/rust.yml` runs only `cargo build` and `cargo test`. Note that
`cargo package --list` is **not** a packaging check — it exits 0 on a tree where
`cargo package` exits 101. Add:

- `cargo package --locked` (the real check),
- the 1.5 smoke fixture, built from the extracted archive,
- `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`,
- an MSRV job on a pinned toolchain (see 2.2).

### 1.7 docs.rs configuration

The `layout` feature is off by default, so `rust-sugiyama`-gated items would be
invisible on docs.rs:

```toml
[package.metadata.docs.rs]
all-features = true
```

## Phase 2 — blocked on the `v1.18.0` tag

### 2.1 Swap the git pins to registry versions

There are seven `rev = "..."` strings: five in the root `Cargo.toml` and two in
`smoke/downstream/Cargo.toml`, which is deliberately outside the workspace and
so inherits nothing. Every other crate inherits with `{ workspace = true }`.

```toml
[dependencies]
slint = { version = "1.18", default-features = false, features = ["std", "compat-1-18"] }

[dev-dependencies]
i-slint-backend-testing = { workspace = true }   # inherited, no rev of its own

[build-dependencies]
slint-build = { version = "1.18", features = ["experimental-module-builds"] }

[workspace.dependencies]
slint = { version = "1.18", default-features = false, features = ["std", "compat-1-18", "backend-winit", "renderer-skia"] }
slint-build = { version = "1.18", features = ["experimental-module-builds"] }
i-slint-backend-testing = "1.18"
```

`i-slint-backend-testing` moved into `[workspace.dependencies]` in `ad620cf` so
the root and `examples/advanced` share one entry. The smoke fixture added the
two outside the workspace; it must move to `version = "1.18"` in the same pass
or `packaged` mode will resolve a different slint than the crate under test.

Two changes from the first draft:

- **`compat-1-18`, not `compat-1-2`.** `compat-1-18` carries the *"Mandatory
  feature"* doc for 1.18; `compat-1-2` is the 1.2-era baseline and additionally
  enables legacy LinuxKMS `libseat`/`libinput` that this crate has no use for.
  Since we require `version = "1.18"`, pre-1.18 compatibility is already
  excluded, so `compat-1-2` buys nothing.
- **`experimental-module-builds`** on `slint-build`, required by `as_library`.

Keep the caret range `"1.18"` rather than `=1.18.0`, so cargo unifies our slint
with a consumer's. Note the limit: `"1.18"` means any `<2.0`, and it will *not*
unify with a consumer who pins an exact incompatible version. Because our public
API exposes Slint types, a duplicate slint in the graph is a hard type error, not
just bloat — so test against both the minimum (1.18.0) and the latest allowed
1.x.

This also retires the parked item from the 1.18 bump: the crate stops being
pinned to a moving pre-release branch tip.

### 2.2 Raise the MSRV to 1.92 — ✅ applied early, still needs verifying

The bump already landed with 1.3: `as_library` emits `cargo::metadata=…`, which
cargo rejects below 1.77, so 1.70 blocked the work outright. 1.92 is the honest
floor because slint 1.18 declares it. What remains here is the *verification* on
a pinned toolchain and the release-note wording.

⚠️ A 22-release jump that will exclude consumers on older toolchains. Forced by
slint, not by our code; state it in the release notes.

Verify on a **pinned** toolchain, not whatever is current:

```bash
cargo +1.92 test --workspace --all-features
```

### 2.3 Re-verify

```bash
cargo update                                  # deliberate; review the dep diff
cargo +1.92 test --workspace --all-features
cargo package --locked                        # the real packaging check
# then build the 1.5 smoke fixture from the extracted archive
cargo publish --dry-run --locked
```

Use `--locked` rather than deleting `Cargo.lock`. The lockfile ships inside the
tarball; consumers ignore a library's lock, but the current one is the 1.18
*pre-release* resolution and would embed a git rev with no permanent upstream
name.

## Phase 3 — publication gates

Ordered, and none of them optional:

1. Working tree clean and committed; no `--allow-dirty`.
2. crates.io credentials present and account owns the name (`cargo owner --list`
   after first publish).
3. `cargo publish --locked`.
4. Tag the exact published commit (`v0.1.0`) and push the tag.
5. Write release notes covering the MSRV jump and the experimental-feature
   requirement for consumers.
6. **After** publication, verify against the real registry: fresh `cargo add
   slint-node-editor` in a scratch project, confirm the docs.rs build succeeded,
   and re-run the smoke fixture against the published crate.

## Licensing — decided: keep `MIT OR Apache-2.0`

Settled 2026-09-05. The crate keeps `license = "MIT OR Apache-2.0"`.

Mirroring Slint's own SPDX expression was considered and rejected as legally
incoherent. Slint declares:

```
GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0
```

Two of those three arms cannot be offered by a third party. Reading
`LICENSES/LicenseRef-Slint-Royalty-free-2.0.md`, the text defines **Software** as
Slint itself and names SixtyFPS GmbH as the grantor: *"SixtyFPS hereby grants You
a world-wide, royalty-free, non-exclusive license to use ... the Software."*
Only SixtyFPS can grant those terms. Copying the expression onto this crate would
purport to license someone else's terms for our code.

`GPL-3.0-only` — the one arm we *could* grant — was also considered and not
taken. The consequence to keep in view: our permissive license covers only our
code. Slint's licensing governs anything built with it independently, so a
downstream user is not as unencumbered as `MIT OR Apache-2.0` alone suggests.
Worth a sentence in the README rather than a license change.

## Points of risk

1. **The experimental dependency is the biggest one.** `as_library` may change
   or be removed (issue #154). If upstream changes it, published 0.1.0 breaks
   and needs a new release. This is the accepted cost of the chosen approach.
   One part of the cost turned out smaller than feared: consumers do *not* have
   to enable `experimental-module-builds` themselves — cargo unifies build-dep
   features, so our build-dependency turns it on for them (measured). Implicit
   rather than guaranteed, so the README should still tell them to declare it.
2. **The dash constraint is silent.** A hyphenated library name fails to resolve
   with no useful error. If the import mysteriously fails, check this first.
3. **The 1.18 API is not final.** The pin is a moving branch tip; Phase 2 assumes
   the tag stays compatible with `2bb5a20` — likely but unverified.
4. **The smoke fixture has only run in `path` mode.** It proves the components
   resolve and compile; it does *not* prove the packaged file set is
   sufficient, because `cargo package` cannot run until 2.1. Running
   `./smoke/run.sh packaged` after 2.1 is mandatory.
5. **We now depend on two undocumented details of `as_library`**: that library
   structs/enums need re-exporting by hand, and that build-dep feature
   unification carries the experimental flag to consumers. Either could change
   without notice.
6. **Publishing is irreversible.** 0.1.0 and the name can never be reused.

## Alternatives considered and rejected

- **Stable `links` + hand-written metadata.** Our build script emits
  `cargo::metadata=SLINT_LIBRARY_SOURCE` by hand; consumers read
  `DEP_NODEEDITOR_SLINT_LIBRARY_SOURCE` and pass it to the *stable*
  `with_library_paths`. No experimental features on either side, at the cost of
  ~3 lines of `build.rs` per consumer. Rejected in favour of the upstream
  mechanism, but it is the fallback if #154 moves against us.
- **Publish against 1.17.1 now.** The code compiles against it (measured), so
  this would ship today. Rejected: conflicts with the goal of releasing on 1.18,
  and 1.17's `slint-build` has no `as_library`.

## Not proposed

Renaming the crate, restructuring the workspace, and splitting the examples into
a separate repo. Note that the *library* name (`nodeeditor`) is necessarily
distinct from the *crate* name (`slint-node-editor`) because of the dash
constraint — that is not a crate rename.
