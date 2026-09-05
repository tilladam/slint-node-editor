#!/usr/bin/env bash
# Downstream smoke test: build a crate that is not in this workspace against
# slint-node-editor and make it compile `import … from "@nodeeditor";`.
#
# This is the check `cargo publish --dry-run` cannot do. A dry-run compiles
# only the Rust library; it never parses a single .slint file, so a broken
# component — or a library name that silently fails to resolve — passes it.
#
#   ./smoke/run.sh            # path mode: against this checkout
#   ./smoke/run.sh packaged   # against an extracted `cargo package` tarball
#
# Packaged mode is blocked until slint 1.18 is on crates.io: `cargo package`
# needs a `version` key on the slint dependency (release plan, phase 2.1).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mode="${1:-path}"

case "$mode" in
path)
    library="$root"
    ;;
packaged)
    cargo package --locked --manifest-path "$root/Cargo.toml"
    version="$(cargo metadata --no-deps --format-version 1 --manifest-path "$root/Cargo.toml" \
        | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
    rm -rf "$root/target/smoke-package"
    mkdir -p "$root/target/smoke-package"
    tar -xzf "$root/target/package/slint-node-editor-$version.crate" \
        -C "$root/target/smoke-package"
    library="$root/target/smoke-package/slint-node-editor-$version"
    ;;
*)
    echo "usage: $0 [path|packaged]" >&2
    exit 2
    ;;
esac

# Build a staged copy so the dependency can be repointed without editing the
# committed manifest.
staging="$root/target/smoke-fixture"
rm -rf "$staging"
mkdir -p "$staging"
cp -R "$root/smoke/downstream/." "$staging/"
python3 - "$staging/Cargo.toml" "$library" <<'PY'
import sys
manifest, library = sys.argv[1], sys.argv[2]
text = open(manifest).read()
old = 'slint-node-editor = { path = "../.." }'
assert old in text, "fixture manifest no longer declares the path dependency"
open(manifest, 'w').write(
    text.replace(old, f'slint-node-editor = {{ path = "{library}" }}'))
PY

echo "smoke: building downstream fixture against $library"
cargo build --manifest-path "$staging/Cargo.toml"
echo "smoke: ok ($mode mode)"
