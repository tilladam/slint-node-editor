//! Simplified setup helpers for NodeEditor with globals architecture.
//!
//! The [`NodeEditorSetup`] provides automatic callback handling. You only need
//! to provide a closure that updates your model when nodes are moved.
//!
//! # Example
//!
//! ```ignore
//! use slint_node_editor::NodeEditorSetup;
//! use slint::{Model, VecModel};
//! use std::rc::Rc;
//!
//! slint::include_modules!();
//!
//! fn main() {
//!     let window = MainWindow::new().unwrap();
//!     let nodes = Rc::new(VecModel::from(vec![/* your nodes */]));
//!     
//!     // Create setup with your model update logic
//!     let setup = NodeEditorSetup::new({
//!         let nodes = nodes.clone();
//!         move |node_id, delta_x, delta_y| {
//!             for i in 0..nodes.row_count() {
//!                 if let Some(mut node) = nodes.row_data(i) {
//!                     if node.id == node_id {
//!                         node.x += delta_x;
//!                         node.y += delta_y;
//!                         nodes.set_row_data(i, node);
//!                         break;
//!                     }
//!                 }
//!             }
//!         }
//!     });
//!     
//!     // Wire callbacks (5 lines for complete setup)
//!     let gc = window.global::<GeometryCallbacks>();
//!     gc.on_report_node_rect(setup.report_node_rect());
//!     gc.on_report_pin_position(setup.report_pin_position());
//!     gc.on_start_node_drag(setup.start_node_drag());
//!     gc.on_end_node_drag(setup.end_node_drag());
//!     window.global::<NodeEditorComputations>().on_compute_link_path(setup.compute_link_path());
//!     
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

    /// Get access to the underlying controller for advanced operations.
    pub fn controller(&self) -> &NodeEditorController {
        &self.controller
    }
    
    /// Get access to the selection set for the macro to wire up.
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

    /// Callback for `NodeEditorComputations.on_compute_link_path`.
    pub fn compute_link_path(&self) -> impl Fn(i32, i32, i32, f32, f32, f32) -> slint::SharedString + 'static {
        self.controller.compute_link_path_callback()
    }

    /// Generate the initial grid commands.
    pub fn generate_initial_grid(&self, width: f32, height: f32) -> slint::SharedString {
        self.controller.generate_initial_grid(width, height)
    }

    /// Generate grid commands for the current viewport.
    pub fn generate_grid(&self, width: f32, height: f32, pan_x: f32, pan_y: f32) -> slint::SharedString {
        self.controller.generate_grid(width, height, pan_x, pan_y)
    }

    /// Set the viewport state.
    pub fn set_viewport(&self, zoom: f32, pan_x: f32, pan_y: f32) {
        self.controller.set_viewport(zoom, pan_x, pan_y);
    }
}
