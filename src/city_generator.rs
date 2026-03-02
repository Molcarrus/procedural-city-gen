use std::f32;

use bevy::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{
    building::{Building, BuildingParams, BuildingStyle, Park, spawn_building, spawn_park},
    road::{Road, spawn_roads},
    terrain::{Terrain, spawn_terrain},
    voronoi_city::{
        BlockType, CityBlock, CityConfig, VoronoiLayout, point_in_polygon, polygon_area,
        shrink_polygon,
    },
};

pub struct CityGeneratorPlugin;

impl Plugin for CityGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CityConfig>()
            .add_systems(Startup, generate_city)
            .add_systems(Update, regenerate_city);
    }
}

#[derive(Component)]
struct CityElement;

fn generate_city(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: ResMut<CityConfig>,
) {
    info!("Generating city with seed {}...", config.seed);

    let layout = VoronoiLayout::generate(&config);

    info!(
        "Generated {} blocks and {} road segments",
        layout.blocks.len(),
        layout.road_segments.len()
    );

    spawn_terrain(&mut commands, &mut meshes, &mut materials, config.city_size);

    spawn_roads(
        &mut commands,
        &mut meshes,
        &mut materials,
        &layout.road_segments,
        config.road_width,
    );

    let mut rng = StdRng::seed_from_u64(config.seed + 1000);

    for block in &layout.blocks {
        match block.block_type {
            BlockType::Park => {
                let area = polygon_area(&block.vertices);
                let radius = (area / std::f32::consts::PI).sqrt().min(8.0);
                spawn_park(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    Vec3::new(block.center.x, 0.0, block.center.y),
                    radius,
                    &mut rng,
                );
            }
            _ => {
                spawn_buildings_in_block(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    block,
                    &config,
                    &mut rng,
                );
            }
        }
    }

    spawn_voronoi_debug(&mut commands, &mut meshes, &mut materials, &layout, &config);

    info!("City generation complete!");
}

fn spawn_buildings_in_block(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    block: &CityBlock,
    config: &CityConfig,
    rng: &mut StdRng,
) {
    let shrunk = shrink_polygon(&block.vertices, config.building_padding + config.road_width);

    if shrunk.len() < 3 {
        return;
    }

    let area = polygon_area(&shrunk);
    if area < 10.0 {
        return;
    }

    let min_x = shrunk.iter().map(|v| v.x).fold(f32::MAX, f32::min);
    let max_x = shrunk.iter().map(|v| v.x).fold(f32::MIN, f32::max);
    let min_y = shrunk.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let max_y = shrunk.iter().map(|v| v.y).fold(f32::MIN, f32::max);

    let target_buildings = ((area * block.density * 0.02) as usize).max(1).min(12);

    let buildings_per_side = (target_buildings as f32).sqrt().ceil() as usize;
    let step_x = (max_x - min_x) / buildings_per_side as f32;
    let step_y = (max_y - min_y) / buildings_per_side as f32;

    let mut placed = 0;

    for gx in 0..buildings_per_side {
        for gy in 0..buildings_per_side {
            if placed >= target_buildings {
                break;
            }

            let jitter_x = rng.random_range(-step_x * 0.2..step_x * 0.2);
            let jitter_y = rng.random_range(-step_y * 0.2..step_y * 0.2);

            let pos = Vec2::new(
                min_x + step_x * (gx as f32 + 0.5) + jitter_x,
                min_y + step_y * (gy as f32 + 0.5) + jitter_y,
            );

            if point_in_polygon(pos, &shrunk) {
                continue;
            }

            let base_size = match block.block_type {
                BlockType::Downtown => rng.random_range(3.0..7.0),
                BlockType::Commercial => rng.random_range(4.0..8.0),
                BlockType::Residential => rng.random_range(3.0..6.0),
                BlockType::Industrial => rng.random_range(5.0..10.0),
                BlockType::Park => continue,
            };

            let width = base_size * rng.random_range(0.8..1.2);
            let depth = base_size * rng.random_range(0.8..1.2);

            let height = match block.block_type {
                BlockType::Downtown => rng.random_range(block.max_height * 0.4..block.max_height),
                BlockType::Commercial => {
                    rng.random_range(config.min_building_height..block.max_height)
                }
                BlockType::Residential => {
                    rng.random_range(config.min_building_height..block.max_height)
                }
                BlockType::Industrial => {
                    rng.random_range(config.min_building_height..block.max_height)
                }
                _ => continue,
            };

            let style = BuildingStyle::random(rng, block.block_type);
            let params = BuildingParams {
                position: Vec3::new(pos.x, 0.0, pos.y),
                width,
                depth,
                height,
                block_type: block.block_type,
                style,
            };

            spawn_building(commands, meshes, materials, &params, rng);
            placed += 1;
        }
    }
}

#[derive(Component)]
struct VoronoiDebug;

fn spawn_voronoi_debug(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    layout: &VoronoiLayout,
    _config: &CityConfig,
) {
    for block in &layout.blocks {
        let color = match block.block_type {
            BlockType::Downtown => Color::srgb(1.0, 0.2, 0.2),
            BlockType::Commercial => Color::srgb(0.2, 0.2, 1.0),
            BlockType::Residential => Color::srgb(0.2, 1.0, 0.2),
            BlockType::Industrial => Color::srgb(1.0, 1.0, 0.2),
            BlockType::Park => Color::srgb(0.0, 0.8, 0.3),
        };

        let mat = materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            unlit: true,
            ..default()
        });

        commands.spawn((
            VoronoiDebug,
            Mesh3d(meshes.add(Sphere::new(0.5).mesh().ico(1).unwrap())),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(block.center.x, 0.5, block.center.y)),
            Visibility::Hidden,
        ));
    }
}

fn regenerate_city(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<CityConfig>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    buildings: Query<Entity, With<Building>>,
    roads: Query<Entity, With<Road>>,
    terrain: Query<Entity, With<Terrain>>,
    parks: Query<Entity, With<Park>>,
    debug_vis: Query<Entity, With<VoronoiDebug>>,
    mut debug_query: Query<&mut Visibility, With<VoronoiDebug>>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        info!("Regenerating City...");
        config.seed = rand::rng().random();

        for entity in buildings
            .iter()
            .chain(roads.iter())
            .chain(terrain.iter())
            .chain(parks.iter())
            .chain(debug_vis.iter())
        {
            commands.entity(entity).despawn();
        }

        generate_city(commands, meshes, materials, config);
    }

    if keyboard.just_pressed(KeyCode::KeyV) {
        for mut vis in debug_query.iter_mut() {
            *vis = match *vis {
                Visibility::Hidden => Visibility::Visible,
                _ => Visibility::Hidden,
            };
        }
    }
}
