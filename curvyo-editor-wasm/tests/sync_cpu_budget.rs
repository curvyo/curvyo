//! Thread-CPU budgets of cooperative editing (`specs/0092-cooperative-editing` adrs.md decision 9,
//! criteria 3 and 13), asserted at four times the stated figure in an optimised build: seal or
//! open of 10 kB 0.2 ms, one presence message encoded and sealed 0.1 ms, a full join of 1,000
//! objects (unwrap the epoch key, open the signed snapshot, open the document) 500 ms. The
//! frame-start import of one remote edit lives next to the session (`session/sync_tests.rs`).
//! Debug builds run them as smoke tests. Run:
//! `cargo test --release -p curvyo-editor-wasm --test sync_cpu_budget -- --nocapture`.

#![cfg(unix)]
#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_crypto_core::{
    DocumentId, EnvelopeKind, EpochKey, IdentityKeys, NonceStream, Sealing, open, seal,
    unwrap_epoch_key, wrap_epoch_key,
};
use curvyo_document_core::{Document, Length, Point, RectBounds, pack};
use curvyo_editor_wasm::Session;
use curvyo_sync_core::ClientFrame;
use curvyo_ui_core::presence::{PRESENCE_VERSION, PresencePayload};

const DOCUMENT: DocumentId = DocumentId([0x5a; 16]);
const RUNS: usize = 50;

fn keys(device: u8) -> IdentityKeys {
    IdentityKeys::from_entropy(&[device; 64])
}

/// The worst thread CPU time of `RUNS` runs of `work` after one warm-up run.
fn worst_of(mut work: impl FnMut()) -> Duration {
    work();
    (0..RUNS)
        .map(|_| cpu_time_of(&mut work).1)
        .max()
        .unwrap_or_default()
}

fn assert_within(name: &str, took: Duration, budget: Duration) {
    eprintln!("{name}: {took:?} (budget {budget:?}, asserted at four times)");
    if !cfg!(debug_assertions) {
        assert!(took <= budget * 4, "{name}: {took:?} over 4 x {budget:?}");
    }
}

#[test]
fn seal_and_open_of_10_kb_within_0_2_ms() {
    let key = EpochKey::from_bytes([7; 32]);
    let sealing = Sealing {
        kind: EnvelopeKind::Update,
        epoch: 0,
        key: &key,
        document_id: DOCUMENT,
        sender_peer: 42,
        signer: None,
    };
    let plaintext: Vec<u8> = (0..10_240u32)
        .map(|n| u8::try_from(n % 251).unwrap())
        .collect();
    let mut nonces = NonceStream::new([3; 32]);
    let mut envelope = Vec::new();
    let sealed = worst_of(|| {
        envelope = seal(&sealing, nonces.next_nonce().unwrap(), &plaintext).unwrap();
    });
    assert_within("seal 10 kB", sealed, Duration::from_micros(200));
    let opened = worst_of(|| {
        let opened = open(&envelope, &key, DOCUMENT, EnvelopeKind::Update).unwrap();
        assert_eq!(opened.plaintext.len(), plaintext.len());
    });
    assert_within("open 10 kB", opened, Duration::from_micros(200));
}

#[test]
fn one_presence_message_encoded_and_sealed_within_0_1_ms() {
    let key = EpochKey::from_bytes([9; 32]);
    let sealing = Sealing {
        kind: EnvelopeKind::Presence,
        epoch: 0,
        key: &key,
        document_id: DOCUMENT,
        sender_peer: 42,
        signer: None,
    };
    let mut nonces = NonceStream::new([4; 32]);
    let payload = PresencePayload {
        v: PRESENCE_VERSION,
        label: "Anna".to_owned(),
        peer: "1234567890123".to_owned(),
        pointer: Some([120.5, 80.25]),
        selection: Vec::new(),
        tool: "select".to_owned(),
        drag: Some([3.0, -1.5]),
    };
    let mut x = 0.0;
    let took = worst_of(|| {
        x += 0.5;
        let mut payload = payload.clone();
        payload.pointer = Some([x, 80.25]);
        let bytes = serde_json::to_vec(&payload).unwrap();
        let envelope = seal(&sealing, nonces.next_nonce().unwrap(), &bytes).unwrap();
        let frame = ClientFrame::Awareness { envelope }.encode();
        assert!(frame.len() > curvyo_crypto_core::HEADER_LEN);
    });
    assert_within("presence encode and seal", took, Duration::from_micros(100));
}

#[test]
fn a_full_join_of_1000_objects_within_500_ms() {
    let document = Document::new(0xa1);
    for n in 0..1_000u32 {
        let _ = document.create_rect(RectBounds {
            origin: Point::new(f64::from(n % 40) * 6.0, f64::from(n / 40) * 6.0),
            width: Length::from_mm(4.0),
            height: Length::from_mm(4.0),
        });
    }
    let packed = pack(&document, "budget").unwrap();
    let admin = keys(1);
    let joiner = keys(2);
    let key = EpochKey::from_bytes([11; 32]);
    let wrap = wrap_epoch_key(&key, 0, DOCUMENT, &joiner.identity(), [12; 32]).unwrap();
    let snapshot = seal(
        &Sealing {
            kind: EnvelopeKind::Snapshot,
            epoch: 0,
            key: &key,
            document_id: DOCUMENT,
            sender_peer: 0xa1,
            signer: Some(&admin),
        },
        NonceStream::nonce_at(&[5; 32], 0),
        &packed,
    )
    .unwrap();
    let mut runs = Vec::new();
    for _ in 0..3 {
        let ((), took) = cpu_time_of(|| {
            let key = unwrap_epoch_key(&wrap, 0, DOCUMENT, &joiner).unwrap();
            let opened = open(&snapshot, &key, DOCUMENT, EnvelopeKind::Snapshot).unwrap();
            assert_eq!(opened.signer, Some(admin.identity().member));
            let session = Session::open(0xb2, &opened.plaintext).unwrap();
            drop(session);
        });
        runs.push(took);
    }
    let worst = runs.into_iter().max().unwrap_or_default();
    assert_within("join of 1,000 objects", worst, Duration::from_millis(500));
}
