use bevy::ecs::resource::Resource;

use crate::config::*;

#[derive(Resource)]
pub struct Params {
    pub max_recursion_depth: usize,
    pub min_building_area: f32,
    pub grid_chaos: f32,
    pub size_chaos: f32,
    pub empty_prob: f32,
    pub alley_width: f32,
    pub alley_chance: f32,
    pub min_wall_height: f32,
    pub max_wall_height: f32,
    pub boundary_vertex_count: usize,
    pub boundary_scale: f32,
    pub boundary_spacing: f32,
    pub boundary_inner_offset: f32,
    pub generator_count: usize,
    pub circumcenter_merge_threshold: f32,
    pub street_width: f32,
    pub plaza_chance: f32,
    pub park_ratio: f32,
    pub courtyard_chance: f32,
    pub courtyard_ratio: f32,
    pub water_enabled: bool,
    pub water_level: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_recursion_depth: MAX_RECURSION_DEPTH,
            min_building_area: BUILDING_AREA_MIN,
            grid_chaos: GRID_CHAOS,
            size_chaos: SIZE_CHAOS,
            empty_prob: EMPTY_PROB,
            alley_width: ALLEY_WIDTH,
            alley_chance: ALLEY_CHANCE,
            min_wall_height: MIN_WALL_HEIGHT,
            max_wall_height: MAX_WALL_HEIGHT,
            boundary_vertex_count: DEFAULT_BOUNDARY_VERTEX_COUNT,
            boundary_scale: DEFAULT_BOUNDARY_SCALE,
            boundary_spacing: BOUNDARY_GENERATOR_SPACING,
            boundary_inner_offset: BOUNDARY_GENERATOR_INNER_OFFSET,
            generator_count: POINT_COUNT,
            circumcenter_merge_threshold: CIRCUMCENTER_MERGE_THRESHOLD,
            street_width: STREET_WIDTH,
            plaza_chance: PLAZA_CHANCE,
            park_ratio: PARK_RATIO,
            courtyard_chance: COURTYARD_CHANCE,
            courtyard_ratio: COURTYARD_RATIO,
            water_enabled: true,
            water_level: WATER_LEVEL,
        }
    }
}
