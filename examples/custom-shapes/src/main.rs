use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{NodeEditorController, NodeGeometry};
use std::cell::Cell;
use std::rc::Rc;

slint::include_modules!();

/// Generate an orthogonal (Manhattan) path: Horizontal -> Vertical -> Horizontal
fn generate_manhattan_path(
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    zoom: f32,
) -> String {
    // Scale coordinates by zoom
    let sx = start_x * zoom;
    let sy = start_y * zoom;
    let ex = end_x * zoom;
    let ey = end_y * zoom;

    // Calculate midpoint for the vertical segment
    let mid_x = (sx + ex) / 2.0;

    // Construct SVG path command
    // M sx sy -> Move to start
    // L mid_x sy -> Line to first corner
    // L mid_x ey -> Line to second corner
    // L ex ey -> Line to end
    format!("M {} {} L {} {} L {} {} L {} {}", sx, sy, mid_x, sy, mid_x, ey, ex, ey)
}

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();
    let w = window.as_weak();

    // Track geometry reports to know when to refresh links
    // We have 3 nodes with 3 pins each = 3 node rects + 9 pin positions = 12 reports
    let geometry_reports = Rc::new(Cell::new(0i32));
    let expected_reports = 12;

    // Set up nodes
    let nodes = Rc::new(VecModel::from(vec![
        NodeData { id: 1, title: SharedString::from("Node A"), x: 100.0, y: 100.0 },
        NodeData { id: 2, title: SharedString::from("Node B"), x: 450.0, y: 250.0 },
        NodeData { id: 3, title: SharedString::from("Node C"), x: 100.0, y: 400.0 },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Set up links
    window.set_links(ModelRc::from(Rc::new(VecModel::from(vec![
        LinkData {
            id: 1,
            start_pin_id: 3, // Node 1 output
            end_pin_id: 4,   // Node 2 input
            color: Color::from_argb_u8(255, 100, 180, 255),
        },
        LinkData {
            id: 2,
            start_pin_id: 7, // Node 3 output
            end_pin_id: 4,   // Node 2 input
            color: Color::from_argb_u8(255, 255, 180, 100),
        },
    ]))));

    // Custom link path computation callback
    window.on_compute_link_path({
        let ctrl = ctrl.clone();
        let w = w.clone();
        move |start_pin, end_pin, _version| {
            let w = match w.upgrade() { Some(w) => w, None => return SharedString::default() };
            let style = w.get_link_style();
            let zoom = w.get_zoom();
            let bezier_offset = w.get_bezier_min_offset();

            // Use the controller's cache to get pin positions
            // Note: We need to use the lower-level cache API because we're doing custom logic
            // that isn't wrapped by the simple `compute_link_path` helper in the controller
            // when we want orthogonal routing.
            let cache = ctrl.cache();
            let cache = cache.borrow();

            let start_pos = cache.pin_positions.get(&start_pin);
            let end_pos = cache.pin_positions.get(&end_pin);

            if let (Some(start), Some(end)) = (start_pos, end_pos) {
                // Resolve absolute positions
                if let (Some(start_rect), Some(end_rect)) = (
                    cache.node_rects.get(&start.node_id).map(|n| n.rect()),
                    cache.node_rects.get(&end.node_id).map(|n| n.rect()),
                ) {
                    let sx = start_rect.0 + start.rel_x;
                    let sy = start_rect.1 + start.rel_y;
                    let ex = end_rect.0 + end.rel_x;
                    let ey = end_rect.1 + end.rel_y;

                    if style == "orthogonal" {
                        generate_manhattan_path(sx, sy, ex, ey, zoom).into()
                    } else {
                        // Fallback to standard Bezier (using library helper)
                        slint_node_editor::generate_bezier_path(sx, sy, ex, ey, zoom, bezier_offset).into()
                    }
                } else {
                    SharedString::default()
                }
            } else {
                SharedString::default()
            }
        }
    });

    // Standard callbacks via controller
    window.on_node_drag_started(ctrl.node_drag_started_callback());

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

    // Node drag
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
