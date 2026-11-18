//! The machine connector's transport (ADR 0035): TLS 1.3 with a pinned key, pairing by SPAKE2, the
//! connector server with a fake controller, and the client the desktop app uses. Native only; the
//! protocol itself is `curvyo-connector-core`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod client;
pub mod clients;
pub mod fake_controller;
pub mod frames;
pub mod hex;
pub mod hub;
pub mod identity;
pub mod job_log;
pub mod job_send;
pub mod pairing;
pub mod pairing_code;
pub mod pin_verifier;
pub mod record;
pub mod send_tracked;
pub mod server;
pub mod server_tls;
mod session;
pub mod spake;
pub mod spki;
pub mod status_limit;
pub mod token;
pub mod watch;

pub use uuid::Uuid;
