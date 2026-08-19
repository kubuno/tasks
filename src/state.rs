use crate::config::{InstanceConfig, Settings};
use sqlx::PgPool;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub db:       PgPool,
    pub settings: Arc<Settings>,
    /// Admin-editable instance settings, refreshed in the background from the core.
    pub instance: Arc<RwLock<InstanceConfig>>,
}

impl AppState {
    /// Snapshot of the current instance settings. Takes the read lock briefly and
    /// returns a cheap `Copy`, so callers never hold the lock across `.await`.
    /// Falls back to the compiled defaults if the lock is poisoned rather than
    /// panicking on a path that records user attachments.
    pub fn instance(&self) -> InstanceConfig {
        self.instance
            .read()
            .map(|g| *g)
            .unwrap_or_default()
    }
}
