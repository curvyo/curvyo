//! The main thread's cost of a synced workspace that is not open (`specs/0096-own-devices-sync`
//! criterion 19, ADR 0040 decision 7): a background session receives a workspace of 20
//! documents of 1,000 objects and then live edits, and each received frame, and the My devices
//! facts read once a second, stay within one 16 ms frame of thread CPU time. The budget is
//! asserted in release builds; a debug build runs a smaller workspace and only prints the times.

use std::time::Duration;

use curvyo_document_core::{Document, Length, Point, RectBounds, pack};

use super::Session;
use super::align_test_support::draw_rect;
use super::device_sync_tests::group;
use super::sync_test_support::{Net, SERVER, seeds, thread_cpu};
use super::workspace_sync::WorkspaceWrite;
use super::workspace_sync_tests::{WORKSPACE, open_document};
use crate::WorkspaceShareBuilder;

const FRAME: Duration = Duration::from_millis(16);

fn drawing(peer: u64, rects: u32) -> Document {
    let doc = Document::new(peer);
    for i in 0..rects {
        let _ = doc.create_rect(RectBounds {
            origin: Point::new(f64::from(i % 100) * 10.0, f64::from(i / 100) * 10.0),
            width: Length::from_mm(5.0),
            height: Length::from_mm(5.0),
        });
    }
    doc
}

#[test]
fn a_background_workspace_stays_within_a_frame_per_received_frame() {
    let (documents, rects) = if cfg!(debug_assertions) {
        (4_u8, 100)
    } else {
        (20, 1_000)
    };
    let mut builder = WorkspaceShareBuilder::new(WORKSPACE, 0xa1, 1_800_000_000, [4; 16]);
    let mut first = Vec::new();
    for n in 0..documents {
        let bytes = pack(&drawing(u64::from(n) + 10, rects), "test").unwrap();
        if n == 0 {
            first.clone_from(&bytes);
        }
        builder
            .add_document(&[format!("Part {n}.curvyo")], &bytes, [n + 1; 16])
            .unwrap();
    }
    let (prepared, files) = builder.finish([6; 16]);
    let mut net = Net::new();
    let a = net.add(Session::new(0xa1));
    let now = net.now;
    net.s(a)
        .share_workspace(SERVER, "Anna", "Box job", prepared, &seeds(1, 0), now)
        .unwrap();
    net.pump();
    net.run(500);
    assert_eq!(
        net.s(a)
            .reconcile_workspace(&group(), true)
            .unwrap()
            .written,
        1
    );
    net.pump();

    // B holds nothing open: the workspace is a background session there.
    let b = net.add(Session::new(0xb2));
    net.watch = Some((b, Duration::ZERO));
    let now = net.now;
    assert!(
        net.s(b)
            .join_synced_workspace(SERVER, WORKSPACE, "Box job", "Ben", &seeds(2, 0), now)
    );
    net.pump();
    net.run(3_000);
    let arrived = net
        .s(b)
        .take_workspace_writes()
        .into_iter()
        .filter(|w| matches!(w, WorkspaceWrite::Document { .. }))
        .count();
    assert_eq!(arrived, usize::from(documents));
    let arrival = net.watch.map(|(_, slowest)| slowest).unwrap();

    // A edits the first document; B queues the records for its background writer.
    let opened = files.iter().find(|(path, _)| path[0] == "Part 0.curvyo");
    open_document(
        &mut net,
        a,
        0xa2,
        &opened.map_or(first, |(_, bytes)| bytes.clone()),
    );
    net.watch = Some((b, Duration::ZERO));
    for i in 0..20 {
        let x = f64::from(i) * 3.0;
        draw_rect(net.s(a), (x, 400.0), (x + 2.0, 402.0));
        net.run(50);
    }
    let live = net.watch.map(|(_, slowest)| slowest).unwrap();
    assert!(
        net.s(b).take_background_one().is_some(),
        "the edits wait for B's background writer"
    );

    let group = group();
    let mut facts = None;
    let read = thread_cpu(|| facts = net.s(b).device_facts(&group, true));
    assert_eq!(facts.unwrap().members.len(), 2);

    println!("arrival frame {arrival:?}, live frame {live:?}, facts {read:?}");
    if !cfg!(debug_assertions) {
        assert!(arrival <= FRAME, "arrival frame took {arrival:?}");
        assert!(live <= FRAME, "live frame took {live:?}");
        assert!(read <= FRAME, "facts took {read:?}");
    }
}
