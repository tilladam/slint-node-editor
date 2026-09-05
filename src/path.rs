/// Below this many pixels between endpoints the link degenerates to a straight
/// segment: a horizontal-biased curve over so short a span reads as a zig-zag.
/// The offset ramps in over the span from here to 4× here, so the transition
/// out of the fallback is seamless.
const STRAIGHT_THRESHOLD: f32 = 20.0;

/// The cubic bezier a link is drawn as.
///
/// Horizontal-biased: each control point shares its endpoint's `y` and is
/// offset along `x`, which is what makes a link leave a pin sideways. These
/// four points are the single description of the curve — the drawn commands,
/// the bounding box and the hit test all read them, so a click lands on the
/// stroke the eye sees.
pub struct CubicBezier {
    pub p0: (f32, f32), // Start point
    pub p1: (f32, f32), // Control point 1
    pub p2: (f32, f32), // Control point 2
    pub p3: (f32, f32), // End point
}

impl CubicBezier {
    /// Build the curve between two pin centres.
    ///
    /// # Arguments
    /// * `start_x`, `start_y` - Start point (pin center)
    /// * `end_x`, `end_y` - End point (pin center)
    /// * `zoom` - Current zoom level (scales the threshold and the offset)
    /// * `min_offset` - Minimum control point offset (typically 50.0)
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
        let threshold = STRAIGHT_THRESHOLD * zoom;

        if dist_sq < threshold * threshold {
            // Control points collapsed onto their endpoints: the cubic *is* the
            // segment, so nothing downstream needs a second shape to handle.
            return CubicBezier {
                p0: (start_x, start_y),
                p1: (start_x, start_y),
                p2: (end_x, end_y),
                p3: (end_x, end_y),
            };
        }

        let dist = dist_sq.sqrt();
        let full_offset = (dx.abs() * 0.5).max(min_offset * zoom);
        let ramp = ((dist - threshold) / (3.0 * threshold)).clamp(0.0, 1.0);
        let offset = full_offset * ramp;

        // Control points extend horizontally, following the direction of dx
        let sign = if dx >= 0.0 { 1.0 } else { -1.0 };
        CubicBezier {
            p0: (start_x, start_y),
            p1: (start_x + sign * offset, start_y),
            p2: (end_x - sign * offset, end_y),
            p3: (end_x, end_y),
        }
    }

    /// Whether the curve collapsed to the straight-line fallback.
    pub fn is_straight(&self) -> bool {
        self.p1 == self.p0 && self.p2 == self.p3
    }

    /// SVG path commands for the curve, with coordinates relative to `origin`.
    ///
    /// Pass `(0.0, 0.0)` for absolute coordinates; pass the origin of
    /// [`bounds`](Self::bounds) to get commands that fit inside that box.
    pub fn commands_from(&self, origin: (f32, f32)) -> String {
        let (ox, oy) = origin;
        if self.is_straight() {
            return format!(
                "M {} {} L {} {}",
                self.p0.0 - ox,
                self.p0.1 - oy,
                self.p3.0 - ox,
                self.p3.1 - oy
            );
        }
        format!(
            "M {} {} C {} {} {} {} {} {}",
            self.p0.0 - ox,
            self.p0.1 - oy,
            self.p1.0 - ox,
            self.p1.1 - oy,
            self.p2.0 - ox,
            self.p2.1 - oy,
            self.p3.0 - ox,
            self.p3.1 - oy
        )
    }

    /// The axis-aligned box containing the curve, as `(x, y, width, height)`.
    ///
    /// A cubic lies within the convex hull of its control points, so the
    /// extremes of the four bound it without subdividing. This is the
    /// centreline's box: a stroke spills half its width past it in every
    /// direction, and whoever knows the stroke width pads for that.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let xs = [self.p0.0, self.p1.0, self.p2.0, self.p3.0];
        let ys = [self.p0.1, self.p1.1, self.p2.1, self.p3.1];
        let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min);
        let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let min_y = ys.iter().copied().fold(f32::INFINITY, f32::min);
        let max_y = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// The sub-curve from `t=0` to `t=t`, by de Casteljau subdivision.
    pub fn split_at(&self, t: f32) -> CubicBezier {
        let q0 = lerp_point(self.p0, self.p1, t);
        let q1 = lerp_point(self.p1, self.p2, t);
        let q2 = lerp_point(self.p2, self.p3, t);

        let r0 = lerp_point(q0, q1, t);
        let r1 = lerp_point(q1, q2, t);

        let s = lerp_point(r0, r1, t);

        CubicBezier {
            p0: self.p0,
            p1: q0,
            p2: r0,
            p3: s,
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
    CubicBezier::from_endpoints(start_x, start_y, end_x, end_y, zoom, min_offset)
        .commands_from((0.0, 0.0))
}

/// Generate SVG path command for a partial bezier link (for animation)
///
/// Creates a "growing" effect where the curve snakes from start to end.
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
    let t = progress.clamp(0.0, 1.0);
    let curve = CubicBezier::from_endpoints(start_x, start_y, end_x, end_y, zoom, min_offset);

    if t >= 1.0 {
        return curve.commands_from((0.0, 0.0));
    }

    if t <= 0.0 || curve.is_straight() {
        // Grow a straight fallback at constant speed. Subdividing its cubic
        // form would ease in and out along a segment that has no curvature for
        // the easing to follow.
        let x = start_x + (end_x - start_x) * t;
        let y = start_y + (end_y - start_y) * t;
        return format!("M {} {} L {} {}", start_x, start_y, x, y);
    }

    curve.split_at(t).commands_from((0.0, 0.0))
}

