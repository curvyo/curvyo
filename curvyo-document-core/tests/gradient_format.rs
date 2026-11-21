//! The stored gradient fill (`specs/0124-gradient-fill` criteria 2, 3, 6 to 8, 16 to 18, 22, 23;
//! ADR 0066 §1, §2, §7, §8): the style edits and their History labels, the merge of a drag with a
//! stop edit, copies and clones, format 49, the golden `gradient_fill_v49.curvyo` and one damaged
//! golden per refusal reason. `CURVYO_WRITE_FIXTURES=1` rewrites the goldens of this file.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::needless_pass_by_value
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, AxisRatio, BoxPoint, CURRENT_FORMAT_VERSION, Color, CopySource, Document,
    EllipseFrame, Fill, FillPaint, FontKind, FontRef, GradientFill, GradientGeometry, GradientKind,
    GradientStop, GradientStops, Length, NewAnchor, NodeId, Opacity, OpenError, Point, RectBounds,
    StopOffset, StyleEdit, StyleEditError, TextLayoutValues, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

const GOLDEN: &str = "gradient_fill_v49.curvyo";

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(name), build()).unwrap();
    }
    std::fs::read(fixture_path(name)).unwrap()
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn rezip(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

fn tree_id(id: NodeId) -> TreeID {
    let v = serde_json::to_value(id).unwrap();
    TreeID::new(
        v["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(v["counter"].as_i64().unwrap()).unwrap(),
    )
}

/// `bytes` with the register `key` of object `id` set to `value` (or removed for `None`).
fn with_register(bytes: &[u8], id: NodeId, key: &str, value: Option<LoroValue>) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(id)).unwrap();
    match value {
        Some(value) => meta.insert(key, value).unwrap(),
        None => meta.delete(key).unwrap(),
    }
    loro.commit();
    let mut files = members(bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

fn register(document: &Document, id: NodeId, key: &str) -> Option<LoroValue> {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(id)).unwrap();
    meta.get(key).map(|v| v.get_deep_value())
}

fn ops_of(document: &Document) -> usize {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro.len_ops()
}

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(100.0),
        height: Length::from_mm(40.0),
    })
}

fn path(document: &Document, first: u64, points: &[(f64, f64)], closed: bool) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .zip(first..)
        .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), Point::new(*x, *y)))
        .collect();
    document.create_path(&anchors, closed)
}

fn fill_of(document: &Document, id: NodeId) -> Fill {
    document.object(id).unwrap().style().fill.clone()
}

fn gradient_of(document: &Document, id: NodeId) -> GradientFill {
    fill_of(document, id).gradient.expect("a gradient")
}

fn edit(document: &Document, id: NodeId, edit: StyleEdit) {
    document.edit_style(&[id], &edit).unwrap();
}

fn labels(document: &Document) -> Vec<String> {
    document.own_steps().into_iter().map(|s| s.label).collect()
}

fn last_label(document: &Document) -> String {
    labels(document).pop().unwrap()
}

fn undo_last(document: &Document) {
    let step = document.own_steps().last().unwrap().step;
    document.undo_step(step).unwrap();
}

fn stop(r: u8, g: u8, b: u8, alpha: u8, offset: f64) -> GradientStop {
    GradientStop {
        colour: Color { r, g, b },
        alpha,
        offset: StopOffset::new(offset).unwrap(),
    }
}

fn red() -> Color {
    Color { r: 255, g: 0, b: 0 }
}

// ---------------------------------------------------------------------------------------------
// Criterion 2, 18: the paint switch
// ---------------------------------------------------------------------------------------------

#[test]
fn choosing_gradient_starts_from_the_colour_as_one_step() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    edit(
        &d,
        a,
        StyleEdit::FillRgba(red(), Opacity::new(0.5).unwrap()),
    );
    let steps = labels(&d).len();
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    assert_eq!(labels(&d).len(), steps + 1, "one step");
    assert_eq!(last_label(&d), "fill_gradient");
    let fill = fill_of(&d, a);
    assert!(fill.enabled && fill.paint == FillPaint::Gradient);
    assert_eq!(fill.painted_gradient(), fill.gradient.as_ref());
    let gradient = fill.gradient.unwrap();
    assert_eq!(gradient.geometry, GradientGeometry::DEFAULT_LINEAR);
    assert_eq!(
        gradient.stops,
        GradientStops::two((red(), 128), (Color::WHITE, 255))
    );
    assert_eq!(
        (fill.color, fill.opacity.get()),
        (red(), 0.5),
        "the Colour is untouched"
    );
    assert_eq!(register(&d, a, "fill_pattern"), None, "no pattern written");
}

