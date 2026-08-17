pub mod generator;
pub mod regeneration;

use bevy::{
    app::{Startup, Update},
    asset::Assets,
    ecs::system::{Commands, Res, ResMut},
    math::Vec2,
    mesh::Mesh,
    pbr::StandardMaterial,
};
pub use generator::*;

use crate::{
    config,
    core::{Is3D, Params, Seed, SkeletonData},
    generation::{
        generate_boundary_generators, generate_boundary_polygon, generate_spiral_points,
        relax_points,
    },
    voronoi::{apply_voronoi_to_skeleton, build_voronoi},
};

pub struct TownPlugin;

impl bevy::prelude::Plugin for TownPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        let skeleton = build_initial_skeleton(&Params::default(), config::INITIAL_SEED);

        app.insert_resource(skeleton)
            .add_systems(Startup, startup_generate)
            .add_systems(Update, regeneration::handle_regeneration);
    }
}

pub fn build_initial_skeleton(params: &Params, seed: u64) -> SkeletonData {
    let boundary =
        generate_boundary_polygon(params.boundary_vertex_count, params.boundary_scale, seed);

    let boundary_gens = generate_boundary_generators(
        &boundary,
        params.boundary_spacing,
        params.boundary_inner_offset,
    );

    let regular = generate_spiral_points(
        params.generator_count,
        config::CANVS_WIDTH,
        config::CANVAS_HEIGHT,
        config::SPIRAL_SPREAD,
        seed,
    );

    let all_generators = relax_points(
        regular,
        boundary_gens,
        4,
        config::CANVS_WIDTH,
        config::CANVAS_HEIGHT,
    );

    let voronoi = build_voronoi(
        &all_generators,
        &boundary,
        params.circumcenter_merge_threshold,
    );

    let mut skeleton = SkeletonData::new_empty(boundary.clone());
    apply_voronoi_to_skeleton(&mut skeleton, all_generators, voronoi);

    skeleton.boundary_offsets = vec![Vec2::ZERO; boundary.len()];

    skeleton
}

