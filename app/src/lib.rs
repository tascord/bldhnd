use {
    futures_signals::signal::Mutable,
    serde::{Deserialize, Serialize},
    std::{
        env,
        fs::{File, OpenOptions},
        io::{Read, Write},
        path::Path,
        sync::{Arc, LazyLock},
    },
    tracing::info,
};

pub mod data;
pub mod events;
pub mod fs;
pub mod logs;
pub mod ui;

static CONFIG: LazyLock<Arc<Mutable<Config>>> = LazyLock::new(|| Arc::new(Mutable::new(Config::new())));
pub fn config() -> Arc<Mutable<Config>> { CONFIG.clone() }

pub fn file() -> File {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".config/").join("bldhnd.json");
    OpenOptions::new().create(true).write(true).truncate(true).read(true).open(p).expect("Failed to open config")
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub volumes: Vec<Volume>,
}

impl Config {
    pub fn new() -> Self {
        let mut s = String::new();
        file().read_to_string(&mut s).unwrap();

        if s.is_empty() { Config::default() } else { serde_json::from_str(&s).unwrap() }
    }

    pub fn commit(&self) {
        let js = serde_json::to_string_pretty(self).unwrap();
        let mut f = file();

        f.write_all(js.as_bytes()).unwrap();
        f.flush().unwrap();

        info!("Changes saved!")
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Volume {
    pub name: String,
    pub path: String,
    pub priority: u8,
    pub max_size_gb: Option<f32>,
}

impl Volume {
    pub fn new(name: impl Into<String>, path: impl Into<String>, priority: u8) -> Self {
        Self { name: name.into(), path: path.into(), priority, max_size_gb: None }
    }

    pub fn with_max_size(mut self, max_size_gb: f32) -> Self {
        self.max_size_gb = Some(max_size_gb);
        self
    }
}