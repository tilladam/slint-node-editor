use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{sugiyama_layout, Direction, NodeEditorController, SugiyamaConfig};

slint::include_modules!();

/// Deterministic LCG — produces the same sequence on first click after each
/// restart, but varies across subsequent clicks within a session.
fn random_f32() -> f32 {
    thread_local! { static SEED: Cell<u64> = Cell::new(12345); }
    SEED.with(|s| {
        let v = s.get().wrapping_mul(6364136223846793005).wrapping_add(1);
        s.set(v);
        ((v >> 33) as f32) / (u32::MAX as f32)
    })
}

/// Build an index from node_id → model row for O(1) lookups.
fn build_node_index(nodes: &VecModel<NodeData>) -> HashMap<i32, usize> {
    (0..nodes.row_count())
        .filter_map(|i| nodes.row_data(i).map(|n| (n.id, i)))
        .collect()
}

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();
    let w = window.as_weak();

    // Create a DAG with 8 nodes:
    //
    //   1 ──► 2 ──► 4 ──► 7
    //   │     │     │
    //   ▼     ▼     ▼
    //   3 ──► 5 ──► 6 ──► 8
    //
    let nodes = Rc::new(VecModel::from(vec![
        NodeData { id: 1, title: SharedString::from("Input"),     x: 50.0,  y: 50.0 },
        NodeData { id: 2, title: SharedString::from("Parse"),     x: 50.0,  y: 120.0 },
        NodeData { id: 3, title: SharedString::from("Validate"),  x: 50.0,  y: 190.0 },
        NodeData { id: 4, title: SharedString::from("Transform"), x: 50.0,  y: 260.0 },
        NodeData { id: 5, title: SharedString::from("Filter"),    x: 50.0,  y: 330.0 },
        NodeData { id: 6, title: SharedString::from("Merge"),     x: 50.0,  y: 400.0 },
        NodeData { id: 7, title: SharedString::from("Format"),    x: 50.0,  y: 470.0 },
        NodeData { id: 8, title: SharedString::from("Output"),    x: 50.0,  y: 540.0 },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // DAG edges as (source_node_id, target_node_id) — single source of truth
    // Pin encoding: input = id*2, output = id*2+1
    let dag_edges: Vec<(i32, i32)> = vec![
        (1, 2), (1, 3),
        (2, 4), (2, 5),
        (3, 5),
        (4, 6), (4, 7),
        (5, 6),
        (6, 8), (7, 8),
    ];

    // Derive LinkData from dag_edges so they can't drift out of sync
    let link_color = Color::from_argb_u8(255, 100, 180, 255);
    let link_data: Vec<LinkData> = dag_edges
        .iter()
        .enumerate()
        .map(|(i, &(src, dst))| LinkData {
            id: (i + 1) as i32,
            start_pin_id: src * 2 + 1, // output pin of source
            end_pin_id: dst * 2,        // input pin of target
            color: link_color,
            line_width: 2.0,
        })
        .collect();
    window.set_links(ModelRc::from(Rc::new(VecModel::from(link_data))));

    // Layout button callback
    window.on_layout_requested({
        let nodes = nodes.clone();
        let dag_edges = dag_edges.clone();
        move || {
            let node_sizes: Vec<(i32, (f64, f64))> = (0..nodes.row_count())
                .filter_map(|i| nodes.row_data(i))
                .map(|n| (n.id, (120.0, 60.0)))
                .collect();

            let mut config = SugiyamaConfig::default();
            config.vertex_spacing = 60.0;
            config.direction = Direction::LeftToRight;

            let positions = sugiyama_layout(&dag_edges, &node_sizes, &config);

            let index = build_node_index(&nodes);
            let offset_x = 80.0_f32;
            let offset_y = 100.0_f32;
            for pos in &positions {
                if let Some(&row) = index.get(&pos.id) {
                    if let Some(mut node) = nodes.row_data(row) {
                        node.x = pos.x as f32 + offset_x;
                        node.y = pos.y as f32 + offset_y;
                        nodes.set_row_data(row, node);
                    }
                }
            }
        }
    });

    // Scramble button callback
    window.on_scramble_requested({
        let nodes = nodes.clone();
        move || {
            for i in 0..nodes.row_count() {
                if let Some(mut node) = nodes.row_data(i) {
                    node.x = 50.0 + random_f32() * 800.0;
                    node.y = 50.0 + random_f32() * 500.0;
                    nodes.set_row_data(i, node);
                }
            }
        }
    });

    // Core callbacks
    window.on_compute_link_path(ctrl.compute_link_path_callback());
    window.on_node_drag_started(ctrl.node_drag_started_callback());

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
                ctrl.set_viewport(z, pan_x, pan_y);
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
            }
        }
    });

    window.on_node_drag_ended({
        let ctrl = ctrl.clone();
        let nodes = nodes.clone();
        move |delta_x, delta_y| {
            let node_id = ctrl.dragged_node_id();
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

    window.invoke_request_grid_update();
    window.run().unwrap();
}
