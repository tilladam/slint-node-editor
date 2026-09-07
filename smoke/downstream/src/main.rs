use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{
    wire_node_editor, wire_selection, GraphLogic, LinkData, LinkPath, MovableNode, NodeEditorSetup,
};
use std::{cell::Cell, collections::HashSet, rc::Rc};

slint::include_modules!();

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

struct QuickStart {
    window: App,
    #[cfg(test)]
    nodes: Rc<VecModel<NodeData>>,
    #[cfg(test)]
    links: Rc<VecModel<LinkData>>,
    #[cfg(test)]
    controller: Rc<slint_node_editor::NodeEditorController>,
}

fn remove_where<T: Clone + 'static>(model: &VecModel<T>, remove: impl Fn(&T) -> bool) {
    for index in (0..model.row_count()).rev() {
        if model.row_data(index).is_some_and(|row| remove(&row)) {
            model.remove(index);
        }
    }
}

fn build_app() -> QuickStart {
    let window = App::new().unwrap();
    let nodes = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Source"),
            x: 100.0,
            y: 100.0,
            selected: false,
        },
        NodeData {
            id: 2,
            title: SharedString::from("Sink"),
            x: 420.0,
            y: 220.0,
            selected: false,
        },
    ]));
    let links = Rc::new(VecModel::<LinkData>::default());
    window.set_nodes(ModelRc::from(nodes.clone()));
    window.set_links(ModelRc::from(links.clone()));

    let setup = NodeEditorSetup::new({
        let nodes = nodes.clone();
        move |dragged, delta_x, delta_y| {
            GraphLogic::commit_drag(&nodes, dragged, delta_x, delta_y);
        }
    });
    let controller = setup.controller().clone();

    // These macros install the geometry/computation callbacks and synchronously
    // project selection gestures into the row flags rendered by the editor.
    wire_node_editor!(window, setup);
    wire_selection!(window, setup, nodes, links);

    window.on_compute_link_at({
        let controller = controller.clone();
        let links = links.clone();
        move |x, y| {
            let links = (0..links.row_count()).filter_map(|index| {
                let link = links.row_data(index)?;
                Some((link.id, link.start_pin_id, link.end_pin_id))
            });
            controller.cache().borrow().find_bezier_link_at_world(
                x,
                y,
                links,
                controller.screen_distance_to_world(8.0),
                50.0,
                20,
            )
        }
    });

    let next_node_id = Rc::new(Cell::new(3));
    window.on_add_node_requested({
        let nodes = nodes.clone();
        let next_node_id = next_node_id.clone();
        move || {
            let id = next_node_id.get();
            next_node_id.set(id + 1);
            nodes.push(NodeData {
                id,
                title: format!("Node {id}").into(),
                x: 120.0 + id as f32 * 35.0,
                y: 120.0 + id as f32 * 25.0,
                selected: false,
            });
        }
    });

    let next_link_id = Rc::new(Cell::new(1));
    window.on_link_requested({
        let window = window.as_weak();
        let controller = controller.clone();
        let links = links.clone();
        move |pin_a, pin_b| {
            let Some(window) = window.upgrade() else {
                return;
            };
            let cache = controller.cache();
            let cache = cache.borrow();
            let (Some(a), Some(b)) = (
                cache.pin_positions.get(&pin_a),
                cache.pin_positions.get(&pin_b),
            ) else {
                return;
            };
            if a.node_id == b.node_id {
                return;
            }
            let pin_types = PinTypes::get(&window);
            let (output, input) = match (a.pin_type, b.pin_type) {
                (a, b) if a == pin_types.get_output() && b == pin_types.get_input() => {
                    (pin_a, pin_b)
                }
                (a, b) if a == pin_types.get_input() && b == pin_types.get_output() => {
                    (pin_b, pin_a)
                }
                _ => return,
            };
            if links
                .iter()
                .any(|link| link.start_pin_id == output && link.end_pin_id == input)
            {
                return;
            }
            links.push(LinkData {
                id: next_link_id.get(),
                start_pin_id: output,
                end_pin_id: input,
                color: Color::from_rgb_u8(90, 175, 255),
                selected: false,
                line_width: 2.0,
                status: -1,
            });
            next_link_id.set(next_link_id.get() + 1);
        }
    });

    window.on_delete_selected_requested({
        let window = window.as_weak();
        let controller = controller.clone();
        let nodes = nodes.clone();
        let links = links.clone();
        move || {
            let removed: HashSet<i32> = nodes
                .iter()
                .filter(|node| node.selected)
                .map(|node| node.id)
                .collect();
            if removed.is_empty() {
                return;
            }

            let cache = controller.cache();
            let cache = cache.borrow();
            remove_where(&links, |link| {
                [link.start_pin_id, link.end_pin_id].iter().any(|pin| {
                    cache
                        .pin_positions
                        .get(pin)
                        .is_some_and(|position| removed.contains(&position.node_id))
                })
            });
            drop(cache);
            remove_where(&nodes, |node| removed.contains(&node.id));

            if let Some(window) = window.upgrade() {
                let lifecycle = window.global::<NodeEditorInternalCallbacks>();
                for id in &removed {
                    lifecycle.invoke_remove_node(*id);
                }
            }
        }
    });

    QuickStart {
        window,
        #[cfg(test)]
        nodes,
        #[cfg(test)]
        links,
        #[cfg(test)]
        controller,
    }
}

