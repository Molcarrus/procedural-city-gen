use bevy::{
    asset::Assets,
    camera::visibility::Visibility,
    color::Color,
    ecs::{entity::Entity, system::{Commands, ResMut}},
    math::Vec2,
    mesh::{Mesh, Mesh3d},
    pbr::{MeshMaterial3d, StandardMaterial, wireframe::NoWireframe},
    transform::components::Transform,
    utils::default,
};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    config,
    core::{
        Block, Building, Courtyard, OpenSpace, OpenSpaceKind, Params, Polygon, SkeletonData,
        StreetSurface, Town, Water,
    },
    generation::{
        generate_boundary_generators, generate_boundary_polygon, generate_road_generators,
        generate_spiral_points, relax_points,
    },
    geometry::{
        inset_polygon, point_in_polygon, polygon_area, polygon_centroid,
        scale_polygon_about_centroid,
    },
    rendering::{polygon_to_building, polygon_to_footprint, polygon_to_footprint_at, water_plane},
    subdivision::{apply_road_corridor, subdivide_to_plots},
    voronoi::{apply_voronoi_to_skeleton, build_voronoi},
};

/// Vertical stacking of the flat surfaces. Kept apart by small deltas so
/// coplanar geometry never z-fights.
const BLOCK_PAVING_LEVEL: f32 = -0.03;
const OPEN_SPACE_LEVEL: f32 = -0.01;

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

    spawn_water(commands, meshes, materials, params, town_entity);
    spawn_street_surface(commands, meshes, materials, skeleton, town_entity);

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

    // Each block gives up half a street on every edge, so the gap between two
    // neighbouring blocks adds up to one full street width.
    let half_street = (params.street_width * 0.5).max(0.0);

    let mut building_id = 0u32;

    for (block_idx, raw_block) in polygonal_regions.iter().enumerate() {
        if raw_block.len() < 3 {
            continue;
        }

        // A block too small to give up its street frontage is dropped rather
        // than left at full size overlapping its neighbours.
        let Some(block_polygon) = inset_polygon(raw_block, half_street) else {
            continue;
        };

        let block_entity = commands
            .spawn(Block {
                polygon: block_polygon.clone(),
                id: block_idx as u32,
            })
            .id();
        commands.entity(town_entity).add_children(&[block_entity]);

        // Paving under the block so it reads as distinct from the street.
        let paving = spawn_flat(
            commands,
            meshes,
            materials,
            &block_polygon,
            BLOCK_PAVING_LEVEL,
            Color::srgb(0.30, 0.29, 0.31),
        );
        commands.entity(block_entity).add_children(&[paving]);

        let mut block_rng = StdRng::seed_from_u64(seed.wrapping_add(block_idx as u64));

        // Some blocks are reserved entirely as open space and carry no buildings.
        if block_rng.random::<f32>() < params.plaza_chance {
            let kind = if block_rng.random::<f32>() < params.park_ratio {
                OpenSpaceKind::Park
            } else {
                OpenSpaceKind::Plaza
            };

            let mesh_entity = spawn_flat(
                commands,
                meshes,
                materials,
                &block_polygon,
                OPEN_SPACE_LEVEL,
                open_space_color(kind),
            );

            let open_entity = commands
                .spawn((
                    OpenSpace {
                        block_id: block_idx as u32,
                        kind,
                        polygon: block_polygon.clone(),
                    },
                    Transform::default(),
                    Visibility::Visible,
                ))
                .id();

            commands.entity(open_entity).add_children(&[mesh_entity]);
            commands.entity(block_entity).add_children(&[open_entity]);
            continue;
        }

        // Other blocks may keep their middle clear, so buildings ring a courtyard.
        let courtyard = if block_rng.random::<f32>() < params.courtyard_chance {
            let ratio = params.courtyard_ratio.clamp(0.05, 0.9);
            Some(scale_polygon_about_centroid(&block_polygon, ratio))
        } else {
            None
        };

        let plots = subdivide_to_plots(
            &block_polygon,
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
            let plot_area = polygon_area(&plot_polygon).abs();
            if plot_polygon.len() < 3 || plot_area < 0.5 {
                continue;
            }

            // Drop plots that fall in the cleared middle.
            if let Some(open_middle) = &courtyard {
                let centre = polygon_centroid(&plot_polygon, polygon_area(&plot_polygon));
                if point_in_polygon(&centre, open_middle) {
                    continue;
                }
            }

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
                        height: wall_height,
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

        if let Some(open_middle) = courtyard {
            let mesh_entity = spawn_flat(
                commands,
                meshes,
                materials,
                &open_middle,
                OPEN_SPACE_LEVEL,
                Color::srgb(0.32, 0.40, 0.30),
            );

            let courtyard_entity = commands
                .spawn((
                    Courtyard {
                        block_id: block_idx as u32,
                        polygon: open_middle,
                    },
                    Transform::default(),
                    Visibility::Visible,
                ))
                .id();

            commands
                .entity(courtyard_entity)
                .add_children(&[mesh_entity]);
            building_entities.push(courtyard_entity);
        }

        commands
            .entity(block_entity)
            .add_children(&building_entities);
    }
}

fn open_space_color(kind: OpenSpaceKind) -> Color {
    match kind {
        OpenSpaceKind::Park => Color::srgb(0.24, 0.46, 0.26),
        OpenSpaceKind::Plaza => Color::srgb(0.58, 0.55, 0.50),
    }
}

/// Spawns a flat coloured polygon at `y` and returns its entity.
fn spawn_flat(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    polygon: &Polygon,
    y: f32,
    color: Color,
) -> Entity {
    let mesh = meshes.add(polygon_to_footprint_at(polygon, y));
    let material = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.95,
        alpha_mode: bevy::render::alpha::AlphaMode::Opaque,
        ..default()
    });

    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Visible,
        ))
        .id()
}

/// The paved surface the blocks sit on. Streets are the gaps between blocks,
/// so this is what shows through them.
fn spawn_street_surface(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    skeleton: &SkeletonData,
    town_entity: Entity,
) {
    if skeleton.boundary.len() < 3 {
        return;
    }

    // Grown slightly past the boundary so blocks touching the edge still sit
    // on paving rather than hanging over open water.
    let extent = skeleton
        .boundary
        .iter()
        .map(|v| v.length())
        .fold(0.0f32, f32::max)
        .max(1.0);
    let factor = 1.0 + config::STREET_SURFACE_MARGIN / extent;
    let surface = scale_polygon_about_centroid(&skeleton.boundary, factor);

    let entity = spawn_flat(
        commands,
        meshes,
        materials,
        &surface,
        config::STREET_LEVEL,
        Color::srgb(0.16, 0.16, 0.18),
    );

    // Its triangulation is a meshing artifact, not part of the city.
    commands.entity(entity).insert((StreetSurface, NoWireframe));
    commands.entity(town_entity).add_children(&[entity]);
}

fn spawn_water(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    params: &Params,
    town_entity: Entity,
) {
    if !params.water_enabled {
        return;
    }

    let mesh = meshes.add(water_plane(config::WATER_EXTENT, params.water_level));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.18, 0.34),
        perceptual_roughness: 0.15,
        metallic: 0.3,
        alpha_mode: bevy::render::alpha::AlphaMode::Opaque,
        ..default()
    });

    let entity = commands
        .spawn((
            Water,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Visible,
            // Without this the quad's diagonal draws a line across the whole scene.
            NoWireframe,
        ))
        .id();

    commands.entity(town_entity).add_children(&[entity]);
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

