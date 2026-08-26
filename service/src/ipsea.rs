use {
    crate::{
        config::Config,
        download::{Download, DownloadItem, DownloadManager, MediaType, Progress},
        notification::{Notification, NotificationBackend, SystemNotifier},
        plex::PlexClient,
        search::SearchHit,
    },
    download::{DownloadBackend, DownloadItem as LibDownloadItem, SearchBackend},
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, sync::mpsc::Sender, time::Duration},
    tokio::time::sleep,
    tracing::{error, info, warn},
};

/// Byte-level progress of in-flight downloads, updated by backend callbacks.
/// Keyed by download id; entries are removed on completion/cancel.
static LIVE_PROGRESS: std::sync::LazyLock<std::sync::Mutex<HashMap<u64, download::DownloadProgress>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

fn report_progress(id: u64, p: download::DownloadProgress) {
    let mut live = LIVE_PROGRESS.lock().unwrap();
    if p.state == download::DownloadState::Complete || p.state == download::DownloadState::Failed {
        // Terminal state: drop the entry so the DB row (authoritative)
        // takes over for future pollers.
        live.remove(&id);
        return;
    }
    live.insert(id, p);
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Request {
    GetConfig,
    UpdateConfig {
        config: Config,
    },
    CommitConfig,
    Search {
        query: String,
        /// Knowledge-base search type: "Music" | "Movie" | "Series".
        #[serde(default)]
        media_type: Option<String>,
        /// Download-backend search: "soulseek" | "torrent" | "usenet".
        #[serde(default)]
        backend: Option<String>,
    },
    ListDownloads,
    GetDownload {
        id: u64,
    },
    StartDownload {
        backend: String,
        title: String,
        filename: String,
        url: String,
        size: u64,
        year: Option<u32>,
        media_type: String,
        info_hash: Option<String>,
    },
    DownloadProgress {
        id: u64,
    },
    /// Re-drive a failed/cancelled download with its stored parameters.
    RetryDownload {
        id: u64,
    },
    CancelDownload {
        id: u64,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
pub enum Response {
    GetConfig { config: Config },
    UpdateConfig,
    CommitConfig,
    Search { results: Vec<SearchHit> },
    ListDownloads { downloads: Vec<Download> },
    GetDownload { download: Option<Download> },
    StartDownload { id: u64 },
    RetryDownload,
    DownloadProgress { progress: Progress },
    CancelDownload,
    Error { message: String },
}

/// Run blocking work on a dedicated OS thread — ipsea handlers run without a
/// tokio reactor, and `reqwest::blocking` / fresh runtimes must not be entered
/// from inside one anyway.
fn run_blocking<T, F>(f: F) -> anyhow::Result<T>
where
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(f).join().map_err(|_| anyhow::anyhow!("worker thread panicked"))?
}

/// Build download-backend config from user settings.
fn backend_config_from(config: &Config) -> download::BackendConfig {
    download::BackendConfig {
        soulseek_username: config.soulseek_username.clone(),
        soulseek_password: config.soulseek_password.clone(),
        bh_server_url: config.bh_server_url.clone(),
        subtitle_api_key: None,
        torrent_indexer: config.torrent_indexers.first().map(|i| (i.url.clone(), i.api_key.clone())),
        usenet_indexer: config.usenet_indexers.first().map(|i| (i.url.clone(), i.api_key.clone())),
        sabnzbd: config.sabnzbd.as_ref().map(|s| (s.url.clone(), Some(s.api_key.clone()))),
    }
}

/// Search every configured indexer of `kind`, concatenating results.
/// Indexers that fail are skipped (logged) so one bad endpoint doesn't kill the search.
fn search_indexers(config: &Config, kind: &str, query: &str) -> anyhow::Result<Vec<download::SearchHit>> {
    let indexers: Vec<&crate::config::Indexer> = match kind {
        "torrent" => config.torrent_indexers.iter().collect(),
        "usenet" => config.usenet_indexers.iter().collect(),
        _ => Vec::new(),
    };

    if indexers.is_empty() {
        anyhow::bail!("No {kind} indexers configured — add one in Settings");
    }

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let mut all = Vec::new();
    let mut last_err = None;

    for idx in indexers {
        let cfg = download::BackendConfig {
            torrent_indexer: (kind == "torrent").then(|| (idx.url.clone(), idx.api_key.clone())),
            usenet_indexer: (kind == "usenet").then(|| (idx.url.clone(), idx.api_key.clone())),
            ..Default::default()
        };
        let result = rt.block_on(async {
            let fut: std::pin::Pin<Box<dyn futures::Future<Output = anyhow::Result<Vec<download::SearchHit>>> + Send>> = match kind {
                "torrent" => match download::torrent::TorrentSearcher::with_config(&cfg) {
                    Some(s) => Box::pin(async move { s.search(query).await }),
                    None => Box::pin(std::future::ready(Err(anyhow::anyhow!("indexer misconfigured")))),
                },
                "usenet" => match download::usenet::UsenetSearcher::with_config(&cfg) {
                    Some(s) => Box::pin(async move { s.search(query).await }),
                    None => Box::pin(std::future::ready(Err(anyhow::anyhow!("indexer misconfigured")))),
                },
                _ => Box::pin(std::future::ready(Err(anyhow::anyhow!("unknown kind")))),
            };
            fut.await
        });
        match result {
            Ok(hits) => all.extend(hits),
            Err(e) => {
                warn!("Indexer '{}' failed: {e}", idx.name);
                last_err = Some(e);
            }
        }
    }

    if all.is_empty() {
        if let Some(e) = last_err {
            return Err(e);
        }
    }
    Ok(all)
}

/// (Re)drive one persisted download through its backend on a dedicated
/// runtime thread. Used by StartDownload, retry, and startup resume.
fn launch_download(d: &crate::download::Download) {
    // Clone everything the job thread needs up front.
    let (backend, title, filename, uri, size, info_hash, media_type_str) = (
        d.backend.clone(),
        d.title.clone(),
        d.filename.clone(),
        d.uri.clone().unwrap_or_default(),
        d.size,
        d.info_hash.clone(),
        d.media_type.to_string(),
    );
    let id = d.id;
    // ipsea handlers run without a tokio reactor — Handle::current() would
    // panic here and silently kill the download before it starts. Drive the
    // async work on a dedicated thread with its own runtime instead.
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    warn!("download runtime failed: {e}");
                    let _ = DownloadManager::new().update_state(id, crate::download::DownloadState::Failed);
                    return;
                }
            };
            rt.block_on(download_job(
                id,
                backend,
                title,
                filename,
                uri,
                size,
                info_hash,
                media_type_str,
                crate::download::download_dir(),
            ));
        }));
        if let Err(panic) = result {
            let msg = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".into());
            error!("download job {id} panicked: {msg}");
            let _ = DownloadManager::new().update_state(id, crate::download::DownloadState::Failed);
        }
    });
}

