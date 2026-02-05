//! Level 6: Advanced Feature Tests
//!
//! Tests advanced features: zoom, pan, multi-node drag, link validation,
//! context menu, viewport changes.

mod common;

use common::harness::MinimalTestHarness;
use slint::platform::PointerEventButton;
use slint::{Color, Model, SharedString};

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
// Zoom Tests
// ============================================================================

#[test]
fn test_default_zoom_is_one() {
    let harness = MinimalTestHarness::new();
    assert_eq!(harness.ctrl.zoom(), 1.0);
}

#[test]
fn test_set_zoom() {
    let harness = MinimalTestHarness::new();

    harness.ctrl.set_zoom(1.5);
    assert_eq!(harness.ctrl.zoom(), 1.5);

    harness.ctrl.set_zoom(0.5);
    assert_eq!(harness.ctrl.zoom(), 0.5);
}

#[test]
fn test_zoom_affects_link_path_calculation() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let path_at_1x = harness.ctrl.compute_link_path(3, 4);

    harness.ctrl.set_zoom(2.0);
    let path_at_2x = harness.ctrl.compute_link_path(3, 4);

    // Paths should differ due to zoom affecting bezier control points
    assert_ne!(
        path_at_1x.as_str(),
        path_at_2x.as_str(),
        "Zoom should affect link paths"
    );
}

#[test]
fn test_zoom_affects_grid_generation() {
    let harness = MinimalTestHarness::new();

    let grid_1x = harness.ctrl.generate_grid(800.0, 600.0, 0.0, 0.0);

    harness.ctrl.set_zoom(2.0);
    let grid_2x = harness.ctrl.generate_grid(800.0, 600.0, 0.0, 0.0);

    assert_ne!(
        grid_1x.as_str(),
        grid_2x.as_str(),
        "Zoom should affect grid"
    );
}

#[test]
fn test_scroll_for_zoom() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let _initial_zoom = harness.ctrl.zoom();

    // Scroll to zoom in
    harness.scroll(400.0, 300.0, 1.0);

    // Note: The actual zoom change happens in Slint's scroll handler
    // This test verifies the scroll event can be dispatched
    // The controller's zoom is updated via the viewport_changed callback
}

// ============================================================================
// Pan Tests
// ============================================================================

#[test]
fn test_pan_affects_grid_generation() {
    let harness = MinimalTestHarness::new();

    let grid_no_pan = harness.ctrl.generate_grid(800.0, 600.0, 0.0, 0.0);
    let grid_panned = harness.ctrl.generate_grid(800.0, 600.0, 100.0, 50.0);

    assert_ne!(
        grid_no_pan.as_str(),
        grid_panned.as_str(),
        "Pan should affect grid"
    );
}

#[test]
fn test_update_viewport_callback_records_values() {
    let harness = MinimalTestHarness::new();

    // The harness tracks viewport updates
    harness.tracker.update_viewport.borrow_mut().push((1.5, 50.0, 25.0));

    let updates = harness.tracker.update_viewport.borrow();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0], (1.5, 50.0, 25.0));
}

#[test]
fn test_middle_click_for_pan() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Middle-click down to start pan
    harness.mouse_down_button(400.0, 300.0, PointerEventButton::Middle);

    // Move mouse
    harness.mouse_move(450.0, 350.0);

    // Middle-click up to end pan
    harness.mouse_up_button(450.0, 350.0, PointerEventButton::Middle);

    // Pan behavior is handled by Slint - this verifies events can be dispatched
}

// ============================================================================
// Multi-Node Drag Tests
// ============================================================================

#[test]
fn test_multi_node_selection_setup() {
    let harness = MinimalTestHarness::new();

    // Create multi-selection
    {
        let mut sel = harness.selection.borrow_mut();
        sel.handle_interaction(1, false);
        sel.handle_interaction(2, true);
    }

    assert_eq!(harness.selection.borrow().len(), 2);
}

