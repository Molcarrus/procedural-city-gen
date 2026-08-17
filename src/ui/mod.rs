pub mod export;
pub mod panel;

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        resource::Resource,
        system::{Res, ResMut},
    },
    input::{ButtonInput, keyboard::KeyCode},
};
use bevy_egui::EguiPrimaryContextPass;

/// Whether the control panel is drawn. Toggled with F1 so the city can be
/// viewed unobstructed.
#[derive(Resource)]
pub struct UiVisible(pub bool);

/// Rebuild while a slider is still being dragged, rather than on release.
/// Off by default: a drag would otherwise rebuild the whole city every frame.
#[derive(Resource, Default)]
pub struct LiveUpdate(pub bool);

/// Result of the most recent export, shown under the button.
#[derive(Resource, Default)]
pub struct ExportStatus(pub String);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Egui runs in multi-pass mode by default in bevy_egui 0.39, so UI
        // systems must live in this schedule rather than Update.
        app.insert_resource(UiVisible(true))
            .init_resource::<LiveUpdate>()
            .init_resource::<ExportStatus>()
            .add_systems(EguiPrimaryContextPass, panel::control_panel)
            .add_systems(Update, (export::handle_export, toggle_panel));
    }
}

fn toggle_panel(keys: Res<ButtonInput<KeyCode>>, mut visible: ResMut<UiVisible>) {
    if keys.just_pressed(KeyCode::F1) {
        visible.0 = !visible.0;
    }
}
