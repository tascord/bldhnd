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
pub(super) struct SparqlResponse {
    pub results: SparqlResults,
}

#[derive(Deserialize, Debug)]
pub(super) struct SparqlResults {
    pub bindings: Vec<SparqlBinding>,
}

#[derive(Deserialize, Debug)]
pub(super) struct SparqlBinding {
    pub item: SparqlValue,
    #[serde(rename = "itemLabel")]
    pub item_label: Option<SparqlValue>,
    #[serde(rename = "releaseDate")]
    pub release_date: Option<SparqlValue>,
    pub genres: Option<SparqlValue>,
    pub country: Option<SparqlValue>,
    pub formats: Option<SparqlValue>,
}

#[derive(Deserialize, Debug)]
pub(super) struct SparqlValue {
    pub value: String,
}
