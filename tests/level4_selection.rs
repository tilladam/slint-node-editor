//! Level 4: Selection Tests
//!
//! The editor keeps no selection state: it reports gestures and renders the
//! `selected` flag it finds on the rows. So every test here drives the real
//! surface — a pointer gesture, or the intent a gesture emits — and asserts on
//! the rows, which are where the selection actually lives. Nothing pokes a
//! selection store directly, because there isn't one; the resolution semantics
//! themselves are unit-tested in `src/selection.rs`.

mod common;

use common::harness::MinimalTestHarness;
use slint::SharedString;

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
// Node click intents → the rows
// ============================================================================

#[test]
fn click_marks_the_row_and_leaves_the_others() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);

    assert_eq!(harness.selected_node_ids(), vec![1]);
    assert!(harness.node_data(1).unwrap().selected, "row 1 selected");
    assert!(!harness.node_data(2).unwrap().selected, "row 2 untouched");
}

#[test]
fn click_replaces_the_previous_selection() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);
    harness.window.invoke_node_selected(2, false);

    assert_eq!(harness.selected_node_ids(), vec![2]);
}

#[test]
fn click_on_the_only_selected_node_keeps_it_selected() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);
    harness.window.invoke_node_selected(1, false);

    assert_eq!(harness.selected_node_ids(), vec![1]);
}

#[test]
fn shift_click_extends_then_toggles_the_rows() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);
    harness.window.invoke_node_selected(2, true);

    assert_eq!(harness.selected_node_ids(), vec![1, 2]);

    // Shift on an already-selected node toggles it back off
    harness.window.invoke_node_selected(1, true);

    assert_eq!(harness.selected_node_ids(), vec![2]);
}

#[test]
fn plain_click_collapses_a_multi_selection() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);
    harness.window.invoke_node_selected(2, true);

    harness.window.invoke_node_selected(2, false);

    assert_eq!(harness.selected_node_ids(), vec![2]);
}

#[test]
fn shift_clicking_everything_off_empties_the_selection() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);
    harness.window.invoke_node_selected(2, true);
    harness.window.invoke_node_selected(1, true);
    harness.window.invoke_node_selected(2, true);

    assert!(harness.selected_node_ids().is_empty());
}

#[test]
fn clearing_drops_every_row() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);
    harness.window.invoke_node_selected(2, true);

    harness.window.invoke_selection_cleared();

    assert!(harness.selected_node_ids().is_empty());
    assert!(!harness.node_data(1).unwrap().selected);
    assert!(!harness.node_data(2).unwrap().selected);
}

// ============================================================================
// Marquee commit intents
// ============================================================================

#[test]
fn box_commit_replaces_the_selection() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    harness.window.invoke_node_selected(2, false);

    // A marquee over node 1 only — without shift it replaces node 2's selection
    harness
        .window
        .invoke_box_selection_committed(50.0, 50.0, 200.0, 200.0, false);

    assert_eq!(harness.selected_node_ids(), vec![1]);
}

#[test]
fn box_commit_with_shift_extends_the_selection() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    harness.window.invoke_node_selected(2, false);

    harness
        .window
        .invoke_box_selection_committed(50.0, 50.0, 200.0, 200.0, true);

    assert_eq!(harness.selected_node_ids(), vec![1, 2], "kept by shift");
}

#[test]
fn box_commit_over_empty_space_clears() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    harness.window.invoke_node_selected(1, false);

    harness
        .window
        .invoke_box_selection_committed(600.0, 50.0, 100.0, 100.0, false);

    assert!(harness.selected_node_ids().is_empty());
}

#[test]
fn box_commit_takes_every_node_it_covers() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Node 1: (100, 100)-(250, 200); node 2: (400, 200)-(550, 300)
    harness
        .window
        .invoke_box_selection_committed(50.0, 50.0, 600.0, 300.0, false);

    assert_eq!(harness.selected_node_ids(), vec![1, 2]);
}

// ============================================================================
// Real pointer gestures → intents
//
// The tests above start at the intent. These start at the pointer, so they
// cover the part no intent-level test can: that the editor actually emits the
// intent, with the rectangle and the shift state it promises.
// ============================================================================

#[test]
fn dragging_on_empty_canvas_emits_a_marquee_commit() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Press on empty canvas, drag across node 1, release.
    harness.mouse_down(60.0, 60.0);
    harness.mouse_move(260.0, 260.0);
    harness.mouse_up(260.0, 260.0);

    assert_eq!(
        harness.selected_node_ids(),
        vec![1],
        "the marquee should have committed the node it covered"
    );
}

