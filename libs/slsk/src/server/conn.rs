//! Raw TCP connection to the Soulseek server.
//!
//! `ServerConn` handles framing and exposes:
//!   - `send(bytes)` — write a framed message
//!   - `recv()` — read and parse the next `ServerMsg`
//!
//! The higher-level `SlskClient` wraps this and runs the dispatch loop.

use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::error::{Result, SlskError};
use crate::proto::frame::try_read_server_frame;
use crate::proto::msg::{parse_server_msg, ServerMsg};

pub struct ServerConn {
    stream: TcpStream,
    buf: BytesMut,
}

impl ServerConn {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream, buf: BytesMut::with_capacity(32 * 1024) })
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).await?;
        Ok(())
    }

    /// Read the next complete message from the server, blocking until available.
    pub async fn recv(&mut self) -> Result<ServerMsg> {
        loop {
            if let Some((code, payload)) = try_read_server_frame(&mut self.buf) {
                return parse_server_msg(code, &payload);
            }
            let mut tmp = [0u8; 8192];
            let n = self.stream.read(&mut tmp).await?;
            if n == 0 {
                return Err(SlskError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "server closed connection",
                )));
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
    }
}