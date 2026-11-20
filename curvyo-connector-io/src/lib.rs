//! The machine connector's transport (ADR 0035): TLS 1.3 with a pinned key, pairing by SPAKE2, the
//! connector server with the fake controller or GRBL over serial, and the client the desktop app
//! uses. Native only; the protocol itself is `curvyo-connector-core`.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::assert_is_empty)
)]

pub mod admin_log;
pub mod client;
pub mod clients;
pub mod config_file;
pub mod controller;
pub mod fake_controller;
pub mod frames;
pub mod grbl;
pub mod hex;
pub mod hub;
pub mod hub_log;
pub mod identity;
pub mod job_log;
pub mod job_send;
pub mod log_tail;
pub mod maintenance;
pub mod pairing;
pub mod pairing_code;
pub mod pin_verifier;
pub mod ports;
pub mod record;
pub mod send_tracked;
#[cfg(feature = "serial")]
pub mod serial_link;
pub mod server;
pub mod server_tls;
mod session;
pub mod spake;
pub mod spki;
pub mod state_dir;
pub mod status_limit;
mod supervisor;
pub mod token;
pub mod watch;

pub use uuid::Uuid;
