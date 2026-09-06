fn main() {
    // `experimental-module-builds` in Cargo.toml resolves `@nodeeditor` from
    // the dependency metadata; no library path is needed here.
    slint_build::compile("ui/app.slint").unwrap();
}
