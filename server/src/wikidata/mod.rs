use std::{collections::HashMap, time::Instant};

use {
    crate::{KnowledgeBase, db, table_list_kv, wikidata::ty::WikiDataItem},
    fz::fzrank,
    redb::{Database, ReadableDatabase, ReadableTable, TableDefinition},
    serde::{Deserialize, Serialize},
    std::sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
    tracing::{info, warn},
};

pub mod ty;

static CLIENT: LazyLock<Arc<WikiData>> = LazyLock::new(|| Arc::new(WikiData::new()));
pub fn client() -> Arc<WikiData> { CLIENT.clone() }

/// Number of QIDs scanned per SPARQL page.
///
/// The page query is a bare `SELECT ?item` scan — no joins, no aggregation,
/// no label service — so WDQS answers comfortably within its 60-second
/// timeout even at this size.
const BATCH_SIZE: usize = 5000;

/// Number of QIDs sent per *lookup* pass (labels / dates, then genres /
/// country / formats). `VALUES`-anchored queries are cheap; these complete in
/// a couple of seconds each.
const LOOKUP_CHUNK: usize = 250;

const SPARQL_ENDPOINT: &str = "https://query.wikidata.org/sparql";
const ENTITY_BASE: &str = "http://www.wikidata.org/entity/";

pub struct WikiData {
    db: Database,
    total: AtomicUsize,
}

impl KnowledgeBase for WikiData {
    type Output = WikiDataItem;

    async fn fetch(&self) -> anyhow::Result<()> {
        // Q11424  = film
        // Q5398426 = television series
        self.fetch_media_type("Q11424", "film").await?;
        self.fetch_media_type("Q5398426", "tv").await?;
        Ok(())
    }

    fn search(&self, q: &str, p: usize) -> anyhow::Result<Vec<Self::Output>> {
        let tx = self.db.begin_read()?;
        let items = tx.open_table(WikiData::items_table_def())?;

        let entries: Vec<(String, Vec<String>)> = table_list_kv(WikiData::indexes_table_def(), &tx)?
            .into_iter()
            .map(|(k, v)| (k.value().clone(), v.value().clone()))
            .collect();

        let titles: Vec<String> = entries.iter().map(|(t, _)| t.clone()).collect();
        let scored = fzrank(q, &titles);

        let mut flat: Vec<(i32, String)> = Vec::new();
        for &(idx, score) in scored.iter() {
            if idx >= entries.len() {
                continue;
            }
            for id in &entries[idx].1 {
                flat.push((score, id.clone()));
            }
        }

        flat.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));

        let offset = 50usize.saturating_mul(p);
        let mut out = Vec::new();

        for (_, id) in flat.into_iter().skip(offset).take(50) {
            if let Some(val) = items.get(&id)? {
                let s = val.value();
                match serde_json::from_str::<WikiDataItem>(&s) {
                    Ok(item) => out.push(item),
                    Err(e) => warn!(error = %e, %id, "failed to deserialize wikidata item"),
                }
            }
        }

        Ok(out)
    }

    fn stats(&self) -> anyhow::Result<usize> { Ok(self.total.load(Ordering::Relaxed)) }
}

#[allow(clippy::new_without_default)]
impl WikiData {
    pub fn new() -> Self {
        let i = Instant::now();
        let db = Database::create(db().join("wikidata.db")).expect("Failed to create WikiData db");
        info!("Took {}ms to open db", i.elapsed().as_millis());

        let txn = db.begin_write().unwrap();
        txn.open_table(Self::items_table_def()).unwrap();
        txn.open_table(Self::indexes_table_def()).unwrap();
        txn.open_table(Self::checkpoint_table_def()).unwrap();
        txn.commit().unwrap();

        let film = load_cursor(&db, "film");
        let tv = load_cursor(&db, "tv");

        Self { db, total: AtomicUsize::new(film.total + tv.total) }
    }

    /// Primary-key table: QID → JSON-serialised `WikiDataItem`.
    pub fn items_table_def<'a>() -> TableDefinition<'a, String, String> { TableDefinition::<String, String>::new("items") }

    /// Inverted-index table: title → Vec<QID>.
    pub fn indexes_table_def<'a>() -> TableDefinition<'a, String, Vec<String>> {
        TableDefinition::<String, Vec<String>>::new("indexes")
    }

    /// Checkpoint table: cursor key → JSON state.
    pub fn checkpoint_table_def<'a>() -> TableDefinition<'a, String, String> {
        TableDefinition::<String, String>::new("checkpoint")
    }

