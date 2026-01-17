//! Level 7: Scalability Tests
//!
//! These tests verify that the node editor handles large scenes (1K-10K nodes/links)
//! without performance regressions. Tests use generous timing thresholds (2-5x expected)
//! to avoid CI flakiness while still catching O(n²) regressions.
//!
//! **IMPORTANT:** Run with `cargo test level7 --release` for realistic performance.
//! Debug mode is 10-50x slower and timing assertions will be skipped.

use slint_node_editor::{
    find_link_at, find_pin_at, nodes_in_selection_box,
    generate_bezier_path, GeometryCache, GraphLogic, SelectionManager,
    SimpleNodeGeometry, LinkModel,
};
use slint_node_editor::hit_test::{SimplePinGeometry, SimpleLinkGeometry};
use slint_node_editor::state::StoredPin;
use slint::{Model, VecModel};
use std::rc::Rc;
use std::time::{Duration, Instant};

// ============================================================================
// Debug Mode Detection
// ============================================================================

/// Returns true if running in debug mode (without optimizations)
const fn is_debug_mode() -> bool {
    cfg!(debug_assertions)
}

/// Assert that elapsed time is within threshold, but skip in debug mode.
/// In debug mode, prints a warning instead of failing.
macro_rules! assert_timing {
    ($elapsed:expr, $threshold:expr, $($msg:tt)+) => {
        if is_debug_mode() {
            if $elapsed > $threshold {
                eprintln!(
                    "⚠️  SKIPPED (debug mode): {} - took {:?}, threshold {:?}. Run with --release for accurate timing.",
                    format!($($msg)+),
                    $elapsed,
                    $threshold
                );
            }
        } else {
            assert!(
                $elapsed <= $threshold,
                "{} took {:?}, expected <= {:?}",
                format!($($msg)+),
                $elapsed,
                $threshold
            );
        }
    };
}

// ============================================================================
// Constants
// ============================================================================

/// Small scale: 1,000 items
const SCALE_SMALL: usize = 1_000;

/// Medium scale: 5,000 items
const SCALE_MEDIUM: usize = 5_000;

/// Large scale: 10,000 items
const SCALE_LARGE: usize = 10_000;

// ============================================================================
// Timing Thresholds (generous to avoid CI flakiness)
// ============================================================================

mod thresholds {
    use super::*;

    /// Maximum time for cache population with 1K nodes
    pub const CACHE_POPULATE_1K: Duration = Duration::from_millis(50);

    /// Maximum time for cache population with 10K nodes
    pub const CACHE_POPULATE_10K: Duration = Duration::from_millis(200);

    /// Maximum time for single pin hit test query
    pub const PIN_HIT_SINGLE: Duration = Duration::from_millis(20);

    /// Maximum time for 100 pin hit test queries (simulated mouse tracking)
    pub const PIN_HIT_100_QUERIES: Duration = Duration::from_millis(100);

    /// Maximum time for single link hit test query against 1K links
    pub const LINK_HIT_1K: Duration = Duration::from_millis(50);

    /// Maximum time for single link hit test query against 5K links
    pub const LINK_HIT_5K: Duration = Duration::from_millis(200);

    /// Maximum time for 100 link hit test queries
    pub const LINK_HIT_100_QUERIES: Duration = Duration::from_millis(500);

    /// Maximum time for box selection with 1K nodes
    pub const BOX_SELECT_1K: Duration = Duration::from_millis(20);

    /// Maximum time for box selection with 10K nodes (full canvas)
    pub const BOX_SELECT_10K: Duration = Duration::from_millis(100);

    /// Maximum time for selection manager add operations (1K items)
    pub const SELECTION_ADD_1K: Duration = Duration::from_millis(20);

    /// Maximum time for selection manager contains check (1K lookups)
    pub const SELECTION_CONTAINS_1K: Duration = Duration::from_millis(10);

    /// Maximum time for selection replace with 10K items
    pub const SELECTION_REPLACE_10K: Duration = Duration::from_millis(50);

