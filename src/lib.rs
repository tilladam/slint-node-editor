//! # Slint Node Editor Library
//!
//! A flexible, generic Slint component library for building visual graph editors.
//! Supports data flow diagrams, state machines, shader graphs, and any visual
//! node-based interface.
//!
//! ## Features
//!
//! - **Generic Design** - Works with any node and link data structures
//! - **Trait-Based Architecture** - Zero coupling via `NodeGeometry` and `LinkModel` traits
//! - **Callback-Based Computation** - Delegates expensive operations to Rust for performance
//! - **Opaque Pin IDs** - Library never prescribes pin encoding; applications choose
//! - **Extensible** - Easy to customize pins, styling, node types, and behaviors
//!
//! ## Quick Start
//!
//! ```slint
//! import { NodeEditor, BaseNode, Pin, PinTypes } from "slint-node-editor/node-editor.slint";
//!
//! export component MainWindow inherits Window {
//!     NodeEditor {
//!         // Your nodes and links here
//!     }
//! }
//! ```
//!
//! ## Core Components
//!
//! - [`NodeEditor`] - Main graph editor component
//! - [`BaseNode`] - Base component for creating custom nodes
//! - [`Pin`] - Connection point component
//! - [`Link`] - Bezier curve link component
//! - [`Minimap`] - Bird's-eye view component
//!
//! ## Accessibility
//!
//! Library components include accessibility roles for screen readers and MCP
//! introspection. When creating custom node components, set `accessible-label`
//! to the node's display title:
//!
//! ```slint
//! component MyNode inherits BaseNode {
//!     in property <string> title;
//!     accessible-label: title;  // override default "Node <id>"
//! }
//! ```
//!
//! ## Rust Helpers
//!
//! This crate provides Rust helper functions for common operations:
//!
//! - [`generate_grid_commands`] - Generate SVG path for grid rendering
//! - [`generate_bezier_path`] - Generate SVG path for bezier curves
//! - [`find_pin_at`] - Hit-test pins at screen coordinates
//! - [`find_link_at`] - Hit-test links at screen coordinates
//! - [`GeometryCache`] - Cache node and pin geometry for fast lookups
//! - [`selection`] - Resolve selection gestures and project the result into rows
//! - [`GraphLogic`] - Helper for managing node graph state
//!
//! ## Limitations
//!
//! **One NodeEditor per window.** The library uses Slint globals (`ViewportState`,
//! `DragState`, `NodeEditorInternalCallbacks`, etc.) for internal communication between
//! `BaseNode`/`Pin` components and the `NodeEditor`. Since Slint globals are
//! window-level singletons, only one `NodeEditor` instance per `Window` is
//! supported. Multiple editors in separate windows work fine. This limitation
//! will be lifted once Slint introduces component-scoped globals.
//!
//! See the [README](https://github.com/slint-ui/slint/tree/master/examples/node-editor/slint-node-editor)
//! for detailed documentation and examples.

pub mod controller;
pub mod graph;
pub mod grid;
pub mod hit_test;
#[cfg(feature = "layout")]
pub mod layout;
pub mod links;
pub mod path;
pub mod selection;
pub mod setup;
pub mod state;
pub mod tracking;

// Re-export traits and functions
pub use grid::generate_grid_commands;
pub use hit_test::{
    find_link_at, find_pin_at, links_in_selection_box, nodes_in_selection_box, LinkGeometry,
    NodeGeometry, PinGeometry, SimpleLinkGeometry, SimpleNodeGeometry,
};
pub use path::{generate_bezier_path, generate_partial_bezier_path};
pub use state::{GeometryCache, StoredPin};
// `selection` is deliberately NOT re-exported at the crate root: it is the
// host's half of selection, a family of its own, and a bare `resolve_click`
// would say nothing about what it resolves.
pub use controller::NodeEditorController;
pub use graph::{
    BasicLinkValidator,
    CompositeValidator,
    GraphLogic,
    LinkModel,
    // Link validation framework
    LinkValidator,
    MovableNode,
    NoDuplicatesValidator,
    SimpleLink,
    ValidationError,
    ValidationResult,
};
#[cfg(feature = "layout")]
pub use layout::{
    sugiyama_layout, sugiyama_layout_from_cache, Direction, NodePosition, SugiyamaConfig,
};
pub use links::LinkManager;
pub use setup::NodeEditorSetup;
pub use tracking::GeometryTracker;

/// Wire up all NodeEditor callbacks with a single macro call.
///
/// This macro sets up default behavior for geometry tracking, computations, and grid updates.
/// You can override any callback after calling this macro - the last `.on_*()` call wins.
///
/// # Example
///
/// ```ignore
/// use slint_node_editor::{NodeEditorSetup, wire_node_editor};
///
/// let setup = NodeEditorSetup::new(|node_id, dx, dy| {
///     // Update your model
/// });
///
/// wire_node_editor!(window, setup);
///
/// // Override specific callbacks if needed:
/// // window.global::<NodeEditorComputations>().on_compute_pin_at(|x, y, radius| { ... });
/// ```
#[macro_export]
macro_rules! wire_node_editor {
    ($window:expr, $setup:expr) => {{
        // Geometry tracking
        let gc = $window.global::<NodeEditorInternalCallbacks>();
        gc.on_report_node_rect($setup.report_node_rect());
        gc.on_report_pin_position($setup.report_pin_position());
        gc.on_end_node_drag($setup.end_node_drag());

        // Computations
        let computations = $window.global::<NodeEditorComputations>();
        computations.on_compute_link_path($setup.controller().compute_link_path_callback());

        let ctrl = $setup.controller().clone();
        computations
            .on_compute_pin_at(move |x, y, radius| ctrl.cache().borrow().find_pin_at(x, y, radius));

        computations.on_compute_link_preview_path(
            |start_x, start_y, end_x, end_y, zoom, bezier_offset| {
                $crate::generate_bezier_path(start_x, start_y, end_x, end_y, zoom, bezier_offset)
                    .into()
            },
        );

        // Auto grid updates
        let ctrl = $setup.controller().clone();
        let w = $window.as_weak();
        computations.on_viewport_changed(move |zoom, pan_x, pan_y| {
            ctrl.set_viewport(zoom, pan_x, pan_y);
            if let Some(w) = w.upgrade() {
                w.set_grid_commands(ctrl.generate_grid(
                    w.get_width_(),
                    w.get_height_(),
                    pan_x,
                    pan_y,
                ));
            }
        });

        // Initial grid
        let ctrl = $setup.controller().clone();
        let w = $window.as_weak();
        if let Some(w) = w.upgrade() {
            w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
        }
    }};
}

