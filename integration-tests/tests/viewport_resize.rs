//! Size notifications through the public module, independently of grid updates.

mod common;

use common::harness::{MinimalTestHarness, NodeEditorComputations};
use slint::{ComponentHandle, LogicalSize};
use std::{cell::RefCell, rc::Rc};

#[test]
fn resize_reports_editor_dimensions_without_a_grid_handler() {
    let harness = MinimalTestHarness::new();
    harness.window.show().unwrap();
    harness.window.set_zoom(2.0);
    harness.window.set_pan_x(30.0);
    harness.window.set_pan_y(40.0);
    harness.pump_events();
    harness.pump_events();
    harness.tracker.update_viewport.borrow_mut().clear();

    let sizes = Rc::new(RefCell::new(Vec::new()));
    let computations = harness.window.global::<NodeEditorComputations>();
    // A host without a grid must still learn about viewport size changes.
    computations.on_request_grid_update(|| {});
    computations.on_viewport_resized({
        let sizes = sizes.clone();
        move |width, height| sizes.borrow_mut().push((width, height))
    });

    harness.window.set_editor_width(480.0);
    harness.pump_events();
    assert_eq!(sizes.borrow().last(), Some(&(480.0, 600.0)));
    sizes.borrow_mut().clear();

    harness.window.set_editor_height(320.0);
    harness.pump_events();
    assert_eq!(sizes.borrow().last(), Some(&(480.0, 320.0)));
    // Dimensions belong to the editor, not its enclosing 800 x 600 window,
    // and are logical pixels rather than zoom-adjusted world coordinates.
    assert_eq!(harness.window.get_width_(), 800.0);
    assert_eq!(harness.window.get_height_(), 600.0);
    assert!(harness.tracker.update_viewport.borrow().is_empty());
}

#[test]
fn window_resize_notifies_the_host_of_its_new_visible_center() {
    let harness = MinimalTestHarness::new();
    harness.window.show().unwrap();
    harness.window.set_zoom(2.0);
    harness.window.set_pan_x(30.0);
    harness.window.set_pan_y(40.0);
    harness.pump_events();
    harness.pump_events();

    let center = Rc::new(RefCell::new(None));
    harness
        .window
        .global::<NodeEditorComputations>()
        .on_viewport_resized({
            let center = center.clone();
            let window = harness.window.as_weak();
            move |width, height| {
                let window = window.upgrade().unwrap();
                *center.borrow_mut() = Some((
                    (width / 2.0 - window.get_pan_x()) / window.get_zoom(),
                    (height / 2.0 - window.get_pan_y()) / window.get_zoom(),
                ));
            }
        });

    harness
        .window
        .window()
        .set_size(LogicalSize::new(1000.0, 700.0));
    harness.pump_events();
    assert_eq!(harness.window.get_width_(), 1000.0);
    assert_eq!(harness.window.get_height_(), 700.0);
    assert_eq!(*center.borrow(), Some((235.0, 155.0)));
}
