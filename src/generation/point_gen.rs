use bevy::math::{Vec2, Vec3};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use spade::{DelaunayTriangulation, LastUsedVertexHintGenerator, Point2, Triangulation};

use crate::{
    config,
    core::Polygon,
    geometry::{calculate_circumcenter, polygon_area, polygon_centroid},
};

pub fn generate_spiral_points(
    count: usize,
    width: f32,
    height: f32,
    spread: f32,
    seed: u64,
) -> Vec<Vec3> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut points = Vec::with_capacity(count);

    for i in 0..count {
        let t = i as f32;
        let angle = t * 0.5 + rng.random_range(-0.3..0.3);
        let radius = t * spread + rng.random_range(-spread * 0.2..spread * 0.2);

        let x = (angle.cos() * radius).clamp(-width, width);
        let z = (angle.sin() * radius).clamp(-height, height);

        points.push(Vec3::new(x, 0.0, z));
    }

    points
}

pub fn generate_boundary_polygon(num_vertices: usize, base_radius: f32, seed: u64) -> Polygon {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut vertices = Vec::with_capacity(num_vertices);

    for i in 0..num_vertices {
        let angle = (i as f32 / num_vertices as f32) * std::f32::consts::TAU;
        let variation = rng.random_range(-0.2..0.2);
        let radius = base_radius * (1.0 + variation);

        vertices.push(Vec2::new(angle.cos() * radius, angle.sin() * radius));
    }

    vertices
}

pub fn generate_boundary_generators(
    boundary: &[Vec2],
    spacing: f32,
    inner_offset: f32,
) -> Vec<Vec3> {
    let mut generators = Vec::new();
    let outer_offset = config::BOUNDARY_GENERATOR_OUTER_OFFSET;

    let signed_area = {
        let n = boundary.len();
        let mut area = 0.0f32;
        for i in 0..n {
            let j = (i + 1) % n;
            area += boundary[i].x * boundary[j].y - boundary[j].x * boundary[i].y;
        }
        area
    };
    let is_ccw = signed_area > 0.0;

    for i in 0..boundary.len() {
        let start = boundary[i];
        let end = boundary[(i + 1) % boundary.len()];
        let edge_vec = end - start;
        let edge_len = edge_vec.length();

        if edge_len < 0.001 {
            continue;
        }

        let edge_dir = edge_vec / edge_len;

        let left_normal = Vec2::new(-edge_dir.y, edge_dir.x);
        let inward_normal = if is_ccw { left_normal } else { -left_normal };
        let outward_normal = -inward_normal;

        let num_points = ((edge_len / spacing).max(1.0)) as usize;

        for j in 0..num_points {
            let t = (j as f32 + 0.5) / num_points as f32;
            let on_edge = start + edge_vec * t;

            let inner = on_edge + inward_normal * inner_offset;
            let outer = on_edge + outward_normal * outer_offset;

            generators.push(Vec3::new(inner.x, 0.0, inner.y));
            generators.push(Vec3::new(outer.x, 0.0, outer.y));
        }
    }

    generators
}

pub fn generate_road_generators(road_path: &[Vec3]) -> Vec<Vec3> {
    if road_path.len() < 2 {
        return Vec::new();
    }

    let spacing = config::ROAD_GENERATOR_SPACING;
    let offset = config::ROAD_GENERATOR_OFFSET;
    let corner_distance = config::CORNER_CONSTRAINT_DISTANCE;

    let mut generators = Vec::new();

    for i in 0..(road_path.len() - 1) {
        let start = road_path[i];
        let end = road_path[i + 1];

        let edge_vec = end - start;
        let edge_len = edge_vec.length();
        if edge_len < 0.001 {
            continue;
        }

        let edge_dir = edge_vec / edge_len;
        let perp = Vec3::new(-edge_dir.z, 0.0, edge_dir.x);

        let seg_start = if i > 0 { corner_distance } else { 0.0 };
        let seg_end = if i < road_path.len() - 2 {
            edge_len - corner_distance
        } else {
            edge_len
        };
        let seg_len = seg_end - seg_start;

        if seg_len < 0.1 {
            continue;
        }

        let num_pairs = (seg_len / spacing).ceil() as usize + 1;

        for j in 0..num_pairs {
            let t = if num_pairs == 1 {
                0.5
            } else {
                j as f32 / (num_pairs - 1) as f32
            };
            let local_t = seg_start + seg_len * t;
            let on_road = start + edge_dir * local_t;

            generators.push(on_road + perp * offset);
            generators.push(on_road - perp * offset);
        }
    }

    generators
}

