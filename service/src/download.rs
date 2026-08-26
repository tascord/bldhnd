use {
    download::{
        DownloadBackend, DownloadItem as DownloadItemTrait, DownloadProgress, DownloadState as DownloadStateTrait,
        SearchBackend, SearchHit,
    },
    redb::{Database, ReadableDatabase, ReadableTable, TableDefinition},
    serde::{Deserialize, Serialize},
    std::{path::PathBuf, sync::Arc},
    tokio::task::JoinSet,
};

pub static DOWNLOAD_DB: std::sync::LazyLock<Arc<Database>> = std::sync::LazyLock::new(|| {
    let dir = data_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).expect("Failed to create data dir");
    }
    let db_path = dir.join("downloads.db");
    Arc::new(Database::create(&db_path).expect("Failed to open downloads db"))
});

pub fn data_dir() -> PathBuf {
    let p = if let Ok(b) = std::env::var("BLDHND_DIR") {
        PathBuf::from(b).join("data")
    } else if let Ok(x) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(x).join("bldhnd")
    } else {
        std::env::home_dir().expect("No home dir").join(".local/share/").join("bldhnd")
    };

    if !p.exists() {
        std::fs::create_dir_all(&p).expect("Failed to create data dir");
    }
    p
}

pub fn download_dir() -> PathBuf {
    let p = if let Ok(b) = std::env::var("BLDHND_DIR") {
        PathBuf::from(b).join("downloads")
    } else if let Ok(x) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(x).join("bldhnd").join("downloads")
    } else {
        std::env::home_dir().expect("No home dir").join(".local/share/").join("bldhnd").join("downloads")
    };

    if !p.exists() {
        std::fs::create_dir_all(&p).expect("Failed to create download dir");
    }
    p
}

const DOWNLOADS_TABLE: TableDefinition<u64, &str> = TableDefinition::new("downloads");
const PARTIAL_TABLE: TableDefinition<u64, &str> = TableDefinition::new("partial");