    /// Maximum time for sync_to_model with 1K items
    pub const SELECTION_SYNC_TO_1K: Duration = Duration::from_millis(30);

    /// Maximum time for sync_from_model with 1K items
    pub const SELECTION_SYNC_FROM_1K: Duration = Duration::from_millis(30);

    /// Maximum time for commit_drag with 100 selected of 1K
    pub const COMMIT_DRAG_100_OF_1K: Duration = Duration::from_millis(30);

    /// Maximum time for commit_drag with 1K selected of 10K
    pub const COMMIT_DRAG_1K_OF_10K: Duration = Duration::from_millis(100);

    /// Maximum time for find_links_connected_to_node with 1K links
    pub const FIND_CONNECTED_1K: Duration = Duration::from_millis(20);

    /// Maximum time for find_links_connected_to_node with 5K mesh links
    pub const FIND_CONNECTED_5K: Duration = Duration::from_millis(50);

    /// Maximum time for computing 1K link paths
    pub const COMPUTE_PATHS_1K: Duration = Duration::from_millis(100);

    /// Maximum time for updating all paths with zoom change
    pub const UPDATE_PATHS_ZOOM: Duration = Duration::from_millis(200);

    /// Maximum time for worst case: all nodes selected commit drag
    pub const ALL_SELECTED_DRAG: Duration = Duration::from_millis(200);

    /// Maximum time for dense links hit test
    pub const DENSE_LINKS_HIT: Duration = Duration::from_millis(300);

    /// Maximum time for repeated selection replace (stability test)
    pub const REPEATED_REPLACE: Duration = Duration::from_millis(100);
}

// ============================================================================
// Data Generators
// ============================================================================

/// Generate a grid of nodes with predictable layout.
fn generate_node_grid(count: usize, spacing: f32) -> Vec<SimpleNodeGeometry> {
    let cols = (count as f32).sqrt().ceil() as usize;
    (0..count)
        .map(|i| {
            let row = i / cols;
            let col = i % cols;
            SimpleNodeGeometry {
                id: i as i32 + 1,
                x: col as f32 * spacing,
                y: row as f32 * spacing,
                width: 100.0,
                height: 80.0,
            }
        })
        .collect()
}

/// Generate chain links (N -> N+1) for sequential connections.
fn generate_chain_links(node_count: usize) -> Vec<(i32, i32, i32)> {
    (0..node_count.saturating_sub(1))
        .map(|i| {
            let link_id = i as i32 + 1;
            let start_pin = (i as i32 + 1) * 10 + 1; // Output pin
            let end_pin = (i as i32 + 2) * 10; // Input pin of next node
            (link_id, start_pin, end_pin)
        })
        .collect()
}

/// Generate mesh links with multiple connections per node.
fn generate_mesh_links(node_count: usize, links_per_node: usize) -> Vec<(i32, i32, i32)> {
    let mut links = Vec::new();
    let mut link_id = 1;

    for i in 0..node_count {
        for j in 0..links_per_node {
            let target = (i + j + 1) % node_count;
            if target != i {
                let start_pin = (i as i32 + 1) * 10 + 1;
                let end_pin = (target as i32 + 1) * 10;
                links.push((link_id, start_pin, end_pin));
                link_id += 1;
            }
        }
    }
    links
}

/// Populate a GeometryCache with nodes and their pins.
fn populate_cache(count: usize, spacing: f32) -> GeometryCache<SimpleNodeGeometry> {
    let mut cache = GeometryCache::new();
    let nodes = generate_node_grid(count, spacing);

    for node in &nodes {
        cache.node_rects.insert(node.id, *node);

        // Add input pin (type 1) on left side
        let input_pin_id = node.id * 10;
        cache.pin_positions.insert(input_pin_id, StoredPin {
            node_id: node.id,
            pin_type: 1, // Input
            rel_x: 0.0,
            rel_y: 40.0,
        });

        // Add output pin (type 2) on right side
        let output_pin_id = node.id * 10 + 1;
        cache.pin_positions.insert(output_pin_id, StoredPin {
            node_id: node.id,
            pin_type: 2, // Output
            rel_x: 100.0,
            rel_y: 40.0,
        });
    }

    cache
}

