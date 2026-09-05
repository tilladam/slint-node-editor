// Node Editor Example
//
// Demonstrates the pure Slint NodeEditor component with application-provided
// computation callbacks.

use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{
    selection, wire_node_editor, BasicLinkValidator, CompositeValidator, GraphLogic, LinkData,
    LinkValidator, MinimapNode, MovableNode, NoDuplicatesValidator, NodeEditorSetup,
    ValidationResult,
};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

impl MovableNode for NodeData {
    fn id(&self) -> i32 {
        self.id
    }
    fn x(&self) -> f32 {
        self.world_x
    }
    fn y(&self) -> f32 {
        self.world_y
    }
    fn selected(&self) -> bool {
        self.selected
    }
    fn set_x(&mut self, x: f32) {
        self.world_x = x;
    }
    fn set_y(&mut self, y: f32) {
        self.world_y = y;
    }
}

impl MovableNode for FilterNodeData {
    fn id(&self) -> i32 {
        self.id
    }
    fn x(&self) -> f32 {
        self.world_x
    }
    fn y(&self) -> f32 {
        self.world_y
    }
    fn selected(&self) -> bool {
        self.selected
    }
    fn set_x(&mut self, x: f32) {
        self.world_x = x;
    }
    fn set_y(&mut self, y: f32) {
        self.world_y = y;
    }
}

/// Reads the current selection out of the rows it spans.
type ReadSelection = Rc<dyn Fn() -> Vec<i32>>;
/// Writes an absolute selection into the rows it spans.
type WriteSelection = Rc<dyn Fn(&[i32])>;

/// Helper to remove items by ID from a model based on selection
fn remove_selected_items<T: Clone + 'static>(
    model: &VecModel<T>,
    get_id: impl Fn(&T) -> i32,
    is_selected: impl Fn(&T) -> bool,
) -> Vec<i32> {
    let mut indices_to_remove = Vec::new();
    let mut removed_ids = Vec::new();
    for i in 0..model.row_count() {
        if let Some(item) = model.row_data(i) {
            if is_selected(&item) {
                indices_to_remove.push(i);
                removed_ids.push(get_id(&item));
            }
        }
    }
    for &i in indices_to_remove.iter().rev() {
        model.remove(i);
    }
    removed_ids
}

/// Compute graph bounds from all nodes
fn compute_graph_bounds(
    nodes: &VecModel<NodeData>,
    filter_nodes: &VecModel<FilterNodeData>,
    node_width: f32,
    node_height: f32,
    filter_width: f32,
    filter_height: f32,
) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    let mut update_bounds = |_id: i32, x: f32, y: f32, w: f32, h: f32| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    };

    for i in 0..nodes.row_count() {
        if let Some(n) = nodes.row_data(i) {
            update_bounds(n.id, n.world_x, n.world_y, node_width, node_height);
        }
    }
    for i in 0..filter_nodes.row_count() {
        if let Some(n) = filter_nodes.row_data(i) {
            update_bounds(n.id, n.world_x, n.world_y, filter_width, filter_height);
        }
    }

    if min_x == f32::MAX {
        (0.0, 0.0, 1600.0, 1200.0)
    } else {
        (min_x - 50.0, min_y - 50.0, max_x + 50.0, max_y + 50.0)
    }
}

/// Build minimap nodes from all nodes
fn build_minimap_nodes(
    nodes: &VecModel<NodeData>,
    filter_nodes: &VecModel<FilterNodeData>,
    node_width: f32,
    node_height: f32,
    filter_width: f32,
    filter_height: f32,
) -> ModelRc<MinimapNode> {
    let mut minimap_nodes = Vec::new();

    for i in 0..nodes.row_count() {
        if let Some(node) = nodes.row_data(i) {
            minimap_nodes.push(MinimapNode {
                id: node.id,
                x: node.world_x,
                y: node.world_y,
                width: node_width,
                height: node_height,
                color: Color::from_rgb_u8(80, 120, 200),
            });
        }
    }

    for i in 0..filter_nodes.row_count() {
        if let Some(node) = filter_nodes.row_data(i) {
            minimap_nodes.push(MinimapNode {
                id: node.id,
                x: node.world_x,
                y: node.world_y,
                width: filter_width,
                height: filter_height,
                color: Color::from_rgb_u8(200, 120, 80),
            });
        }
    }

    Rc::new(VecModel::from(minimap_nodes)).into()
}

