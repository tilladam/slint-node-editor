//! Gesture arbitration and cancellation across editor input surfaces.

mod common;

use common::harness::{
    BoxSelectionModifier, DragState, LinkCreation, LinkCreationState, MinimalTestHarness,
};
use slint::{platform::Key, ComponentHandle, Model};

#[derive(Clone, Copy, Debug)]
enum Surface {
    Background,
    Link,
    Node,
    Pin,
    EmbeddedControl,
}

const SURFACES: [Surface; 5] = [
    Surface::Background,
    Surface::Link,
    Surface::Node,
    Surface::Pin,
    Surface::EmbeddedControl,
];

fn realize(harness: &MinimalTestHarness) {
    harness.window.show().unwrap();
    harness.pump_events();
    harness.pump_events();
    harness.window.invoke_focus_editor();
}

fn surface_point(harness: &MinimalTestHarness, surface: Surface) -> (f32, f32) {
    match surface {
        Surface::Background => (50.0, 50.0),
        Surface::Link => {
            let output = harness.pin_position(3).expect("node 1 output pin");
            let input = harness.pin_position(4).expect("node 2 input pin");
            ((output.0 + input.0) / 2.0, (output.1 + input.1) / 2.0)
        }
        Surface::Node => (120.0, 180.0),
        Surface::Pin => harness.pin_position(3).expect("node 1 output pin"),
        Surface::EmbeddedControl => (180.0, 124.0),
    }
}

fn assert_forced_marquee(modifier: BoxSelectionModifier, key: Key, surface: Surface) {
    let harness = MinimalTestHarness::new();
    harness.window.set_box_selection_modifier(modifier);
    realize(&harness);

    let start = surface_point(&harness, surface);
    harness.key_press(key);
    harness.mouse_down(start.0, start.1);
    assert!(
        harness.window.get_is_selecting(),
        "{modifier:?} did not reserve {surface:?}"
    );
    assert!(!harness.window.global::<DragState>().get_is_dragging());
    assert!(!harness.window.get_is_creating_link());

    harness.mouse_move(650.0, 450.0);
    assert!(harness.window.get_is_selecting());
    harness.mouse_up(650.0, 450.0);
    assert!(!harness.window.get_is_selecting());
    harness.key_release(key);
}

#[test]
fn configured_modifier_reserves_every_input_surface_for_marquee() {
    for (modifier, key) in [
        (BoxSelectionModifier::Ctrl, Key::Control),
        (BoxSelectionModifier::Alt, Key::Alt),
        (BoxSelectionModifier::Shift, Key::Shift),
    ] {
        for surface in SURFACES {
            assert_forced_marquee(modifier, key, surface);
        }
    }
}

#[test]
fn command_is_the_platform_alias_for_the_ctrl_marquee_setting() {
    for surface in SURFACES {
        assert_forced_marquee(BoxSelectionModifier::Ctrl, Key::Meta, surface);
    }
}

#[test]
fn node_and_pin_forward_reserved_gestures_when_modifier_press_was_not_captured() {
    for surface in [Surface::Node, Surface::Pin] {
        let harness = MinimalTestHarness::new();
        realize(&harness);
        harness.window.invoke_clear_editor_focus();

        let point = surface_point(&harness, surface);
        harness.key_press(Key::Control);
        harness.mouse_down(point.0, point.1);
        harness.mouse_move(650.0, 450.0);
        assert!(
            harness.window.get_is_selecting(),
            "{surface:?} did not forward the reserved gesture"
        );
        assert!(!harness.window.get_is_creating_link());
        assert!(!harness.window.global::<DragState>().get_is_dragging());
        harness.mouse_up(650.0, 450.0);
        harness.key_release(Key::Control);
        assert_idle(&harness);
    }
}

#[test]
fn ordinary_embedded_control_input_survives_every_configuration() {
    for modifier in [
        BoxSelectionModifier::None,
        BoxSelectionModifier::Ctrl,
        BoxSelectionModifier::Alt,
        BoxSelectionModifier::Shift,
    ] {
        let harness = MinimalTestHarness::new();
        harness.window.set_box_selection_modifier(modifier);
        realize(&harness);

        let point = surface_point(&harness, Surface::EmbeddedControl);
        harness.click(point.0, point.1);
        assert_eq!(
            harness.window.get_embedded_control_click_count(),
            1,
            "ordinary control click lost under {modifier:?} configuration"
        );
        assert!(!harness.window.get_is_selecting());
    }
}

#[test]
fn none_configuration_preserves_each_surfaces_normal_gesture() {
    let background = MinimalTestHarness::new();
    background
        .window
        .set_box_selection_modifier(BoxSelectionModifier::None);
    realize(&background);
    let point = surface_point(&background, Surface::Background);
    background.mouse_down(point.0, point.1);
    assert!(background.window.get_is_selecting());
    background.mouse_up(300.0, 300.0);

    let link = MinimalTestHarness::new();
    link.window
        .set_box_selection_modifier(BoxSelectionModifier::None);
    realize(&link);
    let point = surface_point(&link, Surface::Link);
    link.click(point.0, point.1);
    assert_eq!(link.selected_link_ids(), vec![1]);
    assert!(!link.window.get_is_selecting());

    let node = MinimalTestHarness::new();
    node.window
        .set_box_selection_modifier(BoxSelectionModifier::None);
    realize(&node);
    let point = surface_point(&node, Surface::Node);
    node.mouse_down(point.0, point.1);
    node.mouse_move(point.0 + 40.0, point.1 + 30.0);
    assert!(node.window.global::<DragState>().get_is_dragging());
    node.mouse_up(point.0 + 40.0, point.1 + 30.0);
    assert_eq!(node.nodes.row_data(0).unwrap().x, 140.0);

    let pin = MinimalTestHarness::new();
    pin.window
        .set_box_selection_modifier(BoxSelectionModifier::None);
    realize(&pin);
    let point = surface_point(&pin, Surface::Pin);
    pin.mouse_down(point.0, point.1);
    assert!(pin.window.get_is_creating_link());
    pin.pointer_exit();

    let control = MinimalTestHarness::new();
    control
        .window
        .set_box_selection_modifier(BoxSelectionModifier::None);
    realize(&control);
    let point = surface_point(&control, Surface::EmbeddedControl);
    control.click(point.0, point.1);
    assert_eq!(control.window.get_embedded_control_click_count(), 1);
}

