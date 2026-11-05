//! The page activation budget (`specs/0098-page-kinds` criterion 21, `adrs.md` decision 1): with
//! 20 pages of all three kinds and 1,000 objects, each of 50 activations of alternating kinds
//! (the tools and tabs following the page, and the panel, page list and status read that the
//! frame of the switch shows) costs at most 50 ms of thread CPU (`tests/cpu_time/`). Like the
//! other budgets, the time is asserted in a release build (CI's `performance-budgets` job); a
//! debug build only prints.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use curvyo_document_core::{Document, DocumentSize, NewOutputPage, PageKind, Point, RectBounds};
use curvyo_editor_wasm::{Session, Workbench};

const BUDGET_S: f64 = 0.050;

/// 20 pages: drawing, sketch and laser pages in turn, 1,000 rectangles over the drawing pages.
fn scene() -> (Session, Vec<String>) {
    let document = Document::new(21);
    let mut ids = vec![document.pages()[0].id.clone()];
    for index in 1..20 {
        let page = match index % 3 {
            0 => document.add_page_of(&PageKind::Drawing).unwrap(),
            1 => document.add_page_of(&PageKind::Sketch).unwrap(),
            _ => document
                .add_output_page(NewOutputPage {
                    process: "curvyo.laser",
                    label: "Laser",
                    profile: None,
                    work_area: DocumentSize::from_mm(400.0, 300.0),
                })
                .unwrap(),
        };
        ids.push(page.id);
    }
    let pages = document.pages();
    document.batch("scene", |doc| {
        for index in 0..1_000_u32 {
            let page = &pages[(index as usize % 7) * 3];
            let x = page.origin.x + 5.0 + f64::from(index % 20) * 8.0;
            let y = page.origin.y + 5.0 + f64::from(index / 20 % 25) * 10.0;
            let _ = doc.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 3.0, y + 3.0),
            ));
        }
    });
    let bytes = curvyo_document_core::pack(&document, "budget").unwrap();
    let mut session = Session::open(22, &bytes).unwrap();
    session.resize_viewport(1920.0, 1080.0);
    (session, ids)
}

#[test]
fn activating_pages_of_alternating_kinds_stays_within_50_ms_each() {
    let (mut session, ids) = scene();
    let _ = session.frame_draw_list();
    let mut worst = 0.0_f64;
    let mut kinds = Vec::new();
    for round in 0..50 {
        // Drawing, sketch and laser pages in turn.
        let id = ids[(round * 7 + 1) % ids.len()].clone();
        let ((), time) = cpu_time::cpu_time_of(|| {
            assert!(session.activate_page(&id));
            let _ = session.panel_view();
            let _ = session.pages_json();
            let _ = session.workbenches_json();
            let _ = session.manufacturing_view_json();
        });
        kinds.push(session.workbench());
        worst = worst.max(time.as_secs_f64());
    }
    assert!(kinds.contains(&Workbench::Vector));
    assert!(kinds.contains(&Workbench::Sketch));
    assert!(
        kinds
            .iter()
            .any(|w| matches!(w, Workbench::Manufacturing(_)))
    );
    assert!(
        kinds.windows(2).filter(|w| w[0] != w[1]).count() >= 40,
        "the kind alternates"
    );
    eprintln!("page activation: worst {worst:.4} s thread CPU");
    if !cfg!(debug_assertions) {
        assert!(worst <= BUDGET_S, "{worst:.4} s > {BUDGET_S} s");
    }
}
