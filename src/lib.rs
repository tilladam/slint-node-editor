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
    ValidationResult, ValidationError, validate_link,
};
pub use tracking::GeometryTracker;
pub use links::{LinkManager, LinkPathProvider};
pub use controller::NodeEditorController;