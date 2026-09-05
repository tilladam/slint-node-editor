fn main() {
    // No library-path plumbing: slint-node-editor declares itself a Slint
    // library module, so `@nodeeditor` resolves from the dependency's
    // metadata. This is exactly what a crates.io consumer writes.
    slint_build::compile("ui/edge-fade.slint").unwrap();
}
