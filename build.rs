fn main() {
    // For integration tests, compile the test UI
    let mut library_paths = std::collections::HashMap::new();
    library_paths.insert("slint-node-editor".into(), "./".into());

    let config = slint_build::CompilerConfiguration::default().with_library_paths(library_paths);
    slint_build::compile_with_config("tests/ui/test.slint", config).unwrap();
}
