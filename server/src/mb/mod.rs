use std::time::Instant;

use {
    crate::{KnowledgeBase, db, mb::ty::MinifiedRelease, table_list_kv},
    anyhow::{anyhow, bail},
    async_compression::tokio::bufread::XzDecoder,
    futures::StreamExt,
    fz::fzrank,
    redb::{Database, ReadableDatabase, ReadableTable, TableDefinition},
    serde_json,
    std::{
        path::Path,
        sync::{
            Arc, LazyLock, RwLock,
            atomic::{AtomicUsize, Ordering},
        },
    },
    tokio::io::{AsyncBufReadExt, BufReader},
    tokio_tar::Archive,
    tokio_util::io::StreamReader,
    tracing::{debug, error, info, warn},
};

pub mod ty;
static CLIENT: LazyLock<Arc<MusicBrainz>> = LazyLock::new(|| Arc::new(MusicBrainz::new()));
pub fn client() -> Arc<MusicBrainz> { CLIENT.clone() }

#[derive(Debug)]
pub struct MusicBrainz {
    latest: Arc<RwLock<[char; 16]>>,
    db: Arc<Database>,
    total: AtomicUsize,
}

impl KnowledgeBase for MusicBrainz {
    type Output = MinifiedRelease;

    async fn fetch(&self) -> anyhow::Result<()> {
        self.update_latest().await?;
        self.process_and_ingest().await?;

        Ok(())
    }

    fn search(&self, q: &str, p: usize) -> anyhow::Result<Vec<Self::Output>> {
        let tx = self.db.begin_read()?;
        let releases = tx.open_table(MusicBrainz::releases_table_def())?;

        let entries: Vec<(String, Vec<String>)> = table_list_kv(MusicBrainz::indexes_table_def(), &tx)?
            .into_iter()
            .map(|(k, v)| (k.value().clone(), v.value().clone()))
            .collect();

        let titles: Vec<String> = entries.iter().map(|(t, _)| t.clone()).collect();
        let scored = fzrank(q, &titles);

        // (score, title-length, title, id) — deterministic ordering on ties.
        let mut flat: Vec<(i32, usize, String, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for &(idx, score) in scored.iter() {
            if idx >= entries.len() {
                continue;
            }
            let (title, ids) = &entries[idx];
            for id in ids {
                if seen.insert(id.clone()) {
                    flat.push((score, title.chars().count(), title.clone(), id.clone()));
                }
            }
        }

        flat.sort_unstable_by(|a, b| {
            b.0.cmp(&a.0)
                .then(a.1.cmp(&b.1))
                .then(a.2.cmp(&b.2))
        });

        let offset = 50usize.saturating_mul(p);
        let mut out = Vec::new();

        for (_, _, _, id) in flat.into_iter().skip(offset).take(50) {
            if let Some(val) = releases.get(&id)? {
                let s = val.value();
                match serde_json::from_str::<MinifiedRelease>(&s) {
                    Ok(min) => out.push(min),
                    Err(e) => warn!(error = %e, id = %id, "failed to deserialize release json"),
                }
            }
        }

        Ok(out)
    }

    fn stats(&self) -> anyhow::Result<usize> { Ok(self.total.load(Ordering::Relaxed)) }
}

#[allow(clippy::new_without_default)]
impl MusicBrainz {
    pub fn new() -> Self {
        let i = Instant::now();
        let db = Database::create(db().join("mb.db")).expect("Failed to create MusicBrain db");
        info!("Took {}ms to open db", i.elapsed().as_millis());

        let txn = db.begin_write().unwrap();
        txn.open_table(Self::releases_table_def()).unwrap();
        txn.open_table(Self::indexes_table_def()).unwrap();
        txn.open_table(Self::checkpoint_table_def()).unwrap();
        txn.commit().unwrap();

        let total = load_mb_cursor(&db).map(|(_, lines)| lines).unwrap_or(0);
        let db = Arc::new(db);

        // Heal older databases that never indexed artist names.
        backfill_artist_index(db.clone());

        Self { latest: Arc::new(RwLock::new(['\0'; 16])), db, total: AtomicUsize::new(total) }
    }

    pub fn releases_table_def<'a>() -> TableDefinition<'a, String, String> {
        TableDefinition::<String, String>::new("releases")
    }

    pub fn indexes_table_def<'a>() -> TableDefinition<'a, String, Vec<String>> {
        TableDefinition::<String, Vec<String>>::new("indexes")
    }

    pub fn checkpoint_table_def<'a>() -> TableDefinition<'a, String, String> {
        TableDefinition::<String, String>::new("checkpoint")
    }

