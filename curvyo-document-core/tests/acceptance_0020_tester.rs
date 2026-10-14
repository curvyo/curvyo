//! Independent tester acceptance tests for `specs/0020-undo-redo/specification.md`, document level:
//! two replicas that merge (criteria 15 to 21), revival of deleted objects (23), step ids (33),
//! step rows (32, 34 to 38, 60), the wipe (47 to 50) and the file footprint (37, 59).
//! Written from the specification and the public `Document` API before the implementation diff was
//! read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::similar_names
)]
#![allow(
    clippy::assert_is_empty,
    clippy::collapsible_if,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::many_single_char_names)]
#![allow(clippy::cast_possible_truncation)]

use std::collections::HashSet;

use curvyo_document_core::{
    Author, CURRENT_FORMAT_VERSION, Color, Direction, Document, Length, NodeId, ObjectSnapshot,
    Point, RectBounds, StepKind, StepRef, StyleEdit, SyncError, Timestamp, Vec2, pack, shown_ids,
    unpack,
};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(x, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn width_mm(d: &Document, id: NodeId) -> f64 {
    match d.object(id).unwrap() {
        ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
        ObjectSnapshot::Clone(p) => p.resolved.style.stroke.width.as_mm(),
        ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
    }
}

fn fill_color(d: &Document, id: NodeId) -> Color {
    match d.object(id).unwrap() {
        ObjectSnapshot::Primitive(p) => p.style.fill.color,
        ObjectSnapshot::Clone(p) => p.resolved.style.fill.color,
        ObjectSnapshot::Path(p) => p.style.fill.color,
    }
}

fn json(d: &Document) -> Vec<u8> {
    d.export_json().unwrap()
}

/// A replica of `d` with another peer id, holding the same history.
fn replica(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "t").unwrap()).unwrap()
}

/// Sends everything `from` has that `to` lacks.
fn sync_into(from: &Document, to: &Document) {
    let bytes = from.export_updates(&to.version()).unwrap();
    to.import_updates(&bytes).unwrap();
}

fn merge(a: &Document, b: &Document) {
    sync_into(a, b);
    sync_into(b, a);
}

fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

fn set_width(d: &Document, id: NodeId, mm: f64) {
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(mm)))
        .unwrap();
}

// ---------------------------------------------------------------------
// Criteria 15 to 21: collaboration
// ---------------------------------------------------------------------

#[test]
fn c15_peer_steps_never_enter_my_own_steps() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    set_width(&a, r, 2.0);
    let b = replica(&a, 2);
    let rb = rect(&b, 50.0);
    set_width(&b, rb, 3.0);
    set_width(&a, r, 4.0);
    let a_own: Vec<_> = a.own_steps().iter().map(|s| s.step).collect();
    merge(&a, &b);
    let a_after: Vec<_> = a.own_steps().iter().map(|s| s.step).collect();
    assert_eq!(a_after, a_own, "a peer's steps are not mine");
    assert!(a_after.iter().all(|s| s.peer() == 1));
    assert!(b.own_steps().iter().all(|s| s.step.peer() == 2));
    assert_eq!(a.history_len(), b.history_len());
}

#[test]
fn c16_conflict_rule_per_field_peer_value_stays_other_field_goes_back() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let before_fill = fill_color(&a, r);
    set_width(&a, r, 1.0);
    let b = replica(&a, 2);
    // A: stroke width 1 -> 2 and a fill colour, in one step S.
    a.batch("edit_style", |d| {
        set_width(d, r, 2.0);
        d.edit_style(&[r], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
            .unwrap();
    });
    let s = last_step(&a);
    // B: stroke width 5 (the field S wrote).
    set_width(&b, r, 5.0);
    merge(&a, &b);
    // The merged width is whichever side wins the LWW tie; the rule only applies when the current
    // value is still the one S wrote. Make the peer's write definitely later.
    set_width(&b, r, 6.0);
    sync_into(&b, &a);
    assert_eq!(width_mm(&a, r), 6.0);
    let report = a.undo_step(s).unwrap();
    assert_eq!(width_mm(&a, r), 6.0, "the peer's width keeps its value");
    assert_eq!(
        fill_color(&a, r),
        before_fill,
        "the fill only A changed goes back"
    );
    assert!(report.wrote);
    assert_eq!(report.total(), 1);
    assert_eq!(
        report.skipped(),
        1,
        "partly kept: counts as an object that kept later changes"
    );
    sync_into(&a, &b);
    assert_eq!(json(&a), json(&b), "criterion 20: the undo replicates");
}

