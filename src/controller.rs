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
//!                 ctrl.set_viewport(z, pan_x, pan_y);
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
use std::collections::HashMap;
use std::rc::Rc;

/// Viewport and configuration state, behind a single `Rc<RefCell<_>>`.
///
/// The geometry cache is kept separate (its own `Rc<RefCell<_>>`) because
/// it is exposed to callers via [`NodeEditorController::cache()`].
struct ViewportState {
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    bezier_offset: f32,
    dragged_node_id: i32,
    grid_spacing: f32,
    /// Links registered for hit testing, keyed by link ID.
    links: HashMap<i32, (i32, i32)>,
}

impl ViewportState {
    fn new() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            bezier_offset: 50.0,
            dragged_node_id: 0,
            grid_spacing: 24.0,
            links: HashMap::new(),
        }
    }

    /// Clamp zoom to a safe positive value.
    fn safe_zoom(&self) -> f32 {
        if self.zoom > 0.0 { self.zoom } else { 1.0 }
    }
}

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
    state: Rc<RefCell<ViewportState>>,
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
            state: Rc::new(RefCell::new(ViewportState::new())),
        }
    }

    /// Set the bezier curve offset for link paths (default: 50.0).
    pub fn set_bezier_offset(&self, offset: f32) {
        self.state.borrow_mut().bezier_offset = offset;
    }

    /// Set the grid spacing (default: 24.0).
    pub fn set_grid_spacing(&self, spacing: f32) {
        self.state.borrow_mut().grid_spacing = spacing;
    }

    /// Get the current zoom level.
    pub fn zoom(&self) -> f32 {
        self.state.borrow().zoom
    }

    /// Get access to the geometry cache.
    pub fn cache(&self) -> Rc<RefCell<GeometryCache>> {
        self.cache.clone()
    }

    /// Get the ID of the node currently being dragged (0 if none).
    pub fn dragged_node_id(&self) -> i32 {
        self.state.borrow().dragged_node_id
    }

    // === Callback factories ===

    /// Returns a callback for `compute-link-path`.
    ///
    /// Computes screen-space bezier paths from world-space cache data.
    pub fn compute_link_path_callback(&self) -> impl Fn(i32, i32, i32) -> SharedString {
        let cache = self.cache.clone();
        let state = self.state.clone();
        move |start_pin, end_pin, _version| {
            let s = state.borrow();
            cache
                .borrow()
                .compute_link_path_screen(
                    start_pin,
                    end_pin,
                    s.zoom,
                    s.pan_x,
                    s.pan_y,
                    s.bezier_offset,
                )
                .unwrap_or_default()
                .into()
        }
    }

    /// Returns a callback for `node-drag-started`.
    pub fn node_drag_started_callback(&self) -> impl Fn(i32) {
        let state = self.state.clone();
        move |node_id| {
            state.borrow_mut().dragged_node_id = node_id;
        }
    }

    // === Direct handlers ===

    /// Handle node-rect-changed: convert screen→world and update cache.
    ///
    /// The UI reports node rects in screen coordinates. This method converts
    /// to world coordinates before caching, making the cache zoom/pan invariant.
    pub fn handle_node_rect(&self, id: i32, x: f32, y: f32, w: f32, h: f32) {
        let s = self.state.borrow();
        let z = s.safe_zoom();
        let world_x = (x - s.pan_x) / z;
        let world_y = (y - s.pan_y) / z;
        let world_w = w / z;
        let world_h = h / z;
        drop(s);
        self.cache
            .borrow_mut()
            .handle_node_rect_report(id, world_x, world_y, world_w, world_h);
    }

    /// Handle pin-position-changed: update cache.
    ///
    /// The Slint `Pin` component reports `rel_x`/`rel_y` as **world-space**
    /// offsets relative to the node origin. The Pin component divides by zoom
    /// internally (`center-x: (self.x + self.width / 2) / zoom`), so the
    /// values received here are already zoom-invariant.
    pub fn handle_pin_position(&self, pid: i32, nid: i32, ptype: i32, x: f32, y: f32) {
        self.cache.borrow_mut().handle_pin_report(pid, nid, ptype, x, y);
    }

    /// Seed a node's world-space rect directly, bypassing screen→world conversion.
    ///
    /// Use this to pre-populate the geometry cache for nodes that haven't been
    /// rendered by Slint yet (e.g. off-screen nodes whose link endpoints are
    /// visible). The coordinates must be in world space.
    pub fn seed_node_world_rect(&self, id: i32, x: f32, y: f32, w: f32, h: f32) {
        self.cache
            .borrow_mut()
            .handle_node_rect_report(id, x, y, w, h);
    }

    /// Handle node-drag-started: track the dragged node.
    pub fn handle_node_drag_started(&self, node_id: i32) {
        self.state.borrow_mut().dragged_node_id = node_id;
    }

    /// Set the zoom level (called from update-viewport).
    #[deprecated(since = "0.2.0", note = "Use set_viewport() which also updates pan state")]
    pub fn set_zoom(&self, zoom: f32) {
        self.state.borrow_mut().zoom = zoom;
    }

    /// Set viewport state: zoom, pan_x, pan_y.
    ///
    /// Since the cache stores world-space coordinates, changing zoom/pan
    /// requires no per-node updates.
    pub fn set_viewport(&self, zoom: f32, pan_x: f32, pan_y: f32) {
        let mut s = self.state.borrow_mut();
        s.zoom = zoom;
        s.pan_x = pan_x;
        s.pan_y = pan_y;
    }

    /// Register a link for hit testing. Idempotent: re-registering the same ID
    /// updates the pin pair.
    pub fn register_link(&self, id: i32, start_pin: i32, end_pin: i32) {
        self.state.borrow_mut().links.insert(id, (start_pin, end_pin));
    }

    /// Unregister a link by ID.
    pub fn unregister_link(&self, id: i32) {
        self.state.borrow_mut().links.remove(&id);
    }

    /// Clear all registered links.
    pub fn clear_links(&self) {
        self.state.borrow_mut().links.clear();
    }

    /// Clear the geometry cache (node rects and pin positions).
    ///
    /// Call this when navigating between subgraphs to prevent stale
    /// pin-to-node associations from producing incorrect link paths.
    pub fn clear_geometry(&self) {
        let mut cache = self.cache.borrow_mut();
        cache.node_rects.clear();
        cache.pin_positions.clear();
    }

    /// Compute link path for given pins (screen-space output from world-space cache).
    pub fn compute_link_path(&self, start_pin: i32, end_pin: i32) -> SharedString {
        let s = self.state.borrow();
        self.cache
            .borrow()
            .compute_link_path_screen(
                start_pin,
                end_pin,
                s.zoom,
                s.pan_x,
                s.pan_y,
                s.bezier_offset,
            )
            .unwrap_or_default()
            .into()
    }

    /// Generate grid commands for current viewport.
    pub fn generate_grid(&self, width: f32, height: f32, pan_x: f32, pan_y: f32) -> SharedString {
        let s = self.state.borrow();
        crate::generate_grid_commands(width, height, s.zoom, pan_x, pan_y, s.grid_spacing).into()
    }

    /// Generate initial grid commands (zoom=1, pan=0).
    pub fn generate_initial_grid(&self, width: f32, height: f32) -> SharedString {
        let spacing = self.state.borrow().grid_spacing;
        crate::generate_grid_commands(width, height, 1.0, 0.0, 0.0, spacing).into()
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
        let s = self.state.borrow();
        let zoom = s.safe_zoom();
        let pan_x = s.pan_x;
        let pan_y = s.pan_y;
        let cache = self.cache.borrow();

        let link_geometries = s.links.iter().filter_map(|(&id, &(start_pin, end_pin))| {
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
        });

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
        let s = self.state.borrow();
        let zoom = s.safe_zoom();
        let pan_x = s.pan_x;
        let pan_y = s.pan_y;
        drop(s);
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
        let s = self.state.borrow();
        let z = s.safe_zoom();
        let world_x = (sx - s.pan_x) / z;
        let world_y = (sy - s.pan_y) / z;
        let world_w = sw / z;
        let world_h = sh / z;
        drop(s);

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
        let s = self.state.borrow();
        let z = s.safe_zoom();
        let world_x = (sx - s.pan_x) / z;
        let world_y = (sy - s.pan_y) / z;
        let world_w = sw / z;
        let world_h = sh / z;
        let cache = self.cache.borrow();

        // Compute world-space link endpoints: node_world + pin_rel
        let link_geometries = s.links.iter().filter_map(|(&id, &(start_pin, end_pin))| {
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
        });

        crate::hit_test::links_in_selection_box(
            world_x,
            world_y,
            world_w,
            world_h,
            link_geometries,
        )
    }
}

