use std::f32;

use bevy::prelude::*;
use rand::{Rng, rngs::StdRng};

use crate::{consts::ROAD_WIDTH, world::terrain::{Polygon, tessellation::math::{get_area, get_centroid, line_intersection}}};

pub fn subdivide_to_plots(
    polygon: &Polygon,
    min_sq: f32,
    grid_chaos: f32,
    size_chaos: f32,
    empty_prob: f32,
    depth: usize,
    rng: &mut StdRng,
    max_recursion_depth: usize,
    alley_chance: f32,
    alley_width: f32,
) -> Vec<Polygon> {
    if depth > max_recursion_depth {
        return vec![polygon.clone()];
    }
    
    let area = get_area(polygon);
    
    if area < min_sq {
        return vec![polygon.clone()];
    }
    
    let Some((longest_idx, _, _)) = longest_edge(polygon) else { return vec![polygon.clone()]; };
    
    let spread = 0.8 * grid_chaos;
    let ratio = (1.0 - spread) / 2.0 + rng.random::<f32>() * spread;
    let angle_spread = if area < min_sq * 4.0 { 0.0 } else { std::f32::consts::PI / 6.0 * grid_chaos };
    let angle_offset = (rng.random::<f32>() - 0.5) * angle_spread;
   
    let depth_factor = 1.0 - (depth as f32 / max_recursion_depth as f32);
    let alley_chance = alley_chance * depth_factor;
    
    let alley_width = if rng.random::<f32>() < alley_chance { alley_width } else { 0.0 };
    
    let halves = bisect_poly(polygon, longest_idx, ratio, angle_offset, alley_width);
    
    if halves.len() == 1 && halves[0].len() == polygon.len() {
        return vec![polygon.clone()];
    }
    
    let mut buildings = Vec::new();
    
    for half in halves {
        let half_area = get_area(&half);
        
        let size_factor = 2_f32.powf(4.0 * size_chaos * (rng.random::<f32>() - 0.5));
        let adjusted_min = min_sq * size_factor;
        
        if half_area < adjusted_min * 2.0 {
            if rng.random::<f32>() >= empty_prob {
                buildings.push(half);
            }
        } else {
            buildings.extend(subdivide_to_plots(
                &half, 
                min_sq, 
                grid_chaos, 
                size_chaos, 
                empty_prob, 
                depth + 1, 
                rng, 
                max_recursion_depth, 
                alley_chance, 
                alley_width
            ));
        }
    }
    
    buildings
}

pub fn longest_edge(polygon: &Polygon) -> Option<(usize, Vec2, f32)> {
    if polygon.len() < 2 {
        return None;
    }
    
    let mut max_length = 0.0;
    let mut longest_idx = 0;
    
    for i in 0..polygon.len() {
        let next = (i + 1) % polygon.len();
        let length = polygon[i].distance(polygon[next]);
        
        if length > max_length {
            max_length = length;
            longest_idx = i;
        }
    }
    
    Some((longest_idx, polygon[longest_idx], max_length))
}

pub fn bisect_poly(
    polygon: &Polygon,
    start_idx: usize,
    ratio: f32,
    angle_offset: f32,
    separation: f32 
) -> Vec<Polygon> {
    if polygon.len() < 3 || start_idx >= polygon.len() {
        return vec![polygon.clone()];
    }
    
    let next_idx = (start_idx + 1) % polygon.len();
    let start_v = polygon[start_idx];
    let next_v = polygon[next_idx];
    
    let edge_dir = next_v - start_v;
    let cut_point = start_v + edge_dir * ratio;
    
    let perp = Vec2::new(-edge_dir.y, edge_dir.x).normalize();
    let rotated = Vec2::new(
        perp.x * angle_offset.cos() - perp.y * angle_offset.sin(),
       perp.x * angle_offset.sin() + perp.y * angle_offset.cos() 
    );
    
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    
    for v in polygon.iter() {
        min_x = min_x.min(v.x);
        max_x = max_x.max(v.x);
        min_y = min_y.min(v.y);
        max_y = max_y.max(v.y);
    }
    
    let line_extent = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
    
    let line_start = cut_point - rotated * line_extent;
    let line_end = cut_point + rotated * line_extent;
    
    let mut intersections = Vec::new();
    for i in 0..polygon.len() {
        let j = (i + 1) % polygon.len();
        let edge_start = polygon[i];
        let edge_end = polygon[j];
        
        if let Some(intersection) = line_intersection(line_start, line_end, edge_start, edge_end) {
            intersections.push((i, intersection));
        }
    }
    
    if intersections.len() != 2 {
        return vec![polygon.clone()];
    }
    
    intersections.sort_by_key(|&(idx, _)| idx);
    
    let (idx1, int1) = intersections[0];
    let (idx2, int2) = intersections[1];
    
    let mut poly1 = Vec::new();
    let mut poly2 = Vec::new();
    
    poly1.push(int1);
    for i in (idx1 + 1)..=idx2 {
        poly1.push(polygon[i]);
    }
    poly1.push(int2);
    
    poly2.push(int2);
    for i in (idx2 + 1)..polygon.len() {
        poly2.push(polygon[i]);
    }
    for i in 0..=idx1 {
        poly2.push(polygon[i]);
    }
    poly2.push(int1);
    
    let mut result = Vec::new();
    if poly1.len() >= 3 && get_area(&poly1) > 0.1 {
        if separation > 0.0 {
            result.push(push_polygon_from_line(&poly1, line_start, line_end, separation * 0.5));
        } else {
            result.push(poly1);
        }
    }
    if poly2.len() >= 3 && get_area(&poly2) > 0.1 {
        if separation > 0.0 {
            result.push(push_polygon_from_line(&poly2, line_start, line_end, separation * 0.5));
        } else {
            result.push(poly2); 
        }
    }
    
    if result.is_empty() {
        vec![polygon.clone()]
    } else {
        result
    }
}

