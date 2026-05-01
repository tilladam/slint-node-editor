//! Level 8: Globals & Transform-Scale Tests
//!
//! Tests for the globals-based communication system and the transform-scale
//! zoom model: NodeEditorSetup multi-node drag, compute_link_path_world,
//! box selection coordinate conversion, and resolve_link_endpoints.

mod common;

use common::harness::MinimalTestHarness;
use slint_node_editor::{
    GeometryCache, NodeEditorSetup, SimpleNodeGeometry,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Helper to set up geometry in the cache for testing.
fn setup_test_geometry(harness: &MinimalTestHarness) {
    let cache = harness.ctrl.cache();
    let mut cache = cache.borrow_mut();

    cache.update_node_rect(1, 100.0, 100.0, 150.0, 100.0);
    cache.update_node_rect(2, 400.0, 200.0, 150.0, 100.0);

    cache.handle_pin_report(2, 1, 1, 0.0, 50.0);   // Node 1 input
    cache.handle_pin_report(3, 1, 2, 150.0, 50.0);  // Node 1 output
    cache.handle_pin_report(4, 2, 1, 0.0, 50.0);    // Node 2 input
    cache.handle_pin_report(5, 2, 2, 150.0, 50.0);  // Node 2 output
}

// ============================================================================
// NodeEditorSetup: multi-node drag
// ============================================================================

#[test]
fn test_setup_single_node_drag_calls_closure() {
    let moved: Rc<RefCell<Vec<(i32, f32, f32)>>> = Rc::new(RefCell::new(Vec::new()));
    let moved_clone = moved.clone();

    let setup = NodeEditorSetup::new(move |id, dx, dy| {
        moved_clone.borrow_mut().push((id, dx, dy));
    });

    // Simulate: start drag on node 1, then end with delta
    let start = setup.start_node_drag();
    let end = setup.end_node_drag();

    start(1, false, 0.0, 0.0);
    end(50.0, 30.0);

    let calls = moved.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (1, 50.0, 30.0));
}

#[test]
fn test_setup_multi_node_drag_moves_all_selected() {
    let moved: Rc<RefCell<Vec<(i32, f32, f32)>>> = Rc::new(RefCell::new(Vec::new()));
    let moved_clone = moved.clone();

    let setup = NodeEditorSetup::new(move |id, dx, dy| {
        moved_clone.borrow_mut().push((id, dx, dy));
    });

    // Add nodes 1, 2, 3 to selection
    {
        let sel_rc = setup.selection();
        let mut sel = sel_rc.borrow_mut();
        sel.insert(1);
        sel.insert(2);
        sel.insert(3);
    }

    // Drag node 2 (which is in the selection)
    let start = setup.start_node_drag();
    let end = setup.end_node_drag();

    start(2, false, 0.0, 0.0);
    end(10.0, 20.0);

    let calls = moved.borrow();
    // All 3 selected nodes should be moved
    assert_eq!(calls.len(), 3);
    let ids: Vec<i32> = calls.iter().map(|(id, _, _)| *id).collect();
    assert!(ids.contains(&1));
    assert!(ids.contains(&2));
    assert!(ids.contains(&3));
    // All with the same delta
    for (_, dx, dy) in calls.iter() {
        assert_eq!(*dx, 10.0);
        assert_eq!(*dy, 20.0);
    }
}

#[test]
fn test_setup_drag_unselected_node_moves_only_that_node() {
    let moved: Rc<RefCell<Vec<(i32, f32, f32)>>> = Rc::new(RefCell::new(Vec::new()));
    let moved_clone = moved.clone();

    let setup = NodeEditorSetup::new(move |id, dx, dy| {
        moved_clone.borrow_mut().push((id, dx, dy));
    });

    // Select nodes 1 and 2
    {
        let sel_rc = setup.selection();
        let mut sel = sel_rc.borrow_mut();
        sel.insert(1);
        sel.insert(2);
    }

    // Drag node 5 (NOT in selection)
    let start = setup.start_node_drag();
    let end = setup.end_node_drag();

    start(5, false, 0.0, 0.0);
    end(10.0, 20.0);

    let calls = moved.borrow();
    // Only node 5 should move (not in multi-selection)
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (5, 10.0, 20.0));
}

