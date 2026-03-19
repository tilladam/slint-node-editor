use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::NodeEditorSetup;
use std::rc::Rc;

slint::include_modules!();

fn main() {
    let window = MainWindow::new().unwrap();
    let w = window.as_weak();

    // Set up nodes
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
            line_width: 2.0,
        },
    ]))));
    
    // Create setup with model update logic - this is the ONLY callback you provide
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

    // Wire geometry callbacks
    let gc = window.global::<GeometryCallbacks>();
    gc.on_report_node_rect(setup.report_node_rect());
    gc.on_report_pin_position(setup.report_pin_position());
    gc.on_start_node_drag(setup.start_node_drag());
    gc.on_end_node_drag(setup.end_node_drag());
    
    // Wire computational callbacks
    window.global::<NodeEditorComputations>().on_compute_link_path(setup.compute_link_path());
    
    window.global::<NodeEditorComputations>().on_viewport_changed({
        let ctrl = setup.controller().clone();
        let w = w.clone();
        move |zoom, pan_x, pan_y| {
            if let Some(w) = w.upgrade() {
                ctrl.set_viewport(zoom, pan_x, pan_y);
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
            }
        }
    });
    
    // Initial grid generation
    window.set_grid_commands(setup.generate_initial_grid(window.get_width_(), window.get_height_()));

    window.run().unwrap();
}