fn startup_generate(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    seed: Res<Seed>,
    params: Res<Params>,
    mut skeleton: ResMut<SkeletonData>,
    is_3d: Res<Is3D>,
) {
    generator::generate_town(
        &mut commands,
        &mut meshes,
        &mut materials,
        seed.0,
        &params,
        &mut skeleton,
        is_3d.0,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::polygon_area;

    /// The skeleton the app actually boots with, using the real config values.
    fn startup_skeleton() -> SkeletonData {
        build_initial_skeleton(&Params::default(), config::INITIAL_SEED)
    }

    #[test]
    fn test_startup_skeleton_is_valid() {
        let skeleton = startup_skeleton();
        assert!(
            skeleton.is_valid(),
            "startup skeleton is invalid: {} circumcenters, {} cells",
            skeleton.circumcenters.len(),
            skeleton.cells.len()
        );
    }

    #[test]
    fn test_startup_circumcenters_are_finite() {
        // A NaN circumcenter propagates into every mesh vertex and silently
        // kills rendering, so guard the whole set.
        let skeleton = startup_skeleton();
        for (i, c) in skeleton.circumcenters.iter().enumerate() {
            assert!(c.is_finite(), "circumcenter {i} is not finite: {c:?}");
        }
    }

    #[test]
    fn test_startup_cells_are_not_all_collapsed() {
        // A broken index mapping points every cell at circumcenter 0, which
        // still "looks" like valid data but has zero area.
        let skeleton = startup_skeleton();
        let distinct: std::collections::HashSet<usize> =
            skeleton.cells.iter().flatten().copied().collect();
        assert!(
            distinct.len() >= skeleton.cells.len(),
            "only {} distinct circumcenters across {} cells - indices look collapsed",
            distinct.len(),
            skeleton.cells.len()
        );
    }

    /// The block polygons the generator starts from, before street insetting.
    fn startup_blocks(skeleton: &SkeletonData) -> Vec<Vec<Vec2>> {
        skeleton
            .cells
            .iter()
            .map(|cell| {
                cell.iter()
                    .map(|&i| Vec2::new(skeleton.circumcenters[i].x, skeleton.circumcenters[i].z))
                    .collect()
            })
            .collect()
    }

    // --- the path the UI drives when a structural param changes ---

    #[test]
    fn test_boundary_vertex_count_reaches_the_city() {
        for sides in [3usize, 4, 6, 9] {
            let params = Params {
                boundary_vertex_count: sides,
                ..Params::default()
            };
            let skeleton = build_initial_skeleton(&params, config::INITIAL_SEED);
            assert_eq!(
                skeleton.boundary.len(),
                sides,
                "boundary_vertex_count {sides} did not reach the skeleton"
            );
        }
    }

    #[test]
    fn test_boundary_scale_reaches_the_city() {
        let small = build_initial_skeleton(
            &Params {
                boundary_scale: 40.0,
                ..Params::default()
            },
            config::INITIAL_SEED,
        );
        let large = build_initial_skeleton(
            &Params {
                boundary_scale: 120.0,
                ..Params::default()
            },
            config::INITIAL_SEED,
        );

        let radius = |s: &SkeletonData| {
            s.boundary.iter().map(|v| v.length()).fold(0.0f32, f32::max)
        };
        assert!(
            radius(&large) > radius(&small) * 2.0,
            "boundary_scale did not change the city size"
        );
    }

    #[test]
    fn test_more_generators_make_more_blocks() {
        let few = build_initial_skeleton(
            &Params {
                generator_count: 15,
                ..Params::default()
            },
            config::INITIAL_SEED,
        );
        let many = build_initial_skeleton(
            &Params {
                generator_count: 90,
                ..Params::default()
            },
            config::INITIAL_SEED,
        );
        assert!(
            many.cells.len() > few.cells.len(),
            "generator_count did not change block count: {} vs {}",
            many.cells.len(),
            few.cells.len()
        );
    }

    #[test]
    fn test_different_seed_gives_a_different_city() {
        let a = build_initial_skeleton(&Params::default(), 1);
        let b = build_initial_skeleton(&Params::default(), 2);
        assert_ne!(
            a.boundary, b.boundary,
            "changing the seed produced an identical boundary"
        );
    }

    #[test]
    fn test_every_param_default_survives_a_build() {
        // Guards against a default that silently produces an empty city.
        let skeleton = build_initial_skeleton(&Params::default(), config::INITIAL_SEED);
        assert!(skeleton.is_valid());
        assert!(!skeleton.cells.is_empty());
    }

    #[test]
    fn test_street_inset_shrinks_every_block() {
        use crate::geometry::inset_polygon;
        let skeleton = startup_skeleton();
        let half_street = config::STREET_WIDTH * 0.5;

        for (i, block) in startup_blocks(&skeleton).iter().enumerate() {
            if let Some(inset) = inset_polygon(block, half_street) {
                let before = polygon_area(block).abs();
                let after = polygon_area(&inset).abs();
                assert!(
                    after < before,
                    "block {i} did not shrink: {after} vs {before}"
                );
                for v in &inset {
                    assert!(v.is_finite(), "block {i} produced a non-finite vertex");
                }
            }
        }
    }

    #[test]
    fn test_street_inset_keeps_most_blocks() {
        // If the inset were dropping most blocks the city would be mostly street.
        use crate::geometry::inset_polygon;
        let skeleton = startup_skeleton();
        let blocks = startup_blocks(&skeleton);
        let half_street = config::STREET_WIDTH * 0.5;

        let kept = blocks
            .iter()
            .filter(|b| inset_polygon(b, half_street).is_some())
            .count();

        assert!(
            kept * 2 > blocks.len(),
            "street inset dropped most blocks: kept {kept} of {}",
            blocks.len()
        );
    }

    #[test]
    fn test_courtyard_stays_inside_its_block() {
        use crate::geometry::{point_in_polygon, scale_polygon_about_centroid};
        let skeleton = startup_skeleton();
        let ratio = config::COURTYARD_RATIO;

        for (i, block) in startup_blocks(&skeleton).iter().enumerate() {
            let courtyard = scale_polygon_about_centroid(block, ratio);
            assert!(
                polygon_area(&courtyard).abs() < polygon_area(block).abs(),
                "courtyard {i} is not smaller than its block"
            );
            for v in &courtyard {
                assert!(
                    point_in_polygon(v, block),
                    "courtyard {i} escaped its block at {v:?}"
                );
            }
        }
    }

    #[test]
    fn test_startup_cells_have_positive_area() {
        let skeleton = startup_skeleton();
        let mut with_area = 0;
        for cell in &skeleton.cells {
            let poly: Vec<Vec2> = cell
                .iter()
                .map(|&i| Vec2::new(skeleton.circumcenters[i].x, skeleton.circumcenters[i].z))
                .collect();
            let area = polygon_area(&poly).abs();
            assert!(area.is_finite(), "cell area is not finite");
            if area > 1.0 {
                with_area += 1;
            }
        }
        assert!(
            with_area > 0,
            "no block in the startup skeleton has a usable area"
        );
    }
}
