//! Simplified setup helpers for NodeEditor with globals architecture.
//!
//! The [`NodeEditorSetup`] eliminates boilerplate by providing pre-configured closures
//! that you can directly wire to the globals. This reduces the typical 40+ lines of
//! callback wiring to just a few lines.
//!
//! # Example
//!
//! ```ignore
//! use slint_node_editor::NodeEditorSetup;
//!
//! slint::include_modules!();
//!
//! fn main() {
//!     let window = MainWindow::new().unwrap();
//!     let nodes = Rc::new(VecModel::from(vec![/* your nodes */]));
//!     
//!     // Create setup helper
//!     let setup = NodeEditorSetup::new();
//!     
//!     // Wire geometry callbacks
//!     window.global::<GeometryCallbacks>().on_report_node_rect(setup.on_report_node_rect());
//!     window.global::<GeometryCallbacks>().on_report_pin_position(setup.on_report_pin_position());
//!     window.global::<GeometryCallbacks>().on_start_node_drag(setup.on_start_node_drag());
//!     window.global::<GeometryCallbacks>().on_end_node_drag(setup.on_end_node_drag(|node_id, dx, dy| {
//!         // Update your model
//!     }));
//!     
//!     // Wire computations
//!     window.global::<NodeEditorComputations>().on_compute_link_path(setup.on_compute_link_path());
//!     
//!     window.run().unwrap();
//! }
//! ```

use crate::controller::NodeEditorController;
use std::cell::RefCell;
use std::rc::Rc;

/// Setup helper that bundles NodeEditorController and common state.
///
/// This helper reduces boilerplate by:
/// - Managing the controller lifecycle
/// - Tracking dragged node ID
/// - Providing pre-configured closures for all callbacks
pub struct NodeEditorSetup {
    controller: Rc<NodeEditorController>,
    dragged_node_id: Rc<RefCell<i32>>,
}

impl Default for NodeEditorSetup {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeEditorSetup {
    /// Create a new setup helper with a fresh controller and state.
    pub fn new() -> Self {
        Self {
            controller: Rc::new(NodeEditorController::new()),
            dragged_node_id: Rc::new(RefCell::new(0i32)),
        }
    }

    /// Get access to the underlying controller for advanced operations.
    pub fn controller(&self) -> &NodeEditorController {
        &self.controller
    }

    /// Get the ID of the currently dragged node (0 if none).
    pub fn dragged_node_id(&self) -> i32 {
        *self.dragged_node_id.borrow()
    }

    /// Create a closure for `GeometryCallbacks.on_report_node_rect`.
    ///
    /// Wire this directly:
    /// ```ignore
    /// window.global::<GeometryCallbacks>()
    ///     .on_report_node_rect(setup.on_report_node_rect());
    /// ```
    pub fn on_report_node_rect(&self) -> impl Fn(i32, f32, f32, f32, f32) + 'static {
        let ctrl = self.controller.clone();
        move |id, x, y, width, h| {
            ctrl.handle_node_rect(id, x, y, width, h);
        }
    }

    /// Create a closure for `GeometryCallbacks.on_report_pin_position`.
    ///
    /// Wire this directly:
    /// ```ignore
    /// window.global::<GeometryCallbacks>()
    ///     .on_report_pin_position(setup.on_report_pin_position());
    /// ```
    pub fn on_report_pin_position(&self) -> impl Fn(i32, i32, i32, f32, f32) + 'static {
        let ctrl = self.controller.clone();
        move |pin_id, node_id, pin_type, x, y| {
            ctrl.handle_pin_position(pin_id, node_id, pin_type, x, y);
        }
    }

    /// Create a closure for `GeometryCallbacks.on_start_node_drag`.
    ///
    /// Wire this directly:
    /// ```ignore
    /// window.global::<GeometryCallbacks>()
    ///     .on_start_node_drag(setup.on_start_node_drag());
    /// ```
    pub fn on_start_node_drag(&self) -> impl Fn(i32, bool, f32, f32) + 'static {
        let dragged = self.dragged_node_id.clone();
        move |node_id, _, _, _| {
            *dragged.borrow_mut() = node_id;
        }
    }

    /// Create a closure for `GeometryCallbacks.on_end_node_drag`.
    ///
    /// This takes a user callback that receives `(node_id, delta_x, delta_y)`
    /// for updating the node model.
    ///
    /// # Example
    /// ```ignore
    /// window.global::<GeometryCallbacks>().on_end_node_drag(
    ///     setup.on_end_node_drag({
    ///         let nodes = nodes.clone();
    ///         move |node_id, dx, dy| {
    ///             // Update your model here
    ///         }
    ///     })
    /// );
    /// ```
    pub fn on_end_node_drag<F>(&self, update_model: F) -> impl Fn(f32, f32) + 'static
    where
        F: Fn(i32, f32, f32) + 'static,
    {
        let dragged = self.dragged_node_id.clone();
        move |delta_x, delta_y| {
            let node_id = *dragged.borrow();
            update_model(node_id, delta_x, delta_y);
        }
    }

    /// Create a closure for `NodeEditorComputations.on_compute_link_path`.
    ///
    /// Wire this directly:
    /// ```ignore
    /// window.global::<NodeEditorComputations>()
    ///     .on_compute_link_path(setup.on_compute_link_path());
    /// ```
    pub fn on_compute_link_path(&self) -> impl Fn(i32, i32, i32, f32, f32, f32) -> slint::SharedString + 'static {
        self.controller.compute_link_path_callback()
    }

    /// Create a closure for `NodeEditorComputations.on_viewport_changed`.
    ///
    /// This handles viewport updates and grid regeneration.
    ///
    /// # Example
    /// ```ignore
    /// window.global::<NodeEditorComputations>().on_viewport_changed(
    ///     setup.on_viewport_changed(&window.as_weak(), |w| {
    ///         (w.get_width_(), w.get_height_())
    ///     })
    /// );
    /// ```
    pub fn on_viewport_changed<W, F>(
        &self,
        window: &slint::Weak<W>,
        get_dimensions: F,
    ) -> impl Fn(f32, f32, f32) + 'static
    where
        W: slint::ComponentHandle + 'static,
        F: Fn(&W) -> (f32, f32) + 'static,
    {
        let ctrl = self.controller.clone();
        let w = window.clone();
        move |zoom, pan_x, pan_y| {
            ctrl.set_viewport(zoom, pan_x, pan_y);
            if let Some(window) = w.upgrade() {
                let (_width, _height) = get_dimensions(&window);
                let _grid = ctrl.generate_grid(_width, _height, pan_x, pan_y);
                // Note: Can't set grid_commands here without knowing the window type
                // Caller must do: w.set_grid_commands(grid);
            }
        }
    }
}
