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
use crate::hit_test::{find_link_at, NodeGeometry, SimpleLinkGeometry};
use slint::SharedString;
use std::cell::RefCell;
use std::rc::Rc;

/// Controller that manages node editor state and provides callback implementations.
///
/// This provides a high-level API that handles:
/// - Geometry caching (node rects, pin positions) in **world space**
/// - Link path computation (world→screen conversion done internally)
/// - Hit testing facades (screen-space input, world-space internals)
/// - Viewport/zoom/pan tracking
/// - Drag tracking
///
/// The cache stores node rects in world coordinates, which are invariant to
/// zoom/pan changes. This eliminates the O(N) viewport resync loop that was
/// previously needed when zoom or pan changed.
///
/// Clone this controller to share it across callbacks.
#[derive(Clone)]
pub struct NodeEditorController {
    cache: Rc<RefCell<GeometryCache>>,
    zoom: Rc<RefCell<f32>>,
    pan_x: Rc<RefCell<f32>>,
    pan_y: Rc<RefCell<f32>>,
    bezier_offset: Rc<RefCell<f32>>,
    dragged_node_id: Rc<RefCell<i32>>,
    grid_spacing: Rc<RefCell<f32>>,
    links: Rc<RefCell<Vec<(i32, i32, i32)>>>,
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
            pan_x: Rc::new(RefCell::new(0.0)),
            pan_y: Rc::new(RefCell::new(0.0)),
            bezier_offset: Rc::new(RefCell::new(50.0)),
            dragged_node_id: Rc::new(RefCell::new(0)),
            grid_spacing: Rc::new(RefCell::new(24.0)),
            links: Rc::new(RefCell::new(Vec::new())),
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
    ///
    /// Computes screen-space bezier paths from world-space cache data.
    pub fn compute_link_path_callback(&self) -> impl Fn(i32, i32, i32) -> SharedString {
        let cache = self.cache.clone();
        let zoom = self.zoom.clone();
        let pan_x = self.pan_x.clone();
        let pan_y = self.pan_y.clone();
        let bezier_offset = self.bezier_offset.clone();
        move |start_pin, end_pin, _version| {
            cache
                .borrow()
                .compute_link_path_screen(
                    start_pin,
                    end_pin,
                    *zoom.borrow(),
                    *pan_x.borrow(),
                    *pan_y.borrow(),
                    *bezier_offset.borrow(),
                )
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

    /// Handle node-rect-changed: convert screen→world and update cache.
    ///
    /// The UI reports node rects in screen coordinates. This method converts
    /// to world coordinates before caching, making the cache zoom/pan invariant.
    pub fn handle_node_rect(&self, id: i32, x: f32, y: f32, w: f32, h: f32) {
        let zoom = *self.zoom.borrow();
        let pan_x = *self.pan_x.borrow();
        let pan_y = *self.pan_y.borrow();
        let z = if zoom > 0.0 { zoom } else { 1.0 };
        let world_x = (x - pan_x) / z;
        let world_y = (y - pan_y) / z;
        let world_w = w / z;
        let world_h = h / z;
        self.cache
            .borrow_mut()
            .handle_node_rect_report(id, world_x, world_y, world_w, world_h);
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

    /// Set viewport state: zoom, pan_x, pan_y.
    ///
    /// This replaces the old pattern of calling `set_zoom` followed by an O(N)
    /// loop to resync all node positions. Since the cache stores world-space
    /// coordinates, changing zoom/pan requires no per-node updates.
    pub fn set_viewport(&self, zoom: f32, pan_x: f32, pan_y: f32) {
        *self.zoom.borrow_mut() = zoom;
        *self.pan_x.borrow_mut() = pan_x;
        *self.pan_y.borrow_mut() = pan_y;
    }

    /// Register a link for hit testing.
    pub fn register_link(&self, id: i32, start_pin: i32, end_pin: i32) {
        self.links.borrow_mut().push((id, start_pin, end_pin));
    }

    /// Unregister a link by ID.
    pub fn unregister_link(&self, id: i32) {
        self.links.borrow_mut().retain(|&(lid, _, _)| lid != id);
    }

    /// Clear all registered links.
    pub fn clear_links(&self) {
        self.links.borrow_mut().clear();
    }

    /// Compute link path for given pins (screen-space output from world-space cache).
    pub fn compute_link_path(&self, start_pin: i32, end_pin: i32) -> SharedString {
        self.cache
            .borrow()
            .compute_link_path_screen(
                start_pin,
                end_pin,
                *self.zoom.borrow(),
                *self.pan_x.borrow(),
                *self.pan_y.borrow(),
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

    // === Screen-space hit-testing facades ===
    //
    // These methods accept screen-space mouse coordinates and handle all
    // coordinate conversion internally using the stored viewport state.

    /// Find the link closest to the given screen-space position.
    ///
    /// Returns the link ID, or -1 if no link is within `hover_distance`.
    /// Internally converts world-space cache data to screen space for accurate
    /// bezier hit testing that matches the rendered curves.
    pub fn find_link_at_screen(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        hover_distance: f32,
        bezier_min_offset: f32,
        hit_samples: usize,
    ) -> i32 {
        let zoom = *self.zoom.borrow();
        let pan_x = *self.pan_x.borrow();
        let pan_y = *self.pan_y.borrow();
        let cache = self.cache.borrow();
        let links = self.links.borrow();

        let link_geometries: Vec<SimpleLinkGeometry> = links
            .iter()
            .filter_map(|&(id, start_pin, end_pin)| {
                let start_pos = cache.pin_positions.get(&start_pin)?;
                let end_pos = cache.pin_positions.get(&end_pin)?;
                let start_rect = cache.node_rects.get(&start_pos.node_id)?.rect();
                let end_rect = cache.node_rects.get(&end_pos.node_id)?.rect();

                // World→screen: (node_world + pin_rel) * zoom + pan
                Some(SimpleLinkGeometry {
                    id,
                    start_x: (start_rect.0 + start_pos.rel_x) * zoom + pan_x,
                    start_y: (start_rect.1 + start_pos.rel_y) * zoom + pan_y,
                    end_x: (end_rect.0 + end_pos.rel_x) * zoom + pan_x,
                    end_y: (end_rect.1 + end_pos.rel_y) * zoom + pan_y,
                })
            })
            .collect();

        find_link_at(
            mouse_x,
            mouse_y,
            link_geometries,
            hover_distance,
            zoom,
            bezier_min_offset,
            hit_samples,
        )
    }

    /// Find the pin closest to the given screen-space position.
    ///
    /// Returns the pin ID, or 0 if no pin is within `hit_radius`.
    pub fn find_pin_at_screen(&self, mouse_x: f32, mouse_y: f32, hit_radius: f32) -> i32 {
        let zoom = *self.zoom.borrow();
        let pan_x = *self.pan_x.borrow();
        let pan_y = *self.pan_y.borrow();
        let cache = self.cache.borrow();

        let pins = cache.pin_positions.iter().filter_map(|(&pin_id, pin)| {
            let rect = cache.node_rects.get(&pin.node_id)?.rect();
            // World→screen: (node_world + pin_rel) * zoom + pan
            let sx = (rect.0 + pin.rel_x) * zoom + pan_x;
            let sy = (rect.1 + pin.rel_y) * zoom + pan_y;
            Some(crate::hit_test::SimplePinGeometry {
                id: pin_id,
                x: sx,
                y: sy,
            })
        });

        crate::hit_test::find_pin_at(mouse_x, mouse_y, pins, hit_radius)
    }

    /// Find all nodes whose world-space rect intersects the given screen-space selection box.
    ///
    /// Converts the selection box from screen→world and performs AABB intersection.
    pub fn nodes_in_selection_box_screen(
        &self,
        sx: f32,
        sy: f32,
        sw: f32,
        sh: f32,
    ) -> Vec<i32> {
        let zoom = *self.zoom.borrow();
        let pan_x = *self.pan_x.borrow();
        let pan_y = *self.pan_y.borrow();
        let z = if zoom > 0.0 { zoom } else { 1.0 };

        let world_x = (sx - pan_x) / z;
        let world_y = (sy - pan_y) / z;
        let world_w = sw / z;
        let world_h = sh / z;

        self.cache
            .borrow()
            .nodes_in_selection_box(world_x, world_y, world_w, world_h)
    }

    /// Find all links that have at least one endpoint inside the given screen-space selection box.
    ///
    /// Converts the box and link endpoints to world space for comparison.
    pub fn links_in_selection_box_screen(
        &self,
        sx: f32,
        sy: f32,
        sw: f32,
        sh: f32,
    ) -> Vec<i32> {
        let zoom = *self.zoom.borrow();
        let pan_x = *self.pan_x.borrow();
        let pan_y = *self.pan_y.borrow();
        let z = if zoom > 0.0 { zoom } else { 1.0 };

        let world_x = (sx - pan_x) / z;
        let world_y = (sy - pan_y) / z;
        let world_w = sw / z;
        let world_h = sh / z;

        let cache = self.cache.borrow();
        let links = self.links.borrow();

        // Compute world-space link endpoints: node_world + pin_rel
        let link_geometries: Vec<SimpleLinkGeometry> = links
            .iter()
            .filter_map(|&(id, start_pin, end_pin)| {
                let start_pos = cache.pin_positions.get(&start_pin)?;
                let end_pos = cache.pin_positions.get(&end_pin)?;
                let start_rect = cache.node_rects.get(&start_pos.node_id)?.rect();
                let end_rect = cache.node_rects.get(&end_pos.node_id)?.rect();

                Some(SimpleLinkGeometry {
                    id,
                    start_x: start_rect.0 + start_pos.rel_x,
                    start_y: start_rect.1 + start_pos.rel_y,
                    end_x: end_rect.0 + end_pos.rel_x,
                    end_y: end_rect.1 + end_pos.rel_y,
                })
            })
            .collect();

        crate::hit_test::links_in_selection_box(
            world_x,
            world_y,
            world_w,
            world_h,
            link_geometries,
        )
    }
}
