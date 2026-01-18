/// Generate SVG path command for a bezier link between two points
///
/// Creates a horizontal-biased cubic bezier curve suitable for node connections.
/// Control points extend horizontally from start and end points.
///
/// # Arguments
/// * `start_x`, `start_y` - Start point (pin center)
/// * `end_x`, `end_y` - End point (pin center)
/// * `zoom` - Current zoom level (affects control point offset)
/// * `min_offset` - Minimum control point offset (default: 50.0)
///
/// # Returns
/// SVG path command string (e.g., "M 10 20 C 60 20 90 80 140 80")
pub fn generate_bezier_path(
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    zoom: f32,
    min_offset: f32,
) -> String {
    // If distance is very small, use a straight line to avoid zig-zags
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let dist_sq = dx * dx + dy * dy;
    let threshold = 10.0 * zoom;

    if dist_sq < threshold * threshold {
        return format!("M {} {} L {} {}", start_x, start_y, end_x, end_y);
    }

    // Calculate control point offset (horizontal bezier)
    let dx_abs = dx.abs();
    let offset = (dx_abs * 0.5).max(min_offset * zoom);

    // Control points extend horizontally from start and end
    let ctrl1_x = start_x + offset;
    let ctrl1_y = start_y;
    let ctrl2_x = end_x - offset;
    let ctrl2_y = end_y;

    // Generate SVG path: M (move to), C (cubic bezier)
    format!(
        "M {} {} C {} {} {} {} {} {}",
        start_x, start_y, ctrl1_x, ctrl1_y, ctrl2_x, ctrl2_y, end_x, end_y
    )
}

/// Generate SVG path command for a partial bezier link (for animation)
///
/// Uses de Casteljau's algorithm to compute the sub-curve from t=0 to t=progress.
/// This creates a "growing" animation effect where the curve snakes from start to end.
///
/// # Arguments
/// * `start_x`, `start_y` - Start point (pin center)
/// * `end_x`, `end_y` - End point (pin center)
/// * `zoom` - Current zoom level (affects control point offset)
/// * `min_offset` - Minimum control point offset (default: 50.0)
/// * `progress` - Animation progress from 0.0 to 1.0
///
/// # Returns
/// SVG path command string for the partial curve
pub fn generate_partial_bezier_path(
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    zoom: f32,
    min_offset: f32,
    progress: f32,
) -> String {
    // Clamp progress to valid range
    let t = progress.clamp(0.0, 1.0);

    if t <= 0.0 {
        // No curve visible yet - just return a point
        return format!("M {} {} L {} {}", start_x, start_y, start_x, start_y);
    }

    if t >= 1.0 {
        // Full curve - use standard function
        return generate_bezier_path(start_x, start_y, end_x, end_y, zoom, min_offset);
    }

    // If distance is very small, use a straight line
    let dx_full = end_x - start_x;
    let dy_full = end_y - start_y;
    let dist_sq = dx_full * dx_full + dy_full * dy_full;
    let threshold = 10.0 * zoom;

    if dist_sq < threshold * threshold {
        let curr_x = start_x + dx_full * t;
        let curr_y = start_y + dy_full * t;
        return format!("M {} {} L {} {}", start_x, start_y, curr_x, curr_y);
    }

    // Calculate full bezier control points
    let dx_abs = dx_full.abs();
    let offset = (dx_abs * 0.5).max(min_offset * zoom);

    let p0 = (start_x, start_y);
    let p1 = (start_x + offset, start_y);
    let p2 = (end_x - offset, end_y);
    let p3 = (end_x, end_y);

    // De Casteljau's algorithm to split at t
    // Level 1: lerp between adjacent points
    let q0 = lerp_point(p0, p1, t);
    let q1 = lerp_point(p1, p2, t);
    let q2 = lerp_point(p2, p3, t);

    // Level 2: lerp between level 1 points
    let r0 = lerp_point(q0, q1, t);
    let r1 = lerp_point(q1, q2, t);

    // Level 3: the point on the curve at t
    let s = lerp_point(r0, r1, t);

    // The partial curve from 0 to t uses:
    // P0' = P0, P1' = Q0, P2' = R0, P3' = S
    format!(
        "M {} {} C {} {} {} {} {} {}",
        p0.0, p0.1, q0.0, q0.1, r0.0, r0.1, s.0, s.1
    )
}

