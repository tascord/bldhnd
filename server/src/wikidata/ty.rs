use serde::{Deserialize, Serialize};

/// A minified Movies/TV entry stored in the knowledge-base.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WikiDataItem {
    /// WikiData QID (e.g. "Q134773")
    pub id: String,
    /// English label / title
    pub title: String,
    /// "film" | "tv"
    pub media_type: String,
    /// ISO-8601 date of earliest known release (e.g. "1999-03-31")
    pub release_date: Option<String>,
    /// Genre labels in English
    pub genres: Vec<String>,
    /// Country of origin label in English
    pub country: Option<String>,
    /// Distribution format labels (e.g. "DVD", "Blu-ray Disc", "VHS")
    pub formats: Vec<String>,
}

// ── SPARQL response wire types ──────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub(super) struct SparqlResponse<T> {
    pub results: SparqlResults<T>,
}

#[derive(Deserialize, Debug)]
pub(super) struct SparqlResults<T> {
    pub bindings: Vec<T>,
}

/// One row of the bare QID scan page.
#[derive(Deserialize, Debug)]
pub(super) struct IdBinding {
    pub item: SparqlValue,
}

impl IdBinding {
    pub fn id(&self) -> String { self.item.value.rsplit('/').next().unwrap_or(&self.item.value).to_string() }
}

#[derive(Deserialize, Debug)]
pub(super) struct SparqlBinding {
    pub item: SparqlValue,
    #[serde(rename = "itemLabel")]
    pub item_label: Option<SparqlValue>,
    #[serde(rename = "releaseDate")]
    pub release_date: Option<SparqlValue>,
}

impl SparqlBinding {
    pub fn id(&self) -> String { self.item.value.rsplit('/').next().unwrap_or(&self.item.value).to_string() }
}

/// One row of the second-pass (enrichment) query. A single item can produce
/// many rows — one per genre / country / format combination.
#[derive(Deserialize, Debug)]
pub(super) struct EnrichBinding {
    pub item: SparqlValue,
    #[serde(rename = "genreLabel")]
    pub genre_label: Option<SparqlValue>,
    #[serde(rename = "countryLabel")]
    pub country_label: Option<SparqlValue>,
    #[serde(rename = "formatLabel")]
    pub format_label: Option<SparqlValue>,
}

impl EnrichBinding {
    pub fn id(&self) -> String { self.item.value.rsplit('/').next().unwrap_or(&self.item.value).to_string() }
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct SparqlValue {
    pub value: String,
}

impl SparqlValue {
    pub fn filter_not_empty(self) -> Option<String> { if self.value.is_empty() { None } else { Some(self.value) } }
}
