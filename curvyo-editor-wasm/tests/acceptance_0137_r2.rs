//! Second acceptance round of `specs/0137-document-library` through the editor session: the
//! preview pane picks by outline and marquee (criterion 5), a drop imports at the point it lands
//! on, a page import sets the page's Came from (criteria 4, 6, 7), the text that left its path is
//! reported (criterion 5 notice, fix 1700aa54), and an SVG export writes nothing of Came from
//! (criterion 9). Written from the spec; the session is the public `Session`.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{
    Affine, CameFrom, CarrierRef, Document, Length, OnPathValues, Point, RectBounds,
    TextLayoutValues, UnixMillis, pack,
};
use curvyo_editor_wasm::{PreviewView, Session};

const PANE: (f64, f64) = (800.0, 600.0);
const NOW: i64 = 1_700_000_500_000;

fn record(page: &str, page_id: &str) -> CameFrom {
    CameFrom {
        document: "Box job".to_owned(),
        page: page.to_owned(),
        page_id: page_id.to_owned(),
        item: None,
        at: UnixMillis::new(1_700_000_000_000),
        online: None,
    }
}

/// A source with two squares on its first page: A at 0..10 and B at 40..50 (document mm).
fn source_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
    ));
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(40.0, 0.0),
        Point::new(50.0, 10.0),
    ));
    pack(&d, "0137-r2").unwrap()
}

fn two_squares() -> (Session, Vec<curvyo_document_core::NodeId>) {
    let session = Session::open(1, &source_bytes()).unwrap();
    let ids = session.page_top_level("first");
    (session, ids)
}

/// Criterion 5: a click picks the object under the pointer by outline, the other object by its
/// own outline, and empty paper picks nothing.
#[test]
fn ac5_a_click_in_the_pane_picks_the_object_under_it_by_outline() {
    let (session, ids) = two_squares();
    assert_eq!(ids.len(), 2, "both squares are pickable: {ids:?}");
    let t = session
        .preview_transform("first", PANE, PreviewView::default())
        .unwrap();
    let a = t.document_to_screen(Point::new(0.0, 5.0));
    let b = t.document_to_screen(Point::new(40.0, 5.0));
    let hit_a = session
        .preview_hit("first", PANE, PreviewView::default(), a)
        .expect("A is hit");
    let hit_b = session
        .preview_hit("first", PANE, PreviewView::default(), b)
        .expect("B is hit");
    assert_ne!(hit_a, hit_b, "each square picks its own object");
    assert!(ids.contains(&hit_a) && ids.contains(&hit_b));
    let far = t.document_to_screen(Point::new(25.0, 80.0));
    assert_eq!(
        session.preview_hit("first", PANE, PreviewView::default(), far),
        None,
        "empty paper picks nothing"
    );
}

/// Criterion 5: the marquee picks the objects it contains (rightward) and nothing outside it.
#[test]
fn ac5_the_marquee_picks_what_it_contains() {
    let (session, _) = two_squares();
    let t = session
        .preview_transform("first", PANE, PreviewView::default())
        .unwrap();
    let p = |x: f64, y: f64| t.document_to_screen(Point::new(x, y));
    let (a0, a1) = (p(-5.0, -5.0), p(60.0, 20.0));
    let all = session.preview_marquee("first", PANE, PreviewView::default(), (a0, a1));
    assert_eq!(all.len(), 2, "a marquee round both picks both: {all:?}");
    let (b0, b1) = (p(-5.0, -5.0), p(20.0, 20.0));
    let only_a = session.preview_marquee("first", PANE, PreviewView::default(), (b0, b1));
    assert_eq!(
        only_a.len(),
        1,
        "a marquee round the left square picks one: {only_a:?}"
    );
    assert_eq!(
        Some(only_a[0]),
        session.preview_hit("first", PANE, PreviewView::default(), p(0.0, 5.0)),
        "and it is the square under the pointer"
    );
}

/// Criterion 4 and 5: a drop imports the picked objects where the drop lands and sets their
/// Came from line; one import is one step.
#[test]
fn ac5_a_drop_imports_the_picked_square_with_its_came_from() {
    let (source, ids) = two_squares();
    let mut target = Session::new(2);
    let done = target
        .import_from_library(
            &source,
            &[ids[0]],
            None,
            &record("Page 1", "first"),
            Some((300.0, 200.0)),
        )
        .unwrap();
    assert_eq!(done.notice, None, "a plain copy has no notice");
    let line = target.came_from_view(NOW, 0);
    assert!(
        line.contains("From Box job"),
        "the copy's Came from line: {line}"
    );
}

/// Criterion 4: a page import makes a new page with the page's Came from record.
#[test]
fn ac4_a_page_import_sets_the_page_came_from_record() {
    let (source, _) = two_squares();
    let mut target = Session::new(2);
    let done = target
        .import_from_library(
            &source,
            &[],
            Some("first"),
            &record("Page 1", "first"),
            None,
        )
        .unwrap();
    let page = done.page.expect("the page is imported");
    let line = target.page_came_from_view(&page, NOW, 0);
    assert!(
        line.contains("Box job"),
        "the page's Came from line: {line}"
    );
}

/// Criterion 5 and the 1700aa54 fix: a text on a path imported without its carrier says so.
#[test]
fn ac5_a_text_that_left_its_path_is_reported_in_the_notice() {
    let d = Document::new(1);
    let carrier = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(30.0, 30.0),
    ));
    let text = d
        .create_text(
            "Along",
            &curvyo_document_core::FontRef {
                family: "Source Sans 3".to_owned(),
                style: "Regular".to_owned(),
                face_hash: "e".repeat(64),
                face_index: 0,
                kind: curvyo_document_core::FontKind::Outline,
            },
            Length::from_mm(5.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    d.put_text_on_path(
        text,
        CarrierRef {
            node: carrier,
            outline: 0,
        },
        &OnPathValues::at(Length::from_mm(0.0)),
        false,
    )
    .unwrap();
    let source = Session::open(3, &pack(&d, "0137-r2").unwrap()).unwrap();
    let mut target = Session::new(4);
    let done = target
        .import_from_library(
            &source,
            &[text],
            None,
            &record("Page 1", "first"),
            Some((10.0, 10.0)),
        )
        .unwrap();
    let notice = done.notice.expect("the text left its path: a notice");
    assert!(notice.contains("text left its path"), "{notice}");
}

/// Criterion 9: the SVG export writes nothing of Came from, though the objects carry it.
#[test]
fn ac9_svg_export_writes_nothing_of_came_from() {
    let (source, ids) = two_squares();
    let mut target = Session::new(5);
    let _ = target
        .import_from_library(
            &source,
            &[ids[0]],
            None,
            &record("Page 1", "first"),
            Some((10.0, 10.0)),
        )
        .unwrap();
    let result = target.export_svg(curvyo_ui_core::SvgExportOptions::default());
    let curvyo_editor_wasm::SvgExportResult::Written { text, objects, .. } = result else {
        panic!("the drawing is exported");
    };
    assert!(objects >= 1, "the copy is written");
    assert!(!text.contains("Box job"), "no document name in the SVG");
    assert!(!text.contains("From "), "no Came from line in the SVG");
    assert!(!text.contains("came"), "no record key in the SVG");
}
