//! Low-level binary framing helpers used by both server and peer connections.
//!
//! Soulseek wire format:
//!   Server messages:  [u32le total_len][u32le code][payload…]
//!   Peer   messages:  [u32le total_len][u8 code][payload…]  (1-byte code!)
//!
//! Strings:  [u32le byte_len][utf-8 bytes]
//! Integers: u32 little-endian unless noted

use bytes::{Buf, BufMut, BytesMut};
use crate::error::{SlskError, Result};

// ---------------------------------------------------------------------------
// MsgBuilder
// ---------------------------------------------------------------------------

pub struct MsgBuilder {
    buf: BytesMut,
}

impl MsgBuilder {
    /// Start a server-style message with a u32 code.
    pub fn server(code: u32) -> Self {
        let mut buf = BytesMut::with_capacity(64);
        buf.put_u32_le(0); // placeholder for length
        buf.put_u32_le(code);
        MsgBuilder { buf }
    }

    /// Start a peer-style message with a u8 code.
    pub fn peer(code: u8) -> Self {
        let mut buf = BytesMut::with_capacity(64);
        buf.put_u32_le(0); // placeholder for length
        buf.put_u8(code);
        MsgBuilder { buf }
    }

    pub fn u32(mut self, v: u32) -> Self {
        self.buf.put_u32_le(v);
        self
    }

    pub fn u64(mut self, v: u64) -> Self {
        self.buf.put_u32_le(v as u32);
        self.buf.put_u32_le((v >> 32) as u32);
        self
    }

    pub fn u8(mut self, v: u8) -> Self {
        self.buf.put_u8(v);
        self
    }

    pub fn str(mut self, s: &str) -> Self {
        self.buf.put_u32_le(s.len() as u32);
        self.buf.put_slice(s.as_bytes());
        self
    }

    pub fn bytes(mut self, b: &[u8]) -> Self {
        self.buf.put_slice(b);
        self
    }

    /// Seal: backfill the length prefix and return the wire bytes.
    pub fn build(mut self) -> Vec<u8> {
        let payload_len = (self.buf.len() - 4) as u32;
        self.buf[0..4].copy_from_slice(&payload_len.to_le_bytes());
        self.buf.to_vec()
    }
}

// ---------------------------------------------------------------------------
// MsgReader
// ---------------------------------------------------------------------------

pub struct MsgReader<'a> {
    pub(crate) buf: &'a [u8],
}

impl<'a> MsgReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        MsgReader { buf }
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        if self.buf.is_empty() {
            return Err(SlskError::Protocol("unexpected EOF reading u8".into()));
        }
        let v = self.buf[0];
        self.buf.advance(1);
        Ok(v)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        if self.buf.len() < 4 {
            return Err(SlskError::Protocol("unexpected EOF reading u32".into()));
        }
        let v = u32::from_le_bytes(self.buf[..4].try_into().unwrap());
        self.buf.advance(4);
        Ok(v)
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let lo = self.read_u32()? as u64;
        let hi = self.read_u32()? as u64;
        Ok(lo | (hi << 32))
    }

    pub fn read_str(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        if self.buf.len() < len {
            return Err(SlskError::Protocol(format!(
                "unexpected EOF reading string (need {len}, have {})",
                self.buf.len()
            )));
        }
        let s = std::str::from_utf8(&self.buf[..len])
            .map_err(|e| SlskError::Protocol(format!("invalid utf8: {e}")))?
            .to_owned();
        self.buf.advance(len);
        Ok(s)
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    /// Read all remaining bytes.
    pub fn read_remaining(&mut self) -> Vec<u8> {
        let v = self.buf.to_vec();
        self.buf = &[];
        v
    }

    pub fn remaining(&self) -> usize {
        self.buf.len()
    }
}

// ---------------------------------------------------------------------------
// Read a complete framed message from a raw byte buffer.
// Returns (code, payload) if a full message is available, and advances buf.
// ---------------------------------------------------------------------------

/// Try to extract one server-style message (u32 code) from `buf`.
/// Returns `None` if not enough data yet.
pub fn try_read_server_frame(buf: &mut BytesMut) -> Option<(u32, bytes::Bytes)> {
    if buf.len() < 8 {
        return None;
    }
    let payload_len = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
    let total = 4 + payload_len;
    if buf.len() < total {
        return None;
    }
    let mut frame = buf.split_to(total);
    frame.advance(4); // skip length
    let code = frame.get_u32_le();
    Some((code, frame.freeze()))
}

/// Try to extract one peer-style message (u8 code) from `buf`.
pub fn try_read_peer_frame(buf: &mut BytesMut) -> Option<(u8, bytes::Bytes)> {
    if buf.len() < 5 {
        return None;
    }
    let payload_len = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
    let total = 4 + payload_len;
    if buf.len() < total {
        return None;
    }
    let mut frame = buf.split_to(total);
    frame.advance(4); // skip length
    let code = frame.get_u8();
    Some((code, frame.freeze()))
}
