use crate::path::{distance_to_bezier, CubicBezier};

/// Trait for link geometry data needed for hit-testing
pub trait LinkGeometry {
    fn id(&self) -> i32;
    fn start(&self) -> (f32, f32);
    fn end(&self) -> (f32, f32);
}

/// Trait for pin geometry data needed for hit-testing
pub trait PinGeometry {
    fn id(&self) -> i32;
    fn position(&self) -> (f32, f32);
}

/// Trait for node geometry data needed for selection
pub trait NodeGeometry {
    fn id(&self) -> i32;
    fn rect(&self) -> (f32, f32, f32, f32); // x, y, width, height
}

// === Standard Implementations ===

/// Simple implementation of LinkGeometry
#[derive(Debug, Clone, Copy)]
pub struct SimpleLinkGeometry {
    pub id: i32,
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
}

impl LinkGeometry for SimpleLinkGeometry {
    fn id(&self) -> i32 { self.id }
    fn start(&self) -> (f32, f32) { (self.start_x, self.start_y) }
    fn end(&self) -> (f32, f32) { (self.end_x, self.end_y) }
}

/// Simple implementation of PinGeometry
#[derive(Debug, Clone, Copy)]
pub struct SimplePinGeometry {
    pub id: i32,
    pub x: f32,
    pub y: f32,
}

impl PinGeometry for SimplePinGeometry {
    fn id(&self) -> i32 { self.id }
    fn position(&self) -> (f32, f32) { (self.x, self.y) }
}

