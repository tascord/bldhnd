use {
    crate::Config,
    ipsea::{self, StreamResponse},
    serde::{Deserialize, Serialize},
    std::{
        io::{Read, Write},
        path::Path,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Request {
    GetConfig,
    UpdateConfig {
        config: Config,
    },
    CommitConfig,
    Search {
        query: String,
        #[serde(default)]
        media_type: String,
        #[serde(default)]
        backend: String,
    },
    ListDownloads,
    GetDownload {
        id: u64,
    },
    StartDownload {
        backend: String,
        title: String,
        filename: String,
        url: String,
        size: u64,
        year: Option<u32>,
        media_type: String,
    },
    DownloadProgress {
        id: u64,
    },
    RetryDownload {
        id: u64,
    },
    CancelDownload {
        id: u64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Response {
    GetConfig { config: Config },
    UpdateConfig,
    CommitConfig,
    Search { results: Vec<SearchHit> },
    ListDownloads { downloads: Vec<DownloadInfo> },
    GetDownload { download: Option<DownloadInfo> },
    StartDownload { id: u64 },
    DownloadProgress { progress: ProgressInfo },
    RetryDownload,
    CancelDownload,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub backend: String,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<i32>,
    pub size: u64,
    pub ext: String,
    /// Magnet / .torrent link for download-backend hits.
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub id: u64,
    pub backend: String,
    pub title: String,
    pub filename: String,
    pub download_path: Option<String>,
    pub size: u64,
    pub year: Option<u32>,
    pub media_type: String,
    pub state: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub bytes_done: u64,
    pub total_bytes: u64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub speed_bps: u64,
}

pub struct Client {
    socket_path: std::path::PathBuf,
}

impl Client {
    pub fn new(socket_path: &Path) -> Self {
        let socket_path_str = socket_path.to_string_lossy();
        let socket_path = if socket_path_str.starts_with('/') {
            socket_path.to_path_buf()
        } else {
            std::path::PathBuf::from("/tmp").join(format!("{}.sock", socket_path_str))
        };
        Client { socket_path }
    }

    pub fn connect() -> Self {
        // Must match the service: the ipsea IPC layer binds at /tmp/{name}.sock.
        Client { socket_path: std::path::PathBuf::from("/tmp/bldhnd.sock") }
    }

    fn send_request(&self, req: Request) -> anyhow::Result<Response> {
        let stream = std::os::unix::net::UnixStream::connect(&self.socket_path).map_err(|e| {
            anyhow::anyhow!(
                "Cannot reach bldhnd service at {}: {e}. Is the service running?",
                self.socket_path.display()
            )
        })?;
        let mut stream = stream;

        let data = serde_json::to_vec(&req)?;
        let len_bytes = (data.len() as u32).to_le_bytes();
        stream.write_all(&len_bytes)?;
        stream.write_all(&data)?;
        stream.flush()?;

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let resp_len = u32::from_le_bytes(len_buf) as usize;

        let mut buf = vec![0u8; resp_len];
        stream.read_exact(&mut buf)?;

        match serde_json::from_slice::<StreamResponse<Response>>(&buf)? {
            StreamResponse::Data(resp) => Ok(resp),
            StreamResponse::EndOfStream => Err(anyhow::anyhow!("Unexpected end of stream")),
        }
    }

    pub fn get_config(&self) -> anyhow::Result<Config> {
        match self.send_request(Request::GetConfig)? {
            Response::GetConfig { config } => Ok(config),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn update_config(&self, config: Config) -> anyhow::Result<()> {
        match self.send_request(Request::UpdateConfig { config })? {
            Response::UpdateConfig => Ok(()),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn commit_config(&self) -> anyhow::Result<()> {
        match self.send_request(Request::CommitConfig)? {
            Response::CommitConfig => Ok(()),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn search(&self, query: &str, media_type: &str) -> anyhow::Result<Vec<SearchHit>> {
        match self.send_request(Request::Search {
            query: query.to_string(),
            media_type: media_type.to_string(),
            backend: String::new(),
        })? {
            Response::Search { results } => Ok(results),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    /// Search a download backend (soulseek/torrent/usenet) by media type.
    /// The service maps the media_type to the appropriate backend.
    pub fn search_backend(&self, query: &str, media_type: &str) -> anyhow::Result<Vec<SearchHit>> {
        let backend = match media_type {
            "Music" => "soulseek",
            "Movie" | "Series" | "TvShow" => "torrent",
            _ => "soulseek",
        };
        match self.send_request(Request::Search {
            query: query.to_string(),
            media_type: String::new(),
            backend: backend.to_string(),
        })? {
            Response::Search { results } => Ok(results),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn list_downloads(&self) -> anyhow::Result<Vec<DownloadInfo>> {
        match self.send_request(Request::ListDownloads)? {
            Response::ListDownloads { downloads } => Ok(downloads),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn get_download(&self, id: u64) -> anyhow::Result<Option<DownloadInfo>> {
        match self.send_request(Request::GetDownload { id })? {
            Response::GetDownload { download } => Ok(download),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn download_progress(&self, id: u64) -> anyhow::Result<ProgressInfo> {
        match self.send_request(Request::DownloadProgress { id })? {
            Response::DownloadProgress { progress } => Ok(progress),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn cancel_download(&self, id: u64) -> anyhow::Result<()> {
        match self.send_request(Request::CancelDownload { id })? {
            Response::CancelDownload => Ok(()),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn retry_download(&self, id: u64) -> anyhow::Result<()> {
        match self.send_request(Request::RetryDownload { id })? {
            Response::RetryDownload => Ok(()),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }

    pub fn start_download(
        &self,
        backend: &str,
        title: &str,
        filename: &str,
        url: &str,
        size: u64,
        year: Option<u32>,
        media_type: &str,
    ) -> anyhow::Result<u64> {
        match self.send_request(Request::StartDownload {
            backend: backend.to_string(),
            title: title.to_string(),
            filename: filename.to_string(),
            url: url.to_string(),
            size,
            year,
            media_type: media_type.to_string(),
        })? {
            Response::StartDownload { id } => Ok(id),
            Response::Error { message } => Err(anyhow::anyhow!("Error: {}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    }
}
