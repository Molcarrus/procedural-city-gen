pub mod generator;
pub mod regeneration;

use bevy::{
    app::{Startup, Update},
    asset::Assets,
    ecs::system::{Commands, Res, ResMut},
    math::{Vec2, VectorSpace},
    mesh::Mesh,
    pbr::StandardMaterial,
};
pub use generator::*;
pub use regeneration::*;

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
        let params = Params::default();
        let seed = config::INITIAL_SEED;

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

        app.insert_resource(skeleton)
            .add_systems(Startup, startup_generate)
            .add_systems(Update, regeneration::handle_regeneration);
    }
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
