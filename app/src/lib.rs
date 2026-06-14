use std::sync::{Arc, LazyLock, RwLock};
use serde::{Deserialize, Serialize};

pub mod ui;
pub mod events;

static CONFIG: LazyLock<Arc<RwLock<Config>>> = LazyLock::new(Default::default);

pub fn config() -> Arc<RwLock<Config>> {
    CONFIG.clone()
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub volumes: Vec<Volume>
}

#[derive(Serialize, Deserialize)]
pub struct Volume {
    priority: u8,
    path: String,
}