use {
    crate::{BackendConfig, DownloadBackend, DownloadItem, DownloadProgress, DownloadState, SearchBackend, SearchHit},
    async_trait::async_trait,
    futures::Stream,
    serde::Deserialize,
    std::{
        path::{Path, PathBuf},
        sync::Arc,
        time::Duration,
    },
};

pub struct TorrentSearcher {
    indexer_url: String,
    api_key: Option<String>,
}

impl TorrentSearcher {
    pub fn new(indexer_url: &str, api_key: Option<String>) -> Self { Self { indexer_url: indexer_url.to_string(), api_key } }
}

#[async_trait]
impl SearchBackend for TorrentSearcher {
    fn source(&self) -> &'static str { "torrent" }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<SearchHit>> {
        let client = reqwest::Client::new();
        let mut url = format!("{}/api?t=search&q={}&o=json", self.indexer_url, urlencoding::encode(query));

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let resp = client.get(&url).send().await?;
        let results: Vec<TorrentResult> = resp.json().await?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    async fn search_streaming(&self, query: &str) -> anyhow::Result<Box<dyn Stream<Item = SearchHit> + Send + '_>> {
        let hits = self.search(query).await?;
        let stream = futures::stream::iter(hits);
        Ok(Box::new(stream) as Box<dyn Stream<Item = SearchHit> + Send + '_>)
    }

    fn with_config(config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        // Prefer the configured Torznab indexer; fall back to bh_server_url.
        config
            .torrent_indexer
            .as_ref()
            .map(|(url, key)| Arc::new(Self::new(url, key.clone())))
            .or_else(|| config.bh_server_url.as_ref().map(|url| Arc::new(Self::new(url, None))))
    }
}

impl From<TorrentResult> for SearchHit {
    fn from(r: TorrentResult) -> Self {
        // Prefer the direct .torrent enclosure; fall back to a magnet built
        // from the info hash (resolvable via DHT).
        let url = match &r.torrent_link {
            Some(link) if !link.is_empty() => link.clone(),
            _ => r.info_hash.as_ref().map(|h| format!("magnet:?xt=urn:btih:{h}")).unwrap_or_default(),
        };
        SearchHit {
            backend: "torrent",
            title: r.name,
            artist: None,
            album: None,
            size: r.size,
            ext: "torrent".to_string(),
            bitrate: None,
            duration_secs: None,
            year: None,
            track: None,
            free_slot: true,
            upload_speed: 0,
            in_queue: 0,
            username: None,
            seeders: r.seeders,
            peers: r.peers,
            url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct TorrentResult {
    id: Option<u64>,
    name: String,
    size: u64,
    seeders: Option<u32>,
    peers: Option<u32>,
    leechers: Option<u32>,
    category: Option<String>,
    torrent_link: Option<String>,
    info_hash: Option<String>,
}

/// Embedded torrent downloader backed by one **persistent** session.
///
/// All downloads share a single librqbit session living on a dedicated
/// thread + runtime: DHT bootstraps once, completed torrents stay in the
/// session (seeding-capable), re-adding a known torrent returns its existing
/// handle, and in-flight downloads can be cancelled by key.
#[derive(Default)]
pub struct TorrentDownloader;

impl TorrentDownloader {
    pub fn new() -> Self { Self }
}

/// Stable identity for a torrent across retry/resume: info hash when known,
/// else the uri itself.
pub fn torrent_key(uri: &str, info_hash: Option<&str>) -> String {
    match info_hash.filter(|h| !h.is_empty()) {
        Some(h) => h.to_lowercase(),
        None => uri.to_string(),
    }
}

enum TorrentCmd {
    Download {
        key: String,
        uri: String,
        dest_dir: PathBuf,
        progress: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
        reply: tokio::sync::oneshot::Sender<anyhow::Result<PathBuf>>,
    },
    Cancel {
        key: String,
    },
}

/// Handle used to talk to the persistent session thread.
static TORRENT_TX: std::sync::LazyLock<std::sync::mpsc::Sender<TorrentCmd>> = std::sync::LazyLock::new(|| {
    let (tx, rx) = std::sync::mpsc::channel::<TorrentCmd>();
    std::thread::Builder::new()
        .name("torrent-session".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("torrent runtime");
            rt.block_on(session_loop(rx));
        })
        .expect("spawn torrent-session thread");
    tx
});

/// key → librqbit torrent id, so cancellation can find active downloads.
static TORRENT_IDS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, usize>>>=
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

async fn session_loop(rx: std::sync::mpsc::Receiver<TorrentCmd>) {
    // Default output folder; per-download dest is passed via options.
    let session = match librqbit::Session::new(std::env::temp_dir()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("torrent session init failed: {e}");
            return;
        }
    };
    tracing::info!("persistent torrent session up");

    // Bridge the blocking std channel into this runtime.
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<TorrentCmd>();
    std::thread::spawn(move || {
        for cmd in rx {
            if cmd_tx.send(cmd).is_err() {
                break;
            }
        }
    });

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            TorrentCmd::Download { key, uri, dest_dir, progress, reply } => {
                let session = session.clone();
                tokio::spawn(async move {
                    let result = run_torrent(&session, &key, &uri, &dest_dir, progress).await;
                    let _ = reply.send(result);
                });
            }
            TorrentCmd::Cancel { key } => {
                let id = TORRENT_IDS.lock().unwrap().get(&key).copied();
                if let Some(id) = id {
                    match session.delete(librqbit::api::TorrentIdOrHash::Id(id), false).await {
                        Ok(()) => {
                            TORRENT_IDS.lock().unwrap().remove(&key);
                            tracing::info!("cancelled torrent '{key}'");
                        }
                        Err(e) => tracing::warn!("cancel torrent '{key}' failed: {e}"),
                    }
                }
            }
        }
    }
}