/// Linear interpolation between two points
fn lerp_point(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Cubic bezier curve for distance calculations
pub struct CubicBezier {
    pub p0: (f32, f32), // Start point
    pub p1: (f32, f32), // Control point 1
    pub p2: (f32, f32), // Control point 2
    pub p3: (f32, f32), // End point
}

impl CubicBezier {
    /// Create a bezier from endpoints using the same logic as generate_bezier_path
    ///
    /// # Arguments
    /// * `start_x`, `start_y` - Start point
    /// * `end_x`, `end_y` - End point
    /// * `zoom` - Current zoom level
    /// * `min_offset` - Minimum control point offset (default: 50.0)
    pub fn from_endpoints(
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        zoom: f32,
        min_offset: f32,
    ) -> Self {
        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let dist_sq = dx * dx + dy * dy;
        let threshold = 10.0 * zoom;

        if dist_sq < threshold * threshold {
            return CubicBezier {
                p0: (start_x, start_y),
                p1: (start_x, start_y),
                p2: (end_x, end_y),
                p3: (end_x, end_y),
            };
        }

        let dx_abs = dx.abs();
        let offset = (dx_abs * 0.5).max(min_offset * zoom);

        CubicBezier {
            p0: (start_x, start_y),
            p1: (start_x + offset, start_y),
            p2: (end_x - offset, end_y),
            p3: (end_x, end_y),
        }
    }

    /// Evaluate the bezier curve at parameter t (0.0 to 1.0)
    pub fn eval(&self, t: f32) -> (f32, f32) {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        let x = mt3 * self.p0.0
            + 3.0 * mt2 * t * self.p1.0
            + 3.0 * mt * t2 * self.p2.0
            + t3 * self.p3.0;
        let y = mt3 * self.p0.1
            + 3.0 * mt2 * t * self.p1.1
            + 3.0 * mt * t2 * self.p2.1
            + t3 * self.p3.1;

        (x, y)
    }
}

/// Calculate squared distance from a point to a line segment
fn distance_to_line_segment_sq(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let ap = (point.0 - a.0, point.1 - a.1);

    let ab_len_sq = ab.0 * ab.0 + ab.1 * ab.1;

    if ab_len_sq < f32::EPSILON {
        // Degenerate segment (a == b)
        return ap.0 * ap.0 + ap.1 * ap.1;
    }

    // Project point onto line, clamped to segment
    let t = ((ap.0 * ab.0 + ap.1 * ab.1) / ab_len_sq).clamp(0.0, 1.0);

    // Closest point on segment
    let closest = (a.0 + t * ab.0, a.1 + t * ab.1);

    // Distance squared from point to closest point
    let dx = point.0 - closest.0;
    let dy = point.1 - closest.1;
    dx * dx + dy * dy
}

/// Calculate the minimum distance from a point to a cubic bezier curve
///
/// Uses subdivision approach: sample curve at regular intervals and find closest point.
///
/// # Arguments
/// * `point` - The point to measure distance from
/// * `bezier` - The bezier curve
/// * `num_samples` - Number of samples for distance calculation (default: 20)
pub fn distance_to_bezier(point: (f32, f32), bezier: &CubicBezier, num_samples: usize) -> f32 {
    let num_samples = if num_samples == 0 { 20 } else { num_samples };

    let mut min_dist_sq = f32::MAX;
    let mut prev_point = bezier.eval(0.0);

    for i in 1..=num_samples {
        let t = i as f32 / num_samples as f32;
        let curr_point = bezier.eval(t);

        let dist_sq = distance_to_line_segment_sq(point, prev_point, curr_point);
        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
        }

        prev_point = curr_point;
    }

    min_dist_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // generate_bezier_path() - SVG Path Generation
    // ========================================================================

    #[test]
    fn test_bezier_path() {
        let path = generate_bezier_path(0.0, 50.0, 100.0, 50.0, 1.0, 50.0);
        assert!(path.starts_with("M 0 50 C"));
        assert!(path.ends_with("100 50"));
    }

    #[test]
    fn test_bezier_path_format() {
        let path = generate_bezier_path(10.0, 20.0, 100.0, 80.0, 1.0, 50.0);
        // Should be: M start_x start_y C ctrl1_x ctrl1_y ctrl2_x ctrl2_y end_x end_y
        assert!(path.starts_with("M 10 20 C"));
        assert!(path.ends_with("100 80"));
    }

    #[test]
    fn test_bezier_path_small_distance() {
        // Distance is 5.0, threshold is 10.0
        let path = generate_bezier_path(0.0, 0.0, 5.0, 0.0, 1.0, 50.0);
        assert!(path.contains(" L "));
        assert!(!path.contains(" C "));

        // Distance is 15.0, threshold is 10.0
        let path2 = generate_bezier_path(0.0, 0.0, 15.0, 0.0, 1.0, 50.0);
        assert!(path2.contains(" C "));
        assert!(!path2.contains(" L "));
    }

    #[test]
    fn test_bezier_path_zero_distance() {
        // Start and end at same point - should produce a straight line (effectively a point)
        let path = generate_bezier_path(50.0, 50.0, 50.0, 50.0, 1.0, 50.0);
        assert!(path.starts_with("M "));
        assert!(path.contains(" L "));
        assert!(!path.contains(" C "));
    }

    #[test]
    fn test_bezier_path_negative_coords() {
        let path = generate_bezier_path(-100.0, -50.0, 100.0, 50.0, 1.0, 50.0);
        assert!(path.starts_with("M -100 -50 C"));
        assert!(path.ends_with("100 50"));
    }

    #[test]
    fn test_bezier_path_zoom_affects_offset() {
        let path1 = generate_bezier_path(0.0, 0.0, 50.0, 0.0, 1.0, 50.0);
        let path2 = generate_bezier_path(0.0, 0.0, 50.0, 0.0, 2.0, 50.0);
        // Different zoom should produce different control points
        assert_ne!(path1, path2);
    }

    // ========================================================================
    // CubicBezier::from_endpoints() - Construction
    // ========================================================================

    #[test]
    fn test_bezier_from_endpoints_creates_correct_points() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 100.0, 1.0, 50.0);

        assert_eq!(bezier.p0, (0.0, 0.0));
        assert_eq!(bezier.p3, (100.0, 100.0));
        // Control points should extend horizontally
        assert_eq!(bezier.p1.1, 0.0); // Same y as start
        assert_eq!(bezier.p2.1, 100.0); // Same y as end
    }

    #[test]
    fn test_bezier_from_endpoints_horizontal_control_points() {
        let bezier = CubicBezier::from_endpoints(0.0, 50.0, 100.0, 50.0, 1.0, 50.0);

        // p1 should be to the right of p0
        assert!(bezier.p1.0 > bezier.p0.0);
        // p2 should be to the left of p3
        assert!(bezier.p2.0 < bezier.p3.0);
    }

    // ========================================================================
    // CubicBezier::eval() - Boundary Values
    // ========================================================================

    #[test]
    fn test_bezier_eval_at_t0_returns_start() {
        let bezier = CubicBezier::from_endpoints(10.0, 20.0, 100.0, 80.0, 1.0, 50.0);
        let point = bezier.eval(0.0);

        assert!((point.0 - 10.0).abs() < 0.001);
        assert!((point.1 - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_bezier_eval_at_t1_returns_end() {
        let bezier = CubicBezier::from_endpoints(10.0, 20.0, 100.0, 80.0, 1.0, 50.0);
        let point = bezier.eval(1.0);

        assert!((point.0 - 100.0).abs() < 0.001);
        assert!((point.1 - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_bezier_eval_at_midpoint() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        let point = bezier.eval(0.5);

        // For a horizontal bezier, midpoint should be roughly at center x
        assert!(point.0 > 40.0 && point.0 < 60.0);
        // Y should stay at 0 since it's a horizontal curve
        assert!((point.1 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_bezier_eval_with_explicit_control_points() {
        // Straight line bezier
        let bezier = CubicBezier {
            p0: (0.0, 0.0),
            p1: (33.33, 33.33),
            p2: (66.66, 66.66),
            p3: (100.0, 100.0),
        };

        // For a straight line, eval(0.5) should be at midpoint
        let mid = bezier.eval(0.5);
        assert!((mid.0 - 50.0).abs() < 1.0);
        assert!((mid.1 - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_bezier_eval_degenerate_point() {
        // All points at same location
        let bezier = CubicBezier {
            p0: (50.0, 50.0),
            p1: (50.0, 50.0),
            p2: (50.0, 50.0),
            p3: (50.0, 50.0),
        };

        // Any t should return the same point
        assert_eq!(bezier.eval(0.0), (50.0, 50.0));
        assert_eq!(bezier.eval(0.5), (50.0, 50.0));
        assert_eq!(bezier.eval(1.0), (50.0, 50.0));
    }

    // ========================================================================
    // distance_to_bezier() - Distance Calculation
    // ========================================================================

    #[test]
    fn test_distance_to_bezier_point_on_start() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        let dist = distance_to_bezier((0.0, 0.0), &bezier, 20);

        // Should be very close to 0
        assert!(dist < 1.0);
    }

    #[test]
    fn test_distance_to_bezier_point_on_end() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        let dist = distance_to_bezier((100.0, 0.0), &bezier, 20);

        // Should be very close to 0
        assert!(dist < 1.0);
    }

    #[test]
    fn test_distance_to_bezier_point_near_curve() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        // Point 5 units above the middle of a horizontal curve
        let dist = distance_to_bezier((50.0, 5.0), &bezier, 20);

        // Should be close to 5
        assert!(dist < 10.0);
        assert!(dist > 2.0);
    }

    #[test]
    fn test_distance_to_bezier_point_far_away() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        let dist = distance_to_bezier((50.0, 100.0), &bezier, 20);

        // Should be approximately 100
        assert!(dist > 90.0);
    }

    #[test]
    fn test_distance_to_bezier_zero_samples_uses_default() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        // Should not panic with 0 samples
        let dist = distance_to_bezier((50.0, 10.0), &bezier, 0);

        assert!(dist.is_finite());
        assert!(dist >= 0.0);
    }

    #[test]
    fn test_distance_to_bezier_one_sample() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        // With 1 sample, it should still work
        let dist = distance_to_bezier((50.0, 10.0), &bezier, 1);

        assert!(dist.is_finite());
        assert!(dist >= 0.0);
    }

    #[test]
    fn test_distance_to_bezier_more_samples_more_accurate() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);
        let point = (50.0, 1.0); // Very close to curve

        let dist_low = distance_to_bezier(point, &bezier, 5);
        let dist_high = distance_to_bezier(point, &bezier, 100);

        // Higher sample count should give equal or better (smaller) distance
        assert!(dist_high <= dist_low + 0.5); // Allow small tolerance
    }

    #[test]
    fn test_distance_to_bezier_always_non_negative() {
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 100.0, 1.0, 50.0);

        // Test various points
        let points = [
            (50.0, 50.0),
            (-100.0, -100.0),
            (200.0, 200.0),
            (0.0, 100.0),
            (100.0, 0.0),
        ];

        for point in points {
            let dist = distance_to_bezier(point, &bezier, 20);
            assert!(dist >= 0.0, "Distance should be non-negative for {:?}", point);
        }
    }

    #[test]
    fn test_distance_to_bezier_negative_coords() {
        let bezier = CubicBezier::from_endpoints(-100.0, -50.0, 100.0, 50.0, 1.0, 50.0);
        let dist = distance_to_bezier((-100.0, -50.0), &bezier, 20);

        // Point on start should be very close
        assert!(dist < 1.0);
    }

    // ========================================================================
    // Property-based tests
    // ========================================================================

    #[test]
    fn test_bezier_symmetry() {
        // A bezier from (0,0) to (100,0) should be symmetric around x=50
        let bezier = CubicBezier::from_endpoints(0.0, 0.0, 100.0, 0.0, 1.0, 50.0);

        let left = bezier.eval(0.25);
        let right = bezier.eval(0.75);

        // The y values should be the same (symmetric curve)
        assert!((left.1 - right.1).abs() < 0.001);
        // The x values should be symmetric around 50
        assert!((left.0 + right.0 - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_bezier_monotonic_x_for_horizontal() {
        let bezier = CubicBezier::from_endpoints(0.0, 50.0, 100.0, 50.0, 1.0, 50.0);

        // For a horizontal bezier, x should be monotonically increasing
        let mut prev_x = bezier.eval(0.0).0;
        for i in 1..=20 {
            let t = i as f32 / 20.0;
            let curr_x = bezier.eval(t).0;
            assert!(
                curr_x >= prev_x - 0.001,
                "X should be monotonic at t={}: {} < {}",
                t,
                curr_x,
                prev_x
            );
            prev_x = curr_x;
        }
    }
}
