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
        config.bh_server_url.as_ref().map(|url| Arc::new(Self::new(url, None)))
    }
}

impl From<TorrentResult> for SearchHit {
    fn from(r: TorrentResult) -> Self {
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

pub struct TorrentDownloader {
    client_api_url: String,
    api_key: Option<String>,
}

impl TorrentDownloader {
    pub fn new(client_api_url: &str, api_key: Option<String>) -> Self {
        Self { client_api_url: client_api_url.to_string(), api_key }
    }
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
        let torrent_data = if let Some(info_hash) = &item.info_hash {
            self.add_by_hash(info_hash, dest_dir).await?
        } else if let Some(uri) = &item.uri {
            self.add_by_uri(uri, dest_dir).await?
        } else {
            return Err(anyhow::anyhow!("No torrent info_hash or URI provided"));
        };

        if let Some(cb) = progress_callback {
            cb(DownloadProgress {
                bytes_done: item.size,
                total_bytes: item.size,
                state: DownloadState::Complete,
                speed_bps: 0,
            });
        }

        Ok(dest_dir.join(&item.filename))
    }

    fn with_config(config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        let url = config.bh_server_url.as_deref().unwrap_or("http://localhost:8080");
        Some(Arc::new(Self::new(url, None)))
    }
}

impl TorrentDownloader {
    async fn add_by_hash(&self, info_hash: &str, dest_dir: &Path) -> anyhow::Result<Vec<u8>> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v2/torrents/add", self.client_api_url);

        let savepath = dest_dir.to_string_lossy().to_string();
        let mut params = vec![("hash", info_hash), ("savepath", savepath.as_str())];

        if let Some(key) = &self.api_key {
            params.push(("apiKey", key.as_str()));
        }

        let resp = client.post(&url).form(&params).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to add torrent: {}", resp.status()));
        }

        Ok(Vec::new())
    }

    async fn add_by_uri(&self, uri: &str, dest_dir: &Path) -> anyhow::Result<Vec<u8>> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v2/torrents/add", self.client_api_url);

        let savepath = dest_dir.to_string_lossy().to_string();
        let mut params = vec![("urls", uri), ("savepath", savepath.as_str())];

        if let Some(key) = &self.api_key {
            params.push(("apiKey", key.as_str()));
        }

        let resp = client.post(&url).form(&params).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to add torrent: {}", resp.status()));
        }

        Ok(Vec::new())
    }
}
