# Releasing slint-node-editor 1.0.1

Release date: 2026-09-17. Release branch: `release/1.0.1`. Tag: `v1.0.1`.
Release notes: [1.0.1](releases/1.0.1.md).

## Dependency and distribution contract

- Runtime, compiler, and testing dependencies use registry requirement `1.18.0`
  (Cargo's compatible range), with 1.18.0 resolved in the committed lockfile.
- Rust 1.92 is the declared minimum; Slint uses `compat-1-18`.
- The library selects no backend or renderer. Examples use Skia; the standalone
  consumer uses Winit and the software renderer.
- Slint library modules remain experimental. Consumers enable
  `experimental-module-builds` on slint-build and import `@nodeeditor`.
- Cargo ships both root Slint files, build.rs, Rust sources, licenses, README,
  changelog, contributor notes, and the integration and component guides.
  Integration tests are a separate unpublished workspace package and are
  excluded from the archive.
- The downstream manifest uses registry dependencies. Before publication,
  `smoke/run.sh packaged` overrides only slint-node-editor with the extracted
  archive; all of its Slint dependencies still come from crates.io.
- Breaking public API changes after 1.0 require a new major version and migration
  notes. See [CONTRIBUTING.md](../CONTRIBUTING.md).

## Release verification

Run on the committed candidate:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.92 test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --locked
./smoke/run.sh packaged
cargo publish --dry-run --locked
```

Packaged smoke verifies the actual archive and executes the consumer interaction
suite with default and layout features. The Release verification workflow runs
on `v*` tags or by manual dispatch; it verifies the archive, downstream consumer,
minimum Rust version, and publication dry run without uploading the crate.
Require a green run for the exact tagged revision before publication.

For uncommitted preparation only, use `./smoke/run.sh packaged --allow-dirty`
and `cargo publish --dry-run --locked --allow-dirty`. Repeat strict checks after
committing; a previous candidate's results do not validate the release commit.

## Preparation

1. Commit all 1.0.1 version references, changelog, and release notes on
   `release/1.0.1` and run the strict checks above.
2. Create an annotated `v1.0.1` tag on that commit and push the branch and tag.
3. Create a GitHub draft release for the tag using [the release notes](releases/1.0.1.md).
4. Confirm the Release verification workflow passes for that revision.

Preparation leaves the crate unpublished and the GitHub release in draft.

## Publication

1. From the clean tagged checkout, confirm crates.io credentials and run
   `cargo publish --locked -p slint-node-editor` when publication is authorized.
2. Run `./smoke/run.sh registry` against the published 1.0.1 crate and confirm
   docs.rs builds successfully. Verify that the README served by crates.io
   contains the intended media URLs and that those URLs download successfully.
3. Publish the prepared GitHub release and mark it as the latest release.
4. Merge the release branch into `main` and remove pre-publication fallback
   instructions there. Preserve the release tag.

Keep the one-editor-per-window, accessibility, and performance limitations
visible. Native visual smoke on claimed platforms is separate from headless
interaction tests. The permissive license applies to this library; Slint
retains its own terms.
