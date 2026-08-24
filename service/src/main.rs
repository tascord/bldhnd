use {
    std::{env, path::PathBuf},
    tracing::{info, warn},
    tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt},
};

mod config;
mod download;
mod ipsea;
mod notification;
mod plex;
mod search;
mod users;

fn setup_logging() -> anyhow::Result<()> {
    let logs_dir = logs_dir();
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir)?;
    }

    let latest = logs_dir.join("LATEST");
    if latest.exists() {
        let meta = std::fs::metadata(&latest)?;
        let sys_time = meta.created().or_else(|_| meta.modified()).unwrap_or(std::time::SystemTime::now());
        let dt: chrono::DateTime<chrono::Local> = sys_time.into();
        let name = format!("{}.log", dt.format("%Y%m%dT%H%M%S%z"));
        let target = logs_dir.join(name);
        let _ = std::fs::rename(&latest, &target);
    }

    let file = std::fs::File::create(&latest)?;

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

fn logs_dir() -> PathBuf {
    if let Ok(s) = env::var("XDG_STATE_HOME") {
        PathBuf::from(s).join("bldhnd").join("logs")
    } else if let Ok(b) = env::var("BLDHND_DIR") {
        PathBuf::from(b).join("logs")
    } else {
        std::env::home_dir().expect("No home dir").join(".bldhnd/logs")
    }
}

fn main() -> anyhow::Result<()> {
    setup_logging()?;

    info!("Started bldhnd service");

    config::init();
    users::init();
    download::init();

    // The ipsea IPC layer binds sockets at /tmp/{name}.sock, so both this
    // service and the TUI (app/src/ipsea.rs) must agree on that exact path.
    let socket_path = std::path::PathBuf::from("/tmp/bldhnd.sock");
    let socket_name = "bldhnd".to_string();

    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    if let Err(e) = ipsea::serve(&socket_name) {
        warn!("bh-service error: {}", e);
    }

    Ok(())
}
