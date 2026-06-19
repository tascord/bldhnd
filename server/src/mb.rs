use std::sync::{LazyLock, OnceLock};

use {
    crate::{KnowledgeBase, db, working},
    anyhow::{anyhow, bail},
    futures::StreamExt,
    redb::{Database, ReadableTable, TableDefinition},
    serde::{Deserialize, Serialize},
    std::sync::{Arc, RwLock},
    tokio::{
        fs::OpenOptions,
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        process::Command,
    },
    tracing::{error, info, warn},
};

static CLIENT: LazyLock<Arc<MusicBrainz>> = LazyLock::new(|| Arc::new(MusicBrainz::new()));
pub fn client() -> Arc<MusicBrainz> {
    CLIENT.clone()
}

pub struct MusicBrainz {
    latest: Arc<RwLock<[char; 16]>>,
    db: Database,
}

pub struct Release {}

impl KnowledgeBase for MusicBrainz {
    async fn fetch(&self) -> anyhow::Result<()> {
        self.update_latest().await?;
        self.fetch_release().await?;
        self.ingest().await?;

        Ok(())
    }
}

impl MusicBrainz {
    pub fn new() -> Self {
        Self {
            latest: Arc::new(RwLock::new(['\0'; 16])),
            db: Database::create(db().join("mb.db")).expect("Failed to create MusicBrain db"),
        }
    }

    async fn update_latest(&self) -> anyhow::Result<()> {
        let latest = reqwest::get("https://data.metabrainz.org/pub/musicbrainz/data/json-dumps/")
            .await?
            .error_for_status()?
            .text()
            .await?;

        if latest.chars().zip(&self.latest.read().map_err(|e| anyhow!("{e:?}")).map(|v| *v)?).all(|(a, b)| a == *b) {
            return Ok(());
        }

        let mut arr = ['\0'; 16];
        for (i, c) in latest.chars().take(16).enumerate() {
            arr[i] = c;
        }

        *self.latest.write().map_err(|e| anyhow!("{e:?}"))? = arr;

        Ok(())
    }

    async fn fetch_release(&self) -> anyhow::Result<()> {
        let latest = self.latest.read().map(|e| String::from_iter(*e)).map_err(|e| anyhow!("{e:?}"))?;
        let path = working().join(format!("mb_release_{}.tar.xz", latest));

        if path.exists() {
            warn!("Release file {} already exists in cache", path.display());
            return Ok(());
        }

        let mut file = {
            let mut o = OpenOptions::new();
            o.create_new(true).write(true).open(&path).await.unwrap()
        };

        info!("Fetching mb json dump");
        let mut stream =
            reqwest::get(format!("https://data.metabrainz.org/pub/musicbrainz/data/json-dumps/{}/release.tar.xz", latest))
                .await?
                .error_for_status()?
                .bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }

        info!("mb json dump downloaded");
        file.flush().await?;

        if !Command::new("which").arg("tar").status().await?.success() {
            error!("No 'tar' binary available. Can't unzip");
            bail!("No tar binary found");
        }

        if !Command::new("tar")
            .arg("-xvf")
            .arg(path)
            .arg("mdump/release")
            .current_dir(working())
            .spawn()?
            .wait()
            .await?
            .success()
        {
            error!("Failed to unzip mb dump");
            bail!("Failed to unzip");
        }

        info!("mb json dump extracted");

        Ok(())
    }

    async fn ingest(&self) -> anyhow::Result<()> {
        let file = OpenOptions::new().read(true).open(working().join("release")).await?;
        let mut lines = BufReader::new(file).lines();

        let tx = self.db.begin_write()?;
        let mut t_data = tx.open_table(TableDefinition::<String, String>::new("releases"))?;
        let mut t_idx = tx.open_table(TableDefinition::<String, Vec<String>>::new("indexes"))?;

        while let Some(line) = lines.next_line().await? {
            match serde_json::from_str::<Root>(&line) {
                Ok(it) => {
                    t_data.insert(it.id.clone(), line)?;
                    t_idx.insert(it.title.clone(), {
                        let mut v = t_idx.get(it.title).ok().flatten().map(|v| v.value()).unwrap_or_default();
                        v.push(it.id);
                        v
                    })?;
                }
                Err(e) => {
                    warn!("Failed to parse release item: {e:?}");
                }
            }
        }

        // tx.commit()?;
        Ok(())
    }
}