#[cfg(test)]
mod tests {
    #[allow(deprecated)]
    use super::*;
    use crate::hit_test::NodeGeometry;

    /// Helper: set up a controller with two nodes and pins, suitable for hit testing.
    fn setup_controller() -> NodeEditorController {
        let ctrl = NodeEditorController::new();
        ctrl.set_viewport(1.0, 0.0, 0.0);

        {
            let mut cache = ctrl.cache.borrow_mut();
            // Node 1 at world (0, 0), 100x50
            cache.handle_node_rect_report(1, 0.0, 0.0, 100.0, 50.0);
            // Node 2 at world (200, 100), 100x50
            cache.handle_node_rect_report(2, 200.0, 100.0, 100.0, 50.0);
            // Pin 1001: output on node 1 at world-relative (100, 25)
            cache.handle_pin_report(1001, 1, 2, 100.0, 25.0);
            // Pin 2001: input on node 2 at world-relative (0, 25)
            cache.handle_pin_report(2001, 2, 1, 0.0, 25.0);
        }

        // Register a link between the pins
        ctrl.register_link(1, 1001, 2001);
        ctrl
    }

    // ========================================================================
    // Construction and basic accessors
    // ========================================================================

    #[test]
    fn test_new_defaults() {
        let ctrl = NodeEditorController::new();
        assert_eq!(ctrl.zoom(), 1.0);
        assert_eq!(ctrl.dragged_node_id(), 0);
    }