fn start_marquee(harness: &MinimalTestHarness) {
    let point = surface_point(harness, Surface::Background);
    harness.mouse_down(point.0, point.1);
    harness.mouse_move(300.0, 300.0);
    assert!(harness.window.get_is_selecting());
}

fn start_node_drag(harness: &MinimalTestHarness) {
    let point = surface_point(harness, Surface::Node);
    harness.mouse_down(point.0, point.1);
    harness.mouse_move(point.0 + 40.0, point.1 + 30.0);
    assert!(harness.window.global::<DragState>().get_is_dragging());
}

fn start_link(harness: &MinimalTestHarness) {
    let point = surface_point(harness, Surface::Pin);
    harness.mouse_down(point.0, point.1);
    harness.mouse_move(point.0 + 30.0, point.1 + 30.0);
    assert!(harness.window.get_is_creating_link());
}

fn assert_idle(harness: &MinimalTestHarness) {
    assert!(!harness.window.get_is_selecting(), "marquee still active");
    assert!(
        !harness.window.global::<DragState>().get_is_dragging(),
        "node drag still active"
    );
    assert!(!harness.window.get_is_creating_link(), "link still active");
    assert_eq!(
        harness.window.global::<LinkCreation>().get_state(),
        LinkCreationState::Idle
    );
    assert_eq!(
        harness.ctrl.dragged_node_id(),
        0,
        "controller drag still active"
    );
}

#[test]
fn pointer_cancel_leaves_marquee_drag_and_link_idle() {
    let marquee = MinimalTestHarness::new();
    realize(&marquee);
    start_marquee(&marquee);
    marquee.pointer_exit();
    assert_idle(&marquee);

    let drag = MinimalTestHarness::new();
    realize(&drag);
    start_node_drag(&drag);
    drag.pointer_exit();
    assert_idle(&drag);
    assert_eq!(drag.nodes.row_data(0).unwrap().x, 100.0);
    assert_eq!(drag.ctrl.cache().borrow().node_rects[&1].x, 100.0);

    let link = MinimalTestHarness::new();
    realize(&link);
    start_link(&link);
    link.pointer_exit();
    assert_idle(&link);
    assert_eq!(*link.tracker.link_cancelled.borrow(), 1);
}

#[test]
fn release_outside_leaves_marquee_drag_and_link_idle() {
    let marquee = MinimalTestHarness::new();
    realize(&marquee);
    start_marquee(&marquee);
    marquee.mouse_up(900.0, 700.0);
    assert_idle(&marquee);

    let drag = MinimalTestHarness::new();
    realize(&drag);
    start_node_drag(&drag);
    drag.mouse_up(900.0, 700.0);
    assert_idle(&drag);

    let link = MinimalTestHarness::new();
    realize(&link);
    start_link(&link);
    link.mouse_up(900.0, 700.0);
    assert_idle(&link);
}

#[test]
fn focus_loss_leaves_marquee_drag_and_link_idle() {
    let marquee = MinimalTestHarness::new();
    realize(&marquee);
    start_marquee(&marquee);
    marquee.set_window_active(false);
    assert_idle(&marquee);

    let drag = MinimalTestHarness::new();
    realize(&drag);
    start_node_drag(&drag);
    drag.set_window_active(false);
    assert_idle(&drag);

    let link = MinimalTestHarness::new();
    realize(&link);
    start_link(&link);
    link.set_window_active(false);
    assert_idle(&link);
}

#[test]
fn public_removal_hook_leaves_marquee_drag_and_link_idle() {
    let marquee = MinimalTestHarness::new();
    realize(&marquee);
    start_marquee(&marquee);
    marquee.window.invoke_cancel_interactions();
    assert_idle(&marquee);

    let drag = MinimalTestHarness::new();
    realize(&drag);
    start_node_drag(&drag);
    drag.window.invoke_cancel_interactions();
    assert_idle(&drag);
    drag.mouse_move(500.0, 400.0);
    drag.mouse_up(500.0, 400.0);
    assert_idle(&drag);
    assert_eq!(drag.nodes.row_data(0).unwrap().x, 100.0);

    let link = MinimalTestHarness::new();
    realize(&link);
    start_link(&link);
    link.window.invoke_cancel_interactions();
    assert_idle(&link);
}

#[test]
fn hiding_or_disabling_the_editor_cancels_active_input() {
    let hidden = MinimalTestHarness::new();
    realize(&hidden);
    start_marquee(&hidden);
    hidden.window.set_editor_visible(false);
    hidden.pump_events();
    assert_idle(&hidden);

    let disabled = MinimalTestHarness::new();
    realize(&disabled);
    start_node_drag(&disabled);
    disabled.window.set_editor_enabled(false);
    disabled.pump_events();
    assert_idle(&disabled);
}