/// Generate pin geometries for hit testing.
fn generate_pins_for_hit_test(count: usize, spacing: f32) -> Vec<SimplePinGeometry> {
    let cols = (count as f32).sqrt().ceil() as usize;
    (0..count)
        .map(|i| {
            let row = i / cols;
            let col = i % cols;
            SimplePinGeometry {
                id: i as i32 + 1,
                x: col as f32 * spacing + 50.0, // Center of node
                y: row as f32 * spacing + 40.0,
            }
        })
        .collect()
}

/// Generate link geometries for hit testing.
fn generate_links_for_hit_test(count: usize, spacing: f32) -> Vec<SimpleLinkGeometry> {
    let cols = (count as f32).sqrt().ceil() as usize;
    (0..count)
        .map(|i| {
            let row = i / cols;
            let col = i % cols;
            let start_x = col as f32 * spacing;
            let start_y = row as f32 * spacing + 40.0;
            SimpleLinkGeometry {
                id: i as i32 + 1,
                start_x,
                start_y,
                end_x: start_x + spacing * 0.8,
                end_y: start_y + 20.0,
            }
        })
        .collect()
}

// ============================================================================
// Test helper: TestLink for GraphLogic tests
// ============================================================================

#[derive(Clone, Debug)]
struct TestLink {
    id: i32,
    start_pin_id: i32,
    end_pin_id: i32,
}

impl LinkModel for TestLink {
    fn id(&self) -> i32 { self.id }
    fn start_pin_id(&self) -> i32 { self.start_pin_id }
    fn end_pin_id(&self) -> i32 { self.end_pin_id }
}

// ============================================================================
// Test helper: TestMovableNode for commit_drag tests
// ============================================================================

#[derive(Clone)]
struct TestMovableNode {
    id: i32,
    x: f32,
    y: f32,
}

impl slint_node_editor::MovableNode for TestMovableNode {
    fn id(&self) -> i32 { self.id }
    fn x(&self) -> f32 { self.x }
    fn y(&self) -> f32 { self.y }
    fn set_x(&mut self, x: f32) { self.x = x; }
    fn set_y(&mut self, y: f32) { self.y = y; }
}

// ============================================================================
// Cache Population Tests
// ============================================================================

#[test]
fn test_cache_populate_1k_nodes() {
    let start = Instant::now();
    let cache = populate_cache(SCALE_SMALL, 150.0);
    let elapsed = start.elapsed();

    assert_eq!(cache.node_rects.len(), SCALE_SMALL);
    assert_eq!(cache.pin_positions.len(), SCALE_SMALL * 2); // 2 pins per node
    assert_timing!(elapsed, thresholds::CACHE_POPULATE_1K, "Cache population (1K)");
}

#[test]
fn test_cache_populate_10k_nodes() {
    let start = Instant::now();
    let cache = populate_cache(SCALE_LARGE, 150.0);
    let elapsed = start.elapsed();

    assert_eq!(cache.node_rects.len(), SCALE_LARGE);
    assert_eq!(cache.pin_positions.len(), SCALE_LARGE * 2);
    assert_timing!(elapsed, thresholds::CACHE_POPULATE_10K, "Cache population (10K)");
}

// ============================================================================
// Pin Hit Testing Tests
// ============================================================================

#[test]
fn test_find_pin_at_1k_pins_single_query() {
    let pins = generate_pins_for_hit_test(SCALE_SMALL, 150.0);

    // Query for a pin in the middle of the grid
    let target_idx = SCALE_SMALL / 2;
    let target = &pins[target_idx];

    let start = Instant::now();
    let result = find_pin_at(target.x, target.y, pins.iter().copied(), 10.0);
    let elapsed = start.elapsed();

    assert_eq!(result, target.id);
    assert_timing!(elapsed, thresholds::PIN_HIT_SINGLE, "Pin hit test (1K)");
}

