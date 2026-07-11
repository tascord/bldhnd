use {
    crate::{
        config::Config,
        download::DownloadManager,
        notification::{Notification, NotificationBackend},
    },
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub enabled: bool,
    pub bind_addr: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
}

impl Default for WebConfig {
    fn default() -> Self {
        WebConfig {
            enabled: false,
            bind_addr: "0.0.0.0".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
        }
    }
}

pub async fn start_web_server(config: &WebConfig) -> anyhow::Result<()> {
    if !config.enabled {
        return Ok(());
    }

    let addr = format!("{}:{}", config.bind_addr, config.port);
    tracing::info!("Starting web server on {}", addr);

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchHitWeb>,
    pub backend: String,
}

#[derive(Debug, Serialize)]
pub struct SearchHitWeb {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub size: u64,
    pub format: String,
    pub bitrate: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct DownloadsResponse {
    pub downloads: Vec<DownloadWeb>,
}

#[derive(Debug, Serialize)]
pub struct DownloadWeb {
    pub id: u64,
    pub title: String,
    pub filename: String,
    pub backend: String,
    pub state: String,
    pub progress_percent: f64,
    pub created_at: i64,
}

impl From<crate::download::Download> for DownloadWeb {
    fn from(d: crate::download::Download) -> Self {
        DownloadWeb {
            id: d.id,
            title: d.title,
            filename: d.filename,
            backend: d.backend,
            state: format!("{}", d.state),
            progress_percent: 0.0,
            created_at: d.created_at,
        }
    }
}
