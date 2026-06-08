use bevy::math::Vec2;

use crate::{
    core::Polygon,
    geometry::{
        line_segment_intersection, point_to_segment_distance, polygon_area, polygon_centroid,
    },
};

pub fn bisect_polygon(
    polygon: &Polygon,
    start_idx: usize,
    ratio: f32,
    angle_offset: f32,
    separation: f32,
) -> Vec<Polygon> {
    if polygon.len() < 3 || start_idx >= polygon.len() {
        return vec![polygon.clone()];
    }

    let next_idx = (start_idx + 1) % polygon.len();
    let start_v = polygon[start_idx];
    let next_v = polygon[next_idx];

    let edge_dir = next_v - start_v;
    let cut_point = start_v + edge_dir * ratio;

    let perp = Vec2::new(-edge_dir.y, edge_dir.x).normalize();
    let rotated = Vec2::new(
        perp.x * angle_offset.cos() - perp.y * angle_offset.sin(),
        perp.x * angle_offset.sin() + perp.y * angle_offset.cos(),
    );

    let bounds_diag = polygon_bounds_diagonal(polygon);
    let line_start = cut_point - rotated * bounds_diag;
    let line_end = cut_point + rotated * bounds_diag;

    let mut intersections = Vec::new();
    for i in 0..polygon.len() {
        let j = (i + 1) % polygon.len();
        if let Some(pt) = line_segment_intersection(line_start, line_end, polygon[i], polygon[j]) {
            intersections.push((i, pt));
        }
    }

    if intersections.len() != 2 {
        return vec![polygon.clone()];
    }

    intersections.sort_by_key(|&(idx, _)| idx);
    let (idx1, int1) = intersections[0];
    let (idx2, int2) = intersections[1];

    let mut poly1 = Vec::new();
    poly1.push(int1);
    for i in (idx1 + 1)..=idx2 {
        poly1.push(polygon[i]);
    }
    poly1.push(int2);

    let mut poly2 = Vec::new();
    poly2.push(int2);
    for i in (idx2 + 1)..polygon.len() {
        poly2.push(polygon[i]);
    }
    for i in 0..=idx1 {
        poly2.push(polygon[i]);
    }
    poly2.push(int1);

    let mut result = Vec::new();

    if poly1.len() >= 3 && polygon_area(&poly1).abs() > 0.1 {
        let final_poly = if separation > 0.0 {
            push_polygon_from_line(&poly2, line_start, line_end, separation * 0.5)
        } else {
            poly1
        };
        result.push(final_poly);
    }

    if poly2.len() >= 3 && polygon_area(&poly2).abs() > 0.1 {
        let final_poly = if separation > 0.0 {
            push_polygon_from_line(&poly2, line_start, line_end, separation * 0.5)
        } else {
            poly2
        };
        result.push(final_poly);
    }

    if result.is_empty() {
        vec![polygon.clone()]
    } else {
        result
    }
}

pub fn push_polygon_from_line(
    polygon: &Polygon,
    line_start: Vec2,
    line_end: Vec2,
    distance: f32,
) -> Polygon {
    if polygon.len() < 3 {
        return polygon.clone();
    }

    let line_dir = (line_end - line_start).normalize();
    let line_normal = Vec2::new(-line_dir.y, line_dir.x);

    let area = polygon_area(polygon);
    let original_area = area.abs();

    if original_area < f32::EPSILON {
        return polygon.clone();
    }

    let centroid = polygon_centroid(polygon, area);
    let side = (centroid - line_start).dot(line_normal);
    let push_dir = if side > 0.0 {
        line_normal
    } else {
        -line_normal
    };

    let centroid_dist = point_to_segment_distance(centroid, line_start, line_end);

    if distance >= centroid_dist {
        return polygon.clone();
    }

    let line_vec = line_end - line_start;
    let line_len_sq = line_vec.length_squared();

    let shrunk = polygon
        .iter()
        .map(|&v| {
            let dist = point_to_segment_distance(v, line_start, line_end);
            if dist < distance * 2.0 {
                let t = (v - line_start).dot(line_vec) / line_len_sq;
                if t >= -0.1 && t <= 1.1 {
                    return v + push_dir * distance;
                }
            }

            v
        })
        .collect::<Polygon>();

    let shrunk_area = polygon_area(&shrunk).abs();

    if shrunk_area < original_area * 0.2 {
        polygon.clone()
    } else {
        shrunk
    }
}

