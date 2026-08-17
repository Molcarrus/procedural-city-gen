use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::{
        message::MessageWriter,
        system::{Query, Res, ResMut, SystemParam},
    },
    pbr::wireframe::WireframeConfig,
};
use bevy_egui::{EguiContexts, egui};
use rand::RngExt;

use crate::{
    core::{
        Block, Building, Courtyard, EditMode, ExportEvent, GenerationMode, GizmosVisible, Is3D,
        OpenSpace, OpenSpaceKind, Params, Preset, RegenerateEvent, Seed,
    },
    ui::{ExportStatus, LiveUpdate, UiVisible},
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

/// True once a widget's value has *settled*: the drag finished, or it was
/// changed without dragging (keyboard, or a click on the track).
///
/// Plain `changed()` fires on every frame of a drag, which would rebuild the
/// whole city - Voronoi, relaxation and all - once per frame while the user
/// drags a slider.
fn settled(r: &egui::Response, live: bool) -> bool {
    if live {
        return r.changed();
    }
    r.drag_stopped() || (r.changed() && !r.dragged())
}

fn slider<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    live: bool,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    label: &str,
    tip: &str,
) -> bool {
    let mut r = ui.add(egui::Slider::new(value, range).text(label));
    if !tip.is_empty() {
        r = r.on_hover_text(tip);
    }
    settled(&r, live)
}

/// Display toggles, grouped so `control_panel` stays under Bevy's 16-param cap.
#[derive(SystemParam)]
pub struct ViewSettings<'w> {
    pub is_3d: ResMut<'w, Is3D>,
    pub gizmos: ResMut<'w, GizmosVisible>,
    pub wireframe: ResMut<'w, WireframeConfig>,
    pub live: ResMut<'w, LiveUpdate>,
    pub ui_visible: ResMut<'w, UiVisible>,
}

/// The spawned city, for the Stats readout.
#[derive(SystemParam)]
pub struct CityStats<'w, 's> {
    pub blocks: Query<'w, 's, &'static Block>,
    pub buildings: Query<'w, 's, &'static Building>,
    pub open_spaces: Query<'w, 's, &'static OpenSpace>,
    pub courtyards: Query<'w, 's, &'static Courtyard>,
}

