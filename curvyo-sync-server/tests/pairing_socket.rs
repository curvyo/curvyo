//! Pairing over real `WebSocket` connections (`specs/0096-own-devices-sync` criteria 1 to 5 and
//! 30, ADR 0040 decision 3): the server routes a connection whose first frame is `PairOpen` or
//! `PairJoin` to its pairing slots, pairs two devices, refuses unknown, used, locked and busy
//! codes, and keeps nothing of a pairing on disk or in its log.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::Path;
use std::time::{Duration, Instant};

use curvyo_crypto_core::DocumentId;
use curvyo_sync_core::{
    PAIR_WRONG_TRIES, PairEvent, PairFailure, PairJoiner, PairOpener, PairOutput, RefusalReason,
};
use futures_util::{SinkExt, StreamExt};
use support::{Captured, identity, start};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const GROUP: DocumentId = DocumentId([0x6e; 16]);

/// One side of a pairing: its machine, its socket and what it showed.
struct Side<M> {
    machine: M,
    socket: Option<Socket>,
    events: Vec<PairEvent>,
}

trait Machine {
    fn receive(&mut self, bytes: &[u8]) -> Vec<PairOutput>;
    fn closed(&mut self) -> Vec<PairOutput>;
}

impl Machine for PairOpener {
    fn receive(&mut self, bytes: &[u8]) -> Vec<PairOutput> {
        PairOpener::receive(self, bytes)
    }
    fn closed(&mut self) -> Vec<PairOutput> {
        PairOpener::closed(self)
    }
}

impl Machine for PairJoiner {
    fn receive(&mut self, bytes: &[u8]) -> Vec<PairOutput> {
        PairJoiner::receive(self, bytes)
    }
    fn closed(&mut self) -> Vec<PairOutput> {
        PairJoiner::closed(self)
    }
}