fn main() -> Result<(), slint::PlatformError> {
    build_app().window.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::{
        platform::{PointerEventButton, WindowEvent},
        ComponentHandle, LogicalPosition,
    };

    fn app() -> QuickStart {
        thread_local! {
            static INITIALIZED: Cell<bool> = const { Cell::new(false) };
        }
        INITIALIZED.with(|initialized| {
            if !initialized.get() {
                i_slint_backend_testing::init_no_event_loop();
                initialized.set(true);
            }
        });
        build_app()
    }

    fn pump() {
        slint::platform::update_timers_and_animations();
        slint::platform::update_timers_and_animations();
    }

    fn realize(app: &QuickStart) {
        app.window.show().unwrap();
        pump();
    }

    fn pointer(app: &QuickStart, event: WindowEvent) {
        app.window.window().dispatch_event(event);
        pump();
    }

    fn press(app: &QuickStart, point: (f32, f32)) {
        pointer(
            app,
            WindowEvent::PointerPressed {
                position: LogicalPosition::new(point.0, point.1),
                button: PointerEventButton::Left,
            },
        );
    }

    fn move_to(app: &QuickStart, point: (f32, f32)) {
        pointer(
            app,
            WindowEvent::PointerMoved {
                position: LogicalPosition::new(point.0, point.1),
            },
        );
    }

    fn release(app: &QuickStart, point: (f32, f32)) {
        pointer(
            app,
            WindowEvent::PointerReleased {
                position: LogicalPosition::new(point.0, point.1),
                button: PointerEventButton::Left,
            },
        );
    }

    fn click(app: &QuickStart, point: (f32, f32)) {
        press(app, point);
        release(app, point);
    }

    fn world_to_screen(app: &QuickStart, point: (f32, f32)) -> (f32, f32) {
        (
            app.window.get_editor_screen_x()
                + point.0 * app.window.get_zoom()
                + app.window.get_pan_x(),
            app.window.get_editor_screen_y()
                + point.1 * app.window.get_zoom()
                + app.window.get_pan_y(),
        )
    }

    fn pin_world(app: &QuickStart, pin_id: i32) -> (f32, f32) {
        let cache = app.controller.cache();
        let cache = cache.borrow();
        let pin = &cache.pin_positions[&pin_id];
        let node = &cache.node_rects[&pin.node_id];
        (node.x + pin.rel_x, node.y + pin.rel_y)
    }

    #[test]
    fn standard_macros_drive_drag_link_creation_and_edge_selection() {
        let app = app();
        app.window.set_zoom(1.5);
        app.window.set_pan_x(27.0);
        app.window.set_pan_y(-18.0);
        realize(&app);
        assert!(app.window.get_editor_screen_y() > 0.0);

        let start = world_to_screen(&app, (150.0, 120.0));
        let end = (start.0 + 60.0, start.1 + 45.0);
        press(&app, start);
        move_to(&app, end);
        release(&app, end);

        let moved = app.nodes.row_data(0).unwrap();
        assert_eq!((moved.x, moved.y), (140.0, 130.0));

        let output = world_to_screen(&app, pin_world(&app, 3));
        let input = world_to_screen(&app, pin_world(&app, 4));
        press(&app, output);
        assert!(app.window.get_is_creating_link());
        move_to(&app, input);
        release(&app, input);
        assert_eq!(app.links.row_count(), 1);
        let link = app.links.row_data(0).unwrap();
        assert_eq!((link.start_pin_id, link.end_pin_id), (3, 4));

        let output_world = pin_world(&app, 3);
        let input_world = pin_world(&app, 4);
        click(
            &app,
            world_to_screen(
                &app,
                (
                    (output_world.0 + input_world.0) / 2.0,
                    (output_world.1 + input_world.1) / 2.0,
                ),
            ),
        );
        assert!(app.links.row_data(0).unwrap().selected);

        click(&app, world_to_screen(&app, (510.0, 260.0)));
        assert!(app.nodes.row_data(1).unwrap().selected);
        app.window.invoke_delete_selected_requested();
        assert_eq!(app.nodes.row_count(), 1);
        assert_eq!(app.links.row_count(), 0);
        assert!(!app.controller.cache().borrow().node_rects.contains_key(&2));

        app.window.invoke_add_node_requested();
        app.window.invoke_add_node_requested();
        assert_eq!(app.nodes.row_count(), 3);
    }

    #[test]
    fn base_node_double_click_reaches_the_public_event_global() {
        let app = app();
        let observed = Rc::new(Cell::new(0));
        app.window
            .global::<NodeEditorEvents>()
            .on_node_double_clicked({
                let observed = observed.clone();
                move |node_id| observed.set(node_id)
            });
        realize(&app);

        let point = world_to_screen(&app, (150.0, 120.0));
        click(&app, point);
        click(&app, point);

        assert_eq!(observed.get(), 1);
    }
}
