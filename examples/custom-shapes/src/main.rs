use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{NodeEditorSetup, NodeGeometry, wire_node_editor};
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
    let w = window.as_weak();

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
            line_width: 2.0,
        },
        LinkData {
            id: 2,
            start_pin_id: 7, // Node 3 output
            end_pin_id: 4,   // Node 2 input
            color: Color::from_argb_u8(255, 255, 180, 100),
            line_width: 3.0, // Thicker link
        },
    ]))));

    // Create setup with model update logic
    let setup = NodeEditorSetup::new({
        let nodes = nodes.clone();
        move |node_id, delta_x, delta_y| {
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

    // Wire all standard callbacks with one macro call
    wire_node_editor!(window, setup);
    // Wire all standard callbacks with one macro call
    wire_node_editor!(window, setup);

    // Custom link path computation via global callback
    window.global::<NodeEditorComputations>().on_compute_link_path({
        let ctrl = setup.controller().clone();
        let w = w.clone();
        move |start_pin, end_pin, _version, _zoom: f32, _pan_x: f32, _pan_y: f32| {
            let w = match w.upgrade() { Some(w) => w, None => return SharedString::default() };
            let style = w.get_link_style();
            let zoom = w.get_zoom();
            let bezier_offset = w.get_bezier_min_offset();

            let cache = ctrl.cache();
            let cache = cache.borrow();

            let start_pos = cache.pin_positions.get(&start_pin);
            let end_pos = cache.pin_positions.get(&end_pin);

            if let (Some(start), Some(end)) = (start_pos, end_pos) {
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

    // Viewport changes via global
    window.global::<NodeEditorComputations>().on_viewport_changed({
        let ctrl = setup.controller().clone();
        let w = w.clone();
        move |z, pan_x, pan_y| {
            if let Some(w) = w.upgrade() {
                ctrl.set_viewport(z, pan_x, pan_y);
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
            }
        }
    });

    // Grid initialization
    window.on_request_grid_update({
        let ctrl = setup.controller().clone();
        let w = w.clone();
        move || {
            if let Some(w) = w.upgrade() {
                w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
            }
        }
    });

    window.invoke_request_grid_update();
    window.run().unwrap();
}
