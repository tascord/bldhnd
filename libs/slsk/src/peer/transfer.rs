//! File download over an established peer "F"-type connection.
//!
//! Protocol after handshake on an "F" connection:
//!   → TransferRequest(direction=0, token, filename)
//!   ← TransferResponse(token, allowed=true, filesize)
//!   → Offset(resume_offset)          — 0 for a fresh download
//!   ← [raw file bytes …]             — until filesize bytes received
//!
//! If allowed=false, TransferResponse carries a reason string instead.

use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, info};

use crate::error::{Result, SlskError};
use crate::proto::frame::MsgBuilder;
use crate::proto::peer_msg::{PeerMsg, PeerCode};
use super::conn::PeerConn;

/// Download progress callback type: `(bytes_done, total_bytes)`.
pub type ProgressFn = Box<dyn Fn(u64, u64) + Send + 'static>;

/// Perform a file download over an already-handshaked peer connection.
///
/// Consumes `peer` because after the transfer handshake the socket carries
/// raw bytes, not framed messages.
pub async fn download_file(
    mut peer: PeerConn,
    token: u32,
    filename: &str,
    dest: &Path,
    resume_offset: u64,
    progress: Option<ProgressFn>,
) -> Result<u64> {
    // --- Step 1: send TransferRequest (direction=0 = we want to download) ---
    let req = MsgBuilder::peer(PeerCode::TransferRequest as u8)
        .u32(0)           // direction: 0 = download
        .u32(token)
        .str(filename)
        .build();
    peer.send_raw(&req).await?;
    debug!("→ TransferRequest token={token} file={filename:?}");

    // --- Step 2: read TransferResponse ---
    let resp = peer.recv().await?;
    let filesize = match resp {
        PeerMsg::TransferResponse { token: rt, allowed: true, filesize, .. } if rt == token => {
            debug!("← TransferResponse allowed size={filesize}");
            filesize
        }
        PeerMsg::TransferResponse { allowed: false, reason, .. } => {
            return Err(SlskError::TransferRefused(reason));
        }
        PeerMsg::QueueFailed { reason, .. } => {
            return Err(SlskError::TransferRefused(reason));
        }
        other => {
            return Err(SlskError::Protocol(format!(
                "expected TransferResponse, got {other:?}"
            )));
        }
    };

    // --- Step 3: send Offset (resume position) ---
    let offset_msg = MsgBuilder::peer(PeerCode::Offset as u8)
        .u64(resume_offset)
        .build();
    peer.send_raw(&offset_msg).await?;
    debug!("→ Offset {resume_offset}");

    // --- Step 4: receive raw file bytes ---
    // Any bytes already sitting in the peer's read buffer belong to the file.
    let buffered = peer.drain_buf();
    let stream: TcpStream = peer.into_stream();

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = if resume_offset > 0 {
        tokio::fs::OpenOptions::new()
            .create(true).append(true)
            .open(dest).await?
    } else {
        tokio::fs::File::create(dest).await?
    };

    let remaining_size = filesize.saturating_sub(resume_offset);
    let mut received: u64 = 0;

    // Write what was already buffered.
    if !buffered.is_empty() {
        let to_write = (buffered.len() as u64).min(remaining_size - received) as usize;
        file.write_all(&buffered[..to_write]).await?;
        received += to_write as u64;
        if let Some(ref cb) = progress {
            cb(received, remaining_size);
        }
    }

    // Stream the rest directly from the socket.
    let (mut reader, _writer) = tokio::io::split(stream);
    let mut chunk = vec![0u8; 32 * 1024];

    while received < remaining_size {
        let want = ((remaining_size - received) as usize).min(chunk.len());
        let n = reader.read(&mut chunk[..want]).await?;
        if n == 0 {
            return Err(SlskError::TransferFailed(format!(
                "peer disconnected after {received}/{remaining_size} bytes"
            )));
        }
        file.write_all(&chunk[..n]).await?;
        received += n as u64;
        if let Some(ref cb) = progress {
            cb(received, remaining_size);
        }
    }

    file.flush().await?;
    info!("Download complete: {received} bytes → {}", dest.display());
    Ok(received)
}