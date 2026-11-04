//! The self-hostable Curvyo relay (ADR 0031 decision 11): a WebSocket and file-store shell
//! around the sans-I/O relay of `curvyo-sync-core`.
//!
//! It decrypts nothing and parses no document: it stores the sealed envelopes and keyring
//! entries the relay rules accept, in one data directory, and passes frames between the devices
//! of a room. The library exists so the integration tests can start the server in-process; the
//! binary in `main.rs` only reads its configuration and waits for SIGTERM or SIGINT.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod backup_file;
mod config;
mod health;
mod hub;
mod log;
mod room_scan;
mod server;
mod snapshot_file;
mod store;

pub use config::{Config, ConfigError, DEFAULT_DATA, DEFAULT_LISTEN, USAGE};
pub use hub::HubError;
pub use log::Log;
pub use server::{Server, ServerError};
pub use store::{StoreError, room_hex};
