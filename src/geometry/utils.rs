use bevy::math::Vec2;
use spade::Point2;

use crate::core::Polygon;

pub fn polygon_area(polygon: &Polygon) -> f32 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }

    let mut area = 0.0f32;
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].x * polygon[j].y;
        area -= polygon[j].x * polygon[i].y;
    }

    area / 2.0
}

pub fn polygon_centroid(polygon: &Polygon, area: f32) -> Vec2 {
    let n = polygon.len();
    if n < 3 || area.abs() < f32::EPSILON {
        return Vec2::ZERO;
    }

    let mut cx = 0.0f64;
    let mut cy = 0.0f64;

    for i in 0..n {
        let j = (i + 1) % n;
        let cross =
            polygon[i].x as f64 * polygon[j].y as f64 - polygon[j].x as f64 * polygon[i].y as f64;
        cx += (polygon[i].x as f64 + polygon[j].x as f64) * cross;
        cy += (polygon[i].y as f64 + polygon[j].y as f64) * cross;
    }

    let area6 = 6.0 * area as f64;
    Vec2::new((cx / area6) as f32, (cy / area6) as f32)
}

pub fn line_segment_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let s1 = p2 - p1;
    let s2 = p4 - p3;

    let denom = s1.x * s2.y - s2.x * s1.y;

    if denom.abs() < 1e-6 {
        return None;
    }

    let s = (s1.x * (p1.y - p3.y) - s1.y * (p1.x - p3.x)) / denom;
    let t = (s2.x * (p1.y - p3.y) - s2.y * (p1.x - p3.x)) / denom;

    if s >= 0.0 && s <= 1.0 && t >= 0.0 && t <= 1.0 {
        Some(p1 + t * s1)
    } else {
        None
    }
}

/// Intersection of two *infinite* lines, each given by a point and a direction.
/// Returns `None` when the lines are parallel.
///
/// Unlike [`line_segment_intersection`] this ignores the extent of the inputs,
/// which is what edge offsetting needs: neighbouring offset edges usually meet
/// outside the span of either original edge.
pub fn line_line_intersection(p1: Vec2, d1: Vec2, p2: Vec2, d2: Vec2) -> Option<Vec2> {
    let denom = d1.x * d2.y - d1.y * d2.x;

    if denom.abs() < 1e-6 {
        return None;
    }

    let diff = p2 - p1;
    let t = (diff.x * d2.y - diff.y * d2.x) / denom;

    Some(p1 + d1 * t)
}

/// Shrinks a polygon inward by `distance`, offsetting every edge along its
/// inward normal and re-intersecting neighbouring edges.
///
/// Returns `None` if the polygon collapses, inverts, or is too degenerate to
/// offset - callers should treat that as "this polygon is too small to inset"
/// rather than falling back to the original, which would overlap its neighbours.
pub fn inset_polygon(polygon: &Polygon, distance: f32) -> Option<Polygon> {
    let n = polygon.len();

    if n < 3 {
        return None;
    }

    if distance <= 0.0 {
        return Some(polygon.clone());
    }

    let area = polygon_area(polygon);
    if area.abs() < f32::EPSILON {
        return None;
    }

    let is_ccw = area > 0.0;

    // Offset each edge inward, keeping it as an (origin, direction) line.
    let mut offset_lines = Vec::with_capacity(n);
    for i in 0..n {
        let start = polygon[i];
        let end = polygon[(i + 1) % n];
        let edge = end - start;

        if edge.length_squared() < 1e-12 {
            return None;
        }

        let dir = edge.normalize();
        let inward = if is_ccw {
            Vec2::new(-dir.y, dir.x)
        } else {
            Vec2::new(dir.y, -dir.x)
        };

        offset_lines.push((start + inward * distance, dir));
    }

    // Each new vertex is where consecutive offset edges meet.
    let mut inset = Vec::with_capacity(n);
    for i in 0..n {
        let prev = (i + n - 1) % n;
        let (prev_point, prev_dir) = offset_lines[prev];
        let (curr_point, curr_dir) = offset_lines[i];

        let vertex = line_line_intersection(prev_point, prev_dir, curr_point, curr_dir)?;

        if !vertex.is_finite() {
            return None;
        }

        inset.push(vertex);
    }

    let inset_area = polygon_area(&inset);

    // An inset that flipped winding turned itself inside out, and one that grew
    // means the offsets crossed over each other.
    if inset_area.abs() < f32::EPSILON
        || (inset_area > 0.0) != is_ccw
        || inset_area.abs() >= area.abs()
    {
        return None;
    }

    Some(inset)
}

