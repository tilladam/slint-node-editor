//! Test harness for the minimal example.
//!
//! Provides a complete setup mirroring `examples/minimal/src/main.rs` with
//! callback tracking and helper methods for simulating user interactions.

#![allow(dead_code)]

use super::CallbackTracker;
use slint::{
    platform::{Key, PointerEventButton, WindowEvent},
    Color, ComponentHandle, LogicalPosition, Model, ModelRc, SharedString, VecModel,
};
use slint_node_editor::{NodeEditorController, SelectionManager};
use std::cell::RefCell;
use std::rc::Rc;

// Include the compiled UI from build.rs
slint::include_modules!();

/// Initialize the testing backend for this thread.
/// With init_no_event_loop(), each test thread can have its own backend instance.
/// Uses thread_local to ensure each thread only initializes once.
fn init_testing_backend() {
    use std::cell::Cell;
    thread_local! {
        static INITIALIZED: Cell<bool> = const { Cell::new(false) };
    }

    INITIALIZED.with(|init| {
        if !init.get() {
            i_slint_backend_testing::init_no_event_loop();
            init.set(true);
        }
    });
}

/// Test harness for the minimal node editor example.
///
/// Sets up nodes, links, and all callbacks with tracking.
pub struct MinimalTestHarness {
    pub window: MainWindow,
    pub ctrl: NodeEditorController,
    pub nodes: Rc<VecModel<NodeData>>,
    pub links: Rc<VecModel<LinkData>>,
    pub tracker: CallbackTracker,
    pub selection: Rc<RefCell<SelectionManager>>,
}

impl MinimalTestHarness {
    /// Create a new test harness with default nodes and links.
    pub fn new() -> Self {
        Self::with_nodes_and_links(
            vec![
                NodeData {
                    id: 1,
                    title: SharedString::from("Node A"),
                    x: 100.0,
                    y: 100.0,
                },
                NodeData {
                    id: 2,
                    title: SharedString::from("Node B"),
                    x: 400.0,
                    y: 200.0,
                },
            ],
            vec![LinkData {
                id: 1,
                start_pin_id: 3, // Node 1 output (node_id * 2 + 1)
                end_pin_id: 4,   // Node 2 input (node_id * 2)
                color: Color::from_argb_u8(255, 100, 180, 255),
                line_width: 2.0,
            }],
        )
    }

