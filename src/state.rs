use std::collections::HashMap;
use crate::hit_test::{
    find_link_at, find_pin_at, links_in_selection_box, nodes_in_selection_box, SimpleLinkGeometry,
    SimpleNodeGeometry, SimplePinGeometry, NodeGeometry,
};
use crate::path::generate_bezier_path;

#[derive(Clone, Copy, Debug)]
pub struct StoredPin {
    pub node_id: i32,
    pub pin_type: i32,
    pub rel_x: f32,
    pub rel_y: f32,
}

/// Helper struct to manage spatial state of the editor (node rects and pin positions)
/// 
/// Generic over N to allow using specialized node types that implement NodeGeometry.
pub struct GeometryCache<N = SimpleNodeGeometry> {
    pub node_rects: HashMap<i32, N>,
    pub pin_positions: HashMap<i32, StoredPin>,
}

impl<N> Default for GeometryCache<N> {
    fn default() -> Self {
        Self {
            node_rects: HashMap::new(),
            pin_positions: HashMap::new(),
        }
    }
}

impl<N> GeometryCache<N> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<N> GeometryCache<N>
where
    N: NodeGeometry + Copy,
{
    /// Iterator over absolute pin positions for hit testing
    pub fn get_absolute_pins(&self) -> impl Iterator<Item = SimplePinGeometry> + '_ {
        self.pin_positions
            .iter()
            .filter_map(move |(&pin_id, pin_pos)| {
                let rect = self.node_rects.get(&pin_pos.node_id)?.rect();
                Some(SimplePinGeometry {
                    id: pin_id,
                    x: rect.0 + pin_pos.rel_x,
                    y: rect.1 + pin_pos.rel_y,
                })
            })
    }

    /// Iterator over absolute link geometries for hit testing
    pub fn get_absolute_links<'a, I>(
        &'a self,
        links: I,
    ) -> impl Iterator<Item = SimpleLinkGeometry> + 'a
    where
        I: Iterator<Item = (i32, i32, i32)> + 'a,
    {
        links.filter_map(move |(id, start_pin, end_pin)| {
            let start_pos = self.pin_positions.get(&start_pin)?;
            let end_pos = self.pin_positions.get(&end_pin)?;

            let start_rect = self.node_rects.get(&start_pos.node_id)?.rect();
            let end_rect = self.node_rects.get(&end_pos.node_id)?.rect();

            Some(SimpleLinkGeometry {
                id,
                start_x: start_rect.0 + start_pos.rel_x,
                start_y: start_rect.1 + start_pos.rel_y,
                end_x: end_rect.0 + end_pos.rel_x,
                end_y: end_rect.1 + end_pos.rel_y,
            })
        })
    }

    /// Find pin at position
    pub fn find_pin_at(&self, x: f32, y: f32, hit_radius: f32) -> i32 {
        find_pin_at(x, y, self.get_absolute_pins(), hit_radius)
    }

    /// Find link at position
    #[allow(clippy::too_many_arguments)]
    pub fn find_link_at<'a, I>(
        &'a self,
        x: f32,
        y: f32,
        links: I,
        hover_distance: f32,
        zoom: f32,
        bezier_min_offset: f32,
        hit_samples: usize,
    ) -> i32
    where
        I: Iterator<Item = (i32, i32, i32)> + 'a,
    {
        find_link_at(
            x,
            y,
            self.get_absolute_links(links),
            hover_distance,
            zoom,
            bezier_min_offset,
            hit_samples,
        )
    }

    /// Compute nodes in selection box
    pub fn nodes_in_selection_box(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Vec<i32> {
        nodes_in_selection_box(
            x,
            y,
            width,
            height,
            self.node_rects.values().copied(),
        )
    }

    /// Compute links in selection box
    pub fn links_in_selection_box<'a, I>(
        &'a self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        links: I,
    ) -> Vec<i32>
    where
        I: Iterator<Item = (i32, i32, i32)> + 'a,
    {
        links_in_selection_box(
            x,
            y,
            width,
            height,
            self.get_absolute_links(links),
        )
    }

    /// Compute bezier path for a link (same-space: node rects and pin offsets in same coordinate system)
    pub fn compute_link_path(
        &self,
        start_pin: i32,
        end_pin: i32,
        zoom: f32,
        bezier_min_offset: f32,
    ) -> Option<String> {
        let start_pos = self.pin_positions.get(&start_pin)?;
        let end_pos = self.pin_positions.get(&end_pin)?;

        let start_rect = self.node_rects.get(&start_pos.node_id)?.rect();
        let end_rect = self.node_rects.get(&end_pos.node_id)?.rect();

        Some(generate_bezier_path(
            start_rect.0 + start_pos.rel_x,
            start_rect.1 + start_pos.rel_y,
            end_rect.0 + end_pos.rel_x,
            end_rect.1 + end_pos.rel_y,
            zoom,
            bezier_min_offset,
        ))
    }

    /// Compute bezier path in screen space from world-space node rects and world-space relative pin offsets.
    ///
    /// Node rects are stored in world coordinates. Pin rel_x/rel_y are also in world space
    /// (unscaled). The screen-space pin position is:
    ///   screen_x = (node_world_x + pin_rel_x) * zoom + pan_x
    ///   screen_y = (node_world_y + pin_rel_y) * zoom + pan_y
    pub fn compute_link_path_screen(
        &self,
        start_pin: i32,
        end_pin: i32,
        zoom: f32,
        pan_x: f32,
        pan_y: f32,
        bezier_min_offset: f32,
    ) -> Option<String> {
        let start_pos = self.pin_positions.get(&start_pin)?;
        let end_pos = self.pin_positions.get(&end_pin)?;

        let start_rect = self.node_rects.get(&start_pos.node_id)?.rect();
        let end_rect = self.node_rects.get(&end_pos.node_id)?.rect();

        let sx = (start_rect.0 + start_pos.rel_x) * zoom + pan_x;
        let sy = (start_rect.1 + start_pos.rel_y) * zoom + pan_y;
        let ex = (end_rect.0 + end_pos.rel_x) * zoom + pan_x;
        let ey = (end_rect.1 + end_pos.rel_y) * zoom + pan_y;

        Some(generate_bezier_path(sx, sy, ex, ey, zoom, bezier_min_offset))
    }

    /// Standard handler for pin position reports from Slint
    pub fn handle_pin_report(
        &mut self,
        pin_id: i32,
        node_id: i32,
        pin_type: i32,
        rel_x: f32,
        rel_y: f32,
    ) {
        self.pin_positions.insert(
            pin_id,
            StoredPin {
                node_id,
                pin_type,
                rel_x,
                rel_y,
            },
        );
    }
}