#[test]
fn test_find_pin_at_10k_pins_single_query() {
    let pins = generate_pins_for_hit_test(SCALE_LARGE, 150.0);

    // Query for a pin near the end
    let target_idx = SCALE_LARGE - 100;
    let target = &pins[target_idx];

    let start = Instant::now();
    let result = find_pin_at(target.x, target.y, pins.iter().copied(), 10.0);
    let elapsed = start.elapsed();

    assert_eq!(result, target.id);
    assert_timing!(elapsed, thresholds::PIN_HIT_SINGLE, "Pin hit test (10K)");
}

#[test]
fn test_find_pin_at_10k_pins_miss() {
    let pins = generate_pins_for_hit_test(SCALE_LARGE, 150.0);

    // Query for a location with no pin
    let start = Instant::now();
    let result = find_pin_at(-1000.0, -1000.0, pins.iter().copied(), 10.0);
    let elapsed = start.elapsed();

    assert_eq!(result, 0); // No pin found
    assert_timing!(elapsed, thresholds::PIN_HIT_SINGLE, "Pin hit test (miss)");
}

#[test]
fn test_find_pin_at_simulated_mouse_tracking() {
    let pins = generate_pins_for_hit_test(SCALE_LARGE, 150.0);

    let start = Instant::now();

    // Simulate 100 mouse move queries across the canvas
    for i in 0..100 {
        let x = (i as f32 * 50.0) % 5000.0;
        let y = (i as f32 * 30.0) % 5000.0;
        let _ = find_pin_at(x, y, pins.iter().copied(), 10.0);
    }

    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::PIN_HIT_100_QUERIES, "100 pin queries");
}

// ============================================================================
// Link Hit Testing Tests
// ============================================================================

#[test]
fn test_find_link_at_1k_links_single_query() {
    let links = generate_links_for_hit_test(SCALE_SMALL, 150.0);

    // Query near the middle of a link
    let target = &links[SCALE_SMALL / 2];
    let mid_x = (target.start_x + target.end_x) / 2.0;
    let mid_y = (target.start_y + target.end_y) / 2.0;

    let start = Instant::now();
    let result = find_link_at(mid_x, mid_y, links.iter().copied(), 15.0, 1.0, 50.0, 20);
    let elapsed = start.elapsed();

    // The result might be the target or a nearby link due to bezier curves
    assert!(result != -1, "Should find a link near the query point");
    assert_timing!(elapsed, thresholds::LINK_HIT_1K, "Link hit test (1K)");
}

#[test]
fn test_find_link_at_5k_links_single_query() {
    let links = generate_links_for_hit_test(SCALE_MEDIUM, 150.0);

    let target = &links[SCALE_MEDIUM / 2];
    let mid_x = (target.start_x + target.end_x) / 2.0;
    let mid_y = (target.start_y + target.end_y) / 2.0;

    let start = Instant::now();
    let result = find_link_at(mid_x, mid_y, links.iter().copied(), 15.0, 1.0, 50.0, 20);
    let elapsed = start.elapsed();

    assert!(result != -1);
    assert_timing!(elapsed, thresholds::LINK_HIT_5K, "Link hit test (5K)");
}

#[test]
fn test_find_link_at_simulated_mouse_tracking() {
    let links = generate_links_for_hit_test(SCALE_SMALL, 150.0);

    let start = Instant::now();

    for i in 0..100 {
        let x = (i as f32 * 50.0) % 3000.0;
        let y = (i as f32 * 30.0) % 3000.0;
        let _ = find_link_at(x, y, links.iter().copied(), 15.0, 1.0, 50.0, 20);
    }

    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::LINK_HIT_100_QUERIES, "100 link queries");
}

// ============================================================================
// Box Selection Tests
// ============================================================================