#[test]
fn c16_undo_when_nobody_else_touched_it_restores_everything() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let before = json(&a);
    a.batch("edit_style", |d| {
        set_width(d, r, 2.0);
        d.edit_style(&[r], &StyleEdit::FillColor(Color { r: 9, g: 9, b: 9 }))
            .unwrap();
    });
    let s = last_step(&a);
    let rep = a.undo_step(s).unwrap();
    assert_eq!(json(&a), before);
    assert_eq!(rep.skipped(), 0);
    let rep = a.redo_step(s).unwrap();
    assert_eq!(rep.skipped(), 0);
    a.undo_step(s).unwrap();
    assert_eq!(json(&a), before, "repeatable");
}

#[test]
fn c16_a_later_own_step_on_the_same_field_is_also_a_later_change() {
    // Three-way: undoing the middle step of three width changes does not clobber the third.
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    set_width(&a, r, 2.0);
    let s2 = last_step(&a);
    set_width(&a, r, 3.0);
    let s3 = last_step(&a);
    let rep = a.undo_step(s2).unwrap();
    assert_eq!(
        width_mm(&a, r),
        3.0,
        "S3 changed the field after S2: S2's undo leaves it"
    );
    assert!(!rep.wrote);
    // The top step still undoes to the value before it, which is the 2.0 S2 wrote.
    a.undo_step(s3).unwrap();
    assert_eq!(width_mm(&a, r), 2.0);
}

#[test]
fn c17_object_deleted_by_a_peer_is_skipped_never_revived() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let keep = rect(&a, 30.0);
    let b = replica(&a, 2);
    a.batch("edit_style", |d| {
        d.edit_style(
            &[r, keep],
            &StyleEdit::FillColor(Color { r: 1, g: 2, b: 3 }),
        )
        .unwrap();
    });
    let s = last_step(&a);
    sync_into(&a, &b);
    b.delete_objects(&[r]).unwrap();
    sync_into(&b, &a);
    assert!(a.object(r).is_none());
    let rep = a.undo_step(s).unwrap();
    assert!(
        a.object(r).is_none(),
        "undo never brings back what a peer removed"
    );
    assert!(a.object(keep).is_some());
    assert_eq!(rep.total(), 2);
    assert_eq!(rep.skipped(), 1);
    assert_ne!(fill_color(&a, keep), Color { r: 1, g: 2, b: 3 });
}

#[test]
fn c17_everything_deleted_by_a_peer_writes_nothing() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    set_width(&a, r, 4.0);
    let s = last_step(&a);
    sync_into(&a, &b);
    b.delete_objects(&[r]).unwrap();
    sync_into(&b, &a);
    let before = json(&a);
    let rep = a.undo_step(s).unwrap();
    assert!(
        !rep.wrote,
        "ADR 0014: a press that skips the whole step writes nothing to the log"
    );
    assert_eq!(rep.skipped(), rep.total());
    assert_eq!(json(&a), before);
}

#[test]
fn c18_created_object_changed_by_a_peer_stays() {
    let a = Document::new(1);
    let b = replica(&a, 2);
    let r = rect(&a, 0.0);
    let s = last_step(&a);
    sync_into(&a, &b);
    set_width(&b, r, 7.0);
    sync_into(&b, &a);
    let rep = a.undo_step(s).unwrap();
    assert!(a.object(r).is_some(), "the object stays");
    assert_eq!(width_mm(&a, r), 7.0);
    assert!(rep.created_object_kept());
    // The unchanged case: no peer touched it, the creation is taken back.
    let c = Document::new(3);
    let q = rect(&c, 0.0);
    let s = last_step(&c);
    let rep = c.undo_step(s).unwrap();
    assert!(c.object(q).is_none());
    assert!(!rep.created_object_kept());
    c.redo_step(s).unwrap();
    assert!(
        c.object(q).is_some(),
        "redo of a creation brings the same object back"
    );
}

#[test]
fn c19_counts_of_skipped_objects() {
    let a = Document::new(1);
    let ids: Vec<_> = (0..3).map(|i| rect(&a, f64::from(i) * 20.0)).collect();
    let b = replica(&a, 2);
    a.translate_objects(&ids, Vec2::new(5.0, 0.0)).unwrap();
    let s = last_step(&a);
    sync_into(&a, &b);
    b.translate_objects(&ids[..1], Vec2::new(0.0, 3.0)).unwrap();
    sync_into(&b, &a);
    let rep = a.undo_step(s).unwrap();
    assert_eq!(rep.total(), 3);
    assert_eq!(
        rep.skipped(),
        1,
        "N of M objects kept later changes by someone else"
    );
}

#[test]
fn c20_undo_replicates_and_the_log_only_grows() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    set_width(&a, r, 3.0);
    let s = last_step(&a);
    sync_into(&a, &b);
    let rows_before = a.history_len();
    let ops_before = a.export_updates(&Document::new(9).version()).unwrap().len();
    a.undo_step(s).unwrap();
    assert_eq!(a.history_len(), rows_before, "an undo adds no row");
    let ops_after = a.export_updates(&Document::new(9).version()).unwrap().len();
    assert!(ops_after > ops_before, "the undo is a forward commit");
    sync_into(&a, &b);
    assert_eq!(json(&a), json(&b));
    // B can redo?? No: redo belongs to A; but A's redo replicates as well.
    a.redo_step(s).unwrap();
    sync_into(&a, &b);
    assert_eq!(json(&a), json(&b));
    assert_eq!(width_mm(&b, r), 3.0);
}

