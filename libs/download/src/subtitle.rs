use {
    super::{BackendConfig, SubtitleBackend, SubtitleHit},
    async_trait::async_trait,
    serde::Deserialize,
    std::{
        path::{Path, PathBuf},
        sync::Arc,
    },
};

pub struct OpenSubtitles {
    api_url: String,
    api_key: Option<String>,
    language: String,
}

impl OpenSubtitles {
    pub fn new(api_url: &str, api_key: Option<String>, language: &str) -> Self {
        Self { api_url: api_url.to_string(), api_key, language: language.to_string() }
    }
}

#[async_trait]
impl SubtitleBackend for OpenSubtitles {
    fn source(&self) -> &'static str { "opensubtitles" }

    async fn search(&self, title: &str, year: Option<u32>, language: &str) -> anyhow::Result<Vec<SubtitleHit>> {
        let client = reqwest::Client::new();
        let mut url =
            format!("{}/search?type=movies&query={}&languages={}", self.api_url, urlencoding::encode(title), language);

        if let Some(y) = year {
            url.push_str(&format!("&year={}", y));
        }

        if let Some(ref key) = self.api_key {
            url.push_str(&format!("&api_key={}", key));
        }

        let resp = client.get(&url).send().await?;
        let results: Vec<SubtitleResult> = resp.json().await?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    async fn download(&self, hit: &SubtitleHit, dest_dir: &Path) -> anyhow::Result<PathBuf> {
        let client = reqwest::Client::new();
        let resp = client.get(&hit.download_url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download subtitle: {}", resp.status()));
        }

        let bytes = resp.bytes().await?;
        let path = dest_dir.join(&hit.filename);
        tokio::fs::write(&path, bytes).await?;

        Ok(path)
    }

    fn with_config(config: &BackendConfig) -> Option<Arc<Self>>
    where
        Self: Sized,
    {
        let url = config.bh_server_url.as_deref().unwrap_or("https://api.opensubtitles.com");
        Some(Arc::new(Self::new(url, config.subtitle_api_key.clone(), "en")))
    }
}

impl From<SubtitleResult> for SubtitleHit {
    fn from(r: SubtitleResult) -> Self {
        SubtitleHit {
            backend: "opensubtitles",
            language: r.language_code.unwrap_or_else(|| "en".to_string()),
            title: r.title,
            filename: r.filename,
            download_url: r.download_url,
            votes: r.votes.unwrap_or(0),
            downloads: r.downloads.unwrap_or(0),
            rating: r.rating,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SubtitleResult {
    id: Option<u64>,
    #[serde(rename = "type")]
    subtype: Option<String>,
    title: String,
    filename: String,
    language_code: Option<String>,
    download_url: String,
    votes: Option<u32>,
    downloads: Option<u32>,
    rating: Option<f32>,
    #[serde(rename = "year")]
    year: Option<u32>,
}