pub fn init() {
    let db = &*DOWNLOAD_DB;
    let mut tx = db.begin_write().expect("Failed to begin write");
    tx.open_table(DOWNLOADS_TABLE).expect("Failed to create downloads table");
    tx.open_table(PARTIAL_TABLE).expect("Failed to create partial table");
    tx.commit().expect("Failed to commit");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialDownload {
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub checkpoint_data: Option<Vec<u8>>,
    pub updated_at: i64,
}

impl PartialDownload {
    pub fn new(bytes_done: u64, total_bytes: u64) -> Self {
        Self { bytes_done, total_bytes, checkpoint_data: None, updated_at: chrono::Utc::now().timestamp() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadState {
    Queued,
    Connecting,
    Downloading,
    Complete,
    Failed,
    Cancelled,
}

impl From<DownloadState> for download::DownloadState {
    fn from(s: DownloadState) -> Self {
        match s {
            DownloadState::Queued => download::DownloadState::Queued,
            DownloadState::Connecting => download::DownloadState::Connecting,
            DownloadState::Downloading => download::DownloadState::Downloading,
            DownloadState::Complete => download::DownloadState::Complete,
            DownloadState::Failed => download::DownloadState::Failed,
            DownloadState::Cancelled => download::DownloadState::Cancelled,
        }
    }
}

impl From<download::DownloadState> for DownloadState {
    fn from(s: download::DownloadState) -> Self {
        match s {
            download::DownloadState::Queued => DownloadState::Queued,
            download::DownloadState::Connecting => DownloadState::Connecting,
            download::DownloadState::Downloading => DownloadState::Downloading,
            download::DownloadState::Complete => DownloadState::Complete,
            download::DownloadState::Failed => DownloadState::Failed,
            download::DownloadState::Cancelled => DownloadState::Cancelled,
        }
    }
}

impl std::fmt::Display for DownloadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub state: DownloadState,
    pub speed_bps: u64,
}

impl Progress {
    pub fn percent(&self) -> f64 {
        if self.total_bytes == 0 { 0.0 } else { (self.bytes_done as f64 / self.total_bytes as f64) * 100.0 }
    }
}

impl From<download::DownloadProgress> for Progress {
    fn from(p: download::DownloadProgress) -> Self {
        Progress { bytes_done: p.bytes_done, total_bytes: p.total_bytes, state: p.state.into(), speed_bps: p.speed_bps }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub backend: String,
    pub title: String,
    pub filename: String,
    pub url: String,
    pub size: u64,
    pub year: Option<u32>,
    pub media_type: MediaType,
    pub info_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Music,
    Movie,
    TvShow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: u64,
    pub backend: String,
    pub title: String,
    pub filename: String,
    pub download_path: Option<PathBuf>,
    pub size: u64,
    pub year: Option<u32>,
    pub media_type: MediaType,
    pub state: DownloadState,
    pub created_at: i64,
}

pub struct DownloadManager {
    next_id: std::sync::atomic::AtomicU64,
    active_downloads: std::sync::Mutex<Vec<u64>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        let next_id = Self::load_max_id().unwrap_or(1);
        DownloadManager {
            next_id: std::sync::atomic::AtomicU64::new(next_id),
            active_downloads: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn load_max_id() -> Option<u64> {
        let db = &*DOWNLOAD_DB;
        let tx = db.begin_read().ok()?;
        let table = tx.open_table(DOWNLOADS_TABLE).ok()?;
        if let Ok(Some((k, _))) = table.last() {
            return Some(k.value());
        }
        None
    }

    pub fn create_download(&self, item: DownloadItem) -> anyhow::Result<u64> {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let db = &*DOWNLOAD_DB;
        let mut tx = db.begin_write()?;
        {
            let mut table = tx.open_table(DOWNLOADS_TABLE)?;
            let download = Download {
                id,
                backend: item.backend.to_string(),
                title: item.title,
                filename: item.filename,
                download_path: None,
                size: item.size,
                year: item.year,
                media_type: item.media_type,
                state: DownloadState::Queued,
                created_at: chrono::Utc::now().timestamp(),
            };
            let json = serde_json::to_string(&download)?;
            table.insert(id, json.as_str())?;
        }
        tx.commit()?;

        {
            let mut active = self.active_downloads.lock().unwrap();
            active.push(id);
        }

        Ok(id)
    }

    pub fn get_download(&self, id: u64) -> anyhow::Result<Option<Download>> {
        let db = &*DOWNLOAD_DB;
        let tx = db.begin_read()?;
        let table = tx.open_table(DOWNLOADS_TABLE)?;

        if let Ok(Some(value)) = table.get(id) {
            let download: Download = serde_json::from_str(value.value())?;
            Ok(Some(download))
        } else {
            Ok(None)
        }
    }

    pub fn list_downloads(&self) -> anyhow::Result<Vec<Download>> {
        let db = &*DOWNLOAD_DB;
        let tx = db.begin_read()?;
        let table = tx.open_table(DOWNLOADS_TABLE)?;

        let mut downloads = Vec::new();
        let start = table.first()?.map(|(k, _)| k.value()).unwrap_or(0);
        let end = table.last()?.map(|(k, _)| k.value()).unwrap_or(u64::MAX);

        for i in start..=end {
            if let Ok(Some(value)) = table.get(i) {
                if let Ok(d) = serde_json::from_str::<Download>(value.value()) {
                    downloads.push(d);
                }
            }
        }
        Ok(downloads)
    }

    pub fn update_state(&self, id: u64, state: DownloadState) -> anyhow::Result<()> {
        let db = &*DOWNLOAD_DB;
        let mut tx = db.begin_write()?;
        {
            // One table binding per transaction — redb rejects reopening.
            let mut table = tx.open_table(DOWNLOADS_TABLE)?;
            let current = table.get(id)?.map(|v| v.value().to_string());
            if let Some(s) = current {
                let mut download: Download = serde_json::from_str(&s)?;
                download.state = state;
                table.insert(id, serde_json::to_string(&download)?.as_str())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn update_path(&self, id: u64, path: PathBuf) -> anyhow::Result<()> {
        let db = &*DOWNLOAD_DB;
        let mut tx = db.begin_write()?;
        {
            // One table binding per transaction — redb rejects reopening.
            let mut table = tx.open_table(DOWNLOADS_TABLE)?;
            let current = table.get(id)?.map(|v| v.value().to_string());
            if let Some(s) = current {
                let mut download: Download = serde_json::from_str(&s)?;
                download.download_path = Some(path);
                download.state = DownloadState::Complete;
                table.insert(id, serde_json::to_string(&download)?.as_str())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn is_active(&self, id: u64) -> bool {
        let active = self.active_downloads.lock().unwrap();
        active.contains(&id)
    }

    pub fn mark_complete(&self, id: u64) {
        let mut active = self.active_downloads.lock().unwrap();
        active.retain(|&i| i != id);
    }

    pub fn save_partial(
        &self,
        id: u64,
        bytes_done: u64,
        total_bytes: u64,
        checkpoint_data: Option<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let db = &*DOWNLOAD_DB;
        let mut tx = db.begin_write()?;
        {
            let mut table = tx.open_table(PARTIAL_TABLE)?;
            let partial =
                PartialDownload { bytes_done, total_bytes, checkpoint_data, updated_at: chrono::Utc::now().timestamp() };
            let json = serde_json::to_string(&partial)?;
            table.insert(id, json.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_partial(&self, id: u64) -> anyhow::Result<Option<PartialDownload>> {
        let db = &*DOWNLOAD_DB;
        let tx = db.begin_read()?;
        let table = tx.open_table(PARTIAL_TABLE)?;
        if let Ok(Some(value)) = table.get(id) {
            let partial: PartialDownload = serde_json::from_str(value.value())?;
            Ok(Some(partial))
        } else {
            Ok(None)
        }
    }

    pub fn delete_partial(&self, id: u64) -> anyhow::Result<()> {
        let db = &*DOWNLOAD_DB;
        let mut tx = db.begin_write()?;
        {
            let mut table = tx.open_table(PARTIAL_TABLE)?;
            table.remove(id)?;
        }
        tx.commit()?;
        Ok(())
    }
}

impl Default for DownloadManager {
    fn default() -> Self { Self::new() }
}
