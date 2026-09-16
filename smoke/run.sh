#!/usr/bin/env bash
# Downstream smoke test: test the compiled quick-start crate outside this
# workspace against slint-node-editor.
#
# Cargo's package verification runs build.rs and parses the library's Slint
# files. This additional consumer verifies that @nodeeditor resolves across
# the dependency boundary and that the public interaction wiring works.
#
#   ./smoke/run.sh            # path mode: against this checkout
#   ./smoke/run.sh registry   # published dependency documented in README
#   ./smoke/run.sh included   # against only the files `include` would ship
#   ./smoke/run.sh packaged   # against an extracted `cargo package` tarball
#
# Path mode cannot tell "the .slint files ship" from "they happen to be on this
# disk", because SLINT_LIBRARY_SOURCE is an absolute path into the library's
# manifest dir. `included` closes that: it copies exactly what
# `cargo package --list` reports and builds against the copy. Prefer
# `packaged` to verify the actual archive and its normalized manifest.
#
# The fixture is deliberately unlocked (smoke/downstream/Cargo.lock is
# gitignored), so it re-resolves on every run. A failure here can therefore be
# dependency drift rather than a regression in this crate — check the
# resolution before assuming the library broke.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mode="${1:-path}"
package_args=(--locked)
# Explicit opt-in for reviewing uncommitted release preparation. CI stays strict.
if [[ "${2:-}" == "--allow-dirty" ]]; then
    package_args+=(--allow-dirty)
elif [[ -n "${2:-}" ]]; then
    echo "unknown option: $2" >&2
    exit 2
fi

case "$mode" in
path)
    library="$root"
    ;;
registry)
    library=""
    ;;
included)
    # cargo's own include filter, without needing `cargo package` to succeed.
    staged_library="$root/target/smoke-included"
    rm -rf "$staged_library"
    mkdir -p "$staged_library"
    cargo package --list --allow-dirty --manifest-path "$root/Cargo.toml" \
        | while read -r file; do
            # Skip the entries cargo synthesises rather than copies.
            case "$file" in
            .cargo_vcs_info.json | Cargo.toml.orig | Cargo.lock) continue ;;
            esac
            mkdir -p "$staged_library/$(dirname "$file")"
            cp "$root/$file" "$staged_library/$file"
        done
    library="$staged_library"
    ;;
packaged)
    cargo package "${package_args[@]}" --manifest-path "$root/Cargo.toml"
    version="$(cargo metadata --no-deps --format-version 1 --manifest-path "$root/Cargo.toml" \
        | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "slint-node-editor"))')"
    rm -rf "$root/target/smoke-package"
    mkdir -p "$root/target/smoke-package"
    tar -xzf "$root/target/package/slint-node-editor-$version.crate" \
        -C "$root/target/smoke-package"
    library="$root/target/smoke-package/slint-node-editor-$version"
    ;;
*)
    echo "usage: $0 [path|registry|included|packaged]" >&2
    exit 2
    ;;
esac

# Build a staged copy so the dependency can be repointed without editing the
# committed manifest.
staging="$root/target/smoke-fixture"
rm -rf "$staging"
mkdir -p "$staging"
cp -R "$root/smoke/downstream/." "$staging/"
if [[ "$mode" != "registry" ]]; then
python3 - "$staging/Cargo.toml" "$library" <<'PY'
import sys
manifest, library = sys.argv[1], sys.argv[2]
text = open(manifest).read()
old = 'slint-node-editor = "0.1.0"'
assert old in text, "fixture manifest no longer declares the documented registry dependency"
open(manifest, 'w').write(
    text.replace(old, f'slint-node-editor = {{ version = "0.1.0", path = "{library}" }}'))
PY
    source_description="$library"
else
    source_description="the published registry versions"
fi

echo "smoke: testing downstream quick start against $source_description"
cargo test --manifest-path "$staging/Cargo.toml" --target-dir "$root/target/smoke-target"
cargo test --manifest-path "$staging/Cargo.toml" --target-dir "$root/target/smoke-target" --features layout
echo "smoke: ok ($mode mode; default and layout features)"
