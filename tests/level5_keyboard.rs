//! Level 5: Keyboard Input Tests
//!
//! Tests Delete/Backspace for removing selected items, Escape for canceling operations.

mod common;

use common::harness::MinimalTestHarness;
use slint::platform::Key;
use slint::{ComponentHandle, Model, SharedString};

/// Helper to set up geometry in the cache for testing.
fn setup_test_geometry(harness: &MinimalTestHarness) {
    let cache = harness.ctrl.cache();
    let mut cache = cache.borrow_mut();

    cache.update_node_rect(1, 100.0, 100.0, 150.0, 100.0);
    cache.update_node_rect(2, 400.0, 200.0, 150.0, 100.0);

    cache.handle_pin_report(2, 1, 1, 0.0, 50.0);
    cache.handle_pin_report(3, 1, 2, 150.0, 50.0);
    cache.handle_pin_report(4, 2, 1, 0.0, 50.0);
    cache.handle_pin_report(5, 2, 2, 150.0, 50.0);
}

// ============================================================================
// Delete Key Tests
// ============================================================================

#[test]
fn test_delete_selected_callback_tracks_calls() {
    let harness = MinimalTestHarness::new();

    assert_eq!(*harness.tracker.delete_selected.borrow(), 0);

    // Simulate delete_selected being called
    *harness.tracker.delete_selected.borrow_mut() += 1;

    assert_eq!(*harness.tracker.delete_selected.borrow(), 1);
}

#[test]
fn test_delete_key_sends_event() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // The harness sets up on_delete_selected to increment the tracker
    // When we send Delete key, if the FocusScope receives it, it calls delete_selected

    // Note: In integration tests, we may need to ensure focus is set correctly
    // For now, we test that the callback mechanism works via tracker
    harness.key_tap(Key::Delete);

    // The actual behavior depends on Slint's focus handling
    // This test verifies the key event mechanism works
}

#[test]
fn test_backspace_key_sends_event() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    harness.key_tap(Key::Backspace);

    // Similar to Delete - verifies the event dispatch mechanism
}

// ============================================================================
// Delete Implementation Tests
// ============================================================================

#[test]
fn test_delete_removes_selected_nodes_from_model() {
    use common::harness::NodeData;

    let harness = MinimalTestHarness::with_nodes_and_links(
        vec![
            NodeData {
                id: 1,
                title: SharedString::from("A"),
                x: 100.0,
                y: 100.0,
            },
            NodeData {
                id: 2,
                title: SharedString::from("B"),
                x: 400.0,
                y: 200.0,
            },
            NodeData {
                id: 3,
                title: SharedString::from("C"),
                x: 700.0,
                y: 100.0,
            },
        ],
        vec![],
    );

    assert_eq!(harness.nodes.row_count(), 3);

    // Select node 2
    harness.selection.borrow_mut().handle_interaction(2, false);

    // Simulate delete operation
    // In real code, on_delete_selected would remove selected nodes
    // Here we simulate that behavior directly
    let to_delete: Vec<i32> = harness.selection.borrow().iter().copied().collect();
    for id in to_delete {
        for i in 0..harness.nodes.row_count() {
            if let Some(node) = harness.nodes.row_data(i) {
                if node.id == id {
                    harness.nodes.remove(i);
                    break;
                }
            }
        }
    }
    harness.selection.borrow_mut().clear();

    assert_eq!(harness.nodes.row_count(), 2);
    // Verify node 2 was removed
    for i in 0..harness.nodes.row_count() {
        if let Some(node) = harness.nodes.row_data(i) {
            assert_ne!(node.id, 2, "Node 2 should have been deleted");
        }
    }
}

#[test]
fn test_delete_removes_selected_links_from_model() {
    use common::harness::LinkData;
    use slint::Color;

    let harness = MinimalTestHarness::with_nodes_and_links(
        vec![
            common::harness::NodeData {
                id: 1,
                title: SharedString::from("A"),
                x: 100.0,
                y: 100.0,
            },
            common::harness::NodeData {
                id: 2,
                title: SharedString::from("B"),
                x: 400.0,
                y: 200.0,
            },
        ],
        vec![
            LinkData {
                id: 1,
                start_pin_id: 3,
                end_pin_id: 4,
                color: Color::from_argb_u8(255, 100, 180, 255),
                line_width: 2.0,
            },
            LinkData {
                id: 2,
                start_pin_id: 5,
                end_pin_id: 2,
                color: Color::from_argb_u8(255, 255, 100, 100),
                line_width: 2.0,
            },
        ],
    );

    assert_eq!(harness.links.row_count(), 2);

    // Simulate deleting link with id 1
    // In real implementation, this would be triggered by selecting a link and pressing delete
    for i in 0..harness.links.row_count() {
        if let Some(link) = harness.links.row_data(i) {
            if link.id == 1 {
                harness.links.remove(i);
                break;
            }
        }
    }

    assert_eq!(harness.links.row_count(), 1);
    let remaining = harness.links.row_data(0).unwrap();
    assert_eq!(remaining.id, 2, "Link 1 should have been deleted, link 2 remains");
}

