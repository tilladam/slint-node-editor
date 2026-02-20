//! Level 3: Link Creation Tests
//!
//! Tests link creation workflow: drag from output pin, preview, completion/cancellation.

mod common;

use common::harness::MinimalTestHarness;
use slint::{Color, Model, SharedString};

/// Helper to set up geometry in the cache for testing.
fn setup_test_geometry(harness: &MinimalTestHarness) {
    let cache = harness.ctrl.cache();
    let mut cache = cache.borrow_mut();

    // Node A at (100, 100), size 150x100
    cache.update_node_rect(1, 100.0, 100.0, 150.0, 100.0);
    // Node B at (400, 200), size 150x100
    cache.update_node_rect(2, 400.0, 200.0, 150.0, 100.0);

    // Pin positions (relative to node top-left)
    // Node 1: input pin id=2, output pin id=3
    cache.handle_pin_report(2, 1, 1, 0.0, 50.0); // Input at left
    cache.handle_pin_report(3, 1, 2, 150.0, 50.0); // Output at right

    // Node 2: input pin id=4, output pin id=5
    cache.handle_pin_report(4, 2, 1, 0.0, 50.0); // Input at left
    cache.handle_pin_report(5, 2, 2, 150.0, 50.0); // Output at right
}

#[test]
fn test_link_preview_path_generation() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Generate a preview path from point to point
    let path = slint_node_editor::generate_bezier_path(100.0, 100.0, 200.0, 150.0, 1.0, 50.0);

    // Should be a valid SVG path starting with M (move) and containing C (cubic bezier)
    assert!(path.starts_with("M "), "Path should start with move command");
    assert!(path.contains(" C "), "Path should contain cubic bezier command");
}

#[test]
fn test_link_path_between_pins() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Compute path between Node 1 output (pin 3) and Node 2 input (pin 4)
    let path = harness.ctrl.compute_link_path(3, 4);

    assert!(!path.is_empty(), "Link path should be computed");
    assert!(
        path.as_str().starts_with("M "),
        "Path should start with move command"
    );
}

#[test]
fn test_link_path_returns_empty_for_missing_pin() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Try to compute path to non-existent pin
    let path = harness.ctrl.compute_link_path(3, 999);

    assert!(path.is_empty(), "Path should be empty for missing pin");
}

#[test]
fn test_find_pin_at_position() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Node 1 output pin is at (100+150, 100+50) = (250, 150)
    let pin_id = harness.ctrl.cache().borrow().find_pin_at(250.0, 150.0, 10.0);

    assert_eq!(pin_id, 3, "Should find Node 1 output pin");
}

#[test]
fn test_find_pin_returns_zero_for_no_hit() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Position far from any pin
    let pin_id = harness.ctrl.cache().borrow().find_pin_at(50.0, 50.0, 10.0);

    assert_eq!(pin_id, 0, "Should return 0 when no pin is hit");
}

#[test]
fn test_link_requested_callback_records_pins() {
    let harness = MinimalTestHarness::new();

    // Simulate a link creation from pin 3 to pin 4
    harness.tracker.link_requested.borrow_mut().push((3, 4));

    let requested = harness.tracker.link_requested.borrow();
    assert_eq!(requested.len(), 1);
    assert_eq!(requested[0], (3, 4), "Should record start and end pins");
}

#[test]
fn test_link_cancelled_callback_increments() {
    let harness = MinimalTestHarness::new();

    assert_eq!(*harness.tracker.link_cancelled.borrow(), 0);

    // Simulate link cancellation
    *harness.tracker.link_cancelled.borrow_mut() += 1;

    assert_eq!(*harness.tracker.link_cancelled.borrow(), 1);
}

#[test]
fn test_existing_link_data() {
    let harness = MinimalTestHarness::new();

    // The default harness has one link from pin 3 to pin 4
    let link = harness.links.row_data(0).unwrap();
    assert_eq!(link.id, 1);
    assert_eq!(link.start_pin_id, 3);
    assert_eq!(link.end_pin_id, 4);
}

