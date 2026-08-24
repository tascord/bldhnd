use {
    futures::Stream,
    std::{fmt, future::Future, pin::Pin},
};

pub trait SearchBackend: Send + Sync {
    fn source(&self) -> &'static str;
    fn search(&self, query: &str) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<SearchHit>>> + Send>>;
    fn search_streaming(
        &self,
        query: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn Stream<Item = SearchHit> + Send>>> + Send>>
    where
        Self: Sized;
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub backend: &'static str,
    pub title: String,
    pub size: u64,
    pub ext: String,
    pub bitrate: Option<u32>,
    pub duration_secs: Option<u32>,
    pub free_slot: bool,
    pub upload_speed: u32,
    pub in_queue: u32,
    pub username: Option<String>,
}

impl fmt::Display for SearchHit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}] ({})", self.title, self.ext, self.backend)
    }
}

pub trait Searcher {
    fn search(&self, query: &str) -> impl Future<Output = anyhow::Result<Vec<SearchHit>>> + Send;
    fn source(&self) -> &'static str;
}
