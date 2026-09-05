use std::path::Path;

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

    // Only in a git checkout: compile the UI the integration tests drive. The
    // published tarball does not ship tests/, so guard on the file's presence.
    //
    // This must stay *after* the library compile: both calls set
    // `SLINT_INCLUDE_GENERATED`, last one wins, and `tests/common/harness.rs`
    // is the caller of `slint::include_modules!()`. `src/lib.rs` includes its
    // generated file by explicit path for the same reason.
    let test_ui = Path::new("tests/ui/test.slint");
    if test_ui.exists() {
        let mut library_paths = std::collections::HashMap::new();
        library_paths.insert("nodeeditor".into(), "./node-editor.slint".into());

        let config =
            slint_build::CompilerConfiguration::default().with_library_paths(library_paths);
        slint_build::compile_with_config(test_ui, config).unwrap();
    }
}
