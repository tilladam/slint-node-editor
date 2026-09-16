//! Keyboard focus through public pointer gestures and the standard wiring macro.
//! Adapted from Olivier de Gaalon's focus regressions in 8f6edbb.

mod common;

use common::harness::{BoxSelectionModifier, MinimalTestHarness};
use slint::{
    platform::{Key, PointerEventButton},
    ComponentHandle,
};

fn realize(harness: &MinimalTestHarness) {
    harness.window.set_focus_controls_enabled(true);
    harness.window.show().unwrap();
    harness.pump_events();
    harness.pump_events();
}

// Screen positions in the test UI: node 1 is 150x100 at (100, 100) with its
// field at (120..180, 170..190) and its output pin centred at (244, 150); the
// built-in minimap occupies the lower-right corner; the probe covers (700..800, 0..20).
const EMPTY_CANVAS: (f32, f32) = (400.0, 450.0);
const NODE_1_CENTER: (f32, f32) = (175.0, 150.0);
const NODE_1_FIELD: (f32, f32) = (150.0, 180.0);
const NODE_1_OUTPUT_PIN: (f32, f32) = (244.0, 150.0);
const MINIMAP: (f32, f32) = (690.0, 517.0);
const PROBE: (f32, f32) = (750.0, 10.0);

/// Which field holds keyboard focus.
#[derive(Debug, PartialEq)]
enum Focused {
    Probe,
    NodeField,
    Editor,
}

fn focused(harness: &MinimalTestHarness) -> Focused {
    let w = &harness.window;
    match (
        w.get_probe_has_focus(),
        w.get_node_field_has_focus(),
        w.get_editor_has_focus(),
    ) {
        (true, false, false) => Focused::Probe,
        (false, true, false) => Focused::NodeField,
        (false, false, true) => Focused::Editor,
        other => panic!("exactly one of (probe, node field, editor) must hold focus: {other:?}"),
    }
}

fn focus_probe(harness: &MinimalTestHarness) {
    harness.window.invoke_focus_probe();
    harness.pump_events();
    assert_eq!(
        focused(harness),
        Focused::Probe,
        "the press must start from a focused field"
    );
}

fn focus_node_field(harness: &MinimalTestHarness) {
    // The nodes are repeated, and a repeater instantiates only when something
    // walks the item tree; with no renderer, a pointer event is that walk.
    let (x, y) = EMPTY_CANVAS;
    harness.mouse_move(x, y);
    harness.window.invoke_focus_node_field();
    harness.pump_events();
    assert_eq!(
        focused(harness),
        Focused::NodeField,
        "the press must start from a focused field"
    );
}

fn press(harness: &MinimalTestHarness, (x, y): (f32, f32), button: PointerEventButton) {
    harness.mouse_down_button(x, y, button);
    harness.mouse_up_button(x, y, button);
}

#[test]
fn test_press_on_canvas_focuses_editor() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    focus_probe(&harness);
    press(&harness, EMPTY_CANVAS, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::Editor);
}

#[test]
fn test_right_press_on_canvas_focuses_editor() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    focus_probe(&harness);
    press(&harness, EMPTY_CANVAS, PointerEventButton::Right);
    assert_eq!(focused(&harness), Focused::Editor);
}

#[test]
fn test_middle_press_on_node_focuses_editor() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    focus_node_field(&harness);
    press(&harness, NODE_1_CENTER, PointerEventButton::Middle);
    assert_eq!(focused(&harness), Focused::Editor);
}

#[test]
fn test_press_on_node_takes_focus_from_its_own_field() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    focus_node_field(&harness);
    press(&harness, NODE_1_CENTER, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::Editor);
}

#[test]
fn test_press_on_pin_focuses_editor() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    focus_node_field(&harness);
    press(&harness, NODE_1_OUTPUT_PIN, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::Editor);
}

#[test]
fn test_press_on_minimap_focuses_editor() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    focus_probe(&harness);
    press(&harness, MINIMAP, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::Editor);
}

#[test]
fn test_press_on_a_field_keeps_its_focus() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    focus_probe(&harness);
    press(&harness, PROBE, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::Probe);

    focus_node_field(&harness);
    press(&harness, NODE_1_FIELD, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::NodeField);
}

/// The editor takes focus inside the press itself, so a focus the embedder
/// takes in response to that press — here on the selection the press asks
/// for, which reaches the host only after the event — is the one left
/// standing.
#[test]
fn test_focus_taken_in_response_to_a_press_wins() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    let w = harness.window.as_weak();
    harness.window.on_node_selected(move |_, _| {
        if let Some(w) = w.upgrade() {
            w.invoke_focus_probe();
        }
    });

    press(&harness, NODE_1_CENTER, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::Probe);
}

#[test]
fn reserved_overlay_takes_focus_from_embedded_field_for_each_button() {
    for (modifier, key) in [
        (BoxSelectionModifier::Ctrl, Key::Control),
        (BoxSelectionModifier::Ctrl, Key::Meta),
        (BoxSelectionModifier::Alt, Key::Alt),
        (BoxSelectionModifier::Shift, Key::Shift),
    ] {
        for button in [
            PointerEventButton::Left,
            PointerEventButton::Middle,
            PointerEventButton::Right,
        ] {
            let harness = MinimalTestHarness::new();
            realize(&harness);
            harness.window.set_box_selection_modifier(modifier);
            focus_node_field(&harness);
            harness.key_press(key);
            // The modifier reserves this press before the TextInput can accept it.
            harness.mouse_down_button(NODE_1_FIELD.0, NODE_1_FIELD.1, button);
            assert_eq!(
                focused(&harness),
                Focused::Editor,
                "{modifier:?}, {button:?}"
            );
            if button == PointerEventButton::Left {
                assert!(harness.window.get_is_selecting());
            }
            harness.mouse_up_button(NODE_1_FIELD.0, NODE_1_FIELD.1, button);
            harness.key_release(key);
            assert!(!harness.window.get_is_selecting());
        }
    }
}

#[test]
fn press_can_transfer_focus_into_an_embedded_field() {
    let harness = MinimalTestHarness::new();
    realize(&harness);
    focus_probe(&harness);
    press(&harness, NODE_1_FIELD, PointerEventButton::Left);
    assert_eq!(focused(&harness), Focused::NodeField);
}

#[test]
fn context_menu_handler_can_focus_another_control() {
    let harness = MinimalTestHarness::new();
    realize(&harness);
    let w = harness.window.as_weak();
    harness.window.on_context_menu_requested(move || {
        if let Some(w) = w.upgrade() {
            w.invoke_focus_probe();
        }
    });
    press(&harness, EMPTY_CANVAS, PointerEventButton::Right);
    assert_eq!(focused(&harness), Focused::Probe);
}