pub fn push_polygon_from_line(
    polygon: &Polygon, 
    line_start: Vec2,
    line_end: Vec2,
    distance: f32
) -> Polygon {
    if polygon.len() < 3 {
        return polygon.clone();
    }
    
    let line_dir = (line_end - line_start).normalize();
    let line_normal = Vec2::new(-line_dir.y, line_dir.x);
    
    let centroid = get_centroid(polygon, get_area(polygon));
    let centroid_to_line = centroid - line_start;
    let centroid_side = centroid_to_line.dot(line_normal);
    let separation_direction = if centroid_side > 0.0 { line_normal } else { -line_normal };
    
    let shrunk_polygon = polygon
        .iter()
        .map(|&vertex| {
            let vertex_distance = pl_distance(vertex, line_start, line_end);
            if vertex_distance < distance * 2.0 {
                let line_vec = line_end - line_start;
                let vertex_vec = vertex - line_start;
                let t = vertex_vec.dot(line_vec) / line_vec.length_squared();
                
                if t >= -0.1 && t <= 1.1 {
                    vertex + separation_direction * distance
                } else {
                    vertex
                }
            } else {
                vertex
            }
        })
        .collect::<Polygon>();
    
    let shrunk_area = get_area(&shrunk_polygon);
    if shrunk_area < get_area(polygon) * 0.2 {
        polygon.clone()
    } else {
        shrunk_polygon
    }
}

pub fn constrain_road_generator_cells(
    cells: Vec<Vec<usize>>,
    points: &[Vec3],
    road_path: &[Vec3],
    road_generator_count: usize
) -> Vec<Vec<usize>> {
    if road_path.len() < 2 || road_generator_count == 0 {
        return cells;
    }
    
    let mut result = cells;
    let road_width = ROAD_WIDTH * 0.5;
    
    for (cell_idx, cell) in result.iter_mut().enumerate() {
        if cell_idx < road_generator_count && cell.len() >= 3 {
            let mut polygon = cell
                .iter()
                .map(|&point_idx| Vec2::new(points[point_idx].x, points[point_idx].z))
                .collect::<Polygon>();
            
            for i in 0..(road_path.len() - 1) {
                let road_start = Vec2::new(road_path[i].x, road_path[i].z);
                let road_end = Vec2::new(road_path[i+1].x, road_path[i+1].z);
                
                if road_start.distance(road_end) > 0.1 {
                    polygon = push_polygon_from_line(&polygon, road_start, road_end, road_width);
                }
            }
            
            for (vertex_idx, vertex) in polygon.iter().enumerate() {
                if vertex_idx < cell.len() {
                    let mut closest_idx = cell[vertex_idx];
                    let mut closest_dist = f32::INFINITY;
                    
                    for (point_idx, point) in points.iter().enumerate() {
                        let point_2d = Vec2::new(point.x, point.z);
                        let dist = vertex.distance(point_2d);
                        if dist < closest_dist {
                            closest_dist = dist;
                            closest_idx = point_idx;
                        }
                    }
                    cell[vertex_idx] = closest_idx;
                }
            }
        }
    }
    
    result
}

pub fn pl_distance(point: Vec2, line_start: Vec2, line_end: Vec2) -> f32 {
    let line_vec = line_end - line_start;
    let point_vec = point - line_start;
    let line_len = line_vec.length();
    
    if line_len < f32::EPSILON {
        return point_vec.length();
    }
    
    let t = (point_vec.dot(line_vec) / line_len.powi(2)).clamp(0.0, 1.0);
    let projection = line_start + line_vec * t;
    
    point.distance(projection)
}