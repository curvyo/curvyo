//! `specs/0156-fused-lettering` criterion 19 in thread CPU time: Bridge on a selection of 60
//! pieces with 3,000 nodes in all takes at most 300 ms end to end (`adrs.md` decision 8). The
//! budget is asserted in release builds (the `performance-budgets` job); a debug build prints the
//! time and checks the result.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    AnchorId, Document, DocumentSize, NewAnchor, NewOutputPage, Point, pack,
};
use curvyo_editor_wasm::{FuseOutcome, Session, Tool};
use curvyo_ui_core::{FuseMode, FuseSettings};

const BUDGET: Duration = Duration::from_millis(300);

/// Whether the time is asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

/// A laser page with 60 discs of 50 corner nodes each (3,000 nodes), 6 mm across, 2 mm apart in
/// a grid of 10 x 6. Returns the file and the laser page id.
fn page_file() -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = laser.origin;
    let mut next = 0_u64;
    for i in 0..60 {
        let (col, row) = (f64::from(i % 10), f64::from(i / 10));
        let (cx, cy) = (origin.x + 10.0 + col * 8.0, origin.y + 10.0 + row * 8.0);
        let anchors: Vec<NewAnchor> = (0..50)
            .map(|k| {
                let a = std::f64::consts::TAU * f64::from(k) / 50.0;
                next += 1;
                NewAnchor::corner(
                    AnchorId::new(3, next),
                    Point::new(cx + 3.0 * a.cos(), cy + 3.0 * a.sin()),
                )
            })
            .collect();
        let _ = d.create_path(&anchors, true);
    }
    (pack(&d, "budget").unwrap(), laser.id)
}

#[test]
fn criterion_19_bridge_on_60_pieces_and_3000_nodes() {
    let (bytes, laser) = page_file();
    let settings = FuseSettings {
        mode: FuseMode::Lettering,
        width_mm: 1.0,
        reach_mm: 5.0,
        join_all: false,
        bridges_per_island: 2.0,
    };
    let cost = (0..3)
        .map(|_| {
            let mut s = Session::open(2, &bytes).unwrap();
            s.resize_viewport(1600.0, 1000.0);
            s.set_tool(Tool::Select);
            assert!(s.activate_page(&laser));
            let (from, to) = (Point::new(-5000.0, -5000.0), Point::new(50_000.0, 50_000.0));
            s.pointer_down(from, false);
            s.pointer_hover(to, false, false);
            s.pointer_up(to, false, false);
            assert_eq!(s.selected_object_count(), 60);
            let (outcome, cost) = cpu_time::cpu_time_of(|| s.apply_fuse(settings));
            assert_eq!(outcome, FuseOutcome::Applied { bridges: 59 });
            cost
        })
        .min()
        .unwrap();
    eprintln!("criterion 19: Bridge on 60 pieces {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}
