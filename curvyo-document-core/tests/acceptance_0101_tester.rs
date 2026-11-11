//! Independent acceptance tests for `specs/0101-laser-operations` on the file: the v39 goldens
//! round trip (criterion 25), the unknown and unreadable records kept and saved back (26), the
//! dangling and clone-member keys (ADR 0046 §2 and §3), a format 38 file opening (25), and the
//! current format number (ADR 0046 §6).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, NodeId, OperationSnapshot, pack, unpack,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// The operations of every manufacturing of `d`, with their manufacturing id.
fn all_operations(d: &Document) -> Vec<(NodeId, Vec<OperationSnapshot>)> {
    d.manufacturings()
        .into_iter()
        .map(|m| (m.id, d.operations(m.id).unwrap()))
        .collect()
}

fn round_trip(bytes: &[u8]) -> Vec<u8> {
    let d = unpack(9, bytes).unwrap();
    pack(&d, "0101-tester").unwrap()
}

#[test]
fn c25_the_format_number_is_39_for_this_slice() {
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 41);
}

#[test]
fn c25_the_v39_golden_reads_all_five_kinds_with_their_switches() {
    let d = unpack(9, &fixture("operations_v39.curvyo")).unwrap();
    let laser: Vec<_> = d
        .manufacturings()
        .into_iter()
        .filter(|m| m.process == "curvyo.laser")
        .collect();
    assert!(!laser.is_empty(), "the golden holds laser pages");
    let ops: Vec<OperationSnapshot> = laser
        .iter()
        .flat_map(|m| d.operations(m.id).unwrap())
        .collect();
    let kinds: Vec<&str> = ops
        .iter()
        .map(|o| o.kind.as_deref().unwrap_or(""))
        .collect();
    for kind in ["cut", "score", "engrave", "engrave_outline", "cut_out"] {
        assert!(kinds.contains(&kind), "golden lacks {kind}: {kinds:?}");
    }
    assert!(
        ops.iter().any(|o| !o.show),
        "a Show-off operation is in the golden"
    );
    assert!(
        ops.iter().any(|o| !o.output),
        "an Output-off operation is in the golden"
    );
    assert!(
        ops.iter().any(|o| o.margin.as_mm() > 0.0),
        "a Cut-out margin is in the golden"
    );
    // Customer 2026-11-10: no operation colour (tester change: this assertion was removed).
}

#[test]
fn c25_the_v39_golden_round_trips_with_identical_operations() {
    let before = unpack(9, &fixture("operations_v39.curvyo")).unwrap();
    let saved = pack(&before, "0101-tester").unwrap();
    let after = unpack(9, &saved).unwrap();
    assert_eq!(
        all_operations(&before),
        all_operations(&after),
        "list, names, switches and memberships"
    );
    let cuts = |d: &Document| -> Vec<Option<serde_json::Value>> {
        d.manufacturings()
            .iter()
            .map(|m| m.process_values.get("x.curvyo.laser.cuts_last").cloned())
            .collect()
    };
    assert_eq!(cuts(&before), cuts(&after), "Cuts last survives the save");
}

#[test]
fn c25_a_clone_member_key_reads_as_a_path_through_the_clone() {
    let d = unpack(9, &fixture("operations_clone_member_v39.curvyo")).unwrap();
    let longest = all_operations(&d)
        .into_iter()
        .flat_map(|(_, ops)| ops.into_iter())
        .flat_map(|o| o.members.into_iter())
        .map(|path| path.len())
        .max()
        .unwrap_or(0);
    assert!(
        longest >= 2,
        "a clone member is keyed by the path clone then member, got {longest}"
    );
}

#[test]
fn c25_the_clone_member_golden_round_trips_unchanged() {
    let bytes = fixture("operations_clone_member_v39.curvyo");
    let first = unpack(9, &bytes).unwrap();
    let again = unpack(9, &round_trip(&bytes)).unwrap();
    assert_eq!(all_operations(&first), all_operations(&again));
}

#[test]
fn c26_an_unknown_kind_is_kept_with_its_memberships_and_saved_back() {
    let bytes = fixture("operations_unknown_kind_v39.curvyo");
    let d = unpack(9, &bytes).unwrap();
    let before = all_operations(&d);
    let unknown: Vec<&OperationSnapshot> = before
        .iter()
        .flat_map(|(_, ops)| ops.iter())
        .filter(|o| {
            !matches!(
                o.kind.as_deref(),
                Some("cut" | "score" | "engrave" | "engrave_outline" | "cut_out")
            )
        })
        .collect();
    assert!(
        !unknown.is_empty(),
        "the golden holds an operation of a kind this build does not know"
    );
    assert!(unknown.iter().all(|o| !o.name.is_empty()));

    let saved = unpack(9, &round_trip(&bytes)).unwrap();
    assert_eq!(
        before,
        all_operations(&saved),
        "kept and saved back unchanged"
    );
}

#[test]
fn c26_an_unreadable_record_is_kept_and_saved_back() {
    let bytes = fixture("operations_unreadable_v39.curvyo");
    let d = unpack(9, &bytes).unwrap();
    let before = all_operations(&d);
    assert!(
        before
            .iter()
            .flat_map(|(_, ops)| ops.iter())
            .any(|o| o.kind.is_none()),
        "the golden holds an operation whose kind cannot be read"
    );
    let saved = unpack(9, &round_trip(&bytes)).unwrap();
    assert_eq!(before, all_operations(&saved));
}

#[test]
fn c25_a_dangling_key_opens_and_is_saved_back_without_being_repaired() {
    let bytes = fixture("operations_dangling_v39.curvyo");
    let d = unpack(9, &bytes).unwrap();
    let before = all_operations(&d);
    for (_, ops) in &before {
        for o in ops {
            for path in &o.members {
                assert!(
                    !path.is_empty(),
                    "a dangling key is not read as a live member"
                );
            }
        }
    }
    let saved = unpack(9, &round_trip(&bytes)).unwrap();
    assert_eq!(
        before,
        all_operations(&saved),
        "a key whose path is not live is never repaired on read"
    );
}

#[test]
fn c25_a_format_38_file_opens_with_no_operations_and_no_error() {
    let bytes = fixture("bring_content_in_v38.curvyo");
    let d = unpack(9, &bytes).unwrap();
    for (_, ops) in all_operations(&d) {
        assert!(ops.is_empty(), "a format 38 file has no operation nodes");
    }
}
