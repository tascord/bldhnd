//! Peer TCP connection.
//!
//! Soulseek peer connections follow this handshake:
//!
//! Initiator (us):
//!   → PierceFirewall(server_token)   [if server told us to connect]
//!   → PeerInit(username, type, token)
//!
//! Or if the remote peer is connecting to us (on our listen port):
//!   ← PierceFirewall(token)          [they pierce our firewall]
//!   ← PeerInit(username, type, token)
//!
//! After the handshake the connection type ("P", "F", "D") determines what
//! messages follow. For file downloads we use type "F".

use bytes::BytesMut;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

use crate::error::{Result, SlskError};
use crate::proto::frame::{try_read_peer_frame, MsgBuilder};
use crate::proto::peer_msg::{parse_peer_msg, PeerMsg};

pub struct PeerConn {
    pub(crate) stream: TcpStream,
    buf: BytesMut,
    /// Username of the remote peer (filled in after handshake).
    pub username: String,
    /// Connection type: "P", "F", or "D".
    pub conn_type: String,
}

impl PeerConn {
    // -----------------------------------------------------------------------
    // Connect and perform initiator-side handshake
    // -----------------------------------------------------------------------

    /// Connect to a peer and send PierceFirewall + PeerInit.
    pub async fn connect_and_init(
        addr: SocketAddr,
        our_username: &str,
        conn_type: &str,
        token: u32,
        server_token: Option<u32>, // Some(t) → send PierceFirewall first
    ) -> Result<Self> {
        debug!("Connecting to peer {addr} (type={conn_type})");
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|_| SlskError::Timeout)?
        .map_err(|e| SlskError::Io(e))?;

        let mut c = PeerConn {
            stream,
            buf: BytesMut::with_capacity(16 * 1024),
            username: String::new(),
            conn_type: conn_type.to_owned(),
        };

        // Optionally pierce the remote's firewall first.
        if let Some(st) = server_token {
            let msg = MsgBuilder::peer(0).u32(st).build();
            c.send_raw(&msg).await?;
        }

        // Send PeerInit.
        let msg = MsgBuilder::peer(1)
            .str(our_username)
            .str(conn_type)
            .u32(token)
            .build();
        c.send_raw(&msg).await?;

        c.username = our_username.to_owned(); // will be overwritten if we receive PeerInit
        Ok(c)
    }

    // -----------------------------------------------------------------------
    // Accept-side: wrap an already-connected socket and read the handshake
    // -----------------------------------------------------------------------

    pub async fn accept(stream: TcpStream) -> Result<Self> {
        let mut c = PeerConn {
            stream,
            buf: BytesMut::with_capacity(16 * 1024),
            username: String::new(),
            conn_type: String::new(),
        };

        // Read the first message; it's either PierceFirewall or PeerInit.
        let first = c.recv().await?;
        match first {
            PeerMsg::PeerInit { username, conn_type, .. } => {
                c.username  = username;
                c.conn_type = conn_type;
            }
            PeerMsg::PierceFirewall { .. } => {
                // After PierceFirewall we expect PeerInit.
                let second = c.recv().await?;
                match second {
                    PeerMsg::PeerInit { username, conn_type, .. } => {
                        c.username  = username;
                        c.conn_type = conn_type;
                    }
                    other => {
                        return Err(SlskError::PeerHandshake(format!(
                            "expected PeerInit after PierceFirewall, got {other:?}"
                        )));
                    }
                }
            }
            other => {
                return Err(SlskError::PeerHandshake(format!(
                    "unexpected first peer message: {other:?}"
                )));
            }
        }

        Ok(c)
    }

    // -----------------------------------------------------------------------
    // Send / receive
    // -----------------------------------------------------------------------

    pub async fn send_raw(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<PeerMsg> {
        loop {
            if let Some((code, payload)) = try_read_peer_frame(&mut self.buf) {
                return parse_peer_msg(code, &payload);
            }
            let mut tmp = [0u8; 8192];
            let n = self.stream.read(&mut tmp).await?;
            if n == 0 {
                return Err(SlskError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "peer closed connection",
                )));
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
    }

    /// Drain any remaining bytes already in the read buffer (used to consume
    /// raw file data after the transfer handshake).
    pub fn drain_buf(&mut self) -> bytes::Bytes {
        self.buf.split().freeze()
    }

    pub fn into_stream(self) -> TcpStream {
        self.stream
    }
}