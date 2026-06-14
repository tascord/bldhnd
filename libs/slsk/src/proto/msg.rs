//! Server ↔ client message definitions and parsing.
//!
//! Reference: https://nicotine-plus.org/doc/SLSKPROTOCOL.html  (server section)

use flate2::read::ZlibDecoder;
use std::io::Read;
use tracing::{debug, warn};

use crate::error::{Result, SlskError};
use super::frame::MsgReader;

// ---------------------------------------------------------------------------
// Outbound message codes (client → server)
// ---------------------------------------------------------------------------

#[repr(u32)]
#[allow(dead_code)]
pub enum ServerCode {
    Login               = 1,
    SetWaitPort         = 2,
    GetPeerAddress      = 3,
    AddUser             = 5,
    GetUserStatus       = 7,
    FileSearch          = 26,
    SetStatus           = 28,
    Ping                = 32,
    SharedFoldersFiles  = 35,
    GetUserStats        = 36,
    UserSearch          = 42,
    JoinRoom            = 14,
    LeaveRoom           = 15,
    RoomList            = 64,
    PrivilegedUsers     = 69,
    HaveNoParent        = 71,  // distributed network
    BranchLevel         = 126,
    BranchRoot          = 127,
    ChildDepth          = 129,
    ResetDistributed    = 130,
}

// ---------------------------------------------------------------------------
// Inbound message types (server → client)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FileResult {
    pub filename: String,
    pub size: u64,
    pub ext: String,
    /// Attribute slots: [bitrate, length_secs, vbr_flag, …]
    pub attrs: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub username: String,
    pub token: u32,
    pub results: Vec<FileResult>,
    pub free_upload_slots: bool,
    pub upload_speed: u32,
    pub in_queue: u32,
}

#[derive(Debug, Clone)]
pub struct UserStats {
    pub username: String,
    pub avg_speed: u32,
    pub upload_num: u32,
    pub files: u32,
    pub dirs: u32,
}

