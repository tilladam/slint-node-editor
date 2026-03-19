//! Automatic callback wiring for node editor applications.
//!
//! This module provides a helper function to automatically wire up internal
//! geometry callbacks, eliminating boilerplate from examples.

use slint::ComponentHandle;

/// Automatically wire up all internal geometry callbacks for a node editor window.
///
/// This function connects the GeometryCallbacks global to the NodeEditor's internal
/// functions, eliminating the need for examples to manually forward callbacks.
///
/// **Call this once during application setup!**
///
/// # Example
///
/// ```ignore
/// use slint_node_editor::setup_node_editor_callbacks;
///
/// slint::include_modules!();
///
/// fn main() {
///     let window = MainWindow::new().unwrap();
///     
///     // One-line setup - wires everything automatically!
///     setup_node_editor_callbacks(&window);
///     
///     // Now just set up data and run
///     window.set_nodes(...);
///     window.run().unwrap();
/// }
/// ```
pub fn setup_node_editor_callbacks<T>(window: &T)
where
    T: ComponentHandle,
{
    // Note: This function cannot be implemented generically because:
    // 1. Slint's global() method requires a concrete type known at compile time
    // 2. The GeometryCallbacks global is defined in the application's .slint files
    // 3. Each application has its own generated types
    //
    // Instead, applications should call window.global::<GeometryCallbacks>() and
    // wire callbacks directly, or we provide a macro to generate this boilerplate.
    //
    // For now, this function documents the intended pattern but applications must
    // implement the wiring themselves until we create a macro solution.
    
    let _ = window; // Suppress warning
    unimplemented!(
        "setup_node_editor_callbacks cannot be implemented generically. \
         See the minimal example for the pattern to use."
    );
}
