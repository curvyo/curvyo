//! Live editing through the simulator with real document replicas: convergence, delta size,
//! undo with a peer and concurrent page resizes (`specs/0092` criteria 6 to 13, 21 to 23, 31).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

mod support;

use curvyo_document_core::{
    Affine, Color, Document, DocumentSize, FontKind, FontRef, Length, MovePlan, MoveTarget, NodeId,
    ObjectSnapshot, Point, RectBounds, Shape, StyleEdit, TextChars, TextLayoutValues, Vec2,
};
use support::ROOM;
use support::sim::Net;

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn rect_origin(doc: &Document, id: NodeId) -> Point {
    let Some(ObjectSnapshot::Primitive(p)) = doc.object(id) else {
        panic!("a primitive");
    };
    let Shape::Rect { bounds, .. } = p.shape else {
        panic!("a rect");
    };
    bounds.origin
}

/// A fresh replica of `doc` (same node ids), as a reopened file.
fn copy(doc: &Document, seed: u64) -> Document {
    curvyo_document_core::unpack(100 + seed, &curvyo_document_core::pack(doc, "t").unwrap())
        .unwrap()
}

/// A shares `doc`, B joins; both hold the same replica.
fn pair(seed: u64, doc: Document) -> (Net, usize, usize) {
    let mut net = Net::new(seed);
    let a = net.add_client(1);
    let b = net.add_client(2);
    net.share(a, ROOM, doc);
    net.run(500);
    let secret = net.invite(a, 24 * 3600);
    net.join(b, secret);
    assert!(net.run_until(5_000, |n| n.converged(&[a, b])).is_some());
    net.run(500);
    (net, a, b)
}

fn settle(net: &mut Net, clients: &[usize]) {
    assert!(
        net.run_until(2_000, |n| n.converged(clients)).is_some(),
        "converged within 2 s"
    );
}

/// Criterion 6: an edit in A reaches B, and back, well within the LAN figures of the simulator.
#[test]
fn an_edit_in_one_replica_appears_in_the_other() {
    let doc = Document::new(100);
    rect(&doc, 0.0);
    let (mut net, a, b) = pair(1, doc);
    for round in 0..20 {
        let writer = if round % 2 == 0 { a } else { b };
        net.edit(writer, |d| rect(d, f64::from(round) * 12.0));
        let took = net.run_until(300, |n| n.converged(&[a, b]));
        assert!(took.is_some(), "round {round} arrived within 300 ms");
    }
    assert_eq!(net.clients[b].doc().object_ids().len(), 21);
}

