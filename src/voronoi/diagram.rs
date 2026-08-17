use bevy::math::{Vec2, Vec3};
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
            (c.x.powi(2) + c.z.powi(2)).sqrt() > extreme_threshold
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

        // The cluster must contain `i` itself: otherwise a circumcenter with no
        // near neighbours averages an empty set (0/0 -> NaN) and never gets an
        // entry in `index_mapping`.
        let mut cluster = vec![i];
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::point_gen::{
        generate_boundary_generators, generate_boundary_polygon, generate_spiral_points,
        relax_points,
    };

    fn make_test_skeleton() -> (Vec<Vec3>, Vec<Vec2>) {
        let seed = 42u64;
        let boundary = generate_boundary_polygon(6, 50.0, seed);
        let boundary_gens = generate_boundary_generators(&boundary, 12.0, 1.0);
        let regular = generate_spiral_points(20, 500.0, 500.0, 3.0, seed);
        let all = relax_points(regular, boundary_gens, 2, 500.0, 500.0);
        (all, boundary)
    }

    #[test]
    fn test_build_voronoi_produces_cells() {
        let (generators, boundary) = make_test_skeleton();
        let result = build_voronoi(&generators, &boundary, 0.01);
        assert!(!result.cells.is_empty(), "should produce at least one cell");
    }

    #[test]
    fn test_build_voronoi_cells_have_minimum_vertices() {
        let (generators, boundary) = make_test_skeleton();
        let result = build_voronoi(&generators, &boundary, 0.01);
        for cell in &result.cells {
            assert!(cell.len() >= 3, "each cell must have at least 3 vertices");
        }
    }

    #[test]
    fn test_build_voronoi_cell_indices_in_bounds() {
        let (generators, boundary) = make_test_skeleton();
        let result = build_voronoi(&generators, &boundary, 0.01);
        let num_circumcenters = result.circumcenters.len();
        for cell in &result.cells {
            for &idx in cell {
                assert!(
                    idx < num_circumcenters,
                    "cell index {idx} out of bounds (total: {num_circumcenters})"
                );
            }
        }
    }

    #[test]
    fn test_build_voronoi_circumcenters_nonempty() {
        let (generators, boundary) = make_test_skeleton();
        let result = build_voronoi(&generators, &boundary, 0.01);
        assert!(!result.circumcenters.is_empty());
    }

    #[test]
    fn test_build_voronoi_deterministic() {
        let (generators, boundary) = make_test_skeleton();
        let r1 = build_voronoi(&generators, &boundary, 0.01);
        let r2 = build_voronoi(&generators, &boundary, 0.01);
        assert_eq!(r1.cells.len(), r2.cells.len());
        assert_eq!(r1.circumcenters.len(), r2.circumcenters.len());
    }

    #[test]
    fn test_build_voronoi_empty_generators() {
        let boundary = generate_boundary_polygon(4, 50.0, 1);
        let result = build_voronoi(&[], &boundary, 0.01);
        assert!(result.cells.is_empty());
        assert!(result.circumcenters.is_empty());
    }

    #[test]
    fn test_merge_circumcenters_no_merge_needed() {
        // Points far apart should not be merged
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(20.0, 0.0, 0.0),
        ];
        let (merged, mapping) = merge_circumcenters(&pts, 0.01);
        assert_eq!(merged.len(), 3);
        assert_eq!(mapping.len(), 3);
    }

    #[test]
    fn test_merge_circumcenters_all_merge() {
        // Points very close together should merge into one
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.001, 0.0, 0.0),
            Vec3::new(0.002, 0.0, 0.0),
        ];
        let (merged, _mapping) = merge_circumcenters(&pts, 0.1);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_merge_circumcenters_singletons_keep_position() {
        // A point with no near neighbour must keep its own coordinates,
        // not become NaN from averaging an empty cluster.
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 5.0),
            Vec3::new(20.0, 0.0, -5.0),
        ];
        let (merged, mapping) = merge_circumcenters(&pts, 0.01);
        assert_eq!(merged.len(), 3);
        for (i, m) in merged.iter().enumerate() {
            assert!(m.is_finite(), "merged[{i}] is not finite: {m:?}");
        }
        // Identity mapping when nothing merges.
        for (i, &m) in mapping.iter().enumerate() {
            assert_eq!(m, i, "mapping[{i}] should be {i}");
        }
        assert!((merged[1].x - 10.0).abs() < 1e-5);
        assert!((merged[1].z - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_merge_circumcenters_mapping_points_at_own_cluster() {
        // Two tight pairs, far apart from each other.
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.001, 0.0, 0.0),
            Vec3::new(50.0, 0.0, 0.0),
            Vec3::new(50.001, 0.0, 0.0),
        ];
        let (merged, mapping) = merge_circumcenters(&pts, 0.1);
        assert_eq!(merged.len(), 2);
        // Every raw index maps to a cluster whose position is near the original.
        for (raw_idx, &new_idx) in mapping.iter().enumerate() {
            assert!(new_idx < merged.len());
            assert!(
                pts[raw_idx].distance(merged[new_idx]) < 0.1,
                "raw {raw_idx} mapped to a distant cluster"
            );
        }
    }

    #[test]
    fn test_merge_circumcenters_averaged_position() {
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0), // identical
        ];
        let (merged, _) = merge_circumcenters(&pts, 0.1);
        assert_eq!(merged.len(), 1);
        assert!((merged[0].x).abs() < 1e-5);
    }

    #[test]
    fn test_apply_voronoi_to_skeleton() {
        let boundary = generate_boundary_polygon(4, 50.0, 1);
        let mut skeleton = SkeletonData::new_empty(boundary.clone());

        let generators = vec![Vec3::new(1.0, 0.0, 2.0)];
        let result = VoronoiResult {
            circumcenters: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            cells: vec![vec![0, 1, 2]],
        };

        apply_voronoi_to_skeleton(&mut skeleton, generators.clone(), result);

        assert_eq!(skeleton.generator_points.len(), 1);
        assert_eq!(skeleton.circumcenters.len(), 3);
        assert_eq!(skeleton.cells.len(), 1);
        // Road path and boundary should be untouched
        assert!(skeleton.road_path.is_empty());
        assert_eq!(skeleton.boundary, boundary);
    }

    #[test]
    fn test_skeleton_is_valid_after_voronoi() {
        let (generators, boundary) = make_test_skeleton();
        let mut skeleton = SkeletonData::new_empty(boundary.clone());
        let result = build_voronoi(&generators, &boundary, 0.01);
        apply_voronoi_to_skeleton(&mut skeleton, generators, result);
        assert!(skeleton.is_valid());
    }

    #[test]
    fn test_skeleton_invalid_when_empty() {
        let boundary = generate_boundary_polygon(4, 50.0, 1);
        let skeleton = SkeletonData::new_empty(boundary);
        assert!(!skeleton.is_valid());
    }
}
