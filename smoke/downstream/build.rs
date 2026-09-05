fn main() {
    // A consumer needs `experimental-module-builds` on slint-build (see
    // Cargo.toml) but writes no library-path plumbing of its own.
    slint_build::compile("ui/app.slint").unwrap();
}