/// Simple implementation of NodeGeometry
#[derive(Debug, Clone, Copy)]
pub struct SimpleNodeGeometry {
    pub id: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl NodeGeometry for SimpleNodeGeometry {
    fn id(&self) -> i32 { self.id }
    fn rect(&self) -> (f32, f32, f32, f32) { (self.x, self.y, self.width, self.height) }
}

/// Find a link at the given position
///
/// Returns the ID of the closest link within hover_distance, or -1 if none.
pub fn find_link_at<L, I>(
    mouse_x: f32,
    mouse_y: f32,
    links: I,
    hover_distance: f32,
    zoom: f32,
    bezier_min_offset: f32,
    hit_samples: usize,
) -> i32
where
    L: LinkGeometry,
    I: IntoIterator<Item = L>,
{
    let mut closest_link_id: i32 = -1;
    let mut closest_distance = hover_distance;

    for link in links {
        let (start_x, start_y) = link.start();
        let (end_x, end_y) = link.end();

        let bezier = CubicBezier::from_endpoints(
            start_x,
            start_y,
            end_x,
            end_y,
            zoom,
            bezier_min_offset,
        );
        let distance = distance_to_bezier((mouse_x, mouse_y), &bezier, hit_samples);

        if distance < closest_distance {
            closest_distance = distance;
            closest_link_id = link.id();
        }
    }

    closest_link_id
}

/// Find a pin at the given position
///
/// Returns the ID of the closest pin within hit_radius, or 0 if none.
pub fn find_pin_at<P, I>(mouse_x: f32, mouse_y: f32, pins: I, hit_radius: f32) -> i32
where
    P: PinGeometry,
    I: IntoIterator<Item = P>,
{
    let hit_radius_sq = hit_radius * hit_radius;

    for pin in pins {
        let (pin_x, pin_y) = pin.position();
        let dx = mouse_x - pin_x;
        let dy = mouse_y - pin_y;
        if dx * dx + dy * dy <= hit_radius_sq {
            return pin.id();
        }
    }

    0 // No pin found
}

/// Find all nodes that intersect with a selection box
pub fn nodes_in_selection_box<N, I>(
    sel_x: f32,
    sel_y: f32,
    sel_width: f32,
    sel_height: f32,
    nodes: I,
) -> Vec<i32>
where
    N: NodeGeometry,
    I: IntoIterator<Item = N>,
{
    nodes
        .into_iter()
        .filter(|node| {
            let (x, y, w, h) = node.rect();
            x < sel_x + sel_width
                && x + w > sel_x
                && y < sel_y + sel_height
                && y + h > sel_y
        })
        .map(|node| node.id())
        .collect()
}

/// Find all links that intersect with a selection box
pub fn links_in_selection_box<L, I>(
    sel_x: f32,
    sel_y: f32,
    sel_width: f32,
    sel_height: f32,
    links: I,
) -> Vec<i32>
where
    L: LinkGeometry,
    I: IntoIterator<Item = L>,
{
    links
        .into_iter()
        .filter(|link| {
            let (start_x, start_y) = link.start();
            let (end_x, end_y) = link.end();

            let start_in_box = start_x >= sel_x && start_x <= sel_x + sel_width
                && start_y >= sel_y && start_y <= sel_y + sel_height;

            let end_in_box = end_x >= sel_x && end_x <= sel_x + sel_width
                && end_y >= sel_y && end_y <= sel_y + sel_height;

            start_in_box || end_in_box
        })
        .map(|link| link.id())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // find_pin_at() - Pin Hit Testing
    // ========================================================================

    #[test]
    fn test_find_pin_at() {
        let pins = vec![
            SimplePinGeometry { id: 1001, x: 10.0, y: 10.0 },
            SimplePinGeometry { id: 2001, x: 50.0, y: 50.0 },
        ];

        // Pass vector (IntoIterator)
        assert_eq!(find_pin_at(12.0, 12.0, pins.clone(), 10.0), 1001);
        assert_eq!(find_pin_at(52.0, 52.0, pins.clone(), 10.0), 2001);
        assert_eq!(find_pin_at(100.0, 100.0, pins, 10.0), 0);
    }

    #[test]
    fn test_find_pin_at_exact_position() {
        let pins = vec![SimplePinGeometry { id: 1001, x: 50.0, y: 50.0 }];
        assert_eq!(find_pin_at(50.0, 50.0, pins, 10.0), 1001);
    }

    #[test]
    fn test_find_pin_at_boundary_radius() {
        let pins = vec![SimplePinGeometry { id: 1001, x: 50.0, y: 50.0 }];

        // Exactly at radius distance
        assert_eq!(find_pin_at(60.0, 50.0, pins.clone(), 10.0), 1001);

        // Just outside radius
        assert_eq!(find_pin_at(60.1, 50.0, pins, 10.0), 0);
    }

    #[test]
    fn test_find_pin_at_empty_list() {
        let pins: Vec<SimplePinGeometry> = vec![];
        assert_eq!(find_pin_at(50.0, 50.0, pins, 10.0), 0);
    }

    #[test]
    fn test_find_pin_at_first_match_wins() {
        // Two overlapping pins - first one should be returned
        let pins = vec![
            SimplePinGeometry { id: 1001, x: 50.0, y: 50.0 },
            SimplePinGeometry { id: 2001, x: 50.0, y: 50.0 },
        ];
        assert_eq!(find_pin_at(50.0, 50.0, pins, 10.0), 1001);
    }

    #[test]
    fn test_find_pin_at_zero_radius() {
        let pins = vec![SimplePinGeometry { id: 1001, x: 50.0, y: 50.0 }];

        // Exact match with zero radius
        assert_eq!(find_pin_at(50.0, 50.0, pins.clone(), 0.0), 1001);

        // Any offset with zero radius should miss
        assert_eq!(find_pin_at(50.1, 50.0, pins, 0.0), 0);
    }

    // ========================================================================
    // find_link_at() - Link Hit Testing (Core function)
    // ========================================================================

    #[test]
    fn test_find_link_at_single_link() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 0.0,
            start_y: 50.0,
            end_x: 100.0,
            end_y: 50.0,
        }];

        // Click on the middle of a horizontal link
        let result = find_link_at(50.0, 50.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_near_start() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 0.0,
            start_y: 50.0,
            end_x: 100.0,
            end_y: 50.0,
        }];

        // Click near the start point
        let result = find_link_at(5.0, 50.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_near_end() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 0.0,
            start_y: 50.0,
            end_x: 100.0,
            end_y: 50.0,
        }];

        // Click near the end point
        let result = find_link_at(95.0, 50.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_miss() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 0.0,
            start_y: 50.0,
            end_x: 100.0,
            end_y: 50.0,
        }];

        // Click far away from the link
        let result = find_link_at(50.0, 200.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_find_link_at_empty_list() {
        let links: Vec<SimpleLinkGeometry> = vec![];
        let result = find_link_at(50.0, 50.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_find_link_at_closest_wins() {
        // Two links at different distances
        let links = vec![
            SimpleLinkGeometry {
                id: 1,
                start_x: 0.0,
                start_y: 50.0, // At y=50
                end_x: 100.0,
                end_y: 50.0,
            },
            SimpleLinkGeometry {
                id: 2,
                start_x: 0.0,
                start_y: 55.0, // At y=55, closer to y=53
                end_x: 100.0,
                end_y: 55.0,
            },
        ];

        // Click at y=53 - link 2 should be closer
        let result = find_link_at(50.0, 53.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_find_link_at_first_wins_on_tie() {
        // Two links at exactly the same position
        let links = vec![
            SimpleLinkGeometry {
                id: 1,
                start_x: 0.0,
                start_y: 50.0,
                end_x: 100.0,
                end_y: 50.0,
            },
            SimpleLinkGeometry {
                id: 2,
                start_x: 0.0,
                start_y: 50.0,
                end_x: 100.0,
                end_y: 50.0,
            },
        ];

        // First link should win on exact tie
        let result = find_link_at(50.0, 50.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_hover_distance_threshold() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 0.0,
            start_y: 50.0,
            end_x: 100.0,
            end_y: 50.0,
        }];

        // Just within hover distance
        let result1 = find_link_at(50.0, 59.0, links.clone(), 10.0, 1.0, 50.0, 20);
        assert_eq!(result1, 1);

        // Just outside hover distance
        let result2 = find_link_at(50.0, 70.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result2, -1);
    }

    #[test]
    fn test_find_link_at_different_zoom() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 0.0,
            start_y: 50.0,
            end_x: 100.0,
            end_y: 50.0,
        }];

        // Same click position with different zoom levels
        // The bezier shape changes with zoom, which may affect hit detection
        let result1 = find_link_at(50.0, 50.0, links.clone(), 10.0, 1.0, 50.0, 20);
        let result2 = find_link_at(50.0, 50.0, links, 10.0, 2.0, 50.0, 20);

        // Both should hit since we're clicking right on the curve
        assert_eq!(result1, 1);
        assert_eq!(result2, 1);
    }

    #[test]
    fn test_find_link_at_very_short_link() {
        // Link where start ≈ end
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 50.0,
            start_y: 50.0,
            end_x: 51.0,
            end_y: 50.0,
        }];

        // Click on the point
        let result = find_link_at(50.5, 50.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_diagonal_link() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 0.0,
            start_y: 0.0,
            end_x: 100.0,
            end_y: 100.0,
        }];

        // Click near the middle of a diagonal bezier
        let result = find_link_at(50.0, 50.0, links, 20.0, 1.0, 50.0, 20);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_link_at_multiple_links_one_hit() {
        let links = vec![
            SimpleLinkGeometry {
                id: 1,
                start_x: 0.0,
                start_y: 0.0,
                end_x: 100.0,
                end_y: 0.0,
            },
            SimpleLinkGeometry {
                id: 2,
                start_x: 0.0,
                start_y: 100.0,
                end_x: 100.0,
                end_y: 100.0,
            },
            SimpleLinkGeometry {
                id: 3,
                start_x: 0.0,
                start_y: 200.0,
                end_x: 100.0,
                end_y: 200.0,
            },
        ];

        // Click on the middle link
        let result = find_link_at(50.0, 100.0, links, 10.0, 1.0, 50.0, 20);
        assert_eq!(result, 2);
    }

    // ========================================================================
    // nodes_in_selection_box() - Box Selection
    // ========================================================================

    #[test]
    fn test_nodes_in_selection_box() {
        let nodes = vec![
            SimpleNodeGeometry { id: 1, x: 0.0, y: 0.0, width: 100.0, height: 80.0 },
            SimpleNodeGeometry { id: 2, x: 200.0, y: 0.0, width: 100.0, height: 80.0 },
            SimpleNodeGeometry { id: 3, x: 50.0, y: 100.0, width: 100.0, height: 80.0 },
        ];

        let selected = nodes_in_selection_box(0.0, 0.0, 150.0, 200.0, nodes);
        assert!(selected.contains(&1));
        assert!(selected.contains(&3));
        assert!(!selected.contains(&2));
    }

    #[test]
    fn test_nodes_in_selection_box_empty() {
        let nodes: Vec<SimpleNodeGeometry> = vec![];
        let selected = nodes_in_selection_box(0.0, 0.0, 100.0, 100.0, nodes);
        assert!(selected.is_empty());
    }

    #[test]
    fn test_nodes_in_selection_box_partial_overlap() {
        let nodes = vec![SimpleNodeGeometry {
            id: 1,
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        }];

        // Selection box partially overlaps the node
        let selected = nodes_in_selection_box(0.0, 0.0, 60.0, 60.0, nodes);
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_nodes_in_selection_box_no_overlap() {
        let nodes = vec![SimpleNodeGeometry {
            id: 1,
            x: 200.0,
            y: 200.0,
            width: 100.0,
            height: 100.0,
        }];

        let selected = nodes_in_selection_box(0.0, 0.0, 100.0, 100.0, nodes);
        assert!(selected.is_empty());
    }

    #[test]
    fn test_nodes_in_selection_box_touching_edge() {
        let nodes = vec![SimpleNodeGeometry {
            id: 1,
            x: 100.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }];

        // Selection box ends exactly where node starts - no overlap
        let selected = nodes_in_selection_box(0.0, 0.0, 100.0, 100.0, nodes);
        assert!(selected.is_empty());
    }

    // ========================================================================
    // links_in_selection_box() - Link Box Selection
    // ========================================================================

    #[test]
    fn test_links_in_selection_box() {
        let links = vec![
            SimpleLinkGeometry { id: 1, start_x: 10.0, start_y: 10.0, end_x: 200.0, end_y: 10.0 },
            SimpleLinkGeometry { id: 2, start_x: 200.0, start_y: 10.0, end_x: 10.0, end_y: 10.0 },
            SimpleLinkGeometry { id: 3, start_x: 200.0, start_y: 10.0, end_x: 300.0, end_y: 10.0 },
        ];

        let selected = links_in_selection_box(0.0, 0.0, 100.0, 100.0, links);
        assert!(selected.contains(&1));
        assert!(selected.contains(&2));
        assert!(!selected.contains(&3));
    }

    #[test]
    fn test_links_in_selection_box_end_inside() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 200.0, // Outside box
            start_y: 50.0,
            end_x: 50.0, // Inside box
            end_y: 50.0,
        }];

        let selected = links_in_selection_box(0.0, 0.0, 100.0, 100.0, links);
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_links_in_selection_box_neither_inside() {
        let links = vec![SimpleLinkGeometry {
            id: 1,
            start_x: 200.0,
            start_y: 50.0,
            end_x: 300.0,
            end_y: 50.0,
        }];

        let selected = links_in_selection_box(0.0, 0.0, 100.0, 100.0, links);
        assert!(selected.is_empty());
    }

    #[test]
    fn test_links_in_selection_box_empty() {
        let links: Vec<SimpleLinkGeometry> = vec![];
        let selected = links_in_selection_box(0.0, 0.0, 100.0, 100.0, links);
        assert!(selected.is_empty());
    }

    // ========================================================================
    // Trait implementations
    // ========================================================================

    #[test]
    fn test_simple_link_geometry_trait() {
        let link = SimpleLinkGeometry {
            id: 42,
            start_x: 10.0,
            start_y: 20.0,
            end_x: 30.0,
            end_y: 40.0,
        };

        assert_eq!(link.id(), 42);
        assert_eq!(link.start(), (10.0, 20.0));
        assert_eq!(link.end(), (30.0, 40.0));
    }

    #[test]
    fn test_simple_pin_geometry_trait() {
        let pin = SimplePinGeometry {
            id: 1001,
            x: 50.0,
            y: 75.0,
        };

        assert_eq!(pin.id(), 1001);
        assert_eq!(pin.position(), (50.0, 75.0));
    }

    #[test]
    fn test_simple_node_geometry_trait() {
        let node = SimpleNodeGeometry {
            id: 1,
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 80.0,
        };

        assert_eq!(node.id(), 1);
        assert_eq!(node.rect(), (10.0, 20.0, 100.0, 80.0));
    }
}
