fn main() {
    // Resolve the same dependency-provided module used by downstream consumers.
    slint_build::compile("tests/ui/test.slint").unwrap();
}