pub fn handle_request(req: Request, tx: Sender<Response>) {    match req {
        Request::GetConfig => {
            let config = Config::load();
            let _ = tx.send(Response::GetConfig { config });
        }
        Request::UpdateConfig { config } => {
            // The TUI owns everything user-editable; preserve service-managed
            // sections and the server url it can't see.
            let current = Config::load();
            let updated = Config {
                bh_server_url: config.bh_server_url.or(current.bh_server_url),
                quality: current.quality,
                notifications: current.notifications,
                plex: current.plex.or(config.plex),
                ..config
            };
            if let Err(e) = updated.save() {
                let _ = tx.send(Response::Error { message: e.to_string() });
            } else {
                let _ = tx.send(Response::UpdateConfig);
            }
        }
        Request::CommitConfig => {
            if let Err(e) = Config::load().save() {
                let _ = tx.send(Response::Error { message: e.to_string() });
            } else {
                let _ = tx.send(Response::CommitConfig);
            }
        }
        Request::Search { query, media_type, backend } => {
            // Knowledge-base search (what the TUI uses).
            //
            // ipsea dispatches handlers on plain OS threads — there is no
            // tokio reactor here, so run blocking work on a dedicated thread.
            if let Some(media_type) = media_type {
                let results = run_blocking(move || crate::search::search(&query, &media_type));
                match results {
                    Ok(results) => {
                        let _ = tx.send(Response::Search { results });
                    }
                    Err(e) => {
                        let _ = tx.send(Response::Error { message: e.to_string() });
                    }
                }
                return;
            }

            // Download-backend search.
            let Some(backend) = backend else {
                let _ = tx.send(Response::Error { message: "Search requires either media_type or backend".into() });
                return;
            };
            let backend_name = backend.clone();
            let results = run_blocking(move || {
                let config = Config::load();

                if backend_name == "soulseek" {
                    let cfg = backend_config_from(&config);
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    return rt.block_on(async {
                        match download::slsk::SlskSearcher::with_config(&cfg) {
                            Some(searcher) => searcher.search(&query).await,
                            None => Err(anyhow::anyhow!("Soulseek credentials not configured")),
                        }
                    });
                }

                search_indexers(&config, &backend_name, &query)
            });
            match results {
                Ok(results) => {
                    let hits = results
                        .into_iter()
                        .map(|r| SearchHit {
                            backend: backend.clone(),
                            title: r.title,
                            artist: r.username,
                            year: None,
                            size: r.size,
                            ext: r.ext,
                            url: r.url,
                        })
                        .collect();
                    let _ = tx.send(Response::Search { results: hits });
                }
                Err(e) => {
                    let _ = tx.send(Response::Error { message: e.to_string() });
                }
            }
        }
        Request::ListDownloads => {
            let manager = DownloadManager::new();
            match manager.list_downloads() {
                Ok(downloads) => {
                    let _ = tx.send(Response::ListDownloads { downloads });
                }
                Err(e) => {
                    let _ = tx.send(Response::Error { message: e.to_string() });
                }
            }
        }
        Request::GetDownload { id } => {
            let manager = DownloadManager::new();
            match manager.get_download(id) {
                Ok(download) => {
                    let _ = tx.send(Response::GetDownload { download });
                }
                Err(e) => {
                    let _ = tx.send(Response::Error { message: e.to_string() });
                }
            }
        }
        Request::StartDownload { backend, title, filename, url, size, year, media_type, info_hash } => {
            let media_type = match media_type.as_str() {
                "Music" => MediaType::Music,
                "Movie" => MediaType::Movie,
                "TvShow" | "Series" => MediaType::TvShow,
                _ => MediaType::Music,
            };
            let item = DownloadItem {
                backend,
                title,
                filename,
                url,
                size,
                year,
                media_type,
                info_hash,
            };
            let manager = DownloadManager::new();
            match manager.create_download(item) {
                Ok(id) => match manager.get_download(id) {
                    Ok(Some(d)) => {
                        manager.mark_active(id);
                        launch_download(&d);
                        let _ = tx.send(Response::StartDownload { id });
                    }
                    Ok(None) => {
                        let _ = tx.send(Response::Error { message: "download row vanished".into() });
                    }
                    Err(e) => {
                        let _ = tx.send(Response::Error { message: e.to_string() });
                    }
                },
                Err(e) => {
                    let _ = tx.send(Response::Error { message: e.to_string() });
                }
            }
        }
        Request::DownloadProgress { id } => {
            // Live byte-level progress from backend callbacks wins over the
            // DB row (which only carries coarse state).
            let progress = LIVE_PROGRESS.lock().unwrap().get(&id).cloned().map(Into::into).unwrap_or_else(|| {
                let manager = DownloadManager::new();
                match manager.get_download(id) {
                    Ok(Some(d)) => {
                        let state = if d.state == crate::download::DownloadState::Complete {
                            crate::download::DownloadState::Complete
                        } else {
                            crate::download::DownloadState::Queued
                        };
                        Progress { bytes_done: 0, total_bytes: d.size, state, speed_bps: 0 }
                    }
                    _ => Progress {
                        bytes_done: 0,
                        total_bytes: 0,
                        state: crate::download::DownloadState::Queued,
                        speed_bps: 0,
                    },
                }
            });
            let _ = tx.send(Response::DownloadProgress { progress });
        }
        Request::RetryDownload { id } => {
            let manager = DownloadManager::new();
            let terminal = |s: &crate::download::DownloadState| {
                matches!(s, crate::download::DownloadState::Failed | crate::download::DownloadState::Cancelled)
            };
            match manager.get_download(id) {
                Ok(Some(d)) if terminal(&d.state) => {
                    match manager.update_state(id, crate::download::DownloadState::Queued) {
                        Ok(()) => {
                            manager.mark_active(id);
                            launch_download(&d);
                            info!("Retrying download {} ({})", id, d.title);
                            let _ = tx.send(Response::RetryDownload);
                        }
                        Err(e) => {
                            let _ = tx.send(Response::Error { message: e.to_string() });
                        }
                    }
                }
                Ok(Some(_)) => {
                    let _ = tx.send(Response::Error { message: "only failed or cancelled downloads can be retried".into() });
                }
                Ok(None) => {
                    let _ = tx.send(Response::Error { message: format!("no download with id {id}") });
                }
                Err(e) => {
                    let _ = tx.send(Response::Error { message: e.to_string() });
                }
            }
        }
        Request::CancelDownload { id } => {
            LIVE_PROGRESS.lock().unwrap().remove(&id);
            // Stop actual torrent transfer, not just the DB row.
            let manager = DownloadManager::new();
            if let Ok(Some(d)) = manager.get_download(id) {
                if d.backend == "torrent" {
                    let key = download::torrent::torrent_key(
                        d.uri.as_deref().unwrap_or(""),
                        d.info_hash.as_deref(),
                    );
                    download::torrent::cancel_torrent(&key);
                }
            }
            if let Err(e) = manager.update_state(id, crate::download::DownloadState::Cancelled) {
                let _ = tx.send(Response::Error { message: e.to_string() });
            } else {
                manager.mark_complete(id);
                let _ = tx.send(Response::CancelDownload);
            }
        }
    }
}

