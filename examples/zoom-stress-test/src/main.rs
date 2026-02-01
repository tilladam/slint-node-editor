// Zoom Stress Test Example
//
// Tests widget scaling behavior at various zoom levels with three complex nodes.

use slint::{Color, ModelRc, SharedString, VecModel};
use slint_node_editor::NodeEditorController;
use std::rc::Rc;

slint::include_modules!();

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();
    let w = window.as_weak();

    // Create input nodes model
    let input_nodes: Rc<VecModel<InputNodeData>> = Rc::new(VecModel::from(vec![InputNodeData {
        id: 1,
        title: SharedString::from("Input"),
        world_x: 400.0,
        world_y: 100.0,
        text_value: SharedString::from("Hello World"),
        spin_value: 50,
        combo_index: 0,
    }]));

    // Create control nodes model
    let control_nodes: Rc<VecModel<ControlNodeData>> = Rc::new(VecModel::from(vec![
        ControlNodeData {
            id: 2,
            title: SharedString::from("Control"),
            world_x: 200.0,
            world_y: 350.0,
            check_a: true,
            check_b: false,
            switch_value: true,
            slider_value: 0.65,
        },
    ]));

    // Create display nodes model
    let display_nodes: Rc<VecModel<DisplayNodeData>> = Rc::new(VecModel::from(vec![
        DisplayNodeData {
            id: 3,
            title: SharedString::from("Display"),
            world_x: 600.0,
            world_y: 350.0,
            progress: 0.75,
            status_text: SharedString::from("Processing"),
            color_r: 0.2,
            color_g: 0.6,
            color_b: 0.9,
            is_loading: true,
        },
    ]));

    // Create links model (connecting the nodes)
    // Pin IDs: node_id * 10 for input, node_id * 10 + 1 for output
    let links: Rc<VecModel<LinkData>> = Rc::new(VecModel::from(vec![
        // Link from Input output (pin 11) to Control input (pin 20)
        LinkData {
            id: 1,
            start_pin_id: 11, // Input node output
            end_pin_id: 20,   // Control node input
            color: Color::from_argb_u8(255, 100, 200, 100),
            line_width: 2.0,
        },
        // Link from Input output (pin 11) to Display input (pin 30)
        LinkData {
            id: 2,
            start_pin_id: 11, // Input node output
            end_pin_id: 30,   // Display node input
            color: Color::from_argb_u8(255, 100, 150, 255),
            line_width: 2.0,
        },
    ]));

    // Set models
    window.set_input_nodes(ModelRc::from(input_nodes.clone()));
    window.set_control_nodes(ModelRc::from(control_nodes.clone()));
    window.set_display_nodes(ModelRc::from(display_nodes.clone()));
    window.set_links(ModelRc::from(links.clone()));

    // Core callbacks - controller handles link path computation
    window.on_compute_link_path(ctrl.compute_link_path_callback());

    // Geometry tracking
    window.on_node_rect_changed({
        let ctrl = ctrl.clone();
        move |id, x, y, width, h| {
            ctrl.handle_node_rect(id, x, y, width, h);
        }
    });

    window.on_pin_position_changed({
        let ctrl = ctrl.clone();
        move |pid, nid, ptype, x, y| {
            ctrl.handle_pin_position(pid, nid, ptype, x, y);
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

    // Input node callbacks
    window.on_input_text_changed(|id, val| {
        println!("Input node {}: text changed to '{}'", id, val);
    });

    window.on_input_spin_changed(|id, val| {
        println!("Input node {}: spin value changed to {}", id, val);
    });

    window.on_input_combo_changed(|id, idx| {
        println!("Input node {}: combo index changed to {}", id, idx);
    });

    // Control node callbacks
    window.on_control_check_a_toggled(|id, val| {
        println!("Control node {}: check A toggled to {}", id, val);
    });

    window.on_control_check_b_toggled(|id, val| {
        println!("Control node {}: check B toggled to {}", id, val);
    });

    window.on_control_switch_toggled(|id, val| {
        println!("Control node {}: switch toggled to {}", id, val);
    });

    window.on_control_slider_changed(|id, val| {
        println!("Control node {}: slider changed to {:.2}", id, val);
    });

    window.on_control_action_clicked(|id, action| {
        println!("Control node {}: action '{}' clicked", id, action);
    });

    // Initial grid generation
    window.invoke_request_grid_update();

    window.run().unwrap();
}
