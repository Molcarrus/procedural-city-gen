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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn test_block() -> Polygon {
        // 20x20 square, area = 400 m²
        vec![
            bevy::prelude::Vec2::new(0.0, 0.0),
            bevy::prelude::Vec2::new(20.0, 0.0),
            bevy::prelude::Vec2::new(20.0, 20.0),
            bevy::prelude::Vec2::new(0.0, 20.0),
        ]
    }

    #[test]
    fn test_subdivide_produces_plots() {
        let mut rng = StdRng::seed_from_u64(42);
        let block = test_block();
        let plots = subdivide_to_plots(&block, 15.0, 0.35, 0.25, 0.0, 0, &mut rng, 10, 0.0, 0.8);
        assert!(!plots.is_empty(), "should produce at least one plot");
    }

    #[test]
    fn test_subdivide_respects_min_area() {
        let mut rng = StdRng::seed_from_u64(42);
        let block = test_block();
        let min_area = 15.0f32;
        let plots =
            subdivide_to_plots(&block, min_area, 0.35, 0.25, 0.0, 0, &mut rng, 10, 0.0, 0.8);
        for plot in &plots {
            let area = polygon_area(plot).abs();
            assert!(
                area >= min_area * 0.1,
                "plot area {area} is unreasonably small"
            );
        }
    }

    #[test]
    fn test_subdivide_total_area_conserved() {
        let mut rng = StdRng::seed_from_u64(42);
        let block = test_block();
        let original_area = polygon_area(&block).abs();
        // No empty plots, no alleys
        let plots = subdivide_to_plots(&block, 15.0, 0.0, 0.0, 0.0, 0, &mut rng, 10, 0.0, 0.0);
        let total: f32 = plots.iter().map(|p| polygon_area(p).abs()).sum();
        assert!(
            (total - original_area).abs() < 1.0,
            "total area {total} should be close to original {original_area}"
        );
    }

    #[test]
    fn test_subdivide_with_empty_prob_reduces_count() {
        let block = test_block();
        let mut rng_no_empty = StdRng::seed_from_u64(42);
        let mut rng_with_empty = StdRng::seed_from_u64(42);

        let plots_full = subdivide_to_plots(
            &block,
            15.0,
            0.35,
            0.25,
            0.0,
            0,
            &mut rng_no_empty,
            10,
            0.0,
            0.8,
        );
        let plots_empty = subdivide_to_plots(
            &block,
            15.0,
            0.35,
            0.25,
            1.0,
            0,
            &mut rng_with_empty,
            10,
            0.0,
            0.8,
        );

        assert!(
            plots_empty.len() <= plots_full.len(),
            "empty_prob=1.0 should produce fewer or equal plots"
        );
    }

    #[test]
    fn test_subdivide_deterministic() {
        let block = test_block();
        let mut rng1 = StdRng::seed_from_u64(99);
        let mut rng2 = StdRng::seed_from_u64(99);

        let a = subdivide_to_plots(&block, 15.0, 0.35, 0.25, 0.05, 0, &mut rng1, 10, 0.8, 0.8);
        let b = subdivide_to_plots(&block, 15.0, 0.35, 0.25, 0.05, 0, &mut rng2, 10, 0.8, 0.8);

        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn test_subdivide_max_depth_zero() {
        let mut rng = StdRng::seed_from_u64(1);
        let block = test_block();
        let plots = subdivide_to_plots(&block, 1.0, 0.35, 0.25, 0.0, 0, &mut rng, 0, 0.0, 0.8);
        // With max_depth=0, depth > max_depth is false on first call
        // but area > min_area so it tries to cut once, which may produce 1 or 2 plots
        assert!(!plots.is_empty());
    }

    #[test]
    fn test_subdivide_tiny_polygon_returns_as_is() {
        let mut rng = StdRng::seed_from_u64(1);
        // 1x1 square, area = 1 m², below min_area of 15
        let tiny = vec![
            bevy::prelude::Vec2::new(0.0, 0.0),
            bevy::prelude::Vec2::new(1.0, 0.0),
            bevy::prelude::Vec2::new(1.0, 1.0),
            bevy::prelude::Vec2::new(0.0, 1.0),
        ];
        let plots = subdivide_to_plots(&tiny, 15.0, 0.35, 0.25, 0.0, 0, &mut rng, 10, 0.0, 0.8);
        assert_eq!(plots.len(), 1);
    }

    #[test]
    fn test_apply_road_corridor_no_path() {
        let polys = vec![test_block()];
        let result = apply_road_corridor(polys.clone(), &[], 1, 2.0);
        assert_eq!(result, polys);
    }

    #[test]
    fn test_apply_road_corridor_zero_count() {
        let polys = vec![test_block()];
        let path = vec![
            bevy::prelude::Vec3::new(-10.0, 0.0, 0.0),
            bevy::prelude::Vec3::new(10.0, 0.0, 0.0),
        ];
        let result = apply_road_corridor(polys.clone(), &path, 0, 2.0);
        assert_eq!(result, polys);
    }

    #[test]
    fn test_apply_road_corridor_shrinks_first_poly() {
        let polys = vec![test_block(), test_block()];
        let path = vec![
            bevy::prelude::Vec3::new(-30.0, 0.0, 0.0),
            bevy::prelude::Vec3::new(30.0, 0.0, 0.0),
        ];
        let original_area = polygon_area(&polys[0]).abs();
        let result = apply_road_corridor(polys, &path, 1, 2.0);

        let new_area = polygon_area(&result[0]).abs();
        // First poly should be smaller or equal after corridor applied
        assert!(new_area <= original_area + 0.01);
        // Second poly should be unchanged
        assert_eq!(result[1], test_block());
    }
}
