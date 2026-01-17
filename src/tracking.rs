//! Convenience helpers for geometry tracking setup.
//!
//! This module provides [`GeometryTracker`], a wrapper around [`GeometryCache`]
//! that simplifies wiring up Slint callbacks for position tracking.
//!
//! # Example
//!
//! ```ignore
//! use slint_node_editor::GeometryTracker;
//!
//! let tracker = GeometryTracker::new();
//!
//! // Wire up callbacks (one-time setup)
//! window.on_node_rect_changed(tracker.node_rect_callback());
//! window.on_pin_position_changed(tracker.pin_position_callback());
//!
//! // Get the cache for use elsewhere
//! let cache = tracker.cache();
//! ```

use crate::hit_test::{NodeGeometry, SimpleNodeGeometry};
use crate::state::GeometryCache;
use std::cell::RefCell;
use std::rc::Rc;

/// Convenience wrapper for [`GeometryCache`] that provides ready-to-use Slint callbacks.
///
/// This eliminates the boilerplate of creating a cache, wrapping it in `Rc<RefCell<_>>`,
/// and manually wiring up the position tracking callbacks.
///
/// # Type Parameter
///
/// - `N`: The node geometry type. Defaults to [`SimpleNodeGeometry`] which stores
///   basic rectangle data (x, y, width, height). For custom node data, implement
///   [`NodeGeometry`] and use `GeometryTracker::<MyNodeGeometry>::new()`.
///
/// # Example
///
/// ```ignore
/// // Create tracker with default SimpleNodeGeometry
/// let tracker = GeometryTracker::new();
///
/// // Wire up Slint callbacks
/// window.on_node_rect_changed(tracker.node_rect_callback());
/// window.on_pin_position_changed(tracker.pin_position_callback());
///
/// // Access the cache for hit testing, path computation, etc.
/// let cache = tracker.cache();
/// let pin_id = cache.borrow().find_pin_at(x, y, 10.0);
/// ```
pub struct GeometryTracker<N = SimpleNodeGeometry> {
    cache: Rc<RefCell<GeometryCache<N>>>,
}

impl<N> Default for GeometryTracker<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N> GeometryTracker<N> {
    /// Create a new geometry tracker with an empty cache.
    pub fn new() -> Self {
        Self {
            cache: Rc::new(RefCell::new(GeometryCache::new())),
        }
    }

    /// Create a tracker wrapping an existing cache.
    ///
    /// Useful when you need to initialize the cache with data before
    /// connecting callbacks.
    pub fn with_cache(cache: Rc<RefCell<GeometryCache<N>>>) -> Self {
        Self { cache }
    }

    /// Get a clone of the internal cache reference.
    ///
    /// Use this to access the cache for hit testing, path computation,
    /// and other operations.
    pub fn cache(&self) -> Rc<RefCell<GeometryCache<N>>> {
        self.cache.clone()
    }
}

impl<N> GeometryTracker<N>
where
    N: NodeGeometry + Copy,
{
    /// Get a callback for pin position updates.
    ///
    /// Wire this to the Slint `on_pin_position_changed` callback:
    ///
    /// ```ignore
    /// window.on_pin_position_changed(tracker.pin_position_callback());
    /// ```
    ///
    /// The callback signature matches Slint's generated callback:
    /// `(pin_id: i32, node_id: i32, pin_type: i32, rel_x: f32, rel_y: f32)`
    pub fn pin_position_callback(&self) -> impl Fn(i32, i32, i32, f32, f32) + Clone {
        let cache = self.cache.clone();
        move |pin_id, node_id, pin_type, rel_x, rel_y| {
            cache
                .borrow_mut()
                .handle_pin_report(pin_id, node_id, pin_type, rel_x, rel_y);
        }
    }
}

impl GeometryTracker<SimpleNodeGeometry> {
    /// Get a callback for node rectangle updates.
    ///
    /// Wire this to the Slint `on_node_rect_changed` callback:
    ///
    /// ```ignore
    /// window.on_node_rect_changed(tracker.node_rect_callback());
    /// ```
    ///
    /// The callback signature matches Slint's generated callback:
    /// `(id: i32, x: f32, y: f32, width: f32, height: f32)`
    ///
    /// # Note
    ///
    /// This method is only available for `GeometryTracker<SimpleNodeGeometry>`.
    /// For custom node types, use [`node_rect_callback_with`] instead.
    pub fn node_rect_callback(&self) -> impl Fn(i32, f32, f32, f32, f32) + Clone {
        let cache = self.cache.clone();
        move |id, x, y, width, height| {
            cache
                .borrow_mut()
                .handle_node_rect_report(id, x, y, width, height);
        }
    }
}

