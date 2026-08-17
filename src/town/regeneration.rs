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
                if seed_changed || event.rebuild_skeleton {
                    // Rebuild the boundary from params first, so vertex count
                    // and scale take effect; the pipeline itself reuses whatever
                    // boundary and road path the skeleton currently holds.
                    skeleton.boundary = rebuild_boundary(
                        params.boundary_vertex_count,
                        params.boundary_scale,
                        event.seed,
                        &[],
                    );
                    skeleton.boundary_offsets = vec![Vec2::ZERO; skeleton.boundary.len()];
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        app::{App, Update},
        asset::Assets,
        ecs::component::Component,
    };

    use crate::core::{Block, Building};

    /// A headless app wired with exactly what `handle_regeneration` touches.
    /// `Assets::default()` is self-contained, so no AssetPlugin is needed.
    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<RegenerateEvent>()
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .insert_resource(Seed(config::INITIAL_SEED))
            .insert_resource(Params::default())
            .insert_resource(crate::town::build_initial_skeleton(
                &Params::default(),
                config::INITIAL_SEED,
            ))
            .insert_resource(Is3D(true))
            .insert_resource(GenerationMode::default())
            .insert_resource(EditMode::default())
            .add_systems(Update, handle_regeneration);
        app
    }

    fn count<C: Component>(app: &mut App) -> usize {
        app.world_mut().query::<&C>().iter(app.world()).count()
    }

    #[test]
    fn test_regenerate_event_builds_a_town() {
        let mut app = test_app();
        assert_eq!(count::<Town>(&mut app), 0, "town exists before any event");

        app.world_mut().write_message(RegenerateEvent {
            seed: config::INITIAL_SEED,
            user_edit: false,
            rebuild_skeleton: false,
        });
        app.update();

        assert_eq!(count::<Town>(&mut app), 1);
        assert!(count::<Block>(&mut app) > 0, "no blocks were spawned");
        assert!(count::<Building>(&mut app) > 0, "no buildings were spawned");
    }

    #[test]
    fn test_regenerate_replaces_the_previous_town() {
        // Two events must not leave two towns stacked on top of each other.
        let mut app = test_app();

        for _ in 0..3 {
            app.world_mut().write_message(RegenerateEvent {
                seed: config::INITIAL_SEED,
                user_edit: false,
                rebuild_skeleton: false,
            });
            app.update();
        }

        assert_eq!(
            count::<Town>(&mut app),
            1,
            "regenerating repeatedly accumulated towns"
        );
    }

    #[test]
    fn test_randomize_path_changes_the_city() {
        // What the Randomize button does: new seed + full skeleton rebuild.
        let mut app = test_app();

        app.world_mut().write_message(RegenerateEvent {
            seed: config::INITIAL_SEED,
            user_edit: false,
            rebuild_skeleton: false,
        });
        app.update();
        let before = count::<Building>(&mut app);
        let boundary_before = app.world().resource::<SkeletonData>().boundary.clone();

        app.world_mut().write_message(RegenerateEvent {
            seed: 987_654_321,
            user_edit: false,
            rebuild_skeleton: true,
        });
        app.update();
        let after = count::<Building>(&mut app);
        let boundary_after = app.world().resource::<SkeletonData>().boundary.clone();

        assert_eq!(app.world().resource::<Seed>().0, 987_654_321);
        assert_ne!(
            boundary_before, boundary_after,
            "a new seed did not rebuild the skeleton"
        );
        assert!(after > 0, "the rebuilt city has no buildings");
        assert_ne!(before, after, "the rebuilt city is identical");
    }

    #[test]
    fn test_param_only_change_keeps_the_skeleton() {
        // Cheap path: sliders that only affect subdivision reuse the skeleton.
        let mut app = test_app();
        app.world_mut().write_message(RegenerateEvent {
            seed: config::INITIAL_SEED,
            user_edit: false,
            rebuild_skeleton: false,
        });
        app.update();
        let boundary_before = app.world().resource::<SkeletonData>().boundary.clone();

        app.world_mut().resource_mut::<Params>().min_building_area = 60.0;
        app.world_mut().write_message(RegenerateEvent {
            seed: config::INITIAL_SEED,
            user_edit: false,
            rebuild_skeleton: false,
        });
        app.update();

        assert_eq!(
            boundary_before,
            app.world().resource::<SkeletonData>().boundary,
            "a subdivision-only change rebuilt the skeleton"
        );
        assert!(count::<Building>(&mut app) > 0);
    }

    #[test]
    fn test_bigger_min_area_makes_fewer_buildings() {
        let build_with = |min_area: f32| {
            let mut app = test_app();
            app.world_mut().resource_mut::<Params>().min_building_area = min_area;
            app.world_mut().write_message(RegenerateEvent {
                seed: config::INITIAL_SEED,
                user_edit: false,
                rebuild_skeleton: false,
            });
            app.update();
            app.world_mut()
                .query::<&Building>()
                .iter(app.world())
                .count()
        };

        let small = build_with(8.0);
        let large = build_with(80.0);
        assert!(
            large < small,
            "raising min plot area did not reduce building count: {large} vs {small}"
        );
    }
}
