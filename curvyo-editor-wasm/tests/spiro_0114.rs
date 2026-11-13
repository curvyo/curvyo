//! `specs/0114-spiro-smoothing-and-width-handles` at the session: a Node drag of a Spiro whose
//! iteration does not converge stores and reopens what it showed (criterion 2), and the SVG export
//! of Spiro paths (criterion 1). The golden is in `tests/fixtures/export/spiro_0114/`; regenerate
//! with `CURVYO_UPDATE_GOLDEN=1` and review the diff.
#![allow(clippy::unwrap_used, clippy::assert_is_empty, missing_docs)]

use std::path::{Path, PathBuf};

use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, Point, pack};
use curvyo_editor_wasm::{Session, SvgExportResult, Tool};
use curvyo_render_core::{DrawList, Vertex};
use curvyo_ui_core::SvgExportOptions;

fn golden(path: &Path, text: &str) {
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let want = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display()));
    assert!(text == want, "{} differs from its golden", path.display());
}

fn smooth(peer: u64, points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points
        .iter()
        .zip(1..)
        .map(|(&(x, y), k)| {
            let mut anchor = NewAnchor::corner(AnchorId::new(peer, k), Point::new(x, y));
            anchor.kind = AnchorKind::Symmetric;
            anchor
        })
        .collect()
}

/// A tight zigzag of nine controls (eight segments): its G2 iteration does not converge in ten
/// steps, so it is drawn as the last iterate (criterion 2).
fn zigzag() -> Vec<(f64, f64)> {
    (0..9)
        .map(|i| {
            (
                20.0 + 4.0 * f64::from(i),
                if i % 2 == 0 { 40.0 } else { 50.0 },
            )
        })
        .collect()
}

fn artwork(list: &DrawList) -> Vec<Vertex> {
    let end = list.layers().last().copied().unwrap_or(0);
    list.triangles[..end].to_vec()
}

/// The farthest any triangle vertex of one drawing lies from the vertices of the other.
fn distance(a: &[Vertex], b: &[Vertex]) -> f64 {
    let reach = |a: &[Vertex], b: &[Vertex]| {
        a.iter()
            .map(|p| {
                b.iter()
                    .map(|q| (p.position.x - q.position.x).hypot(p.position.y - q.position.y))
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0, f64::max)
    };
    reach(a, b).max(reach(b, a))
}

#[test]
fn the_zigzag_is_a_finite_curvature_continuous_curve() {
    let shape = curvyo_ui_core::solve_spiro_nodes(&smooth(1, &zigzag()), false).unwrap();
    assert!(
        shape
            .segments()
            .iter()
            .all(|k| k.iter().all(|c| c.is_finite()) && k[2] == 0.0 && k[3] == 0.0),
        "clothoids: {:?}",
        shape.segments()
    );
}

/// Criterion 2: "what is shown is what a reopen draws", for a Node drag of a curve whose iteration
/// does not converge (architect review 2026-11-12): the last frame of the drag, the committed
/// path and a reopen draw the same curve.
#[test]
fn a_node_drag_of_an_unconverged_spiro_stores_and_reopens_what_it_showed() {
    let doc = Document::new(5);
    let points = zigzag();
    doc.create_editable_curve(&smooth(5, &points), false, CurveKind::Spiro)
        .unwrap();
    let mut s = Session::open(6, &pack(&doc, "test").unwrap()).unwrap();
    s.resize_viewport(1200.0, 800.0);
    let grab = Point::new(points[4].0, points[4].1);
    s.set_tool(Tool::Select);
    s.pointer_hover(grab, false, false);
    s.pointer_down(grab, false);
    s.pointer_up(grab, false, false);
    assert_eq!(s.selected_object_count(), 1);
    s.set_tool(Tool::Node);
    let _ = s.draw_list();
    s.pointer_down(grab, false);
    let mut shown = Vec::new();
    for step in 1..=8 {
        let at = Point::new(
            grab.x + 0.25 * f64::from(step),
            grab.y - 0.5 * f64::from(step),
        );
        s.pointer_hover(at, false, false);
        shown = artwork(&s.draw_list());
    }
    let end = Point::new(grab.x + 2.0, grab.y - 4.0);
    s.pointer_up(end, false, false);
    let committed = artwork(&s.draw_list());
    assert!(!committed.is_empty());
    assert!(
        distance(&shown, &committed) < 0.01,
        "the committed curve is the one shown"
    );
    let mut reopened = Session::open(7, &s.pack("test").unwrap()).unwrap();
    reopened.resize_viewport(1200.0, 800.0);
    assert!(
        distance(&committed, &artwork(&reopened.draw_list())) < 0.01,
        "a reopen draws the committed curve"
    );
}

/// Criterion 1: Spiro paths export as their G2 curve (an open wave, a closed ring and the zigzag
/// as its last iterate), each as its evaluated cubic spans.
#[test]
fn spiro_paths_export_as_their_curves() {
    let doc = Document::new(8);
    let wave = [
        (10.0, 20.0),
        (30.0, 10.0),
        (50.0, 20.0),
        (70.0, 10.0),
        (90.0, 20.0),
    ];
    let ring = [(40.0, 70.0), (60.0, 80.0), (40.0, 90.0), (20.0, 80.0)];
    let mut minted = 0;
    let mut add = |points: &[(f64, f64)], closed: bool| {
        let mut anchors = smooth(8, points);
        for anchor in &mut anchors {
            minted += 1;
            anchor.id = AnchorId::new(8, minted);
        }
        doc.create_editable_curve(&anchors, closed, CurveKind::Spiro)
            .unwrap();
    };
    add(&wave, false);
    add(&ring, true);
    add(&zigzag(), false);
    let s = Session::open(9, &pack(&doc, "test").unwrap()).unwrap();
    let SvgExportResult::Written { text, .. } = s.export_svg(SvgExportOptions::default()) else {
        panic!("SVG not written")
    };
    golden(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/export/spiro_0114/spiro.exported.svg"),
        &text,
    );
}