/// Execute one queued download end-to-end: pick the backend, run with live
/// progress reporting, then persist outcome and fire notifications.
#[allow(clippy::too_many_arguments)]
async fn download_job(
    id: u64,
    backend: String,
    title: String,
    filename: String,
    url: String,
    size: u64,
    info_hash: Option<String>,
    media_type_str: String,
    download_dir: std::path::PathBuf,
) {
    let boxed: Box<str> = backend.into_boxed_str();
    let backend_str: &'static str = Box::leak(boxed);
    let title_for_notif = title.clone();
    let lib_item = LibDownloadItem {
        backend: backend_str,
        title,
        filename,
        size,
        info_hash,
        nzb_data: None,
        uri: if url.is_empty() { None } else { Some(url) },
        username: None,
    };

    let config = crate::config::Config::load();
    let backend_config = backend_config_from(&config);

    // Backend callbacks stream live byte progress into the registry for
    // DownloadProgress pollers.
    let progress_cb: std::sync::Arc<dyn Fn(download::DownloadProgress) + Send + Sync> =
        std::sync::Arc::new(move |p| report_progress(id, p));

    let result = match backend_str {
        "soulseek" => match download::slsk::SlskDownloader::with_config(&backend_config) {
            Some(downloader) => downloader.download(&lib_item, &download_dir, Some(progress_cb)).await,
            None => Err(anyhow::anyhow!("Soulseek credentials not configured")),
        },
        "torrent" => {
            download::torrent::TorrentDownloader::default()
                .download(&lib_item, &download_dir, Some(progress_cb))
                .await
        }
        "usenet" => match download::usenet::UsenetDownloader::with_config(&backend_config) {
            Some(downloader) => downloader.download(&lib_item, &download_dir, Some(progress_cb)).await,
            None => Err(anyhow::anyhow!("SABnzbd URL not configured")),
        },
        _ => Err(anyhow::anyhow!("Unknown backend: {}", backend_str)),
    };

    match result {
        Ok(path) => {
            let mgr = DownloadManager::new();
            info!("Download completed: {} -> {:?}", id, path);
            let path_for_notif = path.clone();
            if let Err(e) = mgr.update_path(id, path) { error!("update_path failed: {e:#}"); }
            mgr.mark_complete(id);

            let cfg = Config::load();
            if cfg.notifications.enabled && cfg.notifications.system_notifications {
                let notif = Notification {
                    title: format!("Download complete: {}", title_for_notif),
                    body: format!("Saved to {:?}", path_for_notif),
                    icon: None,
                    data: None,
                };
                let notifier = SystemNotifier;
                let _ = notifier.send(&notif).await;
            }

            if let Some(ref plex_cfg) = cfg.plex {
                if plex_cfg.auto_scan {
                    let client = PlexClient::new(&plex_cfg.url, plex_cfg.token.clone());
                    let _ = client.process_download(&path_for_notif, &media_type_str).await;
                }
            }
        }
        Err(e) => {
            let mgr = DownloadManager::new();
            if let Err(e) = mgr.update_state(id, crate::download::DownloadState::Failed) { error!("update_state failed: {e:#}"); }
            mgr.mark_complete(id);
            LIVE_PROGRESS.lock().unwrap().remove(&id);
            warn!("Download failed: {} - {}", id, e);

            let cfg = Config::load();
            if cfg.notifications.enabled && cfg.notifications.system_notifications {
                let notif = Notification {
                    title: format!("Download failed: {}", title_for_notif),
                    body: e.to_string(),
                    icon: None,
                    data: None,
                };
                let notifier = SystemNotifier;
                let _ = notifier.send(&notif).await;
            }
        }
    }
}

