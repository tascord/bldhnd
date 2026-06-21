use {
    serde::Serialize,
    std::{
        env, fs,
        path::{Path, PathBuf},
    },
    tracing::{info, level_filters::LevelFilter, warn},
};

mod mb;
mod tm;
mod tv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_max_level(LevelFilter::INFO)
        .with_ansi(true)
        .pretty()
        .init();

    info!("Started bldhnd server");

    let stale = fs::read_dir(working())?.collect::<Result<Vec<_>, _>>()?;
    if !stale.is_empty() {
        warn!(
            "Stale files in cache ({}): {}",
            working().display(),
            stale.iter().map(|s| s.file_name().display().to_string()).collect::<Vec<_>>().join(", ")
        );
    }

    let c = mb::client();
    c.fetch().await?;

    Ok(())
}

pub fn working() -> PathBuf {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".cache/").join("bldhnd/");
    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'working' folder.");
    }

    p
}

pub fn db() -> PathBuf {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".bldhnd/").join(".dbs/");
    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'db' folder.");
    }

    p
}

#[allow(async_fn_in_trait)]
pub trait KnowledgeBase {
    type Output: Serialize;
    async fn fetch(&self) -> anyhow::Result<()>;
    async fn search(&self, q: &str, p: usize) -> anyhow::Result<Vec<Self::Output>>;
}