    #[tracing::instrument(skip(self))]
    async fn update_latest(&self) -> anyhow::Result<()> {
        let url = "https://data.metabrainz.org/pub/musicbrainz/data/json-dumps/LATEST";
        info!(%url, "Fetching latest release tag");

        let latest = reqwest::get(url).await?.error_for_status()?.text().await?;

        let current = *self.latest.read().map_err(|e| anyhow!("{e:?}"))?;
        let current = String::from_iter(&current);

        if latest == current {
            info!(latest = %latest, "Latest release tag unchanged");
            return Ok(());
        }

        info!(old_latest = %current, new_latest = %latest, "Updated latest release tag");

        let mut arr = ['\0'; 16];
        for (i, c) in latest.chars().take(16).enumerate() {
            arr[i] = c;
        }

        *self.latest.write().map_err(|e| anyhow!("{e:?}"))? = arr;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn process_and_ingest(&self) -> anyhow::Result<()> {
        let latest = self.latest.read().map(|e| String::from_iter(*e)).map_err(|e| anyhow!("{e:?}"))?;
        let cursor = load_mb_cursor(&self.db);

        let url = format!("https://data.metabrainz.org/pub/musicbrainz/data/json-dumps/{}/release.tar.xz", latest);
        info!(%url, "Streaming MusicBrainz release archive");

        let client = reqwest::Client::new();
        let res = client.get(&url).send().await?.error_for_status()?;

        let byte_stream = res.bytes_stream().map(|b| b.map_err(std::io::Error::other));
        let stream_reader = StreamReader::new(byte_stream);
        let buf = BufReader::new(stream_reader);

        let decoder = XzDecoder::new(buf);
        let mut archive = Archive::new(decoder);

        let mut entries = archive.entries()?;
        let mut processed = 0usize;
        let mut failures = 0usize;

        while let Some(entry) = entries.next().await {
            let entry = entry?;
            let entry_path = entry.path()?;

            debug!(entry_path = %entry_path.display(), "Inspecting archive entry");

            if entry_path == Path::new("mbdump/release") {
                info!(entry_path = %entry_path.display(), "Found mdump/release, starting line-by-line processing");

                let mut line_reader = BufReader::new(entry).lines();

                if let Some((ref tag, skip_lines)) = cursor
                    && tag == &latest
                    && skip_lines > 0
                {
                    info!(skip_lines, "Resuming: skipping already-processed lines");
                    let mut skipped = 0usize;
                    while line_reader.next_line().await?.is_some() {
                        skipped += 1;
                        if skipped >= skip_lines {
                            break;
                        }
                    }
                    info!(skipped, "Skip phase complete");
                }

                loop {
                    let mut batch_lines: Vec<String> = Vec::with_capacity(1000);
                    for _ in 0..1000 {
                        match line_reader.next_line().await? {
                            Some(l) => batch_lines.push(l),
                            None => break,
                        }
                    }

                    if batch_lines.is_empty() {
                        break;
                    }

                    let tx = self.db.begin_write()?;
                    let mut t_data = tx.open_table(MusicBrainz::releases_table_def())?;
                    let mut t_idx = tx.open_table(MusicBrainz::indexes_table_def())?;
                    let mut batch_count = 0usize;

                    for line in batch_lines {
                        match serde_json::from_str::<ty::Root>(&line).map(MinifiedRelease::from) {
                            Ok(it) => {
                                t_data.insert(it.id.clone(), serde_json::to_string(&it).unwrap())?;

                                index_release(&mut t_idx, &it);

                                processed += 1;
                                batch_count += 1;
                            }
                            Err(e) => {
                                failures += 1;
                                warn!(error = %e, "Failed to parse release item");
                            }
                        }
                    }

                    drop(t_data);
                    drop(t_idx);

                    tx.commit()?;

                    self.total.fetch_add(batch_count, Ordering::Relaxed);

                    save_mb_cursor(&self.db, &latest, processed);

                    info!("Processed {} items", processed);
                }

                info!(processed, failures, "Finished processing mdump/release");
                return Ok(());
            } else {
                warn!(path=%entry_path.display(), "Skipping other file in dump");
            }
        }

        error!(release = %latest, "No mdump/release entry found in archive");
        bail!("No release found in archive");
    }
}

/// Add a release to the search index under every key a user might search by:
/// the release title AND its primary artist. Artist names are the most common
/// kind of query — "remi wolf" must find releases titled "Junuro".
pub(crate) fn index_release(
    t_idx: &mut redb::Table<'_, String, Vec<String>>,
    release: &MinifiedRelease,
) {
    let mut keys: Vec<&str> = vec![release.title.as_str()];
    if !release.primary_artist.is_empty() {
        keys.push(release.primary_artist.as_str());
    }
    for key in keys {
        let mut ids = {
            let Ok(entry) = t_idx.get(key.to_string()) else { continue };
            match entry {
                Some(v) => v.value(),
                None => Vec::new(),
            }
        };
        if ids.iter().any(|id| id == &release.id) {
            continue;
        }
        ids.push(release.id.clone());
        let _ = t_idx.insert(key.to_string(), ids);
    }
}

const BACKFILL_CURSOR: &str = "artist_index_backfill";

/// Backfill artist-name index entries for releases ingested before artists
/// were indexed. Idempotent, cursor-persisted across restarts, runs on a
/// background thread so startup stays fast.
fn backfill_artist_index(db: Arc<Database>) {
    std::thread::Builder::new()
        .name("artist-backfill".into())
        .spawn(move || {
            let started = Instant::now();
            const BATCH: usize = 5_000;

            let mut last_key: Option<String> = (|| {
                let tx = db.begin_read().ok()?;
                let table = tx.open_table(MusicBrainz::checkpoint_table_def()).ok()?;
                let v = table.get(BACKFILL_CURSOR.to_string()).ok()??;
                Some(v.value().to_string())
            })();

            info!(from = ?last_key, "artist index backfill starting");
            let mut total = 0usize;

            loop {
                let Ok(tx) = db.begin_read() else { return };
                let Ok(t_data) = tx.open_table(MusicBrainz::releases_table_def()) else { return };

                let mut batch: Vec<(String, String)> = Vec::with_capacity(BATCH);
                let range = match &last_key {
                    Some(k) => t_data.range(k.clone()..),
                    None => t_data.range::<String>(..),
                };
                let Ok(iter) = range else { return };
                for row in iter.flatten() {
                    let (k, v) = row;
                    let key = k.value();
                    if Some(&key) == last_key.as_ref() {
                        continue; // resume point already processed
                    }
                    batch.push((key, v.value().to_string()));
                    if batch.len() >= BATCH {
                        break;
                    }
                }
                drop(t_data);

                if batch.is_empty() {
                    info!(total, elapsed_s = started.elapsed().as_secs(), "artist index backfill complete");
                    return;
                }

                let new_last = batch.last().map(|(k, _)| k.clone()).unwrap_or_default();

                let Ok(wtx) = db.begin_write() else { return };
                let Ok(mut t_idx) = wtx.open_table(MusicBrainz::indexes_table_def()) else { return };
                for (_, json) in &batch {
                    if let Ok(min) = serde_json::from_str::<MinifiedRelease>(json) {
                        index_release(&mut t_idx, &min);
                        total += 1;
                    }
                }
                drop(t_idx);
                if wtx.commit().is_err() {
                    error!("artist backfill commit failed");
                    return;
                }

                last_key = Some(new_last.clone());
                if let Ok(wtx) = db.begin_write() {
                    let res = (|| -> anyhow::Result<()> {
                        let mut t_cp = wtx.open_table(MusicBrainz::checkpoint_table_def())?;
                        t_cp.insert(BACKFILL_CURSOR.to_string(), new_last)?;
                        drop(t_cp);
                        wtx.commit()?;
                        Ok(())
                    })();
                    if let Err(e) = res {
                        warn!(error = %e, "backfill cursor save failed");
                    }
                }

                debug!(total, "artist backfill progress");
            }
        })
        .expect("spawn artist-backfill thread");
}

fn load_mb_cursor(db: &Database) -> Option<(String, usize)> {    let tx = db.begin_read().ok()?;
    let table = tx.open_table(MusicBrainz::checkpoint_table_def()).ok()?;
    let entry = table.get("cursor".to_string()).ok()??;
    let s = entry.value();
    let parsed = serde_json::from_str::<serde_json::Value>(&s).ok()?;
    let tag = parsed.get("tag")?.as_str()?.to_string();
    let lines = parsed.get("lines")?.as_u64()? as usize;
    Some((tag, lines))
}

fn save_mb_cursor(db: &Database, tag: &str, lines: usize) {
    let tx = db.begin_write().unwrap();
    let mut table = tx.open_table(MusicBrainz::checkpoint_table_def()).unwrap();
    let payload = serde_json::json!({ "tag": tag, "lines": lines }).to_string();
    table.insert("cursor".to_string(), payload).unwrap();
    drop(table);
    tx.commit().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minified(id: &str, title: &str, artist: &str) -> MinifiedRelease {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "title": title,
            "releaseDate": null,
            "country": null,
            "barcode": null,
            "asin": null,
            "primaryArtist": artist,
            "artistCredits": [],
            "genres": [],
            "tags": [],
            "totalDiscs": 1,
            "totalTracks": 1,
            "tracks": [],
            "hasFrontCover": false,
        }))
        .unwrap()
    }

    #[test]
    fn index_release_indexes_title_and_artist() {
        let db = Database::create(std::env::temp_dir().join("mb-test-idx.db")).unwrap();
        let tx = db.begin_write().unwrap();
        let mut t_idx = tx.open_table(MusicBrainz::indexes_table_def()).unwrap();

        index_release(&mut t_idx, &minified("id1", "Junuro", "Remi Wolf"));
        index_release(&mut t_idx, &minified("id2", "I'm Allergic to Dogs!", "Remi Wolf"));
        // Duplicate (same id under same key) must not duplicate entries.
        index_release(&mut t_idx, &minified("id2", "I'm Allergic to Dogs!", "Remi Wolf"));

        let wolf = {
            let v = t_idx.get("Remi Wolf".to_string()).unwrap().unwrap();
            v.value()
        };
        assert_eq!(wolf, vec!["id1".to_string(), "id2".to_string()]);
        let junuro = {
            let v = t_idx.get("Junuro".to_string()).unwrap().unwrap();
            v.value()
        };
        assert_eq!(junuro, vec!["id1".to_string()]);

        drop(t_idx);
        tx.commit().unwrap();
        let _ = std::fs::remove_file(std::env::temp_dir().join("mb-test-idx.db"));
    }
}