    /// Create a new test harness with custom nodes and links.
    pub fn with_nodes_and_links(nodes: Vec<NodeData>, links: Vec<LinkData>) -> Self {
        init_testing_backend();
        let window = MainWindow::new().unwrap();
        let ctrl = NodeEditorController::new();
        let tracker = CallbackTracker::new();
        let selection = Rc::new(RefCell::new(SelectionManager::new()));
        let w = window.as_weak();

        // Set up nodes
        let nodes = Rc::new(VecModel::from(nodes));
        window.set_nodes(ModelRc::from(nodes.clone()));

        // Set up links
        let links = Rc::new(VecModel::from(links));
        window.set_links(ModelRc::from(links.clone()));

        // Core callbacks - controller handles the logic
        window.on_compute_link_path(ctrl.compute_link_path_callback());
        window.on_node_drag_started({
            let ctrl = ctrl.clone();
            let tracker = tracker.clone();
            move |node_id| {
                ctrl.handle_node_drag_started(node_id);
                tracker.node_drag_started.borrow_mut().push(node_id);
            }
        });

        // Geometry tracking - update cache
        window.on_node_rect_changed({
            let ctrl = ctrl.clone();
            let tracker = tracker.clone();
            move |id, x, y, width, h| {
                ctrl.handle_node_rect(id, x, y, width, h);
                tracker.node_rect_changed.borrow_mut().push((id, x, y, width, h));
            }
        });

        window.on_pin_position_changed({
            let ctrl = ctrl.clone();
            let tracker = tracker.clone();
            move |pid, nid, ptype, x, y| {
                ctrl.handle_pin_position(pid, nid, ptype, x, y);
                tracker.pin_position_changed.borrow_mut().push((pid, nid, ptype, x, y));
            }
        });

        // Grid updates
        window.on_request_grid_update({
            let ctrl = ctrl.clone();
            let w = w.clone();
            move || {
                if let Some(w) = w.upgrade() {
                    w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
                }
            }
        });

        window.on_update_viewport({
            let ctrl = ctrl.clone();
            let tracker = tracker.clone();
            let w = w.clone();
            move |z, pan_x, pan_y| {
                if let Some(w) = w.upgrade() {
                    ctrl.set_viewport(z, pan_x, pan_y);
                    w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
                    tracker.update_viewport.borrow_mut().push((z, pan_x, pan_y));
                }
            }
        });

        // Node drag - update positions in model
        window.on_node_drag_ended({
            let ctrl = ctrl.clone();
            let nodes = nodes.clone();
            let tracker = tracker.clone();
            move |delta_x, delta_y| {
                let node_id = ctrl.dragged_node_id();
                for i in 0..nodes.row_count() {
                    if let Some(mut node) = nodes.row_data(i) {
                        if node.id == node_id {
                            node.x += delta_x;
                            node.y += delta_y;
                            nodes.set_row_data(i, node);
                            break;
                        }
                    }
                }
                tracker.node_drag_ended.borrow_mut().push((delta_x, delta_y));
            }
        });

        // Link requested callback
        window.on_link_requested({
            let tracker = tracker.clone();
            move |start_pin, end_pin| {
                tracker.link_requested.borrow_mut().push((start_pin, end_pin));
            }
        });

        // Link cancelled callback
        window.on_link_cancelled({
            let tracker = tracker.clone();
            move || {
                *tracker.link_cancelled.borrow_mut() += 1;
            }
        });

        // Selection changed callback
        window.on_selection_changed({
            let tracker = tracker.clone();
            move || {
                *tracker.selection_changed.borrow_mut() += 1;
            }
        });

        // Delete selected callback
        window.on_delete_selected({
            let tracker = tracker.clone();
            move || {
                *tracker.delete_selected.borrow_mut() += 1;
            }
        });

        // Context menu requested callback
        window.on_context_menu_requested({
            let tracker = tracker.clone();
            move || {
                *tracker.context_menu_requested.borrow_mut() += 1;
            }
        });

        // Add node requested callback
        window.on_add_node_requested({
            let tracker = tracker.clone();
            move || {
                *tracker.add_node_requested.borrow_mut() += 1;
            }
        });

        // Selection callbacks
        window.on_select_node({
            let selection = selection.clone();
            let w = w.clone();
            move |node_id, shift_held| {
                let mut sel = selection.borrow_mut();
                sel.handle_interaction(node_id, shift_held);
                if let Some(w) = w.upgrade() {
                    let ids: Vec<i32> = sel.iter().cloned().collect();
                    w.set_selected_node_ids(ModelRc::from(Rc::new(VecModel::from(ids))));
                    w.set_selection_version(w.get_selection_version() + 1);
                }
            }
        });

        window.on_clear_selection({
            let selection = selection.clone();
            let w = w.clone();
            move || {
                selection.borrow_mut().clear();
                if let Some(w) = w.upgrade() {
                    w.set_selected_node_ids(ModelRc::default());
                    w.set_selection_version(w.get_selection_version() + 1);
                }
            }
        });

        window.on_is_selected({
            let selection = selection.clone();
            move |node_id, _version| selection.borrow().contains(node_id)
        });

        window.on_sync_selection_to_nodes({
            let selection = selection.clone();
            move |ids| {
                let mut sel = selection.borrow_mut();
                sel.clear();
                for i in 0..ids.row_count() {
                    if let Some(id) = ids.row_data(i) {
                        sel.handle_interaction(id, true); // Add each ID
                    }
                }
            }
        });

        // Compute callbacks using controller's cache
        window.on_compute_pin_at({
            let ctrl = ctrl.clone();
            move |x, y| ctrl.cache().borrow().find_pin_at(x, y, 10.0)
        });

        window.on_compute_link_at({
            let ctrl = ctrl.clone();
            let links = links.clone();
            move |x, y| {
                let links_iter = (0..links.row_count()).filter_map(|i| {
                    let link = links.row_data(i)?;
                    Some((link.id, link.start_pin_id, link.end_pin_id))
                });
                ctrl.cache().borrow().find_link_at(x, y, links_iter, 8.0, ctrl.zoom(), 50.0, 20)
            }
        });

        window.on_compute_box_selection({
            let ctrl = ctrl.clone();
            move |x, y, w, h| {
                let ids = ctrl.cache().borrow().nodes_in_selection_box(x, y, w, h);
                ModelRc::from(Rc::new(VecModel::from(ids)))
            }
        });

        window.on_compute_link_preview_path(|start_x, start_y, end_x, end_y| {
            slint_node_editor::generate_bezier_path(start_x, start_y, end_x, end_y, 1.0, 50.0).into()
        });

        // Initialize grid
        window.invoke_request_grid_update();

        Self {
            window,
            ctrl,
            nodes,
            links,
            tracker,
            selection,
        }
    }

