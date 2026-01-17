use crate::state::GeometryCache;
use crate::hit_test::{NodeGeometry, SimpleNodeGeometry};
use crate::selection::SelectionManager;
use slint::{Color, VecModel, Model};
use std::fmt;

/// Trait for link data to support graph topology and rendering operations.
///
/// Implement this trait for your link data type to use with [`LinkManager`]
/// and other graph operations.
///
/// # Example
///
/// ```ignore
/// struct MyLink {
///     id: i32,
///     from: i32,
///     to: i32,
///     color: Color,
///     label: String,  // custom field
/// }
///
/// impl LinkModel for MyLink {
///     fn id(&self) -> i32 { self.id }
///     fn start_pin_id(&self) -> i32 { self.from }
///     fn end_pin_id(&self) -> i32 { self.to }
///     fn color(&self) -> Color { self.color }
/// }
/// ```
pub trait LinkModel {
    /// Unique identifier for the link
    fn id(&self) -> i32;
    /// Pin ID where the link starts (typically an output pin)
    fn start_pin_id(&self) -> i32;
    /// Pin ID where the link ends (typically an input pin)
    fn end_pin_id(&self) -> i32;
    /// Color for rendering the link (default: white)
    fn color(&self) -> Color {
        Color::from_rgb_u8(255, 255, 255)
    }
}

/// Simple link data structure implementing [`LinkModel`].
///
/// Use this for basic link storage, or implement [`LinkModel`] on your own
/// type if you need additional fields.
#[derive(Clone, Debug)]
pub struct SimpleLink {
    pub id: i32,
    pub start_pin_id: i32,
    pub end_pin_id: i32,
    pub color: Color,
}

impl SimpleLink {
    /// Create a new link with the specified endpoints and color.
    pub fn new(id: i32, start_pin_id: i32, end_pin_id: i32, color: Color) -> Self {
        Self { id, start_pin_id, end_pin_id, color }
    }

    /// Create a new link with default white color.
    pub fn with_default_color(id: i32, start_pin_id: i32, end_pin_id: i32) -> Self {
        Self::new(id, start_pin_id, end_pin_id, Color::from_rgb_u8(255, 255, 255))
    }
}

impl LinkModel for SimpleLink {
    fn id(&self) -> i32 { self.id }
    fn start_pin_id(&self) -> i32 { self.start_pin_id }
    fn end_pin_id(&self) -> i32 { self.end_pin_id }
    fn color(&self) -> Color { self.color }
}

/// Trait for nodes that can be moved (dragged) in the editor.
/// This allows generic logic to update node positions.
pub trait MovableNode: Clone + 'static {
    fn id(&self) -> i32;
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn set_x(&mut self, x: f32);
    fn set_y(&mut self, y: f32);
}

/// Helper functions for graph operations
pub struct GraphLogic;

