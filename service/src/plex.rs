use {
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlexConfig {
    pub url: String,
    pub token: Option<String>,
    pub library_sections: Vec<PlexLibrarySection>,
    pub auto_scan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlexLibrarySection {
    pub name: String,
    pub type_filter: String,
}

pub struct PlexClient {
    url: String,
    token: Option<String>,
}

impl PlexClient {
    pub fn new(url: &str, token: Option<String>) -> Self { Self { url: url.to_string(), token } }

    pub async fn scan_library(&self, section_name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();

        let mut url = format!("{}/library/sections", self.url);
        if let Some(ref token) = self.token {
            url.push_str(&format!("?X-Plex-Token={}", token));
        }

        let sections: PlexSectionsResponse = client.get(&url).send().await?.json().await?;

        let section = sections.media_container.directory.iter().find(|s| s.title == section_name);

        if let Some(section) = section {
            let scan_url = format!("{}/library/sections/{}/refresh", self.url, section.key);
            let mut req = client.put(&scan_url);
            if let Some(ref token) = self.token {
                req = req.header("X-Plex-Token", token);
            }
            req.send().await?;
        }

        Ok(())
    }

    pub async fn process_download(&self, path: &PathBuf, media_type: &str) -> anyhow::Result<()> {
        let section = match media_type {
            "Music" | "music" => "Music",
            "Movie" | "movie" => "Movies",
            "TvShow" | "series" => "TV",
            _ => return Ok(()),
        };

        self.scan_library(section).await
    }
}

#[derive(Debug, Deserialize)]
struct PlexSectionsResponse {
    media_container: PlexMediaContainer,
}

#[derive(Debug, Deserialize)]
struct PlexMediaContainer {
    #[serde(rename = "Directory", alias = "directory")]
    directory: Vec<PlexDirectory>,
}

#[derive(Debug, Deserialize)]
struct PlexDirectory {
    key: String,
    title: String,
    #[serde(rename = "type")]
    dir_type: String,
}