//

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub tags: Vec<serde_json::Value>,
    #[serde(rename = "release-group")]
    pub release_group: ReleaseGroup,
    pub relations: Vec<Relation>,
    #[serde(rename = "status-id")]
    pub status_id: Option<String>,
    pub media: Vec<Medium>,
    #[serde(rename = "text-representation")]
    pub text_representation: TextRepresentation,
    pub barcode: Option<String>,
    pub title: String,
    pub disambiguation: Option<String>,
    pub packaging: serde_json::Value,
    #[serde(rename = "cover-art-archive")]
    pub cover_art_archive: CoverArtArchive,
    pub asin: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "release-events")]
    pub release_events: Vec<Event>,
    pub id: String,
    pub genres: Vec<serde_json::Value>,
    pub annotation: serde_json::Value,
    pub aliases: Vec<Alias>,
    pub date: Option<String>,
    pub country: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    #[serde(rename = "packaging-id")]
    pub packaging_id: serde_json::Value,
    pub quality: Option<String>,
    #[serde(rename = "label-info")]
    pub label_info: Vec<LabelInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseGroup {
    pub genres: Vec<Genre>,
    #[serde(rename = "primary-type")]
    pub primary_type: Option<String>,
    pub aliases: Vec<Alias>,
    pub tags: Vec<Tag>,
    pub id: String,
    #[serde(rename = "secondary-type-ids")]
    pub secondary_type_ids: Vec<serde_json::Value>,
    pub title: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    #[serde(rename = "secondary-types")]
    pub secondary_types: Vec<serde_json::Value>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    #[serde(rename = "primary-type-id")]
    pub primary_type_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub name: String,
    pub id: String,
    pub disambiguation: Option<String>,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub count: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtistCredit {
    pub joinphrase: Option<String>,
    pub name: String,
    pub artist: Artist,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
    #[serde(rename = "type")]
    pub artist_type: Option<String>, // 'type' is a reserved keyword in Rust
    pub aliases: Option<Vec<Alias>>,
    pub name: String,
    pub country: Option<serde_json::Value>,
    pub disambiguation: Option<String>,
    pub tags: Option<Vec<Tag>>,
    pub genres: Option<Vec<serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Alias {
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    pub ended: bool,
    #[serde(rename = "type-id")]
    pub type_id: serde_json::Value,
    pub end: serde_json::Value,
    pub begin: serde_json::Value,
    #[serde(rename = "type")]
    pub alias_type: serde_json::Value,
    pub name: String,
    pub locale: serde_json::Value,
    pub primary: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    pub begin: serde_json::Value,
    pub ended: bool,
    pub url: Option<Url>,
    pub direction: String,
    #[serde(rename = "source-credit")]
    pub source_credit: Option<String>,
    pub attributes: Vec<serde_json::Value>,
    #[serde(rename = "type-id")]
    pub type_id: String,
    #[serde(rename = "target-credit")]
    pub target_credit: Option<String>,
    pub end: serde_json::Value,
    #[serde(rename = "type")]
    pub relation_type: String,
    #[serde(rename = "attribute-values")]
    pub attribute_values: serde_json::Value,
    #[serde(rename = "attribute-ids")]
    pub attribute_ids: serde_json::Value,
    #[serde(rename = "target-type")]
    pub target_type: String,
    pub artist: Option<Artist>,
    pub work: Option<Work>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Url {
    pub id: String,
    pub resource: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Medium {
    pub id: String,
    pub format: Option<String>,
    pub discs: Vec<Disc>,
    pub position: i64,
    #[serde(rename = "format-id")]
    pub format_id: Option<String>,
    #[serde(rename = "track-offset")]
    pub track_offset: i64,
    pub tracks: Vec<Track>,
    #[serde(rename = "track-count")]
    pub track_count: i64,
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Disc {
    pub offsets: Vec<i64>,
    pub id: String,
    #[serde(rename = "offset-count")]
    pub offset_count: i64,
    pub sectors: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub recording: Recording,
    pub position: i64,
    pub length: Option<i64>,
    pub number: String,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub tags: Vec<serde_json::Value>,
    pub relations: Vec<Relation>,
    pub length: Option<i64>,
    pub video: bool,
    pub disambiguation: Option<String>,
    pub title: String,
    pub id: String,
    pub isrcs: Option<Vec<String>>,
    pub aliases: Vec<serde_json::Value>,
    pub genres: Vec<serde_json::Value>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub iswcs: Vec<serde_json::Value>,
    pub id: String,
    pub language: Option<String>,
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
    #[serde(rename = "type")]
    pub work_type: Option<String>,
    pub languages: Vec<String>,
    pub attributes: Vec<serde_json::Value>,
    pub disambiguation: Option<String>,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextRepresentation {
    pub language: Option<String>,
    pub script: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtArchive {
    pub darkened: bool,
    pub artwork: bool,
    pub back: bool,
    pub front: bool,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub area: Area,
    pub date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Area {
    pub name: String,
    pub disambiguation: Option<String>,
    pub id: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    #[serde(rename = "type")]
    pub area_type: serde_json::Value,
    #[serde(rename = "type-id")]
    pub type_id: serde_json::Value,
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LabelInfo {
    pub label: Option<Label>,
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    #[serde(rename = "label-code")]
    pub label_code: Option<i64>,
    pub disambiguation: Option<String>,
    pub name: String,
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
    pub genres: Vec<serde_json::Value>,
    pub aliases: Vec<Alias>,
    #[serde(rename = "type")]
    pub label_type: Option<String>,
    pub tags: Vec<serde_json::Value>,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    pub id: String,
}
