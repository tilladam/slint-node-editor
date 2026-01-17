/// Generate SVG path commands for grid lines
///
/// Creates a string of SVG path commands for rendering an infinite grid.
/// The grid adjusts based on pan offset and zoom level.
///
/// # Arguments
/// * `width` - Canvas width in pixels
/// * `height` - Canvas height in pixels
/// * `zoom` - Current zoom level
/// * `pan_x` - Pan offset X in pixels
/// * `pan_y` - Pan offset Y in pixels
/// * `spacing` - Base grid spacing (before zoom)
///
/// # Returns
/// SVG path commands string (e.g., "M 24 0 L 24 600 M 48 0 L 48 600...")
pub fn generate_grid_commands(
    width: f32,
    height: f32,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    spacing: f32,
) -> String {
    let effective_spacing = spacing * zoom;

    // Skip if spacing is too small to be visible
    if effective_spacing < 4.0 {
        return String::new();
    }

    // Calculate grid offset based on pan (modulo spacing for infinite grid effect)
    let offset_x = pan_x.rem_euclid(effective_spacing);
    let offset_y = pan_y.rem_euclid(effective_spacing);

    let mut commands = String::with_capacity(10000);

    // Generate vertical lines
    let mut x = offset_x;
    while x < width + effective_spacing {
        if !commands.is_empty() {
            commands.push(' ');
        }
        commands.push_str(&format!("M {} 0 L {} {}", x, x, height));
        x += effective_spacing;
    }

    // Generate horizontal lines
    let mut y = offset_y;
    while y < height + effective_spacing {
        commands.push(' ');
        commands.push_str(&format!("M 0 {} L {} {}", y, width, y));
        y += effective_spacing;
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Basic Grid Generation
    // ========================================================================

    #[test]
    fn test_grid_commands() {
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 24.0);
        assert!(commands.contains("M 0 0 L 0 100")); // First vertical
        assert!(commands.contains("M 24 0 L 24 100")); // Second vertical
    }

    #[test]
    fn test_grid_commands_format() {
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 50.0);
        // Should have vertical lines
        assert!(commands.contains("L 0 100")); // Vertical line to bottom
        // Should have horizontal lines
        assert!(commands.contains("L 100")); // Horizontal line to right edge
    }

    #[test]
    fn test_grid_contains_horizontal_lines() {
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 25.0);
        // Horizontal lines start with M 0 y and end with L width y
        assert!(commands.contains("M 0 0 L 100 0")); // First horizontal
        assert!(commands.contains("M 0 25 L 100 25")); // Second horizontal
    }

    // ========================================================================
    // Zoom Behavior
    // ========================================================================

    #[test]
    fn test_grid_commands_zoom_affects_spacing() {
        let commands1 = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 20.0);
        let commands2 = generate_grid_commands(100.0, 100.0, 2.0, 0.0, 0.0, 20.0);

        // With zoom=2, effective spacing is 40, so fewer lines
        // Count vertical lines (each starts with "M x 0")
        let count1 = commands1.matches("M ").count();
        let count2 = commands2.matches("M ").count();

        // Higher zoom = larger spacing = fewer lines
        assert!(count1 > count2);
    }

    #[test]
    fn test_grid_commands_very_small_zoom() {
        // When zoom is very small, effective_spacing < 4.0, so no grid
        let commands = generate_grid_commands(100.0, 100.0, 0.1, 0.0, 0.0, 20.0);
        // Effective spacing = 20 * 0.1 = 2.0 < 4.0
        assert!(commands.is_empty());
    }

    #[test]
    fn test_grid_commands_at_spacing_threshold() {
        // Exactly at threshold (effective_spacing = 4.0) - should render
        // The check is `< 4.0`, so 4.0 passes and renders
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 4.0);
        assert!(!commands.is_empty()); // 4.0 is NOT < 4.0, so renders

        // Just below threshold - should NOT render
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 3.9);
        assert!(commands.is_empty()); // 3.9 < 4.0, so empty

        // Just above threshold - should render
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 4.1);
        assert!(!commands.is_empty());
    }

    // ========================================================================
    // Pan Behavior
    // ========================================================================

    #[test]
    fn test_grid_commands_pan_offsets_lines() {
        let commands_no_pan = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 20.0);
        let commands_with_pan = generate_grid_commands(100.0, 100.0, 1.0, 10.0, 0.0, 20.0);

        // The commands should be different due to pan offset
        assert_ne!(commands_no_pan, commands_with_pan);
    }

    #[test]
    fn test_grid_commands_pan_wraps_with_modulo() {
        // Pan of exactly one spacing should look the same (modulo effect)
        let commands1 = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 20.0);
        let commands2 = generate_grid_commands(100.0, 100.0, 1.0, 20.0, 0.0, 20.0);

        assert_eq!(commands1, commands2);
    }

    #[test]
    fn test_grid_commands_negative_pan() {
        // Negative pan should also work (rem_euclid handles this)
        let commands = generate_grid_commands(100.0, 100.0, 1.0, -10.0, -10.0, 20.0);
        // Should still produce valid output
        assert!(!commands.is_empty());
        assert!(commands.contains("M "));
        assert!(commands.contains(" L "));
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_grid_commands_zero_dimensions() {
        let commands = generate_grid_commands(0.0, 0.0, 1.0, 0.0, 0.0, 20.0);
        // With zero dimensions, there shouldn't be many lines
        // (loops iterate while x < 0 + spacing and y < 0 + spacing)
        assert!(!commands.is_empty()); // Will have at least one line due to while condition
    }

    #[test]
    fn test_grid_commands_very_small_spacing() {
        // Spacing below threshold
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 3.0);
        // 3.0 < 4.0, so should return empty
        assert!(commands.is_empty());
    }

    #[test]
    fn test_grid_commands_large_canvas() {
        let commands = generate_grid_commands(1000.0, 1000.0, 1.0, 0.0, 0.0, 50.0);
        // Should have many lines
        assert!(!commands.is_empty());
        // Count approximate number of lines
        let line_count = commands.matches("M ").count();
        // 1000/50 = 20 vertical + 20 horizontal = ~40 lines
        assert!(line_count >= 30);
    }

    #[test]
    fn test_grid_commands_spacing_larger_than_canvas() {
        let commands = generate_grid_commands(50.0, 50.0, 1.0, 0.0, 0.0, 100.0);
        // Spacing > canvas, but should still draw first line
        assert!(!commands.is_empty());
    }

    #[test]
    fn test_grid_commands_non_divisible_spacing() {
        // Canvas size not evenly divisible by spacing
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 33.0);
        assert!(!commands.is_empty());
        // Should have lines at 0, 33, 66, 99 (approximate)
    }

    // ========================================================================
    // Output Format Validation
    // ========================================================================

    #[test]
    fn test_grid_commands_valid_svg_format() {
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 25.0);

        // Each command should follow "M x y L x y" pattern
        for part in commands.split(' ').collect::<Vec<_>>().chunks(6) {
            if part.len() >= 6 {
                assert!(part[0] == "M" || part[0].parse::<f32>().is_ok());
            }
        }
    }

    #[test]
    fn test_grid_commands_no_trailing_space() {
        let commands = generate_grid_commands(100.0, 100.0, 1.0, 0.0, 0.0, 25.0);
        assert!(!commands.ends_with(' '));
    }
}