/// Re-drive downloads that were still in flight when the service last shut
/// down. Called once at boot, after the DB is initialised.
pub fn resume_pending() {
    info!("resume_pending: scanning for in-flight downloads");
    let manager = DownloadManager::new();
    let Ok(pending) = manager.list_downloads() else { return };
    info!("resume_pending: {} row(s) found", pending.len());
    let mut resumed = 0usize;
    for d in pending {
        if matches!(
            d.state,
            crate::download::DownloadState::Queued
                | crate::download::DownloadState::Connecting
                | crate::download::DownloadState::Downloading
        ) {
            // Reset to a clean queued state before relaunching.
            if manager.update_state(d.id, crate::download::DownloadState::Queued).is_ok() {
                info!("Resuming download {} ({})", d.id, d.title);
                manager.mark_active(d.id);
                launch_download(&d);
                resumed += 1;
            }
        }
    }
    if resumed > 0 {
        info!("Resumed {resumed} in-flight download(s)");
    }
}

pub fn serve(socket_name: &str) -> std::io::Result<()> {
    let socket_path = if socket_name.starts_with('/') {
        std::path::PathBuf::from(socket_name)
    } else {
        std::path::PathBuf::from(format!("/tmp/{}.sock", socket_name))
    };

    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    info!("Starting bh-service on {}", socket_path.display());

    ipsea::start_server(socket_name, handle_request)
}
