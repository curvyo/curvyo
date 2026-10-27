//! Independent tester tests for `specs/0024-svg-import-export` (import slice) at the editor
//! session: Open of a real Inkscape 1.4.4 file, Import SVG… as one undo step, refusals that
//! change nothing, even-odd re-winding and group depth in the document. Written from the
//! specification before the implementation was read; only the names of the public seams were
//! looked up. Real files live in `curvyo-svg-core/tests/fixtures/svg_real/` (see its
//! `SOURCES.md`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::case_sensitive_file_extension_comparisons,
    missing_docs
)]

use std::path::{Path, PathBuf};

use curvyo_document_core::{
    BackgroundPaint, Document, NodeKind, NodeRead, ObjectSnapshot, Point, Shape, unpack,
};
use curvyo_editor_wasm::Session;
use curvyo_ui_core::object_outline_bounds;

fn real_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../curvyo-svg-core/tests/fixtures/svg_real")
}

fn real(rel: &str) -> Vec<u8> {
    std::fs::read(real_dir().join(rel)).unwrap()
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn name(n: &NodeRead) -> Option<&str> {
    n.registers.name.as_deref()
}

/// The bounds of all objects (hidden ones included), outlines without stroke.
fn all_bounds(objects: &[ObjectSnapshot]) -> (Point, Point) {
    objects.iter().map(object_outline_bounds).fold(
        (
            Point::new(f64::MAX, f64::MAX),
            Point::new(f64::MIN, f64::MIN),
        ),
        |(a, b), (l, h)| {
            (
                Point::new(a.x.min(l.x), a.y.min(l.y)),
                Point::new(b.x.max(h.x), b.y.max(h.y)),
            )
        },
    )
}

fn signed_area(points: &[Point]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

const ENTITY_BOMB: &[u8] = br#"<?xml version="1.0"?>
<!DOCTYPE svg [
<!ENTITY a "aaaaaaaaaa">
<!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">
<!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">
]>
<svg xmlns="http://www.w3.org/2000/svg"><desc>&c;</desc><rect width="1" height="1"/></svg>"#;

const ENTITY_TEXT: &str = "This file defines XML entities, which Curvyo does not read because they can be used to exhaust memory. Nothing was changed.";

/// Criteria 2, 15, 20, 26 to 28, 33, 45, 47 in the document: Open of a genuine Inkscape 1.4.4
/// file gives an untitled, dirty project with an empty history, Inkscape's page, mm display
/// unit, no background (page opacity 0), its four layers with their states, primitives where
/// exact, paths where not, and four linked clones.
#[test]
fn ac2_open_of_a_real_inkscape_file() {
    let mut s = Session::open_file(
        7,
        &real("inkscape/inkscape_mm_a4.svg"),
        "inkscape_mm_a4.svg",
    )
    .unwrap();
    assert_eq!(s.take_open_notice(), "");
    assert!(s.is_dirty() && s.opened_svg());
    assert_eq!(s.history_len(), 0, "the history is empty");
    let d = doc_of(&s);
    assert!(
        close(d.size().width.as_mm(), 210.0, 1e-9) && close(d.size().height.as_mm(), 297.0, 1e-9)
    );
    assert_eq!(format!("{:?}", d.display_unit()), "Mm");
    assert_eq!(
        d.background().paint,
        BackgroundPaint::None,
        "pageopacity 0 is no background"
    );

    let nodes = d.read_nodes();
    let layers: Vec<&NodeRead> = nodes.iter().filter(|n| n.depth == 0).collect();
    assert!(layers.iter().all(|n| matches!(n.kind, NodeKind::Layer)));
    let names: Vec<_> = layers.iter().map(|n| name(n).unwrap()).collect();
    assert_eq!(
        names,
        ["Cut", "Engrave", "Reference", "Empty"],
        "file order, first at the bottom"
    );
    let engrave = layers[1];
    assert!(engrave.registers.locked && close(engrave.registers.opacity.get(), 0.5, 1e-9));
    assert!(!layers[2].registers.visible, "Reference is hidden");

    let leaf = |label: &str| -> &ObjectSnapshot {
        nodes
            .iter()
            .find(|n| name(n) == Some(label))
            .and_then(|n| match &n.kind {
                NodeKind::Leaf(o) => Some(o),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{label}"))
    };
    let is_prim = |o: &ObjectSnapshot, f: fn(&Shape) -> bool| matches!(o, ObjectSnapshot::Primitive(p) if f(&p.shape));
    assert!(is_prim(leaf("r1"), |s| matches!(s, Shape::Rect { .. })));
    assert!(
        is_prim(leaf("r2"), |s| matches!(s, Shape::Rect { .. })),
        "rounded rect stays"
    );
    assert!(
        matches!(leaf("r3"), ObjectSnapshot::Path(_)),
        "rx != ry becomes a path"
    );
    let ObjectSnapshot::Primitive(r4) = leaf("r4") else {
        panic!("the rotated rectangle stays a rectangle")
    };
    assert!(close(r4.rotation.as_radians().to_degrees(), 30.0, 1e-6));
    assert!(is_prim(leaf("c1"), |s| matches!(s, Shape::Ellipse { .. })));
    assert!(is_prim(leaf("e1"), |s| matches!(s, Shape::Ellipse { .. })));
    assert!(
        is_prim(leaf("s1"), |s| matches!(s, Shape::Star { .. })),
        "Inkscape star"
    );
    assert!(
        is_prim(leaf("s2"), |s| matches!(s, Shape::Polygon { .. })),
        "Inkscape polygon"
    );
    for clone in [
        "clone_moved",
        "clone_rot30",
        "clone_flip",
        "clone_in_rot_group",
    ] {
        assert!(matches!(leaf(clone), ObjectSnapshot::Clone(_)), "{clone}");
    }
    for copy in ["clone_scaled", "clone_of_rect"] {
        assert!(!matches!(leaf(copy), ObjectSnapshot::Clone(_)), "{copy}");
    }
}

/// Criterion 50: Inkscape's Path > Combine of two circles drawn the same way with
/// `fill-rule:evenodd` is re-wound so the inner outline runs the other way (a hole with
/// nonzero); the overlapping pair (criterion 51) stays as written.
#[test]
fn ac50_ac51_real_inkscape_combine_is_rewound() {
    let s = Session::open_file(
        7,
        &real("inkscape/inkscape_mm_a4.svg"),
        "inkscape_mm_a4.svg",
    )
    .unwrap();
    let nodes = doc_of(&s).read_nodes();
    let path = |label: &str| {
        nodes
            .iter()
            .find(|n| name(n) == Some(label))
            .and_then(|n| match &n.kind {
                NodeKind::Leaf(ObjectSnapshot::Path(p)) => Some(p.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{label}"))
    };
    let donut = path("donut");
    assert_eq!(donut.extra_subpaths.len(), 1);
    let a: Vec<Point> = donut.anchors.iter().map(|a| a.point).collect();
    let b: Vec<Point> = donut.extra_subpaths[0]
        .anchors
        .iter()
        .map(|a| a.point)
        .collect();
    assert!(
        signed_area(&a) * signed_area(&b) < 0.0,
        "inner and outer wound opposite"
    );
    // In the file both squares of "overlap" are written the same way: kept as written.
    let overlap = path("overlap");
    let a: Vec<Point> = overlap.anchors.iter().map(|a| a.point).collect();
    let b: Vec<Point> = overlap.extra_subpaths[0]
        .anchors
        .iter()
        .map(|a| a.point)
        .collect();
    assert!(
        signed_area(&a) * signed_area(&b) > 0.0,
        "crossing outlines are not re-wound"
    );
}

/// Criterion 3: Import SVG… of a real file into a project with content: one new top layer named
/// after the file, the file's layers as groups inside it, the top-level children selected,
/// nothing scaled, centred on the page centre without a canvas, one history step "Import SVG";
/// Ctrl+Z removes all of it, Ctrl+Shift+Z brings it back.
#[test]
fn ac3_import_of_a_real_file_is_one_undo_step() {
    let mut s = Session::new(5);
    // Something already in the project.
    let _ = s
        .import_svg(&real("wikimedia/Flag_of_Japan.svg"), "Flag_of_Japan.svg")
        .unwrap();
    let before_steps = s.history_len();
    let before_selected = s.selected_object_count();
    let before = doc_of(&s);
    let before_objects = before.read_objects().objects.len();
    let page = before.size();

    let bytes = real("inkscape/inkscape_mm_a4.svg");
    // The file's own bounds: Open keeps the file's coordinates (criterion 2).
    let opened = doc_of(&Session::open_file(8, &bytes, "x.svg").unwrap());
    let (flo, fhi) = all_bounds(&opened.read_objects().objects);
    let file_bounds = (fhi.x - flo.x, fhi.y - flo.y);
    let outcome = s.import_svg(&bytes, "inkscape_mm_a4.svg").unwrap();
    assert!(
        format!("{outcome:?}").starts_with("Imported("),
        "{outcome:?}"
    );
    assert_eq!(s.history_len(), before_steps + 1, "one history step");
    let rows = s.history_rows_json(0, 1);
    assert!(rows.contains("Import SVG"), "{rows}");

    let d = doc_of(&s);
    let nodes = d.read_nodes();
    let top: Vec<&NodeRead> = nodes.iter().filter(|n| n.depth == 0).collect();
    let layer = *top.last().unwrap();
    assert_eq!(name(layer), Some("inkscape_mm_a4"));
    assert!(matches!(layer.kind, NodeKind::Layer));
    assert_eq!(s.active_layer(), Some(layer.id));
    let children: Vec<&NodeRead> = nodes
        .iter()
        .filter(|n| n.parent == Some(layer.id))
        .collect();
    let child_names: Vec<_> = children.iter().map(|n| name(n).unwrap_or("")).collect();
    assert_eq!(child_names, ["Cut", "Engrave", "Reference"]);
    // The empty layer became an empty group, which is not imported (criterion 44).
    assert!(
        children.iter().all(|n| matches!(n.kind, NodeKind::Group)),
        "layers of the file become groups"
    );
    // Top-level children are selected; a locked (Engrave) or hidden (Reference) child cannot be
    // selected (`0050`), so only Cut is.
    let selectable = children
        .iter()
        .filter(|n| n.registers.visible && !n.registers.locked)
        .count();
    assert_eq!(selectable, 1);
    assert_eq!(s.selected_object_count(), selectable);

    // Placement: a pure translation onto the page centre.
    let imported: Vec<ObjectSnapshot> = {
        let ids: std::collections::HashSet<_> = nodes
            .iter()
            .filter(|n| {
                let mut p = n.parent;
                while let Some(id) = p {
                    if id == layer.id {
                        return true;
                    }
                    p = nodes.iter().find(|m| m.id == id).and_then(|m| m.parent);
                }
                false
            })
            .map(|n| n.id)
            .collect();
        let read = d.read_objects();
        read.objects
            .iter()
            .filter(|o| ids.contains(&o.id()))
            .cloned()
            .collect()
    };
    let (lo, hi) = all_bounds(&imported);
    assert!(close(hi.x - lo.x, file_bounds.0, 1e-6), "nothing is scaled");
    assert!(close(hi.y - lo.y, file_bounds.1, 1e-6));
    assert!(close(
        f64::midpoint(lo.x, hi.x),
        page.width.as_mm() / 2.0,
        1e-6
    ));
    assert!(close(
        f64::midpoint(lo.y, hi.y),
        page.height.as_mm() / 2.0,
        1e-6
    ));

    let _ = s.undo();
    let after_undo = doc_of(&s);
    assert_eq!(after_undo.read_objects().objects.len(), before_objects);
    assert!(
        after_undo
            .read_nodes()
            .iter()
            .all(|n| name(n) != Some("inkscape_mm_a4"))
    );
    assert_eq!(
        s.selected_object_count(),
        before_selected,
        "the import's selection goes with the undo"
    );
    let _ = s.redo();
    assert!(
        doc_of(&s)
            .read_nodes()
            .iter()
            .any(|n| name(n) == Some("inkscape_mm_a4"))
    );
}

/// Criteria 6 (b) and 68: a refused Import and a refused Open change nothing.
#[test]
fn ac6_refusals_change_nothing() {
    let mut s = Session::open_file(
        7,
        &real("inkscape/inkscape_mm_a4.svg"),
        "inkscape_mm_a4.svg",
    )
    .unwrap();
    let steps = s.history_len();
    let before = s.pack("0.1.0").unwrap();
    assert_eq!(
        s.import_svg(ENTITY_BOMB, "bomb.svg").err().as_deref(),
        Some(ENTITY_TEXT)
    );
    assert_eq!(
        s.import_svg(&real("w3c/coords-viewattr-01-b.svg"), "w3c.svg")
            .err()
            .as_deref(),
        Some(ENTITY_TEXT),
        "a real W3C file whose entities hold markup"
    );
    assert_eq!(s.history_len(), steps);
    assert_eq!(s.pack("0.1.0").unwrap(), before, "nothing was written");
    assert!(s.is_dirty() && s.opened_svg());
    assert_eq!(
        Session::open_file(7, ENTITY_BOMB, "bomb.svg")
            .err()
            .as_deref(),
        Some(ENTITY_TEXT)
    );
}

/// Criterion 8: an Import of a file that brings in nothing changes nothing and gives the one
/// notice, the file name cut at 24 characters with the ellipsis in the middle.
#[test]
fn ac8_nothing_imported_notice_and_long_name() {
    let mut s = Session::new(5);
    let steps = s.history_len();
    // Slice 2 (criterion 78) imports live text, so the file that brings in nothing holds a title.
    let only_text = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm"><title>Boho</title></svg>"#;
    let outcome = s
        .import_svg(only_text, "a-very-long-poster-file-name.svg")
        .unwrap();
    let text = format!("{outcome:?}");
    assert!(text.contains("could be imported."), "{text}");
    let shown = text
        .split("Nothing in ")
        .nth(1)
        .unwrap()
        .split(" could")
        .next()
        .unwrap();
    assert_eq!(shown.chars().count(), 24, "{shown}");
    assert!(shown.ends_with(".svg") && shown.contains('…'), "{shown}");
    assert_eq!(s.history_len(), steps);
}

/// Criterion 44 in the document: groups nested deeper than 32 levels merge into their parent, on
/// Open and on Import (where the import's own layer is one level).
#[test]
fn ac44_groups_deeper_than_32_merge() {
    let open = "<g>".repeat(40);
    let close_ = "</g>".repeat(40);
    let text = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10">{open}<rect width="1" height="1"/><rect x="2" width="1" height="1"/>{close_}</svg>"#
    );
    let s = Session::open_file(7, text.as_bytes(), "deep.svg").unwrap();
    let d = doc_of(&s);
    let nodes = d.read_nodes();
    assert_eq!(d.read_objects().objects.len(), 2);
    assert!(
        nodes.iter().all(|n| n.depth <= 32),
        "{}",
        nodes.iter().map(|n| n.depth).max().unwrap()
    );
    let mut s = Session::new(5);
    s.import_svg(text.as_bytes(), "deep.svg").unwrap();
    let d = doc_of(&s);
    assert_eq!(d.read_objects().objects.len(), 2);
    assert!(d.read_nodes().iter().all(|n| n.depth <= 32));
}

/// Criterion 77: a project opened from SVG, saved and opened again is as saved.
#[test]
fn ac77_open_save_reopen() {
    let s = Session::open_file(7, &real("inkscape/inkscape_in_css.svg"), "x.svg").unwrap();
    let d = doc_of(&s);
    let again = unpack(3, &curvyo_document_core::pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&*d.read_objects().objects).unwrap(),
        serde_json::to_value(&*again.read_objects().objects).unwrap()
    );
}
