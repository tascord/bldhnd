use {
    redb::{Database, ReadableDatabase, TableDefinition},
    serde::{Deserialize, Serialize},
    std::{path::PathBuf, sync::Arc},
};

pub static CONFIG_DB: std::sync::LazyLock<Arc<Database>> = std::sync::LazyLock::new(|| {
    let dir = config_dir();
    let db_path = dir.join("config.db");
    let db = Arc::new(Database::create(&db_path).expect("Failed to open config db"));

    // Ensure the table exists before anything reads from it — a fresh
    // Database::create contains no tables and open_table would panic.
    let tx = db.begin_write().expect("Failed to begin write");
    tx.open_table(CONFIG_TABLE).expect("Failed to create config table");
    tx.commit().expect("Failed to commit config table creation");

    db
});

pub fn config_dir() -> PathBuf {
    let p = if let Ok(b) = std::env::var("BLDHND_DIR") {
        PathBuf::from(b).join("config")
    } else if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(x).join("bldhnd")
    } else {
        std::env::home_dir().expect("No home dir").join(".config/").join("bldhnd")
    };

    if !p.exists() {
        std::fs::create_dir_all(&p).expect("Failed to create config dir");
    }
    p
}

const CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("config");

pub fn init() {
    let db = &*CONFIG_DB;
    let mut tx = db.begin_write().expect("Failed to begin write");
    tx.open_table(CONFIG_TABLE).expect("Failed to create config table");
    tx.commit().expect("Failed to commit");
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub volumes: Vec<Volume>,
    pub bh_server_url: Option<String>,
    pub soulseek_username: Option<String>,
    pub soulseek_password: Option<String>,
    pub download_dir: Option<String>,
    /// Torznab-compatible torrent indexers (Prowlarr/Jackett/NZBgeek etc).
    #[serde(default)]
    pub torrent_indexers: Vec<Indexer>,
    /// NZB search indexers (Torznab or newznab-style APIs).
    #[serde(default)]
    pub usenet_indexers: Vec<Indexer>,
    #[serde(default)]
    pub sabnzbd: Option<ApiKeyEndpoint>,
    /// Optional sections — clients (e.g. the TUI) may omit these.
    #[serde(default)]
    pub quality: QualitySettings,
    #[serde(default)]
    pub notifications: NotificationSettings,
    #[serde(default)]
    pub plex: Option<PlexSettings>,
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


/// SABnzbd-style endpoint keyed by API key.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiKeyEndpoint {
    pub url: String,
    pub api_key: String,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QualitySettings {
    pub max_size_gb: Option<f32>,
    pub min_bitrate: Option<u32>,
    pub preferred_languages: Vec<String>,
    pub preferred_formats: Vec<String>,
}

impl Default for QualitySettings {
    fn default() -> Self {
        QualitySettings {
            max_size_gb: None,
            min_bitrate: None,
            preferred_languages: vec!["en".to_string()],
            preferred_formats: vec!["flac".to_string(), "mp3".to_string(), "aac".to_string()],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub system_notifications: bool,
    pub webhook_url: Option<String>,
}

impl Default for NotificationSettings {
    fn default() -> Self { NotificationSettings { enabled: true, system_notifications: true, webhook_url: None } }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlexSettings {
    pub url: String,
    pub token: Option<String>,
    pub auto_scan: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Volume {
    pub name: String,
    pub path: String,
    pub priority: u8,
    pub max_size_gb: Option<f32>,
}

impl Config {
    pub fn load() -> Self {
        let db = &*CONFIG_DB;
        let tx = db.begin_read().expect("Failed to begin read");
        let table = tx.open_table(CONFIG_TABLE).expect("Failed to open config table");

        let mut config = Config::default();

        if let Ok(Some(value)) = table.get("main") {
            if let Ok(c) = serde_json::from_str(value.value()) {
                config = c;
            }
        }

        config
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let db = &*CONFIG_DB;
        let mut tx = db.begin_write().expect("Failed to begin write");
        {
            let mut table = tx.open_table(CONFIG_TABLE).expect("Failed to open config table");
            let json = serde_json::to_string_pretty(self)?;
            table.insert("main", json.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }
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