/// The window plus the models its callbacks mutate.
///
/// Split out of `main` so tests can drive the real wiring: the callbacks
/// installed here are the ones the app runs with.
struct App {
    window: MainWindow,
    // Only the tests reach for the models directly; `main` drives everything
    // through the window's callbacks.
    #[cfg_attr(not(test), allow(dead_code))]
    nodes: Rc<VecModel<NodeData>>,
    #[cfg_attr(not(test), allow(dead_code))]
    filter_nodes: Rc<VecModel<FilterNodeData>>,
}

fn build_app() -> App {
    let window = MainWindow::new().unwrap();

    // Create the node model
    let nodes: Rc<VecModel<NodeData>> = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Input"),
            world_x: 144.0,
            world_y: 264.0,
            selected: false,
        },
        NodeData {
            id: 2,
            title: SharedString::from("Process"),
            world_x: 408.0,
            world_y: 216.0,
            selected: false,
        },
        NodeData {
            id: 3,
            title: SharedString::from("Output"),
            world_x: 648.0,
            world_y: 264.0,
            selected: false,
        },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Create the filter nodes model
    let filter_nodes: Rc<VecModel<FilterNodeData>> =
        Rc::new(VecModel::from(vec![FilterNodeData {
            id: 100,
            title: SharedString::from("Filter"),
            world_x: 408.0,
            world_y: 384.0,
            filter_type_index: 0,
            enabled: true,
            processed_count: 42,
            selected: false,
        }]));
    window.set_filter_nodes(ModelRc::from(filter_nodes.clone()));

    let next_node_id = Rc::new(RefCell::new(4));

    // Create the links model
    let link_colors = [
        Color::from_argb_u8(255, 255, 152, 0),
        Color::from_argb_u8(255, 33, 150, 243),
        Color::from_argb_u8(255, 76, 175, 80),
        Color::from_argb_u8(255, 156, 39, 176),
        Color::from_argb_u8(255, 233, 30, 99),
    ];
    let next_link_id = Rc::new(RefCell::new(3));
    let color_index = Rc::new(RefCell::new(2));

    let links: Rc<VecModel<LinkData>> = Rc::new(VecModel::from(vec![
        LinkData {
            id: 1,
            start_pin_id: 1002,
            end_pin_id: 2001,
            color: link_colors[0],
            line_width: 1.5, // Thin link
            status: -1,
            selected: false,
        },
        LinkData {
            id: 2,
            start_pin_id: 2002,
            end_pin_id: 3001,
            color: link_colors[1],
            line_width: 5.0, // Thick link to demonstrate feature
            status: -1,
            selected: false,
        },
    ]));
    window.set_links(ModelRc::from(links.clone()));

    // Read node layout constants
    let node_constants = NodeConstants::get(&window);
    let filter_node_constants = FilterNodeConstants::get(&window);
    let node_width = node_constants.get_node_base_width();
    let node_height = node_constants.get_node_base_height();
    let filter_width = filter_node_constants.get_base_width();
    let filter_height = filter_node_constants.get_base_height();

    // The minimap model and the graph bounds are snapshots: both copy world
    // positions out of the node models, so a later move of a node does not
    // propagate into them. Rebuild both whenever node geometry or membership
    // changes - on a committed drag, on delete, and on add.
    let refresh_minimap: Rc<dyn Fn()> = {
        let window = window.as_weak();
        let nodes = nodes.clone();
        let filter_nodes = filter_nodes.clone();
        Rc::new(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            window.set_minimap_nodes(build_minimap_nodes(
                &nodes,
                &filter_nodes,
                node_width,
                node_height,
                filter_width,
                filter_height,
            ));
            let (min_x, min_y, max_x, max_y) = compute_graph_bounds(
                &nodes,
                &filter_nodes,
                node_width,
                node_height,
                filter_width,
                filter_height,
            );
            window.set_graph_min_x(min_x);
            window.set_graph_min_y(min_y);
            window.set_graph_max_x(max_x);
            window.set_graph_max_y(max_y);
        })
    };

    // Commit a finished drag into both node models. The dragged node lives in
    // exactly one of them and always moves; selected rows in either move with
    // it. Both read the same `selected` the editor renders.
    let setup = NodeEditorSetup::new({
        let nodes = nodes.clone();
        let filter_nodes = filter_nodes.clone();
        let refresh_minimap = refresh_minimap.clone();
        move |dragged, delta_x, delta_y| {
            GraphLogic::commit_drag(&nodes, dragged, delta_x, delta_y);
            GraphLogic::commit_drag(&filter_nodes, dragged, delta_x, delta_y);
            refresh_minimap();
        }
    });

    // Configure controller
    setup
        .controller()
        .set_grid_spacing(node_constants.get_grid_spacing());

    // Enable minimap and populate it with the initial geometry.
    window.set_minimap_enabled(true);
    refresh_minimap();

    // === Computation Callbacks ===

    // Wire standard callbacks with one macro call
    wire_node_editor!(window, setup);

    window.on_compute_link_at({
        let ctrl = setup.controller().clone();
        let links = links.clone();
        let w = window.as_weak();
        move |x, y| {
            let w = match w.upgrade() {
                Some(w) => w,
                None => return -1,
            };
            let cache = ctrl.cache();
            let cache = cache.borrow();
            let link_iter = (0..links.row_count())
                .filter_map(|i| links.row_data(i))
                .map(|l| (l.id, l.start_pin_id, l.end_pin_id));
            cache.find_link_at(
                x,
                y,
                link_iter,
                w.get_link_hover_distance(),
                w.get_zoom(),
                w.get_bezier_min_offset(),
                w.get_link_hit_samples() as usize,
            )
        }
    });

    // === Selection ===
    // The rows ARE the selection — there is no separate store to keep in step
    // with them. Each gesture reads the current set back out of the rows,
    // resolves it, and writes the answer in. This example has two node models
    // sharing one id space, so "the node selection" spans both; that is the
    // only reason these four accessors exist, and it is why this example wires
    // the callbacks by hand instead of using `wire_selection!`.

    let node_selection: ReadSelection = {
        let nodes = nodes.clone();
        let filter_nodes = filter_nodes.clone();
        Rc::new(move || {
            let mut ids = selection::selected_rows(&nodes, |n| n.id, |n| n.selected);
            ids.extend(selection::selected_rows(
                &filter_nodes,
                |n| n.id,
                |n| n.selected,
            ));
            ids
        })
    };
    let set_node_selection: WriteSelection = {
        let nodes = nodes.clone();
        let filter_nodes = filter_nodes.clone();
        Rc::new(move |next: &[i32]| {
            selection::project_selection(&nodes, |n| next.contains(&n.id), |n| &mut n.selected);
            selection::project_selection(
                &filter_nodes,
                |n| next.contains(&n.id),
                |n| &mut n.selected,
            );
        })
    };
    let link_selection: ReadSelection = {
        let links = links.clone();
        Rc::new(move || selection::selected_rows(&links, |l| l.id, |l| l.selected))
    };
    let set_link_selection: WriteSelection = {
        let links = links.clone();
        Rc::new(move |next: &[i32]| {
            selection::project_selection(&links, |l| next.contains(&l.id), |l| &mut l.selected);
        })
    };

    window.on_node_selected({
        let current = node_selection.clone();
        let set_nodes = set_node_selection.clone();
        let set_links = set_link_selection.clone();
        move |node_id, shift| {
            // A plain click is exclusive across kinds; shift adds to what's
            // there, so it leaves the other kind alone (same as the marquee
            // below, and as `wire_selection!`'s two-model arm).
            if !shift {
                set_links(&[]);
            }
            set_nodes(&selection::resolve_click(&current(), node_id, shift));
        }
    });

    window.on_select_link({
        let current = link_selection.clone();
        let set_nodes = set_node_selection.clone();
        let set_links = set_link_selection.clone();
        move |link_id, shift| {
            if !shift {
                set_nodes(&[]);
            }
            set_links(&selection::resolve_click(&current(), link_id, shift));
        }
    });

    window.on_selection_cleared({
        let set_nodes = set_node_selection.clone();
        let set_links = set_link_selection.clone();
        move || {
            set_nodes(&[]);
            set_links(&[]);
        }
    });

    window.on_box_selection_committed({
        let ctrl = setup.controller().clone();
        let links = links.clone();
        let node_current = node_selection.clone();
        let link_current = link_selection.clone();
        let set_nodes = set_node_selection.clone();
        let set_links = set_link_selection.clone();
        move |x, y, w, h, shift| {
            let (hit_nodes, hit_links) = {
                let cache = ctrl.cache();
                let cache = cache.borrow();
                let link_iter = (0..links.row_count())
                    .filter_map(|i| links.row_data(i))
                    .map(|l| (l.id, l.start_pin_id, l.end_pin_id));
                (
                    cache.nodes_in_selection_box(x, y, w, h),
                    cache.links_in_selection_box(x, y, w, h, link_iter),
                )
            };
            set_nodes(&selection::resolve_box(&node_current(), hit_nodes, shift));
            set_links(&selection::resolve_box(&link_current(), hit_links, shift));
        }
    });

    // === Event Callbacks ===

    window.on_create_link({
        let ctrl = setup.controller().clone();
        let links = links.clone();
        let next_link_id = next_link_id.clone();
        let color_index = color_index.clone();
        let w = window.as_weak();
        move |start_pin, end_pin| {
            let w = match w.upgrade() {
                Some(w) => w,
                None => return,
            };
            let cache = ctrl.cache();
            let cache = cache.borrow();

            // Get pin type constants from Slint's PinTypes global
            let pin_types = PinTypes::get(&w);
            let output_type = pin_types.get_output();

            // Validate link using the new validator framework
            let validator: CompositeValidator<_, LinkData> = CompositeValidator::new()
                .with(BasicLinkValidator::new(output_type))
                .with(NoDuplicatesValidator);

            let links_vec: Vec<LinkData> = links.iter().collect();
            match validator.validate(start_pin, end_pin, &cache, &links_vec) {
                ValidationResult::Valid => {}
                ValidationResult::Invalid(_err) => {
                    // Could log or display error here: eprintln!("Cannot create link: {}", err);
                    return;
                }
            }

            let (output_pin, input_pin) =
                match GraphLogic::normalize_link_direction(start_pin, end_pin, &cache, output_type)
                {
                    Some(p) => p,
                    None => return,
                };

            let id = *next_link_id.borrow();
            *next_link_id.borrow_mut() += 1;
            let idx = *color_index.borrow();
            *color_index.borrow_mut() = (idx + 1) % link_colors.len();
            let color = link_colors[idx];

            if let Some(_path) = cache.compute_link_path(
                output_pin,
                input_pin,
                w.get_zoom(),
                w.get_bezier_min_offset(),
            ) {
                let data = LinkData {
                    id,
                    start_pin_id: output_pin,
                    end_pin_id: input_pin,
                    color,
                    line_width: 2.0,
                    status: -1,
                    selected: false,
                };
                links.push(data);
            }
        }
    });

    window.on_delete_selected_nodes({
        let ctrl = setup.controller().clone();
        let nodes = nodes.clone();
        let filter_nodes = filter_nodes.clone();
        let links = links.clone();
        let refresh_minimap = refresh_minimap.clone();
        move || {
            let mut deleted_node_ids = remove_selected_items(&nodes, |n| n.id, |n| n.selected);
            deleted_node_ids.extend(remove_selected_items(
                &filter_nodes,
                |n| n.id,
                |n| n.selected,
            ));

            let cache = ctrl.cache();
            let cache = cache.borrow();
            let mut link_indices_to_remove: Vec<usize> = Vec::new();

            for i in 0..links.row_count() {
                if let Some(link) = links.row_data(i) {
                    let start_node = cache
                        .pin_positions
                        .get(&link.start_pin_id)
                        .map(|p| p.node_id);
                    let end_node = cache.pin_positions.get(&link.end_pin_id).map(|p| p.node_id);
                    if start_node.is_some_and(|id| deleted_node_ids.contains(&id))
                        || end_node.is_some_and(|id| deleted_node_ids.contains(&id))
                    {
                        link_indices_to_remove.push(i);
                    }
                }
            }
            drop(cache);

            for &i in link_indices_to_remove.iter().rev() {
                links.remove(i);
            }

            refresh_minimap();
        }
    });

    let links_for_link_delete = links.clone();
    window.on_delete_selected_links(move || {
        remove_selected_items(&links_for_link_delete, |l| l.id, |l| l.selected);
    });

    let nodes_for_add = nodes.clone();
    let next_node_id_for_add = next_node_id.clone();
    let refresh_minimap_for_add = refresh_minimap.clone();
    window.on_add_node(move || {
        let id = *next_node_id_for_add.borrow();
        *next_node_id_for_add.borrow_mut() += 1;
        nodes_for_add.push(NodeData {
            id,
            title: SharedString::from(format!("Node {}", id)),
            world_x: 192.0 + (id as f32 * 48.0) % 384.0,
            world_y: 192.0 + (id as f32 * 24.0) % 288.0,
            selected: false,
        });

        refresh_minimap_for_add();
    });

    let filter_nodes_for_type = filter_nodes.clone();
    window.on_filter_type_changed(move |id, idx| {
        if let Some((i, mut node)) =
            GraphLogic::find_node_by_id(&filter_nodes_for_type, id, |n| n.id)
        {
            node.filter_type_index = idx;
            filter_nodes_for_type.set_row_data(i, node);
        }
    });

    let filter_nodes_for_enable = filter_nodes.clone();
    window.on_filter_toggle_enabled(move |id| {
        if let Some((i, mut node)) =
            GraphLogic::find_node_by_id(&filter_nodes_for_enable, id, |n| n.id)
        {
            node.enabled = !node.enabled;
            filter_nodes_for_enable.set_row_data(i, node);
        }
    });

    let filter_nodes_for_reset = filter_nodes.clone();
    window.on_filter_reset(move |id| {
        if let Some((i, mut node)) =
            GraphLogic::find_node_by_id(&filter_nodes_for_reset, id, |n| n.id)
        {
            node.processed_count = 0;
            node.filter_type_index = 0;
            node.enabled = true;
            filter_nodes_for_reset.set_row_data(i, node);
        }
    });

    App {
        window,
        nodes,
        filter_nodes,
    }
}