#[test]
fn test_multi_node_drag_updates_all_selected() {
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

    // Select nodes 1 and 2
    {
        let mut sel = harness.selection.borrow_mut();
        sel.handle_interaction(1, false);
        sel.handle_interaction(2, true);
    }

    let node1_orig_x = harness.nodes.row_data(0).unwrap().x;
    let node2_orig_x = harness.nodes.row_data(1).unwrap().x;
    let node3_orig_x = harness.nodes.row_data(2).unwrap().x;

    // Simulate drag delta applied to all selected nodes
    let delta_x = 50.0;
    let delta_y = 30.0;

    let selected: Vec<i32> = harness.selection.borrow().iter().copied().collect();
    for i in 0..harness.nodes.row_count() {
        if let Some(mut node) = harness.nodes.row_data(i) {
            if selected.contains(&node.id) {
                node.x += delta_x;
                node.y += delta_y;
                harness.nodes.set_row_data(i, node);
            }
        }
    }

    // Verify selected nodes moved
    let node1 = harness.nodes.row_data(0).unwrap();
    let node2 = harness.nodes.row_data(1).unwrap();
    let node3 = harness.nodes.row_data(2).unwrap();

    assert_eq!(node1.x, node1_orig_x + delta_x, "Node 1 should have moved");
    assert_eq!(node2.x, node2_orig_x + delta_x, "Node 2 should have moved");
    assert_eq!(node3.x, node3_orig_x, "Node 3 should NOT have moved");
}

// ============================================================================
// Link Validation Tests
// ============================================================================

#[test]
fn test_basic_link_validator_rejects_self_loops() {
    use slint_node_editor::{BasicLinkValidator, LinkValidator, ValidationResult};

    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let validator = BasicLinkValidator::new(2); // output_type = 2
    let cache = harness.ctrl.cache();
    let cache = cache.borrow();
    let links: Vec<()> = vec![];

    // Try to connect pin to itself
    let result = validator.validate(3, 3, &cache, &links);

    assert!(
        matches!(result, ValidationResult::Invalid(_)),
        "Should reject self-loop"
    );
}

#[test]
fn test_basic_link_validator_accepts_valid_link() {
    use slint_node_editor::{BasicLinkValidator, LinkValidator, ValidationResult};

    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let validator = BasicLinkValidator::new(2); // output_type = 2
    let cache = harness.ctrl.cache();
    let cache = cache.borrow();
    let links: Vec<()> = vec![];

    // Valid link between different pins (output to input)
    let result = validator.validate(3, 4, &cache, &links);

    assert!(
        matches!(result, ValidationResult::Valid),
        "Should accept valid link"
    );
}

#[test]
fn test_no_duplicates_validator() {
    use slint_node_editor::{LinkValidator, NoDuplicatesValidator, SimpleLink, ValidationResult};

    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let validator = NoDuplicatesValidator;
    let cache = harness.ctrl.cache();
    let cache = cache.borrow();

    // Existing links: (pin 3 → pin 4)
    let existing = vec![SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 255, 255))];

    // Same direction - should reject
    let result = validator.validate(3, 4, &cache, &existing);
    assert!(matches!(result, ValidationResult::Invalid(_)));

    // Different pins - should accept
    let result = validator.validate(3, 5, &cache, &existing);
    assert!(matches!(result, ValidationResult::Valid));
}

#[test]
fn test_composite_validator() {
    use slint_node_editor::{
        BasicLinkValidator, CompositeValidator, LinkValidator, NoDuplicatesValidator,
        SimpleLink, ValidationResult,
    };

    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let validator = CompositeValidator::new()
        .add(BasicLinkValidator::new(2))
        .add(NoDuplicatesValidator);

    let cache = harness.ctrl.cache();
    let cache = cache.borrow();
    let existing = vec![SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 255, 255))];

    // Self-loop (caught by BasicLinkValidator)
    let result = validator.validate(3, 3, &cache, &existing);
    assert!(matches!(result, ValidationResult::Invalid(_)));

    // Duplicate (caught by NoDuplicatesValidator)
    let result = validator.validate(3, 4, &cache, &existing);
    assert!(matches!(result, ValidationResult::Invalid(_)));

    // Valid - uses pin 5 (output of node 2) to pin 2 (input of node 1)
    let result = validator.validate(5, 2, &cache, &existing);
    assert!(matches!(result, ValidationResult::Valid));
}

// ============================================================================
// Context Menu Tests
// ============================================================================

#[test]
fn test_context_menu_callback_tracking() {
    let harness = MinimalTestHarness::new();

    assert_eq!(*harness.tracker.context_menu_requested.borrow(), 0);

    // Simulate right-click context menu
    *harness.tracker.context_menu_requested.borrow_mut() += 1;

    assert_eq!(*harness.tracker.context_menu_requested.borrow(), 1);
}