pub fn control_panel(
    mut contexts: EguiContexts,
    mut params: ResMut<Params>,
    mut seed: ResMut<Seed>,
    mut view: ViewSettings,
    mut generation_mode: ResMut<GenerationMode>,
    mut edit_mode: ResMut<EditMode>,
    export_status: Res<ExportStatus>,
    diagnostics: Res<DiagnosticsStore>,
    mut regenerate: MessageWriter<RegenerateEvent>,
    mut export: MessageWriter<ExportEvent>,
    stats: CityStats,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if !view.ui_visible.0 {
        // Leave a hint so the panel is recoverable without reading the source.
        egui::Area::new("hidden_hint".into())
            .fixed_pos(egui::pos2(12.0, 12.0))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("F1 - show panel").weak());
            });
        return;
    }

    let params = params.as_mut();
    let live_update = view.live.0;
    let mut dirty = Dirty::default();

    egui::SidePanel::left("control_panel")
        .default_width(310.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Procedural City");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("hide").on_hover_text("F1").clicked() {
                        view.ui_visible.0 = false;
                    }
                });
            });

            // --- fps ---
            let fps = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|d| d.smoothed());
            let frame_ms = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                .and_then(|d| d.smoothed());
            if let (Some(fps), Some(ms)) = (fps, frame_ms) {
                ui.label(
                    egui::RichText::new(format!("{fps:.0} fps  ({ms:.1} ms)"))
                        .small()
                        .weak(),
                );
            }

            ui.separator();

            // --- seed ---
            ui.horizontal(|ui| {
                ui.label("Seed");
                let mut seed_text = seed.0.to_string();
                let r = ui.add(
                    egui::TextEdit::singleline(&mut seed_text).desired_width(f32::INFINITY),
                );
                if r.changed() {
                    if let Ok(v) = seed_text.trim().parse::<u64>() {
                        seed.0 = v;
                    }
                }
                if r.lost_focus() {
                    dirty.structural(true);
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .button("Randomize")
                    .on_hover_text("New random seed and a fresh city")
                    .clicked()
                {
                    seed.0 = rand::rng().random::<u64>();
                    dirty.structural(true);
                }
                if ui.button("Regenerate").clicked() {
                    dirty.structural(true);
                }
                if ui
                    .button("Reset")
                    .on_hover_text("Restore every parameter to its default")
                    .clicked()
                {
                    *params = Params::default();
                    dirty.structural(true);
                }
            });

            // --- presets ---
            ui.horizontal_wrapped(|ui| {
                for preset in Preset::ALL {
                    if ui
                        .button(preset.label())
                        .on_hover_text(preset.describe())
                        .clicked()
                    {
                        *params = preset.params();
                        dirty.structural(true);
                    }
                }
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::CollapsingHeader::new("City shape")
                    .default_open(true)
                    .show(ui, |ui| {
                        dirty.structural(slider(ui, live_update,
                            &mut params.boundary_vertex_count, 3..=12, "Boundary sides",
                            "Corners of the city outline. Vertices sit on the axes, so 4 gives a diamond."));
                        dirty.structural(slider(ui, live_update,
                            &mut params.boundary_scale, 20.0..=200.0, "City radius",
                            "Distance from centre to the outline."));
                        dirty.structural(slider(ui, live_update,
                            &mut params.generator_count, 5..=200, "Block count",
                            "Voronoi seed points. More points means more, smaller blocks."));
                        dirty.structural(slider(ui, live_update,
                            &mut params.boundary_spacing, 4.0..=40.0, "Edge spacing",
                            "Spacing of the generators pinned along the outline."));
                    });

                ui.collapsing("Streets", |ui| {
                    dirty.cheap(slider(ui, live_update,
                        &mut params.street_width, 0.0..=12.0, "Street width",
                        "Each block gives up half of this on every edge, so the gap \
                         between two blocks is one full street."));
                    ui.label(
                        egui::RichText::new(
                            "Blocks too small to give up their frontage are dropped, \
                             so very wide streets thin the city out.",
                        )
                        .small()
                        .weak(),
                    );
                });

                ui.collapsing("Buildings", |ui| {
                    dirty.cheap(slider(ui, live_update,
                        &mut params.min_building_area, 2.0..=120.0, "Min plot area",
                        "Subdivision stops once a plot is smaller than this."));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.max_recursion_depth, 1..=14, "Max subdivision",
                        "Hard cap on how many times a block can be split."));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.grid_chaos, 0.0..=1.0, "Grid chaos",
                        "0 cuts blocks squarely; 1 cuts at wandering angles."));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.size_chaos, 0.0..=1.0, "Size chaos",
                        "How much plot sizes vary from each other."));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.empty_prob, 0.0..=1.0, "Empty plot chance",
                        "Chance a finished plot is left vacant."));

                    ui.separator();

                    dirty.cheap(slider(ui, live_update,
                        &mut params.min_wall_height, 1.0..=30.0, "Min height", ""));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.max_wall_height, 1.0..=60.0, "Max height", ""));
                    // Building heights are drawn from min..max, and an empty or
                    // inverted range panics, so keep them ordered.
                    if params.max_wall_height <= params.min_wall_height {
                        params.min_wall_height = params.max_wall_height - 0.1;
                    }
                });

                ui.collapsing("Alleys", |ui| {
                    dirty.cheap(slider(ui, live_update,
                        &mut params.alley_chance, 0.0..=1.0, "Alley chance",
                        "Chance a split pushes its halves apart, leaving a gap."));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.alley_width, 0.0..=5.0, "Alley width", ""));
                });

                ui.collapsing("Open space", |ui| {
                    dirty.cheap(slider(ui, live_update,
                        &mut params.plaza_chance, 0.0..=1.0, "Open block chance",
                        "Chance a whole block is reserved with no buildings at all."));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.park_ratio, 0.0..=1.0, "Park vs plaza",
                        "Of the reserved blocks, the share that become green parks \
                         rather than paved plazas."));

                    ui.separator();

                    dirty.cheap(slider(ui, live_update,
                        &mut params.courtyard_chance, 0.0..=1.0, "Courtyard chance",
                        "Chance a block keeps its middle clear, so buildings ring it."));
                    dirty.cheap(slider(ui, live_update,
                        &mut params.courtyard_ratio, 0.05..=0.9, "Courtyard size",
                        "Share of the block kept open at the centre."));
                });

                ui.collapsing("Water", |ui| {
                    dirty.cheap(ui.checkbox(&mut params.water_enabled, "Show water").changed());
                    ui.add_enabled_ui(params.water_enabled, |ui| {
                        dirty.cheap(slider(ui, live_update,
                            &mut params.water_level, -20.0..=0.0, "Water level",
                            "Height of the water plane relative to the ground."));
                    });
                });

                ui.collapsing("Display", |ui| {
                    // Visibility is baked in at spawn, so this rebuilds the town.
                    dirty.cheap(ui.checkbox(&mut view.is_3d.0, "3D buildings")
                        .on_hover_text("Off shows flat footprints only")
                        .changed());
                    ui.checkbox(&mut view.wireframe.global, "Wireframe");
                    ui.checkbox(&mut view.gizmos.0, "Gizmos");
                    ui.checkbox(&mut view.live.0, "Live update")
                        .on_hover_text(
                            "Rebuild while dragging a slider. Smoother feedback, \
                             but heavy on large cities.",
                        );
                });

                ui.collapsing("Generation mode", |ui| {
                    let mut auto = *generation_mode == GenerationMode::Auto;
                    if ui
                        .checkbox(&mut auto, "Auto regenerate skeleton")
                        .on_hover_text("Off keeps the skeleton and only edits the chosen target")
                        .changed()
                    {
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
                                dirty.cheap(true);
                            }
                        }
                    });
                });

                ui.separator();

                egui::CollapsingHeader::new("Stats")
                    .default_open(true)
                    .show(ui, |ui| {
                        let parks = stats.open_spaces
                            .iter()
                            .filter(|o| o.kind == OpenSpaceKind::Park)
                            .count();
                        let plazas = stats.open_spaces.iter().count() - parks;
                        let building_count = stats.buildings.iter().count();
                        let mean_height = if building_count == 0 {
                            0.0
                        } else {
                            stats.buildings.iter().map(|b| b.height).sum::<f32>()
                                / building_count as f32
                        };

                        egui::Grid::new("stats_grid").num_columns(2).show(ui, |ui| {
                            ui.label("Blocks");
                            ui.label(stats.blocks.iter().count().to_string());
                            ui.end_row();
                            ui.label("Buildings");
                            ui.label(building_count.to_string());
                            ui.end_row();
                            ui.label("Mean height");
                            ui.label(format!("{mean_height:.1}"));
                            ui.end_row();
                            ui.label("Parks");
                            ui.label(parks.to_string());
                            ui.end_row();
                            ui.label("Plazas");
                            ui.label(plazas.to_string());
                            ui.end_row();
                            ui.label("Courtyards");
                            ui.label(stats.courtyards.iter().count().to_string());
                            ui.end_row();
                        });
                    });

                ui.separator();

                if ui
                    .button("Export OBJ")
                    .on_hover_text("Writes buildings and open space next to the executable")
                    .clicked()
                {
                    export.write(ExportEvent {
                        filename: format!("city_{}.obj", seed.0),
                    });
                }
                if !export_status.0.is_empty() {
                    ui.label(egui::RichText::new(&export_status.0).small().weak());
                }

                ui.separator();
                ui.label(
                    egui::RichText::new("WASD pan  ·  scroll zoom  ·  F1 panel  ·  Esc quit")
                        .small()
                        .weak(),
                );
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