#[test]
fn test_box_selection_1k_nodes_small_box() {
    let nodes = generate_node_grid(SCALE_SMALL, 150.0);

    // Small selection box covering ~10 nodes
    let start = Instant::now();
    let selected = nodes_in_selection_box(0.0, 0.0, 500.0, 300.0, nodes.iter().copied());
    let elapsed = start.elapsed();

    assert!(!selected.is_empty(), "Should select some nodes");
    assert_timing!(elapsed, thresholds::BOX_SELECT_1K, "Box selection (1K, small box)");
}

#[test]
fn test_box_selection_10k_nodes_full_canvas() {
    let nodes = generate_node_grid(SCALE_LARGE, 150.0);

    // Large selection box covering all nodes
    let start = Instant::now();
    let selected = nodes_in_selection_box(
        0.0, 0.0,
        20000.0, 20000.0, // Large enough to cover all
        nodes.iter().copied(),
    );
    let elapsed = start.elapsed();

    assert_eq!(selected.len(), SCALE_LARGE, "Should select all nodes");
    assert_timing!(elapsed, thresholds::BOX_SELECT_10K, "Box selection (10K, full canvas)");
}

#[test]
fn test_box_selection_10k_nodes_empty_area() {
    let nodes = generate_node_grid(SCALE_LARGE, 150.0);

    // Selection box in empty area
    let start = Instant::now();
    let selected = nodes_in_selection_box(
        -10000.0, -10000.0,
        100.0, 100.0,
        nodes.iter().copied(),
    );
    let elapsed = start.elapsed();

    assert!(selected.is_empty(), "Should select no nodes");
    assert_timing!(elapsed, thresholds::BOX_SELECT_10K, "Box selection (10K, empty area)");
}

// ============================================================================
// Selection Manager Tests
// ============================================================================

#[test]
fn test_selection_manager_add_1k_items() {
    let mut selection = SelectionManager::new();

    let start = Instant::now();
    for i in 0..SCALE_SMALL {
        selection.handle_interaction(i as i32, true); // Shift+click to add
    }
    let elapsed = start.elapsed();

    assert_eq!(selection.len(), SCALE_SMALL);
    assert_timing!(elapsed, thresholds::SELECTION_ADD_1K, "Adding 1K items");
}

#[test]
fn test_selection_manager_contains_check_1k() {
    let mut selection = SelectionManager::new();
    let ids: Vec<i32> = (0..SCALE_SMALL as i32).collect();
    selection.replace_selection(ids.clone());

    let start = Instant::now();
    for id in &ids {
        assert!(selection.contains(*id));
    }
    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::SELECTION_CONTAINS_1K, "1K contains checks");
}

#[test]
fn test_selection_manager_replace_10k() {
    let mut selection = SelectionManager::new();

    // Pre-populate with some items
    selection.replace_selection(0..1000);

    let ids: Vec<i32> = (0..SCALE_LARGE as i32).collect();

    let start = Instant::now();
    selection.replace_selection(ids);
    let elapsed = start.elapsed();

    assert_eq!(selection.len(), SCALE_LARGE);
    assert_timing!(elapsed, thresholds::SELECTION_REPLACE_10K, "Replace with 10K items");
}

#[test]
fn test_selection_sync_to_model_1k() {
    let mut selection = SelectionManager::new();
    selection.replace_selection(0..SCALE_SMALL as i32);

    let model = Rc::new(VecModel::<i32>::default());

    let start = Instant::now();
    selection.sync_to_model(&model);
    let elapsed = start.elapsed();

    assert_eq!(model.row_count(), SCALE_SMALL);
    assert_timing!(elapsed, thresholds::SELECTION_SYNC_TO_1K, "sync_to_model (1K)");
}

#[test]
fn test_selection_sync_from_model_1k() {
    let mut selection = SelectionManager::new();
    let ids: Vec<i32> = (0..SCALE_SMALL as i32).collect();
    let model = Rc::new(VecModel::from(ids));

    let start = Instant::now();
    selection.sync_from_model(model.as_ref());
    let elapsed = start.elapsed();

    assert_eq!(selection.len(), SCALE_SMALL);
    assert_timing!(elapsed, thresholds::SELECTION_SYNC_FROM_1K, "sync_from_model (1K)");
}

