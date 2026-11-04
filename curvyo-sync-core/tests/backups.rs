//! The relay's passkey backups and the client's exchange (ADR 0040 decision 13, Frage 145 A):
//! a write pins its write key, a read returns the record, a delete needs the same key, a
//! replayed signed write, another passkey's write, an over-large record and frames out of order
//! are refused; the relay's outputs never hold the record's plaintext.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_crypto_core::{BackupContent, BackupKeys, BackupOp, DocumentId, IdentityKeys, MemberId};
use curvyo_sync_core::{
    BACKUP_RECORD_MAX, BackupExchange, BackupOutcome, Backups, ClientFrame, ConnectionId,
    RefusalReason, RelayFrame, RelayOutput, StoreOp, StoredBackup,
};

const PRF: [u8; 32] = [0x11; 32];

fn content(label: &str) -> BackupContent {
    BackupContent {
        identity: IdentityKeys::from_entropy(&[0xab; 64]).to_entropy(),
        group_room: DocumentId([0x6e; 16]),
        server: "wss://sync.example.org".to_owned(),
        label: label.to_owned(),
    }
}

/// Runs one exchange against `relay` on connection `n`; the outcome and every relay output.
fn run(
    relay: &mut Backups,
    n: u64,
    mut exchange: BackupExchange,
) -> (BackupOutcome, Vec<RelayOutput>) {
    let connection = ConnectionId(n);
    let mut all = Vec::new();
    let mut message = Some(exchange.first_frame());
    while let Some(bytes) = message.take() {
        let out = relay.receive(connection, &bytes, [u8::try_from(n).unwrap(); 32]);
        for output in &out {
            if let RelayOutput::Send { frame, .. } = output {
                message = exchange.receive(&frame.encode()).or(message);
            }
        }
        all.extend(out);
    }
    exchange.closed();
    (exchange.take_outcome().unwrap(), all)
}

fn write(relay: &mut Backups, n: u64, prf: &[u8; 32], label: &str) -> BackupOutcome {
    run(
        relay,
        n,
        BackupExchange::write(prf, &content(label), &[u8::try_from(n).unwrap(); 24]).unwrap(),
    )
    .0
}

