use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::NodeEditorController;
use std::rc::Rc;
use std::cell::RefCell;

slint::include_modules!();

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();
    let w = window.as_weak();
    
    // Track dragged node ID locally
    let dragged_node_id = Rc::new(RefCell::new(0i32));

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
            line_width: 2.0,
        },
    ]))));

    // Wire GeometryCallbacks to controller
    window.global::<GeometryCallbacks>().on_report_node_rect({
        let ctrl = ctrl.clone();
        move |id, x, y, width, h| {
            ctrl.handle_node_rect(id, x, y, width, h);
        }
    });

    window.global::<GeometryCallbacks>().on_report_pin_position({
        let ctrl = ctrl.clone();
        move |pid, nid, ptype, x, y| {
            ctrl.handle_pin_position(pid, nid, ptype, x, y);
        }
    });
    
    window.global::<GeometryCallbacks>().on_start_node_drag({
        let dragged = dragged_node_id.clone();
        move |node_id, _already_selected, _world_x, _world_y| {
            *dragged.borrow_mut() = node_id;
        }
    });

    window.global::<GeometryCallbacks>().on_update_node_drag({
        let w = w.clone();
        move |offset_x, offset_y| {
            if let Some(w) = w.upgrade() {
                let drag_state = w.global::<DragState>();
                drag_state.set_drag_offset_x(offset_x);
                drag_state.set_drag_offset_y(offset_y);
            }
        }
    });

    window.global::<GeometryCallbacks>().on_end_node_drag({
        let dragged = dragged_node_id.clone();
        move |delta_x, delta_y| {
            let node_id = *dragged.borrow();
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

    // Wire global computational callbacks
    let computations = window.global::<NodeEditorComputations>();
    
    // Link path computation
    computations.on_compute_link_path(ctrl.compute_link_path_callback());

    // Viewport updates
    computations.on_viewport_changed({
        let ctrl = ctrl.clone();
        let w = w.clone();
        move |z, pan_x, pan_y| {
            if let Some(w) = w.upgrade() {
                ctrl.set_viewport(z, pan_x, pan_y);
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
            }
        }
    });
    
    // Initial grid generation
    window.set_grid_commands(ctrl.generate_initial_grid(window.get_width_(), window.get_height_()));

    window.run().unwrap();
}
