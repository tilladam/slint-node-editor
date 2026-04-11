//! Stress test: 2500 nodes (50x50 grid) with ~4900 edges.
//! Run with: cargo run -p sugiyama-stress-test

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{sugiyama_layout, wire_node_editor, Direction, NodeEditorSetup, SugiyamaConfig};

slint::include_modules!();

fn random_f32() -> f32 {
    thread_local! { static SEED: Cell<u64> = Cell::new(12345); }
    SEED.with(|s| {
        let v = s.get().wrapping_mul(6364136223846793005).wrapping_add(1);
        s.set(v);
        ((v >> 33) as f32) / (u32::MAX as f32)
    })
}

fn build_node_index(nodes: &VecModel<NodeData>) -> HashMap<i32, usize> {
    (0..nodes.row_count())
        .filter_map(|i| nodes.row_data(i).map(|n| (n.id, i)))
        .collect()
}

fn main() {
    let window = MainWindow::new().unwrap();
    let w = window.as_weak();

    // 50x50 grid of nodes (2500 nodes total)
    const GRID_SIZE: i32 = 50;
    let mut node_vec = Vec::with_capacity((GRID_SIZE * GRID_SIZE) as usize);
    for row in 0..GRID_SIZE {
        for col in 0..GRID_SIZE {
            let id = row * GRID_SIZE + col + 1;
            node_vec.push(NodeData {
                id,
                title: SharedString::from(format!("{},{}", row, col)),
                x: (col * 140) as f32 + 50.0,
                y: (row * 80) as f32 + 50.0,
            });
        }
    }
    let nodes = Rc::new(VecModel::from(node_vec));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Each node connects to its right and bottom neighbors
    let mut dag_edges: Vec<(i32, i32)> = Vec::new();
    for row in 0..GRID_SIZE {
        for col in 0..GRID_SIZE {
            let id = row * GRID_SIZE + col + 1;
            if col < GRID_SIZE - 1 {
                dag_edges.push((id, id + 1));
            }
            if row < GRID_SIZE - 1 {
                dag_edges.push((id, id + GRID_SIZE));
            }
        }
    }

    let link_color = Color::from_argb_u8(255, 100, 180, 255);
    let link_data: Vec<LinkData> = dag_edges
        .iter()
        .enumerate()
        .map(|(i, &(src, dst))| LinkData {
            id: (i + 1) as i32,
            start_pin_id: src * 2 + 1,
            end_pin_id: dst * 2,
            color: link_color,
            line_width: 2.0,
        })
        .collect();
    window.set_links(ModelRc::from(Rc::new(VecModel::from(link_data))));

    window.on_layout_requested({
        let nodes = nodes.clone();
        let dag_edges = dag_edges.clone();
        let w = w.clone();
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

            if let Some(w) = w.upgrade() {
                let geom_ver = w.global::<GeometryVersion>();
                geom_ver.set_version(geom_ver.get_version() + 1);
            }
        }
    });

    window.on_scramble_requested({
        let nodes = nodes.clone();
        let w = w.clone();
        move || {
            for i in 0..nodes.row_count() {
                if let Some(mut node) = nodes.row_data(i) {
                    node.x = 50.0 + random_f32() * 800.0;
                    node.y = 50.0 + random_f32() * 500.0;
                    nodes.set_row_data(i, node);
                }
            }

            if let Some(w) = w.upgrade() {
                let geom_ver = w.global::<GeometryVersion>();
                geom_ver.set_version(geom_ver.get_version() + 1);
            }
        }
    });

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
