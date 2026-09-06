//! Sugiyama hierarchical graph layout.
//!
//! This module provides functions for computing hierarchical (layered) layouts
//! of directed graphs using the Sugiyama algorithm via the `rust-sugiyama` crate.
//!
//! The layout API uses `f64` coordinates because the underlying `rust-sugiyama`
//! crate operates in `f64`. The rest of this crate uses `f32` (matching Slint),
//! so callers should convert with `as f32` when applying positions back to nodes.
//!
//! Requires the `layout` feature to be enabled.

use std::collections::{BTreeMap, BTreeSet};

use crate::hit_test::NodeGeometry;
use crate::state::GeometryCache;

/// Layout direction for the Sugiyama algorithm.
///
/// Marked `#[non_exhaustive]` so additional directions (e.g. `RightToLeft`,
/// `BottomToTop`) can be added in future versions without breaking callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Direction {
    /// Layers flow top to bottom (default).
    #[default]
    TopToBottom,
    /// Layers flow left to right.
    LeftToRight,
}

/// A positioned node returned by [`sugiyama_layout`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodePosition {
    /// The node ID (same value passed in via `node_sizes`).
    pub id: i32,
    /// X coordinate of the node's top-left corner.
    pub x: f64,
    /// Y coordinate of the node's top-left corner.
    pub y: f64,
}

/// Configuration for the Sugiyama layout algorithm.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct SugiyamaConfig {
    /// Minimum spacing between vertices (default: 0.0, which uses the
    /// `rust-sugiyama` default of 10.0).
    pub vertex_spacing: f64,
    /// Minimum edge length between layers (default: 0, which uses the
    /// `rust-sugiyama` default of 1).
    pub minimum_length: u32,
    /// Whether to include dummy vertices in the layout (default: false).
    pub dummy_vertices: bool,
    /// Layout direction (default: [`Direction::TopToBottom`]).
    pub direction: Direction,
}

