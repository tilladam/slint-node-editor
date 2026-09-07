# Contributing

Use the Slint revision pinned in Cargo.toml and Rust 1.92 or newer. Applications
choose their own backend and renderer; the library does not select one.

Before submitting changes, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
./smoke/run.sh included
```

The downstream fixture exercises the public @nodeeditor import and standard
macros. Add interaction regressions there when changing consumer wiring; keep
pure geometry and selection tests in the library. The standalone fixture can
be formatted with `rustfmt --edition 2021 smoke/downstream/src/main.rs`.

## Supported configurations and limitations

Linux CI checks the workspace and the software-rendered downstream fixture.
The workspace examples use Skia. macOS has local test coverage; Windows runtime
behavior is not currently covered by CI. Do not interpret compilation as a
platform accessibility or rendering certification.

Only one editor per window is supported. Selection and document history belong
to the host. Keyboard-only graph editing and structural accessibility remain
incomplete. Example LOD is application code, not automatic virtualization.
No large-graph frame-time guarantee is currently published. Hosts should offer
reduced-motion controls when adopting animated-links in an application.

## Compatibility and releases

Before 1.0, breaking API changes must be recorded in CHANGELOG.md and explained
with migration instructions. Prefer deprecation when an existing API can remain
correct. Changes to callback ownership, units, IDs, and generated Slint types
are API changes even if their Rust signatures are unchanged.

Run the Release verification workflow on the intended release commit before
publishing. It checks the real archive, downstream interaction, minimum Rust
version, and publication dry run. Included-files smoke success alone is not a
release gate. See docs/crates-io-release-plan.md for the current upstream blocker.