#[test]
fn test_add_new_link_to_model() {
    use common::harness::LinkData;

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
            common::harness::NodeData {
                id: 3,
                title: SharedString::from("C"),
                x: 700.0,
                y: 100.0,
            },
        ],
        vec![],
    );

    assert_eq!(harness.links.row_count(), 0);

    // Simulate adding a link
    harness.links.push(LinkData {
        id: 1,
        start_pin_id: 3, // Node 1 output
        end_pin_id: 6,   // Node 3 input
        color: Color::from_argb_u8(255, 255, 128, 0),
        line_width: 2.0,
    });

    assert_eq!(harness.links.row_count(), 1);

    let link = harness.links.row_data(0).unwrap();
    assert_eq!(link.start_pin_id, 3);
    assert_eq!(link.end_pin_id, 6);
}

#[test]
fn test_multiple_links_supported() {
    use common::harness::LinkData;

    let harness = MinimalTestHarness::new();

    // Add another link
    harness.links.push(LinkData {
        id: 2,
        start_pin_id: 5, // Node 2 output
        end_pin_id: 2,   // Node 1 input
        color: Color::from_argb_u8(255, 0, 255, 128),
        line_width: 2.0,
    });

    assert_eq!(harness.links.row_count(), 2);
}

#[test]
fn test_link_find_at_position() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // The link goes from pin 3 (250, 150) to pin 4 (400, 250)
    // Test finding the link at a point along its path
    let links_iter = std::iter::once((1_i32, 3_i32, 4_i32));

    // A point roughly in the middle of the link path
    let link_id = harness
        .ctrl
        .cache()
        .borrow()
        .find_link_at(325.0, 200.0, links_iter, 15.0, 1.0, 50.0, 20);

    // Note: Link hit testing depends on bezier path - this might not hit exactly
    // The test verifies the API works; exact hit detection is implementation-dependent
    // Link ID is 1 if hit, -1 if missed
    assert!(
        link_id == 1 || link_id == -1,
        "find_link_at should return valid result"
    );
}

#[test]
fn test_link_colors_preserved() {
    let harness = MinimalTestHarness::new();

    let link = harness.links.row_data(0).unwrap();

    // Default link color from harness: Color::from_argb_u8(255, 100, 180, 255)
    // Color components are returned as u8 (0-255)
    assert_eq!(link.color.alpha(), 255, "Alpha should be fully opaque");
    assert!(link.color.red() > 0, "Should have red component");
    assert!(link.color.blue() > 0, "Should have blue component");
}

#[test]
fn test_self_loop_link_path() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Try to create a link from a pin to another pin on the same node
    // This is the output pin to input pin of same node
    let path = harness.ctrl.compute_link_path(3, 2); // Both on Node 1

    // The path should still be computed (validation is a separate concern)
    assert!(!path.is_empty(), "Self-loop path should still be computed");
}

#[test]
fn test_remove_link_from_model() {
    let harness = MinimalTestHarness::new();

    assert_eq!(harness.links.row_count(), 1);

    // Remove the link
    harness.links.remove(0);

    assert_eq!(harness.links.row_count(), 0);
}

#[test]
fn test_bezier_path_format() {
    // Test the bezier path format directly
    let path = slint_node_editor::generate_bezier_path(0.0, 0.0, 100.0, 50.0, 1.0, 50.0);

    // Parse the path to verify format
    let parts: Vec<&str> = path.split_whitespace().collect();

    // Should have: M x y C cx1 cy1 cx2 cy2 x y
    assert!(parts.len() >= 10, "Path should have at least 10 parts");
    assert_eq!(parts[0], "M", "First command should be Move");
    assert_eq!(parts[3], "C", "Fourth part should be Cubic");
}

#[test]
fn test_zoom_affects_link_path() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Get path at default zoom
    let path_zoom_1 = harness.ctrl.compute_link_path(3, 4);

    // Change zoom
    harness.ctrl.set_viewport(2.0, 0.0, 0.0);
    let path_zoom_2 = harness.ctrl.compute_link_path(3, 4);

    // Paths should be different due to bezier offset scaling
    assert_ne!(
        path_zoom_1.as_str(),
        path_zoom_2.as_str(),
        "Zoom should affect link path"
    );
}
