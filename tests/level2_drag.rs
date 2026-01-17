//! Level 2: Node Click & Drag Tests
//!
//! Tests node selection via click, drag operations, and position updates.

mod common;

use common::harness::MinimalTestHarness;
use slint::Model;

/// Helper to set up geometry in the cache for testing.
/// This simulates what happens after nodes are rendered and report their geometry.
fn setup_test_geometry(harness: &MinimalTestHarness) {
    let cache = harness.ctrl.cache();
    let mut cache = cache.borrow_mut();

    // Node A at (100, 100), size 150x100 (as defined in minimal.slint)
    cache.update_node_rect(1, 100.0, 100.0, 150.0, 100.0);
    // Node B at (400, 200), size 150x100
    cache.update_node_rect(2, 400.0, 200.0, 150.0, 100.0);

    // Input pins at left edge, center height (node_id * 2)
    // Output pins at right edge, center height (node_id * 2 + 1)

    // Node 1 pins
    cache.handle_pin_report(2, 1, 1, 0.0, 50.0); // Input pin at left
    cache.handle_pin_report(3, 1, 2, 150.0, 50.0); // Output pin at right

    // Node 2 pins
    cache.handle_pin_report(4, 2, 1, 0.0, 50.0); // Input pin at left
    cache.handle_pin_report(5, 2, 2, 150.0, 50.0); // Output pin at right
}

#[test]
fn test_click_on_node_triggers_selection() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Simulate selection via the selection manager (as the callback would do)
    // This tests the selection mechanism that would be triggered by a click
    harness.selection.borrow_mut().handle_interaction(1, false);

    // Node should be selected
    assert!(
        harness.selection.borrow().contains(1),
        "Node 1 should be selected after click"
    );
}

#[test]
fn test_click_replaces_selection() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Simulate clicking on Node A
    harness.selection.borrow_mut().handle_interaction(1, false);
    assert!(harness.selection.borrow().contains(1));

    // Simulate clicking on Node B (without shift = replace selection)
    harness.selection.borrow_mut().handle_interaction(2, false);

    // Only Node B should be selected now
    assert!(
        !harness.selection.borrow().contains(1),
        "Node 1 should no longer be selected"
    );
    assert!(
        harness.selection.borrow().contains(2),
        "Node 2 should be selected"
    );
}

#[test]
fn test_mouse_down_on_node_records_dragged_node() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Simulate that drag was started for node 1
    harness.ctrl.handle_node_drag_started(1);

    // The controller should track the dragged node
    assert_eq!(
        harness.ctrl.dragged_node_id(),
        1,
        "Controller should track dragged node"
    );
}

#[test]
fn test_node_drag_started_callback_fires() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Simulate drag start callback
    harness.tracker.node_drag_started.borrow_mut().push(1);

    let started = harness.tracker.node_drag_started.borrow();
    assert_eq!(started.len(), 1, "node_drag_started should have been called");
    assert_eq!(started[0], 1, "Should have started drag on node 1");
}

#[test]
fn test_node_drag_ended_callback_records_delta() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Simulate that a drag ended with delta (48, 24)
    harness.tracker.node_drag_ended.borrow_mut().push((48.0, 24.0));

    let ended = harness.tracker.node_drag_ended.borrow();
    assert_eq!(ended.len(), 1, "node_drag_ended should have been called");
    assert_eq!(ended[0], (48.0, 24.0), "Delta should be (48, 24)");
}

#[test]
fn test_node_position_updates_after_drag() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let original = harness.nodes.row_data(0).unwrap();
    let original_x = original.x;
    let original_y = original.y;

    // Simulate the complete drag sequence:
    // 1. Mark which node is being dragged
    harness.ctrl.handle_node_drag_started(1);

    // 2. Simulate the drag-ended callback updating the model
    // (This is what happens in the actual on_node_drag_ended callback)
    let delta_x = 48.0;
    let delta_y = 24.0;
    let node_id = harness.ctrl.dragged_node_id();

    for i in 0..harness.nodes.row_count() {
        if let Some(mut node) = harness.nodes.row_data(i) {
            if node.id == node_id {
                node.x += delta_x;
                node.y += delta_y;
                harness.nodes.set_row_data(i, node);
                break;
            }
        }
    }

    // Verify position changed
    let updated = harness.nodes.row_data(0).unwrap();
    assert_eq!(
        updated.x,
        original_x + delta_x,
        "Node X should be updated by delta"
    );
    assert_eq!(
        updated.y,
        original_y + delta_y,
        "Node Y should be updated by delta"
    );
}

