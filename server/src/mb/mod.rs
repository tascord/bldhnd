use {
    crate::{KnowledgeBase, db, mb::ty::MinifiedRelease},
    anyhow::{anyhow, bail},
    async_compression::tokio::bufread::XzDecoder,
    futures::StreamExt,
    redb::{Database, ReadableDatabase, ReadableTable, TableDefinition},
    serde_json,
    std::{
        path::Path,
        sync::{Arc, LazyLock, RwLock},
    },
    tokio::io::{AsyncBufReadExt, BufReader},
    tokio_tar::Archive,
    tokio_util::io::StreamReader,
    tracing::{debug, error, info, warn},
};

pub mod ty;
static CLIENT: LazyLock<Arc<MusicBrainz>> = LazyLock::new(|| Arc::new(MusicBrainz::new()));
pub fn client() -> Arc<MusicBrainz> {
    CLIENT.clone()
}

#[derive(Debug)]
pub struct MusicBrainz {
    latest: Arc<RwLock<[char; 16]>>,
    db: Database,
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
        let idx = tx.open_table(MusicBrainz::indexes_table_def())?;

        fn score_candidate(title: &str, q: &str) -> Option<i64> {
            if q.is_empty() {
                return Some(0);
            }
            let t = title.to_lowercase();
            let q = q.to_lowercase();
            if let Some(pos) = t.find(&q) {
                return Some((1000i64 - pos as i64).max(1));
            }

            let t_chars: Vec<char> = t.chars().collect();
            let q_chars: Vec<char> = q.chars().collect();

            let mut qi = 0usize;
            let mut first = None;
            let mut last = 0usize;

            for (i, &c) in t_chars.iter().enumerate() {
                if qi < q_chars.len() && c == q_chars[qi] {
                    if first.is_none() {
                        first = Some(i);
                    }
                    last = i;
                    qi += 1;
                }
            }

            if qi == q_chars.len() {
                let first = first.unwrap_or(0);
                let span = (last - first) as i64 + 1;
                let score = 500i64 + (qi as i64 * 10) - span;
                return Some(score.max(1));
            }

            None
        }

        match (idx.first()?, idx.last()?) {
            (Some((lk, _)), Some((rk, _))) => {
                let lkey = lk.value().clone();
                let rkey = rk.value().clone();
                let range = idx.range(lkey..=rkey)?;
                let mut scored: Vec<(i64, String)> = Vec::new();

                let releases = tx.open_table(MusicBrainz::releases_table_def())?;

                for item in range {
                    let (k, v) = item?;
                    let title = k.value();
                    let ids = v.value();
                    if let Some(s) = score_candidate(&title, q) {
                        for id in ids {
                            scored.push((s, id));
                        }
                    }
                }

                scored.sort_by_key(|b| std::cmp::Reverse(b.0));

                let offset = 50usize.saturating_mul(p);
                let mut out = Vec::new();

                for (_score, id) in scored.into_iter().skip(offset).take(50) {
                    if let Some(val) = releases.get(&id)? {
                        let s = val.value();
                        match serde_json::from_str::<MinifiedRelease>(&s) {
                            Ok(min) => out.push(min),
                            Err(e) => {
                                warn!(error = %e, id = %id, "failed to deserialize release json");
                                continue;
                            }
                        }
                    }
                }

                Ok(out)
            }
            _ => Ok(Vec::new()),
        }
    }

    fn stats(&self) -> anyhow::Result<usize> {
        let idx = self.db.begin_read()?.open_table(MusicBrainz::indexes_table_def())?;
        match (idx.first()?, idx.last()?) {
            (Some((lk, _)), Some((rk, _))) => {
                let lkey = lk.value().clone();
                let rkey = rk.value().clone();
                Ok(idx.range(lkey..=rkey)?.count())
            }
            _ => Ok(0),
        }
    }
}

#[allow(clippy::new_without_default)]
impl MusicBrainz {
    pub fn new() -> Self {
        let mut db = Database::create(db().join("mb.db")).expect("Failed to create MusicBrain db");
        db.compact().expect("Failed to compact mb db");

        let txn = db.begin_write().unwrap();
        txn.open_table(Self::releases_table_def()).unwrap();
        txn.open_table(Self::indexes_table_def()).unwrap();
        txn.open_table(Self::checkpoint_table_def()).unwrap();
        txn.commit().unwrap();

        Self { latest: Arc::new(RwLock::new(['\0'; 16])), db }
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

                    for line in batch_lines {
                        match serde_json::from_str::<ty::Root>(&line).map(MinifiedRelease::from) {
                            Ok(it) => {
                                t_data.insert(it.id.clone(), serde_json::to_string(&it).unwrap())?;

                                t_idx.insert(it.title.clone(), {
                                    let mut v = t_idx.get(it.title).ok().flatten().map(|v| v.value()).unwrap_or_default();
                                    v.push(it.id);
                                    v
                                })?;

                                processed += 1;
                            }
                            Err(e) => {
                                failures += 1;
                                warn!(error = %e, line = %line, "Failed to parse release item");
                            }
                        }
                    }

                    drop(t_data);
                    drop(t_idx);

                    tx.commit()?;

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

fn load_mb_cursor(db: &Database) -> Option<(String, usize)> {
    let tx = db.begin_read().ok()?;
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
