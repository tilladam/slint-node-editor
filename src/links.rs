//! Link management for the node editor.
//!
//! This module provides [`LinkManager`], which handles the synchronization
//! between logical link data and visual link paths for rendering.
//!
//! # Example
//!
//! ```ignore
//! use slint_node_editor::{GeometryTracker, LinkManager, SimpleLink};
//! use slint::Color;
//!
//! // Set up geometry tracking
//! let tracker = GeometryTracker::new();
//! window.on_node_rect_changed(tracker.node_rect_callback());
//! window.on_pin_position_changed(tracker.pin_position_callback());
//!
//! // Set up link management
//! let mut links = LinkManager::new(tracker.cache());
//!
//! // Add a link with default line width (2.0)
//! links.add(SimpleLink::new(1, output_pin, input_pin, Color::from_rgb_u8(100, 180, 255)));
//!
//! // Or add a link with custom line width
//! links.add(SimpleLink::with_line_width(2, output_pin, input_pin, Color::from_rgb_u8(255, 100, 100), 4.0));
//!
//! // Connect to Slint
//! window.set_link_paths(links.paths());
//!
//! // Update paths when geometry changes (in callbacks)
//! links.update_paths(zoom, bezier_offset);
//! ```

use crate::graph::LinkModel;
use crate::hit_test::NodeGeometry;
use crate::state::GeometryCache;
use slint::{Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

/// Internal trait for auto-syncing to Slint models.
trait ModelSyncer {
    fn sync(&self, paths: &[LinkPathData]);
}

/// Concrete implementation of ModelSyncer for a specific path type.
struct ConcreteModelSyncer<P, F> {
    model: Rc<VecModel<P>>,
    constructor: F,
}

impl<P, F> ModelSyncer for ConcreteModelSyncer<P, F>
where
    P: Clone + 'static,
    F: Fn(i32, SharedString, slint::Color, f32) -> P,
{
    fn sync(&self, paths: &[LinkPathData]) {
        // Update existing rows or add new ones
        for (i, path) in paths.iter().enumerate() {
            let item = (self.constructor)(path.id, SharedString::from(path.path_commands.as_str()), path.color, path.line_width);
            if i < self.model.row_count() {
                self.model.set_row_data(i, item);
            } else {
                self.model.push(item);
            }
        }
        // Remove excess rows
        while self.model.row_count() > paths.len() {
            self.model.remove(self.model.row_count() - 1);
        }
    }
}

/// Manages links and their visual paths for the node editor.
///
/// `LinkManager` maintains a collection of links and automatically computes
/// their bezier path representations for rendering.
///
/// # Auto-Sync Mode
///
/// Use [`bind_model`](Self::bind_model) to enable automatic synchronization
/// to a Slint `VecModel`. After binding, every call to [`update_paths`](Self::update_paths)
/// automatically updates the Slint model.
///
/// ```ignore
/// let mut links = LinkManager::new(cache);
/// links.add(SimpleLink::new(1, start_pin, end_pin, color));
///
/// // Bind once - auto-syncs on every update_paths call
/// let model = Rc::new(VecModel::<LinkPath>::default());
/// links.bind_model(model.clone(), |id, path, color| LinkPath { id, path_commands: path, color });
/// window.set_link_paths(ModelRc::from(model));
///
/// // Now just call update_paths - model syncs automatically
/// links.update_paths(zoom);
/// ```
///
/// # Type Parameters
///
/// - `L`: The link data type, must implement [`LinkModel`]
/// - `N`: The node geometry type used by the cache (default: `SimpleNodeGeometry`)
pub struct LinkManager<L, N = crate::hit_test::SimpleNodeGeometry> {
    /// The logical link data
    links: Vec<L>,
    /// Reference to the geometry cache for position lookups
    cache: Rc<RefCell<GeometryCache<N>>>,
    /// The computed paths (id, path_commands, color) for Slint binding
    paths: Rc<RefCell<Vec<LinkPathData>>>,
    /// Current zoom level (cached for updates)
    current_zoom: f32,
    /// Bezier curve offset
    bezier_offset: f32,
    /// Optional auto-sync to Slint model
    syncer: Option<Box<dyn ModelSyncer>>,
}

/// Internal representation of a link path.
#[derive(Clone)]
struct LinkPathData {
    id: i32,
    path_commands: String,
    color: slint::Color,
    line_width: f32,
}

impl<L, N> LinkManager<L, N>
where
    L: LinkModel,
    N: NodeGeometry + Copy,
{
    /// Create a new LinkManager with the given geometry cache.
    ///
    /// # Arguments
    ///
    /// * `cache` - Reference to the geometry cache for pin position lookups
    pub fn new(cache: Rc<RefCell<GeometryCache<N>>>) -> Self {
        Self {
            links: Vec::new(),
            cache,
            paths: Rc::new(RefCell::new(Vec::new())),
            current_zoom: 1.0,
            bezier_offset: 50.0,
            syncer: None,
        }
    }

    /// Bind to a Slint model for automatic synchronization.
    ///
    /// After binding, every call to [`update_paths`] automatically
    /// updates the Slint model.
    ///
    /// # Arguments
    ///
    /// * `model` - The VecModel to sync to
    /// * `constructor` - Function to create path items from (id, path_commands, color, line_width)
    pub fn bind_model<P, F>(&mut self, model: Rc<VecModel<P>>, constructor: F)
    where
        P: Clone + 'static,
        F: Fn(i32, SharedString, slint::Color, f32) -> P + 'static,
    {
        self.syncer = Some(Box::new(ConcreteModelSyncer { model, constructor }));
    }

    /// Add a link to the manager.
    ///
    /// The link's visual path will be computed on the next call to [`update_paths`].
    pub fn add(&mut self, link: L) {
        self.links.push(link);
    }

    /// Remove a link by ID.
    ///
    /// Returns `true` if a link was removed.
    pub fn remove(&mut self, id: i32) -> bool {
        let len_before = self.links.len();
        self.links.retain(|link| link.id() != id);
        self.links.len() != len_before
    }

    /// Remove all links.
    pub fn clear(&mut self) {
        self.links.clear();
        self.paths.borrow_mut().clear();
    }

    /// Get the number of links.
    pub fn len(&self) -> usize {
        self.links.len()
    }

    /// Check if there are no links.
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// Get a reference to the links.
    pub fn links(&self) -> &[L] {
        &self.links
    }

    /// Get a mutable reference to the links.
    pub fn links_mut(&mut self) -> &mut Vec<L> {
        &mut self.links
    }

    /// Set the bezier curve offset (default: 50.0).
    ///
    /// This controls how curved the link paths are.
    pub fn set_bezier_offset(&mut self, offset: f32) {
        self.bezier_offset = offset;
    }

    /// Update all link paths based on current pin positions.
    ///
    /// Call this whenever:
    /// - Pin positions change (node moved)
    /// - Zoom level changes
    /// - Links are added/removed
    ///
    /// # Arguments
    ///
    /// * `zoom` - Current viewport zoom level
    pub fn update_paths(&mut self, zoom: f32) {
        self.current_zoom = zoom;
        let cache = self.cache.borrow();
        let mut paths = self.paths.borrow_mut();
        paths.clear();

        for link in &self.links {
            if let Some(path) = cache.compute_link_path(
                link.start_pin_id(),
                link.end_pin_id(),
                zoom,
                self.bezier_offset,
            ) {
                paths.push(LinkPathData {
                    id: link.id(),
                    path_commands: path,
                    color: link.color(),
                    line_width: link.line_width(),
                });
            }
        }

        // Auto-sync to bound model if present
        if let Some(syncer) = &self.syncer {
            syncer.sync(&paths);
        }
    }

    /// Update paths with a specific bezier offset (one-time override).
    pub fn update_paths_with_offset(&mut self, zoom: f32, bezier_offset: f32) {
        self.bezier_offset = bezier_offset;
        self.update_paths(zoom);
    }

    /// Get an iterator over link IDs.
    pub fn ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.links.iter().map(|l| l.id())
    }

    /// Find a link by ID.
    pub fn find(&self, id: i32) -> Option<&L> {
        self.links.iter().find(|l| l.id() == id)
    }

    /// Find a link by ID (mutable).
    pub fn find_mut(&mut self, id: i32) -> Option<&mut L> {
        self.links.iter_mut().find(|l| l.id() == id)
    }
}

