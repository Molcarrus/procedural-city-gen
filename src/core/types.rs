use bevy::{
    ecs::{component::Component, resource::Resource},
    math::{Vec2, Vec3, VectorSpace},
};

pub type Polygon = Vec<Vec2>;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GenerationMode {
    #[default]
    Auto,
    Manual,
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditMode {
    #[default]
    Boundary,
    Generators,
    Circumcenters,
    Roads,
}

#[derive(Resource)]
pub struct Seed(pub u64);

#[derive(Resource)]
pub struct SkeletonData {
    pub generator_points: Vec<Vec3>,
    pub circumcenters: Vec<Vec3>,
    pub cells: Vec<Vec<usize>>,
    pub road_path: Vec<Vec3>,
    pub boundary: Polygon,
    pub boundary_offsets: Vec<Vec2>,
}

impl SkeletonData {
    pub fn new_empty(boundary: Polygon) -> Self {
        let len = boundary.len();
        Self {
            generator_points: Vec::new(),
            circumcenters: Vec::new(),
            cells: Vec::new(),
            road_path: Vec::new(),
            boundary,
            boundary_offsets: vec![Vec2::ZERO; len],
        }
    }

    pub fn boundary_vertex_count(&self) -> usize {
        self.boundary.len()
    }

    pub fn get_boundary_vertex(&self, idx: usize) -> Option<Vec2> {
        self.boundary.get(idx).copied()
    }

    pub fn set_boundary_vertex(&mut self, idx: usize, pos: Vec2) {
        if let Some(v) = self.boundary.get_mut(idx) {
            *v = pos;
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.circumcenters.is_empty() || self.cells.is_empty() {
            return false;
        }
        for cell in &self.cells {
            if cell.len() < 3 {
                return false;
            }
            for &idx in cell {
                if idx >= self.circumcenters.len() {
                    return false;
                }
            }
        }

        true
    }
}

#[derive(Resource, Default)]
pub struct DragState {
    pub dragging_index: Option<usize>,
    pub drag_offset: Vec2,
}

#[derive(Resource, Default)]
pub struct HoveredPoint(pub Option<usize>);

#[derive(Resource, Default)]
pub struct SelectedPoint(pub Option<usize>);

#[derive(Component)]
pub struct Town {
    pub seed: u64,
}

#[derive(Component, Clone)]
pub struct Block {
    pub polygon: Polygon,
    pub id: u32,
}

#[derive(Component)]
pub struct Building {
    pub id: u32,
    pub footprint: Polygon,
}

#[derive(Resource)]
pub struct GizmosVisible(pub bool);

#[derive(Resource)]
pub struct Is3D(pub bool);