    #[test]
    fn test_set_viewport() {
        let ctrl = NodeEditorController::new();
        ctrl.set_viewport(2.0, 10.0, 20.0);
        assert_eq!(ctrl.zoom(), 2.0);
        let s = ctrl.state.borrow();
        assert_eq!(s.pan_x, 10.0);
        assert_eq!(s.pan_y, 20.0);
    }

    #[test]
    #[allow(deprecated)]
    fn test_set_zoom_deprecated_still_works() {
        let ctrl = NodeEditorController::new();
        ctrl.set_zoom(3.0);
        assert_eq!(ctrl.zoom(), 3.0);
    }

    // ========================================================================
    // Link registration (HashMap-based, idempotent)
    // ========================================================================

    #[test]
    fn test_register_link_idempotent() {
        let ctrl = NodeEditorController::new();
        ctrl.register_link(1, 100, 200);
        ctrl.register_link(1, 100, 200); // duplicate
        assert_eq!(ctrl.state.borrow().links.len(), 1);
    }

    #[test]
    fn test_register_link_updates_pins() {
        let ctrl = NodeEditorController::new();
        ctrl.register_link(1, 100, 200);
        ctrl.register_link(1, 300, 400); // same ID, different pins
        let s = ctrl.state.borrow();
        assert_eq!(s.links.get(&1), Some(&(300, 400)));
    }

    #[test]
    fn test_unregister_link() {
        let ctrl = NodeEditorController::new();
        ctrl.register_link(1, 100, 200);
        ctrl.unregister_link(1);
        assert!(ctrl.state.borrow().links.is_empty());
    }

    #[test]
    fn test_clear_links() {
        let ctrl = NodeEditorController::new();
        ctrl.register_link(1, 100, 200);
        ctrl.register_link(2, 300, 400);
        ctrl.clear_links();
        assert!(ctrl.state.borrow().links.is_empty());
    }

    // ========================================================================
    // handle_node_rect: screen→world conversion
    // ========================================================================

    #[test]
    fn test_handle_node_rect_zoom1() {
        let ctrl = NodeEditorController::new();
        ctrl.set_viewport(1.0, 0.0, 0.0);
        ctrl.handle_node_rect(1, 100.0, 200.0, 50.0, 30.0);
        let cache = ctrl.cache.borrow();
        let rect = cache.node_rects.get(&1).unwrap().rect();
        assert_eq!(rect, (100.0, 200.0, 50.0, 30.0));
    }

    #[test]
    fn test_handle_node_rect_zoom2_with_pan() {
        let ctrl = NodeEditorController::new();
        ctrl.set_viewport(2.0, 50.0, 100.0);
        // Screen coords: x=250, y=300, w=100, h=60
        // World: x=(250-50)/2=100, y=(300-100)/2=100, w=50, h=30
        ctrl.handle_node_rect(1, 250.0, 300.0, 100.0, 60.0);
        let cache = ctrl.cache.borrow();
        let rect = cache.node_rects.get(&1).unwrap().rect();
        assert_eq!(rect, (100.0, 100.0, 50.0, 30.0));
    }

    #[test]
    fn test_handle_node_rect_zero_zoom_fallback() {
        let ctrl = NodeEditorController::new();
        ctrl.set_viewport(0.0, 0.0, 0.0);
        ctrl.handle_node_rect(1, 100.0, 200.0, 50.0, 30.0);
        // Should use zoom=1.0 fallback
        let cache = ctrl.cache.borrow();
        let rect = cache.node_rects.get(&1).unwrap().rect();
        assert_eq!(rect, (100.0, 200.0, 50.0, 30.0));
    }

    // ========================================================================
    // find_link_at_screen at various zoom levels
    // ========================================================================