/// Convenience implementation for the default SimpleNodeGeometry
impl GeometryCache<SimpleNodeGeometry> {
    /// Update a node's rectangle (shorthand for SimpleNodeGeometry)
    pub fn update_node_rect(&mut self, id: i32, x: f32, y: f32, width: f32, height: f32) {
        self.node_rects.insert(
            id,
            SimpleNodeGeometry {
                id,
                x,
                y,
                width,
                height,
            },
        );
    }

    /// Standard handler for node rect reports from Slint (for SimpleNodeGeometry)
    pub fn handle_node_rect_report(&mut self, id: i32, x: f32, y: f32, w: f32, h: f32) {
        self.update_node_rect(id, x, y, w, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test cache with two nodes and pins
    fn setup_test_cache() -> GeometryCache<SimpleNodeGeometry> {
        let mut cache = GeometryCache::new();

        // Node 1 at (0, 0) with size 100x50
        cache.update_node_rect(1, 0.0, 0.0, 100.0, 50.0);
        // Node 2 at (200, 100) with size 100x50
        cache.update_node_rect(2, 200.0, 100.0, 100.0, 50.0);

        // Pin 1001: output on node 1 at relative (100, 25) -> absolute (100, 25)
        cache.handle_pin_report(1001, 1, 2, 100.0, 25.0);
        // Pin 2001: input on node 2 at relative (0, 25) -> absolute (200, 125)
        cache.handle_pin_report(2001, 2, 1, 0.0, 25.0);

        cache
    }

    // ========================================================================
    // GeometryCache::new() and Default
    // ========================================================================

    #[test]
    fn test_new_cache_is_empty() {
        let cache: GeometryCache<SimpleNodeGeometry> = GeometryCache::new();
        assert!(cache.node_rects.is_empty());
        assert!(cache.pin_positions.is_empty());
    }

    #[test]
    fn test_default_cache_is_empty() {
        let cache: GeometryCache<SimpleNodeGeometry> = GeometryCache::default();
        assert!(cache.node_rects.is_empty());
        assert!(cache.pin_positions.is_empty());
    }

    // ========================================================================
    // handle_pin_report() - State Mutation
    // ========================================================================

    #[test]
    fn test_handle_pin_report_inserts_pin() {
        let mut cache: GeometryCache<SimpleNodeGeometry> = GeometryCache::new();
        cache.handle_pin_report(1001, 1, 2, 50.0, 25.0);

        let pin = cache.pin_positions.get(&1001).expect("Pin should exist");
        assert_eq!(pin.node_id, 1);
        assert_eq!(pin.pin_type, 2);
        assert_eq!(pin.rel_x, 50.0);
        assert_eq!(pin.rel_y, 25.0);
    }

    #[test]
    fn test_handle_pin_report_overwrites_existing() {
        let mut cache: GeometryCache<SimpleNodeGeometry> = GeometryCache::new();
        cache.handle_pin_report(1001, 1, 2, 50.0, 25.0);
        cache.handle_pin_report(1001, 1, 2, 100.0, 30.0); // Update position

        let pin = cache.pin_positions.get(&1001).expect("Pin should exist");
        assert_eq!(pin.rel_x, 100.0);
        assert_eq!(pin.rel_y, 30.0);
    }

    #[test]
    fn test_handle_pin_report_negative_coordinates() {
        let mut cache: GeometryCache<SimpleNodeGeometry> = GeometryCache::new();
        cache.handle_pin_report(1001, 1, 2, -10.0, -20.0);

        let pin = cache.pin_positions.get(&1001).expect("Pin should exist");
        assert_eq!(pin.rel_x, -10.0);
        assert_eq!(pin.rel_y, -20.0);
    }

    // ========================================================================
    // update_node_rect() - State Mutation
    // ========================================================================

    #[test]
    fn test_update_node_rect_inserts_node() {
        let mut cache = GeometryCache::new();
        cache.update_node_rect(1, 10.0, 20.0, 100.0, 50.0);

        let node = cache.node_rects.get(&1).expect("Node should exist");
        assert_eq!(node.id, 1);
        assert_eq!(node.x, 10.0);
        assert_eq!(node.y, 20.0);
        assert_eq!(node.width, 100.0);
        assert_eq!(node.height, 50.0);
    }

    #[test]
    fn test_update_node_rect_overwrites_existing() {
        let mut cache = GeometryCache::new();
        cache.update_node_rect(1, 10.0, 20.0, 100.0, 50.0);
        cache.update_node_rect(1, 50.0, 60.0, 150.0, 80.0);

        let node = cache.node_rects.get(&1).expect("Node should exist");
        assert_eq!(node.x, 50.0);
        assert_eq!(node.y, 60.0);
        assert_eq!(node.width, 150.0);
        assert_eq!(node.height, 80.0);
    }

    #[test]
    fn test_update_node_rect_negative_coordinates() {
        let mut cache = GeometryCache::new();
        cache.update_node_rect(1, -100.0, -200.0, 100.0, 50.0);

        let node = cache.node_rects.get(&1).expect("Node should exist");
        assert_eq!(node.x, -100.0);
        assert_eq!(node.y, -200.0);
    }

    // ========================================================================
    // get_absolute_pins() - Coordinate Transformation
    // ========================================================================

    #[test]
    fn test_get_absolute_pins_returns_absolute_positions() {
        let cache = setup_test_cache();
        let pins: Vec<SimplePinGeometry> = cache.get_absolute_pins().collect();

        // Find pin 1001: node at (0,0) + rel (100, 25) = (100, 25)
        let pin1 = pins.iter().find(|p| p.id == 1001).expect("Pin 1001 should exist");
        assert_eq!(pin1.x, 100.0);
        assert_eq!(pin1.y, 25.0);

        // Find pin 2001: node at (200, 100) + rel (0, 25) = (200, 125)
        let pin2 = pins.iter().find(|p| p.id == 2001).expect("Pin 2001 should exist");
        assert_eq!(pin2.x, 200.0);
        assert_eq!(pin2.y, 125.0);
    }

    #[test]
    fn test_get_absolute_pins_skips_orphan_pins() {
        let mut cache = setup_test_cache();
        // Add a pin referencing non-existent node
        cache.handle_pin_report(9999, 999, 1, 50.0, 25.0);

        let pins: Vec<SimplePinGeometry> = cache.get_absolute_pins().collect();

        // Should only have 2 valid pins, orphan is skipped
        assert_eq!(pins.len(), 2);
        assert!(!pins.iter().any(|p| p.id == 9999));
    }

    #[test]
    fn test_get_absolute_pins_empty_cache() {
        let cache: GeometryCache<SimpleNodeGeometry> = GeometryCache::new();
        let pins: Vec<SimplePinGeometry> = cache.get_absolute_pins().collect();
        assert!(pins.is_empty());
    }

    #[test]
    fn test_get_absolute_pins_node_at_negative_coords() {
        let mut cache = GeometryCache::new();
        cache.update_node_rect(1, -100.0, -50.0, 100.0, 50.0);
        cache.handle_pin_report(1001, 1, 2, 50.0, 25.0);

        let pins: Vec<SimplePinGeometry> = cache.get_absolute_pins().collect();
        let pin = pins.iter().find(|p| p.id == 1001).expect("Pin should exist");
        assert_eq!(pin.x, -50.0); // -100 + 50
        assert_eq!(pin.y, -25.0); // -50 + 25
    }

    // ========================================================================
    // get_absolute_links() - Complex Transformation
    // ========================================================================

    #[test]
    fn test_get_absolute_links_returns_absolute_positions() {
        let cache = setup_test_cache();
        let links_data = vec![(1, 1001, 2001)]; // (id, start_pin, end_pin)
        let links: Vec<SimpleLinkGeometry> =
            cache.get_absolute_links(links_data.into_iter()).collect();

        assert_eq!(links.len(), 1);
        let link = &links[0];
        assert_eq!(link.id, 1);
        // Start: pin 1001 -> (100, 25)
        assert_eq!(link.start_x, 100.0);
        assert_eq!(link.start_y, 25.0);
        // End: pin 2001 -> (200, 125)
        assert_eq!(link.end_x, 200.0);
        assert_eq!(link.end_y, 125.0);
    }

    #[test]
    fn test_get_absolute_links_skips_missing_start_pin() {
        let cache = setup_test_cache();
        let links_data = vec![(1, 9999, 2001)]; // Missing start pin
        let links: Vec<SimpleLinkGeometry> =
            cache.get_absolute_links(links_data.into_iter()).collect();

        assert!(links.is_empty());
    }

    #[test]
    fn test_get_absolute_links_skips_missing_end_pin() {
        let cache = setup_test_cache();
        let links_data = vec![(1, 1001, 9999)]; // Missing end pin
        let links: Vec<SimpleLinkGeometry> =
            cache.get_absolute_links(links_data.into_iter()).collect();

        assert!(links.is_empty());
    }

    #[test]
    fn test_get_absolute_links_skips_missing_start_node() {
        let mut cache = setup_test_cache();
        // Add pin referencing non-existent node
        cache.pin_positions.insert(
            3001,
            StoredPin {
                node_id: 999,
                pin_type: 1,
                rel_x: 0.0,
                rel_y: 25.0,
            },
        );

        let links_data = vec![(1, 3001, 2001)];
        let links: Vec<SimpleLinkGeometry> =
            cache.get_absolute_links(links_data.into_iter()).collect();

        assert!(links.is_empty());
    }

    #[test]
    fn test_get_absolute_links_empty_input() {
        let cache = setup_test_cache();
        let links_data: Vec<(i32, i32, i32)> = vec![];
        let links: Vec<SimpleLinkGeometry> =
            cache.get_absolute_links(links_data.into_iter()).collect();

        assert!(links.is_empty());
    }

    #[test]
    fn test_get_absolute_links_multiple_links() {
        let mut cache = setup_test_cache();
        // Add another pin on node 2
        cache.handle_pin_report(2002, 2, 1, 0.0, 40.0);

        let links_data = vec![(1, 1001, 2001), (2, 1001, 2002)];
        let links: Vec<SimpleLinkGeometry> =
            cache.get_absolute_links(links_data.into_iter()).collect();

        assert_eq!(links.len(), 2);
    }

    // ========================================================================
    // compute_link_path() - Bezier Path Generation
    // ========================================================================

    #[test]
    fn test_compute_link_path_returns_valid_svg() {
        let cache = setup_test_cache();
        let path = cache
            .compute_link_path(1001, 2001, 1.0, 50.0)
            .expect("Path should be generated");

        assert!(path.starts_with("M "));
        assert!(path.contains(" C "));
    }

    #[test]
    fn test_compute_link_path_returns_none_for_missing_start_pin() {
        let cache = setup_test_cache();
        let path = cache.compute_link_path(9999, 2001, 1.0, 50.0);
        assert!(path.is_none());
    }

    #[test]
    fn test_compute_link_path_returns_none_for_missing_end_pin() {
        let cache = setup_test_cache();
        let path = cache.compute_link_path(1001, 9999, 1.0, 50.0);
        assert!(path.is_none());
    }

    #[test]
    fn test_compute_link_path_returns_none_for_missing_start_node() {
        let mut cache = setup_test_cache();
        cache.pin_positions.insert(
            3001,
            StoredPin {
                node_id: 999,
                pin_type: 1,
                rel_x: 0.0,
                rel_y: 25.0,
            },
        );

        let path = cache.compute_link_path(3001, 2001, 1.0, 50.0);
        assert!(path.is_none());
    }

    #[test]
    fn test_compute_link_path_different_zoom_levels() {
        let cache = setup_test_cache();

        let path1 = cache.compute_link_path(1001, 2001, 1.0, 50.0).unwrap();
        let path2 = cache.compute_link_path(1001, 2001, 2.0, 50.0).unwrap();

        // Different zoom should produce different paths
        assert_ne!(path1, path2);
    }

    // ========================================================================
    // find_pin_at() - Delegated Hit Testing
    // ========================================================================

    #[test]
    fn test_find_pin_at_hits_pin() {
        let cache = setup_test_cache();
        // Pin 1001 is at (100, 25)
        let pin_id = cache.find_pin_at(102.0, 27.0, 10.0);
        assert_eq!(pin_id, 1001);
    }

    #[test]
    fn test_find_pin_at_misses_all() {
        let cache = setup_test_cache();
        let pin_id = cache.find_pin_at(500.0, 500.0, 10.0);
        assert_eq!(pin_id, 0);
    }

    // ========================================================================
    // nodes_in_selection_box() - Selection Box Query
    // ========================================================================

    #[test]
    fn test_nodes_in_selection_box_finds_intersecting() {
        let cache = setup_test_cache();
        // Node 1 is at (0, 0) with size 100x50
        // Selection box covering it
        let selected = cache.nodes_in_selection_box(0.0, 0.0, 50.0, 50.0);
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_nodes_in_selection_box_excludes_non_intersecting() {
        let cache = setup_test_cache();
        // Selection box that doesn't cover node 1 (at 0,0) or node 2 (at 200,100)
        let selected = cache.nodes_in_selection_box(500.0, 500.0, 50.0, 50.0);
        assert!(selected.is_empty());
    }

    // ========================================================================
    // links_in_selection_box() - Link Selection Query
    // ========================================================================

    #[test]
    fn test_links_in_selection_box_finds_link_with_start_inside() {
        let cache = setup_test_cache();
        let links_data = vec![(1, 1001, 2001)];

        // Selection box covering pin 1001 position (100, 25)
        let selected = cache.links_in_selection_box(90.0, 15.0, 20.0, 20.0, links_data.into_iter());
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_links_in_selection_box_excludes_link_outside() {
        let cache = setup_test_cache();
        let links_data = vec![(1, 1001, 2001)];

        // Selection box not covering either pin
        let selected =
            cache.links_in_selection_box(500.0, 500.0, 50.0, 50.0, links_data.into_iter());
        assert!(selected.is_empty());
    }

    // ========================================================================
    // compute_link_path_screen() - World→Screen Path Generation
    // ========================================================================

    #[test]
    fn test_compute_link_path_screen_zoom1_pan0() {
        let cache = setup_test_cache();
        // At zoom=1, pan=0 the screen-space path should equal
        // node_world + pin_rel (same as compute_link_path at zoom=1)
        let path = cache
            .compute_link_path_screen(1001, 2001, 1.0, 0.0, 0.0, 50.0)
            .expect("Path should be generated");
        assert!(path.starts_with("M "));
        assert!(path.contains(" C "));
    }

    #[test]
    fn test_compute_link_path_screen_with_pan() {
        let cache = setup_test_cache();
        // With pan offset, paths should differ from zero-pan
        let path_no_pan = cache
            .compute_link_path_screen(1001, 2001, 1.0, 0.0, 0.0, 50.0)
            .unwrap();
        let path_with_pan = cache
            .compute_link_path_screen(1001, 2001, 1.0, 100.0, 50.0, 50.0)
            .unwrap();
        assert_ne!(path_no_pan, path_with_pan);
    }

    #[test]
    fn test_compute_link_path_screen_with_zoom() {
        let cache = setup_test_cache();
        let path_z1 = cache
            .compute_link_path_screen(1001, 2001, 1.0, 0.0, 0.0, 50.0)
            .unwrap();
        let path_z2 = cache
            .compute_link_path_screen(1001, 2001, 2.0, 0.0, 0.0, 50.0)
            .unwrap();
        assert_ne!(path_z1, path_z2);
    }

    #[test]
    fn test_compute_link_path_screen_missing_pin() {
        let cache = setup_test_cache();
        assert!(cache
            .compute_link_path_screen(9999, 2001, 1.0, 0.0, 0.0, 50.0)
            .is_none());
    }
}