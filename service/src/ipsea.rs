use {
    crate::{
        config::Config,
        download::{Download, DownloadItem, DownloadManager, MediaType, Progress},
        notification::{Notification, NotificationBackend, SystemNotifier},
        plex::PlexClient,
    },
    download::{DownloadBackend, DownloadItem as LibDownloadItem, SearchBackend, SearchHit},
    serde::{Deserialize, Serialize},
    std::{sync::mpsc::Sender, time::Duration},
    tokio::time::sleep,
    tracing::{info, warn},
};

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
        backend: String,
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
    DownloadProgress { progress: Progress },
    CancelDownload,
    Error { message: String },
}

pub fn handle_request(req: Request, tx: Sender<Response>) {
    match req {
        Request::GetConfig => {
            let config = Config::load();
            let _ = tx.send(Response::GetConfig { config });
        }
        Request::UpdateConfig { config } => {
            let current = Config::load();
            let updated = Config {
                volumes: config.volumes,
                bh_server_url: config.bh_server_url.or(current.bh_server_url),
                soulseek_username: config.soulseek_username.or(current.soulseek_username),
                soulseek_password: config.soulseek_password.or(current.soulseek_password),
                download_dir: config.download_dir.or(current.download_dir),
                quality: config.quality,
                notifications: config.notifications,
                plex: config.plex.or(current.plex),
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
        Request::Search { query, backend } => {
            let rt = tokio::runtime::Handle::current();
            let results = rt.block_on(async {
                let config = Config::load();
                let backend_config = download::BackendConfig {
                    soulseek_username: config.soulseek_username.clone(),
                    soulseek_password: config.soulseek_password.clone(),
                    bh_server_url: config.bh_server_url.clone(),
                    subtitle_api_key: None,
                };
                match backend.as_str() {
                    "soulseek" => {
                        if let Some(searcher) = download::slsk::SlskSearcher::with_config(&backend_config) {
                            searcher.search(&query).await
                        } else {
                            Err(anyhow::anyhow!("Soulseek credentials not configured"))
                        }
                    }
                    "torrent" => {
                        if let Some(searcher) = download::torrent::TorrentSearcher::with_config(&backend_config) {
                            searcher.search(&query).await
                        } else {
                            Err(anyhow::anyhow!("Indexer URL not configured"))
                        }
                    }
                    "usenet" => {
                        if let Some(searcher) = download::usenet::UsenetSearcher::with_config(&backend_config) {
                            searcher.search(&query).await
                        } else {
                            Err(anyhow::anyhow!("Indexer URL not configured"))
                        }
                    }
                    _ => Err(anyhow::anyhow!("Unknown backend: {}", backend)),
                }
            });
            match results {
                Ok(results) => {
                    let _ = tx.send(Response::Search { results });
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
            let media_type_str = media_type.clone();
            let media_type = match media_type.as_str() {
                "Music" => MediaType::Music,
                "Movie" => MediaType::Movie,
                "TvShow" | "Series" => MediaType::TvShow,
                _ => MediaType::Music,
            };
            let item = DownloadItem {
                backend: backend.clone(),
                title: title.clone(),
                filename: filename.clone(),
                url: url.clone(),
                size,
                year,
                media_type,
                info_hash: info_hash.clone(),
            };
            let manager = DownloadManager::new();
            match manager.create_download(item) {
                Ok(id) => {
                    let rt = tokio::runtime::Handle::current();
                    let download_dir = crate::download::download_dir();

                    rt.spawn(async move {
                        let boxed: Box<str> = backend.clone().into_boxed_str();
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
                        let backend_config = download::BackendConfig {
                            soulseek_username: config.soulseek_username.clone(),
                            soulseek_password: config.soulseek_password.clone(),
                            bh_server_url: config.bh_server_url.clone(),
                            subtitle_api_key: None,
                        };

                        let result = match backend_str {
                            "soulseek" => {
                                if let Some(downloader) = download::slsk::SlskDownloader::with_config(&backend_config) {
                                    downloader.download(&lib_item, &download_dir, None).await
                                } else {
                                    Err(anyhow::anyhow!("Soulseek credentials not configured"))
                                }
                            }
                            "torrent" => {
                                if let Some(downloader) = download::torrent::TorrentDownloader::with_config(&backend_config)
                                {
                                    downloader.download(&lib_item, &download_dir, None).await
                                } else {
                                    Err(anyhow::anyhow!("qBittorrent URL not configured"))
                                }
                            }
                            "usenet" => {
                                if let Some(downloader) = download::usenet::UsenetDownloader::with_config(&backend_config) {
                                    downloader.download(&lib_item, &download_dir, None).await
                                } else {
                                    Err(anyhow::anyhow!("SABnzbd URL not configured"))
                                }
                            }
                            _ => Err(anyhow::anyhow!("Unknown backend: {}", backend_str)),
                        };

                        match result {
                            Ok(path) => {
                                let mgr = DownloadManager::new();
                                info!("Download completed: {} -> {:?}", id, path);
                                let path_for_notif = path.clone();
                                let _ = mgr.update_path(id, path);
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
                                let _ = mgr.update_state(id, crate::download::DownloadState::Failed);
                                mgr.mark_complete(id);
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
                    });

                    let _ = tx.send(Response::StartDownload { id });
                }
                Err(e) => {
                    let _ = tx.send(Response::Error { message: e.to_string() });
                }
            }
        }
        Request::DownloadProgress { id } => {
            let manager = DownloadManager::new();
            let progress = if manager.is_active(id) {
                Progress { bytes_done: 0, total_bytes: 0, state: crate::download::DownloadState::Downloading, speed_bps: 0 }
            } else {
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
            };
            let _ = tx.send(Response::DownloadProgress { progress });
        }
        Request::CancelDownload { id } => {
            let manager = DownloadManager::new();
            if let Err(e) = manager.update_state(id, crate::download::DownloadState::Cancelled) {
                let _ = tx.send(Response::Error { message: e.to_string() });
            } else {
                manager.mark_complete(id);
                let _ = tx.send(Response::CancelDownload);
            }
        }
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