pub fn relax_points(
    regular_points: Vec<Vec3>,
    fixed_points: Vec<Vec3>,
    steps: usize,
    width: f32,
    height: f32,
) -> Vec<Vec3> {
    let mut regular = regular_points;

    for _ in 0..steps {
        let mut all_points = regular.clone();
        all_points.extend_from_slice(&fixed_points);

        let d_points = all_points
            .iter()
            .map(|p| Point2::new(p.x as f64, p.z as f64))
            .collect::<Vec<_>>();

        let mut triangulation: DelaunayTriangulation<
            Point2<f64>,
            (),
            (),
            (),
            LastUsedVertexHintGenerator,
        > = DelaunayTriangulation::new();

        for pt in &d_points {
            triangulation.insert(*pt).ok();
        }

        let circumcenters = triangulation
            .inner_faces()
            .map(|face| {
                let [v1, v2, v3] = face.vertices();
                calculate_circumcenter(v1.position(), v2.position(), v3.position())
            })
            .collect::<Vec<_>>();

        for (pt_idx, d_pt) in d_points.iter().enumerate().take(regular.len()) {
            let mut cell_verts = Vec::new();

            for (face_idx, face) in triangulation.inner_faces().enumerate() {
                let [v1, v2, v3] = face.vertices();
                if v1.position() == *d_pt || v2.position() == *d_pt || v3.position() == *d_pt {
                    let (cx, cy) = circumcenters[face_idx];
                    cell_verts.push(Vec2::new(cx as f32, cy as f32));
                }
            }

            if cell_verts.len() < 3 {
                continue;
            }

            let center =
                cell_verts.iter().fold(Vec2::ZERO, |acc, &p| acc + p) / cell_verts.len() as f32;

            cell_verts.sort_by(|a, b| {
                let angle_a = (a.y - center.y).atan2(a.x - center.x);
                let angle_b = (b.y - center.y).atan2(b.x - center.x);
                angle_a
                    .partial_cmp(&angle_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let area = polygon_area(&cell_verts);
            if area.abs() < f32::EPSILON {
                continue;
            }

            let centroid = polygon_centroid(&cell_verts, area);
            regular[pt_idx] = Vec3::new(
                centroid.x.clamp(-width, width),
                0.0,
                centroid.y.clamp(-height, height),
            );
        }
    }

    let mut result = regular;
    result.extend_from_slice(&fixed_points);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_spiral_points_count() {
        let pts = generate_spiral_points(20, 500.0, 500.0, 3.0, 42);
        assert_eq!(pts.len(), 20);
    }

    #[test]
    fn test_generate_spiral_points_within_bounds() {
        let pts = generate_spiral_points(50, 500.0, 500.0, 3.0, 42);
        for p in &pts {
            assert!(p.x.abs() <= 500.0);
            assert!(p.z.abs() <= 500.0);
            assert_eq!(p.y, 0.0);
        }
    }

    #[test]
    fn test_generate_spiral_points_deterministic() {
        let a = generate_spiral_points(10, 500.0, 500.0, 3.0, 99);
        let b = generate_spiral_points(10, 500.0, 500.0, 3.0, 99);
        for (pa, pb) in a.iter().zip(b.iter()) {
            assert_eq!(pa, pb);
        }
    }

    #[test]
    fn test_generate_boundary_polygon_count() {
        let poly = generate_boundary_polygon(6, 50.0, 42);
        assert_eq!(poly.len(), 6);
    }

    #[test]
    fn test_generate_boundary_polygon_roughly_circular() {
        let poly = generate_boundary_polygon(8, 50.0, 42);
        for v in &poly {
            // With ±20% variation the radius stays within 40..60
            let r = v.length();
            assert!(r > 35.0 && r < 65.0, "radius out of range: {r}");
        }
    }

    #[test]
    fn test_generate_boundary_polygon_deterministic() {
        let a = generate_boundary_polygon(4, 75.0, 12345);
        let b = generate_boundary_polygon(4, 75.0, 12345);
        assert_eq!(a, b);
    }

    #[test]
    fn test_generate_boundary_generators_nonempty() {
        let poly = generate_boundary_polygon(4, 50.0, 1);
        let gens = generate_boundary_generators(&poly, 12.0, 1.0);
        assert!(!gens.is_empty());
        // Each edge produces at least 2 generators (inner + outer)
        assert!(gens.len() >= poly.len() * 2);
    }

    #[test]
    fn test_generate_boundary_generators_all_at_y_zero() {
        let poly = generate_boundary_polygon(4, 50.0, 1);
        let gens = generate_boundary_generators(&poly, 12.0, 1.0);
        for g in &gens {
            assert_eq!(g.y, 0.0);
        }
    }

    #[test]
    fn test_generate_road_generators_empty_for_short_path() {
        let path = vec![Vec3::new(0.0, 0.0, 0.0)];
        let gens = generate_road_generators(&path);
        assert!(gens.is_empty());
    }

    #[test]
    fn test_generate_road_generators_produces_pairs() {
        let path = vec![Vec3::new(-20.0, 0.0, 0.0), Vec3::new(20.0, 0.0, 0.0)];
        let gens = generate_road_generators(&path);
        // Must be even: always generated in pairs
        assert!(gens.len() % 2 == 0);
        assert!(!gens.is_empty());
    }

    #[test]
    fn test_relax_points_preserves_count() {
        let regular = generate_spiral_points(10, 500.0, 500.0, 3.0, 42);
        let fixed: Vec<Vec3> = vec![Vec3::new(0.0, 0.0, 0.0)];
        let result = relax_points(regular, fixed.clone(), 2, 500.0, 500.0);
        // total = regular + fixed
        assert_eq!(result.len(), 10 + fixed.len());
    }

    #[test]
    fn test_relax_points_fixed_unchanged() {
        let regular = generate_spiral_points(10, 500.0, 500.0, 3.0, 42);
        let fixed = vec![Vec3::new(99.0, 0.0, 99.0)];
        let result = relax_points(regular, fixed.clone(), 3, 500.0, 500.0);
        // Fixed points are appended at the end
        let last = *result.last().unwrap();
        assert!((last.x - 99.0).abs() < 0.01);
        assert!((last.z - 99.0).abs() < 0.01);
    }

    #[test]
    fn test_relax_points_within_bounds() {
        let regular = generate_spiral_points(20, 500.0, 500.0, 3.0, 7);
        let fixed = vec![];
        let result = relax_points(regular, fixed, 2, 500.0, 500.0);
        for p in &result {
            assert!(p.x.abs() <= 500.0);
            assert!(p.z.abs() <= 500.0);
        }
    }
}
