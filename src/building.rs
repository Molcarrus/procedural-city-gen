use bevy::prelude::*;
use rand::{Rng, rngs::StdRng};

use crate::voronoi_city::BlockType;

#[derive(Component)]
pub struct Building;

#[derive(Component)]
pub struct Park;

#[derive(Debug, Clone)]
pub struct BuildingParams {
    pub position: Vec3,
    pub width: f32,
    pub depth: f32,
    pub height: f32,
    pub block_type: BlockType,
    pub style: BuildingStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum BuildingStyle {
    Box,
    Tiered,
    Tower,
    LShape,
    WithAntenna,
}

impl BuildingStyle {
    pub fn random(rng: &mut StdRng, block_type: BlockType) -> Self {
        match block_type {
            BlockType::Downtown => {
                let r = rng.random::<f32>();
                if r < 0.3 {
                    BuildingStyle::Tiered
                } else if r < 0.5 {
                    BuildingStyle::Tower
                } else if r < 0.65 {
                    BuildingStyle::WithAntenna
                } else {
                    BuildingStyle::Box
                }
            }
            BlockType::Commercial => {
                if rng.random_bool(0.3) {
                    BuildingStyle::LShape
                } else {
                    BuildingStyle::Box
                }
            }
            _ => BuildingStyle::Box,
        }
    }
}

pub fn spawn_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    params: &BuildingParams,
    rng: &mut StdRng,
) {
    let color = building_color(params.block_type, rng);
    let material = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.7,
        metallic: match params.block_type {
            BlockType::Downtown => 0.5,
            BlockType::Commercial => 0.3,
            _ => 0.1,
        },
        ..default()
    });

    match params.style {
        BuildingStyle::Box => {
            spawn_box_building(commands, meshes, &material, params);
        }
        BuildingStyle::Tiered => {
            spawn_tiered_building(commands, meshes, materials, params, rng);
        }
        BuildingStyle::Tower => {
            spawn_tower_building(commands, meshes, materials, params, rng);
        }
        BuildingStyle::LShape => {
            spawn_l_building(commands, meshes, &material, params);
        }
        BuildingStyle::WithAntenna => {
            spawn_antenna_building(commands, meshes, materials, &material, params);
        }
    }
}

fn spawn_box_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
    params: &BuildingParams,
) {
    let mesh = meshes.add(Cuboid::new(params.width, params.height, params.depth));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(params.position + Vec3::Y * params.height / 2.0),
        Building,
    ));
}

fn spawn_tiered_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    params: &BuildingParams,
    rng: &mut StdRng,
) {
    let tiers = rng.random_range(2..=4);
    let tier_height = params.height / tiers as f32;
    let mut current_y = 0.0;

    for i in 0..tiers {
        let scale = 1.0 - (i as f32 * 0.15);
        let w = params.width * scale;
        let d = params.depth * scale;

        let brightness_variation = rng.random_range(-0.05..0.05);
        let base_color = building_color(params.block_type, rng);
        let color = Color::srgb(
            (base_color.to_srgba().red + brightness_variation).clamp(0.0, 1.0),
            (base_color.to_srgba().green + brightness_variation).clamp(0.0, 1.0),
            (base_color.to_srgba().blue + brightness_variation).clamp(0.0, 1.0),
        );

        let mat = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.6,
            metallic: 0.4,
            ..default()
        });

        let mesh = meshes.add(Cuboid::new(w, tier_height, d));
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(
                params.position + Vec3::Y * (current_y + tier_height / 2.0),
            ),
            Building,
        ));

        current_y += tier_height;
    }
}

fn spawn_tower_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    params: &BuildingParams,
    rng: &mut StdRng,
) {
    let base_height = params.height * 0.2;
    let base_mesh = meshes.add(Cuboid::new(params.width, base_height, params.depth));
    let base_mat = materials.add(StandardMaterial {
        base_color: building_color(params.block_type, rng),
        perceptual_roughness: 0.7,
        metallic: 0.3,
        ..default()
    });

    commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(base_mat),
        Transform::from_translation(params.position + Vec3::Y * base_height / 2.0),
        Building,
    ));

    let (tower_w, tower_d, tower_h) = (params.width * 0.5, params.depth * 0.5, params.height * 0.8);
    let tower_mesh = meshes.add(Cuboid::new(tower_w, tower_h, tower_d));
    let tower_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.7, 0.85),
        perceptual_roughness: 0.3,
        metallic: 0.7,
        ..default()
    });
    commands.spawn((
        Mesh3d(tower_mesh),
        MeshMaterial3d(tower_mat),
        Transform::from_translation(params.position + Vec3::Y * (base_height + tower_h / 2.0)),
        Building,
    ));
}

