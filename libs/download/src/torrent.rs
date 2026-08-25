use {
    crate::{BackendConfig, DownloadBackend, DownloadItem, DownloadProgress, DownloadState, SearchBackend, SearchHit},
    async_trait::async_trait,
    futures::Stream,
    serde::Deserialize,
    std::{
        path::{Path, PathBuf},
        sync::Arc,
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

/// Embedded torrent downloader — no external client required.
///
/// Spins up a throwaway librqbit session per download (DHT bootstraps each
/// time; fine for one-shot fetches). Accepts magnet URIs (including ones
/// built from bare info hashes) and http(s) .torrent links.
#[derive(Default)]
pub struct TorrentDownloader;

impl TorrentDownloader {
    pub fn new() -> Self { Self }
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

        std::fs::create_dir_all(dest_dir)?;

        let session = librqbit::Session::new(dest_dir.to_path_buf()).await?;
        let response = session
            .add_torrent(
                librqbit::AddTorrent::from_url(&uri),
                Some(librqbit::AddTorrentOptions::default()),
            )
            .await?;
        let handle =
            response.into_handle().ok_or_else(|| anyhow::anyhow!("torrent already active"))?;

        handle.wait_until_completed().await.map_err(|e| anyhow::anyhow!("torrent failed: {e}"))?;

        // Best-effort real content name for the completed path.
        let name = handle.name().unwrap_or_else(|| item.filename.clone());
        let final_path = dest_dir.join(name);

        if let Some(cb) = progress_callback {
            cb(DownloadProgress {
                bytes_done: item.size,
                total_bytes: item.size,
                state: DownloadState::Complete,
                speed_bps: 0,
            });
        }

        Ok(final_path)
    }
}
