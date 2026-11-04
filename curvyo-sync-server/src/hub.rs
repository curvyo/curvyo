//! The single owner of the relay's state, applying every connection's messages to the sans-I/O
//! [`Relay`] or, for a pairing connection, to its [`PairSlots`] in arrival order with each store
//! write on disk before the sends that follow it (ADR 0031 decisions 5 and 11, ADR 0040
//! decision 3).
//!
//! A connection whose first message is `PairOpen` or `PairJoin` is a pairing connection for its
//! whole life; its slots live in memory only and nothing about them is logged. One whose first
//! message is `BackupGet` or `BackupBegin` is a backup connection (ADR 0040 decision 13): one
//! request, one answer.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use curvyo_sync_core::{Backups, ClientFrame, ConnectionId, Millis, PairSlots, Relay, RelayOutput};
use tokio::sync::{mpsc, oneshot};

use crate::store::{Store, StoreError};

/// What a connection's task tells the hub.
pub(crate) enum ToHub {
    /// A WebSocket was accepted: register it and answer with its number.
    Open {
        /// Where the hub sends this connection's outgoing messages.
        outgoing: mpsc::UnboundedSender<Outgoing>,
        /// The connection's number.
        reply: oneshot::Sender<ConnectionId>,
    },
    /// A binary message arrived.
    Message {
        /// From which connection.
        connection: ConnectionId,
        /// Its bytes.
        bytes: Vec<u8>,
    },
    /// The socket closed.
    Gone {
        /// Which connection.
        connection: ConnectionId,
    },
    /// A second passed: pairing slots whose lifetime ended are refused as expired.
    Tick,
}

/// What the hub tells a connection's task.
#[derive(Debug)]
pub(crate) enum Outgoing {
    /// Send one binary message.
    Frame(Vec<u8>),
    /// Close the socket (after the frames before it).
    Close,
}

/// Why the hub stopped before it was asked to.
#[derive(Debug, thiserror::Error)]
pub enum HubError {
    /// A store write failed; the relay's memory is ahead of its disk, so it stops (fail-stop)
    /// and the next start restores from what is on disk. Unacknowledged pushes are resent by
    /// their clients.
    #[error("a store write failed: {0}")]
    Store(#[from] StoreError),
    /// The operating system gave no entropy for a challenge nonce.
    #[error("no entropy for a challenge nonce: {0}")]
    Entropy(getrandom::Error),
}

/// The relay, its store and the open connections.
pub(crate) struct Hub {
    relay: Relay,
    pairing: PairSlots,
    backups: Backups,
    store: Store,
    connections: BTreeMap<ConnectionId, mpsc::UnboundedSender<Outgoing>>,
    /// Connections that have sent nothing yet.
    fresh: BTreeSet<ConnectionId>,
    /// Pairing connections.
    pairs: BTreeSet<ConnectionId>,
    /// Backup connections.
    backup_connections: BTreeSet<ConnectionId>,
}

/// What a connection is, by its first message.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Room,
    Pairing,
    Backup,
}

impl Hub {
    pub(crate) fn new(relay: Relay, backups: Backups, store: Store) -> Self {
        Self {
            relay,
            pairing: PairSlots::new(),
            backups,
            store,
            connections: BTreeMap::new(),
            fresh: BTreeSet::new(),
            pairs: BTreeSet::new(),
            backup_connections: BTreeSet::new(),
        }
    }

    /// Handles messages until every sender is gone (all sockets closed and the server stopped),
    /// so every message received before the shutdown is stored. Blocking: run it on its own
    /// thread.
    pub(crate) fn run(mut self, mut inbox: mpsc::UnboundedReceiver<ToHub>) -> Result<(), HubError> {
        while let Some(message) = inbox.blocking_recv() {
            self.handle(message)?;
        }
        Ok(())
    }

    fn handle(&mut self, message: ToHub) -> Result<(), HubError> {
        match message {
            ToHub::Open { outgoing, reply } => {
                let connection = self.relay.connect();
                self.connections.insert(connection, outgoing);
                self.fresh.insert(connection);
                if reply.send(connection).is_err() {
                    self.gone(connection)?;
                }
            }
            ToHub::Message { connection, bytes } => {
                if self.fresh.remove(&connection) {
                    let route = route(&bytes);
                    if route != Route::Room {
                        // The room relay forgets it; the pairing slots or the backups take it.
                        let out = self.relay.disconnect(connection);
                        self.carry_out(out)?;
                    }
                    match route {
                        Route::Room => {}
                        Route::Pairing => {
                            self.pairs.insert(connection);
                        }
                        Route::Backup => {
                            self.backup_connections.insert(connection);
                        }
                    }
                }
                let mut entropy = [0; 32];
                getrandom::fill(&mut entropy).map_err(HubError::Entropy)?;
                let out = if self.pairs.contains(&connection) {
                    self.pairing.receive(connection, &bytes, now(), entropy)
                } else if self.backup_connections.contains(&connection) {
                    self.backups.receive(connection, &bytes, entropy)
                } else {
                    self.relay.receive(connection, &bytes, now(), entropy)
                };
                self.carry_out(out)?;
            }
            ToHub::Gone { connection } => self.gone(connection)?,
            ToHub::Tick => {
                let out = self.pairing.expire(now());
                self.carry_out(out)?;
            }
        }
        Ok(())
    }

    fn gone(&mut self, connection: ConnectionId) -> Result<(), HubError> {
        self.connections.remove(&connection);
        self.fresh.remove(&connection);
        let out = if self.pairs.remove(&connection) {
            self.pairing.disconnect(connection)
        } else if self.backup_connections.remove(&connection) {
            self.backups.disconnect(connection);
            Vec::new()
        } else {
            self.relay.disconnect(connection)
        };
        self.carry_out(out)
    }

    /// Carries out the relay's outputs in order; a store write is synced before what follows.
    fn carry_out(&mut self, out: Vec<RelayOutput>) -> Result<(), HubError> {
        for output in out {
            match output {
                RelayOutput::Store(op) => self.store.apply(&op)?,
                RelayOutput::Send { connection, frame } => {
                    if let Some(sender) = self.connections.get(&connection) {
                        // A socket that just closed reports `Gone` next; nothing to do here.
                        let _ = sender.send(Outgoing::Frame(frame.encode()));
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(sender) = self.connections.remove(&connection) {
                        let _ = sender.send(Outgoing::Close);
                    }
                }
            }
        }
        Ok(())
    }
}

/// What a connection's first message makes it.
fn route(bytes: &[u8]) -> Route {
    match ClientFrame::decode(bytes) {
        Ok(ClientFrame::PairOpen | ClientFrame::PairJoin { .. }) => Route::Pairing,
        Ok(ClientFrame::BackupGet { .. } | ClientFrame::BackupBegin { .. }) => Route::Backup,
        _ => Route::Room,
    }
}

/// The server's clock as the relay reads it (invite expiry, pairing slots).
fn now() -> Millis {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Millis(u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}
