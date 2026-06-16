//! Soulseek client library.
//!
//! Public API surface:
//!   - [`SlskClient`] — connect, login, search, download
//!   - [`SearchHit`]  — a single result from a search
//!   - [`error`]      — error types

pub mod client;
pub mod error;
pub mod peer;
pub mod proto;
pub mod server;

pub use {
    client::{Config, SearchHit, SlskClient},
    error::SlskError,
};
