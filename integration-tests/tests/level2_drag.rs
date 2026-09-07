//! Level 2: node drag tests through real pointer input.

mod common;

use common::harness::MinimalTestHarness;
use slint::{ComponentHandle, Model};

fn realize(harness: &MinimalTestHarness) {
    harness.window.show().unwrap();
    harness.pump_events();
    harness.pump_events();
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn pointer_drag_commits_to_the_model_and_reports_the_real_gesture() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    let start = harness.node_center(1).expect("rendered node center");
    harness.drag(start.0, start.1, start.0 + 48.0, start.1 + 24.0);

    let moved = harness.node_data(1).unwrap();
    assert_close(moved.x, 148.0);
    assert_close(moved.y, 124.0);
    assert_eq!(&*harness.tracker.node_drag_started.borrow(), &[1]);
    assert_eq!(harness.tracker.node_drag_ended.borrow().len(), 1);
    let delta = harness.tracker.node_drag_ended.borrow()[0];
    assert_close(delta.0, 48.0);
    assert_close(delta.1, 24.0);
    assert_eq!(harness.ctrl.dragged_node_id(), 0);
}

#[test]
fn pointer_drag_converts_screen_delta_at_non_unit_zoom_and_pan() {
    let harness = MinimalTestHarness::new();
    harness.window.set_zoom(1.5);
    harness.window.set_pan_x(31.0);
    harness.window.set_pan_y(-17.0);
    realize(&harness);

    let start = harness.node_center(1).expect("rendered node center");
    assert_close(start.0, 293.5);
    assert_close(start.1, 208.0);
    harness.drag(start.0, start.1, start.0 + 60.0, start.1 + 45.0);

    let moved = harness.node_data(1).unwrap();
    assert_close(moved.x, 140.0);
    assert_close(moved.y, 130.0);
    let delta = harness.tracker.node_drag_ended.borrow()[0];
    assert_close(delta.0, 40.0);
    assert_close(delta.1, 30.0);
}

#[test]
fn negative_pointer_drag_moves_only_the_target_node() {
    let harness = MinimalTestHarness::new();
    realize(&harness);

    let node_a = harness.nodes.row_data(0).unwrap();
    let start = harness.node_center(2).expect("rendered node center");
    harness.drag(start.0, start.1, start.0 - 50.0, start.1 - 30.0);

    let unchanged = harness.nodes.row_data(0).unwrap();
    let moved = harness.nodes.row_data(1).unwrap();
    assert_eq!((unchanged.x, unchanged.y), (node_a.x, node_a.y));
    assert_close(moved.x, 350.0);
    assert_close(moved.y, 170.0);
}

#[test]
fn rendered_geometry_populates_screen_space_helpers() {
    let harness = MinimalTestHarness::new();
    harness.window.set_zoom(2.0);
    harness.window.set_pan_x(11.0);
    harness.window.set_pan_y(13.0);
    realize(&harness);

    let center = harness.node_center(1).expect("rendered node center");
    assert_eq!(center, (361.0, 313.0));
    let output = harness.pin_position(3).expect("rendered output pin");
    assert_eq!(output, (499.0, 313.0));
}

#[test]
fn rejected_and_partially_snapped_commits_reconcile_geometry() {
    use common::harness::NodeEditorInternalCallbacks;
    for snap_x in [None, Some(110.0)] {
        let harness = MinimalTestHarness::new();
        harness
            .window
            .global::<NodeEditorInternalCallbacks>()
            .on_end_node_drag({
                let nodes = harness.nodes.clone();
                move |_, _, _| {
                    if let Some(x) = snap_x {
                        let mut node = nodes.row_data(0).unwrap();
                        node.x = x;
                        nodes.set_row_data(0, node);
                    }
                }
            });
        realize(&harness);
        let start = harness.node_center(1).unwrap();
        harness.drag(start.0, start.1, start.0 + 40.0, start.1 + 30.0);
        harness.pump_events();
        let cache = harness.ctrl.cache();
        let cache = cache.borrow();
        let rect = cache.node_rects[&1];
        assert_close(rect.x, snap_x.unwrap_or(100.0));
        assert_close(rect.y, 100.0);
        drop(cache);
        // A second gesture must not inherit an offset from the rejected one.
        let start = harness.node_center(1).unwrap();
        harness.drag(start.0, start.1, start.0 + 20.0, start.1 + 20.0);
        assert_close(harness.node_center(1).unwrap().0, start.0);
    }
}
