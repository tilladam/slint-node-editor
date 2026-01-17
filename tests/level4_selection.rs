//! Level 4: Selection Tests
//!
//! Tests single selection, multi-selection with Shift, box selection,
//! and selection clearing.

mod common;

use common::harness::MinimalTestHarness;
use slint::{Model, SharedString};

/// Helper to set up geometry in the cache for testing.
fn setup_test_geometry(harness: &MinimalTestHarness) {
    let cache = harness.ctrl.cache();
    let mut cache = cache.borrow_mut();

    // Node A at (100, 100), size 150x100
    cache.update_node_rect(1, 100.0, 100.0, 150.0, 100.0);
    // Node B at (400, 200), size 150x100
    cache.update_node_rect(2, 400.0, 200.0, 150.0, 100.0);

    // Pins
    cache.handle_pin_report(2, 1, 1, 0.0, 50.0);
    cache.handle_pin_report(3, 1, 2, 150.0, 50.0);
    cache.handle_pin_report(4, 2, 1, 0.0, 50.0);
    cache.handle_pin_report(5, 2, 2, 150.0, 50.0);
}

// ============================================================================
// Single Selection Tests
// ============================================================================

#[test]
fn test_single_click_selects_node() {
    let harness = MinimalTestHarness::new();

    // Use SelectionManager directly
    let mut sel = harness.selection.borrow_mut();
    sel.handle_interaction(1, false);

    assert!(sel.contains(1), "Node should be selected");
    assert_eq!(sel.len(), 1, "Only one node should be selected");
}

#[test]
fn test_single_click_replaces_selection() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    // Select node 1
    sel.handle_interaction(1, false);
    assert!(sel.contains(1));

    // Click node 2 (without shift) should replace selection
    sel.handle_interaction(2, false);

    assert!(!sel.contains(1), "Node 1 should be deselected");
    assert!(sel.contains(2), "Node 2 should be selected");
    assert_eq!(sel.len(), 1, "Only one node should be selected");
}

#[test]
fn test_click_on_already_selected_single_item() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    sel.handle_interaction(1, false);
    sel.handle_interaction(1, false); // Click again

    assert!(sel.contains(1), "Node should still be selected");
    assert_eq!(sel.len(), 1);
}

#[test]
fn test_clear_selection() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();
    sel.handle_interaction(1, false);
    sel.handle_interaction(2, true);

    assert_eq!(sel.len(), 2);

    sel.clear();

    assert!(sel.is_empty(), "Selection should be cleared");
    assert!(!sel.contains(1));
    assert!(!sel.contains(2));
}

// ============================================================================
// Multi-Selection Tests (Shift+Click)
// ============================================================================

#[test]
fn test_shift_click_adds_to_selection() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    sel.handle_interaction(1, false); // Select first
    sel.handle_interaction(2, true); // Shift+click second

    assert!(sel.contains(1), "Node 1 should be selected");
    assert!(sel.contains(2), "Node 2 should be selected");
    assert_eq!(sel.len(), 2, "Both nodes should be selected");
}

#[test]
fn test_shift_click_on_selected_removes_it() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    sel.handle_interaction(1, true);
    sel.handle_interaction(2, true);
    assert_eq!(sel.len(), 2);

    // Shift+click on already selected item removes it
    sel.handle_interaction(1, true);

    assert!(!sel.contains(1), "Node 1 should be deselected");
    assert!(sel.contains(2), "Node 2 should still be selected");
    assert_eq!(sel.len(), 1);
}

#[test]
fn test_shift_click_on_empty_selection() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    // Shift+click on empty selection should add the item
    sel.handle_interaction(1, true);

    assert!(sel.contains(1));
    assert_eq!(sel.len(), 1);
}

#[test]
fn test_click_collapses_multi_selection() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    // Create multi-selection
    sel.handle_interaction(1, true);
    sel.handle_interaction(2, true);
    assert_eq!(sel.len(), 2);

    // Normal click on one item collapses to just that item
    sel.handle_interaction(1, false);

    assert!(sel.contains(1));
    assert!(!sel.contains(2));
    assert_eq!(sel.len(), 1);
}

#[test]
fn test_toggle_all_off_with_shift() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    sel.handle_interaction(1, true);
    sel.handle_interaction(1, true); // Toggle off

    assert!(sel.is_empty(), "Selection should be empty after toggling off");
}

// ============================================================================
// Box Selection Tests
// ============================================================================

#[test]
fn test_box_selection_finds_enclosed_nodes() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Selection box that covers Node 1 at (100, 100) with size 150x100
    let selected = harness
        .ctrl
        .cache()
        .borrow()
        .nodes_in_selection_box(50.0, 50.0, 200.0, 200.0);

    assert!(
        selected.contains(&1),
        "Node 1 should be found in selection box"
    );
}

#[test]
fn test_box_selection_excludes_outside_nodes() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Selection box that only covers Node 1 area
    let selected = harness
        .ctrl
        .cache()
        .borrow()
        .nodes_in_selection_box(50.0, 50.0, 200.0, 150.0);

    assert!(selected.contains(&1), "Node 1 should be selected");
    assert!(
        !selected.contains(&2),
        "Node 2 should not be in selection box"
    );
}