    /// Process all pending events and render a frame.
    pub fn pump_events(&self) {
        slint::platform::update_timers_and_animations();
    }

    /// Get the center position of a node by ID (in screen coordinates).
    /// Returns None if node not found or geometry not yet reported.
    pub fn node_center(&self, node_id: i32) -> Option<(f32, f32)> {
        let cache = self.ctrl.cache();
        let cache = cache.borrow();
        let rect = cache.node_rects.get(&node_id)?;
        Some((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))
    }

    /// Get the absolute position of a pin by ID.
    /// Returns None if pin not found or geometry not yet reported.
    pub fn pin_position(&self, pin_id: i32) -> Option<(f32, f32)> {
        let cache = self.ctrl.cache();
        let cache = cache.borrow();
        let pin = cache.pin_positions.get(&pin_id)?;
        let rect = cache.node_rects.get(&pin.node_id)?;
        Some((rect.x + pin.rel_x, rect.y + pin.rel_y))
    }

    /// Get node data by ID.
    pub fn node_data(&self, node_id: i32) -> Option<NodeData> {
        for i in 0..self.nodes.row_count() {
            if let Some(node) = self.nodes.row_data(i) {
                if node.id == node_id {
                    return Some(node);
                }
            }
        }
        None
    }

    // === Mouse event helpers ===

    /// Simulate mouse down at the given position.
    pub fn mouse_down(&self, x: f32, y: f32) {
        self.window
            .window()
            .dispatch_event(WindowEvent::PointerPressed {
                position: LogicalPosition::new(x, y),
                button: PointerEventButton::Left,
            });
        self.pump_events();
    }

    /// Simulate mouse down with a specific button.
    pub fn mouse_down_button(&self, x: f32, y: f32, button: PointerEventButton) {
        self.window
            .window()
            .dispatch_event(WindowEvent::PointerPressed {
                position: LogicalPosition::new(x, y),
                button,
            });
        self.pump_events();
    }

    /// Simulate mouse move to the given position.
    pub fn mouse_move(&self, x: f32, y: f32) {
        self.window
            .window()
            .dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(x, y),
            });
        self.pump_events();
    }

    /// Simulate mouse up at the given position.
    pub fn mouse_up(&self, x: f32, y: f32) {
        self.window
            .window()
            .dispatch_event(WindowEvent::PointerReleased {
                position: LogicalPosition::new(x, y),
                button: PointerEventButton::Left,
            });
        self.pump_events();
    }

    /// Simulate mouse up with a specific button.
    pub fn mouse_up_button(&self, x: f32, y: f32, button: PointerEventButton) {
        self.window
            .window()
            .dispatch_event(WindowEvent::PointerReleased {
                position: LogicalPosition::new(x, y),
                button,
            });
        self.pump_events();
    }

    /// Simulate a complete click (down + up) at the given position.
    pub fn click(&self, x: f32, y: f32) {
        self.mouse_down(x, y);
        self.mouse_up(x, y);
    }

    /// Simulate a complete drag from start to end.
    pub fn drag(&self, start_x: f32, start_y: f32, end_x: f32, end_y: f32) {
        self.mouse_down(start_x, start_y);
        self.mouse_move(end_x, end_y);
        self.mouse_up(end_x, end_y);
    }

    /// Simulate scroll (for zoom).
    pub fn scroll(&self, x: f32, y: f32, delta_y: f32) {
        self.window
            .window()
            .dispatch_event(WindowEvent::PointerScrolled {
                position: LogicalPosition::new(x, y),
                delta_x: 0.0,
                delta_y,
            });
        self.pump_events();
    }

    // === Keyboard event helpers ===

    /// Simulate a key press.
    pub fn key_press(&self, key: Key) {
        self.window.window().dispatch_event(WindowEvent::KeyPressed {
            text: key.into(),
        });
        self.pump_events();
    }

    /// Simulate a key release.
    pub fn key_release(&self, key: Key) {
        self.window.window().dispatch_event(WindowEvent::KeyReleased {
            text: key.into(),
        });
        self.pump_events();
    }

    /// Simulate a complete key press and release.
    pub fn key_tap(&self, key: Key) {
        self.key_press(key);
        self.key_release(key);
    }

    /// Simulate text input.
    pub fn text_input(&self, text: &str) {
        self.window.window().dispatch_event(WindowEvent::KeyPressed {
            text: text.into(),
        });
        self.pump_events();
    }
}

impl Default for MinimalTestHarness {
    fn default() -> Self {
        Self::new()
    }
}