fn polygon_bounds_diagonal(polygon: &Polygon) -> f32 {
    let min_x = polygon.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
    let max_x = polygon
        .iter()
        .map(|v| v.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = polygon.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
    let max_y = polygon
        .iter()
        .map(|v| v.y)
        .fold(f32::NEG_INFINITY, f32::max);

    ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_square() -> Polygon {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ]
    }

    #[test]
    fn test_bisect_produces_two_halves() {
        let sq = unit_square();
        let result = bisect_polygon(&sq, 0, 0.5, 0.0, 0.0);
        assert_eq!(result.len(), 2, "should produce exactly two halves");
    }

    #[test]
    fn test_bisect_areas_sum_to_original() {
        let sq = unit_square();
        let original_area = polygon_area(&sq).abs();
        let result = bisect_polygon(&sq, 0, 0.5, 0.0, 0.0);
        let total: f32 = result.iter().map(|p| polygon_area(p).abs()).sum();
        assert!(
            (total - original_area).abs() < 0.1,
            "areas should sum to original"
        );
    }

    #[test]
    fn test_bisect_degenerate_polygon() {
        let tri = vec![Vec2::ZERO, Vec2::ONE];
        let result = bisect_polygon(&tri, 0, 0.5, 0.0, 0.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], tri);
    }

    #[test]
    fn test_bisect_with_separation_reduces_area() {
        let sq = unit_square();
        let no_sep = bisect_polygon(&sq, 0, 0.5, 0.0, 0.0);
        let with_sep = bisect_polygon(&sq, 0, 0.5, 0.0, 0.2);

        let area_no_sep: f32 = no_sep.iter().map(|p| polygon_area(p).abs()).sum();
        let area_with_sep: f32 = with_sep.iter().map(|p| polygon_area(p).abs()).sum();

        assert!(
            area_with_sep < area_no_sep,
            "separation should reduce total area"
        );
    }

    #[test]
    fn test_bisect_out_of_bounds_index() {
        let sq = unit_square();
        let result = bisect_polygon(&sq, 99, 0.5, 0.0, 0.0);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_bisect_all_halves_have_positive_area() {
        let sq = unit_square();
        let result = bisect_polygon(&sq, 0, 0.5, 0.0, 0.0);
        for (i, poly) in result.iter().enumerate() {
            let area = polygon_area(poly).abs();
            assert!(area > 0.0, "half {i} has zero area");
        }
    }

    #[test]
    fn test_bisect_with_angle_offset_still_splits() {
        let sq = unit_square();
        let result = bisect_polygon(&sq, 0, 0.5, 0.3, 0.0);
        // May produce 1 or 2 depending on geometry, but must not panic
        assert!(!result.is_empty());
    }

    #[test]
    fn test_push_polygon_from_line_moves_vertices() {
        let sq = unit_square();
        let pushed = push_polygon_from_line(&sq, Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0), 0.5);
        assert!(polygon_area(&pushed).abs() > 0.1);
        assert_ne!(pushed, sq);
    }

    #[test]
    fn test_push_polygon_from_line_degenerate_fallback() {
        // Pushing by 100m should cause degeneration on a tiny triangle
        // and return the original unchanged
        let small = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.1, 0.0),
            Vec2::new(0.05, 0.1),
        ];
        let result =
            push_polygon_from_line(&small, Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0), 100.0);
        assert_eq!(result, small, "should return original on degenerate push");
    }

    #[test]
    fn test_push_polygon_preserves_vertex_count() {
        let sq = unit_square();
        let pushed = push_polygon_from_line(&sq, Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0), 0.3);
        assert_eq!(pushed.len(), sq.len());
    }

    #[test]
    fn test_polygon_bounds_diagonal_unit_square() {
        let sq = unit_square();
        let d = polygon_bounds_diagonal(&sq);
        // 2x2 square diagonal = 2*sqrt(2)
        assert!((d - 2.0 * 2.0f32.sqrt()).abs() < 1e-4);
    }
}
