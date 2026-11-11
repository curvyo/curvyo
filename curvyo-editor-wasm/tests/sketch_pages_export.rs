//! SVG export of sketch pages (`specs/0099-sketch-pages` criterion 18): a drawing page with a
//! linked clone of the sketch, a sketch page whose sketch has a closed outline with two holes, a
//! construction line, three dimensions and constraint symbols, and a drawing circle over the
//! sketch page. The goldens are in `curvyo-svg-core/tests/fixtures/export/sketch_pages/`;
//! regenerate with `CURVYO_UPDATE_GOLDEN=1` and review the diff.
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::{Path, PathBuf};

use curvyo_document_core::{Document, EllipseFrame, PageKind, Point, Style, Vec2};
use curvyo_editor_wasm::{Session, SvgExportResult};
use curvyo_sketch_core::{self as sk, Constraint, Expression, Position, Relation, Sketch};
use curvyo_ui_core::SvgExportOptions;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-svg-core/tests/fixtures/export/sketch_pages")
}

fn golden(path: &Path, text: &str) {
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let want = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display()));
    assert!(text == want, "{} differs from its golden", path.display());
}

/// A 80 x 50 mm plate with two 5 mm holes, a construction line, three dimensions and the
/// rectangle's horizontal and vertical relations.
fn plate() -> Sketch {
    let g = sk::create_rectangle(
        &Sketch::new("origin".into()),
        "plate",
        Position::mm(-40.0, -25.0),
        Position::mm(40.0, 25.0),
    )
    .unwrap();
    let g = sk::create_circle(&g, "left", Position::mm(-20.0, 0.0), sk::Length(5.0)).unwrap();
    let g = sk::create_circle(&g, "right", Position::mm(20.0, 0.0), sk::Length(5.0)).unwrap();
    let before: Vec<_> = g.entities.keys().cloned().collect();
    let mut g = sk::create_polyline(
        &g,
        "guide",
        &[Position::mm(-40.0, 0.0), Position::mm(40.0, 0.0)],
        false,
    )
    .unwrap();
    let guide = g
        .entities
        .iter()
        .find(|(id, e)| !before.contains(id) && matches!(e, sk::Entity::Line { .. }))
        .map(|(id, _)| id.clone())
        .unwrap();
    g.construction.insert(guide);
    let circles: Vec<_> = g
        .entities
        .iter()
        .filter(|(_, e)| matches!(e, sk::Entity::Circle { .. }))
        .map(|(id, _)| id.clone())
        .collect();
    let edge = g
        .entities
        .iter()
        .find(|(id, e)| id.0.starts_with("plate") && matches!(e, sk::Entity::Line { .. }))
        .map(|(id, _)| id.clone())
        .unwrap();
    let value = |text: &str, g: &Sketch| Expression::parse(text, &g.parameters).unwrap();
    for (n, circle) in circles.iter().enumerate() {
        let v = value("5 mm", &g);
        g.constraints.insert(
            format!("hole{n}").as_str().into(),
            Constraint::driving(Relation::Radius {
                curve: circle.clone(),
                value: v,
            }),
        );
    }
    let v = value("80 mm", &g);
    g.constraints.insert(
        "width".into(),
        Constraint::driving(Relation::Length {
            curve: edge,
            value: v,
        }),
    );
    g
}

fn project() -> Vec<u8> {
    let d = Document::new(990);
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = d
        .create_page_sketch(&page.id, &plate(), &Style::default())
        .unwrap();
    let first = &d.pages()[0];
    let to = Vec2::new(
        first.origin.x + first.size.width.as_mm() / 2.0
            - (page.origin.x + page.size.width.as_mm() / 2.0),
        first.origin.y + first.size.height.as_mm() / 2.0
            - (page.origin.y + page.size.height.as_mm() / 2.0),
    );
    d.place_linked_clone(sketch, to).unwrap();
    let _ = d.create_ellipse(EllipseFrame::from_corners(
        Point::new(page.origin.x + 12.0, page.origin.y + 12.0),
        Point::new(page.origin.x + 28.0, page.origin.y + 28.0),
    ));
    curvyo_document_core::pack(&d, "golden").unwrap()
}

fn export(s: &Session, active_page_only: bool) -> String {
    let options = SvgExportOptions {
        active_page_only,
        ..SvgExportOptions::default()
    };
    match s.export_svg(options) {
        SvgExportResult::Written { text, .. } => text,
        SvgExportResult::NothingSelected => panic!("nothing selected"),
        SvgExportResult::UnsupportedExtendedPath => panic!("unexpected unsupported extended path"),
    }
}

fn elements(svg: &str) -> usize {
    ["<path", "<circle", "<ellipse", "<line", "<use"]
        .iter()
        .map(|tag| svg.matches(tag).count())
        .sum()
}

#[test]
fn c18_sketch_pages_export_by_the_membership_read() {
    let mut s = Session::open(991, &project()).unwrap();
    s.resize_viewport(1200.0, 800.0);
    let pages: Vec<serde_json::Value> = serde_json::from_str(&s.pages_json()).unwrap();
    let sketch_page = pages.iter().find(|p| p["kind"] == "sketch").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let drawing_page = pages[0]["id"].as_str().unwrap().to_owned();
    // (a), (b), (c): the sketch page holds exactly its sketch: four edges and two holes, no
    // construction line, no dimension, no symbol, and not the circle over it.
    s.activate_page(&sketch_page);
    let sketch_only = export(&s, true);
    assert_eq!(elements(&sketch_only), 6, "{sketch_only}");
    golden(
        &golden_dir().join("active_sketch_page.exported.svg"),
        &sketch_only,
    );
    // (c): the whole plane writes every object at its coordinates, the circle included.
    let whole = export(&s, false);
    assert_eq!(
        elements(&whole),
        6 + 1 + 1,
        "sketch, clone, circle: {whole}"
    );
    golden(&golden_dir().join("whole_plane.exported.svg"), &whole);
    // (d): the drawing page alone: the clone as a path, the sketch not written.
    s.activate_page(&drawing_page);
    let drawing_only = export(&s, true);
    assert_eq!(elements(&drawing_only), 1, "{drawing_only}");
    assert!(!drawing_only.contains("<use"));
    golden(
        &golden_dir().join("active_drawing_page.exported.svg"),
        &drawing_only,
    );
}