#[test]
fn c21_a_peers_own_steps_are_unchanged_by_my_undo() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    set_width(&b, r, 5.0);
    let b_before: Vec<_> = b.own_steps().iter().map(|s| s.step).collect();
    set_width(&a, r, 3.0);
    let s = last_step(&a);
    a.undo_step(s).unwrap();
    merge(&a, &b);
    let b_after: Vec<_> = b.own_steps().iter().map(|s| s.step).collect();
    assert_eq!(b_before, b_after);
    assert_eq!(json(&a), json(&b));
}

#[test]
fn concurrent_edit_of_the_same_field_converges_and_undo_never_diverges() {
    for a_first in [true, false] {
        let a = Document::new(1);
        let r = rect(&a, 0.0);
        let default_width = width_mm(&a, r);
        let b = replica(&a, 2);
        // Concurrent: neither has seen the other's write.
        set_width(&a, r, 2.0);
        let s = last_step(&a);
        set_width(&b, r, 5.0);
        if a_first {
            merge(&a, &b);
        } else {
            merge(&b, &a);
        }
        assert_eq!(json(&a), json(&b), "convergence");
        let winner = width_mm(&a, r);
        assert!(winner == 2.0 || winner == 5.0, "{winner}");
        a.undo_step(s).unwrap();
        sync_into(&a, &b);
        assert_eq!(json(&a), json(&b), "after the undo both replicas agree");
        if winner == 2.0 {
            // The current value is still the one S wrote: the field goes back.
            assert_eq!(width_mm(&a, r), default_width);
        } else {
            // The peer's value won the tie: it is kept.
            assert_eq!(width_mm(&a, r), 5.0);
        }
        a.redo_step(s).unwrap();
        sync_into(&a, &b);
        assert_eq!(json(&a), json(&b));
    }
}

#[test]
fn three_way_restore_with_remote_edits_between_steps() {
    // A, B, A, B, A interleaved with merges; undo A's steps newest first, all converge.
    let a = Document::new(1);
    let r1 = rect(&a, 0.0);
    let b = replica(&a, 2);
    let r2 = rect(&b, 40.0);
    merge(&a, &b);
    set_width(&a, r1, 2.0);
    let s_a1 = last_step(&a);
    set_width(&b, r2, 6.0);
    merge(&a, &b);
    a.translate_objects(&[r1, r2], Vec2::new(1.0, 1.0)).unwrap();
    let s_a2 = last_step(&a);
    merge(&a, &b);
    let rep = a.undo_step(s_a2).unwrap();
    assert_eq!(rep.skipped(), 0);
    let rep = a.undo_step(s_a1).unwrap();
    assert_eq!(rep.skipped(), 0);
    merge(&a, &b);
    assert_eq!(json(&a), json(&b));
    assert_eq!(width_mm(&a, r2), 6.0, "B's step is untouched");
    assert_ne!(width_mm(&a, r1), 2.0);
}

// ---------------------------------------------------------------------
// Criterion 23: revival of deleted objects with the same id
// ---------------------------------------------------------------------

#[test]
fn c23_undo_of_delete_brings_back_same_ids_and_order_even_after_save_and_open() {
    let a = Document::new(1);
    let r0 = rect(&a, 0.0);
    let r1 = rect(&a, 20.0);
    let r2 = rect(&a, 40.0);
    let order = a.object_ids();
    let before = json(&a);
    a.delete_objects(&[r0, r2]).unwrap();
    let s = last_step(&a);
    assert_eq!(a.object_ids(), vec![r1]);
    a.undo_step(s).unwrap();
    assert_eq!(a.object_ids(), order, "same ids, same z-order");
    assert_eq!(json(&a), before);
    // Save and open.
    let reopened = replica(&a, 5);
    assert_eq!(reopened.object_ids(), order);
    assert_eq!(json(&reopened), before);
}

#[test]
fn c23_revival_keeps_a_peer_edit_made_while_the_object_was_deleted() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    a.delete_objects(&[r]).unwrap();
    let s = last_step(&a);
    // B edits the object concurrently (it has not seen the delete).
    set_width(&b, r, 9.0);
    merge(&a, &b);
    a.undo_step(s).unwrap();
    sync_into(&a, &b);
    assert!(a.object(r).is_some(), "revived");
    assert_eq!(width_mm(&a, r), 9.0, "the peer's edit survives the revival");
    assert_eq!(json(&a), json(&b));
}

