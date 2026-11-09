//! SVG export of picture clones and placed pages (`specs/0100-bring-content-in` criterion 20,
//! ADR 0045 §8): a drawing page with a group G (a text, a path and an image), a laser page with a
//! clone of G turned by 30 degrees, a placed page of the drawing page turned by 90 degrees, an
//! own rectangle and an engraved grid. The goldens are in
//! `curvyo-svg-core/tests/fixtures/export/bring_content_in/`; regenerate with
//! `CURVYO_UPDATE_GOLDEN=1` and review the diff.
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, Angle, Document, DocumentSize, FontKind, FontRef, Length, NewAnchor,
    NewImage, NewOutputPage, ObjectSnapshot, Point, RectBounds, TextLayoutValues, Vec2,
};
use curvyo_editor_wasm::{Session, SvgExportResult};
use curvyo_ui_core::SvgExportOptions;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-svg-core/tests/fixtures/export/bring_content_in")
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

fn turn(degrees: f64, about: Point) -> Affine {
    Affine::rotation_about(about, Angle::from_radians(degrees.to_radians()))
}

/// The project of criterion 20; with `drawing_clone`, also a clone of G on the drawing page
/// (criterion 20 d).
fn project(drawing_clone: bool) -> Vec<u8> {
    let d = Document::new(1000);
    let drawing = d.pages()[0].clone();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let text = d
        .create_text(
            "ANNA",
            &FontRef {
                family: "Source Sans 3".into(),
                style: "Regular".into(),
                face_hash: "c".repeat(64),
                face_index: 0,
                kind: FontKind::Outline,
            },
            Length::from_mm(10.0),
            Affine::translation(Vec2::new(20.0, 60.0)),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let c = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1000, k), Point::new(x, y));
    let path = d.create_path(
        &[c(1, 20.0, 20.0), c(2, 60.0, 20.0), c(3, 40.0, 50.0)],
        true,
    );
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png"),
    )
    .unwrap();
    let image = d
        .add_images(&[NewImage {
            bytes: Arc::from(bytes),
            placement: Affine::new(30.0, 0.0, 0.0, 20.0, 70.0, 20.0),
            name: "photo".into(),
        }])
        .unwrap()[0];
    let g = d.group_objects(&[text, path, image]).unwrap();
    let onto = Vec2::new(
        laser.origin.x - drawing.origin.x + 40.0,
        laser.origin.y - drawing.origin.y + 150.0,
    );
    let clone = d.place_linked_clones(&[g], onto).unwrap()[0];
    let ObjectSnapshot::Clone(read) = d.object(clone).unwrap() else {
        panic!("a clone")
    };
    let centre = Point::new(laser.origin.x + 100.0, laser.origin.y + 180.0);
    d.rotate_object(&ObjectSnapshot::Clone(read.mapped_by(turn(30.0, centre))))
        .unwrap();
    let turned =
        Affine::translation(Vec2::new(380.0, 10.0)).compose(turn(90.0, Point::new(0.0, 0.0)));
    d.place_page(&laser.id, &drawing.id, turned).unwrap();
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(laser.origin.x + 250.0, laser.origin.y + 200.0),
        Point::new(laser.origin.x + 300.0, laser.origin.y + 250.0),
    ));
    let manufacturing = laser.kind.manufacturing().unwrap();
    let params = serde_json::from_value(serde_json::json!({
        "spacing": {"mm": 10.0},
        "line_width": {"mm": 0.1},
        "origin_x": {"mm": 0.0},
        "origin_y": {"mm": 0.0},
    }))
    .unwrap();
    d.add_fixture(
        manufacturing,
        curvyo_laser_core::ENGRAVED_GRID,
        "Engraved grid",
        &params,
    )
    .unwrap();
    if drawing_clone {
        d.linked_clone(&[g], &[]).unwrap();
    }
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
    }
}

#[test]
fn c20_picture_clones_and_placed_pages_are_written_as_groups() {
    let mut s = Session::open(1001, &project(false)).unwrap();
    s.resize_viewport(1200.0, 800.0);
    let pages: Vec<serde_json::Value> = serde_json::from_str(&s.pages_json()).unwrap();
    let laser = pages
        .iter()
        .find(|p| p["name"].as_str().is_some_and(|n| n.starts_with("Laser")))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    // (a) Active page only, the laser page: the rectangle, the clone's leaves and the placed
    // page's members; no fixture, origin marker or placement box.
    s.activate_page(&laser);
    let active = export(&s, true);
    assert!(active.contains("Placed page of"), "{active}");
    assert_eq!(
        active.matches("<image").count(),
        2,
        "the clone's and the placed page's"
    );
    assert!(!active.contains("<use"));
    golden(
        &golden_dir().join("active_laser_page.exported.svg"),
        &active,
    );
    // (b) The whole plane: G on its drawing page and, in addition, the clone and the placed
    // page's members.
    let whole = export(&s, false);
    assert_eq!(
        whole.matches("<image").count(),
        3,
        "G's, the clone's, the placed page's"
    );
    golden(&golden_dir().join("whole_plane.exported.svg"), &whole);
    // (d) A clone of a group on a drawing page is written the same way.
    let mut s = Session::open(1002, &project(true)).unwrap();
    s.resize_viewport(1200.0, 800.0);
    let drawing = export(&s, true);
    assert_eq!(drawing.matches("<image").count(), 2, "G's and its clone's");
    assert!(!drawing.contains("<use"));
    golden(
        &golden_dir().join("active_drawing_page.exported.svg"),
        &drawing,
    );
}