    #[test]
    fn test_find_link_at_screen_zoom1() {
        let ctrl = setup_controller();
        // At zoom=1, pan=0: pin 1001 is at screen (100, 25), pin 2001 at (200, 125)
        let result = ctrl.find_link_at_screen(100.0, 25.0, 10.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_screen_miss() {
        let ctrl = setup_controller();
        let result = ctrl.find_link_at_screen(500.0, 500.0, 10.0, 50.0, 20);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_find_link_at_screen_zoom2() {
        let ctrl = setup_controller();
        ctrl.set_viewport(2.0, 0.0, 0.0);
        // At zoom=2, pan=0: pin 1001 screen pos = (0+100)*2+0 = 200, (0+25)*2+0 = 50
        let result = ctrl.find_link_at_screen(200.0, 50.0, 10.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_screen_with_pan() {
        let ctrl = setup_controller();
        ctrl.set_viewport(1.0, 50.0, 30.0);
        // At zoom=1, pan=(50,30): pin 1001 screen pos = (0+100)*1+50 = 150, (0+25)*1+30 = 55
        let result = ctrl.find_link_at_screen(150.0, 55.0, 10.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    // ========================================================================
    // find_pin_at_screen at various zoom levels
    // ========================================================================

    #[test]
    fn test_find_pin_at_screen_zoom1() {
        let ctrl = setup_controller();
        // Pin 1001 at screen (100, 25)
        let result = ctrl.find_pin_at_screen(102.0, 27.0, 10.0);
        assert_eq!(result, 1001);
    }

    #[test]
    fn test_find_pin_at_screen_miss() {
        let ctrl = setup_controller();
        let result = ctrl.find_pin_at_screen(500.0, 500.0, 10.0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_find_pin_at_screen_zoom2() {
        let ctrl = setup_controller();
        ctrl.set_viewport(2.0, 0.0, 0.0);
        // Pin 1001 screen pos = (0+100)*2 = 200, (0+25)*2 = 50
        let result = ctrl.find_pin_at_screen(200.0, 50.0, 10.0);
        assert_eq!(result, 1001);
    }

    #[test]
    fn test_find_pin_at_screen_with_pan() {
        let ctrl = setup_controller();
        ctrl.set_viewport(1.0, 50.0, 30.0);
        // Pin 1001 screen pos = 100+50=150, 25+30=55
        let result = ctrl.find_pin_at_screen(150.0, 55.0, 10.0);
        assert_eq!(result, 1001);
    }

    // ========================================================================
    // nodes_in_selection_box_screen
    // ========================================================================

    #[test]
    fn test_nodes_in_box_zoom1() {
        let ctrl = setup_controller();
        let result = ctrl.nodes_in_selection_box_screen(0.0, 0.0, 50.0, 50.0);
        assert!(result.contains(&1));
        assert!(!result.contains(&2));
    }

    #[test]
    fn test_nodes_in_box_zoom2() {
        let ctrl = setup_controller();
        ctrl.set_viewport(2.0, 0.0, 0.0);
        // Screen box (0,0,100,100) → world (0,0,50,50) → intersects node 1 at (0,0,100,50)
        let result = ctrl.nodes_in_selection_box_screen(0.0, 0.0, 100.0, 100.0);
        assert!(result.contains(&1));
    }

    // ========================================================================
    // links_in_selection_box_screen
    // ========================================================================

    #[test]
    fn test_links_in_box_zoom1() {
        let ctrl = setup_controller();
        // Link 1: start at world (100,25), end at world (200,125)
        let result = ctrl.links_in_selection_box_screen(90.0, 15.0, 20.0, 20.0);
        assert!(result.contains(&1));
    }

    #[test]
    fn test_links_in_box_miss() {
        let ctrl = setup_controller();
        let result = ctrl.links_in_selection_box_screen(500.0, 500.0, 50.0, 50.0);
        assert!(result.is_empty());
    }

    // ========================================================================
    // safe_zoom guard
    // ========================================================================

    #[test]
    fn test_safe_zoom_zero() {
        let ctrl = NodeEditorController::new();
        ctrl.set_viewport(0.0, 0.0, 0.0);
        ctrl.handle_node_rect(1, 100.0, 200.0, 50.0, 30.0);
        let result = ctrl.nodes_in_selection_box_screen(0.0, 0.0, 200.0, 300.0);
        assert!(result.contains(&1));
    }

    #[test]
    fn test_safe_zoom_negative() {
        let ctrl = NodeEditorController::new();
        ctrl.set_viewport(-1.0, 0.0, 0.0);
        ctrl.handle_node_rect(1, 100.0, 200.0, 50.0, 30.0);
        let _ = ctrl.find_link_at_screen(0.0, 0.0, 10.0, 50.0, 20);
        let _ = ctrl.find_pin_at_screen(0.0, 0.0, 10.0);
    }
}
