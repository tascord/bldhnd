use {
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub backend: String,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<i32>,
    pub size: u64,
    pub ext: String,
    #[serde(default)]
    pub url: String,
}

pub fn search(query: &str, media_type: &str) -> anyhow::Result<Vec<SearchHit>> {
    let config = crate::config::Config::load();
    let server_url = config.bh_server_url.unwrap_or_else(|| "https://bldhnd.fargone.sh".to_string());

    match media_type {
        "Music" => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let hits = rt.block_on(async {
                let client = bh_server::Router::client(server_url, Default::default());
                client.music((query.to_string(), 0usize)).await
            })?;
            Ok(hits
                .into_iter()
                .map(|r| SearchHit {
                    backend: "bh-server".to_string(),
                    title: format!("{} - {}", r.title, r.primary_artist),
                    artist: Some(r.primary_artist),
                    year: r.release_date.as_ref().and_then(|s| s.split('-').next()?.parse().ok()),
                    size: (r.total_tracks as u64) * 10_000_000,
                    ext: "flac".to_string(),
                    url: String::new(),
                })
                .collect())
        }
        "Movie" | "Series" => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let hits = rt.block_on(async {
                let client = bh_server::Router::client(server_url, Default::default());
                client.media((query.to_string(), 0usize)).await
            })?;
            Ok(hits
                .into_iter()
                .map(|r| SearchHit {
                    backend: "bh-server".to_string(),
                    title: r.title,
                    artist: None,
                    year: r.release_date.as_ref().and_then(|s| s.split('-').next()?.parse().ok()),
                    size: 0,
                    ext: "".to_string(),
                    url: String::new(),
                })
                .collect())
        }
        _ => Ok(Vec::new()),
    }
}