#[derive(Debug, Clone)]
pub struct PeerAddress {
    pub username: String,
    pub ip: std::net::Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct UserStatus {
    pub username: String,
    /// 0 = offline, 1 = away, 2 = online
    pub status: u32,
    pub privileged: bool,
}

#[derive(Debug)]
pub enum ServerMsg {
    /// Successful login.
    LoginOk { greet: String, ip: u32 },
    /// Failed login with reason string.
    LoginFail { reason: String },
    /// Peer address resolved (response to GetPeerAddress).
    PeerAddress(PeerAddress),
    /// Search results from a remote peer (relayed via server).
    SearchResults(SearchResults),
    /// Full room list.
    RoomList(Vec<(String, u32)>),
    /// User stats.
    UserStats(UserStats),
    /// User status update.
    UserStatus(UserStatus),
    /// ConnectToPeer: server asks us to initiate a peer connection.
    ConnectToPeer {
        username: String,
        conn_type: String,
        ip: std::net::Ipv4Addr,
        port: u16,
        token: u32,
        privileged: bool,
    },
    /// CantConnectToPeer: peer is unreachable.
    CantConnectToPeer { token: u32, username: String },
    /// Privileged users list.
    PrivilegedUsers(Vec<String>),
    /// Ping (server echoes pings back).
    Ping,
    /// Any message we don't handle yet.
    Unknown { code: u32, len: usize },
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

pub fn parse_server_msg(code: u32, payload: &[u8]) -> Result<ServerMsg> {
    debug!("← server code={code} len={}", payload.len());
    let mut r = MsgReader::new(payload);

    match code {
        // Login response
        1 => {
            let success = r.read_bool()?;
            if success {
                let greet = r.read_str()?;
                let ip    = r.read_u32()?;
                Ok(ServerMsg::LoginOk { greet, ip })
            } else {
                let reason = r.read_str()?;
                Ok(ServerMsg::LoginFail { reason })
            }
        }

        // GetPeerAddress response
        3 => {
            let username = r.read_str()?;
            let ip_raw   = r.read_u32()?;
            let port     = r.read_u32()? as u16;
            let ip       = std::net::Ipv4Addr::from(ip_raw);
            Ok(ServerMsg::PeerAddress(PeerAddress { username, ip, port }))
        }

        // GetUserStatus response
        7 => {
            let username  = r.read_str()?;
            let status    = r.read_u32()?;
            let privileged = r.read_bool().unwrap_or(false);
            Ok(ServerMsg::UserStatus(UserStatus { username, status, privileged }))
        }

        // ConnectToPeer
        18 => {
            let username  = r.read_str()?;
            let conn_type = r.read_str()?;
            let ip_raw    = r.read_u32()?;
            let port      = r.read_u32()? as u16;
            let token     = r.read_u32()?;
            let privileged = r.read_bool().unwrap_or(false);
            let ip = std::net::Ipv4Addr::from(ip_raw);
            Ok(ServerMsg::ConnectToPeer { username, conn_type, ip, port, token, privileged })
        }

        // Ping
        32 => Ok(ServerMsg::Ping),

        // GetUserStats response
        36 => {
            let username    = r.read_str()?;
            let avg_speed   = r.read_u32()?;
            let upload_num  = r.read_u32()?;
            let files       = r.read_u32()?;
            let dirs        = r.read_u32()?;
            Ok(ServerMsg::UserStats(UserStats { username, avg_speed, upload_num, files, dirs }))
        }

        // FileSearchResult — payload is zlib-compressed after the header fields
        44 => parse_search_results(&mut r),

        // RoomList
        64 => {
            let num_rooms = r.read_u32()?;
            let mut rooms: Vec<(String, u32)> = (0..num_rooms)
                .map(|_| Ok((r.read_str()?, 0u32)))
                .collect::<Result<_>>()?;
            let num_counts = r.read_u32().unwrap_or(0);
            for i in 0..num_counts as usize {
                if i < rooms.len() {
                    rooms[i].1 = r.read_u32().unwrap_or(0);
                }
            }
            Ok(ServerMsg::RoomList(rooms))
        }

        // PrivilegedUsers
        69 => {
            let n = r.read_u32()?;
            let users = (0..n).map(|_| r.read_str()).collect::<Result<_>>()?;
            Ok(ServerMsg::PrivilegedUsers(users))
        }

        // CantConnectToPeer
        1001 => {
            let token    = r.read_u32()?;
            let username = r.read_str().unwrap_or_default();
            Ok(ServerMsg::CantConnectToPeer { token, username })
        }

        _ => {
            if !payload.is_empty() {
                warn!("Unhandled server code={code} len={}", payload.len());
            }
            Ok(ServerMsg::Unknown { code, len: payload.len() })
        }
    }
}

fn parse_search_results(r: &mut MsgReader<'_>) -> Result<ServerMsg> {
    let username = r.read_str()?;
    let token    = r.read_u32()?;

    // The rest of the payload is zlib-compressed.
    let compressed = r.read_remaining();
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw).map_err(|e| {
        SlskError::Protocol(format!("zlib decompress failed: {e}"))
    })?;

    let mut r2 = MsgReader::new(&raw);
    let num_results = r2.read_u32()?;
    let mut results = Vec::with_capacity(num_results as usize);

    for _ in 0..num_results {
        let _attr_code = r2.read_u8()?; // always 1
        let filename   = r2.read_str()?;
        let size       = r2.read_u64()?;
        let ext        = r2.read_str()?;
        let num_attrs  = r2.read_u32()?;
        let mut attrs  = Vec::with_capacity(num_attrs as usize);
        for _ in 0..num_attrs {
            let _attr_type = r2.read_u32()?;
            attrs.push(r2.read_u32()?);
        }
        results.push(FileResult { filename, size, ext, attrs });
    }

    let free_upload_slots = r2.read_bool().unwrap_or(false);
    let upload_speed      = r2.read_u32().unwrap_or(0);
    let in_queue          = r2.read_u32().unwrap_or(0);

    Ok(ServerMsg::SearchResults(SearchResults {
        username, token, results, free_upload_slots, upload_speed, in_queue,
    }))
}
