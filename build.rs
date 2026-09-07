fn main() {
    // Always: compile node-editor.slint as a Slint library module. This emits the
    // DEP_NODEEDITOR_SLINT_LIBRARY_* metadata (via the `links = "nodeeditor"` key)
    // that lets a consumer write `import { NodeEditor } from "@nodeeditor";` with
    // no build.rs plumbing of its own. The generated Rust lands in OUT_DIR and is
    // included by `src/lib.rs` under a module matching `rust_module` below —
    // consumers reference it as `slint_node_editor::nodeeditor::<Component>`.
    let library_config = slint_build::CompilerConfiguration::new()
        .as_library("nodeeditor")
        .rust_module("nodeeditor");
    slint_build::compile_with_config("node-editor.slint", library_config).unwrap();
}