impl<M: Machine> Side<M> {
    async fn new(url: &str, machine: M) -> Self {
        let (socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        Self {
            machine,
            socket: Some(socket),
            events: Vec::new(),
        }
    }

    async fn carry_out(&mut self, out: Vec<PairOutput>) {
        for output in out {
            match output {
                PairOutput::Send(bytes) => {
                    if let Some(socket) = self.socket.as_mut() {
                        socket.send(Message::Binary(bytes.into())).await.unwrap();
                    }
                }
                PairOutput::Close => {
                    if let Some(mut socket) = self.socket.take() {
                        let _ = socket.close(None).await;
                    }
                }
                PairOutput::Event(event) => self.events.push(event),
            }
        }
    }

    /// Reads what arrived within a few milliseconds.
    async fn step(&mut self) {
        while let Some(socket) = self.socket.as_mut() {
            match tokio::time::timeout(Duration::from_millis(2), socket.next()).await {
                Ok(Some(Ok(Message::Binary(bytes)))) => {
                    let out = self.machine.receive(&bytes);
                    self.carry_out(out).await;
                }
                Ok(Some(Ok(Message::Close(_)) | Err(_)) | None) => {
                    self.socket = None;
                    let out = self.machine.closed();
                    self.carry_out(out).await;
                }
                Ok(Some(Ok(_))) => {}
                Err(_) => return,
            }
        }
    }

    fn failure(&self) -> Option<PairFailure> {
        self.events.iter().find_map(|event| match event {
            PairEvent::Failed(failure) => Some(*failure),
            _ => None,
        })
    }

    fn compared(&self) -> Option<[&'static str; 4]> {
        self.events.iter().find_map(|event| match event {
            PairEvent::Compare { words, .. } => Some(*words),
            _ => None,
        })
    }

    fn paired(&self) -> bool {
        self.events
            .iter()
            .any(|event| matches!(event, PairEvent::Paired { .. }))
    }
}

/// Steps both sides until `done` holds or 5 s passed.
async fn run<A: Machine, B: Machine>(
    a: &mut Side<A>,
    b: &mut Side<B>,
    done: impl Fn(&Side<A>, &Side<B>) -> bool,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if done(a, b) {
            return true;
        }
        a.step().await;
        b.step().await;
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    false
}

async fn opener(url: &str, n: u8) -> (Side<PairOpener>, String) {
    let mut machine = PairOpener::new(url, identity(n), "Workshop", [n; 32]).unwrap();
    let out = machine.start().unwrap();
    let mut side = Side::new(url, machine).await;
    side.carry_out(out).await;
    let start = Instant::now();
    loop {
        side.step().await;
        let code = side.events.iter().find_map(|event| match event {
            PairEvent::Code { code, .. } => Some(code.clone()),
            _ => None,
        });
        if let Some(code) = code {
            return (side, code);
        }
        assert!(start.elapsed() < Duration::from_secs(5), "no code");
    }
}

async fn joiner(url: &str, code: &str, n: u8) -> Side<PairJoiner> {
    let mut machine = PairJoiner::new(url, code, identity(n), "ZQX-PC-7731", [n; 32]).unwrap();
    let out = machine.start().unwrap();
    let mut side = Side::new(url, machine).await;
    side.carry_out(out).await;
    side
}

/// A code with a wrong secret on the same slot.
fn wrong(code: &str) -> String {
    let mut chars: Vec<char> = code.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == '7' { '8' } else { '7' };
    chars.into_iter().collect()
}

/// A code on another slot.
fn other_slot(code: &str) -> String {
    let mut chars: Vec<char> = code.chars().collect();
    chars[0] = if chars[0] == '2' { '3' } else { '2' };
    chars.into_iter().collect()
}

fn files_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for item in std::fs::read_dir(dir).unwrap() {
        let path = item.unwrap().path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// Criteria 1 to 3, 4 and 30: two devices pair through the server with matching check words;
/// the used code is refused afterwards; a wrong code is refused and locks after five tries; a
/// busy slot and an unknown slot are refused; the data directory stays empty and the log names
/// neither the code nor the device name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devices_pair_through_the_server_and_nothing_stays() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());

    // Pair.
    let (mut a, code) = opener(&url, 1).await;
    let mut b = joiner(&url, &code, 2).await;
    assert!(
        run(&mut a, &mut b, |a, b| a.compared().is_some()
            && b.compared().is_some())
        .await
    );
    assert_eq!(a.compared(), b.compared());
    assert!(!b.paired(), "nothing before Pair (criterion 2)");
    let out = a.machine.pair(GROUP).unwrap();
    a.carry_out(out).await;
    assert!(run(&mut a, &mut b, |a, b| a.paired() && b.paired()).await);

    // The code was used.
    let mut late = joiner(&url, &code, 3).await;
    assert!(run(&mut late, &mut b, |late, _| late.failure().is_some()).await);
    assert_eq!(
        late.failure(),
        Some(PairFailure::Refused(RefusalReason::Used))
    );

    // An unknown slot.
    let mut unknown = joiner(&url, &other_slot(&code), 3).await;
    assert!(run(&mut unknown, &mut b, |u, _| u.failure().is_some()).await);
    assert_eq!(
        unknown.failure(),
        Some(PairFailure::Refused(RefusalReason::Unknown))
    );

    // A new code: a wrong secret is refused, a second joiner meanwhile is busy, and five wrong
    // tries lock the code.
    let (mut a, code) = opener(&url, 4).await;
    for attempt in 0..PAIR_WRONG_TRIES {
        let mut guess = joiner(&url, &wrong(&code), 10 + attempt).await;
        if attempt == 0 {
            let mut second = joiner(&url, &wrong(&code), 30).await;
            assert!(run(&mut second, &mut guess, |s, _| s.failure().is_some()).await);
            assert_eq!(
                second.failure(),
                Some(PairFailure::Refused(RefusalReason::Busy))
            );
        }
        assert!(run(&mut a, &mut guess, |_, g| g.failure().is_some()).await);
        let expected = if attempt + 1 == PAIR_WRONG_TRIES {
            PairFailure::Locked
        } else {
            PairFailure::WrongCode
        };
        assert_eq!(guess.failure(), Some(expected), "{attempt}");
    }
    assert!(run(&mut a, &mut b, |a, _| a.failure().is_some()).await);
    assert_eq!(a.failure(), Some(PairFailure::Locked));
    let mut after = joiner(&url, &code, 40).await;
    assert!(run(&mut after, &mut b, |s, _| s.failure().is_some()).await);
    assert_eq!(
        after.failure(),
        Some(PairFailure::Refused(RefusalReason::Locked))
    );

    server.shutdown().await.unwrap();
    let stored = files_under(data.path());
    assert!(stored.is_empty(), "a pairing left files: {stored:?}");
    let logged = String::from_utf8(log.bytes()).unwrap();
    for secret in [
        code.as_str(),
        &code.replace('-', ""),
        "ZQX-PC-7731",
        "Workshop",
    ] {
        assert!(!logged.contains(secret), "the log names {secret}");
    }
}

/// Criterion 5: a joiner waits while the opener's code is shown and does not pair on its own; the
/// opener closing ends the joiner's wait as gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_joiner_never_pairs_without_the_opener() {
    let data = tempfile::tempdir().unwrap();
    let log = Captured::default();
    let server = start("127.0.0.1:0".parse().unwrap(), data.path(), &log).await;
    let url = format!("ws://{}", server.local_addr());
    let (mut a, code) = opener(&url, 1).await;
    let mut b = joiner(&url, &code, 2).await;
    assert!(run(&mut a, &mut b, |_, b| b.compared().is_some()).await);
    let out = a.machine.cancel();
    a.carry_out(out).await;
    assert!(run(&mut a, &mut b, |_, b| b.failure().is_some()).await);
    assert_eq!(b.failure(), Some(PairFailure::Cancelled));
    assert!(!b.paired());
    server.shutdown().await.unwrap();
}
