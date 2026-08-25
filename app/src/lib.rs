use {
    futures_signals::signal::Mutable,
    serde::{Deserialize, Serialize},
    std::sync::{Arc, LazyLock},
};

pub mod data;
pub mod fs;
pub mod ipsea;
pub mod logs;
pub mod ui;

static CONFIG: LazyLock<Arc<Mutable<Config>>> = LazyLock::new(|| Arc::new(Mutable::new(Config::load())));
pub fn config() -> Arc<Mutable<Config>> { CONFIG.clone() }

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub volumes: Vec<Volume>,
    pub soulseek_username: Option<String>,
    pub soulseek_password: Option<String>,
    pub bh_server_url: Option<String>,
    pub download_dir: Option<String>,
    #[serde(default)]
    pub torrent_indexers: Vec<Indexer>,
    #[serde(default)]
    pub usenet_indexers: Vec<Indexer>,
    #[serde(default)]
    pub qbittorrent: Option<ClientEndpoint>,
    #[serde(default)]
    pub sabnzbd: Option<ApiKeyEndpoint>,
    #[serde(default)]
    pub aria2: Option<SecretEndpoint>,
}

impl Config {
    pub fn load() -> Self { ipsea::Client::connect().get_config().unwrap_or_default() }

    pub fn commit(&self) {
        if let Err(e) = ipsea::Client::connect().update_config(self.clone()) {
            tracing::warn!("Failed to save config: {}", e);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

/// A search indexer (torrent or usenet): named endpoint + optional API key.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Indexer {
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
}

impl Indexer {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self { name: name.into(), url: url.into(), api_key: None }
    }
}

/// qBittorrent WebUI login.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientEndpoint {
    pub url: String,
    pub username: String,
    pub password: String,
}

/// SABnzbd-style endpoint keyed by API key.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiKeyEndpoint {
    pub url: String,
    pub api_key: String,
}

/// aria2 JSON-RPC endpoint with a secret token.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecretEndpoint {
    pub url: String,
    pub secret: String,
}
