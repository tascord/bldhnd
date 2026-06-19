use {
    crate::{KnowledgeBase, db, mb::ty::MinifiedRelease, working},
    anyhow::{anyhow, bail},
    async_compression::tokio::bufread::GzipDecoder,
    futures::StreamExt,
    redb::{Database, ReadableTable, TableDefinition},
    std::{
        path::Path,
        sync::{Arc, LazyLock, RwLock},
    },
    tokio::{
        fs::{File, OpenOptions},
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    },
    tokio_tar::Archive,
    tracing::{debug, error, info, warn},
};

pub mod ty;
static CLIENT: LazyLock<Arc<MusicBrainz>> = LazyLock::new(|| Arc::new(MusicBrainz::new()));
pub fn client() -> Arc<MusicBrainz> { CLIENT.clone() }

#[derive(Debug)]
pub struct MusicBrainz {
    latest: Arc<RwLock<[char; 16]>>,
    db: Database,
}

impl KnowledgeBase for MusicBrainz {
    async fn fetch(&self) -> anyhow::Result<()> {
        self.update_latest().await?;
        self.fetch_release().await?;
        self.process_and_ingest().await?;

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
    async fn fetch_release(&self) -> anyhow::Result<()> {
        let latest = self.latest.read().map(|e| String::from_iter(*e)).map_err(|e| anyhow!("{e:?}"))?;
        let path = working().join(format!("mb_release_{}.tar.xz", latest));

        info!(latest = %latest, path = %path.display(), "Checking for cached release archive");

        if path.exists() {
            info!(path = %path.display(), "Release archive already cached");
            return Ok(());
        }

        let mut file = OpenOptions::new().create_new(true).write(true).open(&path).await?;

        let url = format!("https://data.metabrainz.org/pub/musicbrainz/data/json-dumps/{}/release.tar.xz", latest);
        info!(%url, "Fetching MusicBrainz release archive");

        let res = reqwest::get(&url).await?.error_for_status()?;
        let size = res.content_length().unwrap_or(0) as usize;

        if size > 0 {
            info!(total_size = size, "Response content length reported");
        } else {
            info!("Response content length unknown");
        }

        let mut stream = res.bytes_stream();
        let mut downloaded = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;

            downloaded += chunk.len();
            if size > 0 {
                let progress = downloaded as f64 / size as f64 * 100.0;
                info!(downloaded, total_size = size, progress = %format!("{progress:.2}%"), "Download progress");
            } else {
                debug!(downloaded, "Downloaded bytes so far");
            }
        }

        info!(path = %path.display(), downloaded = downloaded, "Download complete");
        file.flush().await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn process_and_ingest(&self) -> anyhow::Result<()> {
        let path = working().join("release");
        info!(path = %path.display(), "Opening MusicBrainz release archive");

        let file = File::open(&path).await?;
        let reader = BufReader::new(file);

        let decoder = GzipDecoder::new(reader);
        let mut archive = Archive::new(decoder);

        let mut entries = archive.entries()?;
        let mut processed = 0usize;
        let mut failures = 0usize;

        while let Some(entry) = entries.next().await {
            let entry = entry?;
            let entry_path = entry.path()?;

            debug!(entry_path = %entry_path.display(), "Inspecting archive entry");

            if entry_path == Path::new("mdump/release") {
                info!(entry_path = %entry_path.display(), "Found mdump/release, starting line-by-line processing");

                let mut line_reader = BufReader::new(entry).lines();

                while let Some(line) = line_reader.next_line().await? {
                    let tx = self.db.begin_write()?;
                    let mut t_data = tx.open_table(TableDefinition::<String, String>::new("releases"))?;
                    let mut t_idx = tx.open_table(TableDefinition::<String, Vec<String>>::new("indexes"))?;

                    match serde_json::from_str::<ty::Root>(&line).map(MinifiedRelease::from) {
                        Ok(it) => {
                            t_data.insert(it.id.clone(), line.to_string())?;

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

                info!(processed, failures, "Finished processing mdump/release");
                return Ok(());
            }
        }

        error!(path = %path.display(), "No mdump/release entry found in archive");
        bail!("No release found in archive");
    }
}
