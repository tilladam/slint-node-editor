//! Pin Compatibility Example
//!
//! Demonstrates the validation framework with a complex type compatibility matrix.
//! Two nodes with 8 pin types each, showing which types can connect.
//!
//! Type Compatibility Matrix:
//! - Execute: Only connects to Execute (flow control)
//! - Integer: Connects to Integer, Float, String, Any
//! - Float: Connects to Float, String, Any
//! - String: Connects to String, Any
//! - Boolean: Connects to Boolean, Integer, Any
//! - Object: Connects to Object, Any
//! - Array: Connects to Array, Any
//! - Any: Connects to all types

use slint::{Color, Model, ModelRc, SharedString, VecModel};
use slint_node_editor::{
    BasicLinkValidator, CompositeValidator, GeometryCache, LinkModel, LinkValidator,
    NodeEditorController, SimpleNodeGeometry, ValidationError, ValidationResult,
};
use std::rc::Rc;

slint::include_modules!();

/// Data type constants matching the Slint DataTypes global
mod data_types {
    pub const EXECUTE: i32 = 0;
    pub const INTEGER: i32 = 1;
    pub const FLOAT: i32 = 2;
    pub const STRING: i32 = 3;
    pub const BOOLEAN: i32 = 4;
    pub const OBJECT: i32 = 5;
    pub const ARRAY: i32 = 6;
    pub const ANY: i32 = 7;
}

/// Extract data type from pin ID.
/// Pin IDs are encoded as: base + data_type
/// - Source node (outputs): 100 + data_type
/// - Sink node (inputs): 200 + data_type
fn get_data_type(pin_id: i32) -> i32 {
    pin_id % 100
}

/// Check if a data type can be connected from source to target.
/// This implements the type compatibility matrix.
fn types_compatible(source_type: i32, target_type: i32) -> bool {
    use data_types::*;

    // Any accepts everything
    if target_type == ANY {
        return true;
    }

    // Any can send to everything
    if source_type == ANY {
        return true;
    }

    match source_type {
        EXECUTE => target_type == EXECUTE,
        INTEGER => matches!(target_type, INTEGER | FLOAT | STRING),
        FLOAT => matches!(target_type, FLOAT | STRING),
        STRING => target_type == STRING,
        BOOLEAN => matches!(target_type, BOOLEAN | INTEGER),
        OBJECT => target_type == OBJECT,
        ARRAY => target_type == ARRAY,
        _ => false,
    }
}

/// Get a color for a link based on the output pin's data type
fn get_link_color(output_pin_id: i32) -> Color {
    use data_types::*;
    match get_data_type(output_pin_id) {
        EXECUTE => Color::from_rgb_u8(255, 255, 255), // White
        INTEGER => Color::from_rgb_u8(79, 195, 247),  // Cyan
        FLOAT => Color::from_rgb_u8(129, 199, 132),   // Green
        STRING => Color::from_rgb_u8(255, 183, 77),   // Orange
        BOOLEAN => Color::from_rgb_u8(229, 115, 115), // Red
        OBJECT => Color::from_rgb_u8(186, 104, 200),  // Purple
        ARRAY => Color::from_rgb_u8(240, 98, 146),    // Pink
        ANY => Color::from_rgb_u8(144, 164, 174),     // Gray
        _ => Color::from_rgb_u8(200, 200, 200),       // Default
    }
}

/// Custom validator that checks type compatibility between pins.
#[derive(Clone, Copy, Debug)]
pub struct TypeCompatibilityValidator;

impl<N, L> LinkValidator<N, L> for TypeCompatibilityValidator {
    fn validate(
        &self,
        start_pin: i32,
        end_pin: i32,
        cache: &GeometryCache<N>,
        _links: &[L],
    ) -> ValidationResult {
        // Get pin info from cache to determine which is output and which is input
        let start_info = cache.pin_positions.get(&start_pin);
        let end_info = cache.pin_positions.get(&end_pin);

        let (start_info, _end_info) = match (start_info, end_info) {
            (Some(s), Some(e)) => (s, e),
            _ => return ValidationResult::Invalid(ValidationError::PinNotFound(start_pin)),
        };

        // Determine which pin is output (type 2) and which is input (type 1)
        let (output_pin, input_pin) = if start_info.pin_type == 2 {
            (start_pin, end_pin)
        } else {
            (end_pin, start_pin)
        };

        let source_type = get_data_type(output_pin);
        let target_type = get_data_type(input_pin);

        if types_compatible(source_type, target_type) {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(ValidationError::TypeMismatch {
                expected: target_type,
                found: source_type,
            })
        }
    }
}

