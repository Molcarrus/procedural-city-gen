use bevy::{
    asset::RenderAssetUsages,
    math::Vec2,
    mesh::{Indices, Mesh},
};

use crate::{
    core::Polygon,
    geometry::{polygon_area, polygon_centroid},
};

pub fn polygon_to_footprint(polygon: &Polygon) -> Mesh {
    if polygon.len() < 3 {
        return empty_mesh();
    }

    let area = polygon_area(polygon);
    let centroid = polygon_centroid(polygon, area);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let (min_x, max_x, min_y, max_y) = polygon_uv_bounds(polygon);
    let uv_width = (max_x - min_x).max(f32::EPSILON);
    let uv_height = (max_y - min_y).max(f32::EPSILON);

    positions.push([centroid.x, 0.0, centroid.y]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    for vertex in polygon.iter() {
        positions.push([vertex.x, 0.0, vertex.y]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([
            (vertex.x - min_x) / uv_width,
            (vertex.y - min_y) / uv_height,
        ]);
    }

    let n = polygon.len() as u32;
    for i in 0..n {
        let curr = 1 + i;
        let next = 1 + (i + 1) % n;
        indices.extend([0, next, curr]);
    }

    build_mesh(positions, normals, uvs, indices)
}

pub fn polygon_to_building(polygon: &Polygon, wall_height: f32) -> Mesh {
    if polygon.len() < 3 {
        return empty_mesh();
    }

    let area = polygon_area(polygon);
    let centroid = polygon_centroid(polygon, area);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for i in 0..polygon.len() {
        let next = (i + 1) % polygon.len();
        let v1 = polygon[i];
        let v2 = polygon[next];

        let edge = v2 - v1;
        let edge_len = edge.length();

        let normal = Vec2::new(edge.y, -edge.x).normalize();
        let wall_normal = [normal.x, 0.0, normal.y];

        let base = positions.len() as u32;

        positions.extend([
            [v1.x, 0.0, v1.y],
            [v2.x, 0.0, v2.y],
            [v1.x, wall_height, v1.y],
            [v2.x, wall_height, v2.y],
        ]);

        normals.extend([wall_normal; 4]);

        uvs.extend([
            [0.0, 0.0],
            [edge_len, 0.0],
            [0.0, wall_height],
            [edge_len, wall_height],
        ]);

        indices.extend([base, base + 2, base + 1]);
        indices.extend([base + 1, base + 2, base + 3]);
    }

    let bottom_center = positions.len() as u32;
    positions.push([centroid.x, 0.0, centroid.y]);
    normals.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    for i in 0..polygon.len() {
        let v = polygon[i];
        positions.push([v.x, 0.0, v.y]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.0, 0.0]);

        let curr = bottom_center + 1 + i as u32;
        let next = bottom_center + 1 + ((i + 1) % polygon.len()) as u32;
        indices.extend([bottom_center, curr, next]);
    }

    let top_center = positions.len() as u32;
    positions.push([centroid.x, wall_height, centroid.y]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.0, 0.0]);

    for i in 0..polygon.len() {
        let v = polygon[i];
        positions.push([v.x, wall_height, v.y]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.0, 0.0]);

        let curr = top_center + 1 + i as u32;
        let next = top_center + 1 + ((i + 1) % polygon.len()) as u32;
        indices.extend([top_center, next, curr]);
    }

    build_mesh(positions, normals, uvs, indices)
}