fn main() {
    let app = build_app();
    app.window.invoke_request_grid_update();
    app.window.run().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_backend() {
        use std::cell::Cell;
        thread_local! {
            static INITIALIZED: Cell<bool> = const { Cell::new(false) };
        }
        INITIALIZED.with(|init| {
            if !init.get() {
                i_slint_backend_testing::init_no_event_loop();
                init.set(true);
            }
        });
    }

    fn app() -> App {
        init_backend();
        build_app()
    }

    /// Minimap rows, as (id, x, y).
    fn minimap(app: &App) -> Vec<(i32, f32, f32)> {
        app.window
            .get_minimap_nodes()
            .iter()
            .map(|n| (n.id, n.x, n.y))
            .collect()
    }

    fn find(rows: &[(i32, f32, f32)], id: i32) -> (f32, f32) {
        rows.iter()
            .find(|r| r.0 == id)
            .map(|r| (r.1, r.2))
            .unwrap_or_else(|| panic!("no minimap row for node {id}, have {rows:?}"))
    }

    fn select(nodes: &VecModel<NodeData>, id: i32) {
        let (i, mut node) =
            GraphLogic::find_node_by_id(nodes, id, |n| n.id).expect("node to select");
        node.selected = true;
        nodes.set_row_data(i, node);
    }

    /// The minimap is a snapshot of the node models, so a committed drag has to
    /// rebuild it. Regression test: it used to be built once during setup and
    /// never again, leaving the minimap showing the startup layout forever.
    #[test]
    fn minimap_follows_a_committed_drag() {
        let app = app();
        let before = find(&minimap(&app), 1);

        app.window
            .global::<NodeEditorInternalCallbacks>()
            .invoke_end_node_drag(1, 120.0, 90.0);

        let after = find(&minimap(&app), 1);
        assert_eq!(
            (after.0 - before.0, after.1 - before.1),
            (120.0, 90.0),
            "minimap row for node 1 should move by the committed delta"
        );
    }

    /// Deleting a node has to shrink the minimap too.
    #[test]
    fn minimap_drops_a_deleted_node() {
        let app = app();
        let before = minimap(&app);
        assert!(before.iter().any(|r| r.0 == 1));

        select(&app.nodes, 1);
        app.window.invoke_delete_selected_nodes();

        let after = minimap(&app);
        assert_eq!(after.len(), before.len() - 1);
        assert!(
            !after.iter().any(|r| r.0 == 1),
            "deleted node should be gone from the minimap, got {after:?}"
        );
    }

    /// ...and adding one has to grow it.
    #[test]
    fn minimap_gains_an_added_node() {
        let app = app();
        let before = minimap(&app);

        app.window.invoke_add_node();

        let after = minimap(&app);
        assert_eq!(after.len(), before.len() + 1);
    }

    /// The graph bounds are the other half of the same snapshot: they drive the
    /// minimap's viewport indicator, and were also only computed once.
    #[test]
    fn graph_bounds_follow_a_committed_drag() {
        let app = app();
        let before = app.window.get_graph_max_x();

        // Node 3 is the right-most node, so pushing it right must widen the graph.
        app.window
            .global::<NodeEditorInternalCallbacks>()
            .invoke_end_node_drag(3, 200.0, 0.0);

        assert_eq!(
            app.window.get_graph_max_x() - before,
            200.0,
            "graph bounds should widen by the committed delta"
        );
    }

    /// Every node model feeds the minimap, not just the plain ones.
    #[test]
    fn minimap_covers_filter_nodes() {
        let app = app();
        let rows = minimap(&app);
        for i in 0..app.filter_nodes.row_count() {
            let id = app.filter_nodes.row_data(i).unwrap().id;
            assert!(
                rows.iter().any(|r| r.0 == id),
                "filter node {id} missing from minimap"
            );
        }
    }
    /// Horizontal extent of a text run, found by its rendered string.
    ///
    /// `Text` sets `accessible-label` to its own text by default, so the label
    /// is the handle. Returns `(left, right)` in window coordinates.
    fn text_span(app: &App, label: &str) -> (f32, f32) {
        let e = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
            &app.window, label,
        )
        .next()
        .unwrap_or_else(|| panic!("no element with accessible label {label:?}"));
        let x = e.absolute_position().x;
        (x, x + e.size().width)
    }

    /// The filter node's pin labels sit in a fixed-width column and are offset
    /// far enough into it to clear the pin. Regression test: the column was
    /// narrower than the offset plus the label, and `Rectangle` does not clip,
    /// so `Ctrl` rendered 8px into `Active` and `In` grazed `Type:`.
    #[test]
    fn filter_node_pin_labels_do_not_overlap_the_content_column() {
        let app = app();

        for (pin_label, content_label) in [("Ctrl", "Active"), ("In", "Type:")] {
            let (_, pin_right) = text_span(&app, pin_label);
            let (content_left, _) = text_span(&app, content_label);
            assert!(
                pin_right <= content_left,
                "{pin_label:?} ends at {pin_right} but {content_label:?} starts at \
                 {content_left} — overlapping by {}px",
                pin_right - content_left
            );
        }
    }

    /// The right-hand label is width-constrained rather than free-flowing, so
    /// it fails the opposite way: too narrow a column truncates it instead of
    /// overflowing. Guard the width it actually needs.
    #[test]
    fn filter_node_out_label_is_not_truncated() {
        let app = app();
        let (left, right) = text_span(&app, "Out");
        assert!(
            right - left >= 15.0,
            "\"Out\" got {}px, too narrow to render at font-size 10",
            right - left
        );
    }

    /// Widening the pin columns comes out of the content column, so the widget
    /// that has to absorb it needs a floor. Guards the trade-off rather than
    /// the widget: if a future change re-widens the pin columns, this is what
    /// says the ComboBox has run out of room.
    #[test]
    fn filter_node_combobox_keeps_usable_width() {
        let app = app();
        let combo = i_slint_backend_testing::ElementHandle::find_by_element_type_name(
            &app.window, "ComboBox",
        )
        .next()
        .expect("the filter node has a ComboBox");
        let width = combo.size().width;
        assert!(
            width >= 100.0,
            "ComboBox is {width}px wide, below the 100px floor"
        );
    }

    /// The fix was supposed to be absorbed by the content column, not paid for
    /// by a wider node.
    #[test]
    fn filter_node_width_is_unchanged() {
        let app = app();
        let node = i_slint_backend_testing::ElementHandle::find_by_element_type_name(
            &app.window, "FilterNode",
        )
        .next()
        .expect("a FilterNode is instantiated");
        assert_eq!(node.size().width, 260.0, "filter node width");
    }

    /// Everything the node draws has to stay inside the node.
    ///
    /// Regression test, and the one that matters most here: the first attempt
    /// at fixing the label overlap widened the pin columns and assumed the
    /// content column would absorb it. It did not — the ComboBox has a
    /// minimum width — so the layout overflowed and pushed the right-hand pin
    /// column off the node entirely, trading a text overlap for a worse one.
    /// Checking the labels against each other could not see that; only
    /// checking them against the node's own bounds can.
    #[test]
    fn filter_node_content_stays_inside_the_node() {
        let app = app();
        let node = i_slint_backend_testing::ElementHandle::find_by_element_type_name(
            &app.window, "FilterNode",
        )
        .next()
        .expect("a FilterNode is instantiated");
        let left = node.absolute_position().x;
        let right = left + node.size().width;

        for label in ["In", "Ctrl", "Type:", "Active", "Out", "Bypass", "Reset"] {
            let (l, r) = text_span(&app, label);
            assert!(
                l >= left && r <= right,
                "{label:?} spans [{l},{r}], outside the node's [{left},{right}]"
            );
        }
    }

}