#[test]
fn test_box_selection_empty_area() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Selection box in empty area
    let selected = harness
        .ctrl
        .cache()
        .borrow()
        .nodes_in_selection_box(600.0, 50.0, 100.0, 100.0);

    assert!(selected.is_empty(), "No nodes should be selected");
}

#[test]
fn test_box_selection_both_nodes() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Large selection box covering both nodes
    // Node 1: (100, 100) - (250, 200)
    // Node 2: (400, 200) - (550, 300)
    let selected = harness
        .ctrl
        .cache()
        .borrow()
        .nodes_in_selection_box(50.0, 50.0, 600.0, 300.0);

    assert!(selected.contains(&1), "Node 1 should be selected");
    assert!(selected.contains(&2), "Node 2 should be selected");
    assert_eq!(selected.len(), 2);
}

#[test]
fn test_replace_selection_for_box() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    // Previous selection
    sel.handle_interaction(1, false);

    // Box selection replaces entire selection
    sel.replace_selection(vec![2, 3, 4]);

    assert!(!sel.contains(1), "Previous selection should be cleared");
    assert!(sel.contains(2));
    assert!(sel.contains(3));
    assert!(sel.contains(4));
    assert_eq!(sel.len(), 3);
}

// ============================================================================
// Selection Manager Sync Tests
// ============================================================================

#[test]
fn test_sync_to_model() {
    use slint::VecModel;
    use std::rc::Rc;

    let harness = MinimalTestHarness::new();

    let sel = harness.selection.borrow_mut();
    drop(sel);

    harness.selection.borrow_mut().replace_selection(vec![1, 2]);

    let model = Rc::new(VecModel::<i32>::default());
    harness.selection.borrow().sync_to_model(&model);

    assert_eq!(model.row_count(), 2);
}

#[test]
fn test_sync_from_model() {
    use slint::VecModel;
    use std::rc::Rc;

    let harness = MinimalTestHarness::new();

    let model = Rc::new(VecModel::from(vec![3, 4, 5]));

    harness
        .selection
        .borrow_mut()
        .sync_from_model(model.as_ref());

    assert!(harness.selection.borrow().contains(3));
    assert!(harness.selection.borrow().contains(4));
    assert!(harness.selection.borrow().contains(5));
    assert_eq!(harness.selection.borrow().len(), 3);
}

// ============================================================================
// Selection Iterator Tests
// ============================================================================

#[test]
fn test_iterate_over_selection() {
    let harness = MinimalTestHarness::new();

    harness.selection.borrow_mut().replace_selection(vec![1, 2]);

    let ids: Vec<i32> = harness.selection.borrow().iter().copied().collect();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&1));
    assert!(ids.contains(&2));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_negative_node_ids() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();

    sel.handle_interaction(-1, false);
    sel.handle_interaction(-2, true);

    assert!(sel.contains(-1));
    assert!(sel.contains(-2));
}

#[test]
fn test_zero_node_id() {
    let harness = MinimalTestHarness::new();

    let mut sel = harness.selection.borrow_mut();
    sel.handle_interaction(0, false);

    assert!(sel.contains(0));
}

#[test]
fn test_large_selection() {
    let harness = MinimalTestHarness::new();

    let ids: Vec<i32> = (1..=100).collect();
    harness.selection.borrow_mut().replace_selection(ids);

    assert_eq!(harness.selection.borrow().len(), 100);
    assert!(harness.selection.borrow().contains(1));
    assert!(harness.selection.borrow().contains(50));
    assert!(harness.selection.borrow().contains(100));
}

#[test]
fn test_selection_with_many_nodes() {
    use common::harness::NodeData;

    let nodes: Vec<NodeData> = (1..=10)
        .map(|i| NodeData {
            id: i,
            title: SharedString::from(format!("Node {}", i)),
            x: (i as f32) * 150.0,
            y: 100.0,
        })
        .collect();

    let harness = MinimalTestHarness::with_nodes_and_links(nodes, vec![]);

    // Select every other node using shift
    {
        let mut sel = harness.selection.borrow_mut();
        sel.handle_interaction(1, false);
        sel.handle_interaction(3, true);
        sel.handle_interaction(5, true);
        sel.handle_interaction(7, true);
        sel.handle_interaction(9, true);
    }

    let sel = harness.selection.borrow();
    assert_eq!(sel.len(), 5);
    assert!(sel.contains(1));
    assert!(!sel.contains(2));
    assert!(sel.contains(3));
}

// ============================================================================
// Selection Changed Callback Tests
// ============================================================================

#[test]
fn test_selection_changed_callback_tracking() {
    let harness = MinimalTestHarness::new();

    assert_eq!(*harness.tracker.selection_changed.borrow(), 0);

    // Simulate selection changed being called
    *harness.tracker.selection_changed.borrow_mut() += 1;

    assert_eq!(*harness.tracker.selection_changed.borrow(), 1);
}
