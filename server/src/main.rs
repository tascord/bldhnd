use {
    bh_server::{mb::ty::MinifiedRelease, wikidata::ty::WikiDataItem, *},
    milrouter::*,
    reqwest::header::HeaderMap,
    serde_json::json,
    std::fs::{self, File},
    tokio::spawn,
    tracing::{info, warn},
    tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt},
};

async fn auth(_: HeaderMap) -> anyhow::Result<()> { Ok(()) }

fn setup_logging() -> anyhow::Result<()> {
    // rotate previous LATEST -> timestamped file
    let logs_dir = logs();
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }

    let latest = logs_dir.join("LATEST");
    if latest.exists() {
        let meta = fs::metadata(&latest)?;
        // prefer creation time, fall back to modification time
        let sys_time = meta.created().or_else(|_| meta.modified()).unwrap_or(std::time::SystemTime::now());

        let dt: chrono::DateTime<chrono::Local> = sys_time.into();
        let name = format!("{}.log", dt.format("%Y%m%dT%H%M%S%z"));
        let target = logs_dir.join(name);
        // ignore error if rename fails for some reason
        let _ = fs::rename(&latest, &target);
    }

    // create new LATEST log file
    let file = File::create(&latest)?;

    let stdout_layer = fmt::layer()
        .with_file(true)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_ansi(true)
        .pretty()
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    let file_layer = fmt::layer()
        .with_file(true)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_ansi(false)
        .with_writer(move || file.try_clone().expect("failed to clone log file"))
        .compact()
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    tracing_subscriber::registry().with(stdout_layer).with(file_layer).init();

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging()?;

    info!("Started bldhnd server");

    let stale = fs::read_dir(working())?.collect::<Result<Vec<_>, _>>()?;
    if !stale.is_empty() {
        warn!(
            "Stale files in cache ({}): {}",
            working().display(),
            stale.iter().map(|s| s.file_name().display().to_string()).collect::<Vec<_>>().join(", ")
        );
    }

    spawn(async {
        let c = mb::client();
        let _ = c.fetch().await.inspect_err(|e| warn!("{e:?}"));
    });

    spawn(async {
        let c = wikidata::client();
        let _ = c.fetch().await.inspect_err(|e| warn!("{e:?}"));
    });

    serve(Router::route).await
}

#[endpoint(auth = auth)]
async fn music(q: (String, usize)) -> anyhow::Result<Vec<MinifiedRelease>> { mb::client().search(&q.0, q.1) }

#[endpoint(auth = auth)]
async fn media(q: (String, usize)) -> anyhow::Result<Vec<WikiDataItem>> { wikidata::client().search(&q.0, q.1) }

#[endpoint(auth = auth)]
async fn stats() -> anyhow::Result<serde_json::Value> {
    let mb = mb::client().stats()?;
    let wd = wikidata::client().stats()?;

    Ok(json!({
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
    Stats(EndpointStats)
}

pub fn notice() -> String {
    include_str!("../../CREDITS.md").to_string()
}