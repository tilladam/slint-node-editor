use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{wire_node_editor, NodeEditorSetup};
use std::rc::Rc;

slint::include_modules!();

fn main() {
    let window = MainWindow::new().unwrap();

    let nodes = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Source"),
            x: 80.0,
            y: 80.0,
            color: Color::from_argb_u8(255, 229, 57, 53), // red
        },
        NodeData {
            id: 2,
            title: SharedString::from("Filter"),
            x: 380.0,
            y: 60.0,
            color: Color::from_argb_u8(255, 156, 39, 176), // purple
        },
        NodeData {
            id: 3,
            title: SharedString::from("Transform"),
            x: 380.0,
            y: 220.0,
            color: Color::from_argb_u8(255, 0, 137, 123), // teal
        },
        NodeData {
            id: 4,
            title: SharedString::from("Output"),
            x: 680.0,
            y: 140.0,
            color: Color::from_argb_u8(255, 30, 136, 229), // blue
        },
        NodeData {
            id: 5,
            title: SharedString::from("Monitor"),
            x: 680.0,
            y: 320.0,
            color: Color::from_argb_u8(255, 255, 179, 0), // amber
        },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    window.set_links(ModelRc::from(Rc::new(VecModel::from(vec![
        LinkData {
            id: 1,
            start_pin_id: 3,                                // Node 1 output
            end_pin_id: 4,                                  // Node 2 input
            color: Color::from_argb_u8(255, 244, 143, 177), // pink
            line_width: 2.5,
            status: -1,
        },
        LinkData {
            id: 2,
            start_pin_id: 3,                                // Node 1 output
            end_pin_id: 6,                                  // Node 3 input
            color: Color::from_argb_u8(255, 128, 203, 196), // teal light
            line_width: 2.5,
            status: -1,
        },
        LinkData {
            id: 3,
            start_pin_id: 5,                                // Node 2 output
            end_pin_id: 8,                                  // Node 4 input
            color: Color::from_argb_u8(255, 206, 147, 216), // purple light
            line_width: 2.5,
            status: -1,
        },
        LinkData {
            id: 4,
            start_pin_id: 7,                                // Node 3 output
            end_pin_id: 8,                                  // Node 4 input
            color: Color::from_argb_u8(255, 100, 221, 221), // cyan
            line_width: 2.5,
            status: -1,
        },
        LinkData {
            id: 5,
            start_pin_id: 7,                               // Node 3 output
            end_pin_id: 10,                                // Node 5 input
            color: Color::from_argb_u8(255, 255, 213, 79), // amber light
            line_width: 2.5,
            status: -1,
        },
    ]))));

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

    wire_node_editor!(window, setup);

    window.run().unwrap();
}
