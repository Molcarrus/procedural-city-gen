use bevy::{
    ecs::{
        message::MessageWriter,
        system::{Query, ResMut},
    },
    pbr::wireframe::WireframeConfig,
};
use bevy_egui::{EguiContexts, egui};
use rand::RngExt;

use crate::core::{
    Block, Building, Courtyard, EditMode, ExportEvent, GenerationMode, GizmosVisible, Is3D,
    OpenSpace, OpenSpaceKind, Params, RegenerateEvent, Seed,
};

/// Tracks what the user touched this frame, so we only pay for a full skeleton
/// rebuild when a parameter that actually shapes it changed.
#[derive(Default)]
struct Dirty {
    regenerate: bool,
    rebuild_skeleton: bool,
}

impl Dirty {
    fn cheap(&mut self, changed: bool) {
        self.regenerate |= changed;
    }

    fn structural(&mut self, changed: bool) {
        if changed {
            self.regenerate = true;
            self.rebuild_skeleton = true;
        }
    }
}

pub fn control_panel(
    mut contexts: EguiContexts,
    mut params: ResMut<Params>,
    mut seed: ResMut<Seed>,
    mut is_3d: ResMut<Is3D>,
    mut gizmos: ResMut<GizmosVisible>,
    mut wireframe: ResMut<WireframeConfig>,
    mut generation_mode: ResMut<GenerationMode>,
    mut edit_mode: ResMut<EditMode>,
    mut regenerate: MessageWriter<RegenerateEvent>,
    mut export: MessageWriter<ExportEvent>,
    blocks: Query<&Block>,
    buildings: Query<&Building>,
    open_spaces: Query<&OpenSpace>,
    courtyards: Query<&Courtyard>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let params = params.as_mut();
    let mut dirty = Dirty::default();

    egui::SidePanel::left("control_panel")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Procedural City");
            ui.separator();

            // --- seed ---
            ui.horizontal(|ui| {
                ui.label("Seed");
                let mut seed_text = seed.0.to_string();
                if ui.text_edit_singleline(&mut seed_text).changed() {
                    if let Ok(v) = seed_text.parse::<u64>() {
                        seed.0 = v;
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Randomize").clicked() {
                    seed.0 = rand::rng().random::<u64>();
                    dirty.structural(true);
                }
                if ui.button("Regenerate").clicked() {
                    dirty.structural(true);
                }
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("City shape", |ui| {
                    dirty.structural(
                        ui.add(
                            egui::Slider::new(&mut params.boundary_vertex_count, 3..=12)
                                .text("Boundary sides"),
                        )
                        .changed(),
                    );
                    dirty.structural(
                        ui.add(
                            egui::Slider::new(&mut params.boundary_scale, 20.0..=200.0)
                                .text("City radius"),
                        )
                        .changed(),
                    );
                    dirty.structural(
                        ui.add(
                            egui::Slider::new(&mut params.generator_count, 5..=200)
                                .text("Block count"),
                        )
                        .changed(),
                    );
                    dirty.structural(
                        ui.add(
                            egui::Slider::new(&mut params.boundary_spacing, 4.0..=40.0)
                                .text("Edge spacing"),
                        )
                        .changed(),
                    );
                });

                ui.collapsing("Streets", |ui| {
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.street_width, 0.0..=12.0)
                                .text("Street width"),
                        )
                        .changed(),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Each block gives up half a street on every edge, \
                             so the gap between two blocks is one full street.",
                        )
                        .small()
                        .weak(),
                    );
                });

                ui.collapsing("Buildings", |ui| {
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.min_building_area, 2.0..=120.0)
                                .text("Min plot area"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.max_recursion_depth, 1..=14)
                                .text("Max subdivision"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.grid_chaos, 0.0..=1.0).text("Grid chaos"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.size_chaos, 0.0..=1.0).text("Size chaos"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.empty_prob, 0.0..=1.0)
                                .text("Empty plot chance"),
                        )
                        .changed(),
                    );

                    ui.separator();

                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.min_wall_height, 1.0..=30.0)
                                .text("Min height"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.max_wall_height, 1.0..=60.0)
                                .text("Max height"),
                        )
                        .changed(),
                    );
                    // random_range panics on an empty range, so keep max above min.
                    if params.max_wall_height <= params.min_wall_height {
                        params.max_wall_height = params.min_wall_height + 0.1;
                    }
                });

                ui.collapsing("Alleys", |ui| {
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.alley_chance, 0.0..=1.0)
                                .text("Alley chance"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.alley_width, 0.0..=5.0)
                                .text("Alley width"),
                        )
                        .changed(),
                    );
                });

                ui.collapsing("Open space", |ui| {
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.plaza_chance, 0.0..=1.0)
                                .text("Open block chance"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.park_ratio, 0.0..=1.0)
                                .text("Park vs plaza"),
                        )
                        .changed(),
                    );

                    ui.separator();

                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.courtyard_chance, 0.0..=1.0)
                                .text("Courtyard chance"),
                        )
                        .changed(),
                    );
                    dirty.cheap(
                        ui.add(
                            egui::Slider::new(&mut params.courtyard_ratio, 0.05..=0.9)
                                .text("Courtyard size"),
                        )
                        .changed(),
                    );
                });

                ui.collapsing("Water", |ui| {
                    dirty.cheap(
                        ui.checkbox(&mut params.water_enabled, "Show water").changed(),
                    );
                    ui.add_enabled_ui(params.water_enabled, |ui| {
                        dirty.cheap(
                            ui.add(
                                egui::Slider::new(&mut params.water_level, -20.0..=0.0)
                                    .text("Water level"),
                            )
                            .changed(),
                        );
                    });
                });

                ui.collapsing("Display", |ui| {
                    // Visibility is baked in at spawn time, so flipping this
                    // has to rebuild the town entities.
                    dirty.cheap(ui.checkbox(&mut is_3d.0, "3D buildings").changed());
                    ui.checkbox(&mut wireframe.global, "Wireframe");
                    ui.checkbox(&mut gizmos.0, "Gizmos");
                });

                ui.collapsing("Generation mode", |ui| {
                    let mut auto = *generation_mode == GenerationMode::Auto;
                    if ui.checkbox(&mut auto, "Auto regenerate skeleton").changed() {
                        *generation_mode = if auto {
                            GenerationMode::Auto
                        } else {
                            GenerationMode::Manual
                        };
                    }

                    ui.add_enabled_ui(!auto, |ui| {
                        ui.label("Edit target");
                        for (mode, label) in [
                            (EditMode::Boundary, "Boundary"),
                            (EditMode::Generators, "Generators"),
                            (EditMode::Circumcenters, "Circumcenters"),
                            (EditMode::Roads, "Roads"),
                        ] {
                            let selected = *edit_mode == mode;
                            if ui.selectable_label(selected, label).clicked() {
                                *edit_mode = mode;
                            }
                        }
                    });
                });

                ui.separator();

                egui::CollapsingHeader::new("Stats")
                    .default_open(true)
                    .show(ui, |ui| {
                    let parks = open_spaces
                        .iter()
                        .filter(|o| o.kind == OpenSpaceKind::Park)
                        .count();
                    let plazas = open_spaces.iter().count() - parks;

                    egui::Grid::new("stats_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Blocks");
                        ui.label(blocks.iter().count().to_string());
                        ui.end_row();
                        ui.label("Buildings");
                        ui.label(buildings.iter().count().to_string());
                        ui.end_row();
                        ui.label("Parks");
                        ui.label(parks.to_string());
                        ui.end_row();
                        ui.label("Plazas");
                        ui.label(plazas.to_string());
                        ui.end_row();
                        ui.label("Courtyards");
                        ui.label(courtyards.iter().count().to_string());
                        ui.end_row();
                    });
                });

                ui.separator();

                if ui.button("Export OBJ").clicked() {
                    export.write(ExportEvent {
                        filename: format!("city_{}.obj", seed.0),
                    });
                }
            });
        });

    if dirty.regenerate {
        regenerate.write(RegenerateEvent {
            seed: seed.0,
            user_edit: false,
            rebuild_skeleton: dirty.rebuild_skeleton,
        });
    }
}
