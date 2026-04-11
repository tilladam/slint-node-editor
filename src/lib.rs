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
//! - [`SelectionManager`] - Manage selection state with O(1) lookups
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

pub mod grid;
pub mod path;
pub mod hit_test;
pub mod state;
pub mod selection;
pub mod graph;
pub mod tracking;
pub mod links;
pub mod controller;
pub mod setup;
#[cfg(feature = "layout")]
pub mod layout;

// Re-export traits and functions
pub use hit_test::{
    find_link_at, find_pin_at, links_in_selection_box, nodes_in_selection_box, LinkGeometry,
    NodeGeometry, PinGeometry, SimpleLinkGeometry, SimpleNodeGeometry,
};
pub use grid::generate_grid_commands;
pub use path::{generate_bezier_path, generate_partial_bezier_path};
pub use state::{GeometryCache, StoredPin};
pub use selection::SelectionManager;
pub use graph::{
    GraphLogic, LinkModel, MovableNode, SimpleLink,
    // Link validation framework
    LinkValidator, BasicLinkValidator, NoDuplicatesValidator, CompositeValidator,
    ValidationResult, ValidationError,
};
pub use tracking::GeometryTracker;
pub use links::LinkManager;
pub use controller::NodeEditorController;
pub use setup::NodeEditorSetup;
#[cfg(feature = "layout")]
pub use layout::{sugiyama_layout, sugiyama_layout_from_cache, Direction, NodePosition, SugiyamaConfig};

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
        gc.on_start_node_drag($setup.start_node_drag());
        gc.on_end_node_drag($setup.end_node_drag());

        // Computations
        let computations = $window.global::<NodeEditorComputations>();
        computations.on_compute_link_path($setup.controller().compute_link_path_callback());

        let ctrl = $setup.controller().clone();
        computations.on_compute_pin_at(move |x, y, radius| {
            ctrl.cache().borrow().find_pin_at(x, y, radius)
        });

        computations.on_compute_link_preview_path(|start_x, start_y, end_x, end_y, zoom, bezier_offset| {
            $crate::generate_bezier_path(start_x, start_y, end_x, end_y, zoom, bezier_offset).into()
        });

        let ctrl = $setup.controller().clone();
        computations.on_compute_box_selection(move |x, y, w, h| {
            let ids = ctrl.cache().borrow().nodes_in_selection_box(x, y, w, h);
            ids.as_slice().into()
        });

        // Selection state tracking
        let selection_set = $setup.selection();

        let sm_check = selection_set.clone();
        computations.on_is_node_selected(move |id, _version| sm_check.borrow().contains(&id));

        let sm_update = selection_set.clone();
        computations.on_sync_selection_to_nodes(move |ids_model| {
            use slint::Model;
            let mut set = sm_update.borrow_mut();
            set.clear();
            for i in 0..ids_model.row_count() {
                if let Some(id) = ids_model.row_data(i) {
                    set.insert(id);
                }
            }
        });

        // Auto grid updates
        let ctrl = $setup.controller().clone();
        let w = $window.as_weak();
        computations.on_viewport_changed(move |zoom, pan_x, pan_y| {
            ctrl.set_viewport(zoom, pan_x, pan_y);
            if let Some(w) = w.upgrade() {
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
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
