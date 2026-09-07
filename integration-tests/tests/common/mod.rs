//! Common test utilities for integration tests.

#![allow(dead_code)]

pub mod harness;

use std::cell::RefCell;
use std::rc::Rc;

/// Tracks callback invocations for testing.
///
/// Each field records calls to the corresponding callback with their arguments.
#[derive(Default, Clone)]
pub struct CallbackTracker {
    /// (node_id,)
    pub node_drag_started: Rc<RefCell<Vec<i32>>>,
    /// (delta_x, delta_y)
    pub node_drag_ended: Rc<RefCell<Vec<(f32, f32)>>>,
    /// (start_pin, end_pin)
    pub link_requested: Rc<RefCell<Vec<(i32, i32)>>>,
    /// Count of link_cancelled calls
    pub link_cancelled: Rc<RefCell<usize>>,
    /// (node_id, x, y, width, height)
    #[allow(clippy::type_complexity)]
    pub node_rect_changed: Rc<RefCell<Vec<(i32, f32, f32, f32, f32)>>>,
    /// (pin_id, node_id, pin_type, x, y)
    #[allow(clippy::type_complexity)]
    pub pin_position_changed: Rc<RefCell<Vec<(i32, i32, i32, f32, f32)>>>,
    /// (zoom, pan_x, pan_y)
    pub update_viewport: Rc<RefCell<Vec<(f32, f32, f32)>>>,
    /// Count of context_menu_requested calls
    pub context_menu_requested: Rc<RefCell<usize>>,
}

impl CallbackTracker {
    pub fn new() -> Self {
        Self::default()
    }
}
