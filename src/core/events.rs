#[derive(bevy::prelude::Message)]
pub struct RegenerateEvent {
    pub seed: u64,
    pub user_edit: bool,
}

#[derive(bevy::prelude::Message)]
pub struct ExportEvent {
    pub filename: String,
}
