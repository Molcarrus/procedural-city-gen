pub const CANVS_WIDTH: f32 = 500.0;
pub const CANVAS_HEIGHT: f32 = 500.0;

pub const INITIAL_SEED: u64 = 1512086461918454205;
pub const POINT_COUNT: usize = 30;
pub const SPIRAL_SPREAD: f32 = 3.0;

pub const BUILDING_AREA_MIN: f32 = 15.0;
pub const BUILDING_AREA_MAX: f32 = 40.0;

pub const CIRCUMCENTER_MERGE_THRESHOLD: f32 = 0.01;

pub const GRID_CHAOS: f32 = 0.35;
pub const SIZE_CHAOS: f32 = 0.25;
pub const EMPTY_PROB: f32 = 0.05;

pub const BOUNDARY_GENERATOR_SPACING: f32 = 12.0;
pub const BOUNDARY_GENERATOR_INNER_OFFSET: f32 = 1.0;
pub const BOUNDARY_GENERATOR_OUTER_OFFSET: f32 = 2.0;

pub const MAX_RECURSION_DEPTH: usize = 10;

pub const ALLEY_WIDTH_MIN: f32 = 0.5;
pub const ALLEY_WIDTH_MAX: f32 = 1.5;
pub const ALLEY_WIDTH: f32 = 0.8;
pub const ALLEY_CHANCE: f32 = 0.8;

pub const ROAD_GENERATOR_SPACING: f32 = 7.0;
pub const ROAD_GENERATOR_OFFSET: f32 = 0.1;
pub const CORNER_CONSTRAINT_DISTANCE: f32 = 2.0;
pub const ROAD_WIDTH: f32 = 4.0;

pub const MIN_WALL_HEIGHT: f32 = 2.0;
pub const MAX_WALL_HEIGHT: f32 = 6.0;

pub const DEFAULT_BOUNDARY_VERTEX_COUNT: usize = 4;
pub const DEFAULT_BOUNDARY_SCALE: f32 = 75.0;

// --- street network ---
/// Every block is inset by half of this, so the gap between two neighbouring
/// blocks adds up to a full street width.
pub const STREET_WIDTH: f32 = 3.0;
/// Street surface sits just below the blocks so it shows through the gaps
/// without z-fighting against block footprints at y = 0.
pub const STREET_LEVEL: f32 = -0.06;
/// The street surface is grown slightly past the boundary so blocks touching
/// the edge still sit on paving.
pub const STREET_SURFACE_MARGIN: f32 = 6.0;

// --- open space ---
pub const PLAZA_CHANCE: f32 = 0.14;
/// Of the blocks reserved as open space, the fraction that become parks
/// rather than paved plazas.
pub const PARK_RATIO: f32 = 0.6;
pub const COURTYARD_CHANCE: f32 = 0.28;
/// Fraction of a block's "radius" kept clear at its centre for a courtyard.
pub const COURTYARD_RATIO: f32 = 0.38;

// --- water ---
pub const WATER_LEVEL: f32 = -2.5;
pub const WATER_EXTENT: f32 = 900.0;
