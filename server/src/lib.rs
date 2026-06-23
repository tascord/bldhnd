use {
    crate::{mb::ty::MinifiedRelease, reqwest::header::HeaderMap, wikidata::ty::WikiDataItem},
    milrouter::*,
    serde::Serialize,
    std::{
        env,
        fs::{self},
        path::{Path, PathBuf},
    },
};

pub mod mb;
pub mod wikidata;

pub fn working() -> PathBuf {
    // Prefer XDG cache dir, then BLDHND_DIR, then default to ~/.bldhnd
    let p = if let Ok(x) = env::var("XDG_CACHE_HOME") {
        PathBuf::from(x).join("bldhnd").join("cache")
    } else if let Ok(b) = env::var("BLDHND_DIR") {
        PathBuf::from(b).join("cache")
    } else {
        Path::new(&env::home_dir().expect("No home dir")).join(".bldhnd/").join("cache/")
    };

    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'working' folder.");
    }

    p
}

pub fn logs() -> PathBuf {
    // Prefer XDG state dir, then BLDHND_DIR, then default to ~/.bldhnd
    let p = if let Ok(s) = env::var("XDG_STATE_HOME") {
        PathBuf::from(s).join("bldhnd").join("logs")
    } else if let Ok(b) = env::var("BLDHND_DIR") {
        PathBuf::from(b).join("logs")
    } else {
        Path::new(&env::home_dir().expect("No home dir")).join(".bldhnd/").join("logs/")
    };

    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'logs' folder.");
    }

    p
}

pub fn db() -> PathBuf {
    // Use BLDHND_DIR or XDG_DATA_HOME (as bldhnd subdir) or default to ~/.bldhnd
    let p = if let Ok(b) = env::var("BLDHND_DIR") {
        PathBuf::from(b).join("dbs")
    } else if let Ok(x) = env::var("XDG_DATA_HOME") {
        PathBuf::from(x).join("bldhnd").join("dbs")
    } else {
        Path::new(&env::home_dir().expect("No home dir")).join(".bldhnd/").join("dbs/")
    };

    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'db' folder.");
    }

    p
}

#[allow(async_fn_in_trait)]
pub trait KnowledgeBase {
    type Output: Serialize;
    async fn fetch(&self) -> anyhow::Result<()>;
    fn search(&self, q: &str, p: usize) -> anyhow::Result<Vec<Self::Output>>;
    fn stats(&self) -> anyhow::Result<usize>;
}

async fn auth(_: HeaderMap) -> anyhow::Result<()> { Ok(()) }

#[endpoint(auth = auth)]
async fn music(q: (String, usize)) -> anyhow::Result<Vec<MinifiedRelease>> { mb::client().search(&q.0, q.1) }

#[endpoint(auth = auth)]
async fn media(q: (String, usize)) -> anyhow::Result<Vec<WikiDataItem>> { wikidata::client().search(&q.0, q.1) }

#[endpoint(auth = auth)]
async fn stats() -> anyhow::Result<serde_json::Value> {
    let mb = mb::client().stats()?;
    let wd = wikidata::client().stats()?;

    Ok(serde_json::json!({
        "music": mb,
        "media": wd
    }))
}

#[derive(Router)]
#[assets(../../_assets)]
#[html(notice)]
pub enum Router {
    Music(EndpointMusic),
    Media(EndpointMedia),
    Stats(EndpointStats),
}

pub fn notice() -> String { include_str!("../../CREDITS.md").to_string() }