impl GraphLogic {
    /// Find all links connected to a specific node
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node being deleted/queried
    /// * `links` - Iterator over the links
    /// * `cache` - Geometry cache to look up pin ownership
    pub fn find_links_connected_to_node<'a, I, L, N>(
        node_id: i32,
        links: I,
        cache: &GeometryCache<N>,
    ) -> Vec<i32>
    where
        I: Iterator<Item = L>,
        L: LinkModel,
        N: NodeGeometry + Copy,
    {
        links
            .filter(|link| {
                let start_node = cache.pin_positions.get(&link.start_pin_id()).map(|p| p.node_id);
                let end_node = cache.pin_positions.get(&link.end_pin_id()).map(|p| p.node_id);

                start_node == Some(node_id) || end_node == Some(node_id)
            })
            .map(|l| l.id())
            .collect()
    }


    /// Normalize a link so (start, end) is always (Output, Input)
    /// 
    /// Returns (output_pin_id, input_pin_id)
    pub fn normalize_link_direction<N>(
        pin_a: i32, 
        pin_b: i32, 
        cache: &GeometryCache<N>,
        output_type: i32
    ) -> Option<(i32, i32)> 
    where
        N: NodeGeometry + Copy,
    {
        let pos_a = cache.pin_positions.get(&pin_a)?;
        // We assume validity was checked, but check existence
        
        if pos_a.pin_type == output_type {
            Some((pin_a, pin_b))
        } else {
            Some((pin_b, pin_a))
        }
    }

    /// Apply a drag translation to selected nodes in a model
    pub fn commit_drag<T>(
        model: &VecModel<T>,
        selection: &SelectionManager,
        delta_x: f32,
        delta_y: f32,
    ) where
        T: MovableNode,
    {
        for i in 0..model.row_count() {
            if let Some(mut node) = model.row_data(i) {
                let id = MovableNode::id(&node);
                if selection.contains(id) {
                    node.set_x(node.x() + delta_x);
                    node.set_y(node.y() + delta_y);
                    model.set_row_data(i, node);
                }
            }
        }
    }

    /// Check if a link with the given direction already exists
    ///
    /// Prevents duplicate connections between the same pins.
    ///
    /// # Arguments
    /// * `start_pin` - Start pin ID
    /// * `end_pin` - End pin ID
    /// * `links` - Iterator over existing links
    pub fn duplicate_link_exists<I, L>(
        start_pin: i32,
        end_pin: i32,
        links: I,
    ) -> bool
    where
        I: IntoIterator<Item = L>,
        L: LinkModel,
    {
        links.into_iter().any(|link| {
            link.start_pin_id() == start_pin && link.end_pin_id() == end_pin
        })
    }

    /// Find a node by ID in a VecModel using a predicate function
    ///
    /// Useful for searching multiple node models when IDs need to be matched
    /// against specific fields.
    ///
    /// # Arguments
    /// * `model` - The VecModel containing nodes
    /// * `id` - The ID to search for
    /// * `predicate` - Function that extracts ID from a node
    ///
    /// # Example
    /// ```ignore
    /// let (index, node) = GraphLogic::find_node_by_id(
    ///     &nodes_model,
    ///     42,
    ///     |n| n.id,
    /// ).unwrap();
    /// ```
    pub fn find_node_by_id<T, F>(
        model: &VecModel<T>,
        id: i32,
        predicate: F,
    ) -> Option<(usize, T)>
    where
        T: Clone + 'static,
        F: Fn(&T) -> i32,
    {
        for i in 0..model.row_count() {
            if let Some(node) = model.row_data(i) {
                if predicate(&node) == id {
                    return Some((i, node));
                }
            }
        }
        None
    }
}

// ============================================================================
// Link Validation Framework
// ============================================================================

/// Result of link validation with optional rejection reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Link is valid
    Valid,
    /// Link is invalid with a reason
    Invalid(ValidationError),
}

impl ValidationResult {
    /// Check if the result is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    /// Combine two results (AND logic): returns first error if any
    pub fn and(self, other: ValidationResult) -> ValidationResult {
        match self {
            ValidationResult::Valid => other,
            invalid => invalid,
        }
    }
}

/// Reasons why a link validation failed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Pin does not exist in the geometry cache
    PinNotFound(i32),
    /// Cannot link a pin to itself
    SamePin,
    /// Cannot link pins on the same node
    SameNode,
    /// Both pins are inputs or both are outputs
    IncompatibleDirection,
    /// A link between these pins already exists
    DuplicateLink,
    /// Pin has reached maximum connections
    MaxConnectionsReached { pin_id: i32, max: usize },
    /// Data types are incompatible
    TypeMismatch { expected: i32, found: i32 },
    /// Custom validation failure
    Custom(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PinNotFound(id) => write!(f, "Pin {} not found", id),
            Self::SamePin => write!(f, "Cannot link pin to itself"),
            Self::SameNode => write!(f, "Cannot link pins on same node"),
            Self::IncompatibleDirection => write!(f, "Must connect input to output"),
            Self::DuplicateLink => write!(f, "Link already exists"),
            Self::MaxConnectionsReached { pin_id, max } => {
                write!(f, "Pin {} has reached max {} connections", pin_id, max)
            }
            Self::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

/// Trait for custom link validation logic.
///
/// Implement this to add custom validation rules for connecting pins.
/// Use with `validate_link()` function or compose with `CompositeValidator`.
///
/// The trait is generic over:
/// - `N`: The node geometry type (defaults to `SimpleNodeGeometry`)
/// - `L`: The link type for accessing existing links (must implement `LinkModel`)
///
/// # Example
///
/// ```ignore
/// struct MyValidator;
///
/// impl<N, L> LinkValidator<N, L> for MyValidator
/// where
///     N: NodeGeometry + Copy,
/// {
///     fn validate(
///         &self,
///         start_pin: i32,
///         end_pin: i32,
///         cache: &GeometryCache<N>,
///         links: &[L],
///     ) -> ValidationResult {
///         // Custom validation logic
///         ValidationResult::Valid
///     }
/// }
/// ```
pub trait LinkValidator<N = SimpleNodeGeometry, L = ()> {
    /// Check if a link between two pins is valid
    ///
    /// # Arguments
    /// * `start_pin` - ID of the starting pin
    /// * `end_pin` - ID of the ending pin
    /// * `cache` - Geometry cache for pin information
    /// * `links` - Slice of existing links for duplicate/fan-out checks
    ///
    /// # Returns
    /// `ValidationResult::Valid` if the link is allowed,
    /// `ValidationResult::Invalid(reason)` otherwise
    fn validate(
        &self,
        start_pin: i32,
        end_pin: i32,
        cache: &GeometryCache<N>,
        links: &[L],
    ) -> ValidationResult;
}

/// Default validator: checks basic I/O compatibility
///
/// This validator implements the standard validation rules:
/// 1. Pins must exist
/// 2. Pins must be on different nodes
/// 3. One pin must be input, one must be output
///
/// Returns detailed error information via `ValidationResult`.
///
/// # Example
///
/// ```ignore
/// let validator = BasicLinkValidator::new(2); // output_type = 2
/// let result = validator.validate(start_pin, end_pin, &cache, &links);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BasicLinkValidator {
    output_type: i32,
}

