// Node Editor Example
//
// Demonstrates the pure Slint NodeEditor component with application-provided
// computation callbacks.

use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{
    GraphLogic, LinkModel, MovableNode, NodeEditorController, SelectionManager,
    BasicLinkValidator, NoDuplicatesValidator, CompositeValidator, LinkValidator, ValidationResult,
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
    fn set_x(&mut self, x: f32) {
        self.world_x = x;
    }
    fn set_y(&mut self, y: f32) {
        self.world_y = y;
    }
}

impl LinkModel for LinkData {
    fn id(&self) -> i32 {
        self.id
    }
    fn start_pin_id(&self) -> i32 {
        self.start_pin_id
    }
    fn end_pin_id(&self) -> i32 {
        self.end_pin_id
    }
    fn line_width(&self) -> f32 {
        self.line_width
    }
}

/// Helper to remove items by ID from a model based on selection
fn remove_selected_items<T: Clone + 'static>(
    model: &VecModel<T>,
    get_id: impl Fn(&T) -> i32,
    selection: &SelectionManager,
) -> Vec<i32> {
    let mut indices_to_remove = Vec::new();
    let mut removed_ids = Vec::new();
    for i in 0..model.row_count() {
        if let Some(item) = model.row_data(i) {
            let id = get_id(&item);
            if selection.contains(id) {
                indices_to_remove.push(i);
                removed_ids.push(id);
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

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();

    let selection_manager = Rc::new(RefCell::new(SelectionManager::new()));
    let link_selection_manager = Rc::new(RefCell::new(SelectionManager::new()));

    // Create the node model
    let nodes: Rc<VecModel<NodeData>> = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Input"),
            world_x: 144.0,
            world_y: 264.0,
        },
        NodeData {
            id: 2,
            title: SharedString::from("Process"),
            world_x: 408.0,
            world_y: 216.0,
        },
        NodeData {
            id: 3,
            title: SharedString::from("Output"),
            world_x: 648.0,
            world_y: 264.0,
        },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Create the filter nodes model
    let filter_nodes: Rc<VecModel<FilterNodeData>> = Rc::new(VecModel::from(vec![FilterNodeData {
        id: 100,
        title: SharedString::from("Filter"),
        world_x: 408.0,
        world_y: 384.0,
        filter_type_index: 0,
        enabled: true,
        processed_count: 42,
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
        },
        LinkData {
            id: 2,
            start_pin_id: 2002,
            end_pin_id: 3001,
            color: link_colors[1],
            line_width: 5.0, // Thick link to demonstrate feature
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

    // Configure controller
    ctrl.set_grid_spacing(node_constants.get_grid_spacing());

    // Create selection models
    let selected_node_ids: Rc<VecModel<i32>> = Rc::new(VecModel::default());
    let selected_link_ids: Rc<VecModel<i32>> = Rc::new(VecModel::default());
    window.set_selected_node_ids(ModelRc::from(selected_node_ids.clone()));
    window.set_selected_link_ids(ModelRc::from(selected_link_ids.clone()));

    // Enable minimap
    window.set_minimap_enabled(true);
    window.set_minimap_nodes(build_minimap_nodes(&nodes, &filter_nodes, node_width, node_height, filter_width, filter_height));

    let (min_x, min_y, max_x, max_y) = compute_graph_bounds(&nodes, &filter_nodes, node_width, node_height, filter_width, filter_height);
    window.set_graph_min_x(min_x);
    window.set_graph_min_y(min_y);
    window.set_graph_max_x(max_x);
    window.set_graph_max_y(max_y);

    // === Computation Callbacks ===

    window.on_request_grid_update({
        let ctrl = ctrl.clone();
        let w = window.as_weak();
        move || {
            if let Some(w) = w.upgrade() {
                w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
            }
        }
    });

    window.on_pin_position_changed({
        let ctrl = ctrl.clone();
        move |pin_id, node_id, pin_type, rel_x, rel_y| {
            ctrl.handle_pin_position(pin_id, node_id, pin_type, rel_x, rel_y);
        }
    });

    window.on_node_rect_changed({
        let ctrl = ctrl.clone();
        move |id, x, y, width, height| {
            ctrl.handle_node_rect(id, x, y, width, height);
        }
    });

    window.on_compute_pin_at({
        let ctrl = ctrl.clone();
        let w = window.as_weak();
        move |x, y| {
            let w = match w.upgrade() { Some(w) => w, None => return 0 };
            ctrl.cache().borrow().find_pin_at(x as f32, y as f32, w.get_pin_hit_radius() as f32)
        }
    });

    window.on_compute_link_at({
        let ctrl = ctrl.clone();
        let links = links.clone();
        let w = window.as_weak();
        move |x, y| {
            let w = match w.upgrade() { Some(w) => w, None => return -1 };
            let cache = ctrl.cache();
            let cache = cache.borrow();
            let link_iter = (0..links.row_count()).filter_map(|i| links.row_data(i)).map(|l| (l.id, l.start_pin_id, l.end_pin_id));
            cache.find_link_at(x as f32, y as f32, link_iter, w.get_link_hover_distance() as f32, w.get_zoom(), w.get_bezier_min_offset(), w.get_link_hit_samples() as usize)
        }
    });

    window.on_compute_box_selection({
        let ctrl = ctrl.clone();
        move |x, y, w, h| {
            ModelRc::from(Rc::new(VecModel::from(ctrl.cache().borrow().nodes_in_selection_box(x as f32, y as f32, w as f32, h as f32))))
        }
    });

    window.on_compute_link_box_selection({
        let ctrl = ctrl.clone();
        let links = links.clone();
        move |x, y, w, h| {
            let cache = ctrl.cache();
            let cache = cache.borrow();
            let link_iter = (0..links.row_count()).filter_map(|i| links.row_data(i)).map(|l| (l.id, l.start_pin_id, l.end_pin_id));
            ModelRc::from(Rc::new(VecModel::from(cache.links_in_selection_box(x as f32, y as f32, w as f32, h as f32, link_iter))))
        }
    });

    window.on_compute_link_path({
        let ctrl = ctrl.clone();
        let w = window.as_weak();
        move |start_pin, end_pin, _version, _zoom: f32, _pan_x: f32, _pan_y: f32| {
            let w = match w.upgrade() { Some(w) => w, None => return SharedString::default() };
            ctrl.cache().borrow()
                .compute_link_path(start_pin, end_pin, w.get_zoom(), w.get_bezier_min_offset())
                .unwrap_or_default()
                .into()
        }
    });

    let window_for_preview = window.as_weak();
    window.on_compute_link_preview_path(move |start_x, start_y, end_x, end_y| {
        let w = match window_for_preview.upgrade() { Some(w) => w, None => return "".into() };
        slint_node_editor::generate_bezier_path(start_x as f32, start_y as f32, end_x as f32, end_y as f32, w.get_zoom(), w.get_bezier_min_offset()).into()
    });

    // === Selection Checking Callbacks ===

    let sm_check = selection_manager.clone();
    window.on_is_node_selected(move |id| sm_check.borrow().contains(id));

    let lsm_check = link_selection_manager.clone();
    window.on_is_link_selected(move |id| lsm_check.borrow().contains(id));

    // === Selection Manipulation Callbacks ===

    let sn_ids = selected_node_ids.clone();
    let sl_ids = selected_link_ids.clone();
    let sm_select = selection_manager.clone();
    let lsm_select = link_selection_manager.clone();
    let window_for_select_node = window.as_weak();
    window.on_select_node(move |node_id, shift| {
        lsm_select.borrow_mut().clear();
        lsm_select.borrow().sync_to_model(&*sl_ids);

        let mut sm = sm_select.borrow_mut();
        sm.handle_interaction(node_id, shift);
        sm.sync_to_model(&*sn_ids);

        if let Some(w) = window_for_select_node.upgrade() {
            w.set_selection_version(w.get_selection_version() + 1);
            w.invoke_selection_changed();
        }
    });

    let sn_ids_l = selected_node_ids.clone();
    let sl_ids_l = selected_link_ids.clone();
    let sm_select_l = selection_manager.clone();
    let lsm_select_l = link_selection_manager.clone();
    let window_for_select_link = window.as_weak();
    window.on_select_link(move |link_id, shift| {
        sm_select_l.borrow_mut().clear();
        sm_select_l.borrow().sync_to_model(&*sn_ids_l);

        let mut lsm = lsm_select_l.borrow_mut();
        if link_id >= 0 {
            lsm.handle_interaction(link_id, shift);
        } else {
            lsm.clear();
        }
        lsm.sync_to_model(&*sl_ids_l);

        if let Some(w) = window_for_select_link.upgrade() {
            w.set_selection_version(w.get_selection_version() + 1);
            w.invoke_selection_changed();
        }
    });

    let sn_ids_c = selected_node_ids.clone();
    let sl_ids_c = selected_link_ids.clone();
    let sm_clear = selection_manager.clone();
    let lsm_clear = link_selection_manager.clone();
    let window_for_clear = window.as_weak();
    window.on_clear_selection(move || {
        sm_clear.borrow_mut().clear();
        sm_clear.borrow().sync_to_model(&*sn_ids_c);
        lsm_clear.borrow_mut().clear();
        lsm_clear.borrow().sync_to_model(&*sl_ids_c);

        if let Some(w) = window_for_clear.upgrade() {
            w.set_selection_version(w.get_selection_version() + 1);
            w.invoke_selection_changed();
        }
    });

    let sm_sync = selection_manager.clone();
    let window_for_sync = window.as_weak();
    window.on_sync_selection_to_nodes(move |ids_model| {
        sm_sync.borrow_mut().sync_from_model(&ids_model);
        if let Some(w) = window_for_sync.upgrade() { w.set_selection_version(w.get_selection_version() + 1); }
    });

    let lsm_sync = link_selection_manager.clone();
    let window_for_sync_links = window.as_weak();
    window.on_sync_selection_to_links(move |ids_model| {
        lsm_sync.borrow_mut().sync_from_model(&ids_model);
        if let Some(w) = window_for_sync_links.upgrade() { w.set_selection_version(w.get_selection_version() + 1); }
    });

    // === Event Callbacks ===

    window.on_create_link({
        let ctrl = ctrl.clone();
        let links = links.clone();
        let next_link_id = next_link_id.clone();
        let color_index = color_index.clone();
        let w = window.as_weak();
        move |start_pin, end_pin| {
            let w = match w.upgrade() { Some(w) => w, None => return };
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
                ValidationResult::Valid => {},
                ValidationResult::Invalid(_err) => {
                    // Could log or display error here: eprintln!("Cannot create link: {}", err);
                    return;
                }
            }

            let (output_pin, input_pin) = match GraphLogic::normalize_link_direction(start_pin, end_pin, &cache, output_type) { Some(p) => p, None => return };

            let id = *next_link_id.borrow();
            *next_link_id.borrow_mut() += 1;
            let idx = *color_index.borrow();
            *color_index.borrow_mut() = (idx + 1) % link_colors.len();
            let color = link_colors[idx];

            if let Some(_path) = cache.compute_link_path(output_pin, input_pin, w.get_zoom(), w.get_bezier_min_offset()) {
                let data = LinkData { id, start_pin_id: output_pin, end_pin_id: input_pin, color, line_width: 2.0 };
                links.push(data);
            }
        }
    });

    window.on_update_viewport({
        let ctrl = ctrl.clone();
        let w = window.as_weak();
        move |zoom, pan_x, pan_y| {
            let w = match w.upgrade() { Some(w) => w, None => return };
            ctrl.set_zoom(zoom);

            // Update grid
            w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
        }
    });

    let nodes_for_drag = nodes.clone();
    let filter_nodes_for_drag = filter_nodes.clone();
    let sm_drag = selection_manager.clone();
    window.on_commit_drag(move |dx, dy| {
        let sm = sm_drag.borrow();
        GraphLogic::commit_drag(&nodes_for_drag, &sm, dx, dy);
        GraphLogic::commit_drag(&filter_nodes_for_drag, &sm, dx, dy);
    });

    window.on_delete_selected_nodes({
        let ctrl = ctrl.clone();
        let nodes = nodes.clone();
        let filter_nodes = filter_nodes.clone();
        let links = links.clone();
        let sm = selection_manager.clone();
        move || {
            let sm = sm.borrow();
            let mut deleted_node_ids = remove_selected_items(&nodes, |n| n.id, &sm);
            deleted_node_ids.extend(remove_selected_items(&filter_nodes, |n| n.id, &sm));

            let cache = ctrl.cache();
            let cache = cache.borrow();
            let mut link_indices_to_remove: Vec<usize> = Vec::new();

            for i in 0..links.row_count() {
                if let Some(link) = links.row_data(i) {
                    let start_node = cache.pin_positions.get(&link.start_pin_id).map(|p| p.node_id);
                    let end_node = cache.pin_positions.get(&link.end_pin_id).map(|p| p.node_id);
                    if start_node.map_or(false, |id| deleted_node_ids.contains(&id)) || end_node.map_or(false, |id| deleted_node_ids.contains(&id)) {
                        link_indices_to_remove.push(i);
                    }
                }
            }
            drop(cache);

            for &i in link_indices_to_remove.iter().rev() { links.remove(i); }
        }
    });

    let links_for_link_delete = links.clone();
    let lsm_delete = link_selection_manager.clone();
    window.on_delete_selected_links(move || {
        let lsm = lsm_delete.borrow();
        let mut indices_to_remove: Vec<usize> = Vec::new();
        for i in 0..links_for_link_delete.row_count() {
            if let Some(link) = links_for_link_delete.row_data(i) {
                if lsm.contains(link.id) { indices_to_remove.push(i); }
            }
        }
        for &i in indices_to_remove.iter().rev() { links_for_link_delete.remove(i); }
    });

    let nodes_for_add = nodes.clone();
    let next_node_id_for_add = next_node_id.clone();
    let window_for_add = window.as_weak();
    window.on_add_node(move || {
        let w = match window_for_add.upgrade() { Some(w) => w, None => return };
        let id = *next_node_id_for_add.borrow();
        *next_node_id_for_add.borrow_mut() += 1;
        nodes_for_add.push(NodeData { id, title: SharedString::from(format!("Node {}", id)), world_x: w.invoke_snap_to_grid(192.0 + (id as f32 * 48.0) % 384.0), world_y: w.invoke_snap_to_grid(192.0 + (id as f32 * 24.0) % 288.0) });
    });

    let filter_nodes_for_type = filter_nodes.clone();
    window.on_filter_type_changed(move |id, idx| {
        if let Some((i, mut node)) = GraphLogic::find_node_by_id(&filter_nodes_for_type, id, |n| n.id) {
            node.filter_type_index = idx;
            filter_nodes_for_type.set_row_data(i, node);
        }
    });

    let filter_nodes_for_enable = filter_nodes.clone();
    window.on_filter_toggle_enabled(move |id| {
        if let Some((i, mut node)) = GraphLogic::find_node_by_id(&filter_nodes_for_enable, id, |n| n.id) {
            node.enabled = !node.enabled;
            filter_nodes_for_enable.set_row_data(i, node);
        }
    });

    let filter_nodes_for_reset = filter_nodes.clone();
    window.on_filter_reset(move |id| {
        if let Some((i, mut node)) = GraphLogic::find_node_by_id(&filter_nodes_for_reset, id, |n| n.id) {
            node.processed_count = 0;
            node.filter_type_index = 0;
            node.enabled = true;
            filter_nodes_for_reset.set_row_data(i, node);
        }
    });

    window.invoke_request_grid_update();
    window.run().unwrap();
}