/// Wire the editor's selection intents, using your model rows as the store.
///
/// The editor holds no selection state: it reports gestures (`node-selected`,
/// `select-link`, `selection-cleared`, `box-selection-committed`) and renders
/// whatever `selected` flag it finds on the rows. This macro closes that loop
/// without introducing a second copy of the selection — each gesture reads the
/// current set back out of the rows, resolves it through
/// [`resolve_click`] / [`resolve_box`], and projects the result in. There is
/// nothing to keep in sync, because there is only ever one record of what is
/// selected.
///
/// Rows need an `id` and a `selected` field.
///
/// Two arms: node selection alone, or nodes and links together. Applications
/// with several node models or their own gesture semantics wire the four
/// callbacks by hand out of the [`selection`] module — see the `advanced`
/// example.
///
/// [`resolve_click`]: crate::selection::resolve_click
/// [`resolve_box`]: crate::selection::resolve_box
/// [`selection`]: crate::selection
///
/// # Example
///
/// ```ignore
/// wire_node_editor!(window, setup);
/// wire_selection!(window, setup, nodes);
/// // …or, with selectable links:
/// wire_selection!(window, setup, nodes, links);
/// ```
#[macro_export]
macro_rules! wire_selection {
    ($window:expr, $setup:expr, $nodes:expr) => {{
        // `select-link` is deliberately not wired: the editor only emits it
        // when the host opts into `has-link-selection`, and a host doing that
        // wants the four-argument arm.

        $window.on_node_selected({
            let nodes = $nodes.clone();
            move |node_id, shift| {
                $crate::selection::apply_click(
                    &nodes,
                    |n| n.id,
                    |n| &mut n.selected,
                    node_id,
                    shift,
                )
            }
        });

        $window.on_selection_cleared({
            let nodes = $nodes.clone();
            move || $crate::selection::clear_selection(&nodes, |n| &mut n.selected)
        });

        $window.on_box_selection_committed({
            let nodes = $nodes.clone();
            let ctrl = $setup.controller().clone();
            move |x, y, w, h, shift| {
                let hits = ctrl.cache().borrow().nodes_in_selection_box(x, y, w, h);
                $crate::selection::apply_box(&nodes, |n| n.id, |n| &mut n.selected, hits, shift)
            }
        });
    }};

    ($window:expr, $setup:expr, $nodes:expr, $links:expr) => {{
        // Nodes and links are selected exclusively: picking one drops the other.
        $window.on_node_selected({
            let nodes = $nodes.clone();
            let links = $links.clone();
            move |node_id, shift| {
                $crate::selection::clear_selection(&links, |l| &mut l.selected);
                $crate::selection::apply_click(
                    &nodes,
                    |n| n.id,
                    |n| &mut n.selected,
                    node_id,
                    shift,
                )
            }
        });

        $window.on_select_link({
            let nodes = $nodes.clone();
            let links = $links.clone();
            move |link_id, shift| {
                $crate::selection::clear_selection(&nodes, |n| &mut n.selected);
                $crate::selection::apply_click(
                    &links,
                    |l| l.id,
                    |l| &mut l.selected,
                    link_id,
                    shift,
                )
            }
        });

        $window.on_selection_cleared({
            let nodes = $nodes.clone();
            let links = $links.clone();
            move || {
                $crate::selection::clear_selection(&nodes, |n| &mut n.selected);
                $crate::selection::clear_selection(&links, |l| &mut l.selected);
            }
        });

        $window.on_box_selection_committed({
            let nodes = $nodes.clone();
            let links = $links.clone();
            let ctrl = $setup.controller().clone();
            move |x, y, w, h, shift| {
                let (node_hits, link_hits) = {
                    let cache = ctrl.cache();
                    let cache = cache.borrow();
                    let rows = (0..slint::Model::row_count(&*links))
                        .filter_map(|i| slint::Model::row_data(&*links, i))
                        .map(|l| (l.id, l.start_pin_id, l.end_pin_id));
                    (
                        cache.nodes_in_selection_box(x, y, w, h),
                        cache.links_in_selection_box(x, y, w, h, rows),
                    )
                };
                $crate::selection::apply_box(
                    &nodes,
                    |n| n.id,
                    |n| &mut n.selected,
                    node_hits,
                    shift,
                );
                $crate::selection::apply_box(
                    &links,
                    |l| l.id,
                    |l| &mut l.selected,
                    link_hits,
                    shift,
                )
            }
        });
    }};
}
