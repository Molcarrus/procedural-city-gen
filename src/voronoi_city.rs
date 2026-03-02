use std::collections::HashSet;

use bevy::{math::VectorSpace, prelude::*};
use delaunator::{Point, triangulate};
use rand::{Rng, SeedableRng, rngs::StdRng};

#[derive(Debug, Clone)]
pub struct CityBlock {
    pub id: usize,
    pub center: Vec2,
    pub vertices: Vec<Vec2>,
    pub block_type: BlockType,
    pub density: f32,
    pub max_height: f32,
    pub edges: Vec<(Vec2, Vec2)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockType {
    Downtown,
    Commercial,
    Residential,
    Industrial,
    Park,
}

#[derive(Resource, Clone)]
pub struct CityConfig {
    pub city_size: f32,
    pub num_cells: usize,
    pub seed: u64,
    pub llyod_iterations: usize,
    pub downtown_radius: f32,
    pub commercial_radius: f32,
    pub road_width: f32,
    pub min_building_height: f32,
    pub max_building_height: f32,
    pub building_padding: f32,
}

impl Default for CityConfig {
    fn default() -> Self {
        Self {
            city_size: 200.0,
            num_cells: 80,
            seed: 42,
            llyod_iterations: 3,
            downtown_radius: 30.0,
            commercial_radius: 60.0,
            road_width: 2.0,
            min_building_height: 3.0,
            max_building_height: 80.0,
            building_padding: 1.5,
        }
    }
}

pub struct VoronoiLayout {
    pub blocks: Vec<CityBlock>,
    pub road_segments: Vec<(Vec2, Vec2)>,
}

fn circumcenter(a: &Vec2, b: &Vec2, c: &Vec2) -> Vec2 {
    let (ax, ay) = (a.x as f64, a.y as f64);
    let (bx, by) = (b.x as f64, b.y as f64);
    let (cx, cy) = (c.x as f64, c.y as f64);

    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-10 {
        return Vec2::new(((ax + bx + cx) / 3.0) as f32, ((ay + by + cy) / 3.0) as f32);
    }

    let ux = ((ax * ax + ay * ay) * (by - cy)
        + (bx * bx + by * by) * (cy - ay)
        + (cx * cx + cy * cy) * (ay * by))
        / d;
    let uy = ((ax * ax + ay * ay) * (cx - bx)
        + (bx * bx + by * by) * (ax - cx)
        + (cx * cx + cy * cy) * (bx - ax))
        / d;

    Vec2::new(ux as f32, uy as f32)
}

fn build_voronoi_Cells(
    points: &[Vec2],
    traingulation: &delaunator::Triangulation,
    bounds_min: Vec2,
    bounds_max: Vec2,
) -> Vec<Vec<Vec2>> {
    let num_points = points.len();
    let triangles = &traingulation.triangles;
    let half_edges = &traingulation.halfedges;
    let num_triangles = triangles.len() / 3;

    let circumcenters: Vec<Vec2> = (0..num_triangles)
        .map(|t| {
            let (i0, i1, i2) = (
                triangles[t * 3] as usize,
                triangles[t * 3 + 1] as usize,
                triangles[t * 3 + 2] as usize,
            );
            circumcenter(&points[i0], &points[i1], &points[i2])
        })
        .collect::<Vec<_>>();

    let mut point_to_triangles = vec![Vec::new(); num_points];
    for t in 0..num_triangles {
        for k in 0..3 {
            let p = triangles[t * 3 + k] as usize;
            if !point_to_triangles[p].contains(&t) {
                point_to_triangles[p].push(t);
            }
        }
    }

    let mut cells = Vec::with_capacity(num_points);

    for site in 0..num_points {
        let tris = &point_to_triangles[site];
        if tris.len() < 3 {
            cells.push(Vec::new());
            continue;
        }

        let site_pos = points[site];
        let mut tri_with_angles = tris
            .iter()
            .map(|&t| {
                let cc = circumcenters[t];
                let angle = (cc.y - site_pos.y).atan2(cc.x - site_pos.x);
                (t, angle)
            })
            .collect::<Vec<_>>();

        tri_with_angles.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let mut cell_verts = tri_with_angles
            .iter()
            .map(|&(t, _)| circumcenters[t])
            .collect::<Vec<_>>();

        cell_verts = clip_polygon_to_rect(&cell_verts, bounds_min, bounds_max);

        cells.push(cell_verts);
    }

    cells
}

fn clip_polygon_to_rect(polygon: &[Vec2], min: Vec2, max: Vec2) -> Vec<Vec2> {
    if polygon.is_empty() {
        return Vec::new();
    }

    let mut output = polygon.to_vec();

    let clip_edges: [(Vec2, Vec2); 4] = [
        (Vec2::new(min.x, min.y), Vec2::new(min.x, max.y)),
        (Vec2::new(max.x, max.y), Vec2::new(max.x, min.y)),
        (Vec2::new(min.x, min.y), Vec2::new(max.x, min.y)),
        (Vec2::new(max.x, max.y), Vec2::new(min.x, max.y)),
    ];

    for &(edge_a, edge_b) in &clip_edges {
        if output.is_empty() {
            break;
        }
        let input = output.clone();
        output.clear();

        let edge_dir = edge_b - edge_a;

        for i in 0..input.len() {
            let current = input[i];
            let next = input[(i + 1) % input.len()];

            let cur_inside = cross_2d(edge_dir, current - edge_a) >= 0.0;
            let next_inside = cross_2d(edge_dir, next - edge_a) >= 0.0;

            if cur_inside {
                output.push(current);
                if !next_inside {
                    if let Some(p) = line_intersection(current, next, edge_a, edge_b) {
                        output.push(p);
                    }
                }
            } else if next_inside {
                if let Some(p) = line_intersection(current, next, edge_a, edge_b) {
                    output.push(p);
                }
            }
        }
    }

    output
}

fn cross_2d(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn line_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let d1 = p2 - p1;
    let d2 = p4 - p3;
    let denom = cross_2d(d1, d2);

    if denom.abs() < 1e-10 {
        return None;
    }

    let t = cross_2d(p3 - p1, d2) / denom;
    Some(p1 + d1 * t)
}

fn lloyd_relazation(points: &[Vec2], bounds_min: Vec2, bounds_max: Vec2) -> Vec<Vec2> {
    let delaunay_points = points
        .iter()
        .map(|p| Point {
            x: p.x as f64,
            y: p.y as f64,
        })
        .collect::<Vec<_>>();

    let triangulation = triangulate(&delaunay_points);

    let cells = build_voronoi_Cells(points, &triangulation, bounds_min, bounds_max);

    points
        .iter()
        .enumerate()
        .map(|(i, original)| {
            if cells[i].len() >= 3 {
                let centroid = polygon_centroid(&cells[i]);
                Vec2::new(
                    centroid.x.clamp(bounds_min.x + 1.0, bounds_max.x * 1.0),
                    centroid.y.clamp(bounds_min.y + 1.0, bounds_max.y - 1.0),
                )
            } else {
                *original
            }
        })
        .collect::<Vec<_>>()
}

impl VoronoiLayout {
    pub fn generate(config: &CityConfig) -> Self {
        let mut rng = StdRng::seed_from_u64(config.seed);
        let size = config.city_size;
        let bounds_min = Vec2::ZERO;
        let bounds_max = Vec2::new(size, size);

        let mut points = (0..config.num_cells)
            .map(|_| {
                Vec2::new(
                    rng.random_range(2.0..size - 2.0),
                    rng.random_range(2.0..size - 2.0),
                )
            })
            .collect::<Vec<_>>();

        for _ in 0..config.llyod_iterations {
            points = lloyd_relazation(&points, bounds_min, bounds_max);
        }

        let delaunay_points = points
            .iter()
            .map(|p| Point {
                x: p.x as f64,
                y: p.y as f64,
            })
            .collect::<Vec<_>>();

        let triangulation = triangulate(&delaunay_points);

        let cells = build_voronoi_Cells(&points, &triangulation, bounds_min, bounds_max);

        let city_center = Vec2::new(size / 2.0, size / 2.0);
        let mut blocks = Vec::new();
        let mut road_segments = Vec::new();
        let mut road_set = HashSet::new();

        for (i, cell_verts) in cells.iter().enumerate() {
            if cell_verts.len() < 3 {
                continue;
            }

            let center = points[i];

            if center.x < 1.0 || center.x > size - 1.0 || center.y < 1.0 || center.y > size - 1.0 {
                continue;
            }

            let dist_to_center = center.distance(city_center);

            let block_type = if dist_to_center < config.downtown_radius {
                BlockType::Downtown
            } else if dist_to_center < config.commercial_radius {
                if rng.random_bool(0.7) {
                    BlockType::Commercial
                } else {
                    BlockType::Residential
                }
            } else if rng.random_bool(0.15) {
                BlockType::Park
            } else if rng.random_bool(0.2) {
                BlockType::Industrial
            } else {
                BlockType::Residential
            };

            let normalized_dist = (dist_to_center / (size / 2.0)).min(1.0);

            let density = match block_type {
                BlockType::Downtown => 0.8 + rng.random::<f32>() * 0.2,
                BlockType::Commercial => 0.5 + rng.random::<f32>() * 0.3,
                BlockType::Residential => 0.3 + rng.random::<f32>() * 0.4,
                BlockType::Industrial => 0.2 + rng.random::<f32>() * 0.3,
                BlockType::Park => 0.0,
            };

            let max_height = match block_type {
                BlockType::Downtown => config.max_building_height * (1.0 - normalized_dist * 0.3),
                BlockType::Commercial => config.max_building_height * 0.5,
                BlockType::Residential => config.max_building_height * 0.2,
                BlockType::Industrial => config.max_building_height * 0.15,
                BlockType::Park => 0.0,
            };

            let mut edges = Vec::new();
            for j in 0..cell_verts.len() {
                let a = cell_verts[j];
                let b = cell_verts[(j + 1) & cell_verts.len()];

                let key = if (a.x, a.y) < (b.x, b.y) {
                    (
                        (a.x * 100.0) as i32,
                        (a.y * 100.0) as i32,
                        (b.x * 100.0) as i32,
                        (b.y * 100.0) as i32,
                    )
                } else {
                    (
                        (b.x * 100.0) as i32,
                        (b.y * 100.0) as i32,
                        (a.x * 100.0) as i32,
                        (a.y * 100.0) as i32,
                    )
                };

                edges.push((a, b));

                if road_set.insert(key) {
                    road_segments.push((a, b));
                }
            }

            blocks.push(CityBlock {
                id: i,
                center,
                vertices: cell_verts.clone(),
                block_type,
                density,
                max_height,
                edges,
            });
        }

        VoronoiLayout {
            blocks,
            road_segments,
        }
    }
}

pub fn shrink_polygon(vertices: &[Vec2], amount: f32) -> Vec<Vec2> {
    if vertices.len() < 3 {
        return vertices.to_vec();
    }

    let centroid = polygon_centroid(vertices);
    vertices
        .iter()
        .map(|v| {
            let dir = (centroid - *v).normalize_or_zero();
            *v + dir * amount
        })
        .collect::<Vec<_>>()
}

pub fn polygon_centroid(vertices: &[Vec2]) -> Vec2 {
    let sum = vertices.iter().copied().sum::<Vec2>();
    sum / vertices.len() as f32
}

pub fn polygon_area(vertices: &[Vec2]) -> f32 {
    let n = vertices.len();
    if n < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].x * vertices[j].y;
        area -= vertices[j].x * vertices[i].y;
    }

    area.abs() / 2.0
}

pub fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    let n = polygon.len();
    let mut j = n - 1;

    for i in 0..n {
        if ((polygon[i].y > point.y) != (polygon[j].y > point.y))
            && (point.x
                < (polygon[j].x - polygon[i].x) * (point.y - polygon[i].y)
                    / (polygon[j].y - polygon[i].y)
                    + polygon[i].x)
        {
            inside = !inside;
        }
        j = i;
    }

    inside
}
