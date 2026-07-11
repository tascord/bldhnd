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

pub struct UsenetSearcher {
    api_url: String,
    api_key: Option<String>,
}

impl UsenetSearcher {
    pub fn new(api_url: &str, api_key: Option<String>) -> Self { Self { api_url: api_url.to_string(), api_key } }
}

#[async_trait]
impl SearchBackend for UsenetSearcher {
    fn source(&self) -> &'static str { "usenet" }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<SearchHit>> {
        let client = reqwest::Client::new();
        let mut url = format!("{}/api?t=search&q={}&o=json", self.api_url, urlencoding::encode(query));

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let resp = client.get(&url).send().await?;
        let results: Vec<UsenetResult> = resp.json().await?;

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

impl From<UsenetResult> for SearchHit {
    fn from(r: UsenetResult) -> Self {
        let name = r.name.clone();
        SearchHit {
            backend: "usenet",
            title: name.clone(),
            artist: None,
            album: None,
            size: r.size,
            ext: std::path::Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("").to_string(),
            bitrate: None,
            duration_secs: None,
            year: None,
            track: None,
            free_slot: true,
            upload_speed: 0,
            in_queue: 0,
            username: None,
            seeders: None,
            peers: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct UsenetResult {
    name: String,
    size: u64,
    poster: Option<String>,
    post_date: Option<String>,
    subjects: Option<Vec<String>>,
    total_parts: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    results: Option<Vec<UsenetResult>>,
}

pub struct UsenetDownloader {
    api_url: String,
    api_key: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl UsenetDownloader {
    pub fn new(api_url: &str, api_key: Option<String>, username: Option<String>, password: Option<String>) -> Self {
        Self { api_url: api_url.to_string(), api_key, username, password }
    }
}

#[async_trait]
impl DownloadBackend for UsenetDownloader {
    fn source(&self) -> &'static str { "usenet" }

    async fn download(
        &self,
        item: &DownloadItem,
        dest_dir: &Path,
        progress_callback: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> anyhow::Result<PathBuf> {
        if let Some(data) = &item.nzb_data {
            self.send_to_sabnzbd(data, dest_dir, item.filename.clone(), item.size, progress_callback).await
        } else if let Some(uri) = &item.uri {
            self.download_from_uri(uri, dest_dir, item.filename.clone(), progress_callback).await
        } else {
            Err(anyhow::anyhow!("No NZB data or URI provided"))
        }
    }

    fn with_config(config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        let url = config.bh_server_url.as_deref().unwrap_or("http://localhost:8080");
        Some(Arc::new(Self::new(url, None, None, None)))
    }
}

impl UsenetDownloader {
    async fn send_to_sabnzbd(
        &self,
        nzb_data: &[u8],
        dest_dir: &Path,
        filename: String,
        total_bytes: u64,
        progress_callback: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> anyhow::Result<PathBuf> {
        let client = reqwest::Client::new();
        let url = format!("{}/api", self.api_url);

        let mut form = reqwest::multipart::Form::new().part(
            "nzb_file",
            reqwest::multipart::Part::bytes(nzb_data.to_vec()).file_name("download.nzb").mime_str("application/x-nzb")?,
        );

        let dir = dest_dir.to_string_lossy().to_string();
        let mut params = vec![("mode", "addfile"), ("output_mode", "json"), ("dir", dir.as_str())];

        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            params.push(("ma_username", u.as_str()));
            params.push(("ma_password", p.as_str()));
        }

        if let Some(key) = &self.api_key {
            params.push(("apikey", key.as_str()));
        }

        for (k, v) in params {
            form = form.text(k.to_string(), v.to_string());
        }

        let resp = client.post(&url).multipart(form).send().await?;
        let _result: serde_json::Value = resp.json().await?;

        if let Some(cb) = progress_callback {
            cb(DownloadProgress { bytes_done: total_bytes, total_bytes, state: DownloadState::Complete, speed_bps: 0 });
        }

        Ok(dest_dir.join(filename))
    }

    async fn download_from_uri(
        &self,
        uri: &str,
        dest_dir: &Path,
        filename: String,
        progress_callback: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> anyhow::Result<PathBuf> {
        let client = reqwest::Client::new();
        let resp = client.get(uri).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download NZB: {}", resp.status()));
        }

        let bytes = resp.bytes().await?;

        if let Some(cb) = progress_callback {
            cb(DownloadProgress {
                bytes_done: bytes.len() as u64,
                total_bytes: bytes.len() as u64,
                state: DownloadState::Complete,
                speed_bps: 0,
            });
        }

        Ok(dest_dir.join(filename))
    }
}
