//! Simplified setup helpers for NodeEditor with globals architecture.
//!
//! The [`NodeEditorSetup`] provides automatic callback handling. You only need
//! to provide a closure that updates your model when nodes are moved.
//!
//! # Example
//!
//! ```ignore
//! use slint_node_editor::{NodeEditorSetup, wire_node_editor};
//!
//! slint::include_modules!();
//!
//! fn main() {
//!     let window = MainWindow::new().unwrap();
//!
//!     let setup = NodeEditorSetup::new(|node_id, delta_x, delta_y| {
//!         // Update your node model positions here
//!     });
//!
//!     wire_node_editor!(window, setup);
//!     window.run().unwrap();
//! }
//! ```

use crate::controller::NodeEditorController;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Setup helper that bundles NodeEditorController and automatic model updates.
///
/// This helper eliminates boilerplate by:
/// - Managing the controller lifecycle
/// - Tracking dragged node ID internally
/// - Tracking selection for multi-node drag
/// - Calling your model-update closure automatically on drag end
pub struct NodeEditorSetup<F>
where
    F: Fn(i32, f32, f32) + 'static,
{
    controller: Rc<NodeEditorController>,
    dragged_node_id: Rc<RefCell<i32>>,
    selection: Rc<RefCell<HashSet<i32>>>,
    on_node_moved: Rc<F>,
}

impl<F> NodeEditorSetup<F>
where
    F: Fn(i32, f32, f32) + 'static,
{
    /// Create a new setup helper with a node-moved callback.
    ///
    /// The callback receives `(node_id, delta_x, delta_y)` when a node drag ends.
    /// This is the ONLY callback you need to provide - everything else is handled internally.
    pub fn new(on_node_moved: F) -> Self {
        Self {
            controller: Rc::new(NodeEditorController::new()),
            dragged_node_id: Rc::new(RefCell::new(0i32)),
            selection: Rc::new(RefCell::new(HashSet::new())),
            on_node_moved: Rc::new(on_node_moved),
        }
    }

    /// Get the underlying controller for advanced operations.
    pub fn controller(&self) -> &Rc<NodeEditorController> {
        &self.controller
    }

    #[doc(hidden)]
    /// Internal: exposes selection set for the `wire_node_editor!` macro.
    pub fn selection(&self) -> Rc<RefCell<HashSet<i32>>> {
        self.selection.clone()
    }

    /// Callback for `GeometryCallbacks.on_report_node_rect`.
    pub fn report_node_rect(&self) -> impl Fn(i32, f32, f32, f32, f32) + 'static {
        let ctrl = self.controller.clone();
        move |id, x, y, w, h| {
            ctrl.handle_node_rect(id, x, y, w, h);
        }
    }

    /// Callback for `GeometryCallbacks.on_report_pin_position`.
    pub fn report_pin_position(&self) -> impl Fn(i32, i32, i32, f32, f32) + 'static {
        let ctrl = self.controller.clone();
        move |pin_id, node_id, pin_type, x, y| {
            ctrl.handle_pin_position(pin_id, node_id, pin_type, x, y);
        }
    }

    /// Callback for `GeometryCallbacks.on_start_node_drag`.
    pub fn start_node_drag(&self) -> impl Fn(i32, bool, f32, f32) + 'static {
        let dragged = self.dragged_node_id.clone();
        move |node_id, _, _, _| {
            *dragged.borrow_mut() = node_id;
        }
    }

    /// Callback for `GeometryCallbacks.on_end_node_drag`.
    /// 
    /// This automatically calls your model-update closure with the dragged node ID.
    /// If the dragged node is part of a multi-node selection, all selected nodes are moved.
    pub fn end_node_drag(&self) -> impl Fn(f32, f32) + 'static {
        let dragged = self.dragged_node_id.clone();
        let selection = self.selection.clone();
        let on_moved = self.on_node_moved.clone();
        move |delta_x, delta_y| {
            let node_id = *dragged.borrow();
            let sel = selection.borrow();
            
            // If dragged node is in a multi-node selection, move all selected nodes
            if sel.contains(&node_id) && sel.len() > 1 {
                for &id in sel.iter() {
                    on_moved(id, delta_x, delta_y);
                }
            } else {
                // Single node drag
                on_moved(node_id, delta_x, delta_y);
            }
        }
    }

}