#[test]
fn c23_delete_undo_redo_undo_repeats() {
    let a = Document::new(1);
    let r0 = rect(&a, 0.0);
    let r1 = rect(&a, 20.0);
    let before = json(&a);
    a.delete_objects(&[r0, r1]).unwrap();
    let s = last_step(&a);
    for round in 0..4 {
        a.undo_step(s).unwrap();
        assert_eq!(json(&a), before, "undo round {round}");
        a.redo_step(s).unwrap();
        assert!(a.object_ids().is_empty(), "redo round {round}");
    }
}

#[test]
fn c23_undo_of_delete_after_a_peer_replica_merged_in_between() {
    let a = Document::new(1);
    let r0 = rect(&a, 0.0);
    let b = replica(&a, 2);
    a.delete_objects(&[r0]).unwrap();
    let s = last_step(&a);
    merge(&a, &b);
    let rb = rect(&b, 70.0);
    merge(&a, &b);
    a.undo_step(s).unwrap();
    merge(&a, &b);
    assert!(a.object(r0).is_some());
    assert!(a.object(rb).is_some());
    assert_eq!(json(&a), json(&b));
}

// ---------------------------------------------------------------------
// Criteria 32 to 38 and 60: step data
// ---------------------------------------------------------------------

#[test]
fn c32_kinds_come_from_what_the_step_did() {
    let a = Document::new(1);
    a.set_step_time(Timestamp::from_secs(1_700_000_000));
    let r = rect(&a, 0.0);
    set_width(&a, r, 2.0);
    a.delete_objects(&[r]).unwrap();
    let rows = a.history_page(0, 10);
    assert_eq!(rows.len(), 3, "new_document's setup is listed nowhere");
    // Newest first.
    assert_eq!(rows[0].kind, StepKind::Delete);
    assert_eq!(rows[1].kind, StepKind::Change);
    assert_eq!(rows[2].kind, StepKind::Create);
    assert_eq!(rows[0].label, "delete_objects");
    assert_eq!(rows[2].label, "create_rect");
    assert!(rows.iter().all(|row| row.objects.iter().all(|o| o.id == r)));
    // After the delete the row still names the object and its shape (38).
    assert_eq!(rows[0].objects.len(), 1);
    // A document-only step has kind Document.
    let size = a.size();
    let _ = size;
}

#[test]
fn c32_document_kind_for_size_changes() {
    let a = Document::new(1);
    let size =
        curvyo_document_core::DocumentSize::new(Length::from_mm(300.0), Length::from_mm(200.0));
    assert!(a.resize(size).unwrap());
    let rows = a.history_page(0, 10);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, StepKind::Document);
    assert!(rows[0].objects.is_empty());
}

#[test]
fn c33_step_ids_are_seven_chars_stable_and_equal_on_every_replica() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    for i in 0..30 {
        set_width(&a, r, 1.0 + f64::from(i) * 0.1);
    }
    let rows = a.history_page(0, 100);
    assert_eq!(rows.len(), 31);
    for row in &rows {
        assert!(row.shown_id.len() >= 7, "{}", row.shown_id);
        assert!(
            row.shown_id
                .bytes()
                .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()),
            "{}",
            row.shown_id
        );
    }
    let unique: HashSet<_> = rows.iter().map(|r| r.shown_id.clone()).collect();
    assert_eq!(unique.len(), rows.len());
    // Save and open: ids equal.
    let reopened = replica(&a, 77);
    let rows2 = reopened.history_page(0, 100);
    assert_eq!(
        rows.iter().map(|r| &r.shown_id).collect::<Vec<_>>(),
        rows2.iter().map(|r| &r.shown_id).collect::<Vec<_>>()
    );
    // Merge two replicas: ids equal on both.
    let b = replica(&a, 2);
    set_width(&b, r, 8.0);
    set_width(&a, r, 9.0);
    merge(&a, &b);
    let ia: Vec<_> = a
        .history_page(0, 100)
        .iter()
        .map(|r| r.shown_id.clone())
        .collect();
    let ib: Vec<_> = b
        .history_page(0, 100)
        .iter()
        .map(|r| r.shown_id.clone())
        .collect();
    assert_eq!(ia.len(), 33);
    let mut sa = ia.clone();
    let mut sb = ib.clone();
    sa.sort();
    sb.sort();
    assert_eq!(sa, sb, "the same ids on both replicas");
}

