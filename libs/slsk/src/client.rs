//! High-level Soulseek client API.
//!
//! # Quick start
//!
//! ```no_run
//! # tokio_test::block_on(async {
//! use soulseek_client::SlskClient;
//! use std::path::Path;
//!
//! let mut client = SlskClient::connect("server.slsknet.org:2416").await?;
//! client.login("username", "password").await?;
//!
//! // Search
//! let results = client.search("pink floyd flac", std::time::Duration::from_secs(10)).await?;
//! for r in &results {
//!     println!("{} — {}", r.username, r.filename);
//! }
//!
//! // Download the first result
//! if let Some(first) = results.first() {
//!     client.download(first, Path::new("downloads"), None).await?;
//! }
//! # Ok::<(), soulseek_client::error::SlskError>(())
//! # });
//! ```

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rand::RngExt;
use tracing::{debug, info, warn};

use crate::error::{Result, SlskError};
use crate::peer::conn::PeerConn;
use crate::peer::transfer::{download_file, ProgressFn};
use crate::proto::frame::MsgBuilder;
use crate::proto::msg::{ServerCode, ServerMsg};
use crate::server::conn::ServerConn;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single downloadable search result, combining the peer username with file info.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub username: String,
    pub filename: String,
    pub size: u64,
    /// File extension (e.g. "flac", "mp3").
    pub ext: String,
    /// Attribute slots: [bitrate, duration_secs, vbr, …] — indices depend on format.
    pub attrs: Vec<u32>,
    /// Whether the peer has a free upload slot right now.
    pub free_slot: bool,
    /// Peer's reported upload speed (bytes/s).
    pub upload_speed: u32,
    /// Position in peer's upload queue (0 = immediate).
    pub in_queue: u32,
}

