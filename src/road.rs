use bevy::{asset::RenderAssetUsages, mesh::Indices, prelude::*};

#[derive(Component)]
pub struct Road;

pub fn spawn_roads(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_segments: &[(Vec2, Vec2)],
    road_width: f32,
) {
    let road_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.28),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });

    let sidewalk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.58, 0.55),
        perceptual_roughness: 0.9,
        ..default()
    });

    let mut road_positions = Vec::new();
    let mut road_normals = Vec::new();
    let mut road_uvs = Vec::new();
    let mut road_indices = Vec::new();

    let mut sidewal_positions = Vec::new();
    let mut sidewalk_normals = Vec::new();
    let mut sidewalk_uvs = Vec::new();
    let mut sidewalk_indices = Vec::new();

    for (a, b) in road_segments {
        let dir = (*b - *a).normalize_or_zero();
        let perp = Vec2::new(-dir.y, dir.x);

        let half_road = road_width / 2.0;
        let half_sidewalk = road_width / 2.0 + 0.5;

        let base_idx = road_positions.len() as u32;
        let y_road = 0.02;

        let p0 = *a + perp * half_road;
        let p1 = *a - perp * half_road;
        let p2 = *b - perp * half_road;
        let p3 = *b + perp * half_road;

        road_positions.extend_from_slice(&[
            [p0.x, y_road, p0.y],
            [p1.x, y_road, p1.y],
            [p2.x, y_road, p2.y],
            [p3.x, y_road, p3.y],
        ]);
        road_normals.extend_from_slice(&[
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        road_uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        road_indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);

        let sw_base = sidewal_positions.len() as u32;
        let y_sw = 0.05;

        let s0 = *a + perp * half_sidewalk;
        let s1 = *a + perp * half_road;
        let s2 = *b + perp * half_road;
        let s3 = *b + perp * half_sidewalk;

        sidewal_positions.extend_from_slice(&[
            [s0.x, y_sw, s0.y],
            [s1.x, y_sw, s1.y],
            [s2.x, y_sw, s2.y],
            [s3.x, y_sw, s3.y],
        ]);
        sidewalk_normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
        sidewalk_uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        sidewalk_indices.extend_from_slice(&[
            sw_base,
            sw_base + 1,
            sw_base + 2,
            sw_base,
            sw_base + 1,
            sw_base + 3,
        ]);
    }

    if !road_positions.is_empty() {
        let mut road_mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        road_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, road_positions);
        road_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, road_normals);
        road_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, road_uvs);
        road_mesh.insert_indices(Indices::U32(road_indices));

        commands.spawn((
            Road,
            Mesh3d(meshes.add(road_mesh)),
            MeshMaterial3d(road_material),
        ));
    }

    if !sidewal_positions.is_empty() {
        let mut sw_mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        sw_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, sidewal_positions);
        sw_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, sidewalk_normals);
        sw_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, sidewalk_uvs);
        sw_mesh.insert_indices(Indices::U32(sidewalk_indices));

        commands.spawn((
            Road,
            Mesh3d(meshes.add(sw_mesh)),
            MeshMaterial3d(sidewalk_material),
        ));
    }
}