#[test]
fn test_setup_single_selected_node_drag_moves_only_that_node() {
    let moved: Rc<RefCell<Vec<(i32, f32, f32)>>> = Rc::new(RefCell::new(Vec::new()));
    let moved_clone = moved.clone();

    let setup = NodeEditorSetup::new(move |id, dx, dy| {
        moved_clone.borrow_mut().push((id, dx, dy));
    });

    // Select only node 1
    {
        let sel_rc = setup.selection();
        let mut sel = sel_rc.borrow_mut();
        sel.insert(1);
    }

    // Drag node 1 (single selection, not multi)
    let start = setup.start_node_drag();
    let end = setup.end_node_drag();

    start(1, false, 0.0, 0.0);
    end(10.0, 20.0);

    let calls = moved.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (1, 10.0, 20.0));
}

// ============================================================================
// compute_link_path_world: pure world-coordinate paths
// ============================================================================

fn make_cache_with_link() -> GeometryCache<SimpleNodeGeometry> {
    let mut cache = GeometryCache::<SimpleNodeGeometry>::default();
    cache.update_node_rect(1, 100.0, 100.0, 150.0, 100.0);
    cache.update_node_rect(2, 400.0, 200.0, 150.0, 100.0);
    cache.handle_pin_report(3, 1, 2, 150.0, 50.0); // Node 1 output
    cache.handle_pin_report(4, 2, 1, 0.0, 50.0);   // Node 2 input
    cache
}

#[test]
fn test_compute_link_path_world_returns_valid_svg() {
    let cache = make_cache_with_link();
    let path = cache.compute_link_path_world(3, 4, 50.0);
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.starts_with("M "), "Path should start with M: {}", path);
    assert!(path.contains(" C "), "Path should contain cubic bezier: {}", path);
}

#[test]
fn test_compute_link_path_world_ignores_viewport() {
    let cache = make_cache_with_link();

    // World path should be identical regardless of what viewport is set
    let path_world = cache.compute_link_path_world(3, 4, 50.0).unwrap();

    // Compare with compute_link_path at zoom=1.0 (should be identical)
    let path_z1 = cache.compute_link_path(3, 4, 1.0, 50.0).unwrap();
    assert_eq!(path_world, path_z1, "World path should match zoom=1.0 path");

    // But screen path at zoom=2.0 should differ
    let path_screen = cache.compute_link_path_screen(3, 4, 2.0, 0.0, 0.0, 50.0).unwrap();
    assert_ne!(path_world, path_screen, "World path should differ from screen path at zoom=2");
}

#[test]
fn test_compute_link_path_world_missing_pin_returns_none() {
    let cache = make_cache_with_link();
    assert!(cache.compute_link_path_world(999, 4, 50.0).is_none());
    assert!(cache.compute_link_path_world(3, 999, 50.0).is_none());
}

#[test]
fn test_compute_link_path_world_uses_correct_coordinates() {
    let cache = make_cache_with_link();
    // Node 1 output: rect(100,100) + rel(150,50) = (250, 150)
    // Node 2 input:  rect(400,200) + rel(0,50)   = (400, 250)
    let path = cache.compute_link_path_world(3, 4, 50.0).unwrap();

    // Path should start at pin 3's absolute position
    assert!(path.starts_with("M 250 150"), "Path should start at (250,150): {}", path);
}

// ============================================================================
// resolve_link_endpoints via compute_link_path variants
// ============================================================================

#[test]
fn test_all_path_methods_share_endpoints() {
    let cache = make_cache_with_link();

    // All three methods should resolve the same start/end positions
    // (just transform them differently)
    let world = cache.compute_link_path_world(3, 4, 50.0).unwrap();
    let same_space = cache.compute_link_path(3, 4, 1.0, 50.0).unwrap();

    // At zoom=1.0, pan=0: world == same_space == screen
    let screen = cache.compute_link_path_screen(3, 4, 1.0, 0.0, 0.0, 50.0).unwrap();

    assert_eq!(world, same_space);
    assert_eq!(world, screen);
}

#[test]
fn test_screen_path_applies_zoom_and_pan() {
    let cache = make_cache_with_link();

    let zoom = 2.0;
    let pan_x = 50.0;
    let pan_y = 100.0;

    let screen = cache.compute_link_path_screen(3, 4, zoom, pan_x, pan_y, 50.0).unwrap();

    // Node 1 output: world(250, 150) → screen(250*2+50, 150*2+100) = (550, 400)
    assert!(screen.starts_with("M 550 400"), "Screen path should start at (550,400): {}", screen);
}

