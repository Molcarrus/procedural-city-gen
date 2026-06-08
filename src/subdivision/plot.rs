use bevy::math::Vec2;
use rand::{RngExt, rngs::StdRng};

use crate::{
    core::Polygon,
    geometry::{longest_edge, polygon_area},
    subdivision::{bisect_polygon, push_polygon_from_line},
};

pub fn subdivide_to_plots(
    polygon: &Polygon,
    min_area: f32,
    grid_chaos: f32,
    size_chaos: f32,
    empty_prob: f32,
    depth: usize,
    rng: &mut StdRng,
    max_depth: usize,
    alley_chance: f32,
    alley_width: f32,
) -> Vec<Polygon> {
    if depth > max_depth {
        return vec![polygon.clone()];
    }

    let area = polygon_area(polygon).abs();

    if area < min_area {
        return vec![polygon.clone()];
    }

    let Some((longest_idx, _, _)) = longest_edge(polygon) else {
        return vec![polygon.clone()];
    };

    let spread = 0.8 * grid_chaos;
    let ratio = (1.0 - spread) / 2.0 + rng.random::<f32>() * spread;

    let angle_spread = if area < min_area * 4.0 {
        0.0
    } else {
        std::f32::consts::PI / 6.0 * grid_chaos
    };
    let angle_offset = (rng.random::<f32>() - 0.5) * angle_spread;

    let depth_factor = 1.0 - (depth as f32 / max_depth as f32);
    let effective_alley_chance = alley_chance * depth_factor;
    let separation = if rng.random::<f32>() < effective_alley_chance {
        alley_width
    } else {
        0.0
    };

    let halves = bisect_polygon(polygon, longest_idx, ratio, angle_offset, separation);

    if halves.len() == 1 && halves[0].len() == polygon.len() {
        return vec![polygon.clone()];
    }

    let mut plots = Vec::new();

    for half in halves {
        let half_area = polygon_area(&half).abs();

        let size_factor = 2_f32.powf(4.0 * size_chaos * (rng.random::<f32>()) * 0.5);
        let adjusted_min = min_area * size_factor;

        if half_area < adjusted_min * 2.0 {
            if rng.random::<f32>() >= empty_prob {
                plots.push(half);
            }
        } else {
            plots.extend(subdivide_to_plots(
                &half,
                min_area,
                grid_chaos,
                size_chaos,
                empty_prob,
                depth + 1,
                rng,
                max_depth,
                alley_chance,
                alley_width,
            ));
        }
    }

    plots
}

pub fn apply_road_corridor(
    mut polygons: Vec<Polygon>,
    road_path: &[bevy::prelude::Vec3],
    road_generator_count: usize,
    road_half_width: f32,
) -> Vec<Polygon> {
    if road_path.len() < 2 || road_generator_count == 0 {
        return polygons;
    }

    for poly in polygons.iter_mut().take(road_generator_count) {
        for i in 0..(road_path.len() - 1) {
            let road_start = Vec2::new(road_path[i].x, road_path[i].z);
            let road_end = Vec2::new(road_path[i + 1].x, road_path[i + 1].z);

            if road_start.distance(road_end) > 0.1 {
                *poly = push_polygon_from_line(poly, road_start, road_end, road_half_width);
            }
        }
    }

    polygons
}
