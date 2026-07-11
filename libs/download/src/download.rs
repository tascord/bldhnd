use {
    futures::Stream,
    std::{fmt, future::Future, path::PathBuf, pin::Pin},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Queued,
    Downloading,
    Complete,
    Failed,
    Cancelled,
}

impl fmt::Display for DownloadState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadState::Queued => write!(f, "queued"),
            DownloadState::Downloading => write!(f, "downloading"),
            DownloadState::Complete => write!(f, "complete"),
            DownloadState::Failed => write!(f, "failed"),
            DownloadState::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub state: DownloadState,
}

impl Progress {
    pub fn percent(&self) -> f64 {
        if self.total_bytes == 0 { 0.0 } else { (self.bytes_done as f64 / self.total_bytes as f64) * 100.0 }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub backend: &'static str,
    pub title: String,
    pub filename: String,
    pub url: String,
    pub size: u64,
    pub year: Option<u32>,
    pub media_type: MediaType,
}

#[derive(Debug, Clone)]
pub enum MediaType {
    Music,
    Movie,
    TvShow,
    Album,
}

pub trait DownloadBackend: Send + Sync {
    fn source(&self) -> &'static str;
    fn download(&self, item: &DownloadItem) -> Pin<Box<dyn Future<Output = anyhow::Result<PathBuf>> + Send>>;
    fn download_streaming(
        &self,
        item: &DownloadItem,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn Stream<Item = Progress> + Send>>> + Send>>
    where
        Self: Sized;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DownloadId(u64);

impl DownloadId {
    pub fn new(id: u64) -> Self { DownloadId(id) }

    pub fn inner(&self) -> u64 { self.0 }
}

pub trait Downloader {
    fn download(&self, item: &DownloadItem) -> impl Future<Output = anyhow::Result<PathBuf>> + Send;
    fn source(&self) -> &'static str;
    fn progress_stream(&self, id: DownloadId) -> impl Stream<Item = Progress> + Send;
}