#[test]
fn a_shift_marquee_extends_instead_of_replacing() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    // Node 2 selected up front, by intent.
    harness.window.invoke_node_selected(2, false);

    // Now marquee over node 1 with shift held for the whole gesture. The
    // release event carries no modifiers of its own, so this also pins down
    // that the editor captured shift at press and replayed it at commit.
    harness.shift_down();
    harness.mouse_down(60.0, 60.0);
    harness.mouse_move(260.0, 260.0);
    harness.mouse_up(260.0, 260.0);
    harness.shift_up();

    assert_eq!(
        harness.selected_node_ids(),
        vec![1, 2],
        "shift+marquee extends rather than replacing"
    );
}

#[test]
fn a_marquee_release_without_shift_replaces() {
    let harness = MinimalTestHarness::new();
    setup_test_geometry(&harness);

    harness.window.invoke_node_selected(2, false);

    harness.mouse_down(60.0, 60.0);
    harness.mouse_move(260.0, 260.0);
    harness.mouse_up(260.0, 260.0);

    assert_eq!(harness.selected_node_ids(), vec![1]);
}

// ============================================================================
// Link selection
// ============================================================================

#[test]
fn clicking_a_link_marks_its_row() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_select_link(1, false);

    assert_eq!(harness.selected_link_ids(), vec![1]);
}

#[test]
fn selecting_a_link_drops_the_node_selection() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_node_selected(1, false);
    harness.window.invoke_select_link(1, false);

    assert!(
        harness.selected_node_ids().is_empty(),
        "a plain click is exclusive across kinds"
    );
    assert_eq!(harness.selected_link_ids(), vec![1]);
}

#[test]
fn selecting_a_node_drops_the_link_selection() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_select_link(1, false);
    harness.window.invoke_node_selected(1, false);

    assert!(harness.selected_link_ids().is_empty());
    assert_eq!(harness.selected_node_ids(), vec![1]);
}

/// Shift means "add to what I have", and that holds across kinds: a
/// shift-click reaches into the other model only to leave it alone. A
/// shift-marquee already builds mixed selections, and a delete acts on the
/// mixed set — a shift-click that silently dropped half of it would be the
/// odd one out.
#[test]
fn shift_click_builds_a_mixed_node_and_link_selection() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_select_link(1, false);
    harness.window.invoke_node_selected(1, true);

    assert_eq!(harness.selected_node_ids(), vec![1]);
    assert_eq!(
        harness.selected_link_ids(),
        vec![1],
        "shift must not drop the other kind"
    );

    // And the mirror: shift-clicking a link keeps the selected nodes.
    harness.window.invoke_node_selected(2, true);
    harness.window.invoke_select_link(1, true);
    assert_eq!(harness.selected_node_ids(), vec![1, 2]);
    assert!(
        harness.selected_link_ids().is_empty(),
        "shift toggles within the clicked kind — link 1 was selected, so it drops"
    );
}

#[test]
fn clearing_drops_links_too() {
    let harness = MinimalTestHarness::new();

    harness.window.invoke_select_link(1, false);
    harness.window.invoke_selection_cleared();

    assert!(harness.selected_link_ids().is_empty());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn negative_node_ids_select() {
    let harness = MinimalTestHarness::with_nodes_and_links(
        vec![common::harness::NodeData {
            id: -1,
            title: SharedString::from("Negative"),
            x: 0.0,
            y: 0.0,
            selected: false,
        }],
        vec![],
    );

    harness.window.invoke_node_selected(-1, false);

    assert_eq!(harness.selected_node_ids(), vec![-1]);
}

#[test]
fn a_zero_node_id_selects() {
    let harness = MinimalTestHarness::with_nodes_and_links(
        vec![common::harness::NodeData {
            id: 0,
            title: SharedString::from("Zero"),
            x: 0.0,
            y: 0.0,
            selected: false,
        }],
        vec![],
    );

    harness.window.invoke_node_selected(0, false);

    assert_eq!(harness.selected_node_ids(), vec![0]);
}

#[test]
fn selection_scales_to_many_nodes() {
    let nodes: Vec<common::harness::NodeData> = (1..=100)
        .map(|i| common::harness::NodeData {
            id: i,
            title: SharedString::from(format!("Node {}", i)),
            x: (i as f32) * 150.0,
            y: 100.0,
            selected: false,
        })
        .collect();

    let harness = MinimalTestHarness::with_nodes_and_links(nodes, vec![]);

    harness.window.invoke_node_selected(1, false);
    for id in 2..=100 {
        harness.window.invoke_node_selected(id, true);
    }

    assert_eq!(harness.selected_node_ids().len(), 100);
}