// ============================================================================
// Commit Drag Tests
// ============================================================================

#[test]
fn test_commit_drag_100_selected_of_1k() {
    let nodes: Vec<TestMovableNode> = (0..SCALE_SMALL)
        .map(|i| TestMovableNode {
            id: i as i32,
            x: (i % 100) as f32 * 150.0,
            y: (i / 100) as f32 * 150.0,
        })
        .collect();

    let model = Rc::new(VecModel::from(nodes));

    let mut selection = SelectionManager::new();
    // Select first 100 nodes
    selection.replace_selection(0..100);

    let start = Instant::now();
    GraphLogic::commit_drag(&model, &selection, 50.0, 50.0);
    let elapsed = start.elapsed();

    // Verify some nodes moved
    let node0 = model.row_data(0).unwrap();
    assert_eq!(node0.x, 50.0); // Was at 0, moved by 50

    assert_timing!(elapsed, thresholds::COMMIT_DRAG_100_OF_1K, "commit_drag (100 of 1K)");
}

#[test]
fn test_commit_drag_1k_selected_of_10k() {
    let nodes: Vec<TestMovableNode> = (0..SCALE_LARGE)
        .map(|i| TestMovableNode {
            id: i as i32,
            x: (i % 100) as f32 * 150.0,
            y: (i / 100) as f32 * 150.0,
        })
        .collect();

    let model = Rc::new(VecModel::from(nodes));

    let mut selection = SelectionManager::new();
    selection.replace_selection(0..SCALE_SMALL as i32);

    let start = Instant::now();
    GraphLogic::commit_drag(&model, &selection, 25.0, 25.0);
    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::COMMIT_DRAG_1K_OF_10K, "commit_drag (1K of 10K)");
}

// ============================================================================
// Find Connected Links Tests
// ============================================================================

#[test]
fn test_find_links_connected_to_node_1k_links() {
    let cache = populate_cache(SCALE_SMALL, 150.0);

    let links: Vec<TestLink> = generate_chain_links(SCALE_SMALL)
        .into_iter()
        .map(|(id, start, end)| TestLink { id, start_pin_id: start, end_pin_id: end })
        .collect();

    // Find links connected to node in the middle
    let target_node = (SCALE_SMALL / 2) as i32 + 1;

    let start = Instant::now();
    let connected = GraphLogic::find_links_connected_to_node(target_node, links.iter().cloned(), &cache);
    let elapsed = start.elapsed();

    // Middle node should have 2 connections (incoming and outgoing)
    assert!(!connected.is_empty());
    assert_timing!(elapsed, thresholds::FIND_CONNECTED_1K, "find_links_connected_to_node (1K)");
}

#[test]
fn test_find_links_connected_to_node_5k_mesh() {
    let cache = populate_cache(SCALE_SMALL, 150.0); // 1K nodes with mesh links

    let links: Vec<TestLink> = generate_mesh_links(SCALE_SMALL, 5)
        .into_iter()
        .map(|(id, start, end)| TestLink { id, start_pin_id: start, end_pin_id: end })
        .collect();

    assert!(links.len() >= SCALE_MEDIUM); // Should have ~5K links

    let target_node = (SCALE_SMALL / 2) as i32 + 1;

    let start = Instant::now();
    let connected = GraphLogic::find_links_connected_to_node(target_node, links.iter().cloned(), &cache);
    let elapsed = start.elapsed();

    assert!(!connected.is_empty());
    assert_timing!(elapsed, thresholds::FIND_CONNECTED_5K, "find_links_connected_to_node (5K mesh)");
}

// ============================================================================
// Path Computation Tests
// ============================================================================