#[test]
fn c33_twenty_thousand_generated_steps_have_no_duplicate_shown_id() {
    let steps: Vec<StepRef> = (0..20_000)
        .map(|i| {
            StepRef::new(
                u64::from((i % 7) as u32)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_add(3),
                i * 3,
            )
        })
        .collect();
    let shown = shown_ids(&steps);
    assert_eq!(shown.len(), steps.len());
    let unique: HashSet<_> = shown.iter().collect();
    assert_eq!(unique.len(), shown.len(), "no duplicate shown id");
    for (step, id) in steps.iter().zip(&shown) {
        assert!(id.len() >= 7);
        assert!(
            step.full_id().starts_with(id.as_str()),
            "a shown id is a prefix of the full id"
        );
        assert!(
            id.bytes()
                .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
        );
    }
    // Deterministic.
    assert_eq!(shown, shown_ids(&steps));
    // Order independent: each step has the same id no matter the other steps' order when no
    // collision shortens or lengthens it (the 7-char form of a step is a prefix of its full id).
    let reversed: Vec<StepRef> = steps.iter().rev().copied().collect();
    let shown_rev = shown_ids(&reversed);
    for (i, id) in shown.iter().enumerate() {
        assert_eq!(*id, shown_rev[steps.len() - 1 - i], "step {i}");
    }
}

#[test]
fn c33_a_collision_is_lengthened_for_both() {
    // Search the first two steps whose 7-character ids collide.
    let mut seen: std::collections::HashMap<String, StepRef> = std::collections::HashMap::new();
    let mut pair = None;
    // Random peers (a xorshift stream), as real documents have random peer ids.
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    for i in 0..3_000_000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let s = StepRef::new(x, i % 1000);
        let key = s.full_id()[..7].to_owned();
        if let Some(prev) = seen.insert(key, s) {
            if prev != s {
                pair = Some((prev, s));
                break;
            }
        }
    }
    let (p, q) =
        pair.expect("a 7-char collision among 3,000,000 ids (36^7 = 78e9: birthday ~ 350,000)");
    let shown = shown_ids(&[p, q]);
    assert_ne!(shown[0], shown[1]);
    assert!(shown[0].len() > 7 && shown[1].len() > 7);
    assert!(p.full_id().starts_with(&shown[0]) && q.full_id().starts_with(&shown[1]));
    // A third step without collision keeps seven characters.
    let lone = StepRef::new(1, 1);
    let three = shown_ids(&[p, q, lone]);
    assert_eq!(three[2].len(), 7);
}

#[test]
fn c34_time_is_what_the_caller_passed_and_zero_for_unknown() {
    let a = Document::new(1);
    a.set_step_time(Timestamp::from_secs(1_700_000_123));
    let r = rect(&a, 0.0);
    a.set_step_time(Timestamp::from_secs(1_700_000_456));
    set_width(&a, r, 3.0);
    let rows = a.history_page(0, 10);
    assert_eq!(rows[0].time, Timestamp::from_secs(1_700_000_456));
    assert_eq!(rows[1].time, Timestamp::from_secs(1_700_000_123));
    // Persisted: save and open keep the times.
    let reopened = replica(&a, 3);
    let rows2 = reopened.history_page(0, 10);
    assert_eq!(rows2[0].time, rows[0].time);
    assert_eq!(rows2[1].time, rows[1].time);
}

#[test]
fn c35_author_is_you_for_this_session_and_earlier_for_every_other_peer() {
    let a = Document::new(1);
    rect(&a, 0.0);
    let b = replica(&a, 2);
    // Reopened replica is a new session: the old step is "Earlier".
    assert_eq!(b.history_page(0, 1)[0].author, Author::Earlier);
    let rb = rect(&b, 30.0);
    let rows = b.history_page(0, 5);
    assert_eq!(rows[0].author, Author::You);
    assert_eq!(rows[0].peer, 2);
    assert_eq!(rows[1].author, Author::Earlier);
    assert_eq!(rows[1].peer, 1);
    let _ = rb;
    // The file holds no personal data: only the numeric peer id (no user name in the snapshot).
    let bytes = pack(&b, "t").unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("marc"));
}

#[test]
fn c37_c59_no_format_version_bump_and_optional_keys_load_in_a_reader_that_ignores_them() {
    // History adds no bump of its own: the version is whatever the build had (10 on
    // 2026-10-10; `0023-groups` took 11 afterwards).
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    set_width(&a, r, 2.0);
    let mut wiped = replica(&a, 4);
    wiped.wipe_history().unwrap();
    assert!(wiped.history_epoch().is_some());
    let plain = json(&a);
    let bytes = pack(&wiped, "t").unwrap();
    let opened = unpack(5, &bytes).unwrap();
    assert_eq!(
        json(&opened),
        plain,
        "the drawing is equal with the optional key present"
    );
    // The manifest says the unchanged version.
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut manifest = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("manifest.json").unwrap(),
        &mut manifest,
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(v["format_version"], CURRENT_FORMAT_VERSION);
    // No new member in the container.
    let names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    let plain_archive = zip::ZipArchive::new(std::io::Cursor::new(pack(&a, "t").unwrap())).unwrap();
    let mut plain_names: Vec<String> = plain_archive.file_names().map(str::to_owned).collect();
    let mut names_sorted = names;
    names_sorted.sort();
    plain_names.sort();
    assert_eq!(names_sorted, plain_names, "no new member of the container");
}

