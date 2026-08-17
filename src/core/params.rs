use bevy::ecs::resource::Resource;

use crate::config::*;

#[derive(Resource, Clone, PartialEq)]
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

/// Named starting points, so the sliders have somewhere interesting to start
/// from instead of only the one default city.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    Default,
    Medieval,
    ModernGrid,
    Suburb,
}

impl Preset {
    pub const ALL: [Preset; 4] = [
        Preset::Default,
        Preset::Medieval,
        Preset::ModernGrid,
        Preset::Suburb,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Preset::Default => "Default",
            Preset::Medieval => "Medieval",
            Preset::ModernGrid => "Modern grid",
            Preset::Suburb => "Suburb",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Preset::Default => "The baseline city.",
            Preset::Medieval => "Dense, irregular, narrow alleys, small buildings.",
            Preset::ModernGrid => "Regular blocks, wide streets, tall towers.",
            Preset::Suburb => "Sparse and low, lots of green space.",
        }
    }

    pub fn params(self) -> Params {
        let base = Params::default();
        match self {
            Preset::Default => base,

            Preset::Medieval => Params {
                boundary_vertex_count: 7,
                generator_count: 45,
                street_width: 1.8,
                min_building_area: 7.0,
                grid_chaos: 0.85,
                size_chaos: 0.6,
                alley_chance: 0.9,
                alley_width: 0.5,
                min_wall_height: 2.0,
                max_wall_height: 7.0,
                plaza_chance: 0.07,
                park_ratio: 0.25,
                courtyard_chance: 0.45,
                courtyard_ratio: 0.3,
                ..base
            },

            Preset::ModernGrid => Params {
                boundary_vertex_count: 4,
                generator_count: 70,
                street_width: 5.0,
                min_building_area: 30.0,
                grid_chaos: 0.05,
                size_chaos: 0.1,
                alley_chance: 0.15,
                alley_width: 1.2,
                min_wall_height: 6.0,
                max_wall_height: 34.0,
                plaza_chance: 0.12,
                park_ratio: 0.5,
                courtyard_chance: 0.2,
                courtyard_ratio: 0.45,
                ..base
            },

            Preset::Suburb => Params {
                boundary_vertex_count: 9,
                generator_count: 40,
                street_width: 4.0,
                min_building_area: 45.0,
                grid_chaos: 0.4,
                size_chaos: 0.35,
                empty_prob: 0.25,
                alley_chance: 0.1,
                alley_width: 1.5,
                min_wall_height: 2.0,
                max_wall_height: 5.0,
                plaza_chance: 0.3,
                park_ratio: 0.9,
                courtyard_chance: 0.15,
                courtyard_ratio: 0.5,
                ..base
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_preset_has_a_usable_wall_height_range() {
        // random_range panics on an empty or inverted range.
        for preset in Preset::ALL {
            let p = preset.params();
            assert!(
                p.max_wall_height > p.min_wall_height,
                "{:?} has an empty wall height range",
                preset
            );
        }
    }

    #[test]
    fn test_every_preset_has_sane_probabilities() {
        for preset in Preset::ALL {
            let p = preset.params();
            for (name, v) in [
                ("grid_chaos", p.grid_chaos),
                ("size_chaos", p.size_chaos),
                ("empty_prob", p.empty_prob),
                ("alley_chance", p.alley_chance),
                ("plaza_chance", p.plaza_chance),
                ("park_ratio", p.park_ratio),
                ("courtyard_chance", p.courtyard_chance),
            ] {
                assert!(
                    (0.0..=1.0).contains(&v),
                    "{preset:?}.{name} is out of range: {v}"
                );
            }
            assert!(p.min_building_area > 0.0, "{preset:?} has no minimum area");
            assert!(p.generator_count > 0, "{preset:?} has no generators");
            assert!(
                p.boundary_vertex_count >= 3,
                "{preset:?} cannot form a boundary"
            );
        }
    }

    #[test]
    fn test_presets_are_actually_distinct() {
        let all: Vec<Params> = Preset::ALL.iter().map(|p| p.params()).collect();
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert!(
                    all[i] != all[j],
                    "{:?} and {:?} are identical",
                    Preset::ALL[i],
                    Preset::ALL[j]
                );
            }
        }
    }
}
