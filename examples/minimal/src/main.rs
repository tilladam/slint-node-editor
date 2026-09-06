use slint::{Color, ModelRc, SharedString, VecModel};
use slint_node_editor::{
    wire_node_editor, wire_selection, GraphLogic, LinkData, LinkPath, MovableNode, NodeEditorSetup,
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

struct App {
    window: MainWindow,
    #[cfg(test)]
    controller: Rc<slint_node_editor::NodeEditorController>,
}

fn build_app() -> App {
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
    #[cfg(test)]
    let controller = setup.controller().clone();

    // Wire all callbacks with one macro call
    wire_node_editor!(window, setup);
    wire_selection!(window, setup, nodes);

    App {
        window,
        #[cfg(test)]
        controller,
    }
}

fn main() {
    build_app().window.run().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::{
        platform::{PointerEventButton, WindowEvent},
        ComponentHandle, LogicalPosition,
    };
    use std::cell::Cell;

    fn initialize_backend() {
        thread_local! {
            static INITIALIZED: Cell<bool> = const { Cell::new(false) };
        }
        INITIALIZED.with(|initialized| {
            if !initialized.get() {
                i_slint_backend_testing::init_no_event_loop();
                initialized.set(true);
            }
        });
    }

    fn app() -> App {
        initialize_backend();
        build_app()
    }

    fn pump() {
        slint::platform::update_timers_and_animations();
        slint::platform::update_timers_and_animations();
    }

    fn realize(app: &App) {
        app.window.show().unwrap();
        pump();
    }

    fn click(app: &App, x: f32, y: f32) {
        let position = LogicalPosition::new(x, y);
        app.window.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        app.window.window().dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
        pump();
    }

    #[test]
    fn public_configuration_controls_the_controller_and_custom_nodes() {
        let app = app();
        assert_eq!(app.controller.grid_spacing(), 37.0);
        assert_eq!(app.controller.bezier_offset(), 120.0);
        assert!(app.window.get_grid_commands().contains("M 37 0"));

        realize(&app);
        let path_before = app
            .window
            .global::<NodeEditorComputations>()
            .invoke_compute_link_path(3, 4, app.window.get_geometry_version());
        let version = app.window.get_geometry_version();
        app.window.set_grid_spacing(43.0);
        app.window.set_bezier_min_offset(140.0);
        pump();
        assert_eq!(app.controller.grid_spacing(), 43.0);
        assert_eq!(app.controller.bezier_offset(), 140.0);
        assert!(app.window.get_geometry_version() > version);
        let path_after = app
            .window
            .global::<NodeEditorComputations>()
            .invoke_compute_link_path(3, 4, app.window.get_geometry_version());
        assert_ne!(path_before.commands, path_after.commands);

        app.window.set_zoom(0.6);
        pump();
        assert_eq!(app.controller.cache().borrow().node_rects[&1].height, 60.0);
        app.window.set_zoom(0.3);
        pump();
        assert_eq!(app.controller.cache().borrow().node_rects[&1].height, 40.0);

        app.window.set_min_node_width(320.0);
        app.window.set_min_node_height(140.0);
        pump();
        let cache = app.controller.cache();
        let cache = cache.borrow();
        assert_eq!(cache.node_rects[&1].width, 320.0);
        assert_eq!(cache.node_rects[&1].height, 140.0);
    }

    #[test]
    fn public_geometry_functions_reach_the_canonical_lifecycle() {
        let app = app();
        realize(&app);
        let version = app.window.get_geometry_version();

        app.window
            .invoke_report_node_geometry(99, 10.0, 20.0, 80.0, 40.0);
        app.window
            .invoke_report_pin_geometry(990, 99, 1, 5.0, 6.0);
        pump();

        let cache = app.controller.cache();
        let cache = cache.borrow();
        assert_eq!(cache.node_rects[&99].x, 10.0);
        assert_eq!(cache.pin_positions[&990].node_id, 99);
        drop(cache);
        assert_eq!(
            app.window
                .global::<NodeEditorComputations>()
                .invoke_compute_pin_at(15.0, 26.0, 1.0),
            990,
        );
        assert!(app.window.get_geometry_version() > version);
    }

    #[test]
    fn base_node_double_click_uses_the_public_event_global() {
        let app = app();
        let observed = Rc::new(Cell::new(0));
        app.window
            .global::<NodeEditorEvents>()
            .on_node_double_clicked({
                let observed = observed.clone();
                move |node_id| observed.set(node_id)
            });
        realize(&app);

        click(&app, 150.0, 120.0);
        click(&app, 150.0, 120.0);

        assert_eq!(observed.get(), 1);
    }

    #[test]
    fn public_grid_request_regenerates_commands() {
        let app = app();
        app.window.set_grid_commands("stale".into());
        app.window
            .global::<NodeEditorComputations>()
            .invoke_request_grid_update();
        assert_ne!(app.window.get_grid_commands().as_str(), "stale");
    }
}