#[test]
fn c37_history_is_saved_with_the_file_and_stacks_are_not() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    set_width(&a, r, 2.0);
    a.delete_objects(&[r]).unwrap();
    let rows = a.history_page(0, 10);
    let reopened = replica(&a, 8);
    let rows2 = reopened.history_page(0, 10);
    assert_eq!(rows.len(), rows2.len());
    for (x, y) in rows.iter().zip(&rows2) {
        assert_eq!(
            (&x.shown_id, &x.label, x.kind, x.time),
            (&y.shown_id, &y.label, y.kind, y.time)
        );
        assert_eq!(x.objects, y.objects);
    }
    assert!(
        reopened.own_steps().is_empty(),
        "nothing on the stacks after open"
    );
}

#[test]
fn c38_deleted_objects_stay_visible_after_save_and_open() {
    let a = Document::new(1);
    let r0 = rect(&a, 0.0);
    let r1 = rect(&a, 20.0);
    a.delete_objects(&[r0, r1]).unwrap();
    let reopened = replica(&a, 8);
    let rows = reopened.history_page(0, 10);
    assert_eq!(rows[0].kind, StepKind::Delete);
    assert_eq!(rows[0].objects.len(), 2, "2 rectangles");
    assert!(
        rows[0]
            .objects
            .iter()
            .all(|o| matches!(o.shape, curvyo_document_core::ObjectShape::Rectangle))
    );
    assert!(reopened.object_ids().is_empty());
}

#[test]
fn c60_same_id_author_time_kind_objects_on_both_replicas() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    a.set_step_time(Timestamp::from_secs(111));
    set_width(&a, r, 2.0);
    b.set_step_time(Timestamp::from_secs(222));
    b.delete_objects(&[r]).unwrap();
    merge(&a, &b);
    let ra = a.history_page(0, 20);
    let rb = b.history_page(0, 20);
    assert_eq!(ra.len(), rb.len());
    for x in &ra {
        let y = rb
            .iter()
            .find(|y| y.step == x.step)
            .expect("same step on both");
        assert_eq!(x.shown_id, y.shown_id);
        assert_eq!(x.peer, y.peer);
        assert_eq!(x.time, y.time);
        assert_eq!(x.kind, y.kind);
        assert_eq!(x.objects, y.objects);
    }
    // A step by a peer whose objects are gone still shows kind and summary: A's width change on
    // an object B deleted.
    let step = ra
        .iter()
        .find(|r| r.peer == 1 && r.label == "edit_style")
        .unwrap();
    assert_eq!(step.kind, StepKind::Change);
    assert_eq!(step.objects.len(), 1);
}

#[test]
fn history_page_arguments_are_forgiving() {
    let a = Document::new(1);
    rect(&a, 0.0);
    assert!(a.history_page(5, 5).is_empty());
    assert!(a.history_page(0, 0).is_empty());
    assert_eq!(a.history_page(0, usize::MAX).len(), 1);
    assert!(a.history_page(usize::MAX, 10).is_empty());
    assert_eq!(a.history_page(usize::MAX - 1, usize::MAX).len(), 0);
    let empty = Document::new(2);
    assert_eq!(
        empty.history_len(),
        0,
        "the new_document setup commit is no row"
    );
    assert!(empty.history_page(0, 10).is_empty());
    assert!(empty.own_steps().is_empty());
}

// ---------------------------------------------------------------------
// Criteria 47 to 51: wipe
// ---------------------------------------------------------------------

fn doc_with_history(steps: usize) -> (Document, NodeId) {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    for i in 1..steps {
        set_width(&a, r, 1.0 + (i % 40) as f64 * 0.25);
    }
    (a, r)
}

#[test]
fn c48_wipe_keeps_the_drawing_and_leaves_one_row() {
    let (mut a, r) = doc_with_history(200);
    let r2 = rect(&a, 50.0);
    let before = json(&a);
    let ids = a.object_ids();
    let removed = a.wipe_history().unwrap();
    assert_eq!(removed, 201);
    assert_eq!(json(&a), before, "byte-equal drawing");
    assert_eq!(a.object_ids(), ids);
    assert_eq!(a.history_len(), 1);
    let rows = a.history_page(0, 10);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, StepKind::Document);
    assert_eq!(rows[0].author, Author::You);
    // A step afterwards has the same author and follows the wipe.
    set_width(&a, r, 9.0);
    let rows = a.history_page(0, 10);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].author, Author::You);
    assert_eq!(
        rows[1].author,
        Author::You,
        "the peer id is unchanged by the wipe"
    );
    assert_eq!(a.object_ids(), ids);
    let _ = r2;
    // Snapshot into a new document: one row after the wipe step plus the step after it.
    let snapshot = a.export_loro_snapshot().unwrap();
    let _ = snapshot;
    let reopened = replica(&a, 3);
    assert_eq!(reopened.history_len(), 2);
    assert_eq!(json(&reopened), json(&a));
}

