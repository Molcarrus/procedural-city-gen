#[derive(bevy::prelude::Message)]
pub struct RegenerateEvent {
    pub seed: u64,
    pub user_edit: bool,
    /// Set when a parameter that shapes the Voronoi skeleton changed (boundary,
    /// generator count, merge threshold). Params that only affect subdivision or
    /// materials can leave this false and reuse the existing skeleton, which is
    /// much cheaper.
    pub rebuild_skeleton: bool,
}

#[derive(bevy::prelude::Message)]
pub struct ExportEvent {
    pub filename: String,
}
