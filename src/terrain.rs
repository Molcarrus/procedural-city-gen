use bevy::prelude::*;

#[derive(Component)]
pub struct Terrain;

pub fn spawn_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    size: f32,
) {
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.38, 0.32),
        perceptual_roughness: 0.95,
        ..default()
    });

    commands.spawn((
        Terrain,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
        MeshMaterial3d(ground_mat),
        Transform::from_translation(Vec3::new(size / 2.0, 0.0, size / 2.0)),
    ));

    let water_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.3, 0.5, 0.6),
        perceptual_roughness: 0.1,
        metallic: 0.6,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Terrain,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size * 3.0, size * 3.0))),
        MeshMaterial3d(water_mat),
        Transform::from_translation(Vec3::new(size / 2.0, -0.5, size / 2.0)),
    ));
}