    /// Scrape all entities of a given WikiData instance type (`type_qid`, e.g.
    /// `"Q11424"`) and ingest them into the local redb database.
    ///
    /// Each page is three steps:
    ///   1. a bare QID scan (`FILTER(?item > wd:<last>) ORDER BY ?item`) —
    ///      deterministic pagination that never skips rows;
    ///   2. `VALUES`-anchored lookups for titles/release dates and for
    ///      genre/country/format metadata, in small chunks;
    ///   3. one redb transaction committing the whole page.
    ///
    /// Each media type keeps its own checkpoint under `cursor:{media_type}`.
    #[tracing::instrument(skip(self), fields(%type_qid, %media_type))]
    async fn fetch_media_type(&self, type_qid: &str, media_type: &str) -> anyhow::Result<()> {
        let http = reqwest::Client::builder()
            .user_agent("bh-server/0.1 (https://github.com/tascord/bldhnd; knowledge-base bot)")
            .timeout(std::time::Duration::from_secs(180))
            .build()?;

        let mut cursor = load_cursor(&self.db, media_type);

        let mut total_processed = 0usize;
        let mut total_skipped = 0usize;

        info!(last = %cursor.last, "Starting WikiData scrape");

        loop {
            let after = if cursor.last.is_empty() { None } else { Some(cursor.last.as_str()) };
            let ids_query = build_ids_query(type_qid, BATCH_SIZE, after);

            info!(last = %cursor.last, "Querying WikiData SPARQL endpoint");
            let ids_resp: ty::SparqlResponse<ty::IdBinding> = sparql_get(&http, &ids_query).await?;

            let qids: Vec<String> =
                ids_resp.results.bindings.iter().map(|b| b.id()).collect();
            if qids.is_empty() {
                break;
            }

            // ── Lookup pass 1: English labels + release dates ─────────────
            let mut labelled: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
            for chunk in qids.chunks(LOOKUP_CHUNK) {
                let query = build_labels_query(chunk);
                match sparql_get::<ty::SparqlBinding>(&http, &query).await {
                    Ok(r) => {
                        for b in r.results.bindings {
                            let title = b.item_label.clone().and_then(|v| v.filter_not_empty());
                            let date = b.release_date.clone().and_then(|v| v.filter_not_empty())
                                .map(|v| v.split('T').next().unwrap_or("").to_string())
                                .filter(|d| !d.is_empty());
                            labelled.insert(b.id(), (title, date));
                        }
                    }
                    Err(e) => warn!(count = chunk.len(), error = %e, "Label lookup failed"),
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }

            // ── Lookup pass 2: genres / country / formats ─────────────────
            let mut meta: HashMap<String, ItemMeta> = HashMap::new();
            for chunk in qids.chunks(LOOKUP_CHUNK) {
                let enrich_query = build_enrich_query(chunk);
                match sparql_get::<ty::EnrichBinding>(&http, &enrich_query).await {
                    Ok(er) => {
                        for eb in er.results.bindings {
                            let entry = meta.entry(eb.id()).or_default();
                            if let Some(g) = eb.genre_label.and_then(ty::SparqlValue::filter_not_empty) {
                                if !entry.genres.contains(&g) {
                                    entry.genres.push(g);
                                }
                            }
                            if let Some(c) = eb.country_label.and_then(ty::SparqlValue::filter_not_empty) {
                                entry.country.get_or_insert(c);
                            }
                            if let Some(f) = eb.format_label.and_then(ty::SparqlValue::filter_not_empty) {
                                if !entry.formats.contains(&f) {
                                    entry.formats.push(f);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(count = chunk.len(), error = %e, "Enrichment pass failed, storing without metadata");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }

            // ── Write page into redb ──────────────────────────────────────
            let mut inserted = 0usize;

            let tx = self.db.begin_write()?;
            {
                let mut t_items = tx.open_table(WikiData::items_table_def())?;
                let mut t_idx = tx.open_table(WikiData::indexes_table_def())?;

                for id in &qids {
                    let Some((Some(title), release_date)) = labelled.remove(id) else {
                        // No usable English label – skip; a QID-only title is not useful
                        total_skipped += 1;
                        continue;
                    };

                    let m = meta.remove(id).unwrap_or_default();

                    let item = WikiDataItem {
                        id: id.clone(),
                        title: title.clone(),
                        media_type: media_type.to_string(),
                        release_date,
                        genres: m.genres,
                        country: m.country,
                        formats: m.formats,
                    };

                    t_items.insert(id.clone(), serde_json::to_string(&item)?)?;

                    // Update inverted index (title → [id, …])
                    let mut ids = t_idx.get(&title).ok().flatten().map(|v| v.value()).unwrap_or_default();
                    ids.push(id.clone());
                    t_idx.insert(title, ids)?;

                    inserted += 1;
                }
            }
            tx.commit()?;

            // Advance the pagination cursor past everything scanned on this
            // page, including items we chose not to store.
            cursor.last = qids.last().cloned().unwrap_or(cursor.last);
            cursor.total += inserted;
            save_cursor(&self.db, media_type, &cursor);
            self.total.fetch_add(inserted, Ordering::Relaxed);

            total_processed += inserted;
            info!(total_processed, total_skipped, ingested = inserted, last = %cursor.last, "Committed WikiData page");

            if qids.len() < BATCH_SIZE {
                // Last (partial) page – done.
                break;
            }

            // Brief pause to be polite to WikiData's public endpoint.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        info!(total_processed, total_skipped, "Finished WikiData scrape");
        Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
struct ItemMeta {
    genres: Vec<String>,
    country: Option<String>,
    formats: Vec<String>,
}

/// Resume state for one media type's scrape. Pagination is keyed on the last
/// ingested QID rather than a row offset.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Cursor {
    #[serde(default)]
    last: String,
    #[serde(default)]
    total: usize,
}

fn cursor_key(media_type: &str) -> String { format!("cursor:{media_type}") }

fn load_cursor(db: &Database, media_type: &str) -> Cursor {
    let tx = db.begin_read().unwrap();
    let table = tx.open_table(WikiData::checkpoint_table_def()).unwrap();
    table
        .get(cursor_key(media_type))
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str::<Cursor>(&v.value()).ok())
        .unwrap_or_default()
}

fn save_cursor(db: &Database, media_type: &str, cursor: &Cursor) {
    let tx = db.begin_write().unwrap();
    let mut table = tx.open_table(WikiData::checkpoint_table_def()).unwrap();
    let payload = serde_json::to_string(cursor).unwrap();
    table.insert(cursor_key(media_type), payload).unwrap();
    drop(table);
    tx.commit().unwrap();
}

/// GET the SPARQL endpoint with retries on transport errors, HTTP 429 and 5xx
/// (WDQS regularly answers 504 when a query is slow — that must not abort an
/// hours-long scrape).
async fn sparql_get<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    query: &str,
) -> anyhow::Result<ty::SparqlResponse<T>> {
    const MAX_BACKOFF_MS: u64 = 60_000;
    let mut backoff_ms = 1000u64;

    loop {
        match http
            .get(SPARQL_ENDPOINT)
            .query(&[("query", query), ("format", "json")])
            .header(reqwest::header::ACCEPT, "application/sparql-results+json")
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                    if backoff_ms > MAX_BACKOFF_MS {
                        anyhow::bail!("WikiData endpoint kept failing ({status})");
                    }
                    warn!(%status, backoff_ms, "WikiData request failed, retrying");
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms *= 2;
                    continue;
                }
                return Ok(resp.error_for_status()?.json::<ty::SparqlResponse<T>>().await?);
            }
            Err(e) => {
                if backoff_ms > MAX_BACKOFF_MS {
                    return Err(e.into());
                }
                warn!(error = %e, backoff_ms, "WikiData transport error, retrying");
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                backoff_ms *= 2;
            }
        }
    }
}

/// One page of bare QIDs of the given instance type, strictly after
/// `after_qid`. Deterministic thanks to `ORDER BY ?item`, and cheap because
/// there are no joins or label resolution involved.
fn build_ids_query(type_qid: &str, limit: usize, after_qid: Option<&str>) -> String {
    let after = after_qid.map(|q| format!("FILTER(STR(?item) > \"{ENTITY_BASE}{q}\").")).unwrap_or_default();
    format!(
        r#"SELECT ?item
WHERE {{
  ?item wdt:P31 wd:{type_qid}.
  {after}
}}
ORDER BY ?item
LIMIT {limit}"#
    )
}

/// Titles + release dates for a chunk of QIDs via a `VALUES` join.
fn build_labels_query(qids: &[String]) -> String {
    let values = qids.iter().map(|q| format!("wd:{q}")).collect::<Vec<_>>().join(" ");
    format!(
        r#"SELECT ?item ?itemLabel ?releaseDate
WHERE {{
  VALUES ?item {{ {values} }}
  OPTIONAL {{ ?item wdt:P577 ?releaseDate. }}
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "en". }}
}}"#
    )
}

/// Genre / country / distribution-format metadata for a chunk of QIDs. Returns
/// one row per (item, genre | country | format) combination, merged client-side.
fn build_enrich_query(qids: &[String]) -> String {
    let values = qids.iter().map(|q| format!("wd:{q}")).collect::<Vec<_>>().join(" ");
    format!(
        r#"SELECT ?item ?genreLabel ?countryLabel ?formatLabel
WHERE {{
  VALUES ?item {{ {values} }}
  OPTIONAL {{ ?item wdt:P136 ?genre. ?genre rdfs:label ?genreLabel. FILTER(LANG(?genreLabel) = "en") }}
  OPTIONAL {{ ?item wdt:P495 ?country. ?country rdfs:label ?countryLabel. FILTER(LANG(?countryLabel) = "en") }}
  OPTIONAL {{ ?item wdt:P437 ?format. ?format rdfs:label ?formatLabel. FILTER(LANG(?formatLabel) = "en") }}
}}"#
    )
}
