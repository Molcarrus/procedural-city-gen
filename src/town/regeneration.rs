use bevy::{
    asset::Assets,
    ecs::{
        entity::Entity,
        message::MessageReader,
        query::With,
        system::{Commands, Query, Res, ResMut},
    },
    math::Vec2,
    mesh::Mesh,
    pbr::StandardMaterial,
};

use crate::{
    config,
    core::{EditMode, GenerationMode, Is3D, Params, RegenerateEvent, Seed, SkeletonData, Town},
    generation::{
        generate_boundary_generators, generate_road_generators, generate_spiral_points,
        relax_points,
    },
    town::{generate_town, rebuild_boundary, run_generation_pipeline},
    voronoi::{apply_voronoi_to_skeleton, build_voronoi},
};

pub fn handle_regeneration(
    mut commands: Commands,
    mut events: MessageReader<RegenerateEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut seed_res: ResMut<Seed>,
    params: Res<Params>,
    mut skeleton: ResMut<SkeletonData>,
    town_query: Query<Entity, With<Town>>,
    is_3d: Res<Is3D>,
    generation_mode: Res<GenerationMode>,
    edit_mode: Res<EditMode>,
) {
    for event in events.read() {
        for entity in town_query.iter() {
            commands.entity(entity).try_despawn();
        }

        let seed_changed = seed_res.0 != event.seed;
        seed_res.0 = event.seed;

        match *generation_mode {
            GenerationMode::Auto => {
                if seed_changed {
                    run_generation_pipeline(&mut skeleton, &params, event.seed);
                }
            }
            GenerationMode::Manual => {
                handle_manual_regeneration(
                    &mut skeleton,
                    &params,
                    event.seed,
                    event.user_edit,
                    *edit_mode,
                );
            }
        }

        generate_town(
            &mut commands,
            &mut meshes,
            &mut materials,
            event.seed,
            &params,
            &mut skeleton,
            is_3d.0,
        );
    }
}

fn handle_manual_regeneration(
    skeleton: &mut ResMut<SkeletonData>,
    params: &Params,
    seed: u64,
    user_edit: bool,
    edit_mode: EditMode,
) {
    match edit_mode {
        EditMode::Generators => {
            if !user_edit {
                let boundary_gens = generate_boundary_generators(
                    &skeleton.boundary,
                    params.boundary_spacing,
                    params.boundary_inner_offset,
                );
                let road_gens = generate_road_generators(&skeleton.road_path);
                let regular = generate_spiral_points(
                    params.generator_count,
                    config::CANVS_WIDTH,
                    config::CANVAS_HEIGHT,
                    config::SPIRAL_SPREAD,
                    seed,
                );
                let mut fixed = road_gens;
                fixed.extend(boundary_gens);
                skeleton.generator_points = relax_points(
                    regular,
                    fixed,
                    4,
                    config::CANVS_WIDTH,
                    config::CANVAS_HEIGHT,
                );
            }

            let voronoi = build_voronoi(
                &skeleton.generator_points,
                &skeleton.boundary,
                params.circumcenter_merge_threshold,
            );

            skeleton.circumcenters = voronoi.circumcenters;
            skeleton.cells = voronoi.cells;
        }
        EditMode::Circumcenters => {
            if !user_edit {
                let voronoi = build_voronoi(
                    &skeleton.generator_points,
                    &skeleton.boundary,
                    params.circumcenter_merge_threshold,
                );
                skeleton.circumcenters = voronoi.circumcenters;
                skeleton.cells = voronoi.cells;
            }
        }
        EditMode::Roads => {
            let boundary_gens = generate_boundary_generators(
                &skeleton.boundary,
                params.boundary_spacing,
                params.boundary_inner_offset,
            );
            let road_gens = generate_road_generators(&skeleton.road_path);
            let regular = generate_spiral_points(
                params.generator_count,
                config::CANVS_WIDTH,
                config::CANVAS_HEIGHT,
                config::SPIRAL_SPREAD,
                seed,
            );
            let mut fixed = road_gens;
            fixed.extend(boundary_gens);
            let all_generators = relax_points(
                regular,
                fixed,
                4,
                config::CANVS_WIDTH,
                config::CANVAS_HEIGHT,
            );

            let voronoi = build_voronoi(
                &all_generators,
                &skeleton.boundary,
                params.circumcenter_merge_threshold,
            );
            apply_voronoi_to_skeleton(skeleton, all_generators, voronoi);
        }
        EditMode::Boundary => {
            let current_count = skeleton.boundary.len();

            if current_count != params.boundary_vertex_count {
                skeleton.boundary_offsets = vec![Vec2::ZERO; params.boundary_vertex_count];
            }

            skeleton.boundary = rebuild_boundary(
                params.boundary_vertex_count,
                params.boundary_scale,
                seed,
                &skeleton.boundary_offsets,
            );

            let boundary_gens = generate_boundary_generators(
                &skeleton.boundary,
                params.boundary_spacing,
                params.boundary_inner_offset,
            );
            let road_gens = generate_road_generators(&skeleton.road_path);
            let regular = generate_spiral_points(
                params.generator_count,
                config::CANVS_WIDTH,
                config::CANVAS_HEIGHT,
                config::SPIRAL_SPREAD,
                seed,
            );

            let mut fixed = road_gens;
            fixed.extend(boundary_gens);
            let all_generators = relax_points(
                regular,
                fixed,
                4,
                config::CANVS_WIDTH,
                config::CANVAS_HEIGHT,
            );

            let voronoi = build_voronoi(
                &all_generators,
                &skeleton.boundary,
                params.circumcenter_merge_threshold,
            );

            apply_voronoi_to_skeleton(skeleton, all_generators, voronoi);
        }
    }
}
