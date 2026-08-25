use {
    super::{BackendConfig, DownloadBackend, DownloadItem, DownloadProgress, DownloadState, SearchBackend, SearchHit},
    async_trait::async_trait,
    futures::Stream,
    slsk::{SearchHit as SlskHit, SlskClient},
    std::{
        path::{Path, PathBuf},
        sync::Arc,
        time::Duration,
    },
};

pub struct SlskSearcher {
    username: String,
    password: String,
}

impl SlskSearcher {
    pub fn new(username: &str, password: &str) -> Self {
        Self { username: username.to_owned(), password: password.to_owned() }
    }
}

#[async_trait]
impl SearchBackend for SlskSearcher {
    fn source(&self) -> &'static str { "soulseek" }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<SearchHit>> {
        let mut client = SlskClient::connect("server.slsknet.org:2416").await?;
        client.login(&self.username, &self.password).await?;
        let hits = client.search(query, Duration::from_secs(10)).await?;
        Ok(hits.into_iter().map(|h| h.into()).collect())
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
        config.soulseek().map(|(u, p)| Arc::new(Self::new(u, p)))
    }
}

impl From<SlskHit> for SearchHit {
    fn from(hit: SlskHit) -> Self {
        let ext = hit.ext.clone();
        let filename = hit.filename.clone();
        let bitrate = hit.bitrate();
        let duration_secs = hit.duration_secs();
        let free_slot = hit.free_slot;
        let upload_speed = hit.upload_speed;
        let in_queue = hit.in_queue;
        let username = hit.username;
        let size = hit.size;
        SearchHit {
            backend: "soulseek",
            title: filename,
            artist: None,
            album: None,
            size,
            ext,
            bitrate,
            duration_secs,
            year: None,
            track: None,
            free_slot,
            upload_speed,
            in_queue,
            username: Some(username),
            seeders: None,
            peers: None,
            url: String::new(),
        }
    }
}

pub struct SlskDownloader {
    username: String,
    password: String,
}

impl SlskDownloader {
    pub fn new(username: &str, password: &str) -> Self {
        Self { username: username.to_owned(), password: password.to_owned() }
    }
}

#[async_trait]
impl DownloadBackend for SlskDownloader {
    fn source(&self) -> &'static str { "soulseek" }

    async fn download(
        &self,
        item: &DownloadItem,
        dest_dir: &Path,
        progress_callback: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> anyhow::Result<PathBuf> {
        let mut client = SlskClient::connect("server.slsknet.org:2416").await?;
        client.login(&self.username, &self.password).await?;

        let username = item.username.clone().unwrap_or_default();
        let filename = item.filename.clone();
        let size = item.size;
        let ext = std::path::Path::new(&item.filename).extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

        let slsk_hit = SlskHit {
            username,
            filename: filename.clone(),
            size,
            ext,
            attrs: vec![],
            free_slot: true,
            upload_speed: 0,
            in_queue: 0,
        };

        if let Some(ref cb) = progress_callback {
            cb(DownloadProgress { bytes_done: 0, total_bytes: size, state: DownloadState::Connecting, speed_bps: 0 });
        }

        let result = client.download(&slsk_hit, dest_dir, None).await;

        if let Some(ref cb) = progress_callback {
            match &result {
                Ok(_) => cb(DownloadProgress {
                    bytes_done: size,
                    total_bytes: size,
                    state: DownloadState::Complete,
                    speed_bps: 0,
                }),
                Err(_) => {
                    cb(DownloadProgress { bytes_done: 0, total_bytes: size, state: DownloadState::Failed, speed_bps: 0 })
                }
            }
        }

        result.map_err(|e| anyhow::anyhow!("Soulseek error: {}", e))
    }

    fn with_config(config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        config.soulseek().map(|(u, p)| Arc::new(Self::new(u, p)))
    }
}