#[test]
fn c48_exported_snapshot_of_a_wiped_document_has_one_row_and_equal_objects() {
    let (mut a, _) = doc_with_history(200);
    let before = json(&a);
    a.wipe_history().unwrap();
    let opened = unpack(9, &pack(&a, "t").unwrap()).unwrap();
    assert_eq!(opened.history_len(), 1);
    assert_eq!(json(&opened), before);
}

#[test]
fn c49_a_wiped_file_is_smaller_and_holds_no_older_row() {
    let (mut a, _) = doc_with_history(500);
    let big = pack(&a, "t").unwrap().len();
    a.wipe_history().unwrap();
    let small = pack(&a, "t").unwrap().len();
    assert!(small < big, "{small} < {big}");
    let opened = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    assert_eq!(opened.history_len(), 1);
}

#[test]
fn c49_epochs_wipe_a_refuses_pre_wipe_updates_of_b_and_nothing_merges() {
    let (a0, r) = doc_with_history(20);
    let mut a = replica(&a0, 11);
    let b = replica(&a0, 12);
    set_width(&b, r, 8.0); // B's pre-wipe update
    a.wipe_history().unwrap();
    let before = json(&a);
    let rows = a.history_len();
    let bytes = b.export_updates(&a.version()).unwrap();
    assert_eq!(a.import_updates(&bytes), Err(SyncError::EpochMismatch));
    assert_eq!(json(&a), before, "A unchanged");
    assert_eq!(a.history_len(), rows);
    // The other direction is refused too: a replica that still has the pre-wipe steps does not
    // take the wiped replica's updates.
    let bytes = a.export_updates(&b.version()).unwrap();
    assert!(b.import_updates(&bytes).is_err());
    // Garbage and truncated bytes never panic.
    assert!(a.import_updates(&[]).is_err());
    assert!(a.import_updates(&[1, 2, 3]).is_err());
    assert!(a.import_updates(&bytes[..bytes.len() / 2]).is_err());
}

#[test]
fn c49_equal_epochs_merge_as_before() {
    let (mut a, r) = doc_with_history(20);
    a.wipe_history().unwrap();
    let b = replica(&a, 12);
    assert_eq!(a.history_epoch(), b.history_epoch());
    set_width(&b, r, 8.0);
    set_width(&a, r, 7.0);
    merge(&a, &b);
    assert_eq!(json(&a), json(&b));
}

#[test]
fn c50_a_shared_document_is_not_wiped() {
    let (mut a, _) = doc_with_history(5);
    a.mark_shared();
    assert!(a.is_shared());
    let before = json(&a);
    let rows = a.history_len();
    assert!(a.wipe_history().is_err());
    assert_eq!(json(&a), before);
    assert_eq!(a.history_len(), rows);
}

#[test]
fn wipe_twice_and_wipe_of_an_empty_document_do_not_corrupt() {
    let (mut a, _) = doc_with_history(10);
    a.wipe_history().unwrap();
    let before = json(&a);
    a.wipe_history().unwrap();
    assert_eq!(json(&a), before);
    assert!(a.history_len() <= 2);
    let mut empty = Document::new(3);
    let before = json(&empty);
    let _ = empty.wipe_history();
    assert_eq!(json(&empty), before);
    let _ = pack(&empty, "t").unwrap();
}

#[test]
fn undo_after_wipe_of_a_pre_wipe_step_is_refused_not_a_panic() {
    let (mut a, r) = doc_with_history(5);
    let old = a.own_steps()[2].step;
    a.wipe_history().unwrap();
    let before = json(&a);
    let result = a.undo_step(old);
    assert!(
        result.is_err(),
        "a wiped step is not in the log: {result:?}"
    );
    assert_eq!(json(&a), before);
    // A step made after the wipe undoes fine.
    set_width(&a, r, 20.0);
    let s = last_step(&a);
    a.undo_step(s).unwrap();
    assert_ne!(width_mm(&a, r), 20.0);
}

#[test]
fn undo_of_an_unknown_step_is_an_error() {
    let a = Document::new(1);
    rect(&a, 0.0);
    let before = json(&a);
    assert!(a.undo_step(StepRef::new(999, 1)).is_err());
    assert!(a.redo_step(StepRef::new(999, 1)).is_err());
    assert_eq!(json(&a), before);
    let _ = Direction::Undo;
}

// ---------------------------------------------------------------------
// Criterion 54: continuation commits
// ---------------------------------------------------------------------

