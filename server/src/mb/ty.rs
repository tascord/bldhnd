use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinifiedRelease {
    // Core Album/Release Info
    pub id: String,
    pub title: String,
    pub release_date: Option<String>,
    pub country: Option<String>,
    pub barcode: Option<String>,
    pub asin: Option<String>,

    // Artists & Credits (Flattened string for searching, structured array for Plex metadata)
    pub primary_artist: String,
    pub artist_credits: Vec<MinifiedArtist>,

    // Search & Categorization Indexing
    pub genres: Vec<String>,
    pub tags: Vec<String>,

    // Media & Tracks (Highly minimized tree)
    pub total_discs: usize,
    pub total_tracks: usize,
    pub tracks: Vec<MinifiedTrack>,

    // Plex specific flags
    pub has_front_cover: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinifiedArtist {
    pub id: String,
    pub name: String,
    pub sort_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinifiedTrack {
    pub id: String,
    pub position: i64,      // Track number on the disc
    pub disc_position: i64, // Disc number
    pub title: String,
    pub length_ms: Option<i64>,
    pub artist: String, // Simplified display string for the track artist
}

impl From<Root> for MinifiedRelease {
    fn from(root: Root) -> Self {
        // 1. Build cohesive primary artist names and credits
        let artist_credits: Vec<MinifiedArtist> = root.artist_credit
            .iter()
            .map(|ac| MinifiedArtist {
                id: ac.artist.id.clone(),
                name: ac.artist.name.clone(),
                sort_name: ac.artist.sort_name.clone(),
            })
            .collect();

        let primary_artist = root.artist_credit
            .iter()
            .map(|ac| {
                let name = &ac.artist.name;
                let phrase = ac.joinphrase.as_deref().unwrap_or("");
                format!("{}{}", name, phrase)
            })
            .collect::<Vec<String>>()
            .join("");

        // 2. Extract Genres from both Release level and ReleaseGroup level
        let mut genres = Vec::new();
        for genre in root.release_group.genres {
            genres.push(genre.name);
        }
        // Fallback for release group tags
        let mut tags: Vec<String> = root.release_group.tags
            .iter()
            .map(|t| t.name.clone())
            .collect();

        // 3. Process Medias & Tracks down into flat items
        let total_discs = root.media.len();
        let mut total_tracks = 0;
        let mut tracks = Vec::new();

        for medium in root.media {
            total_tracks += medium.tracks.len();
            for track in medium.tracks {
                let track_artist = track.artist_credit
                    .iter()
                    .map(|ac| format!("{}{}", ac.artist.name, ac.joinphrase.as_deref().unwrap_or("")))
                    .collect::<Vec<String>>()
                    .join("");

                tracks.push(MinifiedTrack {
                    id: track.id,
                    position: track.position,
                    disc_position: medium.position,
                    title: track.title,
                    length_ms: track.length.or(track.recording.length),
                	artist: if track_artist.is_empty() { primary_artist.clone() } else { track_artist },
                });
            }
        }

        // 4. Build output struct
        MinifiedRelease {
            id: root.id,
            title: root.title,
            // Prefer the broad Release Group date, fallback to the specific release date
            release_date: root.date.or(root.release_group.first_release_date),
            country: root.country,
            barcode: root.barcode,
            asin: root.asin,
            primary_artist,
            artist_credits,
            genres,
            tags,
            total_discs,
            total_tracks,
            tracks,
            has_front_cover: root.cover_art_archive.front,
        }
    }
}