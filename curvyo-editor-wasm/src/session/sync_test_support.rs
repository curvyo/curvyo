//! The test network of the sharing tests (`specs/0092-cooperative-editing`): editing sessions
//! against one in-process relay, with the transport carried out as the host would
//! (`TransportAction`s) on a virtual clock.

use std::collections::VecDeque;

use curvyo_sync_core::{ConnectionId, Millis, Relay, RelayOutput};
use serde_json::Value;

use super::Session;
use super::align_test_support::draw_rect;
use super::sync::{SyncSeeds, TransportAction};

pub(super) const STEP_MS: u64 = 10;
pub(super) const START_MS: u64 = 1_800_000_000_000;
pub(super) const ROOM: [u8; 16] = [0x5a; 16];
pub(super) const SERVER: &str = "ws://relay.test:8787";

pub(super) fn seeds(device: u8, salt: u8) -> SyncSeeds {
    let mut identity = [0; 64];
    for (i, b) in identity.iter_mut().enumerate() {
        *b = device
            .wrapping_mul(29)
            .wrapping_add(u8::try_from(i).unwrap());
    }
    SyncSeeds {
        identity,
        nonce: [device.wrapping_add(salt); 32],
        entropy: [device.wrapping_mul(7).wrapping_add(salt).wrapping_add(1); 32],
    }
}

pub(super) struct Device {
    pub(super) session: Session,
    pub(super) conn: Option<ConnectionId>,
    pub(super) link_up: bool,
    pub(super) connects: usize,
}

pub(super) struct Net {
    pub(super) relay: Relay,
    pub(super) now: Millis,
    pub(super) devices: Vec<Device>,
    nonce: u8,
}

enum Delivery {
    Bytes(usize, Vec<u8>),
    Closed(usize),
}

impl Net {
    pub(super) fn new() -> Self {
        Self {
            relay: Relay::new(),
            now: Millis(START_MS),
            devices: Vec::new(),
            nonce: 0,
        }
    }

    pub(super) fn add(&mut self, session: Session) -> usize {
        self.devices.push(Device {
            session,
            conn: None,
            link_up: true,
            connects: 0,
        });
        self.devices.len() - 1
    }

    pub(super) fn s(&mut self, i: usize) -> &mut Session {
        &mut self.devices[i].session
    }

    fn device_of(&self, connection: ConnectionId) -> Option<usize> {
        self.devices.iter().position(|d| d.conn == Some(connection))
    }

    fn route(&mut self, out: Vec<RelayOutput>, queue: &mut VecDeque<Delivery>) {
        for output in out {
            match output {
                RelayOutput::Send { connection, frame } => {
                    if let Some(i) = self.device_of(connection) {
                        queue.push_back(Delivery::Bytes(i, frame.encode()));
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(i) = self.device_of(connection) {
                        self.devices[i].conn = None;
                        queue.push_back(Delivery::Closed(i));
                    }
                }
                RelayOutput::Store(_) => {}
            }
        }
    }

    /// Carries out every transport action and delivery until nothing moves.
    pub(super) fn pump(&mut self) {
        let mut queue = VecDeque::new();
        loop {
            let mut moved = false;
            for i in 0..self.devices.len() {
                while let Some(action) = self.devices[i].session.sync_next_action() {
                    moved = true;
                    let now = self.now;
                    match action {
                        TransportAction::Connect(address) => {
                            assert_eq!(address, format!("{SERVER}/"));
                            self.devices[i].connects += 1;
                            if self.devices[i].link_up {
                                let conn = self.relay.connect();
                                self.devices[i].conn = Some(conn);
                                self.devices[i].session.sync_connected(now);
                            } else {
                                self.devices[i].session.sync_disconnected(now);
                            }
                        }
                        TransportAction::Send(bytes) => {
                            if let (true, Some(conn)) =
                                (self.devices[i].link_up, self.devices[i].conn)
                            {
                                self.nonce = self.nonce.wrapping_add(1);
                                let out = self.relay.receive(conn, &bytes, now, [self.nonce; 32]);
                                self.route(out, &mut queue);
                            }
                        }
                        TransportAction::Close => {
                            if let Some(conn) = self.devices[i].conn.take() {
                                let out = self.relay.disconnect(conn);
                                self.route(out, &mut queue);
                            }
                        }
                    }
                }
            }
            while let Some(delivery) = queue.pop_front() {
                moved = true;
                let now = self.now;
                match delivery {
                    Delivery::Bytes(i, bytes) => {
                        if self.devices[i].link_up {
                            self.devices[i].session.sync_received(&bytes, now);
                        }
                    }
                    Delivery::Closed(i) => self.devices[i].session.sync_disconnected(now),
                }
            }
            if !moved {
                return;
            }
        }
    }

    /// Cuts or restores a device's network; a cut closes its socket at the relay only.
    pub(super) fn set_link(&mut self, i: usize, up: bool) {
        self.devices[i].link_up = up;
        if !up && let Some(conn) = self.devices[i].conn.take() {
            let out = self.relay.disconnect(conn);
            let mut queue = VecDeque::new();
            self.route(out, &mut queue);
        }
    }

    /// Advances the clock by `ms`: each step ticks and starts a frame in every session.
    pub(super) fn run(&mut self, ms: u64) {
        let until = self.now.plus(ms);
        while self.now < until {
            self.now = self.now.plus(STEP_MS);
            let now = self.now;
            for d in &mut self.devices {
                d.session.sync_tick(now);
                d.session.sync_frame_start();
            }
            self.pump();
        }
    }

    pub(super) fn json(&mut self, i: usize) -> Vec<u8> {
        self.devices[i].session.document.export_json().unwrap()
    }
}

pub(super) fn view(session: &Session) -> Value {
    serde_json::from_str(&session.share_view_json()).unwrap()
}

/// A shares a document holding one rectangle; B joins by A's invite.
pub(super) fn shared_pair() -> (Net, usize, usize) {
    let mut net = Net::new();
    let mut a = Session::new(0xa1);
    draw_rect(&mut a, (10.0, 10.0), (40.0, 30.0));
    let a = net.add(a);
    let now = net.now;
    net.s(a)
        .share_document(SERVER, "Anna", ROOM, &seeds(1, 0), now)
        .unwrap();
    net.pump();
    net.run(200);
    let (link, _) = net.s(a).create_invite(24 * 3600, now).unwrap();
    net.pump();
    let b = net.add(Session::new(0xb2));
    net.s(b)
        .join_document(&link, "Ben", &seeds(2, 0), now)
        .unwrap();
    net.pump();
    net.run(1_000);
    (net, a, b)
}
