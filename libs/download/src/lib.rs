pub mod slsk;
pub mod subtitle;
pub mod torrent;
pub mod usenet;

use {
    futures::Stream,
    serde::{Deserialize, Serialize},
    std::{fmt, sync::Arc},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub backend: &'static str,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub size: u64,
    pub ext: String,
    pub bitrate: Option<u32>,
    pub duration_secs: Option<u32>,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub free_slot: bool,
    pub upload_speed: u32,
    pub in_queue: u32,
    pub username: Option<String>,
    pub seeders: Option<u32>,
    pub peers: Option<u32>,
}

impl fmt::Display for SearchHit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}] ({})", self.title, self.ext, self.backend)
    }
}

impl SearchHit {
    pub fn title_artist(&self) -> String {
        match (&self.artist, &self.album) {
            (Some(a), Some(b)) => format!("{} - {} ({})", self.title, a, b),
            (Some(a), None) => format!("{} - {}", self.title, a),
            (None, Some(b)) => format!("{} ({})", self.title, b),
            (None, None) => self.title.clone(),
        }
    }
}

#[async_trait::async_trait]
pub trait SearchBackend: Send + Sync {
    fn source(&self) -> &'static str;
    async fn search(&self, query: &str) -> anyhow::Result<Vec<SearchHit>>;
    async fn search_streaming(&self, query: &str) -> anyhow::Result<Box<dyn Stream<Item = SearchHit> + Send + '_>>;
    fn with_config(_config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        None
    }
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub state: DownloadState,
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Queued,
    Connecting,
    Downloading,
    Complete,
    Failed,
    Cancelled,
}

impl fmt::Display for DownloadState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadState::Queued => write!(f, "queued"),
            DownloadState::Connecting => write!(f, "connecting"),
            DownloadState::Downloading => write!(f, "downloading"),
            DownloadState::Complete => write!(f, "complete"),
            DownloadState::Failed => write!(f, "failed"),
            DownloadState::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub backend: &'static str,
    pub title: String,
    pub filename: String,
    pub size: u64,
    pub info_hash: Option<String>,
    pub nzb_data: Option<Vec<u8>>,
    pub uri: Option<String>,
    pub username: Option<String>,
}

#[async_trait::async_trait]
pub trait DownloadBackend: Send + Sync {
    fn source(&self) -> &'static str;
    async fn download(
        &self,
        item: &DownloadItem,
        dest_dir: &std::path::Path,
        progress_callback: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> anyhow::Result<std::path::PathBuf>;
    fn is_private(&self) -> bool { false }
    fn with_config(_config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        None
    }
}

pub trait BackendRegistry: Send + Sync {
    fn get_search_backend(&self, source: &str) -> Option<Arc<dyn SearchBackend>>;
    fn get_download_backend(&self, source: &str) -> Option<Arc<dyn DownloadBackend>>;
    fn list_search_backends(&self) -> Vec<&'static str>;
    fn list_download_backends(&self) -> Vec<&'static str>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleHit {
    pub backend: &'static str,
    pub language: String,
    pub title: String,
    pub filename: String,
    pub download_url: String,
    pub votes: u32,
    pub downloads: u32,
    pub rating: Option<f32>,
}

#[async_trait::async_trait]
pub trait SubtitleBackend: Send + Sync {
    fn source(&self) -> &'static str;
    async fn search(&self, title: &str, year: Option<u32>, language: &str) -> anyhow::Result<Vec<SubtitleHit>>;
    async fn download(&self, hit: &SubtitleHit, dest_dir: &std::path::Path) -> anyhow::Result<std::path::PathBuf>;
    fn with_config(_config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct BackendConfig {
    pub soulseek_username: Option<String>,
    pub soulseek_password: Option<String>,
    pub bh_server_url: Option<String>,
    pub subtitle_api_key: Option<String>,
    /// Torznab indexer (url, api_key) for torrent search.
    pub torrent_indexer: Option<(String, Option<String>)>,
    /// qBittorrent WebUI (url, username, password).
    pub qbittorrent: Option<(String, String, String)>,
    /// NZB indexer (url, api_key) for usenet search.
    pub usenet_indexer: Option<(String, Option<String>)>,
    /// SABnzbd (url, api_key).
    pub sabnzbd: Option<(String, Option<String>)>,
    /// aria2 JSON-RPC (url, secret).
    pub aria2: Option<(String, Option<String>)>,
}

impl BackendConfig {
    pub fn soulseek(&self) -> Option<(&str, &str)> {
        match (&self.soulseek_username, &self.soulseek_password) {
            (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => Some((u.as_str(), p.as_str())),
            _ => None,
        }
    }
}
