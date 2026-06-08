use bevy::math::{Vec2, Vec3, VectorSpace};
use spade::{DelaunayTriangulation, LastUsedVertexHintGenerator, Point2, Triangulation};

use crate::{
    config,
    core::SkeletonData,
    geometry::{calculate_circumcenter, point_in_polygon},
};

pub struct VoronoiResult {
    pub circumcenters: Vec<Vec3>,
    pub cells: Vec<Vec<usize>>,
}

pub fn build_voronoi(
    generator_points: &[Vec3],
    boundary_polygon: &[Vec2],
    merge_threshold: f32,
) -> VoronoiResult {
    let d_points = generator_points
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

    for pd in &d_points {
        triangulation.insert(*pd).ok();
    }

    let canvas_w = config::CANVS_WIDTH;
    let canvas_h = config::CANVAS_HEIGHT;
    let bound_margin = 2.0f32;

    let raw_circumcenters = triangulation
        .inner_faces()
        .map(|face| {
            let [v1, v2, v3] = face.vertices();
            let (cx, cz) = calculate_circumcenter(v1.position(), v2.position(), v3.position());
            let cx = cx as f32;
            let cz = cz as f32;

            if cx.abs() <= canvas_w * bound_margin && cz.abs() <= canvas_h * bound_margin {
                Vec3::new(cx, 0.0, cz)
            } else {
                let fallback_x =
                    ((v1.position().x + v2.position().x + v3.position().x) / 3.0) as f32;
                let fallback_z =
                    ((v1.position().y + v2.position().y + v3.position().y) / 3.0) as f32;

                Vec3::new(fallback_x, 0.0, fallback_z)
            }
        })
        .collect::<Vec<_>>();

    let (circumcenters, index_mapping) = merge_circumcenters(&raw_circumcenters, merge_threshold);

    let mut voronoi_circumcenters = vec![Vec::new(); d_points.len()];

    for (face_idx, face) in triangulation.inner_faces().enumerate() {
        let [v1, v2, v3] = face.vertices();

        let Some(&new_idx) = index_mapping.get(face_idx) else {
            continue;
        };

        for (gen_idx, d_pt) in d_points.iter().enumerate() {
            if v1.position() == *d_pt || v2.position() == *d_pt || v3.position() == *d_pt {
                if !voronoi_circumcenters[gen_idx].contains(&new_idx) {
                    voronoi_circumcenters[gen_idx].push(new_idx);
                }
            }
        }
    }

    let mut cells = Vec::new();
    let extreme_threshold = config::CANVS_WIDTH * 3.0;

    for (gen_idx, circumcenter_indices) in voronoi_circumcenters.iter().enumerate() {
        if circumcenter_indices.len() < 3 {
            continue;
        }

        let gen_pos = Vec2::new(d_points[gen_idx].x as f32, d_points[gen_idx].y as f32);
        if !point_in_polygon(&gen_pos, boundary_polygon) {
            continue;
        }

        let mut is_boundary = false;
        'outer: for face in triangulation.inner_faces() {
            let [v1, v2, v3] = face.vertices();
            if v1.position() == d_points[gen_idx]
                || v2.position() == d_points[gen_idx]
                || v3.position() == d_points[gen_idx]
            {
                for edge in face.adjacent_edges() {
                    if edge.face().is_outer() {
                        is_boundary = true;
                        break 'outer;
                    }
                }
            }
        }
        if is_boundary {
            continue;
        }

        let has_extreme = circumcenter_indices.iter().any(|&ci| {
            let c = circumcenters[ci];
            (c.x.powi(2) + c.x.powi(2)).sqrt() > extreme_threshold
        });
        if has_extreme {
            continue;
        }

        let mut sorted = circumcenter_indices.clone();
        sorted.sort_by(|&a, &b| {
            let a_pos = Vec2::new(circumcenters[a].x, circumcenters[a].z);
            let b_pos = Vec2::new(circumcenters[b].x, circumcenters[b].z);
            let angle_a = (a_pos.y - gen_pos.y).atan2(a_pos.x - gen_pos.x);
            let angle_b = (b_pos.y - gen_pos.y).atan2(b_pos.x - gen_pos.x);
            angle_a
                .partial_cmp(&angle_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        cells.push(sorted);
    }

    VoronoiResult {
        circumcenters,
        cells,
    }
}

fn merge_circumcenters(raw: &[Vec3], threshold: f32) -> (Vec<Vec3>, Vec<usize>) {
    let mut merged = Vec::new();
    let mut index_mapping = vec![0; raw.len()];
    let mut used = vec![false; raw.len()];

    for i in 0..raw.len() {
        if used[i] {
            continue;
        }

        let mut cluster = vec![];
        used[i] = true;

        for j in (i + 1)..raw.len() {
            if !used[j] && raw[i].distance(raw[j]) < threshold {
                cluster.push(j);
                used[j] = true;
            }
        }

        let avg = cluster
            .iter()
            .map(|&idx| raw[idx])
            .fold(Vec3::ZERO, |acc, p| acc + p)
            / cluster.len() as f32;

        let new_idx = merged.len();
        merged.push(avg);

        for &old_idx in &cluster {
            index_mapping[old_idx] = new_idx;
        }
    }

    (merged, index_mapping)
}

pub fn apply_voronoi_to_skeleton(
    skeleton: &mut SkeletonData,
    generators: Vec<Vec3>,
    result: VoronoiResult,
) {
    skeleton.generator_points = generators;
    skeleton.circumcenters = result.circumcenters;
    skeleton.cells = result.cells;
}
