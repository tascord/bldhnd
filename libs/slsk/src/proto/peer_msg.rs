//! Peer ↔ peer message protocol.
//!
//! Peer connections use a *different* framing to server connections:
//!   [u32le length][u8 code][payload…]
//!
//! Handshake sequence (initiator side):
//!   1. PierceFirewall(token)        — if we're piercing NAT
//!   2. PeerInit(username, type, token) — "P" = file transfer, "F" = file
//!
//! After handshake, for file downloads:
//!   → TransferRequest(direction=1, token, filename)
//!   ← TransferResponse(token, ok, filesize / reason)
//!   → Offset(0)                    — resume offset
//!   ← [raw bytes]                  — file data stream

use crate::error::{Result};
use super::frame::MsgReader;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Outbound peer message codes
// ---------------------------------------------------------------------------

#[repr(u8)]
pub enum PeerCode {
    PierceFirewall   = 0,
    PeerInit         = 1,
    SharesRequest    = 4,
    SharesReply      = 5,
    SearchRequest    = 8,
    SearchReply      = 9,
    InfoRequest      = 15,
    InfoReply        = 16,
    FolderContents   = 36,
    TransferRequest  = 40,
    TransferResponse = 41,
    UploadPlaceholder= 42,
    QueueDownload    = 43,
    PlaceInQueue     = 44,
    UploadFailed     = 46,
    QueueFailed      = 50,
    Offset           = 65,
}

// ---------------------------------------------------------------------------
// Inbound peer message types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum PeerMsg {
    /// Initial handshake from the remote peer (they are initiating to us).
    PierceFirewall { token: u32 },
    /// Peer identifies itself and the connection type.
    PeerInit {
        username: String,
        conn_type: String, // "P" = peer, "F" = file, "D" = distributed
        token: u32,
    },
    /// Remote peer wants to send us a file or receive one.
    TransferRequest {
        /// 0 = peer wants to download from us, 1 = peer wants to upload to us
        direction: u32,
        token: u32,
        filename: String,
        filesize: Option<u64>,
    },
    /// Response to our TransferRequest.
    TransferResponse {
        token: u32,
        allowed: bool,
        filesize: u64,
        reason: String,
    },
    /// File data offset for resume (always 0 for fresh downloads).
    Offset { offset: u64 },
    /// Place in upload queue.
    PlaceInQueue { filename: String, place: u32 },
    /// Upload failed.
    UploadFailed { filename: String },
    /// Queue failed.
    QueueFailed { filename: String, reason: String },
    Unknown { code: u8, len: usize },
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

pub fn parse_peer_msg(code: u8, payload: &[u8]) -> Result<PeerMsg> {
    debug!("← peer code={code} len={}", payload.len());
    let mut r = MsgReader::new(payload);

    match code {
        0 => Ok(PeerMsg::PierceFirewall { token: r.read_u32()? }),

        1 => Ok(PeerMsg::PeerInit {
            username:  r.read_str()?,
            conn_type: r.read_str()?,
            token:     r.read_u32()?,
        }),

        40 => {
            let direction = r.read_u32()?;
            let token     = r.read_u32()?;
            let filename  = r.read_str()?;
            let filesize  = if direction == 1 { Some(r.read_u64().unwrap_or(0)) } else { None };
            Ok(PeerMsg::TransferRequest { direction, token, filename, filesize })
        }

        41 => {
            let token   = r.read_u32()?;
            let allowed = r.read_bool()?;
            let filesize = if allowed { r.read_u64().unwrap_or(0) } else { 0 };
            let reason   = if !allowed { r.read_str().unwrap_or_default() } else { String::new() };
            Ok(PeerMsg::TransferResponse { token, allowed, filesize, reason })
        }

        44 => Ok(PeerMsg::PlaceInQueue {
            filename: r.read_str()?,
            place:    r.read_u32()?,
        }),

        46 => Ok(PeerMsg::UploadFailed { filename: r.read_str()? }),

        50 => Ok(PeerMsg::QueueFailed {
            filename: r.read_str()?,
            reason:   r.read_str()?,
        }),

        65 => Ok(PeerMsg::Offset { offset: r.read_u64()? }),

        _ => {
            warn!("Unhandled peer code={code} len={}", payload.len());
            Ok(PeerMsg::Unknown { code, len: payload.len() })
        }
    }
}
