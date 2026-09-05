use slint::{Color, ModelRc, SharedString, VecModel};
use slint_node_editor::{
    wire_node_editor, wire_selection, GraphLogic, LinkData, MovableNode, NodeEditorSetup,
};
use std::rc::Rc;

slint::include_modules!();

// The row is the node: its position and whether it's selected both live here,
// which is what lets a drag commit read the same `selected` the editor renders.
impl MovableNode for NodeData {
    fn id(&self) -> i32 {
        self.id
    }
    fn x(&self) -> f32 {
        self.x
    }
    fn y(&self) -> f32 {
        self.y
    }
    fn selected(&self) -> bool {
        self.selected
    }
    fn set_x(&mut self, x: f32) {
        self.x = x;
    }
    fn set_y(&mut self, y: f32) {
        self.y = y;
    }
}

fn main() {
    let window = MainWindow::new().unwrap();

    // Set up nodes
    let nodes = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Node A"),
            x: 100.0,
            y: 100.0,
            selected: false,
        },
        NodeData {
            id: 2,
            title: SharedString::from("Node B"),
            x: 400.0,
            y: 200.0,
            selected: false,
        },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Set up links
    window.set_links(ModelRc::from(Rc::new(VecModel::from(vec![LinkData {
        id: 1,
        start_pin_id: 3, // Node 1 output (node_id * 2 + 1)
        end_pin_id: 4,   // Node 2 input (node_id * 2)
        color: Color::from_argb_u8(255, 100, 180, 255),
        line_width: 2.0,
        status: -1,
        selected: false,
    }]))));

    // Commit a finished drag: the dragged node, plus anything else the rows
    // show as selected. There is no selection state to keep anywhere else.
    let setup = NodeEditorSetup::new({
        let nodes = nodes.clone();
        move |dragged, delta_x, delta_y| {
            GraphLogic::commit_drag(&nodes, dragged, delta_x, delta_y);
        }
    });

    // Wire all callbacks with one macro call
    wire_node_editor!(window, setup);
    wire_selection!(window, setup, nodes);

    window.run().unwrap();
}