/// Linear interpolation between two points
fn lerp_point(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
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
        // Distance is 5.0, threshold is 20.0 — should produce straight line
        let path = generate_bezier_path(0.0, 0.0, 5.0, 0.0, 1.0, 50.0);
        assert!(path.contains(" L "));
        assert!(!path.contains(" C "));

        // Distance is 25.0, threshold is 20.0 — should produce bezier curve
        let path2 = generate_bezier_path(0.0, 0.0, 25.0, 0.0, 1.0, 50.0);
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
            assert!(
                dist >= 0.0,
                "Distance should be non-negative for {:?}",
                point
            );
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
    // The drawn curve, the box and the hit test are one curve
    // ========================================================================

    #[test]
    fn test_from_endpoints_matches_generated_path() {
        // A span short enough to sit inside the old 10-vs-20 threshold gap:
        // from_endpoints used to fall through to a curve here while the drawn
        // path was a straight line.
        for (sx, sy, ex, ey) in [
            (0.0, 0.0, 15.0, 0.0),
            (0.0, 0.0, 25.0, 0.0),
            (10.0, 20.0, 100.0, 80.0),
            (100.0, 0.0, 0.0, 50.0),
        ] {
            let curve = CubicBezier::from_endpoints(sx, sy, ex, ey, 1.0, 50.0);
            assert_eq!(
                curve.commands_from((0.0, 0.0)),
                generate_bezier_path(sx, sy, ex, ey, 1.0, 50.0),
                "curve and drawn path disagree for ({sx},{sy})->({ex},{ey})"
            );
        }
    }

    #[test]
    fn test_short_link_is_straight() {
        let curve = CubicBezier::from_endpoints(0.0, 0.0, 5.0, 0.0, 1.0, 50.0);
        assert!(curve.is_straight());
        assert!(!CubicBezier::from_endpoints(0.0, 0.0, 25.0, 0.0, 1.0, 50.0).is_straight());
    }

    // ========================================================================
    // CubicBezier::bounds() - the box a bounded link element takes
    // ========================================================================

    #[test]
    fn test_bounds_contains_the_curve() {
        let curve = CubicBezier::from_endpoints(0.0, 0.0, 200.0, 120.0, 1.0, 50.0);
        let (x, y, w, h) = curve.bounds();

        for i in 0..=50 {
            let (px, py) = curve.eval(i as f32 / 50.0);
            assert!(
                px >= x - 0.001 && px <= x + w + 0.001,
                "x {px} outside {x}..{}",
                x + w
            );
            assert!(
                py >= y - 0.001 && py <= y + h + 0.001,
                "y {py} outside {y}..{}",
                y + h
            );
        }
    }

    #[test]
    fn test_bounds_of_horizontal_link_has_zero_height() {
        // Both control points share their endpoint's y, so a link between pins
        // at the same height is flat — the caller has to pad for the stroke.
        let curve = CubicBezier::from_endpoints(0.0, 50.0, 300.0, 50.0, 1.0, 50.0);
        let (_, y, _, h) = curve.bounds();
        assert_eq!(y, 50.0);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn test_bounds_spans_the_control_points_not_just_the_endpoints() {
        // A near-vertical link still leaves its pins sideways by `min_offset`,
        // so its box is far wider than the two endpoints.
        let curve = CubicBezier::from_endpoints(100.0, 0.0, 110.0, 300.0, 1.0, 50.0);
        let (x, _, w, _) = curve.bounds();
        assert!(
            x < 100.0,
            "box should start left of the start point, got {x}"
        );
        assert!(
            x + w > 110.0,
            "box should end right of the end point, got {}",
            x + w
        );
        assert!(w > 50.0, "box should be at least the offset wide, got {w}");
    }

    #[test]
    fn test_bounds_of_straight_link_is_the_segment() {
        let curve = CubicBezier::from_endpoints(10.0, 20.0, 15.0, 24.0, 1.0, 50.0);
        assert_eq!(curve.bounds(), (10.0, 20.0, 5.0, 4.0));
    }

    // ========================================================================
    // commands_from() - relative coordinates
    // ========================================================================

    #[test]
    fn test_commands_from_bounds_origin_are_box_relative() {
        let curve = CubicBezier::from_endpoints(100.0, 40.0, 300.0, 160.0, 1.0, 50.0);
        let (x, y, w, h) = curve.bounds();
        let commands = curve.commands_from((x, y));

        let coords: Vec<f32> = commands
            .split_whitespace()
            .filter_map(|t| t.parse::<f32>().ok())
            .collect();
        assert_eq!(coords.len(), 8);
        for pair in coords.chunks(2) {
            assert!(
                pair[0] >= -0.001 && pair[0] <= w + 0.001,
                "x {} outside 0..{w}",
                pair[0]
            );
            assert!(
                pair[1] >= -0.001 && pair[1] <= h + 0.001,
                "y {} outside 0..{h}",
                pair[1]
            );
        }
    }

    // ========================================================================
    // split_at() - the animation sub-curve
    // ========================================================================

    #[test]
    fn test_split_at_traces_the_same_curve() {
        let curve = CubicBezier::from_endpoints(0.0, 0.0, 200.0, 100.0, 1.0, 50.0);
        let half = curve.split_at(0.5);

        assert_eq!(half.p0, curve.p0);
        // The sub-curve's end is the full curve's midpoint, and its own
        // midpoint is the full curve at a quarter.
        let end = half.eval(1.0);
        let mid = curve.eval(0.5);
        assert!((end.0 - mid.0).abs() < 0.01 && (end.1 - mid.1).abs() < 0.01);

        let quarter = curve.eval(0.25);
        let half_mid = half.eval(0.5);
        assert!((half_mid.0 - quarter.0).abs() < 0.01 && (half_mid.1 - quarter.1).abs() < 0.01);
    }

    #[test]
    fn test_partial_path_grows_from_the_start() {
        let full = generate_partial_bezier_path(0.0, 0.0, 200.0, 100.0, 1.0, 50.0, 1.0);
        assert_eq!(
            full,
            generate_bezier_path(0.0, 0.0, 200.0, 100.0, 1.0, 50.0)
        );

        let none = generate_partial_bezier_path(0.0, 0.0, 200.0, 100.0, 1.0, 50.0, 0.0);
        assert_eq!(none, "M 0 0 L 0 0");

        let part = generate_partial_bezier_path(0.0, 0.0, 200.0, 100.0, 1.0, 50.0, 0.5);
        assert!(part.starts_with("M 0 0 C"));
        assert_ne!(part, full);
    }

    #[test]
    fn test_partial_path_of_short_link_stays_straight() {
        let part = generate_partial_bezier_path(0.0, 0.0, 10.0, 0.0, 1.0, 50.0, 0.5);
        assert_eq!(part, "M 0 0 L 5 0");
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
