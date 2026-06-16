use std::{env, fmt::Display, fs::{File, OpenOptions, read_to_string}, io::{Read, Write}, path::{Path, PathBuf}};

use tokio::spawn;
use zeroize::Zeroizing;

use crate::data::{KnowledgeBase, mb, tm, tv};

use {
    serde::{Deserialize, Serialize},
    std::sync::{Arc, LazyLock, RwLock},
};

pub mod events;
pub mod fs;
pub mod ui;
pub mod data;

static CONFIG: LazyLock<Arc<RwLock<Config>>> = LazyLock::new(|| Arc::new(RwLock::new(Config::new())));
pub fn config() -> Arc<RwLock<Config>> { CONFIG.clone() }

pub fn file() -> File {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".config/").join("bldhnd.json");
    OpenOptions::new().create(true).write(true).truncate(true).read(true).open(p).expect("Failed to open config")
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub volumes: Vec<Volume>,
    pub key_mb: String,
    pub key_tm: String,
    pub key_tv: String,
}

impl Config {
    pub fn new() -> Self {
        let mut s = String::new();
        file().read_to_string(&mut s).unwrap();

        if s.is_empty() {
            Config::default()
        } else {
            serde_json::from_str(&s).unwrap()
        }
    }

    pub fn commit(&self) {
        let js = serde_json::to_string_pretty(self).unwrap();
        let mut f = file();

        spawn(async { let v = mb(); let _ = v.login().await;});
        spawn(async { let v = tv(); let _ = v.login().await;});
        spawn(async { let v = tm(); let _ = v.login().await;});

        f.write_all(js.as_bytes()).unwrap();
        f.flush().unwrap();
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Volume {
    priority: u8,
    path: String,
}