/// Criteria 8 and 9: different objects and different fields merge; the same field converges to
/// one of the two values.
#[test]
fn concurrent_edits_merge_per_field_and_contested_fields_converge() {
    let doc = Document::new(100);
    let one = rect(&doc, 0.0);
    let two = rect(&doc, 20.0);
    for seed in 0..4 {
        let (mut net, a, b) = pair(seed, copy(&doc, seed));
        net.edit(a, |d| {
            d.translate_objects(&[one], Vec2::new(5.0, 0.0)).unwrap();
        });
        net.edit(b, |d| {
            d.translate_objects(&[two], Vec2::new(0.0, 5.0)).unwrap();
        });
        let red = Color { r: 255, g: 0, b: 0 };
        net.edit(a, |d| {
            d.edit_style(&[one], &StyleEdit::FillColor(red)).unwrap();
        });
        net.edit(b, |d| {
            d.edit_style(&[one], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
                .unwrap();
        });
        settle(&mut net, &[a, b]);
        let merged = net.clients[a].doc().primitive(one).unwrap().style;
        assert_eq!(merged.fill.color, red);
        assert_eq!(merged.stroke.width, Length::from_mm(2.0));
        assert_eq!(rect_origin(net.clients[b].doc(), one), Point::new(5.0, 0.0));
        assert_eq!(
            rect_origin(net.clients[a].doc(), two),
            Point::new(20.0, 5.0)
        );
        // The same scalar field set to two values at once: one wins, everywhere.
        net.edit(a, |d| {
            d.edit_style(&[two], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
                .unwrap();
        });
        net.edit(b, |d| {
            d.edit_style(&[two], &StyleEdit::StrokeWidth(Length::from_mm(4.0)))
                .unwrap();
        });
        settle(&mut net, &[a, b]);
        let width = net.clients[b]
            .doc()
            .primitive(two)
            .unwrap()
            .style
            .stroke
            .width;
        assert!(width == Length::from_mm(3.0) || width == Length::from_mm(4.0));
        assert_eq!(
            net.clients[a].import_failures + net.clients[b].import_failures,
            0
        );
    }
}

/// Criterion 10: insertions at different positions of one text are both kept.
#[test]
fn concurrent_typing_in_one_text_keeps_both_insertions() {
    let doc = Document::new(100);
    let font = FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let text = doc
        .create_text(
            "Hello world",
            &font,
            Length::from_mm(10.0),
            Affine::translation(Vec2::new(10.0, 20.0)),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let (mut net, a, b) = pair(3, doc);
    net.edit(a, |d| {
        d.edit_text(text, "Hello, world", &TextChars::default())
            .unwrap();
    });
    net.edit(b, |d| {
        d.edit_text(text, "Hello world!", &TextChars::default())
            .unwrap();
    });
    settle(&mut net, &[a, b]);
    assert_eq!(
        net.clients[a].doc().text(text).unwrap().string,
        "Hello, world!"
    );
}

/// Criterion 11: a path deleted in A while B moves it converges, never half deleted.
#[test]
fn a_delete_concurrent_with_an_edit_converges() {
    let doc = Document::new(100);
    let id = rect(&doc, 0.0);
    let (mut net, a, b) = pair(4, doc);
    net.edit(a, |d| d.delete_objects(&[id]).unwrap());
    net.edit(b, |d| {
        d.translate_objects(&[id], Vec2::new(3.0, 3.0)).unwrap();
    });
    settle(&mut net, &[a, b]);
    let present = |i: usize| net.clients[i].doc().object(id).is_some();
    assert_eq!(present(a), present(b));
    assert!(!present(a), "the delete wins over the move");
}

/// Criterion 12: one object moved into two groups at once ends in exactly one group, the same
/// everywhere (movable tree).
#[test]
fn an_object_moved_into_two_groups_ends_in_one() {
    let doc = Document::new(100);
    let x = rect(&doc, 0.0);
    let left = doc
        .group_objects(&[rect(&doc, 20.0), rect(&doc, 30.0)])
        .unwrap();
    let right = doc
        .group_objects(&[rect(&doc, 40.0), rect(&doc, 50.0)])
        .unwrap();
    for seed in 0..4 {
        let (mut net, a, b) = pair(seed, copy(&doc, seed));
        let into = |group| MovePlan {
            nodes: vec![x],
            target: MoveTarget::FrontOf(Some(group)),
        };
        net.edit(a, |d| d.move_nodes(&into(left)).unwrap());
        net.edit(b, |d| d.move_nodes(&into(right)).unwrap());
        settle(&mut net, &[a, b]);
        let parent = net.clients[a].doc().parent_group(x);
        assert_eq!(parent, net.clients[b].doc().parent_group(x));
        assert!(parent == Some(left) || parent == Some(right));
        let count = |g| {
            net.clients[a]
                .doc()
                .children(Some(g))
                .iter()
                .filter(|&&c| c == x)
                .count()
        };
        assert_eq!(count(left) + count(right), 1);
    }
}

/// Criterion 13 (bytes): one edit in a document of 1,000 objects sends at most 10 kB.
#[test]
fn one_edit_in_a_thousand_objects_sends_at_most_10_kb() {
    let doc = Document::new(100);
    let ids: Vec<NodeId> = (0..1_000)
        .map(|n| rect(&doc, f64::from(n % 40) * 12.0))
        .collect();
    let (mut net, a, b) = pair(5, doc);
    let before = net.clients[a].sent.len();
    net.edit(a, |d| {
        d.translate_objects(&[ids[500]], Vec2::new(1.0, 1.0))
            .unwrap();
    });
    settle(&mut net, &[a, b]);
    let pushed: usize = net.clients[a].sent[before..]
        .iter()
        .filter(|m| m.first() == Some(&0x04))
        .map(Vec::len)
        .sum();
    assert!(pushed > 0 && pushed <= 10_000, "{pushed} bytes");
}

/// Criterion 21: A's undo reverts A's own last edit only; B's undo stack is unchanged.
#[test]
fn undo_reverts_only_my_own_edit() {
    let doc = Document::new(100);
    let one = rect(&doc, 0.0);
    let two = rect(&doc, 20.0);
    let (mut net, a, b) = pair(6, doc);
    net.edit(b, |d| {
        d.translate_objects(&[two], Vec2::new(0.0, 7.0)).unwrap();
    });
    net.edit(a, |d| {
        d.translate_objects(&[one], Vec2::new(7.0, 0.0)).unwrap();
    });
    settle(&mut net, &[a, b]);
    let b_steps = net.clients[b].doc().own_steps();
    net.edit(a, |d| {
        let mine = d.own_steps().last().unwrap().step;
        d.undo_step(mine).unwrap();
    });
    settle(&mut net, &[a, b]);
    assert_eq!(rect_origin(net.clients[b].doc(), one), Point::new(0.0, 0.0));
    assert_eq!(
        rect_origin(net.clients[b].doc(), two),
        Point::new(20.0, 7.0)
    );
    assert_eq!(net.clients[b].doc().own_steps(), b_steps);
}

/// Criterion 22: undo of a creation a peer has since deleted is skipped, nothing comes back.
#[test]
fn undo_of_an_object_a_peer_deleted_is_skipped() {
    let (mut net, a, b) = pair(7, Document::new(100));
    let id = net.edit(a, |d| rect(d, 0.0));
    settle(&mut net, &[a, b]);
    net.edit(b, |d| d.delete_objects(&[id]).unwrap());
    settle(&mut net, &[a, b]);
    let report = net.edit(a, |d| {
        let mine = d.own_steps().last().unwrap().step;
        d.undo_step(mine).unwrap()
    });
    settle(&mut net, &[a, b]);
    assert!(report.skipped() > 0);
    assert!(net.clients[a].doc().object(id).is_none());
    assert!(net.clients[b].doc().object(id).is_none());
}

/// Criterion 23: with nobody else connected, undo and redo behave as in an unshared document.
#[test]
fn undo_alone_behaves_as_unshared() {
    let mut net = Net::new(8);
    let a = net.add_client(1);
    net.share(a, ROOM, Document::new(100));
    net.run(500);
    let plain = Document::new(100);
    let shared_id = net.edit(a, |d| rect(d, 0.0));
    let plain_id = rect(&plain, 0.0);
    net.edit(a, |d| {
        d.translate_objects(&[shared_id], Vec2::new(4.0, 4.0))
            .unwrap();
    });
    plain
        .translate_objects(&[plain_id], Vec2::new(4.0, 4.0))
        .unwrap();
    let shared = net.edit(a, |d| {
        let mine = d.own_steps().last().unwrap().step;
        d.undo_step(mine).unwrap()
    });
    let alone = plain
        .undo_step(plain.own_steps().last().unwrap().step)
        .unwrap();
    assert_eq!(shared.skipped(), alone.skipped());
    assert_eq!(shared.total(), alone.total());
    assert_eq!(
        rect_origin(net.clients[a].doc(), shared_id),
        rect_origin(&plain, plain_id)
    );
}

/// Criterion 31: two peers resize the same page offline to different sizes; after the merge
/// both hold one whole frame and no object moved.
#[test]
fn concurrent_resizes_converge_to_one_whole_frame() {
    let doc = Document::new(100);
    let id = rect(&doc, 10.0);
    let (mut net, a, b) = pair(9, doc);
    net.set_link(a, false);
    net.set_link(b, false);
    net.edit(a, |d| {
        d.resize(DocumentSize::from_mm(300.0, 297.0)).unwrap()
    });
    net.edit(b, |d| {
        d.resize(DocumentSize::from_mm(210.0, 400.0)).unwrap()
    });
    let frame = |net: &Net, i: usize| {
        let page = net.clients[i].doc().pages()[0].clone();
        (page.origin, page.size)
    };
    let (frame_a, frame_b) = (frame(&net, a), frame(&net, b));
    net.run(5_000);
    net.set_link(a, true);
    net.set_link(b, true);
    assert!(net.run_until(15_000, |n| n.converged(&[a, b])).is_some());
    let merged = frame(&net, a);
    assert_eq!(merged, frame(&net, b));
    assert!(merged == frame_a || merged == frame_b, "{merged:?}");
    for i in [a, b] {
        assert_eq!(rect_origin(net.clients[i].doc(), id), Point::new(10.0, 0.0));
    }
}