impl BasicLinkValidator {
    /// Create a new basic validator
    ///
    /// # Arguments
    /// * `output_type` - The pin type integer representing "Output"
    ///   (typically `PinTypes::output` which is 2)
    pub fn new(output_type: i32) -> Self {
        Self { output_type }
    }
}

impl<N, L> LinkValidator<N, L> for BasicLinkValidator
where
    N: NodeGeometry + Copy,
{
    fn validate(
        &self,
        start_pin: i32,
        end_pin: i32,
        cache: &GeometryCache<N>,
        _links: &[L],
    ) -> ValidationResult {
        if start_pin == end_pin {
            return ValidationResult::Invalid(ValidationError::SamePin);
        }

        let start_pos = match cache.pin_positions.get(&start_pin) {
            Some(p) => p,
            None => return ValidationResult::Invalid(ValidationError::PinNotFound(start_pin)),
        };
        let end_pos = match cache.pin_positions.get(&end_pin) {
            Some(p) => p,
            None => return ValidationResult::Invalid(ValidationError::PinNotFound(end_pin)),
        };

        if start_pos.node_id == end_pos.node_id {
            return ValidationResult::Invalid(ValidationError::SameNode);
        }

        let start_is_output = start_pos.pin_type == self.output_type;
        let end_is_output = end_pos.pin_type == self.output_type;

        if start_is_output == end_is_output {
            return ValidationResult::Invalid(ValidationError::IncompatibleDirection);
        }

        ValidationResult::Valid
    }
}

/// Validator that prevents duplicate links
///
/// This wraps the existing `GraphLogic::duplicate_link_exists` helper.
///
/// # Example
///
/// ```ignore
/// let validator = NoDuplicatesValidator;
/// let result = validator.validate(start_pin, end_pin, &cache, &links);
/// ```
#[derive(Clone, Debug, Default)]
pub struct NoDuplicatesValidator;

impl<N, L> LinkValidator<N, L> for NoDuplicatesValidator
where
    L: LinkModel + Clone,
{
    fn validate(
        &self,
        start_pin: i32,
        end_pin: i32,
        _cache: &GeometryCache<N>,
        links: &[L],
    ) -> ValidationResult {
        // Use existing helper from GraphLogic
        if GraphLogic::duplicate_link_exists(start_pin, end_pin, links.iter().cloned()) {
            ValidationResult::Invalid(ValidationError::DuplicateLink)
        } else {
            ValidationResult::Valid
        }
    }
}

/// Composite validator that combines multiple validators
///
/// All validators must return Valid for the link to be valid (AND logic).
/// Returns the first error encountered (short-circuits on failure).
///
/// Note: Uses `Vec<Box<dyn ...>>` which allocates. For zero-allocation
/// validation, chain validators manually using `ValidationResult::and()`.
///
/// # Example
///
/// ```ignore
/// let validator = CompositeValidator::new()
///     .add(BasicLinkValidator::new(2))
///     .add(NoDuplicatesValidator);
///
/// let result = validator.validate(start_pin, end_pin, &cache, &links);
/// ```
pub struct CompositeValidator<N = SimpleNodeGeometry, L = ()> {
    validators: Vec<Box<dyn LinkValidator<N, L>>>,
}

