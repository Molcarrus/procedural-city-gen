pub mod export;
pub mod panel;

use bevy::app::{App, Plugin, Update};
use bevy_egui::EguiPrimaryContextPass;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Egui runs in multi-pass mode by default in bevy_egui 0.39, so UI
        // systems must live in this schedule rather than Update.
        app.add_systems(EguiPrimaryContextPass, panel::control_panel)
            .add_systems(Update, export::handle_export);
    }
}
