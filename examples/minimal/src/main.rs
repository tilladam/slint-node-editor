use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::NodeEditorController;
use std::cell::Cell;
use std::rc::Rc;

slint::include_modules!();

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();
    let w = window.as_weak();

    // Track geometry reports to know when to refresh links
    // We have 2 nodes with 2 pins each = 2 node rects + 4 pin positions = 6 reports
    let geometry_reports = Rc::new(Cell::new(0i32));
    let expected_reports = 6; // 2 nodes + 4 pins

    // Set up nodes (keep reference for drag updates)
    let nodes = Rc::new(VecModel::from(vec![
        NodeData { id: 1, title: SharedString::from("Node A"), x: 100.0, y: 100.0 },
        NodeData { id: 2, title: SharedString::from("Node B"), x: 400.0, y: 200.0 },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Set up links
    window.set_links(ModelRc::from(Rc::new(VecModel::from(vec![
        LinkData {
            id: 1,
            start_pin_id: 3, // Node 1 output (node_id * 2 + 1)
            end_pin_id: 4,   // Node 2 input (node_id * 2)
            color: Color::from_argb_u8(255, 100, 180, 255),
        },
    ]))));

    // Core callbacks - controller handles the logic
    window.on_compute_link_path(ctrl.compute_link_path_callback());
    window.on_node_drag_started(ctrl.node_drag_started_callback());

    // Geometry tracking - update cache and refresh links when all geometry is reported
    window.on_node_rect_changed({
        let ctrl = ctrl.clone();
        let reports = geometry_reports.clone();
        let w = w.clone();
        move |id, x, y, width, h| {
            ctrl.handle_node_rect(id, x, y, width, h);
            let count = reports.get() + 1;
            reports.set(count);
            if count == expected_reports {
                if let Some(win) = w.upgrade() {
                    win.invoke_refresh_links();
                    win.window().request_redraw();
                }
            }
        }
    });

    window.on_pin_position_changed({
        let ctrl = ctrl.clone();
        let reports = geometry_reports.clone();
        let w = w.clone();
        move |pid, nid, ptype, x, y| {
            ctrl.handle_pin_position(pid, nid, ptype, x, y);
            let count = reports.get() + 1;
            reports.set(count);
            if count == expected_reports {
                if let Some(win) = w.upgrade() {
                    win.invoke_refresh_links();
                    win.window().request_redraw();
                }
            }
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
        let w = w.clone();
        move |z, pan_x, pan_y| {
            if let Some(w) = w.upgrade() {
                ctrl.set_zoom(z);
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
            }
        }
    });

    // Node drag - update positions in model
    window.on_node_drag_ended({
        let ctrl = ctrl.clone();
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
        }
    });

    window.invoke_request_grid_update();
    window.run().unwrap();
}