/// Scales a polygon about its own centroid. `factor` above 1.0 grows it, below
/// 1.0 shrinks it.
///
/// Unlike [`inset_polygon`] this keeps the shape *similar* rather than holding
/// edges a constant distance apart, which is what "a smaller copy in the middle"
/// wants - a courtyard should echo the shape of its block.
pub fn scale_polygon_about_centroid(polygon: &Polygon, factor: f32) -> Polygon {
    if polygon.len() < 3 {
        return polygon.clone();
    }

    let area = polygon_area(polygon);
    if area.abs() < f32::EPSILON {
        return polygon.clone();
    }

    let centroid = polygon_centroid(polygon, area);

    polygon
        .iter()
        .map(|&v| centroid + (v - centroid) * factor)
        .collect()
}

pub fn calculate_circumcenter(p1: Point2<f64>, p2: Point2<f64>, p3: Point2<f64>) -> (f64, f64) {
    let ax = p1.x;
    let ay = p1.y;
    let bx = p2.x;
    let by = p2.y;
    let cx = p3.x;
    let cy = p3.y;

    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));

    let centroid_x = (ax + bx + cx) / 3.0;
    let centroid_y = (ay + by + cy) / 3.0;

    if d.abs() < f64::EPSILON {
        return (centroid_x, centroid_y);
    }

    let ux = ((ax * ax + ay * ay) * (by - cy)
        + (bx * bx + by * by) * (cy - ay)
        + (cx * cx + cy * cy) * (ay - by))
        / d;

    let uy = ((ax * ax + ay * ay) * (cx - bx)
        + (bx * bx + by * by) * (ax - cx)
        + (cx * cx + cy * cy) * (bx - ax))
        / d;

    let canvas_bound = crate::config::CANVS_WIDTH as f64 * 5.0;
    let dist = ((ux - centroid_x).powi(2) + (uy - centroid_y).powi(2)).sqrt();

    if dist > canvas_bound || ux.abs() > canvas_bound || uy.abs() > canvas_bound {
        return (centroid_x, centroid_y);
    }

    (ux, uy)
}

pub fn point_in_polygon(point: &Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = polygon.len() - 1;

    for i in 0..polygon.len() {
        let xi = polygon[i].x;
        let yi = polygon[i].y;
        let xj = polygon[j].x;
        let yj = polygon[j].y;

        if ((yi > point.y) != (yj > point.y))
            && (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi)
        {
            inside = !inside;
        }

        j = i;
    }

    inside
}

pub fn point_to_segment_distance(point: Vec2, seg_start: Vec2, seg_end: Vec2) -> f32 {
    let seg_vec = seg_end - seg_start;
    let point_vec = point - seg_start;
    let seg_len_sq = seg_vec.length_squared();

    if seg_len_sq < f32::EPSILON {
        return point_vec.length();
    }

    let t = (point_vec.dot(seg_vec) / seg_len_sq).clamp(0.0, 1.0);
    let projection = seg_start + seg_vec * t;

    point.distance(projection)
}

