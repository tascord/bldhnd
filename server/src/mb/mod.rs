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
    tracing::{info, warn},
};

pub mod ty;
static CLIENT: LazyLock<Arc<MusicBrainz>> = LazyLock::new(|| Arc::new(MusicBrainz::new()));
pub fn client() -> Arc<MusicBrainz> { CLIENT.clone() }

#[derive(Debug)]
pub struct MusicBrainz {
    latest: Arc<RwLock<[char; 16]>>,
    db: Database,
}

pub struct Release {}

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

    #[tracing::instrument]
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

    #[tracing::instrument]
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

        Ok(())
    }

    #[tracing::instrument]
    async fn process_and_ingest(&self) -> anyhow::Result<()> {
        let file = File::open(working().join("release")).await?;
        let reader = BufReader::new(file);

        let decoder = GzipDecoder::new(reader);
        let mut archive = Archive::new(decoder);

        let mut entries = archive.entries()?;

        while let Some(entry) = entries.next().await {
            let entry = entry?;
            let entry_path = entry.path()?;

            if entry_path == Path::new("mdump/release") {
                info!("Found mdump/release, starting line-by-line processing...");

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
                        }
                        Err(e) => {
                            warn!("Failed to parse release item: {e:?}");
                        }
                    }
                }

                info!("Finished processing mdump/release smoothly!");
                return Ok(());
            }
        }

        bail!("No release found in archive");
    }
}