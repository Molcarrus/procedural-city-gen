use bevy::{
    asset::Assets,
    camera::visibility::Visibility,
    color::Color,
    ecs::system::{Commands, ResMut},
    math::Vec2,
    mesh::{Mesh, Mesh3d},
    pbr::{MeshMaterial3d, StandardMaterial},
    transform::components::Transform,
    utils::default,
};
use bevy_egui::egui::FontSelection::Default;
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    config,
    core::{Block, Building, Params, SkeletonData, Town},
    generation::{
        generate_boundary_generators, generate_boundary_polygon, generate_road_generators,
        generate_spiral_points, relax_points,
    },
    rendering::{polygon_to_building, polygon_to_footprint},
    subdivision::{apply_road_corridor, subdivide_to_plots},
    voronoi::{apply_voronoi_to_skeleton, build_voronoi},
};

pub fn generate_town(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    seed: u64,
    params: &Params,
    skeleton: &mut ResMut<SkeletonData>,
    is_3d: bool,
) {
    if skeleton.circumcenters.is_empty() || skeleton.cells.is_empty() {
        return;
    }

    let town_entity = commands.spawn(Town { seed }).id();

    let raw_regions = skeleton
        .cells
        .iter()
        .map(|cell| {
            cell.iter()
                .map(|&idx| Vec2::new(skeleton.circumcenters[idx].x, skeleton.circumcenters[idx].z))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let road_generator_count = generate_road_generators(&skeleton.road_path).len();
    let polygonal_regions = apply_road_corridor(
        raw_regions,
        &skeleton.road_path,
        road_generator_count,
        config::ROAD_WIDTH * 0.5,
    );

    let mut building_id = 0u32;

    for (block_idx, block_polygon) in polygonal_regions.iter().enumerate() {
        if block_polygon.len() < 3 {
            continue;
        }

        let block_entity = commands
            .spawn(Block {
                polygon: block_polygon.clone(),
                id: block_idx as u32,
            })
            .id();
        commands.entity(town_entity).add_children(&[block_entity]);

        let mut block_rng = StdRng::seed_from_u64(seed.wrapping_add(block_idx as u64));
        let plots = subdivide_to_plots(
            block_polygon,
            params.min_building_area,
            params.grid_chaos,
            params.size_chaos,
            params.empty_prob,
            0,
            &mut block_rng,
            params.max_recursion_depth,
            params.alley_chance,
            params.alley_width,
        );

        let mut building_entities = Vec::new();

        for plot_polygon in plots {
            let wall_height =
                block_rng.random_range(params.min_wall_height..params.max_wall_height);

            let footprint_mesh = polygon_to_footprint(&plot_polygon);
            let building_mesh = polygon_to_building(&plot_polygon, wall_height);

            let footprint_handle = meshes.add(footprint_mesh);
            let building_handle = meshes.add(building_mesh);

            let base_r = (0.8 + block_rng.random_range(-0.05_f32..0.05)).clamp(0.0, 1.0);
            let base_g = (0.8 + block_rng.random_range(-0.05_f32..0.05)).clamp(0.0, 1.0);
            let base_b = (0.9 + block_rng.random_range(-0.05_f32..0.05)).clamp(0.0, 1.0);

            let footprint_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(base_r * 0.8, base_g * 0.8, base_b),
                alpha_mode: bevy::render::alpha::AlphaMode::Opaque,
                ..default()
            });

            let building_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(base_r, base_g, base_b),
                alpha_mode: bevy::render::alpha::AlphaMode::Opaque,
                ..default()
            });

            let building_entity = commands
                .spawn((
                    Building {
                        id: building_id,
                        footprint: plot_polygon,
                    },
                    Transform::default(),
                    Visibility::Visible,
                ))
                .id();

            let footprint_entity = commands
                .spawn((
                    Mesh3d(footprint_handle),
                    MeshMaterial3d(footprint_mat),
                    Transform::default(),
                    Visibility::Visible,
                ))
                .id();

            let building_3d_entity = commands
                .spawn((
                    Mesh3d(building_handle),
                    MeshMaterial3d(building_mat),
                    Transform::default(),
                    if is_3d {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                ))
                .id();

            commands
                .entity(building_entity)
                .add_children(&[footprint_entity, building_3d_entity]);

            building_entities.push(building_entity);
            building_id += 1;
        }

        commands
            .entity(block_entity)
            .add_children(&building_entities);
    }
}

pub fn run_generation_pipeline(skeleton: &mut ResMut<SkeletonData>, params: &Params, seed: u64) {
    let boundary_generators = generate_boundary_generators(
        &skeleton.boundary,
        params.boundary_spacing,
        params.boundary_inner_offset,
    );

    let road_generators = generate_road_generators(&skeleton.road_path);

    let regular = generate_spiral_points(
        params.generator_count,
        config::CANVS_WIDTH,
        config::CANVAS_HEIGHT,
        config::SPIRAL_SPREAD,
        seed,
    );

    let mut fixed = road_generators;
    fixed.extend(boundary_generators);

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

pub fn rebuild_boundary(vertex_count: usize, scale: f32, seed: u64, offsets: &[Vec2]) -> Vec<Vec2> {
    let mut base = generate_boundary_polygon(vertex_count, scale, seed);
    for (i, &offset) in offsets.iter().enumerate() {
        if i < base.len() {
            base[i] += offset;
        }
    }

    base
}