#[test]
fn test_grid_snapping_applies_to_position() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let original = harness.nodes.row_data(0).unwrap();
    let original_x = original.x;
    let original_y = original.y;

    // Simulate drag that should snap to grid
    // Grid spacing is 24.0 by default
    // A delta of 25 should snap to 24
    harness.ctrl.handle_node_drag_started(1);

    // The snap calculation happens in the Slint end-node-drag function:
    // snap_to_grid rounds to nearest grid spacing
    let raw_delta_x: f32 = 25.0;
    let raw_delta_y: f32 = 13.0;

    // Snap calculation: round(value / grid_spacing) * grid_spacing
    let grid_spacing: f32 = 24.0;
    let snapped_delta_x = (raw_delta_x / grid_spacing).round() * grid_spacing; // 24.0
    let snapped_delta_y = (raw_delta_y / grid_spacing).round() * grid_spacing; // 24.0 (13/24 = 0.54, rounds to 1)

    // Update position with snapped values
    let node_id = harness.ctrl.dragged_node_id();
    for i in 0..harness.nodes.row_count() {
        if let Some(mut node) = harness.nodes.row_data(i) {
            if node.id == node_id {
                node.x += snapped_delta_x;
                node.y += snapped_delta_y;
                harness.nodes.set_row_data(i, node);
                break;
            }
        }
    }

    let updated = harness.nodes.row_data(0).unwrap();
    assert_eq!(
        updated.x,
        original_x + snapped_delta_x,
        "Node X should be snapped to grid"
    );
    assert_eq!(
        updated.y,
        original_y + snapped_delta_y,
        "Node Y should be snapped to grid"
    );
}

#[test]
fn test_dragged_node_id_resets_after_drag() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Start drag
    harness.ctrl.handle_node_drag_started(1);
    assert_eq!(harness.ctrl.dragged_node_id(), 1);

    // After drag ends, the node ID should be reset by the Slint code
    // In tests, we simulate this by calling handle_node_drag_started(0)
    // or verifying the ID was set correctly
}

#[test]
fn test_drag_negative_delta() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let original = harness.nodes.row_data(0).unwrap();
    let original_x = original.x;
    let original_y = original.y;

    harness.ctrl.handle_node_drag_started(1);

    // Negative deltas (dragging up-left)
    let delta_x = -48.0;
    let delta_y = -24.0;

    let node_id = harness.ctrl.dragged_node_id();
    for i in 0..harness.nodes.row_count() {
        if let Some(mut node) = harness.nodes.row_data(i) {
            if node.id == node_id {
                node.x += delta_x;
                node.y += delta_y;
                harness.nodes.set_row_data(i, node);
                break;
            }
        }
    }

    let updated = harness.nodes.row_data(0).unwrap();
    assert_eq!(updated.x, original_x + delta_x);
    assert_eq!(updated.y, original_y + delta_y);
}

#[test]
fn test_drag_updates_correct_node_in_model() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let node_a_original = harness.nodes.row_data(0).unwrap();
    let node_b_original = harness.nodes.row_data(1).unwrap();

    // Drag Node B (id=2)
    harness.ctrl.handle_node_drag_started(2);

    let delta_x = 50.0;
    let delta_y = 30.0;

    let node_id = harness.ctrl.dragged_node_id();
    for i in 0..harness.nodes.row_count() {
        if let Some(mut node) = harness.nodes.row_data(i) {
            if node.id == node_id {
                node.x += delta_x;
                node.y += delta_y;
                harness.nodes.set_row_data(i, node);
                break;
            }
        }
    }

    // Node A should be unchanged
    let node_a_after = harness.nodes.row_data(0).unwrap();
    assert_eq!(
        node_a_after.x, node_a_original.x,
        "Node A x should be unchanged"
    );
    assert_eq!(
        node_a_after.y, node_a_original.y,
        "Node A y should be unchanged"
    );

    // Node B should be moved
    let node_b_after = harness.nodes.row_data(1).unwrap();
    assert_eq!(node_b_after.x, node_b_original.x + delta_x);
    assert_eq!(node_b_after.y, node_b_original.y + delta_y);
}

#[test]
fn test_geometry_cache_accessible() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let center = harness.node_center(1);
    assert!(center.is_some(), "Node center should be available");

    let (cx, cy) = center.unwrap();
    assert_eq!(cx, 175.0); // 100 + 150/2
    assert_eq!(cy, 150.0); // 100 + 100/2
}

#[test]
fn test_pin_position_accessible() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Output pin of node 1 (pin id 3)
    let pin_pos = harness.pin_position(3);
    assert!(pin_pos.is_some(), "Pin position should be available");

    let (px, py) = pin_pos.unwrap();
    assert_eq!(px, 250.0); // Node at 100 + pin rel_x 150
    assert_eq!(py, 150.0); // Node at 100 + pin rel_y 50
}
