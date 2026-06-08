use bevy::math::Vec2;

use crate::{
    core::Polygon,
    geometry::{
        line_segment_interaction, point_to_segment_distance, polygon_area, polygon_centroid,
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
        if let Some(pt) = line_segment_interaction(line_start, line_end, polygon[i], polygon[j]) {
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
    let centroid = polygon_centroid(polygon, area);
    let side = (centroid - line_start).dot(line_normal);
    let push_dir = if side > 0.0 {
        line_normal
    } else {
        -line_normal
    };

    let line_vec = line_end - line_start;
    let line_len_sq = line_vec.length_squared();

    let shrunk = polygon
        .iter()
        .map(|&v| {
            let dist = point_to_segment_distance(v, line_start, line_end);
            if dist < distance * 2.0 {
                let t = (v - line_start).dot(line_vec) / line_len_sq;
                if t >= -0.1 && t < 1.1 {
                    return v + push_dir * distance;
                }
            }

            v
        })
        .collect::<Polygon>();

    let shrunk_area = polygon_area(&shrunk).abs();
    let original_area = area.abs();
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
