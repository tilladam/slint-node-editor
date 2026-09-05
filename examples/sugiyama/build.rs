fn main() {
    let mut library_paths = std::collections::HashMap::new();
    library_paths.insert("nodeeditor".into(), "../../".into());

    let config = slint_build::CompilerConfiguration::default()
        .with_library_paths(library_paths);
    slint_build::compile_with_config("ui/sugiyama.slint", config).unwrap();
}
