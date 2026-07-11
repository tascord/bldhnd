use {
    chrono::NaiveDate,
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
}

pub fn search(query: &str, media_type: &str) -> anyhow::Result<Vec<SearchHit>> {
    let config = crate::config::Config::load();
    let server_url = config.bh_server_url.unwrap_or_else(|| "https://bldhnd.fargone.sh".to_string());

    let client = reqwest::blocking::Client::new();

    match media_type {
        "Music" => {
            let url = format!("{}/Music", server_url);
            let resp = client.post(&url).json(&(query.to_string(), 0usize)).send()?;
            let results: Vec<MusicResult> = resp.json()?;
            Ok(results
                .into_iter()
                .map(|r| SearchHit {
                    backend: "bh-server".to_string(),
                    title: format!("{} - {}", r.title, r.primary_artist),
                    artist: Some(r.primary_artist),
                    year: r.release_date.as_ref().and_then(|s| s.split('-').next()?.parse().ok()),
                    size: (r.total_tracks as u64) * 10_000_000,
                    ext: "flac".to_string(),
                })
                .collect())
        }
        "Movie" | "Series" => {
            let url = format!("{}/Media", server_url);
            let resp = client.post(&url).json(&(query.to_string(), 0usize)).send()?;
            let results: Vec<MediaResult> = resp.json()?;
            Ok(results
                .into_iter()
                .map(|r| SearchHit {
                    backend: "bh-server".to_string(),
                    title: r.title,
                    artist: None,
                    year: r.release_date.as_ref().and_then(|s| s.split('-').next()?.parse().ok()),
                    size: 0,
                    ext: "".to_string(),
                })
                .collect())
        }
        _ => Ok(Vec::new()),
    }
}

#[derive(Debug, Deserialize)]
struct MusicResult {
    title: String,
    primary_artist: String,
    total_tracks: u32,
    release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaResult {
    title: String,
    release_date: Option<String>,
}