#[test]
fn switching_away_keeps_the_gradient_and_switching_back_restores_it() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    edit(&d, a, StyleEdit::GradientKind(GradientKind::Radial));
    edit(&d, a, StyleEdit::AddGradientStop(None));
    let kept = gradient_of(&d, a);
    for paint in [FillPaint::Colour, FillPaint::Pattern] {
        edit(&d, a, StyleEdit::FillPaint(paint));
        assert_eq!(fill_of(&d, a).gradient.as_ref(), Some(&kept));
        assert_eq!(fill_of(&d, a).painted_gradient(), None);
        let ops = ops_of(&d);
        edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
        assert_eq!(ops_of(&d) - ops, 1, "only the paint register is written");
        assert_eq!(gradient_of(&d, a), kept);
    }
    edit(&d, a, StyleEdit::FillEnabled(false));
    assert!(!fill_of(&d, a).paints());
    assert_eq!(fill_of(&d, a).gradient, Some(kept));
}

// ---------------------------------------------------------------------------------------------
// Criteria 3, 6, 18: the gradient edits, their labels and undo
// ---------------------------------------------------------------------------------------------

#[test]
fn every_gradient_edit_is_one_named_step_and_undo_restores_it() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    let cases = [
        (
            StyleEdit::GradientKind(GradientKind::Radial),
            "gradient_radial",
        ),
        (
            StyleEdit::GradientKind(GradientKind::Linear),
            "gradient_linear",
        ),
        (StyleEdit::AddGradientStop(None), "add_gradient_stop"),
        (StyleEdit::GradientStopColour(1, red(), 9), "edit_gradient"),
        (StyleEdit::GradientStopOffset(1, 0.25), "edit_gradient"),
        (StyleEdit::ReverseGradient, "reverse_gradient"),
        (StyleEdit::RemoveGradientStop(1), "remove_gradient_stop"),
        (
            StyleEdit::GradientGeometry(GradientGeometry::Linear {
                start: BoxPoint::new(0.5, 0.0).unwrap(),
                end: BoxPoint::new(0.5, 1.0).unwrap(),
            }),
            "edit_gradient",
        ),
        (
            StyleEdit::GradientStops(GradientStops::two((red(), 255), (red(), 0))),
            "edit_gradient",
        ),
    ];
    for (change, label) in cases {
        let before = gradient_of(&d, a);
        edit(&d, a, change.clone());
        assert_eq!(change.commit_label(), label);
        assert_eq!(last_label(&d), label, "{change:?}");
        assert_ne!(
            gradient_of(&d, a),
            before,
            "{change:?} changes the gradient"
        );
        undo_last(&d);
        assert_eq!(gradient_of(&d, a), before, "{change:?} undone exactly");
        edit(&d, a, change);
    }
}

#[test]
fn the_kind_switch_keeps_the_stops_and_sets_the_kinds_default() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    edit(&d, a, StyleEdit::AddGradientStop(None));
    let stops = gradient_of(&d, a).stops;
    edit(&d, a, StyleEdit::GradientKind(GradientKind::Radial));
    let gradient = gradient_of(&d, a);
    assert_eq!(gradient.geometry, GradientGeometry::DEFAULT_RADIAL);
    assert_eq!(gradient.stops, stops);
    let ops = ops_of(&d);
    edit(&d, a, StyleEdit::GradientKind(GradientKind::Radial));
    assert_eq!(ops_of(&d), ops, "the same kind writes nothing");
}

#[test]
fn the_ends_are_never_removed_and_an_index_past_the_list_changes_nothing() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    let ops = ops_of(&d);
    for change in [
        StyleEdit::RemoveGradientStop(0),
        StyleEdit::RemoveGradientStop(1),
        StyleEdit::GradientStopColour(7, red(), 1),
        StyleEdit::GradientStopOffset(0, 0.5),
    ] {
        edit(&d, a, change);
    }
    assert_eq!(ops_of(&d), ops);
}