impl<N> GeometryTracker<N>
where
    N: NodeGeometry + Copy,
{
    /// Get a callback for node rectangle updates with a custom conversion function.
    ///
    /// Use this when your cache uses a custom `NodeGeometry` type instead of
    /// `SimpleNodeGeometry`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(Clone, Copy)]
    /// struct MyNodeGeometry {
    ///     id: i32,
    ///     bounds: (f32, f32, f32, f32),
    ///     collapsed: bool,
    /// }
    ///
    /// impl NodeGeometry for MyNodeGeometry {
    ///     fn id(&self) -> i32 { self.id }
    ///     fn rect(&self) -> (f32, f32, f32, f32) { self.bounds }
    /// }
    ///
    /// let tracker = GeometryTracker::<MyNodeGeometry>::new();
    /// window.on_node_rect_changed(tracker.node_rect_callback_with(|id, x, y, w, h| {
    ///     MyNodeGeometry {
    ///         id,
    ///         bounds: (x, y, w, h),
    ///         collapsed: false,
    ///     }
    /// }));
    /// ```
    pub fn node_rect_callback_with<F>(&self, convert: F) -> impl Fn(i32, f32, f32, f32, f32) + Clone
    where
        F: Fn(i32, f32, f32, f32, f32) -> N + Clone + 'static,
    {
        let cache = self.cache.clone();
        move |id, x, y, width, height| {
            let node = convert(id, x, y, width, height);
            cache.borrow_mut().node_rects.insert(id, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hit_test::SimpleNodeGeometry;

    #[test]
    fn test_tracker_new_creates_empty_cache() {
        let tracker: GeometryTracker<SimpleNodeGeometry> = GeometryTracker::new();
        let cache = tracker.cache();
        assert!(cache.borrow().node_rects.is_empty());
        assert!(cache.borrow().pin_positions.is_empty());
    }

    #[test]
    fn test_tracker_default_creates_empty_cache() {
        let tracker: GeometryTracker<SimpleNodeGeometry> = GeometryTracker::default();
        let cache = tracker.cache();
        assert!(cache.borrow().node_rects.is_empty());
    }

    #[test]
    fn test_tracker_with_existing_cache() {
        let cache = Rc::new(RefCell::new(GeometryCache::<SimpleNodeGeometry>::new()));
        cache.borrow_mut().update_node_rect(1, 10.0, 20.0, 100.0, 50.0);

        let tracker = GeometryTracker::with_cache(cache.clone());

        // Should share the same cache
        assert!(tracker.cache().borrow().node_rects.contains_key(&1));
    }

    #[test]
    fn test_node_rect_callback_updates_cache() {
        let tracker = GeometryTracker::new();
        let callback = tracker.node_rect_callback();

        callback(1, 10.0, 20.0, 100.0, 50.0);

        let cache = tracker.cache();
        let node = cache.borrow().node_rects.get(&1).copied();
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.id, 1);
        assert_eq!(node.x, 10.0);
        assert_eq!(node.y, 20.0);
        assert_eq!(node.width, 100.0);
        assert_eq!(node.height, 50.0);
    }

    #[test]
    fn test_pin_position_callback_updates_cache() {
        let tracker: GeometryTracker<SimpleNodeGeometry> = GeometryTracker::new();
        let callback = tracker.pin_position_callback();

        callback(1001, 1, 2, 50.0, 25.0);

        let cache = tracker.cache();
        let pin = cache.borrow().pin_positions.get(&1001).copied();
        assert!(pin.is_some());
        let pin = pin.unwrap();
        assert_eq!(pin.node_id, 1);
        assert_eq!(pin.pin_type, 2);
        assert_eq!(pin.rel_x, 50.0);
        assert_eq!(pin.rel_y, 25.0);
    }

    #[test]
    fn test_callbacks_share_same_cache() {
        let tracker = GeometryTracker::new();

        let node_cb = tracker.node_rect_callback();
        let pin_cb = tracker.pin_position_callback();

        node_cb(1, 0.0, 0.0, 100.0, 50.0);
        pin_cb(1001, 1, 2, 50.0, 25.0);

        let cache = tracker.cache();
        assert_eq!(cache.borrow().node_rects.len(), 1);
        assert_eq!(cache.borrow().pin_positions.len(), 1);
    }

    #[test]
    fn test_multiple_cache_clones_share_data() {
        let tracker = GeometryTracker::new();

        let cache1 = tracker.cache();
        let cache2 = tracker.cache();

        cache1.borrow_mut().update_node_rect(1, 0.0, 0.0, 100.0, 50.0);

        // cache2 should see the update
        assert!(cache2.borrow().node_rects.contains_key(&1));
    }

    #[test]
    fn test_callback_is_clone() {
        let tracker = GeometryTracker::new();

        let cb1 = tracker.node_rect_callback();
        let cb2 = cb1.clone();

        cb1(1, 0.0, 0.0, 100.0, 50.0);
        cb2(2, 100.0, 0.0, 100.0, 50.0);

        let cache = tracker.cache();
        assert_eq!(cache.borrow().node_rects.len(), 2);
    }

    #[test]
    fn test_custom_node_geometry() {
        #[derive(Clone, Copy)]
        struct CustomNode {
            id: i32,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            extra: bool,
        }

        impl NodeGeometry for CustomNode {
            fn id(&self) -> i32 {
                self.id
            }
            fn rect(&self) -> (f32, f32, f32, f32) {
                (self.x, self.y, self.w, self.h)
            }
        }

        let tracker = GeometryTracker::<CustomNode>::new();
        let callback = tracker.node_rect_callback_with(|id, x, y, w, h| CustomNode {
            id,
            x,
            y,
            w,
            h,
            extra: true,
        });

        callback(1, 10.0, 20.0, 100.0, 50.0);

        let cache = tracker.cache();
        let node = cache.borrow().node_rects.get(&1).copied();
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.id, 1);
        assert!(node.extra);
    }
}