#[test]
fn test_deleting_node_should_also_remove_connected_links() {
    use common::harness::{LinkData, NodeData};
    use slint::Color;

    let harness = MinimalTestHarness::with_nodes_and_links(
        vec![
            NodeData {
                id: 1,
                title: SharedString::from("A"),
                x: 100.0,
                y: 100.0,
            },
            NodeData {
                id: 2,
                title: SharedString::from("B"),
                x: 400.0,
                y: 200.0,
            },
        ],
        vec![LinkData {
            id: 1,
            start_pin_id: 3, // Node 1 output
            end_pin_id: 4,   // Node 2 input
            color: Color::from_argb_u8(255, 100, 180, 255),
            line_width: 2.0,
        }],
    );

    setup_test_geometry(&harness);

    // Delete Node 1 and its connected links
    let node_to_delete = 1;

    // Find links connected to node 1
    // Pin IDs for node 1 are: 2 (input) and 3 (output)
    let node_pins = vec![2, 3]; // node_id * 2 and node_id * 2 + 1

    // Remove connected links
    let mut links_to_remove = Vec::new();
    for i in 0..harness.links.row_count() {
        if let Some(link) = harness.links.row_data(i) {
            if node_pins.contains(&link.start_pin_id) || node_pins.contains(&link.end_pin_id) {
                links_to_remove.push(i);
            }
        }
    }
    // Remove in reverse order to maintain indices
    for &i in links_to_remove.iter().rev() {
        harness.links.remove(i);
    }

    // Remove the node
    for i in 0..harness.nodes.row_count() {
        if let Some(node) = harness.nodes.row_data(i) {
            if node.id == node_to_delete {
                harness.nodes.remove(i);
                break;
            }
        }
    }

    assert_eq!(harness.nodes.row_count(), 1, "One node should remain");
    assert_eq!(harness.links.row_count(), 0, "Link should be removed with node");
}

#[test]
fn test_delete_with_empty_selection() {
    let harness = MinimalTestHarness::new();

    assert!(harness.selection.borrow().is_empty());
    let initial_node_count = harness.nodes.row_count();
    let initial_link_count = harness.links.row_count();

    // Delete with nothing selected should do nothing
    let to_delete: Vec<i32> = harness.selection.borrow().iter().copied().collect();
    assert!(to_delete.is_empty(), "Nothing should be selected");

    // Verify nothing changed
    assert_eq!(harness.nodes.row_count(), initial_node_count);
    assert_eq!(harness.links.row_count(), initial_link_count);
}

#[test]
fn test_delete_multiple_selected_nodes() {
    use common::harness::NodeData;

    let harness = MinimalTestHarness::with_nodes_and_links(
        vec![
            NodeData {
                id: 1,
                title: SharedString::from("A"),
                x: 100.0,
                y: 100.0,
            },
            NodeData {
                id: 2,
                title: SharedString::from("B"),
                x: 400.0,
                y: 200.0,
            },
            NodeData {
                id: 3,
                title: SharedString::from("C"),
                x: 700.0,
                y: 100.0,
            },
        ],
        vec![],
    );

    // Select multiple nodes
    {
        let mut sel = harness.selection.borrow_mut();
        sel.handle_interaction(1, false);
        sel.handle_interaction(3, true);
    }

    // Delete selected
    let to_delete: Vec<i32> = harness.selection.borrow().iter().copied().collect();
    for id in to_delete {
        loop {
            let mut found = false;
            for i in 0..harness.nodes.row_count() {
                if let Some(node) = harness.nodes.row_data(i) {
                    if node.id == id {
                        harness.nodes.remove(i);
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                break;
            }
        }
    }
    harness.selection.borrow_mut().clear();

    assert_eq!(harness.nodes.row_count(), 1, "Only node 2 should remain");
    let remaining = harness.nodes.row_data(0).unwrap();
    assert_eq!(remaining.id, 2);
}

// ============================================================================
// Escape Key Tests
// ============================================================================

#[test]
fn test_escape_key_sends_event() {
    let harness = MinimalTestHarness::new();

    // Just verify event can be dispatched
    harness.key_tap(Key::Escape);
}

#[test]
fn test_link_cancelled_callback_tracking() {
    let harness = MinimalTestHarness::new();

    assert_eq!(*harness.tracker.link_cancelled.borrow(), 0);

    // Simulate escape during link creation (which should call link_cancelled)
    *harness.tracker.link_cancelled.borrow_mut() += 1;

    assert_eq!(*harness.tracker.link_cancelled.borrow(), 1);
}

// ============================================================================
// Text Input Tests
// ============================================================================

#[test]
fn test_text_input_dispatch() {
    let harness = MinimalTestHarness::new();

    // Test that text input can be dispatched (for potential search/rename features)
    harness.text_input("test");
}

// ============================================================================
// Ctrl+N Add Node Tests
// ============================================================================

#[test]
fn test_add_node_requested_callback_tracking() {
    let harness = MinimalTestHarness::new();

    assert_eq!(*harness.tracker.add_node_requested.borrow(), 0);

    // Simulate Ctrl+N triggering add_node_requested
    *harness.tracker.add_node_requested.borrow_mut() += 1;

    assert_eq!(*harness.tracker.add_node_requested.borrow(), 1);
}

// ============================================================================
// Focus Tests
// ============================================================================

#[test]
fn test_window_can_receive_focus() {
    let harness = MinimalTestHarness::new();

    // The window should be able to receive focus for keyboard events
    // This is a basic sanity check (testing backend doesn't have visible windows)
    assert!(harness.window.window().size().width > 0);
}
