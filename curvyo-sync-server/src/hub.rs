//! The single owner of the relay's state, applying every connection's messages to the sans-I/O
//! [`Relay`] in arrival order with each store write on disk before the sends that follow it
//! (ADR 0031 decisions 5 and 11).

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use curvyo_sync_core::{ConnectionId, Millis, Relay, RelayOutput};
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
    store: Store,
    connections: BTreeMap<ConnectionId, mpsc::UnboundedSender<Outgoing>>,
}

impl Hub {
    pub(crate) fn new(relay: Relay, store: Store) -> Self {
        Self {
            relay,
            store,
            connections: BTreeMap::new(),
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
                if reply.send(connection).is_err() {
                    self.connections.remove(&connection);
                    let out = self.relay.disconnect(connection);
                    self.carry_out(out)?;
                }
            }
            ToHub::Message { connection, bytes } => {
                let mut nonce = [0; 32];
                getrandom::fill(&mut nonce).map_err(HubError::Entropy)?;
                let out = self.relay.receive(connection, &bytes, now(), nonce);
                self.carry_out(out)?;
            }
            ToHub::Gone { connection } => {
                self.connections.remove(&connection);
                let out = self.relay.disconnect(connection);
                self.carry_out(out)?;
            }
        }
        Ok(())
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

/// The server's clock as the relay reads it (invite expiry).
fn now() -> Millis {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Millis(u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}
