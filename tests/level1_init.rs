//! Level 1: Basic Initialization Tests
//!
//! Tests window creation, initial state verification, and geometry callbacks.

mod common;

use common::harness::MinimalTestHarness;
use slint::{ComponentHandle, Model};

#[test]
fn test_window_creates_successfully() {
    let harness = MinimalTestHarness::new();
    // If we got here, window creation succeeded
    // Note: In the testing backend, windows are not "visible" in the UI sense
    assert!(harness.window.window().size().width > 0);
}

#[test]
fn test_nodes_model_contains_expected_data() {
    let harness = MinimalTestHarness::new();

    assert_eq!(harness.nodes.row_count(), 2);

    let node_a = harness.nodes.row_data(0).unwrap();
    assert_eq!(node_a.id, 1);
    assert_eq!(node_a.title.as_str(), "Node A");
    assert_eq!(node_a.x, 100.0);
    assert_eq!(node_a.y, 100.0);

    let node_b = harness.nodes.row_data(1).unwrap();
    assert_eq!(node_b.id, 2);
    assert_eq!(node_b.title.as_str(), "Node B");
    assert_eq!(node_b.x, 400.0);
    assert_eq!(node_b.y, 200.0);
}

#[test]
fn test_links_model_contains_expected_data() {
    let harness = MinimalTestHarness::new();

    assert_eq!(harness.links.row_count(), 1);

    let link = harness.links.row_data(0).unwrap();
    assert_eq!(link.id, 1);
    assert_eq!(link.start_pin_id, 3); // Node 1 output (1 * 2 + 1)
    assert_eq!(link.end_pin_id, 4); // Node 2 input (2 * 2)
}

#[test]
fn test_controller_initializes_with_defaults() {
    let harness = MinimalTestHarness::new();

    // Default zoom is 1.0
    assert_eq!(harness.ctrl.zoom(), 1.0);

    // No node is being dragged
    assert_eq!(harness.ctrl.dragged_node_id(), 0);
}

#[test]
fn test_selection_starts_empty() {
    let harness = MinimalTestHarness::new();

    assert!(harness.selection.borrow().is_empty());
    assert_eq!(harness.selection.borrow().len(), 0);
    assert!(!harness.selection.borrow().contains(1));
    assert!(!harness.selection.borrow().contains(2));
}

#[test]
fn test_custom_nodes_and_links() {
    use common::harness::NodeData;
    use slint::SharedString;

    let harness = MinimalTestHarness::with_nodes_and_links(
        vec![
            NodeData {
                id: 10,
                title: SharedString::from("Custom Node"),
                x: 50.0,
                y: 75.0,
            },
        ],
        vec![],
    );

    assert_eq!(harness.nodes.row_count(), 1);
    assert_eq!(harness.links.row_count(), 0);

    let node = harness.nodes.row_data(0).unwrap();
    assert_eq!(node.id, 10);
    assert_eq!(node.title.as_str(), "Custom Node");
}

#[test]
fn test_empty_models() {
    let harness = MinimalTestHarness::with_nodes_and_links(vec![], vec![]);

    assert_eq!(harness.nodes.row_count(), 0);
    assert_eq!(harness.links.row_count(), 0);
}

#[test]
fn test_callback_tracker_starts_empty() {
    let harness = MinimalTestHarness::new();

    assert!(harness.tracker.node_drag_started.borrow().is_empty());
    assert!(harness.tracker.node_drag_ended.borrow().is_empty());
    assert!(harness.tracker.link_requested.borrow().is_empty());
    assert_eq!(*harness.tracker.link_cancelled.borrow(), 0);
    assert_eq!(*harness.tracker.selection_changed.borrow(), 0);
    assert_eq!(*harness.tracker.delete_selected.borrow(), 0);
}

#[test]
fn test_callback_tracker_can_be_cleared() {
    let harness = MinimalTestHarness::new();

    // Simulate some callbacks being recorded
    harness.tracker.node_drag_started.borrow_mut().push(1);
    *harness.tracker.link_cancelled.borrow_mut() = 5;

    // Clear and verify
    harness.tracker.clear();

    assert!(harness.tracker.node_drag_started.borrow().is_empty());
    assert_eq!(*harness.tracker.link_cancelled.borrow(), 0);
}