fn polygon_uv_bounds(polygon: &Polygon) -> (f32, f32, f32, f32) {
    let min_x = polygon.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
    let max_x = polygon
        .iter()
        .map(|v| v.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = polygon.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
    let max_y = polygon
        .iter()
        .map(|v| v.y)
        .fold(f32::NEG_INFINITY, f32::max);

    (min_x, max_x, min_y, max_y)
}

fn build_mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> Mesh {
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

fn empty_mesh() -> Mesh {
    Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use bevy::mesh::VertexAttributeValues;

    use super::*;

    fn unit_square() -> Polygon {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(0.0, 4.0),
        ]
    }

    fn triangle() -> Polygon {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(1.5, 3.0),
        ]
    }

    fn get_positions(mesh: &Mesh) -> Vec<[f32; 3]> {
        match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(v) => v.clone(),
            _ => panic!("unexpected position format"),
        }
    }

    fn get_indices(mesh: &Mesh) -> Vec<u32> {
        match mesh.indices().unwrap() {
            Indices::U32(v) => v.clone(),
            _ => panic!("unexpected index format"),
        }
    }

    fn get_normals(mesh: &Mesh) -> Vec<[f32; 3]> {
        match mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap() {
            VertexAttributeValues::Float32x3(v) => v.clone(),
            _ => panic!("unexpected normal format"),
        }
    }

    // --- footprint tests ---

    #[test]
    fn test_footprint_vertex_count() {
        let sq = unit_square();
        let mesh = polygon_to_footprint(&sq);
        // 1 center + 4 boundary
        assert_eq!(get_positions(&mesh).len(), 5);
    }

    #[test]
    fn test_footprint_index_count() {
        let sq = unit_square();
        let mesh = polygon_to_footprint(&sq);
        // 4 triangles * 3 indices
        assert_eq!(get_indices(&mesh).len(), 12);
    }

    #[test]
    fn test_footprint_all_at_y_zero() {
        let sq = unit_square();
        let mesh = polygon_to_footprint(&sq);
        for pos in get_positions(&mesh) {
            assert_eq!(pos[1], 0.0);
        }
    }

    #[test]
    fn test_footprint_normals_all_up() {
        let sq = unit_square();
        let mesh = polygon_to_footprint(&sq);
        for n in get_normals(&mesh) {
            assert!((n[1] - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_footprint_degenerate_polygon() {
        let line = vec![Vec2::ZERO, Vec2::ONE];
        let mesh = polygon_to_footprint(&line);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_none());
    }

    #[test]
    fn test_footprint_triangle() {
        let tri = triangle();
        let mesh = polygon_to_footprint(&tri);
        // 1 center + 3 boundary = 4 vertices
        assert_eq!(get_positions(&mesh).len(), 4);
        // 3 triangles * 3 indices = 9
        assert_eq!(get_indices(&mesh).len(), 9);
    }

    #[test]
    fn test_footprint_indices_in_bounds() {
        let sq = unit_square();
        let mesh = polygon_to_footprint(&sq);
        let n = get_positions(&mesh).len();
        for &idx in &get_indices(&mesh) {
            assert!((idx as usize) < n);
        }
    }

    #[test]
    fn test_footprint_center_vertex_at_centroid() {
        let sq = unit_square();
        let mesh = polygon_to_footprint(&sq);
        let positions = get_positions(&mesh);
        // Centroid of 4x4 square at origin = (2, 0, 2)
        assert!((positions[0][0] - 2.0).abs() < 1e-4);
        assert!((positions[0][2] - 2.0).abs() < 1e-4);
    }

    // --- building tests ---

    #[test]
    fn test_building_vertex_count() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 3.0);
        // n=4: walls = 4*4=16, bottom = 1+4=5, top = 1+4=5 -> total 26
        assert_eq!(get_positions(&mesh).len(), 26);
    }

    #[test]
    fn test_building_index_count() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 3.0);
        // n=4: walls = 4*6=24, bottom = 4*3=12, top = 4*3=12 -> total 48
        assert_eq!(get_indices(&mesh).len(), 48);
    }

    #[test]
    fn test_building_indices_in_bounds() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 3.0);
        let n = get_positions(&mesh).len();
        for &idx in &get_indices(&mesh) {
            assert!((idx as usize) < n, "index {idx} out of bounds (total: {n})");
        }
    }

    #[test]
    fn test_building_wall_height_respected() {
        let sq = unit_square();
        let wall_height = 4.5f32;
        let mesh = polygon_to_building(&sq, wall_height);
        let max_y = get_positions(&mesh)
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((max_y - wall_height).abs() < 1e-4);
    }

    #[test]
    fn test_building_min_y_is_zero() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 3.0);
        let min_y = get_positions(&mesh)
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min);
        assert!(min_y.abs() < 1e-4);
    }

    #[test]
    fn test_building_has_upward_normals() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 3.0);
        let has_up = get_normals(&mesh).iter().any(|n| (n[1] - 1.0).abs() < 1e-5);
        assert!(has_up);
    }

    #[test]
    fn test_building_has_downward_normals() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 3.0);
        let has_down = get_normals(&mesh).iter().any(|n| (n[1] + 1.0).abs() < 1e-5);
        assert!(has_down);
    }

    #[test]
    fn test_building_degenerate_polygon() {
        let line = vec![Vec2::ZERO, Vec2::ONE];
        let mesh = polygon_to_building(&line, 3.0);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_none());
    }

    #[test]
    fn test_building_triangle_footprint() {
        let tri = triangle();
        let mesh = polygon_to_building(&tri, 2.0);
        // n=3: walls=3*4=12, bottom=1+3=4, top=1+3=4 -> total 20
        assert_eq!(get_positions(&mesh).len(), 20);
        // n=3: walls=3*6=18, bottom=3*3=9, top=3*3=9 -> total 36
        assert_eq!(get_indices(&mesh).len(), 36);
    }

    #[test]
    fn test_building_wall_normals_are_horizontal() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 3.0);
        let normals = get_normals(&mesh);
        let wall_normals: Vec<_> = normals.iter().filter(|n| n[1].abs() < 1e-5).collect();
        assert!(!wall_normals.is_empty());
        for n in &wall_normals {
            let xz_len_sq = n[0] * n[0] + n[2] * n[2];
            assert!(
                (xz_len_sq - 1.0).abs() < 1e-4,
                "wall normal XZ length should be 1.0, got {xz_len_sq}"
            );
        }
    }

    #[test]
    fn test_building_zero_height() {
        let sq = unit_square();
        let mesh = polygon_to_building(&sq, 0.0);
        assert!(!get_positions(&mesh).is_empty());
    }

    #[test]
    fn test_building_vertex_count_formula() {
        // Verify formula 6n+2 holds for various polygon sizes
        for n in [3usize, 4, 5, 6, 8] {
            let poly: Polygon = (0..n)
                .map(|i| {
                    let angle = i as f32 / n as f32 * std::f32::consts::TAU;
                    Vec2::new(angle.cos() * 10.0, angle.sin() * 10.0)
                })
                .collect();
            let mesh = polygon_to_building(&poly, 3.0);
            let expected = 6 * n + 2;
            assert_eq!(
                get_positions(&mesh).len(),
                expected,
                "n={n}: expected {expected} vertices"
            );
        }
    }

    #[test]
    fn test_building_index_count_formula() {
        // Verify formula 12n holds for various polygon sizes
        for n in [3usize, 4, 5, 6, 8] {
            let poly: Polygon = (0..n)
                .map(|i| {
                    let angle = i as f32 / n as f32 * std::f32::consts::TAU;
                    Vec2::new(angle.cos() * 10.0, angle.sin() * 10.0)
                })
                .collect();
            let mesh = polygon_to_building(&poly, 3.0);
            let expected = 12 * n;
            assert_eq!(
                get_indices(&mesh).len(),
                expected,
                "n={n}: expected {expected} indices"
            );
        }
    }
}
