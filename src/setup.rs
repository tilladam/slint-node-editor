//! Simplified setup helpers for NodeEditor with globals architecture.
//!
//! The [`NodeEditorSetup`] provides automatic callback handling. You only need
//! to provide a closure that commits a finished drag into your model.
//!
//! # Example
//!
//! ```ignore
//! use slint_node_editor::{GraphLogic, NodeEditorSetup, wire_node_editor};
//!
//! slint::include_modules!();
//!
//! fn main() {
//!     let window = MainWindow::new().unwrap();
//!
//!     let setup = NodeEditorSetup::new({
//!         let nodes = nodes.clone();
//!         move |dragged, delta_x, delta_y| {
//!             GraphLogic::commit_drag(&nodes, dragged, delta_x, delta_y);
//!         }
//!     });
//!
//!     wire_node_editor!(window, setup);
//!     window.run().unwrap();
//! }
//! ```

use crate::controller::NodeEditorController;
use std::rc::Rc;

/// Setup helper that bundles NodeEditorController and automatic model updates.
///
/// This helper eliminates boilerplate by:
/// - Managing the controller lifecycle
/// - Calling your drag-commit closure automatically when a drag ends
///
/// The setup knows nothing about selection. A finished drag hands your closure
/// the node that was dragged, and committing it — including moving whatever
/// else is selected — is [`GraphLogic::commit_drag`], which reads `selected`
/// off the model rows.
///
/// [`GraphLogic::commit_drag`]: crate::GraphLogic::commit_drag
pub struct NodeEditorSetup<F>
where
    F: Fn(i32, f32, f32) + 'static,
{
    controller: Rc<NodeEditorController>,
    on_drag_committed: Rc<F>,
}

impl<F> NodeEditorSetup<F>
where
    F: Fn(i32, f32, f32) + 'static,
{
    /// Create a new setup helper with a drag-commit callback.
    ///
    /// The callback receives `(dragged_node_id, delta_x, delta_y)` once, when a
    /// node drag ends. This is the ONLY callback you need to provide - everything
    /// else is handled internally.
    pub fn new(on_drag_committed: F) -> Self {
        Self {
            controller: Rc::new(NodeEditorController::new()),
            on_drag_committed: Rc::new(on_drag_committed),
        }
    }

    /// Get the underlying controller for advanced operations.
    pub fn controller(&self) -> &Rc<NodeEditorController> {
        &self.controller
    }

    /// Callback for `NodeEditorInternalCallbacks.on_report_node_rect`.
    pub fn report_node_rect(&self) -> impl Fn(i32, f32, f32, f32, f32) + 'static {
        let ctrl = self.controller.clone();
        move |id, x, y, w, h| {
            ctrl.handle_node_rect(id, x, y, w, h);
        }
    }

    /// Callback for `NodeEditorInternalCallbacks.on_report_pin_position`.
    pub fn report_pin_position(&self) -> impl Fn(i32, i32, i32, f32, f32) + 'static {
        let ctrl = self.controller.clone();
        move |pin_id, node_id, pin_type, x, y| {
            ctrl.handle_pin_position(pin_id, node_id, pin_type, x, y);
        }
    }

    /// Callback for `NodeEditorInternalCallbacks.on_end_node_drag`.
    ///
    /// Calls your drag-commit closure once, with the node the user dragged.
    pub fn end_node_drag(&self) -> impl Fn(i32, f32, f32) + 'static {
        let on_committed = self.on_drag_committed.clone();
        move |node_id, delta_x, delta_y| {
            on_committed(node_id, delta_x, delta_y);
        }
    }
}