impl<N, L> Default for CompositeValidator<N, L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, L> CompositeValidator<N, L> {
    /// Create a new empty composite validator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Add a validator to the composite
    ///
    /// Validators are checked in the order they were added.
    /// The first validator to return Invalid will short-circuit.
    pub fn add<V: LinkValidator<N, L> + 'static>(mut self, validator: V) -> Self {
        self.validators.push(Box::new(validator));
        self
    }
}

impl<N, L> LinkValidator<N, L> for CompositeValidator<N, L> {
    fn validate(
        &self,
        start_pin: i32,
        end_pin: i32,
        cache: &GeometryCache<N>,
        links: &[L],
    ) -> ValidationResult {
        for v in &self.validators {
            let result = v.validate(start_pin, end_pin, cache, links);
            if !result.is_valid() {
                return result;
            }
        }
        ValidationResult::Valid
    }
}

/// Convenience function to validate a link with any validator
///
/// # Example
///
/// ```ignore
/// let validator = BasicLinkValidator::new(2);
/// let result = validate_link(start_pin, end_pin, &cache, &links, &validator);
///
/// match result {
///     ValidationResult::Valid => { /* create link */ }
///     ValidationResult::Invalid(err) => eprintln!("Cannot create link: {}", err),
/// }
/// ```
pub fn validate_link<V, N, L>(
    start_pin: i32,
    end_pin: i32,
    cache: &GeometryCache<N>,
    links: &[L],
    validator: &V,
) -> ValidationResult
where
    V: LinkValidator<N, L>,
{
    validator.validate(start_pin, end_pin, cache, links)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hit_test::SimpleNodeGeometry;
    use crate::state::StoredPin;

    /// Helper to create a test geometry cache with pins
    fn setup_cache() -> GeometryCache<SimpleNodeGeometry> {
        let mut cache = GeometryCache::new();

        // Add two nodes
        cache.node_rects.insert(
            1,
            SimpleNodeGeometry {
                id: 1,
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
        );
        cache.node_rects.insert(
            2,
            SimpleNodeGeometry {
                id: 2,
                x: 200.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
        );

        // Pin 1001: output on node 1 (pin_type = 2)
        cache.pin_positions.insert(
            1001,
            StoredPin {
                node_id: 1,
                pin_type: 2,
                rel_x: 100.0,
                rel_y: 25.0,
            },
        );
        // Pin 2001: input on node 2 (pin_type = 1)
        cache.pin_positions.insert(
            2001,
            StoredPin {
                node_id: 2,
                pin_type: 1,
                rel_x: 0.0,
                rel_y: 25.0,
            },
        );
        // Pin 2002: another input on node 2
        cache.pin_positions.insert(
            2002,
            StoredPin {
                node_id: 2,
                pin_type: 1,
                rel_x: 0.0,
                rel_y: 40.0,
            },
        );

        cache
    }

    // Test helper that implements LinkModel
    #[derive(Clone, Debug)]
    struct TestLink {
        id: i32,
        start: i32,
        end: i32,
    }

    impl LinkModel for TestLink {
        fn id(&self) -> i32 {
            self.id
        }
        fn start_pin_id(&self) -> i32 {
            self.start
        }
        fn end_pin_id(&self) -> i32 {
            self.end
        }
    }

    #[test]
    fn test_basic_validator_accepts_valid_link() {
        let cache = setup_cache();
        let validator = BasicLinkValidator::new(2); // output_type = 2
        let links: Vec<TestLink> = vec![];

        let result = validator.validate(1001, 2001, &cache, &links);
        assert!(result.is_valid());
    }

    #[test]
    fn test_basic_validator_rejects_same_pin() {
        let cache = setup_cache();
        let validator = BasicLinkValidator::new(2);
        let links: Vec<TestLink> = vec![];

        let result = validator.validate(1001, 1001, &cache, &links);
        assert_eq!(result, ValidationResult::Invalid(ValidationError::SamePin));
    }

    #[test]
    fn test_basic_validator_rejects_same_node() {
        let mut cache = setup_cache();
        // Add another pin on node 1
        cache.pin_positions.insert(
            1002,
            StoredPin {
                node_id: 1,
                pin_type: 1,
                rel_x: 0.0,
                rel_y: 25.0,
            },
        );

        let validator = BasicLinkValidator::new(2);
        let links: Vec<TestLink> = vec![];

        let result = validator.validate(1001, 1002, &cache, &links);
        assert_eq!(
            result,
            ValidationResult::Invalid(ValidationError::SameNode)
        );
    }

    #[test]
    fn test_basic_validator_rejects_same_direction() {
        let mut cache = setup_cache();
        // Add another output on node 2
        cache.pin_positions.insert(
            2003,
            StoredPin {
                node_id: 2,
                pin_type: 2,
                rel_x: 100.0,
                rel_y: 25.0,
            },
        );

        let validator = BasicLinkValidator::new(2);
        let links: Vec<TestLink> = vec![];

        // Both are outputs
        let result = validator.validate(1001, 2003, &cache, &links);
        assert_eq!(
            result,
            ValidationResult::Invalid(ValidationError::IncompatibleDirection)
        );
    }

    #[test]
    fn test_basic_validator_rejects_missing_pin() {
        let cache = setup_cache();
        let validator = BasicLinkValidator::new(2);
        let links: Vec<TestLink> = vec![];

        let result = validator.validate(1001, 9999, &cache, &links);
        assert_eq!(
            result,
            ValidationResult::Invalid(ValidationError::PinNotFound(9999))
        );
    }

    #[test]
    fn test_no_duplicates_validator_accepts_new_link() {
        let cache = setup_cache();
        let validator = NoDuplicatesValidator;
        let links = vec![TestLink {
            id: 1,
            start: 1001,
            end: 2002,
        }];

        // Different link - should pass
        let result = validator.validate(1001, 2001, &cache, &links);
        assert!(result.is_valid());
    }

    #[test]
    fn test_no_duplicates_validator_rejects_duplicate() {
        let cache = setup_cache();
        let validator = NoDuplicatesValidator;
        let links = vec![TestLink {
            id: 1,
            start: 1001,
            end: 2001,
        }];

        // Same link - should fail
        let result = validator.validate(1001, 2001, &cache, &links);
        assert_eq!(
            result,
            ValidationResult::Invalid(ValidationError::DuplicateLink)
        );
    }

    #[test]
    fn test_composite_validator_passes_all() {
        let cache = setup_cache();
        let validator: CompositeValidator<_, TestLink> = CompositeValidator::new()
            .add(BasicLinkValidator::new(2))
            .add(NoDuplicatesValidator);

        let links = vec![];

        let result = validator.validate(1001, 2001, &cache, &links);
        assert!(result.is_valid());
    }

    #[test]
    fn test_composite_validator_short_circuits_on_basic() {
        let cache = setup_cache();
        let validator: CompositeValidator<_, TestLink> = CompositeValidator::new()
            .add(BasicLinkValidator::new(2))
            .add(NoDuplicatesValidator);

        let links = vec![];

        // Should fail on BasicValidator (same pin)
        let result = validator.validate(1001, 1001, &cache, &links);
        assert_eq!(result, ValidationResult::Invalid(ValidationError::SamePin));
    }

    #[test]
    fn test_composite_validator_short_circuits_on_duplicates() {
        let cache = setup_cache();
        let validator: CompositeValidator<_, TestLink> = CompositeValidator::new()
            .add(BasicLinkValidator::new(2))
            .add(NoDuplicatesValidator);

        let links = vec![TestLink {
            id: 1,
            start: 1001,
            end: 2001,
        }];

        // Should pass BasicValidator but fail on NoDuplicatesValidator
        let result = validator.validate(1001, 2001, &cache, &links);
        assert_eq!(
            result,
            ValidationResult::Invalid(ValidationError::DuplicateLink)
        );
    }

    #[test]
    fn test_validate_link_convenience_function() {
        let cache = setup_cache();
        let validator = BasicLinkValidator::new(2);
        let links: Vec<TestLink> = vec![];

        let result = validate_link(1001, 2001, &cache, &links, &validator);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validation_result_and_combinator() {
        let result1 = ValidationResult::Valid;
        let result2 = ValidationResult::Valid;
        assert!(result1.and(result2).is_valid());

        let result1 = ValidationResult::Valid;
        let result2 = ValidationResult::Invalid(ValidationError::SamePin);
        assert_eq!(
            result1.and(result2),
            ValidationResult::Invalid(ValidationError::SamePin)
        );

        let result1 = ValidationResult::Invalid(ValidationError::SameNode);
        let result2 = ValidationResult::Valid;
        assert_eq!(
            result1.and(result2),
            ValidationResult::Invalid(ValidationError::SameNode)
        );

        // First error wins
        let result1 = ValidationResult::Invalid(ValidationError::SameNode);
        let result2 = ValidationResult::Invalid(ValidationError::SamePin);
        assert_eq!(
            result1.and(result2),
            ValidationResult::Invalid(ValidationError::SameNode)
        );
    }

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            format!("{}", ValidationError::PinNotFound(42)),
            "Pin 42 not found"
        );
        assert_eq!(
            format!("{}", ValidationError::SamePin),
            "Cannot link pin to itself"
        );
        assert_eq!(
            format!("{}", ValidationError::SameNode),
            "Cannot link pins on same node"
        );
        assert_eq!(
            format!("{}", ValidationError::IncompatibleDirection),
            "Must connect input to output"
        );
        assert_eq!(
            format!("{}", ValidationError::DuplicateLink),
            "Link already exists"
        );
        assert_eq!(
            format!(
                "{}",
                ValidationError::MaxConnectionsReached {
                    pin_id: 123,
                    max: 3
                }
            ),
            "Pin 123 has reached max 3 connections"
        );
        assert_eq!(
            format!(
                "{}",
                ValidationError::TypeMismatch {
                    expected: 1,
                    found: 2
                }
            ),
            "Type mismatch: expected 1, found 2"
        );
        assert_eq!(
            format!("{}", ValidationError::Custom("Test error".to_string())),
            "Test error"
        );
    }

    /// Test a custom validator implementation
    #[test]
    fn test_custom_validator() {
        #[derive(Clone, Copy)]
        struct FanOutValidator {
            max_connections: usize,
        }

        impl<N, L> LinkValidator<N, L> for FanOutValidator
        where
            L: LinkModel,
        {
            fn validate(
                &self,
                start_pin: i32,
                _end_pin: i32,
                _cache: &GeometryCache<N>,
                links: &[L],
            ) -> ValidationResult {
                let existing_count = links
                    .iter()
                    .filter(|link| link.start_pin_id() == start_pin)
                    .count();

                if existing_count >= self.max_connections {
                    ValidationResult::Invalid(ValidationError::MaxConnectionsReached {
                        pin_id: start_pin,
                        max: self.max_connections,
                    })
                } else {
                    ValidationResult::Valid
                }
            }
        }

        let cache = setup_cache();
        let validator = FanOutValidator { max_connections: 1 };

        // One existing link from pin 1001
        let existing_links = vec![TestLink {
            id: 1,
            start: 1001,
            end: 2001,
        }];

        // Second connection should be rejected
        let result = validator.validate(1001, 2002, &cache, &existing_links);
        assert_eq!(
            result,
            ValidationResult::Invalid(ValidationError::MaxConnectionsReached {
                pin_id: 1001,
                max: 1,
            })
        );

        // First connection would have been accepted
        let no_links: Vec<TestLink> = vec![];
        let result = validator.validate(1001, 2001, &cache, &no_links);
        assert!(result.is_valid());
    }

    // ========================================================================
    // GraphLogic::find_links_connected_to_node() tests
    // ========================================================================

    #[test]
    fn test_find_links_connected_to_node_by_start() {
        let cache = setup_cache();
        let links = vec![
            TestLink { id: 1, start: 1001, end: 2001 },
            TestLink { id: 2, start: 2001, end: 1001 },
        ];

        // Node 1 owns pin 1001 - link 1 starts from it, link 2 ends at it
        let connected = GraphLogic::find_links_connected_to_node(1, links.into_iter(), &cache);
        assert!(connected.contains(&1));
        assert!(connected.contains(&2));
    }

    #[test]
    fn test_find_links_connected_to_node_no_connections() {
        let cache = setup_cache();
        let links = vec![TestLink { id: 1, start: 1001, end: 2001 }];

        // Node 999 doesn't exist - no links connected
        let connected = GraphLogic::find_links_connected_to_node(999, links.into_iter(), &cache);
        assert!(connected.is_empty());
    }

    #[test]
    fn test_find_links_connected_to_node_empty_links() {
        let cache = setup_cache();
        let links: Vec<TestLink> = vec![];

        let connected = GraphLogic::find_links_connected_to_node(1, links.into_iter(), &cache);
        assert!(connected.is_empty());
    }

    #[test]
    fn test_find_links_connected_to_node_multiple_pins() {
        let mut cache = setup_cache();
        // Add another pin on node 1
        cache.pin_positions.insert(
            1002,
            StoredPin {
                node_id: 1,
                pin_type: 1,
                rel_x: 0.0,
                rel_y: 40.0,
            },
        );

        let links = vec![
            TestLink { id: 1, start: 1001, end: 2001 },
            TestLink { id: 2, start: 2001, end: 1002 },
            TestLink { id: 3, start: 2001, end: 2002 }, // Not connected to node 1
        ];

        let connected = GraphLogic::find_links_connected_to_node(1, links.into_iter(), &cache);
        assert!(connected.contains(&1));
        assert!(connected.contains(&2));
        assert!(!connected.contains(&3));
    }

    // ========================================================================
    // GraphLogic::normalize_link_direction() tests
    // ========================================================================

    #[test]
    fn test_normalize_link_direction_output_first() {
        let cache = setup_cache();
        // Pin 1001 is output (pin_type = 2), pin 2001 is input (pin_type = 1)
        let result = GraphLogic::normalize_link_direction(1001, 2001, &cache, 2);
        assert_eq!(result, Some((1001, 2001)));
    }

    #[test]
    fn test_normalize_link_direction_input_first() {
        let cache = setup_cache();
        // Pin 2001 is input, pin 1001 is output - should swap
        let result = GraphLogic::normalize_link_direction(2001, 1001, &cache, 2);
        assert_eq!(result, Some((1001, 2001)));
    }

    #[test]
    fn test_normalize_link_direction_missing_pin() {
        let cache = setup_cache();
        let result = GraphLogic::normalize_link_direction(9999, 2001, &cache, 2);
        assert!(result.is_none());
    }

    // ========================================================================
    // GraphLogic::duplicate_link_exists() tests
    // ========================================================================

    #[test]
    fn test_duplicate_link_exists_true() {
        let links = vec![TestLink { id: 1, start: 1001, end: 2001 }];
        assert!(GraphLogic::duplicate_link_exists(1001, 2001, links));
    }

    #[test]
    fn test_duplicate_link_exists_false() {
        let links = vec![TestLink { id: 1, start: 1001, end: 2001 }];
        assert!(!GraphLogic::duplicate_link_exists(1001, 2002, links));
    }

    #[test]
    fn test_duplicate_link_exists_empty() {
        let links: Vec<TestLink> = vec![];
        assert!(!GraphLogic::duplicate_link_exists(1001, 2001, links));
    }

    #[test]
    fn test_duplicate_link_exists_direction_matters() {
        let links = vec![TestLink { id: 1, start: 1001, end: 2001 }];
        // Reversed direction - not a duplicate
        assert!(!GraphLogic::duplicate_link_exists(2001, 1001, links));
    }

    // ========================================================================
    // GraphLogic::find_node_by_id() tests
    // ========================================================================

    #[test]
    fn test_find_node_by_id_found() {
        #[derive(Clone)]
        struct TestNode {
            id: i32,
            name: String,
        }

        let model = std::rc::Rc::new(VecModel::from(vec![
            TestNode { id: 1, name: "Node 1".to_string() },
            TestNode { id: 2, name: "Node 2".to_string() },
            TestNode { id: 3, name: "Node 3".to_string() },
        ]));

        let result = GraphLogic::find_node_by_id(&model, 2, |n| n.id);
        assert!(result.is_some());
        let (index, node) = result.unwrap();
        assert_eq!(index, 1);
        assert_eq!(node.id, 2);
        assert_eq!(node.name, "Node 2");
    }

    #[test]
    fn test_find_node_by_id_not_found() {
        #[derive(Clone)]
        struct TestNode { id: i32 }

        let model = std::rc::Rc::new(VecModel::from(vec![
            TestNode { id: 1 },
            TestNode { id: 2 },
        ]));

        let result = GraphLogic::find_node_by_id(&model, 999, |n| n.id);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_node_by_id_empty_model() {
        #[derive(Clone)]
        struct TestNode { id: i32 }

        let model = std::rc::Rc::new(VecModel::<TestNode>::default());

        let result = GraphLogic::find_node_by_id(&model, 1, |n| n.id);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_node_by_id_first_match() {
        #[derive(Clone)]
        struct TestNode { id: i32, value: i32 }

        // Two nodes with same id - should return first
        let model = std::rc::Rc::new(VecModel::from(vec![
            TestNode { id: 1, value: 100 },
            TestNode { id: 1, value: 200 },
        ]));

        let result = GraphLogic::find_node_by_id(&model, 1, |n| n.id);
        assert!(result.is_some());
        let (index, node) = result.unwrap();
        assert_eq!(index, 0);
        assert_eq!(node.value, 100);
    }

    // ========================================================================
    // GraphLogic::commit_drag() tests
    // ========================================================================

    #[test]
    fn test_commit_drag_moves_selected_nodes() {
        #[derive(Clone)]
        struct TestMovableNode {
            id: i32,
            x: f32,
            y: f32,
        }

        impl MovableNode for TestMovableNode {
            fn id(&self) -> i32 { self.id }
            fn x(&self) -> f32 { self.x }
            fn y(&self) -> f32 { self.y }
            fn set_x(&mut self, x: f32) { self.x = x; }
            fn set_y(&mut self, y: f32) { self.y = y; }
        }

        let model = std::rc::Rc::new(VecModel::from(vec![
            TestMovableNode { id: 1, x: 0.0, y: 0.0 },
            TestMovableNode { id: 2, x: 100.0, y: 100.0 },
            TestMovableNode { id: 3, x: 200.0, y: 200.0 },
        ]));

        let mut selection = crate::selection::SelectionManager::new();
        selection.replace_selection(vec![1, 3]); // Select nodes 1 and 3

        GraphLogic::commit_drag(&model, &selection, 10.0, 20.0);

        // Node 1 should be moved
        let node1 = model.row_data(0).unwrap();
        assert_eq!(node1.x, 10.0);
        assert_eq!(node1.y, 20.0);

        // Node 2 should NOT be moved (not selected)
        let node2 = model.row_data(1).unwrap();
        assert_eq!(node2.x, 100.0);
        assert_eq!(node2.y, 100.0);

        // Node 3 should be moved
        let node3 = model.row_data(2).unwrap();
        assert_eq!(node3.x, 210.0);
        assert_eq!(node3.y, 220.0);
    }

    #[test]
    fn test_commit_drag_empty_selection() {
        #[derive(Clone)]
        struct TestMovableNode {
            id: i32,
            x: f32,
            y: f32,
        }

        impl MovableNode for TestMovableNode {
            fn id(&self) -> i32 { self.id }
            fn x(&self) -> f32 { self.x }
            fn y(&self) -> f32 { self.y }
            fn set_x(&mut self, x: f32) { self.x = x; }
            fn set_y(&mut self, y: f32) { self.y = y; }
        }

        let model = std::rc::Rc::new(VecModel::from(vec![
            TestMovableNode { id: 1, x: 50.0, y: 50.0 },
        ]));

        let selection = crate::selection::SelectionManager::new(); // Empty

        GraphLogic::commit_drag(&model, &selection, 100.0, 100.0);

        // Node should NOT be moved
        let node = model.row_data(0).unwrap();
        assert_eq!(node.x, 50.0);
        assert_eq!(node.y, 50.0);
    }

    #[test]
    fn test_commit_drag_negative_delta() {
        #[derive(Clone)]
        struct TestMovableNode {
            id: i32,
            x: f32,
            y: f32,
        }

        impl MovableNode for TestMovableNode {
            fn id(&self) -> i32 { self.id }
            fn x(&self) -> f32 { self.x }
            fn y(&self) -> f32 { self.y }
            fn set_x(&mut self, x: f32) { self.x = x; }
            fn set_y(&mut self, y: f32) { self.y = y; }
        }

        let model = std::rc::Rc::new(VecModel::from(vec![
            TestMovableNode { id: 1, x: 100.0, y: 100.0 },
        ]));

        let mut selection = crate::selection::SelectionManager::new();
        selection.handle_interaction(1, false);

        GraphLogic::commit_drag(&model, &selection, -50.0, -30.0);

        let node = model.row_data(0).unwrap();
        assert_eq!(node.x, 50.0);
        assert_eq!(node.y, 70.0);
    }
}