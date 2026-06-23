use {
    crate::{KnowledgeBase, db, table_list_kv, wikidata::ty::WikiDataItem},
    fz::fzrank,
    redb::{Database, ReadableDatabase, ReadableTable, TableDefinition},
    std::sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
    tracing::{info, warn},
};

pub mod ty;

static CLIENT: LazyLock<Arc<WikiData>> = LazyLock::new(|| Arc::new(WikiData::new()));
pub fn client() -> Arc<WikiData> { CLIENT.clone() }

/// Number of items requested per SPARQL page.
///
/// WikiData's public endpoint (Blazegraph) handles GROUP BY aggregation over
/// multi-valued properties (genres, formats) comfortably at this size while
/// staying well within the 60-second query timeout.  Adjust down if you
/// consistently see timeout errors, or up (to ~10 000) on a dedicated mirror.
const BATCH_SIZE: usize = 1000;
const SPARQL_ENDPOINT: &str = "https://query.wikidata.org/sparql";

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

        flat.sort_unstable_by(|a, b| b.0.cmp(&a.0));

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
        let db_path = db().join("wikidata.db");
        let mut db = Database::create(&db_path).unwrap_or_else(|e| {
            panic!("Failed to create WikiData db at {}: {e} (check disk space and permissions)", db_path.display())
        });
        db.compact().expect("Failed to compact WikiData db");

        let txn = db.begin_write().unwrap();
        txn.open_table(Self::items_table_def()).unwrap();
        txn.open_table(Self::indexes_table_def()).unwrap();
        txn.open_table(Self::checkpoint_table_def()).unwrap();
        txn.commit().unwrap();

        let (_, total) = load_wikidata_offset(&db);

        Self { db, total: AtomicUsize::new(total) }
    }

    /// Primary-key table: QID → JSON-serialised `WikiDataItem`.
    pub fn items_table_def<'a>() -> TableDefinition<'a, String, String> { TableDefinition::<String, String>::new("items") }

    /// Inverted-index table: lowercase title → Vec<QID>.
    pub fn indexes_table_def<'a>() -> TableDefinition<'a, String, Vec<String>> {
        TableDefinition::<String, Vec<String>>::new("indexes")
    }

    /// Checkpoint table: cursor key → JSON state.
    pub fn checkpoint_table_def<'a>() -> TableDefinition<'a, String, String> {
        TableDefinition::<String, String>::new("checkpoint")
    }

    /// Scrape all entities of a given WikiData instance type (`type_qid`, e.g.
    /// `"Q11424"`) in paginated SPARQL batches, then ingest them into the local
    /// redb database.
    #[tracing::instrument(skip(self), fields(%type_qid, %media_type))]
    async fn fetch_media_type(&self, type_qid: &str, media_type: &str) -> anyhow::Result<()> {
        let http = reqwest::Client::builder()
            .user_agent("bh-server/0.1 (https://github.com/tascord/bldhnd; knowledge-base bot)")
            .timeout(std::time::Duration::from_secs(180))
            .build()?;

        let (mut offset, _) = load_wikidata_offset(&self.db);

        let mut total_processed = 0usize;
        let mut total_skipped = 0usize;
        let mut backoff_ms = 1000u64;

        info!("Starting WikiData scrape");

        loop {
            let query = build_sparql_query(type_qid, BATCH_SIZE, offset);

            save_wikidata_offset(&self.db, offset, self.total.load(Ordering::Relaxed));
            info!(%offset, "Querying WikiData SPARQL endpoint");

            let resp = loop {
                match http
                    .get(SPARQL_ENDPOINT)
                    .query(&[("query", query.as_str()), ("format", "json")])
                    .header(reqwest::header::ACCEPT, "application/sparql-results+json")
                    .send()
                    .await
                {
                    Ok(resp) => break resp,
                    Err(e) if backoff_ms > 60_000 => {
                        warn!(error = %e, "WikiData request failed permanently");
                        return Err(e.into());
                    }
                    Err(e) => {
                        warn!(error = %e, backoff_ms, "WikiData request failed, retrying");
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                        backoff_ms *= 2;
                    }
                }
            };
            backoff_ms = backoff_ms.saturating_div(2).max(1000);

            let resp = resp.error_for_status()?.json::<ty::SparqlResponse>().await?;

            let bindings = resp.results.bindings;
            let batch_len = bindings.len();

            if batch_len == 0 {
                break;
            }

            // ── Write batch into redb ─────────────────────────────────────
            let tx = self.db.begin_write()?;
            {
                let mut t_items = tx.open_table(WikiData::items_table_def())?;
                let mut t_idx = tx.open_table(WikiData::indexes_table_def())?;

                for binding in bindings {
                    // Extract QID from the full URI, e.g.
                    // "http://www.wikidata.org/entity/Q134773" → "Q134773"
                    let uri = &binding.item.value;
                    let id = uri.rsplit('/').next().unwrap_or(uri).to_string();

                    let title = match binding.item_label {
                        Some(ref lbl) if !lbl.value.is_empty() && lbl.value != id => lbl.value.clone(),
                        _ => {
                            // No English label – skip; a QID-only title is not useful
                            total_skipped += 1;
                            continue;
                        }
                    };

                    let release_date = binding.release_date.and_then(|v| {
                        // XSD datetime "1999-03-31T00:00:00Z" → "1999-03-31"
                        let d = v.value.split('T').next().unwrap_or("").to_string();
                        if d.is_empty() { None } else { Some(d) }
                    });

                    let genres = split_pipe(binding.genres);
                    let country = binding.country.map(|v| v.value).filter(|s| !s.is_empty());
                    let formats = split_pipe(binding.formats);

                    let item = WikiDataItem {
                        id: id.clone(),
                        title: title.clone(),
                        media_type: media_type.to_string(),
                        release_date,
                        genres,
                        country,
                        formats,
                    };

                    t_items.insert(id.clone(), serde_json::to_string(&item)?)?;

                    // Update inverted index (title → [id, …])
                    let mut ids = t_idx.get(&title).ok().flatten().map(|v| v.value()).unwrap_or_default();
                    ids.push(id);
                    t_idx.insert(title, ids)?;

                    total_processed += 1;
                }
            }
            tx.commit()?;

            self.total.fetch_add(batch_len, Ordering::Relaxed);
            save_wikidata_offset(&self.db, offset, self.total.load(Ordering::Relaxed));

            info!(total_processed, total_skipped, %offset, "Committed WikiData batch");

            if batch_len < BATCH_SIZE {
                // Last (partial) page – done.
                break;
            }

            offset += BATCH_SIZE;

            // Brief pause to be polite to WikiData's public endpoint.
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        info!(total_processed, total_skipped, "Finished WikiData scrape");
        Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn load_wikidata_offset(db: &Database) -> (usize, usize) {
    let tx = db.begin_read().unwrap();
    let table = tx.open_table(WikiData::checkpoint_table_def()).unwrap();
    table
        .get("cursor".to_string())
        .ok()
        .flatten()
        .and_then(|v| {
            let s = v.value();
            let parsed = serde_json::from_str::<serde_json::Value>(&s).ok()?;
            let offset = parsed.get("offset").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(0);
            let total = parsed.get("total").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(offset);
            Some((offset, total))
        })
        .unwrap_or((0, 0))
}

fn save_wikidata_offset(db: &Database, offset: usize, total: usize) {
    let tx = db.begin_write().unwrap();
    let mut table = tx.open_table(WikiData::checkpoint_table_def()).unwrap();
    let payload = serde_json::json!({ "offset": offset, "total": total }).to_string();
    table.insert("cursor".to_string(), payload).unwrap();
    drop(table);
    tx.commit().unwrap();
}

/// Split a `"|"`-delimited GROUP_CONCAT value into a filtered Vec<String>.
fn split_pipe(v: Option<ty::SparqlValue>) -> Vec<String> {
    v.map(|sv| sv.value.split('|').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect())
        .unwrap_or_default()
}

/// Build a SPARQL SELECT query that fetches one page of `media_type` items.
///
/// Uses `GROUP_CONCAT` so that multi-valued properties (genre, format) are
/// collapsed into one row per item, separated by `|`.
fn build_sparql_query(type_qid: &str, limit: usize, offset: usize) -> String {
    format!(
        r#"
SELECT ?item ?itemLabel
       (SAMPLE(STR(?releaseDate)) AS ?releaseDate)
       (GROUP_CONCAT(DISTINCT ?genreLabel; SEPARATOR="|") AS ?genres)
       (SAMPLE(?countryLabel) AS ?country)
       (GROUP_CONCAT(DISTINCT ?formatLabel; SEPARATOR="|") AS ?formats)
WHERE {{
  ?item wdt:P31 wd:{type_qid}.
  OPTIONAL {{ ?item wdt:P577 ?releaseDate. }}
  OPTIONAL {{
    ?item wdt:P136 ?genre.
    ?genre rdfs:label ?genreLabel.
    FILTER(LANG(?genreLabel) = "en")
  }}
  OPTIONAL {{
    ?item wdt:P495 ?country.
    ?country rdfs:label ?countryLabel.
    FILTER(LANG(?countryLabel) = "en")
  }}
  OPTIONAL {{
    ?item wdt:P437 ?format.
    ?format rdfs:label ?formatLabel.
    FILTER(LANG(?formatLabel) = "en")
  }}
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "en". }}
}}
GROUP BY ?item ?itemLabel
ORDER BY ?item
LIMIT {limit} OFFSET {offset}
"#
    )
}