#[test]
fn a_gradient_edit_on_a_shape_without_one_changes_nothing() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let ops = ops_of(&d);
    edit(&d, a, StyleEdit::ReverseGradient);
    edit(&d, a, StyleEdit::AddGradientStop(None));
    assert_eq!(ops_of(&d), ops);
    assert!(StyleEdit::ReverseGradient.is_fill_edit());
    assert!(StyleEdit::GradientKind(GradientKind::Linear).is_gradient_edit());
}

/// Criterion 8: typing on several shapes is one step; Reverse reverses each shape's own list.
#[test]
fn a_selection_edit_is_one_step_and_reverse_is_per_shape() {
    let d = Document::new(1);
    let (a, b) = (rect(&d, 0.0), rect(&d, 120.0));
    d.edit_style(&[a, b], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    edit(&d, b, StyleEdit::AddGradientStop(None));
    let steps = labels(&d).len();
    d.edit_style(&[a, b], &StyleEdit::ReverseGradient).unwrap();
    assert_eq!(labels(&d).len(), steps + 1);
    assert_eq!(gradient_of(&d, a).stops.len(), 2);
    assert_eq!(gradient_of(&d, b).stops.len(), 3);
    assert_eq!(gradient_of(&d, a).stops.as_slice()[0].colour, Color::WHITE);
}

/// Criteria 3, 4, 8: Angle and Radius over shapes whose boxes differ, in one step.
#[test]
fn per_shape_geometries_are_one_step_that_writes_only_the_geometry() {
    let d = Document::new(1);
    let (a, b, plain) = (rect(&d, 0.0), rect(&d, 120.0), rect(&d, 240.0));
    d.edit_style(&[a, b], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    let up = |x: f64| GradientGeometry::Linear {
        start: BoxPoint::new(x, 1.0).unwrap(),
        end: BoxPoint::new(x, 0.0).unwrap(),
    };
    let (steps, ops) = (labels(&d).len(), ops_of(&d));
    d.set_gradient_geometries(&[(a, up(0.25)), (b, up(0.75)), (plain, up(0.5))])
        .unwrap();
    assert_eq!(labels(&d).len(), steps + 1);
    assert_eq!(last_label(&d), "edit_gradient");
    assert_eq!(
        ops_of(&d) - ops,
        2,
        "one register per shape with a gradient"
    );
    assert_eq!(gradient_of(&d, a).geometry, up(0.25));
    assert_eq!(gradient_of(&d, b).geometry, up(0.75));
    assert_eq!(
        fill_of(&d, plain).gradient,
        None,
        "a shape without one is left alone"
    );
    let steps = labels(&d).len();
    d.set_gradient_geometries(&[(a, up(0.25))]).unwrap();
    assert_eq!(labels(&d).len(), steps, "no change, no step");
    d.delete_objects(&[b]).unwrap();
    assert_eq!(
        d.set_gradient_geometries(&[(a, up(0.1)), (b, up(0.1))]),
        Err(StyleEditError::NoSuchObject)
    );
    assert_eq!(gradient_of(&d, a).geometry, up(0.25), "nothing written");
}

// ---------------------------------------------------------------------------------------------
// ADR 0066 §2: a drag on one peer and a stop edit on another both survive a merge
// ---------------------------------------------------------------------------------------------

#[test]
fn a_drag_and_a_stop_edit_on_two_peers_merge_without_loss() {
    let base = Document::new(1);
    let a = rect(&base, 0.0);
    edit(&base, a, StyleEdit::FillPaint(FillPaint::Gradient));
    let bytes = pack(&base, "t").unwrap();
    let (x, y) = (unpack(2, &bytes).unwrap(), unpack(3, &bytes).unwrap());
    let dragged = GradientGeometry::Linear {
        start: BoxPoint::new(0.1, 0.1).unwrap(),
        end: BoxPoint::new(0.9, 0.9).unwrap(),
    };
    x.set_gradient_geometries(&[(a, dragged)]).unwrap();
    edit(&y, a, StyleEdit::GradientStopColour(0, red(), 255));
    let (xv, yv) = (x.version(), y.version());
    let (xu, yu) = (
        x.export_updates(&yv).unwrap(),
        y.export_updates(&xv).unwrap(),
    );
    x.import_updates(&yu).unwrap();
    y.import_updates(&xu).unwrap();
    for merged in [&x, &y] {
        let gradient = gradient_of(merged, a);
        assert_eq!(gradient.geometry, dragged);
        assert_eq!(gradient.stops.as_slice()[0].colour, red());
    }
}

/// A clone with own style whose gradient was kept while the paint went back to Colour: one peer
/// picks Gradient again, the other resets the clone's own style from its root. Whatever wins, the merged file opens:
/// `fill_paint` `gradient` never stands without both registers (ADR 0066 §2, §7).
fn a_repick_racing_a_style_reset_merges_into_an_openable_file(
    peers: (u64, u64),
    pattern: bool,
    repick_later: bool,
) {
    let base = Document::new(1);
    let root = path(&base, 0, &[(0.0, 0.0), (80.0, 0.0), (80.0, 30.0)], true);
    let clone = base.linked_clone(&[root], &[]).unwrap()[0];
    base.set_own_style(&[clone], true).unwrap();
    if pattern {
        edit(&base, clone, StyleEdit::FillPaint(FillPaint::Pattern));
    }
    edit(&base, clone, StyleEdit::FillPaint(FillPaint::Gradient));
    edit(&base, clone, StyleEdit::FillPaint(FillPaint::Colour));
    let bytes = pack(&base, "t").unwrap();
    let (x, y) = (
        unpack(peers.0, &bytes).unwrap(),
        unpack(peers.1, &bytes).unwrap(),
    );
    // The peer with more own work first carries the later Lamport stamps.
    for k in 0..20 {
        rect(if repick_later { &x } else { &y }, f64::from(k));
    }
    edit(&x, clone, StyleEdit::FillPaint(FillPaint::Gradient));
    // Original style and Own style again: the clone's style keys are cleared and rewritten
    // from its root, which has no gradient.
    y.set_own_style(&[clone], false).unwrap();
    y.set_own_style(&[clone], true).unwrap();
    let (xv, yv) = (x.version(), y.version());
    let (xu, yu) = (
        x.export_updates(&yv).unwrap(),
        y.export_updates(&xv).unwrap(),
    );
    let label = format!("peers {peers:?}, pattern {pattern}, later {repick_later}");
    x.import_updates(&yu)
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
    y.import_updates(&xu)
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
    for merged in [&x, &y] {
        let reopened = pack(merged, "t").unwrap();
        assert!(
            unpack(9, &reopened).is_ok(),
            "peers {peers:?}, pattern {pattern}, later {repick_later}: the merged file opens"
        );
    }
}

#[test]
fn a_gradient_repick_racing_a_clone_style_reset_leaves_an_openable_file() {
    for peers in [(2, 3), (3, 2), (5, 4), (4, 5)] {
        for pattern in [false, true] {
            for later in [false, true] {
                a_repick_racing_a_style_reset_merges_into_an_openable_file(peers, pattern, later);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 16, 17: copies and clones
// ---------------------------------------------------------------------------------------------

#[test]
fn a_duplicate_has_the_same_gradient_and_editing_it_leaves_the_original() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    edit(&d, a, StyleEdit::AddGradientStop(None));
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap()[0];
    assert_eq!(fill_of(&d, copy), fill_of(&d, a));
    edit(&d, copy, StyleEdit::ReverseGradient);
    assert_ne!(gradient_of(&d, copy), gradient_of(&d, a));
}

#[test]
fn a_linked_clone_shows_its_originals_gradient_and_own_style_carries_its_own() {
    let d = Document::new(1);
    let a = path(&d, 1, &[(0.0, 0.0), (50.0, 0.0), (50.0, 30.0)], true);
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    let clone = d.linked_clone(&[a], &[]).unwrap()[0];
    assert_eq!(fill_of(&d, clone), fill_of(&d, a));
    edit(&d, a, StyleEdit::GradientKind(GradientKind::Radial));
    assert_eq!(
        gradient_of(&d, clone).geometry,
        GradientGeometry::DEFAULT_RADIAL
    );
    d.set_own_style(&[clone], true).unwrap();
    edit(&d, clone, StyleEdit::GradientKind(GradientKind::Linear));
    assert_eq!(
        gradient_of(&d, clone).geometry,
        GradientGeometry::DEFAULT_LINEAR
    );
    assert_eq!(
        gradient_of(&d, a).geometry,
        GradientGeometry::DEFAULT_RADIAL
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: format 49 and the goldens
// ---------------------------------------------------------------------------------------------

#[test]
fn the_format_is_49() {
    // Superseded by 0135 slice 1 (2026-11-19): format 51.
    assert_eq!(CURRENT_FORMAT_VERSION, 52);
}

fn font() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "a".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

/// The objects of the golden, in document order.
struct Golden {
    document: Document,
    linear: NodeId,
    radial: NodeId,
    ellipse: NodeId,
    nine: NodeId,
    open: NodeId,
    text: NodeId,
    clone: NodeId,
    own: NodeId,
}

fn nine_stops() -> GradientStops {
    GradientStops::new(vec![
        stop(0, 0, 0, 255, 0.0),
        stop(255, 0, 0, 255, 0.125),
        stop(255, 128, 0, 200, 0.25),
        stop(255, 255, 0, 255, 0.375),
        stop(0, 255, 0, 255, 0.5),
        stop(0, 255, 255, 255, 0.5),
        stop(0, 0, 255, 128, 0.75),
        stop(128, 0, 255, 255, 0.875),
        stop(255, 255, 255, 0, 1.0),
    ])
    .unwrap()
}

fn elliptical() -> GradientGeometry {
    GradientGeometry::Radial {
        centre: BoxPoint::new(0.4, 0.5).unwrap(),
        radius: BoxPoint::new(0.9, 0.25).unwrap(),
        ratio: AxisRatio::new(0.5).unwrap(),
    }
}

/// Every kind of the golden, in steps whose neighbours never share a label (`specs/0020-undo-redo`
/// criterion 36's check over the fixtures).
fn golden() -> Golden {
    let d = Document::new(1);
    let linear = rect(&d, 0.0);
    let radial = d.create_ellipse(EllipseFrame {
        center: Point::new(160.0, 20.0),
        rx: Length::from_mm(30.0),
        ry: Length::from_mm(20.0),
    });
    let ellipse = path(
        &d,
        10,
        &[(0.0, 60.0), (80.0, 60.0), (80.0, 100.0), (0.0, 100.0)],
        true,
    );
    let text = d
        .create_text(
            "Shade",
            &font(),
            Length::from_mm(10.0),
            Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 140.0),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let open = path(
        &d,
        20,
        &[(100.0, 60.0), (160.0, 100.0), (200.0, 60.0)],
        false,
    );
    let nine = rect(&d, 120.0);
    let original = path(&d, 30, &[(0.0, 200.0), (40.0, 200.0), (20.0, 240.0)], true);
    d.batch("edit_style", |d| {
        for id in [linear, radial, ellipse, nine, open, text, original] {
            edit(d, id, StyleEdit::FillPaint(FillPaint::Gradient));
        }
        edit(d, radial, StyleEdit::GradientKind(GradientKind::Radial));
        edit(d, ellipse, StyleEdit::GradientGeometry(elliptical()));
        edit(d, nine, StyleEdit::GradientStops(nine_stops()));
        edit(d, open, StyleEdit::GradientStopColour(0, red(), 255));
        edit(d, text, StyleEdit::GradientKind(GradientKind::Radial));
        edit(d, original, StyleEdit::AddGradientStop(None));
    });
    let own = d.linked_clone(&[original], &[]).unwrap()[0];
    d.set_own_style(&[own], true).unwrap();
    edit(&d, own, StyleEdit::ReverseGradient);
    let clone = d.linked_clone(&[original], &[]).unwrap()[0];
    Golden {
        document: d,
        linear,
        radial,
        ellipse,
        nine,
        open,
        text,
        clone,
        own,
    }
}

fn golden_bytes() -> Vec<u8> {
    pack(&golden().document, "0.1.0").unwrap()
}

#[test]
fn the_golden_reads_back_every_gradient() {
    let bytes = read_or_write(GOLDEN, golden_bytes);
    assert_eq!(format_of(&bytes), 49);
    let g = golden();
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    for id in [
        g.linear, g.radial, g.ellipse, g.nine, g.open, g.text, g.clone, g.own,
    ] {
        assert_eq!(
            fill_of(&opened, id),
            fill_of(&g.document, id),
            "{id:?} reads back"
        );
        assert!(
            fill_of(&opened, id).painted_gradient().is_some(),
            "{id:?} paints"
        );
    }
    assert_eq!(
        gradient_of(&opened, g.linear).geometry,
        GradientGeometry::DEFAULT_LINEAR
    );
    assert_eq!(
        gradient_of(&opened, g.radial).geometry,
        GradientGeometry::DEFAULT_RADIAL
    );
    assert_eq!(gradient_of(&opened, g.ellipse).geometry, elliptical());
    assert_eq!(gradient_of(&opened, g.nine).stops, nine_stops());
    assert_eq!(gradient_of(&opened, g.clone).stops.len(), 3);
    assert_ne!(gradient_of(&opened, g.own), gradient_of(&opened, g.clone));
    let saved = pack(&opened, "0.1.0").unwrap();
    // Saved by a build of format 51 (`0135`; Superseded by 0135 slice 1 (2026-11-19)): the same content under its own version.
    let json = |bytes: &[u8]| {
        String::from_utf8(members(bytes)["document.json"].clone())
            .unwrap()
            .replacen("\"format_version\": 52,", "\"format_version\": 49,", 1)
    };
    assert_eq!(
        json(&saved),
        json(&bytes),
        "saved back with the same content"
    );
}

#[test]
fn the_golden_stores_the_documented_shape() {
    let bytes = read_or_write(GOLDEN, golden_bytes);
    let g = golden();
    let loro = LoroDoc::new();
    loro.import(&members(&bytes)["document.loro"]).unwrap();
    let meta = |id| loro.get_tree("paths").get_meta(tree_id(id)).unwrap();
    let value = |id, key: &str| meta(id).get(key).unwrap().get_deep_value();
    assert_eq!(value(g.linear, "fill_paint"), LoroValue::from("gradient"));
    assert_eq!(
        value(g.linear, "fill_gradient"),
        LoroValue::from("linear 0 0.5 1 0.5")
    );
    assert_eq!(
        value(g.ellipse, "fill_gradient"),
        LoroValue::from("radial 0.4 0.5 0.9 0.25 0.5")
    );
    assert_eq!(
        value(g.radial, "fill_gradient"),
        LoroValue::from("radial 0.5 0.5 1 0.5"),
        "a circle writes no ratio"
    );
    assert_eq!(
        value(g.nine, "fill_gradient_stops"),
        LoroValue::from(
            "#000000 0;#FF0000 0.125;#FF8000C8 0.25;#FFFF00 0.375;#00FF00 0.5;#00FFFF 0.5;\
             #0000FF80 0.75;#8000FF 0.875;#FFFFFF00 1"
        )
    );
}

/// The base of the damaged goldens: one rectangle with a Gradient fill.
fn damaged_base() -> (Vec<u8>, NodeId) {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    edit(&d, a, StyleEdit::FillPaint(FillPaint::Gradient));
    (pack(&d, "0.1.0").unwrap(), a)
}

fn damaged(reason: &str) -> Vec<u8> {
    let (bytes, a) = damaged_base();
    let set = |key, text: Option<&str>| with_register(&bytes, a, key, text.map(LoroValue::from));
    match reason {
        "colour" => set("fill_gradient_stops", Some("#00000 0;#FFFFFF 1")),
        "fraction" => set("fill_gradient", Some("linear abc 0.5 1 0.5")),
        "one_stop" => set("fill_gradient_stops", Some("#000000 0")),
        "unordered" => set(
            "fill_gradient_stops",
            Some("#000000 0;#FF0000 0.8;#00FF00 0.2;#FFFFFF 1"),
        ),
        "paint_without_gradient" => set("fill_gradient", None),
        other => panic!("{other}"),
    }
}

#[test]
fn each_damaged_golden_is_refused() {
    for reason in [
        "colour",
        "fraction",
        "one_stop",
        "unordered",
        "paint_without_gradient",
    ] {
        let name = format!("gradient_fill_damaged_{reason}_v49.curvyo");
        let bytes = read_or_write(&name, || damaged(reason));
        assert_eq!(format_of(&bytes), 49, "{name}");
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name} is refused"
        );
    }
    let (base, _) = damaged_base();
    assert!(unpack(2, &base).is_ok(), "the undamaged base opens");
}

/// This build refuses a file of the next format as saved by a newer version.
#[test]
fn a_newer_file_is_refused_by_version() {
    let bytes = read_or_write(GOLDEN, golden_bytes);
    let mut files = members(&bytes);
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["format_version"] = serde_json::json!(53);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    assert!(matches!(
        unpack(2, &rezip(&files)),
        Err(OpenError::FormatTooNew {
            found: 53,
            supported: 52
        })
    ));
}