pub fn longest_edge(polygon: &Polygon) -> Option<(usize, Vec2, f32)> {
    if polygon.len() < 2 {
        return None;
    }

    let mut max_len = 0.0f32;
    let mut best_idx = 0;

    for i in 0..polygon.len() {
        let next = (i + 1) % polygon.len();
        let len = polygon[i].distance(polygon[next]);
        if len > max_len {
            max_len = len;
            best_idx = i;
        }
    }

    Some((best_idx, polygon[best_idx], max_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn approx_eq2(a: Vec2, b: Vec2) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y)
    }

    // unit square CCW
    fn unit_square() -> Polygon {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ]
    }

    #[test]
    fn test_polygon_area_unit_square() {
        let sq = unit_square();
        // CCW winding -> positive area
        assert!(approx_eq(polygon_area(&sq).abs(), 1.0));
    }

    #[test]
    fn test_polygon_area_degenerate() {
        assert_eq!(polygon_area(&vec![Vec2::ZERO, Vec2::ONE]), 0.0);
        assert_eq!(polygon_area(&vec![]), 0.0);
    }

    #[test]
    fn test_polygon_area_triangle() {
        let tri = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(0.0, 3.0),
        ];
        assert!(approx_eq(polygon_area(&tri).abs(), 6.0));
    }

    #[test]
    fn test_polygon_centroid_unit_square() {
        let sq = unit_square();
        let area = polygon_area(&sq);
        let centroid = polygon_centroid(&sq, area);
        assert!(approx_eq2(centroid, Vec2::new(0.5, 0.5)));
    }

    #[test]
    fn test_polygon_centroid_degenerate() {
        let result = polygon_centroid(&vec![Vec2::ZERO], 0.0);
        assert_eq!(result, Vec2::ZERO);
    }

    #[test]
    fn test_line_segment_intersection_cross() {
        // Two segments crossing at (0.5, 0.5)
        let result = line_segment_intersection(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
        );
        assert!(result.is_some());
        assert!(approx_eq2(result.unwrap(), Vec2::new(0.5, 0.5)));
    }

    #[test]
    fn test_line_segment_intersection_parallel() {
        let result = line_segment_intersection(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_line_segment_intersection_no_overlap() {
        let result = line_segment_intersection(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.4, 0.4),
            Vec2::new(0.6, 0.6),
            Vec2::new(1.0, 1.0),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_point_in_polygon_inside() {
        let sq = unit_square();
        assert!(point_in_polygon(&Vec2::new(0.5, 0.5), &sq));
    }

    #[test]
    fn test_point_in_polygon_outside() {
        let sq = unit_square();
        assert!(!point_in_polygon(&Vec2::new(2.0, 2.0), &sq));
    }

    #[test]
    fn test_point_in_polygon_degenerate() {
        assert!(!point_in_polygon(&Vec2::ZERO, &vec![Vec2::ZERO, Vec2::ONE]));
    }

    #[test]
    fn test_point_to_segment_distance_perpendicular() {
        let d = point_to_segment_distance(
            Vec2::new(0.5, 1.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        );
        assert!(approx_eq(d, 1.0));
    }

    #[test]
    fn test_point_to_segment_distance_endpoint() {
        let d = point_to_segment_distance(
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        );
        assert!(approx_eq(d, 1.0));
    }

    #[test]
    fn test_longest_edge_square() {
        let sq = unit_square();
        let result = longest_edge(&sq);
        assert!(result.is_some());
        let (_, _, len) = result.unwrap();
        assert!(approx_eq(len, 1.0));
    }

    #[test]
    fn test_longest_edge_degenerate() {
        assert!(longest_edge(&vec![Vec2::ZERO]).is_none());
    }

    #[test]
    fn test_circumcenter_equilateral() {
        use spade::Point2;
        // equilateral triangle, circumcenter at centroid-ish
        let p1 = Point2::new(0.0f64, 0.0);
        let p2 = Point2::new(2.0f64, 0.0);
        let p3 = Point2::new(1.0f64, 3.0f64.sqrt());
        let (cx, cy) = calculate_circumcenter(p1, p2, p3);
        // circumcenter should be at (1.0, 1/sqrt(3))
        assert!((cx - 1.0).abs() < 1e-4);
        assert!((cy - 1.0 / 3.0f64.sqrt()).abs() < 1e-4);
    }

    #[test]
    fn test_line_line_intersection_beyond_segment_extent() {
        // The lines meet at (2,2), well outside the span of either sample point.
        let r = line_line_intersection(
            Vec2::new(0.0, 2.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 1.0),
        );
        assert!(approx_eq2(r.unwrap(), Vec2::new(2.0, 2.0)));
    }

    #[test]
    fn test_line_line_intersection_parallel() {
        let r = line_line_intersection(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 5.0),
            Vec2::new(1.0, 0.0),
        );
        assert!(r.is_none());
    }

    fn square(size: f32) -> Polygon {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(size, 0.0),
            Vec2::new(size, size),
            Vec2::new(0.0, size),
        ]
    }

    #[test]
    fn test_inset_square_shrinks_by_distance() {
        let inset = inset_polygon(&square(10.0), 1.0).unwrap();
        assert_eq!(inset.len(), 4);
        // A 10x10 square inset by 1 on every side is 8x8.
        assert!(approx_eq(polygon_area(&inset).abs(), 64.0));
        assert!(approx_eq2(inset[0], Vec2::new(1.0, 1.0)));
        assert!(approx_eq2(inset[2], Vec2::new(9.0, 9.0)));
    }

    #[test]
    fn test_inset_preserves_winding_for_cw_input() {
        let mut cw = square(10.0);
        cw.reverse();
        let original = polygon_area(&cw);
        let inset = inset_polygon(&cw, 1.0).unwrap();
        let after = polygon_area(&inset);
        assert!(
            (original > 0.0) == (after > 0.0),
            "inset flipped the winding"
        );
        assert!(approx_eq(after.abs(), 64.0));
    }

    #[test]
    fn test_inset_rejects_collapse() {
        // Inset by more than half the width - nothing sensible is left.
        assert!(inset_polygon(&square(4.0), 5.0).is_none());
    }

    #[test]
    fn test_inset_always_shrinks() {
        let poly = square(20.0);
        let original = polygon_area(&poly).abs();
        for d in [0.5f32, 1.0, 2.0, 4.0] {
            let inset = inset_polygon(&poly, d).expect("should inset");
            let a = polygon_area(&inset).abs();
            assert!(a < original, "inset by {d} did not shrink: {a} vs {original}");
        }
    }

    #[test]
    fn test_inset_degenerate_input() {
        assert!(inset_polygon(&vec![Vec2::ZERO, Vec2::ONE], 1.0).is_none());
        assert!(inset_polygon(&vec![], 1.0).is_none());
    }

    #[test]
    fn test_inset_zero_distance_is_identity() {
        let poly = square(10.0);
        assert_eq!(inset_polygon(&poly, 0.0).unwrap(), poly);
    }

    #[test]
    fn test_scale_polygon_about_centroid_shrinks_by_area_squared() {
        let poly = square(10.0);
        let half = scale_polygon_about_centroid(&poly, 0.5);
        // Linear scale f changes area by f^2.
        assert!(approx_eq(polygon_area(&half).abs(), 25.0));
        // Centroid is preserved.
        let c0 = polygon_centroid(&poly, polygon_area(&poly));
        let c1 = polygon_centroid(&half, polygon_area(&half));
        assert!(approx_eq2(c0, c1));
    }

    #[test]
    fn test_scale_polygon_about_centroid_grows() {
        let poly = square(10.0);
        let big = scale_polygon_about_centroid(&poly, 1.2);
        assert!(polygon_area(&big).abs() > polygon_area(&poly).abs());
        assert_eq!(big.len(), poly.len());
    }

    #[test]
    fn test_scale_polygon_about_centroid_degenerate() {
        let line = vec![Vec2::ZERO, Vec2::ONE];
        assert_eq!(scale_polygon_about_centroid(&line, 0.5), line);
    }

    #[test]
    fn test_circumcenter_right_triangle() {
        use spade::Point2;
        // Right angle at the origin -> circumcenter is the hypotenuse midpoint.
        // Unlike the equilateral case this does NOT coincide with the centroid,
        // so it catches a wrong determinant instead of silently falling back.
        let p1 = Point2::new(0.0f64, 0.0);
        let p2 = Point2::new(4.0f64, 0.0);
        let p3 = Point2::new(0.0f64, 3.0);
        let (cx, cy) = calculate_circumcenter(p1, p2, p3);
        assert!((cx - 2.0).abs() < 1e-4, "cx was {cx}, expected 2.0");
        assert!((cy - 1.5).abs() < 1e-4, "cy was {cy}, expected 1.5");
    }

    #[test]
    fn test_circumcenter_collinear_fallback() {
        use spade::Point2;
        let p1 = Point2::new(0.0f64, 0.0);
        let p2 = Point2::new(1.0f64, 0.0);
        let p3 = Point2::new(2.0f64, 0.0);
        // collinear -> falls back to centroid
        let (cx, cy) = calculate_circumcenter(p1, p2, p3);
        assert!((cx - 1.0).abs() < 1e-4);
        assert!(cy.abs() < 1e-4);
    }
}