#[test]
fn c54_eleven_continuation_commits_are_one_step() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    for _ in 0..10 {
        a.continue_step();
        a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    }
    assert_eq!(
        a.own_steps().len(),
        2,
        "the rectangle and one move step of 11 commits"
    );
    let rows = a.history_page(0, 10);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].label, "translate_objects");
    // One press takes back all eleven.
    let before_move = {
        let b = Document::new(2);
        let _ = b;
        0.0
    };
    let _ = before_move;
    let step = last_step(&a);
    a.undo_step(step).unwrap();
    match a.object(r).unwrap() {
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => match p.shape {
            curvyo_document_core::Shape::Rect { bounds, .. } => {
                assert!(
                    (bounds.origin.x - 0.0).abs() < 1e-9,
                    "x = {}",
                    bounds.origin.x
                );
            }
            _ => panic!("rect"),
        },
        ObjectSnapshot::Path(_) => panic!("rect"),
    }
}

#[test]
fn c54_a_merge_between_continuations_makes_two_steps() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    for i in 0..10 {
        if i == 4 {
            // After the 5th commit of the run a peer's edit arrives.
            set_width(&b, r, 3.0);
            sync_into(&b, &a);
        }
        a.continue_step();
        a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    }
    let own = a.own_steps();
    assert_eq!(own.len(), 3, "the rectangle and two move steps: {own:?}");
    assert_eq!(own[1].label, "translate_objects");
    assert_eq!(own[2].label, "translate_objects");
}

#[test]
fn c54_continue_step_without_a_previous_step_is_not_joined_and_is_single_use() {
    let a = Document::new(1);
    a.continue_step();
    let r = rect(&a, 0.0);
    assert_eq!(a.own_steps().len(), 1);
    a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    assert_eq!(a.own_steps().len(), 2);
    a.continue_step();
    a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    assert_eq!(a.own_steps().len(), 2, "joined");
    // continue_step is single use: the next commit without it is a new step.
    a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    assert_eq!(a.own_steps().len(), 3);
    // A continue_step before anything else was committed after another peer's merge does not join.
    let b = replica(&a, 2);
    set_width(&b, r, 2.0);
    sync_into(&b, &a);
    a.continue_step();
    a.translate_objects(&[r], Vec2::new(1.0, 0.0)).unwrap();
    assert_eq!(
        a.own_steps().len(),
        4,
        "a merge since the last step ends the run"
    );
}

#[test]
fn batch_makes_one_step_and_an_empty_batch_none() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let steps = a.own_steps().len();
    a.batch("edit_style", |d| {
        set_width(d, r, 2.0);
        set_width(d, r, 3.0);
        set_width(d, r, 4.0);
    });
    assert_eq!(a.own_steps().len(), steps + 1);
    a.batch("edit_style", |_| {});
    assert_eq!(
        a.own_steps().len(),
        steps + 1,
        "a batch that writes nothing adds no step"
    );
    // Nested batches: the outer one decides.
    a.batch("outer", |d| {
        d.batch("inner", |d2| set_width(d2, r, 5.0));
        set_width(d, r, 6.0);
    });
    assert_eq!(a.own_steps().len(), steps + 2);
    let s = last_step(&a);
    a.undo_step(s).unwrap();
    assert_eq!(width_mm(&a, r), 4.0);
}

/// Criterion 33: one known (peer, counter) gives one known id. The values were computed outside
/// the code base (Python: FNV-1a 64 over the 8 bytes of the peer and the 4 bytes of the counter,
/// little endian, base 36 with the least significant digit first, so that every character
/// carries the full 36 values). The specification fixes the hash and the base but not the digit
/// order or the byte layout; this test pins what this build does so a change is deliberate.
#[test]
fn c33_known_ids_are_pinned() {
    for (peer, counter, expected) in [
        (1_u64, 0_i32, "8vcv1yz"),
        (11, 3, "px0eaw6"),
        (42, 7, "49ay9nd"),
    ] {
        assert_eq!(
            &StepRef::new(peer, counter).full_id()[..7],
            expected,
            "({peer}, {counter})"
        );
        assert_eq!(
            shown_ids(&[StepRef::new(peer, counter)]),
            vec![expected.to_owned()]
        );
    }
    assert_eq!(StepRef::new(1, 0).full_id().len(), 13);
}

#[test]
fn c49_an_import_that_leaves_updates_pending_is_an_error_and_changes_nothing_visible() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    set_width(&b, r, 3.0); // X
    let after_x = b.version();
    set_width(&b, r, 4.0); // Y depends on X
    let only_y = b.export_updates(&after_x).unwrap();
    let before = json(&a);
    assert_eq!(a.import_updates(&only_y), Err(SyncError::Pending));
    assert_eq!(
        json(&a),
        before,
        "nothing of the pending updates is visible"
    );
    // Sending everything afterwards works and converges.
    sync_into(&b, &a);
    assert_eq!(json(&a), json(&b));
}
