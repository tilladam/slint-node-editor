# Releasing slint-node-editor 0.1.0

Updated 2026-09-16. Slint 1.18.0 is published and the registry migration is
implemented. The historical pre-release plan is available in Git history.

## Dependency and distribution contract

- Runtime, compiler, and testing dependencies use registry requirement `1.18.0`
  (Cargo's compatible range), with 1.18.0 resolved in the committed lockfile.
- Rust 1.92 remains the declared minimum; `compat-1-18` replaces the older
  compatibility feature and removes its implicit Linux input dependencies.
- The library selects no backend or renderer. Examples use Skia; the standalone
  consumer uses Winit and the software renderer.
- Slint library modules remain experimental. Consumers enable
  `experimental-module-builds` on slint-build and import `@nodeeditor`.
- Cargo ships both root Slint files, build.rs, Rust sources, licenses, README,
  changelog, and contributor notes. Integration tests are a separate unpublished
  workspace package and are excluded from the archive.
- The downstream manifest uses registry dependencies. Before first publication,
  `smoke/run.sh packaged` overrides only slint-node-editor with the extracted
  archive; all of its Slint dependencies still come from crates.io.

## Release preparation checks

Run on the candidate checkout:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.92 test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --locked
./smoke/run.sh packaged
cargo publish --dry-run --locked
```

Packaged smoke verifies the actual archive and executes the consumer interaction
suite with default and layout features. Ordinary CI now runs packaged smoke;
the release workflow also checks Rust 1.92 and a publication dry run.

During review of uncommitted changes, use `./smoke/run.sh packaged --allow-dirty`
and `cargo publish --dry-run --locked --allow-dirty`. These validate the working
tree, not a final release commit. Repeat the strict commands after committing.

Validated on 2026-09-16: Rust 1.92 passed 416 workspace tests and 14 doctests;
formatting, clippy, and documentation passed with warnings denied where applicable.
Cargo verified the archive, both downstream configurations passed their two
interaction tests, and the publication dry run succeeded. Archive and dry-run
checks used `--allow-dirty` for this uncommitted candidate.

## Publication checklist

1. Commit the reviewed candidate and verify the release workflow is green for
   that exact revision; repeat strict packaging without `--allow-dirty`.
2. Review CHANGELOG.md and record the release date. Keep the one-editor-per-window,
   accessibility, and performance limitations visible. Native visual smoke on
   the claimed platforms is separate from headless interaction tests.
3. Confirm registry credentials and the crate name, then explicitly authorize
   and run `cargo publish --locked`.
4. Tag the published commit `v0.1.0`, push the tag, and create release notes from
   CHANGELOG.md.
5. Run `./smoke/run.sh registry` against the actual published crate and confirm
   docs.rs succeeds. Remove README's pre-publication path fallback then.

No publication, tag, or registry ownership change is part of preparation.
The permissive license applies to this library; Slint retains its own terms.
