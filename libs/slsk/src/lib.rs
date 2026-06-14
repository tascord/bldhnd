//! Soulseek client library.
//!
//! Public API surface:
//!   - [`SlskClient`] — connect, login, search, download
//!   - [`SearchHit`]  — a single result from a search
//!   - [`error`]      — error types

pub mod error;
pub mod proto;
pub mod server;
pub mod peer;
pub mod client;

pub use client::{SlskClient, SearchHit, Config};
pub use error::SlskError;