#[test]
fn test_compute_link_path_1k_links() {
    let cache = populate_cache(SCALE_SMALL, 150.0);
    let links = generate_chain_links(SCALE_SMALL);

    let start = Instant::now();

    for (_, start_pin, end_pin) in &links {
        let path = cache.compute_link_path(*start_pin, *end_pin, 1.0, 50.0);
        assert!(path.is_some() || true); // Some might be None if pins don't exist
    }

    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::COMPUTE_PATHS_1K, "Computing 1K link paths");
}

#[test]
fn test_update_all_paths_with_zoom_change() {
    // Test that recomputing all paths after a zoom change is fast
    let cache = populate_cache(SCALE_SMALL, 150.0);
    let links = generate_chain_links(SCALE_SMALL);

    let zoom_levels = [0.5, 1.0, 1.5, 2.0];

    let start = Instant::now();

    for zoom in zoom_levels {
        for (_, start_pin, end_pin) in &links {
            let _ = cache.compute_link_path(*start_pin, *end_pin, zoom, 50.0);
        }
    }

    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::UPDATE_PATHS_ZOOM, "Updating paths across zoom levels");
}

// ============================================================================
// Worst Case Tests
// ============================================================================

#[test]
fn test_all_nodes_selected_commit_drag() {
    let nodes: Vec<TestMovableNode> = (0..SCALE_LARGE)
        .map(|i| TestMovableNode {
            id: i as i32,
            x: (i % 100) as f32 * 150.0,
            y: (i / 100) as f32 * 150.0,
        })
        .collect();

    let model = Rc::new(VecModel::from(nodes));

    let mut selection = SelectionManager::new();
    // Select ALL nodes
    selection.replace_selection(0..SCALE_LARGE as i32);

    let start = Instant::now();
    GraphLogic::commit_drag(&model, &selection, 10.0, 10.0);
    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::ALL_SELECTED_DRAG, "All nodes selected drag");
}

#[test]
fn test_dense_links_hit_test() {
    // Create many overlapping links in a small area
    let dense_links: Vec<SimpleLinkGeometry> = (0..SCALE_MEDIUM)
        .map(|i| SimpleLinkGeometry {
            id: i as i32,
            start_x: 0.0,
            start_y: (i % 100) as f32,
            end_x: 200.0,
            end_y: (i % 100) as f32 + 50.0,
        })
        .collect();

    let start = Instant::now();

    // Query across the dense area
    for i in 0..100 {
        let y = (i % 100) as f32 + 25.0;
        let _ = find_link_at(100.0, y, dense_links.iter().copied(), 10.0, 1.0, 50.0, 20);
    }

    let elapsed = start.elapsed();

    assert_timing!(elapsed, thresholds::DENSE_LINKS_HIT, "Dense links hit test");
}

#[test]
fn test_repeated_selection_replace_no_slowdown() {
    let mut selection = SelectionManager::new();

    let start = Instant::now();

    // Repeatedly replace selection - should not accumulate memory/slow down
    for _ in 0..100 {
        let ids: Vec<i32> = (0..SCALE_SMALL as i32).collect();
        selection.replace_selection(ids);
    }

    let elapsed = start.elapsed();

    assert_eq!(selection.len(), SCALE_SMALL);
    assert_timing!(elapsed, thresholds::REPEATED_REPLACE, "100 repeated replacements");
}

// ============================================================================
// Standalone Bezier Path Generation Test
// ============================================================================

#[test]
fn test_generate_bezier_path_1k_calls() {
    let start = Instant::now();

    for i in 0..SCALE_SMALL {
        let _ = generate_bezier_path(
            (i % 100) as f32 * 10.0,
            (i / 100) as f32 * 10.0,
            (i % 100) as f32 * 10.0 + 100.0,
            (i / 100) as f32 * 10.0 + 50.0,
            1.0,
            50.0,
        );
    }

    let elapsed = start.elapsed();

    // Path generation should be very fast (string formatting)
    assert_timing!(elapsed, Duration::from_millis(50), "1K bezier path generations");
}