impl SearchHit {
    /// Convenience: bitrate attribute (attrs[0] when present).
    pub fn bitrate(&self) -> Option<u32> {
        self.attrs.first().copied()
    }
    /// Convenience: duration in seconds (attrs[1] when present).
    pub fn duration_secs(&self) -> Option<u32> {
        self.attrs.get(1).copied()
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Options for `SlskClient::connect_with`.
pub struct Config {
    pub server_addr: String,
    /// Port we listen on for incoming peer connections. 0 = don't listen.
    pub listen_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server_addr: "server.slsknet.org:2416".into(),
            listen_port: 2234,
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct SlskClient {
    conn: ServerConn,
    username: String,
    next_token: u32,
}

impl SlskClient {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Connect to `server.slsknet.org:2416`.
    pub async fn connect(addr: &str) -> Result<Self> {
        info!("Connecting to {addr}");
        Ok(SlskClient {
            conn: ServerConn::connect(addr).await?,
            username: String::new(),
            next_token: rand::rng().random_range(1..u32::MAX / 2),
        })
    }

    // -----------------------------------------------------------------------
    // Login
    // -----------------------------------------------------------------------

    /// Login and perform post-login housekeeping (set port, status, share counts).
    /// Returns the server's greeting string.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<String> {
        self.username = username.to_owned();

        let md5_pass = format!("{:x}", md5::compute(password));
        let md5_hash = format!("{:x}", md5::compute(format!("{password}{md5_pass}")));

        let msg = MsgBuilder::server(ServerCode::Login as u32)
            .str(username)
            .str(password)
            .u32(160)
            .str(&md5_hash)
            .u32(1)
            .build();

        info!("→ Login as {username}");
        self.conn.send(&msg).await?;

        let greet = loop {
            match self.conn.recv().await? {
                ServerMsg::LoginOk { greet, .. } => break greet,
                ServerMsg::LoginFail { reason } => return Err(SlskError::LoginFailed(reason)),
                _ => {}
            }
        };

        // Housekeeping: tell server our port, status, and shared file counts.
        self.send_set_wait_port(2234).await?;
        self.send_status(2).await?;
        self.send_shared_counts(0, 0).await?;

        // Tell the distributed network we have no parent (keeps things simple).
        self.conn
            .send(&MsgBuilder::server(ServerCode::HaveNoParent as u32).u8(1).build())
            .await?;

        info!("Logged in. Server: {greet}");
        Ok(greet)
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    /// Search and collect results for `timeout`.
    /// Returns all hits received within that window, deduplicated by (username, filename).
    pub async fn search(&mut self, query: &str, timeout: Duration) -> Result<Vec<SearchHit>> {
        let token = self.token();
        let msg = MsgBuilder::server(ServerCode::FileSearch as u32)
            .u32(token)
            .str(query)
            .build();
        self.conn.send(&msg).await?;
        info!("→ Search [{token}] {query:?}");

        let deadline = tokio::time::Instant::now() + timeout;
        let mut hits: Vec<SearchHit> = Vec::new();
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, self.conn.recv()).await {
                Ok(Ok(ServerMsg::SearchResults(r))) if r.token == token => {
                    for f in r.results {
                        let key = (r.username.clone(), f.filename.clone());
                        if seen.insert(key) {
                            hits.push(SearchHit {
                                username:     r.username.clone(),
                                filename:     f.filename,
                                size:         f.size,
                                ext:          f.ext,
                                attrs:        f.attrs,
                                free_slot:    r.free_upload_slots,
                                upload_speed: r.upload_speed,
                                in_queue:     r.in_queue,
                            });
                        }
                    }
                }
                Ok(Ok(_)) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => break, // timeout
            }
        }

        info!("Search done: {} hits for {query:?}", hits.len());
        Ok(hits)
    }

    // -----------------------------------------------------------------------
    // Download
    // -----------------------------------------------------------------------

    /// Download a single `SearchHit` to `dest_dir/<filename>`.
    ///
    /// `progress` receives `(bytes_done, total_bytes)` during the transfer.
    pub async fn download(
        &mut self,
        hit: &SearchHit,
        dest_dir: &Path,
        progress: Option<ProgressFn>,
    ) -> Result<PathBuf> {
        // Derive a local filename from the last path component.
        let basename = hit
            .filename
            .replace('\\', "/")
            .rsplit('/')
            .next()
            .unwrap_or("download")
            .to_owned();
        let dest = dest_dir.join(&basename);

        // Resolve the peer's address from the server.
        let addr = self.resolve_peer(&hit.username).await?;

        // Open a peer connection and do the transfer.
        let token = self.token();
        let peer = self
            .connect_peer(addr, &hit.username, "F", token, None)
            .await?;

        download_file(peer, token, &hit.filename, &dest, 0, progress).await?;
        Ok(dest)
    }

    /// Download multiple hits sequentially.
    pub async fn download_all(
        &mut self,
        hits: &[SearchHit],
        dest_dir: &Path,
    ) -> Vec<Result<PathBuf>> {
        let mut results = Vec::with_capacity(hits.len());
        for hit in hits {
            results.push(self.download(hit, dest_dir, None).await);
        }
        results
    }

    // -----------------------------------------------------------------------
    // Peer resolution + connection
    // -----------------------------------------------------------------------

    /// Ask the server for a peer's IP:port, waiting for the PeerAddress reply.
    pub async fn resolve_peer(&mut self, username: &str) -> Result<SocketAddr> {
        let msg = MsgBuilder::server(ServerCode::GetPeerAddress as u32)
            .str(username)
            .build();
        self.conn.send(&msg).await?;
        debug!("→ GetPeerAddress {username}");

        // Drain messages until we get the address reply for this user.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(SlskError::Timeout);
            }
            match tokio::time::timeout(remaining, self.conn.recv()).await {
                Ok(Ok(ServerMsg::PeerAddress(pa))) if pa.username == username => {
                    if pa.ip == Ipv4Addr::UNSPECIFIED || pa.port == 0 {
                        return Err(SlskError::PeerUnreachable);
                    }
                    let addr = SocketAddr::new(pa.ip.into(), pa.port);
                    debug!("Resolved {username} → {addr}");
                    return Ok(addr);
                }
                Ok(Ok(_)) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(SlskError::Timeout),
            }
        }
    }

    /// Open and handshake a peer connection.
    ///
    /// If direct connect fails, we attempt NAT pierce via the server's
    /// ConnectToPeer relay.
    pub async fn connect_peer(
        &mut self,
        addr: SocketAddr,
        username: &str,
        conn_type: &str,
        token: u32,
        server_token: Option<u32>,
    ) -> Result<PeerConn> {
        // Try direct connection first.
        match PeerConn::connect_and_init(addr, &self.username, conn_type, token, server_token)
            .await
        {
            Ok(peer) => return Ok(peer),
            Err(e) => {
                warn!("Direct peer connect to {addr} failed ({e}), trying NAT pierce");
            }
        }

        // NAT pierce: ask the server to tell the remote peer to connect to us.
        // Then we wait for ConnectToPeer from the server with the peer's address.
        let _pierce_token = self.token();
        let msg = MsgBuilder::server(ServerCode::GetPeerAddress as u32)
            .str(username)
            .build();
        self.conn.send(&msg).await?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(SlskError::PeerUnreachable);
            }
            match tokio::time::timeout(remaining, self.conn.recv()).await {
                Ok(Ok(ServerMsg::ConnectToPeer {
                    username: u,
                    conn_type: ct,
                    ip,
                    port,
                    token: st,
                    ..
                })) if u == username && ct == conn_type => {
                    let peer_addr = SocketAddr::new(ip.into(), port);
                    return PeerConn::connect_and_init(
                        peer_addr,
                        &self.username,
                        conn_type,
                        token,
                        Some(st),
                    )
                    .await;
                }
                Ok(Ok(ServerMsg::CantConnectToPeer { username: u, .. })) if u == username => {
                    return Err(SlskError::PeerUnreachable);
                }
                Ok(Ok(_)) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(SlskError::Timeout),
            }
        }
    }

    // -----------------------------------------------------------------------
    // Misc server messages
    // -----------------------------------------------------------------------

    /// Send a ping (keeps connection alive; server echoes it back).
    pub async fn ping(&mut self) -> Result<()> {
        self.conn
            .send(&MsgBuilder::server(ServerCode::Ping as u32).build())
            .await
    }

    /// Request the full room list.
    pub async fn room_list(&mut self) -> Result<Vec<(String, u32)>> {
        self.conn
            .send(&MsgBuilder::server(ServerCode::RoomList as u32).build())
            .await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(SlskError::Timeout);
            }
            match tokio::time::timeout(remaining, self.conn.recv()).await {
                Ok(Ok(ServerMsg::RoomList(rooms))) => return Ok(rooms),
                Ok(Ok(_)) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(SlskError::Timeout),
            }
        }
    }

    /// Read the next raw server message. Useful if you want to drive the event
    /// loop yourself (e.g. in a select! with other futures).
    pub async fn next_event(&mut self) -> Result<ServerMsg> {
        self.conn.recv().await
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn token(&mut self) -> u32 {
        let t = self.next_token;
        self.next_token = self.next_token.wrapping_add(1);
        t
    }

    async fn send_set_wait_port(&mut self, port: u16) -> Result<()> {
        self.conn
            .send(
                &MsgBuilder::server(ServerCode::SetWaitPort as u32)
                    .u32(port as u32)
                    .build(),
            )
            .await
    }

    async fn send_status(&mut self, status: u32) -> Result<()> {
        self.conn
            .send(
                &MsgBuilder::server(ServerCode::SetStatus as u32)
                    .u32(status)
                    .build(),
            )
            .await
    }

    async fn send_shared_counts(&mut self, folders: u32, files: u32) -> Result<()> {
        self.conn
            .send(
                &MsgBuilder::server(ServerCode::SharedFoldersFiles as u32)
                    .u32(folders)
                    .u32(files)
                    .build(),
            )
            .await
    }
}