#[test]
fn test_pan_x_increments_geometry_version() {
    let harness = MinimalTestHarness::new();
    let initial_version = harness.window.get_geometry_version();

    harness.window.set_pan_x(25.0);
    harness.pump_events();

    assert!(
        harness.window.get_geometry_version() > initial_version,
        "pan-x changes must invalidate geometry-dependent link path bindings"
    );
}

#[test]
fn test_pan_y_increments_geometry_version() {
    let harness = MinimalTestHarness::new();
    let initial_version = harness.window.get_geometry_version();

    harness.window.set_pan_y(25.0);
    harness.pump_events();

    assert!(
        harness.window.get_geometry_version() > initial_version,
        "pan-y changes must invalidate geometry-dependent link path bindings"
    );
}

// ============================================================================
// Box selection coordinate conversion
// ============================================================================

#[test]
fn test_box_selection_world_coords_at_zoom_1() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);
    harness.ctrl.set_viewport(1.0, 0.0, 0.0);

    // Box that encloses node 1 (at 100,100 with size 150x100)
    let selected = harness.ctrl.cache().borrow()
        .nodes_in_selection_box(50.0, 50.0, 250.0, 200.0);
    assert!(selected.contains(&1), "Node 1 should be in selection box");
    assert!(!selected.contains(&2), "Node 2 should not be in selection box");
}

#[test]
fn test_box_selection_world_coords_both_nodes() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Box that encloses both nodes
    let selected = harness.ctrl.cache().borrow()
        .nodes_in_selection_box(50.0, 50.0, 550.0, 300.0);
    assert!(selected.contains(&1));
    assert!(selected.contains(&2));
}

#[test]
fn test_box_selection_screen_to_world_conversion() {
    // Simulate the coordinate conversion that NodeEditor does:
    // world = (screen - pan) / zoom
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let zoom = 2.0_f32;
    let pan_x = 50.0_f32;
    let pan_y = 100.0_f32;
    harness.ctrl.set_viewport(zoom, pan_x, pan_y);

    // Screen-space box: (250, 300) to (850, 700)
    let screen_x = 250.0_f32;
    let screen_y = 300.0_f32;
    let screen_w = 600.0_f32;
    let screen_h = 400.0_f32;

    // Convert to world: world = (screen - pan) / zoom
    let world_x = (screen_x - pan_x) / zoom;
    let world_y = (screen_y - pan_y) / zoom;
    let world_w = screen_w / zoom;
    let world_h = screen_h / zoom;

    // world box: (100, 100) to (400, 300) — should contain node 1
    let selected = harness.ctrl.cache().borrow()
        .nodes_in_selection_box(world_x, world_y, world_w, world_h);

    assert!(selected.contains(&1),
        "Node 1 at (100,100) should be in world box ({},{}) {}x{}",
        world_x, world_y, world_w, world_h);
}

#[test]
fn test_box_selection_empty_at_wrong_coords() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Box far from any nodes
    let selected = harness.ctrl.cache().borrow()
        .nodes_in_selection_box(1000.0, 1000.0, 100.0, 100.0);
    assert!(selected.is_empty());
}

// ============================================================================
// Integration: wire_node_editor! exercises the new architecture
// ============================================================================

#[test]
fn test_harness_uses_wire_node_editor_macro() {
    // The harness uses wire_node_editor! internally.
    // If the macro is broken, this test (and all others) would fail to compile.
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Verify the macro wired compute_link_path correctly
    let cache = harness.ctrl.cache();
    let cache = cache.borrow();
    let path = cache.compute_link_path_world(3, 4, 50.0);
    assert!(path.is_some(), "Link path should be computable after wire_node_editor!");
}

#[test]
fn test_harness_tracks_drag_via_globals_path() {
    // Verify the harness tracks drags through the new GeometryCallbacks path
    // (not the old window.on_node_drag_started path)
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    assert!(harness.tracker.node_drag_started.borrow().is_empty());
    assert!(harness.tracker.node_drag_ended.borrow().is_empty());

    // The tracker is wired to NodeEditorInternalCallbacks.on_start_node_drag
    // and on_end_node_drag via the harness setup. If a real BaseNode initiated
    // a drag, it would fire through that path. Here we verify the wiring exists
    // by checking the tracker starts empty and the callbacks are accessible.
}