fn spawn_l_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
    params: &BuildingParams,
) {
    let mesh1 = meshes.add(Cuboid::new(params.width, params.height, params.depth * 0.5));
    commands.spawn((
        Mesh3d(mesh1),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(
            params.position + Vec3::new(0.0, params.height / 2.0, params.depth * 0.25),
        ),
        Building,
    ));

    let mesh2 = meshes.add(Cuboid::new(
        params.width * 0.5,
        params.height,
        params.depth * 0.5,
    ));
    commands.spawn((
        Mesh3d(mesh2),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(
            params.position
                + Vec3::new(
                    params.width * 0.25,
                    params.height / 2.0,
                    -params.depth * 0.25,
                ),
        ),
        Building,
    ));
}

fn spawn_antenna_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    material: &Handle<StandardMaterial>,
    params: &BuildingParams,
) {
    let mesh = meshes.add(Cuboid::new(params.width, params.height, params.depth));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(params.position + Vec3::Y * params.height / 2.0),
        Building,
    ));

    let antenna_height = params.height * 0.3;
    let antenna_mesh = meshes.add(Cylinder::new(0.15, antenna_height));
    let antenna_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        emissive: Color::srgb(1.0, 0.2, 0.2).into(),
        ..default()
    });
    commands.spawn((
        Mesh3d(antenna_mesh),
        MeshMaterial3d(antenna_mat),
        Transform::from_translation(
            params.position + Vec3::Y * (params.height + antenna_height / 2.0),
        ),
        Building,
    ));
}

fn building_color(block_type: BlockType, rng: &mut StdRng) -> Color {
    match block_type {
        BlockType::Downtown => {
            let v = rng.random_range(0.4..0.8);
            let choices = [
                Color::srgb(v * 0.7, v * 0.8, v),
                Color::srgb(v, v, v),
                Color::srgb(v * 0.9, v * 0.85, v * 0.7),
            ];
            choices[rng.random_range(0..choices.len())]
        }
        BlockType::Commercial => {
            let v = rng.random_range(0.5..0.85);
            Color::srgb(v, v * 0.95, v * 0.9)
        }
        BlockType::Residential => {
            let choices = [
                Color::srgb(0.85, 0.75, 0.65),
                Color::srgb(0.8, 0.8, 0.75),
                Color::srgb(0.7, 0.65, 0.6),
                Color::srgb(0.9, 0.85, 0.8),
                Color::srgb(0.75, 0.3, 0.3),
            ];
            choices[rng.random_range(0..choices.len())]
        }
        BlockType::Industrial => {
            let v = rng.random_range(0.4..0.6);
            Color::srgb(v, v * 0.95, v * 0.9)
        }
        BlockType::Park => Color::srgb(0.2, 0.6, 0.2),
    }
}

pub fn spawn_park(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    radius: f32,
    rng: &mut StdRng,
) {
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.55, 0.15),
        perceptual_roughness: 0.55,
        ..default()
    });
    let ground_mesh = meshes.add(Cylinder::new(radius, 0.1));
    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_mat),
        Transform::from_translation(position + Vec3::Y * 0.05),
        Park,
    ));

    let tree_count = rng.random_range(3..8);
    let trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.3, 0.15),
        ..default()
    });
    let leaves_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.5, 0.1),
        ..default()
    });

    for _ in 0..tree_count {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let dist = rng.random_range(1.0..radius * 0.8);
        let tree_pos = position + Vec3::new(angle.cos() * dist, 0.0, angle.sin() * dist);

        let trunk_h = rng.random_range(2.0..4.0);
        let trunk_mesh = meshes.add(Cylinder::new(0.2, trunk_h));
        commands.spawn((
            Mesh3d(trunk_mesh),
            MeshMaterial3d(trunk_mat.clone()),
            Transform::from_translation(tree_pos + Vec3::Y * trunk_h / 2.0),
        ));

        let leaves_r = rng.random_range(1.2..2.5);
        let leaves_mesh = meshes.add(Sphere::new(leaves_r).mesh().ico(2).unwrap());
        commands.spawn((
            Mesh3d(leaves_mesh),
            MeshMaterial3d(leaves_mat.clone()),
            Transform::from_translation(tree_pos + Vec3::Y * (trunk_h + leaves_r * 0.5)),
        ));
    }
}
