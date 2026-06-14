use std::io;

#[derive(Debug, thiserror::Error)]
pub enum SlskError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Login failed: {0}")]
    LoginFailed(String),

    #[error("Peer handshake failed: {0}")]
    PeerHandshake(String),

    #[error("Transfer refused: {0}")]
    TransferRefused(String),

    #[error("Transfer failed: {0}")]
    TransferFailed(String),

    #[error("Peer unreachable")]
    PeerUnreachable,

    #[error("Timeout")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, SlskError>;