/// Get type name for display
fn type_name(data_type: i32) -> &'static str {
    use data_types::*;
    match data_type {
        EXECUTE => "Execute",
        INTEGER => "Integer",
        FLOAT => "Float",
        STRING => "String",
        BOOLEAN => "Boolean",
        OBJECT => "Object",
        ARRAY => "Array",
        ANY => "Any",
        _ => "Unknown",
    }
}

impl LinkModel for LinkData {
    fn id(&self) -> i32 {
        self.id
    }
    fn start_pin_id(&self) -> i32 {
        self.start_pin_id
    }
    fn end_pin_id(&self) -> i32 {
        self.end_pin_id
    }
    fn color(&self) -> Color {
        self.color
    }
    fn line_width(&self) -> f32 {
        self.line_width
    }
}

fn main() {
    let window = MainWindow::new().unwrap();
    let ctrl = NodeEditorController::new();
    let w = window.as_weak();

    // Set up nodes
    let nodes = Rc::new(VecModel::from(vec![
        NodeData {
            id: 1,
            title: SharedString::from("Source"),
            x: 150.0,
            y: 150.0,
        },
        NodeData {
            id: 2,
            title: SharedString::from("Sink"),
            x: 550.0,
            y: 150.0,
        },
    ]));
    window.set_nodes(ModelRc::from(nodes.clone()));

    // Set up links (start empty)
    let links = Rc::new(VecModel::<LinkData>::default());
    window.set_links(ModelRc::from(links.clone()));

    // Link ID counter
    let next_link_id = Rc::new(std::cell::Cell::new(1));

    // Core callbacks
    window.on_compute_link_path({
        let ctrl = ctrl.clone();
        let w = window.as_weak();
        move |start_pin, end_pin, _version| {
            let w = match w.upgrade() {
                Some(w) => w,
                None => return SharedString::default(),
            };
            ctrl.cache()
                .borrow()
                .compute_link_path(start_pin, end_pin, w.get_zoom(), 50.0)
                .unwrap_or_default()
                .into()
        }
    });
    window.on_node_drag_started(ctrl.node_drag_started_callback());

    // Pin hit detection for link completion
    window.on_compute_pin_at({
        let ctrl = ctrl.clone();
        move |x, y| {
            ctrl.cache().borrow().find_pin_at(x as f32, y as f32, 20.0)
        }
    });

    // Link validation callback for hover feedback
    window.on_validate_link({
        let ctrl = ctrl.clone();
        let links = links.clone();
        move |start_pin, end_pin| {
            let cache = ctrl.cache();
            let cache = cache.borrow();

            // Create composite validator with basic checks + type compatibility
            let validator: CompositeValidator<SimpleNodeGeometry, LinkData> =
                CompositeValidator::new()
                    .with(BasicLinkValidator::new(2)) // 2 = output pin type
                    .with(TypeCompatibilityValidator);

            let links_vec: Vec<LinkData> = links.iter().collect();
            matches!(
                validator.validate(start_pin, end_pin, &cache, &links_vec),
                ValidationResult::Valid
            )
        }
    });

    // Link preview path generation
    window.on_compute_link_preview_path({
        let w = window.as_weak();
        move |start_x, start_y, end_x, end_y| {
            let w = match w.upgrade() {
                Some(w) => w,
                None => return SharedString::default(),
            };
            slint_node_editor::generate_bezier_path(
                start_x as f32,
                start_y as f32,
                end_x as f32,
                end_y as f32,
                w.get_zoom(),
                50.0,
            )
            .into()
        }
    });

    // Geometry tracking
    window.on_node_rect_changed({
        let ctrl = ctrl.clone();
        move |id, x, y, width, h| {
            ctrl.handle_node_rect(id, x, y, width, h);
        }
    });

    window.on_pin_position_changed({
        let ctrl = ctrl.clone();
        move |pid, nid, ptype, x, y| {
            ctrl.handle_pin_position(pid, nid, ptype, x, y);
        }
    });

    // Link requested - validate and create
    window.on_link_requested({
        let ctrl = ctrl.clone();
        let links = links.clone();
        let next_link_id = next_link_id.clone();
        move |start_pin, end_pin| {
            println!("link_requested: {} -> {}", start_pin, end_pin);
            let cache = ctrl.cache();
            let cache = cache.borrow();

            // Create composite validator with basic checks + type compatibility
            let validator: CompositeValidator<SimpleNodeGeometry, LinkData> =
                CompositeValidator::new()
                    .with(BasicLinkValidator::new(2)) // 2 = output pin type
                    .with(TypeCompatibilityValidator);

            let links_vec: Vec<LinkData> = links.iter().collect();
            match validator.validate(start_pin, end_pin, &cache, &links_vec) {
                ValidationResult::Valid => {
                    // Determine output and input pins
                    let start_info = cache.pin_positions.get(&start_pin);
                    let (output_pin, input_pin) = if start_info.map(|s| s.pin_type) == Some(2) {
                        (start_pin, end_pin)
                    } else {
                        (end_pin, start_pin)
                    };

                    let link = LinkData {
                        id: next_link_id.get(),
                        start_pin_id: output_pin,
                        end_pin_id: input_pin,
                        color: get_link_color(output_pin),
                        line_width: 2.5,
                    };

                    println!(
                        "Link created: {} -> {} ({})",
                        type_name(get_data_type(output_pin)),
                        type_name(get_data_type(input_pin)),
                        link.id
                    );

                    links.push(link);
                    next_link_id.set(next_link_id.get() + 1);
                }
                ValidationResult::Invalid(err) => {
                    match &err {
                        ValidationError::TypeMismatch { expected, found } => {
                            println!(
                                "Cannot connect: {} is not compatible with {}",
                                type_name(*found),
                                type_name(*expected)
                            );
                        }
                        ValidationError::SameNode => {
                            println!("Cannot connect: pins are on the same node");
                        }
                        ValidationError::IncompatibleDirection => {
                            println!("Cannot connect: need one input and one output");
                        }
                        _ => {
                            println!("Cannot create link: {:?}", err);
                        }
                    }
                }
            }
        }
    });

    // Grid updates
    window.on_request_grid_update({
        let ctrl = ctrl.clone();
        let w = w.clone();
        move || {
            if let Some(w) = w.upgrade() {
                w.set_grid_commands(ctrl.generate_initial_grid(w.get_width_(), w.get_height_()));
            }
        }
    });

    window.on_update_viewport({
        let ctrl = ctrl.clone();
        let w = w.clone();
        move |z, pan_x, pan_y| {
            if let Some(w) = w.upgrade() {
                ctrl.set_zoom(z);
                w.set_grid_commands(ctrl.generate_grid(w.get_width_(), w.get_height_(), pan_x, pan_y));
            }
        }
    });

    // Node drag
    window.on_node_drag_ended({
        let ctrl = ctrl.clone();
        move |delta_x, delta_y| {
            let node_id = ctrl.dragged_node_id();
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

    // Print compatibility matrix on startup
    println!("\n=== Pin Type Compatibility Matrix ===");
    println!("Try connecting pins to see validation in action!\n");
    println!("Compatible connections:");
    println!("  Execute  -> Execute only");
    println!("  Integer  -> Integer, Float, String, Any");
    println!("  Float    -> Float, String, Any");
    println!("  String   -> String, Any");
    println!("  Boolean  -> Boolean, Integer, Any");
    println!("  Object   -> Object, Any");
    println!("  Array    -> Array, Any");
    println!("  Any      -> All types");
    println!("\nDrag from an output pin (right side) to an input pin (left side).\n");

    window.invoke_request_grid_update();
    window.run().unwrap();
}