#[test]
fn a_backup_is_written_read_replaced_and_deleted() {
    let mut relay = Backups::default();
    let (outcome, out) = run(
        &mut relay,
        1,
        BackupExchange::write(&PRF, &content("Firefox on Linux"), &[1; 24]).unwrap(),
    );
    assert_eq!(outcome, BackupOutcome::Done);
    let stored: Vec<_> = out
        .iter()
        .filter_map(|o| match o {
            RelayOutput::Store(StoreOp::Backup(b)) => Some(b.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, BackupKeys::derive(&PRF).id());
    assert!(matches!(out.last(), Some(RelayOutput::Close { .. })));
    assert_eq!(
        run(&mut relay, 2, BackupExchange::read(&PRF)).0,
        BackupOutcome::Read(content("Firefox on Linux"))
    );
    assert_eq!(write(&mut relay, 3, &PRF, "Renamed"), BackupOutcome::Done);
    assert_eq!(relay.count(), 1);
    assert_eq!(
        run(&mut relay, 4, BackupExchange::read(&PRF)).0,
        BackupOutcome::Read(content("Renamed"))
    );
    let (outcome, out) = run(&mut relay, 5, BackupExchange::delete(&PRF));
    assert_eq!(outcome, BackupOutcome::Done);
    assert!(
        out.iter()
            .any(|o| matches!(o, RelayOutput::Store(StoreOp::DropBackup { .. })))
    );
    assert_eq!(
        run(&mut relay, 6, BackupExchange::read(&PRF)).0,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
}

#[test]
fn reading_or_deleting_a_backup_that_is_not_there_is_unknown() {
    let mut relay = Backups::default();
    assert_eq!(
        run(&mut relay, 1, BackupExchange::read(&PRF)).0,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
    assert_eq!(
        run(&mut relay, 2, BackupExchange::delete(&PRF)).0,
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
}

/// A write needs the write key the first put pinned: another key under the same id is refused,
/// and so is a signature over another challenge (a replayed write).
#[test]
fn another_write_key_or_a_replayed_write_is_forbidden() {
    let keys = BackupKeys::derive(&PRF);
    let mut relay = Backups::default();
    assert_eq!(write(&mut relay, 1, &PRF, "A"), BackupOutcome::Done);
    let stranger = BackupKeys::derive(&[0x22; 32]);
    let record = keys.seal(&content("B"), &[0; 24]).unwrap();

    let connection = ConnectionId(7);
    let nonce = [7; 32];
    relay.receive(
        connection,
        &ClientFrame::BackupBegin { id: keys.id() }.encode(),
        nonce,
    );
    let out = relay.receive(
        connection,
        &ClientFrame::BackupPut {
            write_key: stranger.write_key(),
            signature: stranger.sign_write(BackupOp::Put, &nonce, &record),
            record: record.clone(),
        }
        .encode(),
        [0; 32],
    );
    assert_eq!(refusal(&out), Some(RefusalReason::Forbidden));

    let connection = ConnectionId(8);
    relay.receive(
        connection,
        &ClientFrame::BackupBegin { id: keys.id() }.encode(),
        [8; 32],
    );
    let replayed = ClientFrame::BackupPut {
        write_key: keys.write_key(),
        signature: keys.sign_write(BackupOp::Put, &[1; 32], &record),
        record,
    };
    assert_eq!(
        refusal(&relay.receive(connection, &replayed.encode(), [0; 32])),
        Some(RefusalReason::Forbidden)
    );
    let connection = ConnectionId(9);
    relay.receive(
        connection,
        &ClientFrame::BackupBegin { id: keys.id() }.encode(),
        [9; 32],
    );
    let delete = ClientFrame::BackupDelete {
        signature: stranger.sign_write(BackupOp::Delete, &[9; 32], &[]),
    };
    assert_eq!(
        refusal(&relay.receive(connection, &delete.encode(), [0; 32])),
        Some(RefusalReason::Forbidden)
    );
    assert_eq!(
        run(&mut relay, 10, BackupExchange::read(&PRF)).0,
        BackupOutcome::Read(content("A"))
    );
}

#[test]
fn an_over_large_record_and_frames_out_of_order_are_refused() {
    let keys = BackupKeys::derive(&PRF);
    let mut relay = Backups::default();
    let connection = ConnectionId(1);
    relay.receive(
        connection,
        &ClientFrame::BackupBegin { id: keys.id() }.encode(),
        [1; 32],
    );
    let record = vec![1; BACKUP_RECORD_MAX + 1];
    let put = ClientFrame::BackupPut {
        write_key: keys.write_key(),
        signature: keys.sign_write(BackupOp::Put, &[1; 32], &record),
        record,
    };
    assert_eq!(
        refusal(&relay.receive(connection, &put.encode(), [0; 32])),
        Some(RefusalReason::TooLarge)
    );
    let put_first = ClientFrame::BackupPut {
        write_key: MemberId([1; 32]),
        signature: [0; 64],
        record: vec![1],
    };
    assert_eq!(
        refusal(&relay.receive(ConnectionId(2), &put_first.encode(), [0; 32])),
        Some(RefusalReason::Protocol)
    );
    assert_eq!(
        refusal(&relay.receive(ConnectionId(3), &[0x0d, 1], [0; 32])),
        Some(RefusalReason::Protocol)
    );
    assert_eq!(relay.count(), 0);
}

#[test]
fn a_record_another_passkey_cannot_open_is_damaged() {
    let mut relay = Backups::restore(vec![StoredBackup {
        id: BackupKeys::derive(&PRF).id(),
        write_key: MemberId([1; 32]),
        record: BackupKeys::derive(&[0x33; 32])
            .seal(&content("x"), &[0; 24])
            .unwrap(),
    }]);
    assert_eq!(
        run(&mut relay, 1, BackupExchange::read(&PRF)).0,
        BackupOutcome::Damaged
    );
}

#[test]
fn an_exchange_without_an_answer_failed() {
    let mut exchange = BackupExchange::read(&PRF);
    exchange.closed();
    assert_eq!(exchange.take_outcome(), Some(BackupOutcome::Failed));
}

fn refusal(out: &[RelayOutput]) -> Option<RefusalReason> {
    out.iter().find_map(|o| match o {
        RelayOutput::Send {
            frame: RelayFrame::Refused { reason },
            ..
        } => Some(*reason),
        _ => None,
    })
}