/// Compute Sugiyama hierarchical layout positions.
///
/// Takes edges as `(source_node_id, target_node_id)` pairs and node sizes as
/// `(node_id, (width, height))` pairs. Returns a [`NodePosition`] for each node.
///
/// Node IDs can be any `i32` values — they are mapped to sequential `u32` indices
/// internally for `rust-sugiyama` and translated back before returning.
///
/// Duplicate node IDs in `node_sizes` are ignored (first occurrence wins),
/// then nodes are ordered by ID. Nodes whose width or height is non-finite or
/// not positive are omitted. Edges with an omitted or unknown endpoint,
/// self-loops, and duplicate edges are ignored; retained edges are ordered by
/// their node-ID pair. Results are returned in ascending node-ID order.
pub fn sugiyama_layout(
    edges: &[(i32, i32)],
    node_sizes: &[(i32, (f64, f64))],
    config: &SugiyamaConfig,
) -> Vec<NodePosition> {
    let horizontal = config.direction == Direction::LeftToRight;

    // Preserve the documented first occurrence, then let BTreeMap provide the
    // canonical node-ID order used by every downstream table.
    let mut nodes_by_id = BTreeMap::new();
    for &(node_id, size) in node_sizes {
        nodes_by_id.entry(node_id).or_insert(size);
    }
    nodes_by_id.retain(|_, (width, height)| {
        width.is_finite() && *width > 0.0 && height.is_finite() && *height > 0.0
    });
    let nodes: Vec<(i32, (f64, f64))> = nodes_by_id.into_iter().collect();
    if nodes.is_empty() {
        return Vec::new();
    }

    let id_to_idx: BTreeMap<i32, u32> = nodes
        .iter()
        .enumerate()
        .map(|(idx, &(node_id, _))| (node_id, idx as u32))
        .collect();

    // For horizontal layout, swap width/height so the algorithm spaces layers
    // along what will become the x-axis.
    let vertices: Vec<(u32, (f64, f64))> = nodes
        .iter()
        .enumerate()
        .map(|(idx, &(_, (width, height)))| {
            let size = if horizontal {
                (height, width)
            } else {
                (width, height)
            };
            (idx as u32, size)
        })
        .collect();

    let mapped_edges: Vec<(u32, u32)> = edges
        .iter()
        .filter(|(src, dst)| src != dst)
        .filter_map(|&(src, dst)| Some((*id_to_idx.get(&src)?, *id_to_idx.get(&dst)?)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // Configure rust-sugiyama
    let mut sg_config = rust_sugiyama::configure::Config {
        dummy_vertices: config.dummy_vertices,
        ..Default::default()
    };
    if config.vertex_spacing > 0.0 {
        sg_config.vertex_spacing = config.vertex_spacing;
    }
    if config.minimum_length > 0 {
        sg_config.minimum_length = config.minimum_length;
    }

    // Run layout — returns Vec<(Vec<(usize, (f64, f64))>, f64, f64)> (subgraphs)
    let mut subgraphs =
        rust_sugiyama::from_vertices_and_edges(&vertices, &mapped_edges, &sg_config);
    subgraphs.sort_by_key(|(layout, _, _)| {
        layout
            .iter()
            .filter_map(|(idx, _)| nodes.get(*idx).map(|(node_id, _)| *node_id))
            .min()
            .unwrap_or(i32::MAX)
    });

    // Collect results from all subgraphs, translating indices back to node IDs.
    // For horizontal layout, swap x/y so layers run left-to-right.
    // Each subgraph is offset along the perpendicular axis so disconnected
    // components (and isolated nodes) don't overlap.
    let spacing = if config.vertex_spacing > 0.0 {
        config.vertex_spacing
    } else {
        10.0
    };
    let mut results = Vec::with_capacity(nodes.len());
    let mut perpendicular_offset = 0.0_f64;

    for (layout, _width, _height) in &subgraphs {
        // Compute bounding box of this subgraph in output coordinates
        let mut min_perp = f64::INFINITY;
        let mut max_perp = f64::NEG_INFINITY;

        let start = results.len();
        for &(idx, (x, y)) in layout {
            if let Some(&(node_id, (node_width, node_height))) = nodes.get(idx) {
                let (px, py) = if horizontal { (y, x) } else { (x, y) };
                results.push(NodePosition { id: node_id, x: px, y: py });

                let (perpendicular_position, perpendicular_size) = if horizontal {
                    (py, node_height)
                } else {
                    (px, node_width)
                };
                min_perp = min_perp.min(perpendicular_position);
                max_perp = max_perp.max(perpendicular_position + perpendicular_size);
            }
        }

        if start < results.len() {
            // Shift this subgraph so its perpendicular leading edge sits at
            // the running offset.
            let shift = perpendicular_offset - min_perp;
            for pos in &mut results[start..] {
                if horizontal {
                    pos.y += shift;
                } else {
                    pos.x += shift;
                }
            }
            perpendicular_offset += max_perp - min_perp + spacing;
        }
    }

    results.sort_by_key(|position| position.id);
    results
}

/// Compute Sugiyama layout using data from a [`GeometryCache`].
///
/// Edges are given as `(start_pin_id, end_pin_id)` pairs — the same format used
/// by link models. Pin IDs are resolved to node IDs via `cache.pin_positions`,
/// and node dimensions are read from `cache.node_rects`.
///
/// Unknown pins are ignored. Resolved edges then use the raw layout policy:
/// missing node endpoints, self-loops, and duplicate node pairs are ignored.
/// Cache `HashMap` iteration cannot affect the result because the raw layout
/// canonicalizes nodes and edges before invoking the algorithm.
///
/// Returns a [`NodePosition`] for each laid-out node.
pub fn sugiyama_layout_from_cache<N>(
    cache: &GeometryCache<N>,
    edges: &[(i32, i32)],
    config: &SugiyamaConfig,
) -> Vec<NodePosition>
where
    N: NodeGeometry + Copy,
{
    // Resolve known pin IDs to node IDs. The raw API owns canonical edge
    // filtering so both entry points have the same self-loop/duplicate policy.
    let node_edges: Vec<(i32, i32)> = edges
        .iter()
        .filter_map(|&(start_pin, end_pin)| {
            let src_node = cache.pin_positions.get(&start_pin)?.node_id;
            let dst_node = cache.pin_positions.get(&end_pin)?.node_id;
            Some((src_node, dst_node))
        })
        .collect();

    // Extract node sizes from cache
    let node_sizes: Vec<(i32, (f64, f64))> = cache
        .node_rects
        .iter()
        .map(|(&id, geom)| {
            let (_, _, w, h) = geom.rect();
            (id, (w as f64, h as f64))
        })
        .collect();

    sugiyama_layout(&node_edges, &node_sizes, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper to collect positions into a HashMap for easy lookup.
    fn pos_map(positions: Vec<NodePosition>) -> HashMap<i32, (f64, f64)> {
        positions.into_iter().map(|p| (p.id, (p.x, p.y))).collect()
    }

    #[test]
    fn test_empty_input() {
        let result = sugiyama_layout(&[], &[], &SugiyamaConfig::default());
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_node_no_edges() {
        let sizes = vec![(1, (100.0, 50.0))];
        let result = sugiyama_layout(&[], &sizes, &SugiyamaConfig::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn test_two_nodes_one_edge() {
        let sizes = vec![(10, (100.0, 50.0)), (20, (100.0, 50.0))];
        let edges = vec![(10, 20)];
        let result = sugiyama_layout(&edges, &sizes, &SugiyamaConfig::default());
        assert_eq!(result.len(), 2);

        let pos = pos_map(result);
        assert!(pos.contains_key(&10));
        assert!(pos.contains_key(&20));

        // Source should be above target in top-to-bottom layout
        assert!(pos[&10].1 < pos[&20].1, "source node should be in an earlier layer");
    }

    #[test]
    fn test_diamond_dag() {
        // Diamond: 1 -> 2, 1 -> 3, 2 -> 4, 3 -> 4
        let sizes = vec![
            (1, (80.0, 40.0)),
            (2, (80.0, 40.0)),
            (3, (80.0, 40.0)),
            (4, (80.0, 40.0)),
        ];
        let edges = vec![(1, 2), (1, 3), (2, 4), (3, 4)];
        let result = sugiyama_layout(&edges, &sizes, &SugiyamaConfig::default());
        assert_eq!(result.len(), 4);

        let pos = pos_map(result);

        // Node 1 (root) should be in the first layer, node 4 (sink) in the last
        assert!(pos[&1].1 < pos[&4].1);
        // Nodes 2 and 3 should be in the same middle layer
        assert!((pos[&2].1 - pos[&3].1).abs() < 1.0);
    }

    #[test]
    fn test_edges_with_unknown_nodes_are_skipped() {
        let sizes = vec![(1, (100.0, 50.0))];
        let edges = vec![(1, 999)]; // 999 not in sizes
        let result = sugiyama_layout(&edges, &sizes, &SugiyamaConfig::default());
        // Should still return node 1, just without the edge
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn test_duplicate_node_ids_first_wins() {
        let base_sizes = vec![(1, (100.0, 50.0)), (2, (60.0, 30.0))];
        let duplicate_sizes = vec![
            (1, (100.0, 50.0)),
            (1, (500.0, 400.0)),
            (2, (60.0, 30.0)),
        ];

        assert_eq!(
            sugiyama_layout(&[], &duplicate_sizes, &SugiyamaConfig::default()),
            sugiyama_layout(&[], &base_sizes, &SugiyamaConfig::default())
        );
    }

    #[test]
    fn invalid_node_dimensions_are_omitted() {
        let sizes = vec![
            (1, (100.0, 50.0)),
            (2, (0.0, 50.0)),
            (3, (100.0, -1.0)),
            (4, (f64::NAN, 50.0)),
            (5, (100.0, f64::INFINITY)),
            // First occurrence wins even when it is invalid.
            (2, (100.0, 50.0)),
        ];

        let result = sugiyama_layout(&[(1, 2)], &sizes, &SugiyamaConfig::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert!(result[0].x.is_finite());
        assert!(result[0].y.is_finite());
    }

    #[test]
    fn raw_layout_ignores_duplicate_self_and_unknown_edges() {
        let sizes = vec![
            (1, (80.0, 40.0)),
            (2, (80.0, 40.0)),
            (3, (80.0, 40.0)),
        ];
        let expected = sugiyama_layout(&[(1, 2)], &sizes, &SugiyamaConfig::default());
        let noisy = sugiyama_layout(
            &[(1, 2), (1, 2), (1, 1), (3, 3), (1, 999), (999, 2)],
            &sizes,
            &SugiyamaConfig::default(),
        );

        assert_eq!(noisy, expected);
    }

    #[test]
    fn equivalent_input_permutations_have_identical_positions_and_order() {
        let sizes = vec![
            (30, (70.0, 35.0)),
            (10, (80.0, 40.0)),
            (40, (90.0, 45.0)),
            (20, (60.0, 30.0)),
        ];
        let mut reversed_sizes = sizes.clone();
        reversed_sizes.reverse();
        let edges = vec![(10, 20), (10, 30), (20, 40), (30, 40)];
        let mut reversed_edges = edges.clone();
        reversed_edges.reverse();

        for direction in [Direction::TopToBottom, Direction::LeftToRight] {
            let config = SugiyamaConfig { direction, ..Default::default() };
            let expected = sugiyama_layout(&edges, &sizes, &config);
            let actual = sugiyama_layout(&reversed_edges, &reversed_sizes, &config);

            assert_eq!(actual, expected);
            assert_eq!(
                actual.iter().map(|position| position.id).collect::<Vec<_>>(),
                vec![10, 20, 30, 40]
            );
        }
    }

    #[test]
    fn test_config_default() {
        let config = SugiyamaConfig::default();
        assert_eq!(config.vertex_spacing, 0.0);
        assert_eq!(config.minimum_length, 0);
        assert!(!config.dummy_vertices);
        assert_eq!(config.direction, Direction::TopToBottom);
    }

    #[test]
    fn test_direction_default_is_top_to_bottom() {
        assert_eq!(Direction::default(), Direction::TopToBottom);
    }

    #[test]
    fn test_node_position_fields() {
        let sizes = vec![(42, (80.0, 40.0))];
        let result = sugiyama_layout(&[], &sizes, &SugiyamaConfig::default());
        assert_eq!(result.len(), 1);
        let pos = result[0];
        assert_eq!(pos.id, 42);
        // x and y should be finite numbers
        assert!(pos.x.is_finite());
        assert!(pos.y.is_finite());
    }

    #[test]
    fn test_left_to_right_swaps_axes() {
        let sizes = vec![(10, (100.0, 50.0)), (20, (100.0, 50.0))];
        let edges = vec![(10, 20)];

        let ttb = sugiyama_layout(&edges, &sizes, &SugiyamaConfig::default());
        let ltr = sugiyama_layout(&edges, &sizes, &SugiyamaConfig {
            direction: Direction::LeftToRight,
            ..Default::default()
        });

        let ttb_pos = pos_map(ttb);
        let ltr_pos = pos_map(ltr);

        // In top-to-bottom, layers differ in y; in left-to-right, layers differ in x
        let ttb_dy = (ttb_pos[&20].1 - ttb_pos[&10].1).abs();
        let ltr_dx = (ltr_pos[&20].0 - ltr_pos[&10].0).abs();

        assert!(ttb_dy > 1.0, "top-to-bottom should separate layers in y");
        assert!(ltr_dx > 1.0, "left-to-right should separate layers in x");
    }

    #[test]
    fn test_left_to_right_diamond() {
        let sizes = vec![
            (1, (80.0, 40.0)),
            (2, (80.0, 40.0)),
            (3, (80.0, 40.0)),
            (4, (80.0, 40.0)),
        ];
        let edges = vec![(1, 2), (1, 3), (2, 4), (3, 4)];
        let config = SugiyamaConfig {
            direction: Direction::LeftToRight,
            ..Default::default()
        };
        let result = sugiyama_layout(&edges, &sizes, &config);
        let pos = pos_map(result);

        // Root should be leftmost, sink rightmost
        assert!(pos[&1].0 < pos[&4].0);
        // Middle nodes should be at the same x (same layer)
        assert!((pos[&2].0 - pos[&3].0).abs() < 1.0);
    }

    #[test]
    fn test_disconnected_graph() {
        // Two separate subgraphs: 1->2 and 3->4
        let sizes = vec![
            (1, (80.0, 40.0)),
            (2, (80.0, 40.0)),
            (3, (80.0, 40.0)),
            (4, (80.0, 40.0)),
        ];
        let edges = vec![(1, 2), (3, 4)];
        let result = sugiyama_layout(&edges, &sizes, &SugiyamaConfig::default());

        assert_eq!(result.len(), 4);
        let pos = pos_map(result);
        assert!(pos.contains_key(&1));
        assert!(pos.contains_key(&2));
        assert!(pos.contains_key(&3));
        assert!(pos.contains_key(&4));
        // Each subgraph should respect layer ordering
        assert!(pos[&1].1 < pos[&2].1);
        assert!(pos[&3].1 < pos[&4].1);
    }

    #[test]
    fn test_isolated_nodes_are_packed_on_the_perpendicular_axis() {
        let sizes = vec![
            (1, (100.0, 50.0)),
            (2, (100.0, 50.0)),
            (3, (100.0, 50.0)),
        ];

        let top_to_bottom = sugiyama_layout(&[], &sizes, &SugiyamaConfig::default());
        assert!(top_to_bottom.windows(2).all(|pair| pair[0].x + 110.0 <= pair[1].x));
        assert!(top_to_bottom.windows(2).all(|pair| pair[0].y == pair[1].y));

        let left_to_right = sugiyama_layout(
            &[],
            &sizes,
            &SugiyamaConfig { direction: Direction::LeftToRight, ..Default::default() },
        );
        assert!(left_to_right.windows(2).all(|pair| pair[0].y + 60.0 <= pair[1].y));
        assert!(left_to_right.windows(2).all(|pair| pair[0].x == pair[1].x));
    }

    #[test]
    fn disconnected_components_do_not_overlap_in_either_direction() {
        let sizes = vec![
            (1, (100.0, 30.0)),
            (2, (60.0, 80.0)),
            (3, (120.0, 40.0)),
            (4, (50.0, 90.0)),
        ];
        let edges = vec![(1, 2), (3, 4)];

        for direction in [Direction::TopToBottom, Direction::LeftToRight] {
            let positions = pos_map(sugiyama_layout(
                &edges,
                &sizes,
                &SugiyamaConfig { direction, ..Default::default() },
            ));
            for left_id in [1, 2] {
                for right_id in [3, 4] {
                    let (left_x, left_y) = positions[&left_id];
                    let (right_x, right_y) = positions[&right_id];
                    let (_, (left_width, left_height)) = sizes
                        .iter()
                        .find(|(id, _)| *id == left_id)
                        .copied()
                        .unwrap();
                    let (_, (right_width, right_height)) = sizes
                        .iter()
                        .find(|(id, _)| *id == right_id)
                        .copied()
                        .unwrap();
                    assert!(
                        left_x + left_width <= right_x
                            || right_x + right_width <= left_x
                            || left_y + left_height <= right_y
                            || right_y + right_height <= left_y,
                        "components overlap in {direction:?}: {left_id} and {right_id}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_cycle_does_not_panic() {
        // rust-sugiyama handles cycles internally; verify we don't break
        let sizes = vec![
            (1, (80.0, 40.0)),
            (2, (80.0, 40.0)),
            (3, (80.0, 40.0)),
        ];
        let edges = vec![(1, 2), (2, 3), (3, 1)];
        let result = sugiyama_layout(&edges, &sizes, &SugiyamaConfig::default());

        // All nodes should still get positions
        assert_eq!(result.len(), 3);
        let pos = pos_map(result);
        assert!(pos[&1].0.is_finite());
        assert!(pos[&2].0.is_finite());
        assert!(pos[&3].0.is_finite());
    }

    // ========================================================================
    // sugiyama_layout_from_cache() tests
    // ========================================================================

    use crate::hit_test::SimpleNodeGeometry;
    use crate::state::StoredPin;

    /// Build a GeometryCache with the given nodes and pins.
    fn make_cache(
        nodes: &[(i32, f32, f32, f32, f32)],
        pins: &[(i32, i32, i32, f32, f32)],
    ) -> GeometryCache<SimpleNodeGeometry> {
        let mut cache = GeometryCache::default();
        for &(id, x, y, w, h) in nodes {
            cache.node_rects.insert(id, SimpleNodeGeometry { id, x, y, width: w, height: h });
        }
        for &(pin_id, node_id, pin_type, rel_x, rel_y) in pins {
            cache.pin_positions.insert(pin_id, StoredPin { node_id, pin_type, rel_x, rel_y });
        }
        cache
    }

    #[test]
    fn test_from_cache_resolves_pins_to_nodes() {
        // Two nodes with one output pin each and one input pin each
        // Pin encoding: output = id*2+1, input = id*2
        let cache = make_cache(
            &[(1, 0.0, 0.0, 100.0, 50.0), (2, 200.0, 0.0, 100.0, 50.0)],
            &[
                (3, 1, 2, 100.0, 25.0), // output pin on node 1
                (4, 2, 1, 0.0, 25.0),   // input pin on node 2
            ],
        );
        // Edge from pin 3 (node 1 output) → pin 4 (node 2 input)
        let result = sugiyama_layout_from_cache(&cache, &[(3, 4)], &SugiyamaConfig::default());
        let pos = pos_map(result);

        assert_eq!(pos.len(), 2);
        assert!(pos.contains_key(&1));
        assert!(pos.contains_key(&2));
        // Node 1 should be above node 2 (top-to-bottom default)
        assert!(pos[&1].1 < pos[&2].1, "source should be in earlier layer");
    }

    #[test]
    fn test_from_cache_skips_self_loops() {
        // Two pins on the same node — edge between them is a self-loop
        let cache = make_cache(
            &[(1, 0.0, 0.0, 100.0, 50.0)],
            &[
                (10, 1, 2, 100.0, 25.0), // output pin on node 1
                (11, 1, 1, 0.0, 25.0),   // input pin on node 1
            ],
        );
        let result = sugiyama_layout_from_cache(&cache, &[(10, 11)], &SugiyamaConfig::default());

        // Node should still appear (it exists in cache), but with no edges
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn test_from_cache_deduplicates_edges() {
        // Two separate pin-pairs connecting node 1 → node 2
        let cache = make_cache(
            &[(1, 0.0, 0.0, 100.0, 50.0), (2, 200.0, 0.0, 100.0, 50.0)],
            &[
                (10, 1, 2, 100.0, 10.0), // output pin A on node 1
                (11, 1, 2, 100.0, 40.0), // output pin B on node 1
                (20, 2, 1, 0.0, 10.0),   // input pin A on node 2
                (21, 2, 1, 0.0, 40.0),   // input pin B on node 2
            ],
        );
        // Two edges that both resolve to (node 1 → node 2)
        let result = sugiyama_layout_from_cache(
            &cache,
            &[(10, 20), (11, 21)],
            &SugiyamaConfig::default(),
        );

        // Should produce exactly 2 positioned nodes, not crash or duplicate
        assert_eq!(result.len(), 2);
        let pos = pos_map(result);
        assert!(pos.contains_key(&1));
        assert!(pos.contains_key(&2));
    }

    #[test]
    fn test_from_cache_skips_unknown_pins() {
        let cache = make_cache(
            &[(1, 0.0, 0.0, 100.0, 50.0)],
            &[(10, 1, 2, 100.0, 25.0)],
        );
        // Edge references pin 999 which doesn't exist
        let result = sugiyama_layout_from_cache(
            &cache,
            &[(10, 999)],
            &SugiyamaConfig::default(),
        );

        // Node 1 still appears (from cache), edge is just ignored
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn test_from_cache_empty() {
        let cache: GeometryCache<SimpleNodeGeometry> = GeometryCache::default();
        let result = sugiyama_layout_from_cache(&cache, &[], &SugiyamaConfig::default());
        assert!(result.is_empty());
    }

    #[test]
    fn test_from_cache_reads_node_dimensions() {
        // Nodes with different sizes — layout should use cache dimensions
        let cache = make_cache(
            &[
                (1, 0.0, 0.0, 200.0, 30.0), // wide node
                (2, 0.0, 0.0, 50.0, 100.0),  // tall node
            ],
            &[
                (10, 1, 2, 200.0, 15.0),
                (20, 2, 1, 0.0, 50.0),
            ],
        );
        let result = sugiyama_layout_from_cache(&cache, &[(10, 20)], &SugiyamaConfig::default());
        assert_eq!(result.len(), 2);

        let pos = pos_map(result);
        // Both nodes should have valid positions
        assert!(pos[&1].0.is_finite());
        assert!(pos[&2].0.is_finite());
    }

    #[test]
    fn test_from_cache_with_direction() {
        let cache = make_cache(
            &[(1, 0.0, 0.0, 100.0, 50.0), (2, 200.0, 0.0, 100.0, 50.0)],
            &[
                (10, 1, 2, 100.0, 25.0),
                (20, 2, 1, 0.0, 25.0),
            ],
        );
        let config = SugiyamaConfig {
            direction: Direction::LeftToRight,
            ..Default::default()
        };
        let result = sugiyama_layout_from_cache(&cache, &[(10, 20)], &config);
        let pos = pos_map(result);

        // In left-to-right, source should be left of target
        assert!(pos[&1].0 < pos[&2].0, "source should be left of target in LTR layout");
    }

    #[test]
    fn cache_insertion_and_edge_order_do_not_change_layout() {
        let nodes = vec![
            (30, 0.0, 0.0, 70.0, 35.0),
            (10, 0.0, 0.0, 80.0, 40.0),
            (40, 0.0, 0.0, 90.0, 45.0),
            (20, 0.0, 0.0, 60.0, 30.0),
        ];
        let pins = vec![
            (101, 10, 2, 80.0, 20.0),
            (201, 20, 1, 0.0, 15.0),
            (102, 10, 2, 80.0, 20.0),
            (301, 30, 1, 0.0, 17.5),
            (202, 20, 2, 60.0, 15.0),
            (401, 40, 1, 0.0, 22.5),
            (302, 30, 2, 70.0, 17.5),
            (402, 40, 1, 0.0, 22.5),
        ];
        let mut reversed_nodes = nodes.clone();
        reversed_nodes.reverse();
        let mut reversed_pins = pins.clone();
        reversed_pins.reverse();
        let cache = make_cache(&nodes, &pins);
        let reversed_cache = make_cache(&reversed_nodes, &reversed_pins);
        let edges = vec![(101, 201), (102, 301), (202, 401), (302, 402)];
        let mut reversed_edges = edges.clone();
        reversed_edges.reverse();

        for direction in [Direction::TopToBottom, Direction::LeftToRight] {
            let config = SugiyamaConfig { direction, ..Default::default() };
            assert_eq!(
                sugiyama_layout_from_cache(&reversed_cache, &reversed_edges, &config),
                sugiyama_layout_from_cache(&cache, &edges, &config)
            );
        }
    }
}