/// Add the torrent to the shared session (or reuse its existing handle),
/// stream progress, wait for completion, return the content path.
async fn run_torrent(
    session: &Arc<librqbit::Session>,
    key: &str,
    uri: &str,
    dest_dir: &Path,
    progress: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(dest_dir)?;
    let response = session
        .add_torrent(
            librqbit::AddTorrent::from_url(uri),
            Some(librqbit::AddTorrentOptions {
                output_folder: Some(dest_dir.to_string_lossy().to_string()),
                // Write on top of existing files so resumed/retried
                // downloads continue instead of erroring out.
                overwrite: true,
                ..Default::default()
            }),
        )
        .await?;
    let handle =
        response.into_handle().ok_or_else(|| anyhow::anyhow!("list-only torrent handle"))?;
    TORRENT_IDS.lock().unwrap().insert(key.to_string(), handle.id());

    // Stream byte-level progress out via the callback while downloading.
    let tick_handle = handle.clone();
    let ticker = progress.as_ref().map(|cb| {
        let cb = Arc::clone(cb);
        tokio::spawn(async move {
            loop {
                let s = tick_handle.stats();
                cb(DownloadProgress {
                    bytes_done: s.progress_bytes,
                    total_bytes: s.total_bytes,
                    state: if s.finished { DownloadState::Complete } else { DownloadState::Downloading },
                    speed_bps: (s.live.map(|l| l.download_speed.mbps).unwrap_or(0.0) * 1_000_000.0) as u64,
                });
                if s.finished || s.error.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
    });

    let waited = handle.wait_until_completed().await;
    if let Some(t) = &ticker {
        t.abort();
    }
    waited.map_err(|e| anyhow::anyhow!("torrent failed: {e}"))?;

    // Best-effort real content name for the completed path.
    let name = handle.name().unwrap_or_else(|| "download".to_string());

    Ok(dest_dir.join(name))
}

/// Ask the persistent session to download `uri` into `dest_dir`; resolves
/// when the torrent finishes.
async fn session_download(
    key: String,
    uri: String,
    dest_dir: PathBuf,
    progress: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
) -> anyhow::Result<PathBuf> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    TORRENT_TX
        .send(TorrentCmd::Download { key, uri, dest_dir, progress, reply: reply_tx })
        .map_err(|_| anyhow::anyhow!("torrent session unavailable"))?;
    reply_rx.await.map_err(|_| anyhow::anyhow!("torrent session dropped the request"))?
}

/// Cancel an in-flight torrent by key (keeps files on disk).
pub fn cancel_torrent(key: &str) -> bool {
    TORRENT_TX.send(TorrentCmd::Cancel { key: key.to_string() }).is_ok()
}

#[async_trait]
impl DownloadBackend for TorrentDownloader {
    fn source(&self) -> &'static str { "torrent" }

    async fn download(
        &self,
        item: &DownloadItem,
        dest_dir: &Path,
        progress_callback: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> anyhow::Result<PathBuf> {
        // Resolve a usable uri: explicit magnet/.torrent link, or a magnet
        // built from the info hash.
        let uri = match &item.uri {
            Some(u) if !u.is_empty() => u.clone(),
            _ => match &item.info_hash {
                Some(hash) if !hash.is_empty() => format!("magnet:?xt=urn:btih:{hash}"),
                _ => return Err(anyhow::anyhow!("No torrent URI or info_hash provided")),
            },
        };

        if let Some(cb) = progress_callback.as_ref() {
            cb(DownloadProgress {
                bytes_done: 0,
                total_bytes: item.size,
                state: DownloadState::Connecting,
                speed_bps: 0,
            });
        }

        session_download(torrent_key(&uri, item.info_hash.as_deref()), uri, dest_dir.to_path_buf(), progress_callback)
            .await
    }
}