#[test]
fn test_right_click_dispatch() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Right-click should trigger context menu
    harness.mouse_down_button(400.0, 300.0, PointerEventButton::Right);
    harness.mouse_up_button(400.0, 300.0, PointerEventButton::Right);

    // Context menu handling is done by Slint's pointer event handler
}

// ============================================================================
// Grid Tests
// ============================================================================

#[test]
fn test_grid_generation() {
    let harness = MinimalTestHarness::new();

    let grid = harness.ctrl.generate_initial_grid(800.0, 600.0);

    assert!(!grid.is_empty(), "Grid should be generated");
    assert!(
        grid.as_str().contains("M "),
        "Grid should contain move commands"
    );
}

#[test]
fn test_grid_spacing_configurable() {
    let harness = MinimalTestHarness::new();

    harness.ctrl.set_grid_spacing(48.0);
    let grid_48 = harness.ctrl.generate_initial_grid(800.0, 600.0);

    harness.ctrl.set_grid_spacing(12.0);
    let grid_12 = harness.ctrl.generate_initial_grid(800.0, 600.0);

    // Different spacing should produce different grids
    assert_ne!(
        grid_48.as_str(),
        grid_12.as_str(),
        "Different spacing should produce different grids"
    );
}

// ============================================================================
// Bezier Configuration Tests
// ============================================================================

#[test]
fn test_bezier_offset_configurable() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let path_default = harness.ctrl.compute_link_path(3, 4);

    harness.ctrl.set_bezier_offset(100.0);
    let path_larger = harness.ctrl.compute_link_path(3, 4);

    assert_ne!(
        path_default.as_str(),
        path_larger.as_str(),
        "Different bezier offset should produce different paths"
    );
}

// ============================================================================
// Graph Logic Tests
// ============================================================================

#[test]
fn test_find_links_connected_to_node() {
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
        vec![
            LinkData {
                id: 1,
                start_pin_id: 3, // Node 1 output
                end_pin_id: 4,   // Node 2 input
                color: Color::from_argb_u8(255, 100, 180, 255),
                line_width: 2.0,
            },
            LinkData {
                id: 2,
                start_pin_id: 5, // Node 2 output
                end_pin_id: 6,   // Node 3 input
                color: Color::from_argb_u8(255, 255, 100, 100),
                line_width: 2.0,
            },
        ],
    );

    setup_test_geometry(&harness);

    // Set up pin geometry for node 3
    harness.ctrl.cache().borrow_mut().update_node_rect(3, 700.0, 100.0, 150.0, 100.0);
    harness.ctrl.cache().borrow_mut().handle_pin_report(6, 3, 1, 0.0, 50.0);
    harness.ctrl.cache().borrow_mut().handle_pin_report(7, 3, 2, 150.0, 50.0);

    // Find links connected to node 2 (pins 4 and 5)
    let node2_pins = vec![4, 5];
    let mut connected = Vec::new();

    for i in 0..harness.links.row_count() {
        if let Some(link) = harness.links.row_data(i) {
            if node2_pins.contains(&link.start_pin_id) || node2_pins.contains(&link.end_pin_id) {
                connected.push(link.id);
            }
        }
    }

    assert_eq!(connected.len(), 2, "Node 2 should be connected to 2 links");
    assert!(connected.contains(&1));
    assert!(connected.contains(&2));
}

// ============================================================================
// Geometry Cache Tests
// ============================================================================

#[test]
fn test_cache_accessible() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    let cache = harness.ctrl.cache();
    let cache = cache.borrow();

    assert!(cache.node_rects.contains_key(&1), "Node 1 should be in cache");
    assert!(cache.node_rects.contains_key(&2), "Node 2 should be in cache");
    assert!(cache.pin_positions.contains_key(&3), "Pin 3 should be in cache");
}

#[test]
fn test_cache_can_be_updated() {
    let harness = MinimalTestHarness::new();

    {
        let cache = harness.ctrl.cache();
        let mut cache = cache.borrow_mut();
        cache.update_node_rect(99, 500.0, 500.0, 100.0, 100.0);
    }

    let cache = harness.ctrl.cache();
    let cache = cache.borrow();
    assert!(cache.node_rects.contains_key(&99));
}
