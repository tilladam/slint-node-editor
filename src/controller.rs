//! High-level controller for node editor applications.
//!
//! The [`NodeEditorController`] reduces boilerplate by managing geometry tracking,
//! link path computation, and viewport state in one place.
//!
//! # Example
//!
//! ```ignore
//! use slint_node_editor::NodeEditorController;
//!
//! slint::include_modules!();
//!
//! fn main() {
//!     let window = MainWindow::new().unwrap();
//!     let ctrl = NodeEditorController::new();
//!     let w = window.as_weak();
//!
//!     // Core callbacks - controller handles the logic
//!     window.on_compute_link_path(ctrl.compute_link_path_callback());
//!     window.on_node_drag_started(ctrl.node_drag_started_callback());
//!
//!     // Geometry tracking
//!     window.on_node_rect_changed({
//!         let ctrl = ctrl.clone();
//!         move |id, x, y, w, h| ctrl.handle_node_rect(id, x, y, w, h)
//!     });
//!
//!     window.on_pin_position_changed({
//!         let ctrl = ctrl.clone();
//!         move |pid, nid, ptype, x, y| ctrl.handle_pin_position(pid, nid, ptype, x, y)
//!     });
//!
//!     // Grid updates
//!     window.on_request_grid_update({
//!         let ctrl = ctrl.clone();
//!         let w = w.clone();
//!         move || {
//!             if let Some(w) = w.upgrade() {
//!                 w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
//!             }
//!         }
//!     });
//!
//!     window.on_update_viewport({
//!         let ctrl = ctrl.clone();
//!         let w = w.clone();
//!         move |z, pan_x, pan_y| {
//!             if let Some(w) = w.upgrade() {
//!                 ctrl.set_zoom(z);
//!                 w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
//!             }
//!         }
//!     });
//!
//!     // App-specific: update node positions after drag
//!     window.on_node_drag_ended({
//!         let ctrl = ctrl.clone();
//!         move |delta_x, delta_y| {
//!             let node_id = ctrl.dragged_node_id();
//!             // Update your node model here
//!         }
//!     });
//!
//!     window.invoke_request_grid_update();
//!     window.run().unwrap();
//! }
//! ```

use crate::state::GeometryCache;
use slint::SharedString;
use std::cell::RefCell;
use std::rc::Rc;

/// Controller that manages node editor state and provides callback implementations.
///
/// This provides a high-level API that handles:
/// - Geometry caching (node rects, pin positions)
/// - Link path computation
/// - Viewport/zoom tracking
/// - Drag tracking
///
/// Clone this controller to share it across callbacks.
#[derive(Clone)]
pub struct NodeEditorController {
    cache: Rc<RefCell<GeometryCache>>,
    zoom: Rc<RefCell<f32>>,
    bezier_offset: Rc<RefCell<f32>>,
    dragged_node_id: Rc<RefCell<i32>>,
    grid_spacing: Rc<RefCell<f32>>,
}

impl Default for NodeEditorController {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeEditorController {
    /// Create a new controller with default settings.
    pub fn new() -> Self {
        Self {
            cache: Rc::new(RefCell::new(GeometryCache::new())),
            zoom: Rc::new(RefCell::new(1.0)),
            bezier_offset: Rc::new(RefCell::new(50.0)),
            dragged_node_id: Rc::new(RefCell::new(0)),
            grid_spacing: Rc::new(RefCell::new(24.0)),
        }
    }

    /// Set the bezier curve offset for link paths (default: 50.0).
    pub fn set_bezier_offset(&self, offset: f32) {
        *self.bezier_offset.borrow_mut() = offset;
    }

    /// Set the grid spacing (default: 24.0).
    pub fn set_grid_spacing(&self, spacing: f32) {
        *self.grid_spacing.borrow_mut() = spacing;
    }

    /// Get the current zoom level.
    pub fn zoom(&self) -> f32 {
        *self.zoom.borrow()
    }

    /// Get access to the geometry cache.
    pub fn cache(&self) -> Rc<RefCell<GeometryCache>> {
        self.cache.clone()
    }

    /// Get the ID of the node currently being dragged (0 if none).
    pub fn dragged_node_id(&self) -> i32 {
        *self.dragged_node_id.borrow()
    }

    // === Callback factories ===

    /// Returns a callback for `compute-link-path`.
    pub fn compute_link_path_callback(&self) -> impl Fn(i32, i32, i32) -> SharedString {
        let cache = self.cache.clone();
        let zoom = self.zoom.clone();
        let bezier_offset = self.bezier_offset.clone();
        move |start_pin, end_pin, _version| {
            cache
                .borrow()
                .compute_link_path(start_pin, end_pin, *zoom.borrow(), *bezier_offset.borrow())
                .unwrap_or_default()
                .into()
        }
    }

    /// Returns a callback for `node-drag-started`.
    pub fn node_drag_started_callback(&self) -> impl Fn(i32) {
        let dragged_node_id = self.dragged_node_id.clone();
        move |node_id| {
            *dragged_node_id.borrow_mut() = node_id;
        }
    }

    // === Direct handlers ===

    /// Handle node-rect-changed: update cache.
    pub fn handle_node_rect(&self, id: i32, x: f32, y: f32, w: f32, h: f32) {
        self.cache.borrow_mut().handle_node_rect_report(id, x, y, w, h);
    }

    /// Handle pin-position-changed: update cache.
    pub fn handle_pin_position(&self, pid: i32, nid: i32, ptype: i32, x: f32, y: f32) {
        self.cache.borrow_mut().handle_pin_report(pid, nid, ptype, x, y);
    }

    /// Handle node-drag-started: track the dragged node.
    pub fn handle_node_drag_started(&self, node_id: i32) {
        *self.dragged_node_id.borrow_mut() = node_id;
    }

    /// Set the zoom level (called from update-viewport).
    pub fn set_zoom(&self, zoom: f32) {
        *self.zoom.borrow_mut() = zoom;
    }

    /// Compute link path for given pins.
    pub fn compute_link_path(&self, start_pin: i32, end_pin: i32) -> SharedString {
        self.cache
            .borrow()
            .compute_link_path(
                start_pin,
                end_pin,
                *self.zoom.borrow(),
                *self.bezier_offset.borrow(),
            )
            .unwrap_or_default()
            .into()
    }

    /// Generate grid commands for current viewport.
    pub fn generate_grid(&self, width: f32, height: f32, pan_x: f32, pan_y: f32) -> SharedString {
        crate::generate_grid_commands(
            width,
            height,
            *self.zoom.borrow(),
            pan_x,
            pan_y,
            *self.grid_spacing.borrow(),
        )
        .into()
    }

    /// Generate initial grid commands (zoom=1, pan=0).
    pub fn generate_initial_grid(&self, width: f32, height: f32) -> SharedString {
        crate::generate_grid_commands(width, height, 1.0, 0.0, 0.0, *self.grid_spacing.borrow()).into()
    }
}
