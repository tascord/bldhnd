use {
    std::{
        env, fs,
        path::{Path, PathBuf},
    },
    tokio::spawn,
    tracing::{info, level_filters::LevelFilter},
};

mod mb;
mod tm;
mod tv;

#[tokio::main]
async fn main() {
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

    spawn(async move {
        let c = mb::client();
        let _ = c.fetch().await;
    });
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
    async fn fetch(&self) -> anyhow::Result<()>;
}
