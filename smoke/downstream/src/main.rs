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
    wire_selection!(window, setup, nodes);

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
    use slint::ComponentHandle;

    fn app() -> QuickStart {
        i_slint_backend_testing::init_no_event_loop();
        build_app()
    }

    fn report_graph(app: &QuickStart) {
        let lifecycle = app.window.global::<NodeEditorInternalCallbacks>();
        lifecycle.invoke_report_node_rect(1, 100.0, 100.0, 180.0, 80.0);
        lifecycle.invoke_report_node_rect(2, 420.0, 220.0, 180.0, 80.0);
        lifecycle.invoke_report_pin_position(2, 1, 1, 0.0, 40.0, true);
        lifecycle.invoke_report_pin_position(3, 1, 2, 180.0, 40.0, true);
        lifecycle.invoke_report_pin_position(4, 2, 1, 0.0, 40.0, true);
        lifecycle.invoke_report_pin_position(5, 2, 2, 180.0, 40.0, true);
    }

    #[test]
    fn quick_start_edits_two_nodes_end_to_end() {
        let app = app();
        report_graph(&app);
        assert_eq!(app.nodes.row_count(), 2);

        app.window.invoke_node_selected(1, false);
        assert!(app.nodes.row_data(0).unwrap().selected);

        app.window
            .global::<NodeEditorInternalCallbacks>()
            .invoke_end_node_drag(1, 25.0, 15.0);
        let moved = app.nodes.row_data(0).unwrap();
        assert_eq!((moved.x, moved.y), (125.0, 115.0));

        app.window.invoke_link_requested(3, 4);
        assert_eq!(app.links.row_count(), 1);
        app.window.invoke_link_requested(4, 3);
        assert_eq!(app.links.row_count(), 1);

        app.window.invoke_node_selected(2, true);
        app.window.invoke_delete_selected_requested();
        assert_eq!(app.nodes.row_count(), 0);
        assert_eq!(app.links.row_count(), 0);
        assert!(!app.controller.cache().borrow().node_rects.contains_key(&1));
        assert!(!app.controller.cache().borrow().node_rects.contains_key(&2));

        app.window.invoke_add_node_requested();
        app.window.invoke_add_node_requested();
        assert_eq!(app.nodes.row_count(), 2);
    }
}