/// Trait for creating Slint-compatible LinkPath models.
///
/// This trait is implemented by LinkManager and allows it to produce
/// a model that can be bound to Slint's link rendering.
pub trait LinkPathProvider {
    /// Create a Slint-compatible model of link paths.
    ///
    /// The returned closure takes a constructor function that creates
    /// the Slint LinkPath type from (id, path_commands, color, line_width).
    fn create_paths_model<P, F>(&self, constructor: F) -> ModelRc<P>
    where
        P: Clone + 'static,
        F: Fn(i32, slint::SharedString, slint::Color, f32) -> P + 'static;

    /// Update an existing paths model in place.
    ///
    /// This is more efficient than creating a new model each time.
    fn update_paths_model<P, F>(&self, model: &VecModel<P>, constructor: F)
    where
        P: Clone + 'static,
        F: Fn(i32, slint::SharedString, slint::Color, f32) -> P;
}

impl<L, N> LinkPathProvider for LinkManager<L, N>
where
    L: LinkModel,
    N: NodeGeometry + Copy,
{
    fn create_paths_model<P, F>(&self, constructor: F) -> ModelRc<P>
    where
        P: Clone + 'static,
        F: Fn(i32, slint::SharedString, slint::Color, f32) -> P + 'static,
    {
        let paths = self.paths.borrow();
        let items: Vec<P> = paths
            .iter()
            .map(|p| constructor(p.id, SharedString::from(p.path_commands.as_str()), p.color, p.line_width))
            .collect();
        ModelRc::from(Rc::new(VecModel::from(items)))
    }

    fn update_paths_model<P, F>(&self, model: &VecModel<P>, constructor: F)
    where
        P: Clone + 'static,
        F: Fn(i32, slint::SharedString, slint::Color, f32) -> P,
    {
        let paths = self.paths.borrow();

        // Update existing rows or add new ones
        for (i, path) in paths.iter().enumerate() {
            let item = constructor(path.id, SharedString::from(path.path_commands.as_str()), path.color, path.line_width);
            if i < model.row_count() {
                model.set_row_data(i, item);
            } else {
                model.push(item);
            }
        }

        // Remove excess rows
        while model.row_count() > paths.len() {
            model.remove(model.row_count() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::SimpleLink;
    use crate::hit_test::SimpleNodeGeometry;
    use slint::Color;

    fn setup_cache() -> Rc<RefCell<GeometryCache<SimpleNodeGeometry>>> {
        let cache = Rc::new(RefCell::new(GeometryCache::new()));

        // Add two nodes
        cache.borrow_mut().update_node_rect(1, 0.0, 0.0, 100.0, 50.0);
        cache.borrow_mut().update_node_rect(2, 200.0, 100.0, 100.0, 50.0);

        // Add pins (output on node 1, input on node 2)
        cache.borrow_mut().handle_pin_report(3, 1, 2, 100.0, 25.0); // Node 1 output
        cache.borrow_mut().handle_pin_report(4, 2, 1, 0.0, 25.0);   // Node 2 input

        cache
    }

    #[test]
    fn test_new_manager_is_empty() {
        let cache = setup_cache();
        let manager: LinkManager<SimpleLink, _> = LinkManager::new(cache);
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_add_link() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 0, 0)));

        assert_eq!(manager.len(), 1);
        assert!(!manager.is_empty());
    }

    #[test]
    fn test_remove_link() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 0, 0)));
        manager.add(SimpleLink::new(2, 3, 4, Color::from_rgb_u8(0, 255, 0)));

        assert!(manager.remove(1));
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.links()[0].id(), 2);
    }

    #[test]
    fn test_remove_nonexistent_link() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 0, 0)));

        assert!(!manager.remove(999));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_clear() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 0, 0)));
        manager.add(SimpleLink::new(2, 3, 4, Color::from_rgb_u8(0, 255, 0)));

        manager.clear();

        assert!(manager.is_empty());
    }

    #[test]
    fn test_update_paths() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 0, 0)));
        manager.update_paths(1.0);

        let paths = manager.paths.borrow();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].id, 1);
        assert!(paths[0].path_commands.starts_with("M "));
    }

    #[test]
    fn test_update_paths_missing_pin() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        // Link with non-existent pin
        manager.add(SimpleLink::new(1, 3, 999, Color::from_rgb_u8(255, 0, 0)));
        manager.update_paths(1.0);

        // Should produce no path
        let paths = manager.paths.borrow();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_find() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 0, 0)));
        manager.add(SimpleLink::new(2, 3, 4, Color::from_rgb_u8(0, 255, 0)));

        let link = manager.find(2);
        assert!(link.is_some());
        assert_eq!(link.unwrap().id(), 2);

        assert!(manager.find(999).is_none());
    }

    #[test]
    fn test_ids() {
        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(SimpleLink::new(1, 3, 4, Color::from_rgb_u8(255, 0, 0)));
        manager.add(SimpleLink::new(5, 3, 4, Color::from_rgb_u8(0, 255, 0)));
        manager.add(SimpleLink::new(3, 3, 4, Color::from_rgb_u8(0, 0, 255)));

        let ids: Vec<i32> = manager.ids().collect();
        assert_eq!(ids, vec![1, 5, 3]);
    }

    #[test]
    fn test_custom_link_type() {
        #[derive(Clone)]
        struct MyLink {
            id: i32,
            from: i32,
            to: i32,
            label: String,
        }

        impl LinkModel for MyLink {
            fn id(&self) -> i32 { self.id }
            fn start_pin_id(&self) -> i32 { self.from }
            fn end_pin_id(&self) -> i32 { self.to }
            fn color(&self) -> Color { Color::from_rgb_u8(128, 128, 128) }
        }

        let cache = setup_cache();
        let mut manager = LinkManager::new(cache);

        manager.add(MyLink {
            id: 1,
            from: 3,
            to: 4,
            label: "data flow".to_string(),
        });

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.links()[0].label, "data flow");
    }
}
