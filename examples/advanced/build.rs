fn main() {
    // No library-path plumbing: slint-node-editor declares itself a Slint
    // library module, so `@nodeeditor` resolves from the dependency's
    // metadata. This is exactly what a crates.io consumer writes.
    //
    // The one addition is debug info, which the ElementHandle API in this
    // example's tests needs in order to locate elements at all.
    let config = slint_build::CompilerConfiguration::new().with_debug_info(true);
    slint_build::compile_with_config("ui/ui.slint", config).unwrap();
}
