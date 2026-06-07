use bevy::math::{Vec2, VectorSpace};
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

pub fn line_segment_interaction(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
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

pub fn calculate_circumcenter(p1: Point2<f64>, p2: Point2<f64>, p3: Point2<f64>) -> (f64, f64) {
    let ax = p1.x;
    let ay = p1.y;
    let bx = p2.x;
    let by = p2.y;
    let cx = p3.x;
    let cy = p3.y;

    let d = 2.0 * (ax * (by - cy) + bx * (cy * ay) + cx * (ay - by));

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
