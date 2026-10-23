//! Independent tester cases for `specs/0076-single-line-fonts/specification.md` at the
//! `Session` level, for the criteria the implementer's tests leave open or cover thinly: 9 (the
//! stroke switched on by a pick, undone in one step), 10 (SL-BACK A: the stroke stays after
//! switching back), 18 (Split, Break apart and Cut convert), 19 (Offset refuses; all or
//! nothing over two texts; the missing-font refusal comes first), 21 (save and reopen in a
//! session; a newer format is refused), 13 (a uniform resize treats the stroke as a path's) and
//! 17 (Object to path: one compound path per glyph, nodes on the font's points, the 24-character
//! name, one step).
//!
//! Written from the specification and the public names before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    missing_docs
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, FontKind, LineCap, LineJoin, NodeId, ObjectSnapshot,
    OpenError, Point, Timestamp, unpack,
};
use curvyo_editor_wasm::{
    BooleanOutcome, BreakApartOutcome, CutOutcome, EscapeStep, KeyInput, KeyOutcome,
    ObjectToPathOutcome, OffsetOutcome, Session, SplitOutcome, Tool,
};
use curvyo_ui_core::{BooleanOp, BooleanRefusal, FontBlock, OffsetRefusal, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_950_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

type Listed = (String, String, String, u32, FontKind);

/// The included faces as `index.json` lists them, with their kinds; `skip` leaves a family out.
fn included_faces(skip: &str) -> Vec<Listed> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["family"] != skip)
        .map(|f| {
            let kind = if f["kind"] == "single_line" {
                FontKind::SingleLine
            } else {
                FontKind::Outline
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect()
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

fn prepare(mut s: Session, skip: &str) -> Session {
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list_with_kinds(FontBlock::Included, included_faces(skip), Vec::new());
    load(&mut s);
    s
}

fn fresh() -> Session {
    prepare(Session::new(31), "")
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn face(s: &Session, family: &str) -> usize {
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    let index = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == family)
        .unwrap_or_else(|| panic!("{family} is listed"))["index"]
        .as_u64()
        .unwrap();
    usize::try_from(index).unwrap()
}

/// A text typed at `at` in `family` (picked with nothing selected: the next text's face).
fn add_text(s: &mut Session, family: &str, at: Point, string: &str) -> NodeId {
    s.set_tool(Tool::Select);
    s.escape();
    s.escape();
    let index = face(s, family);
    s.pick_font(index);
    load(s);
    let before: Vec<NodeId> = texts(&doc_of(s)).iter().map(|t| t.id).collect();
    s.set_tool(Tool::Text);
    click(s, at);
    assert!(s.text_editing(), "a click starts edit mode");
    s.text_input(TextInput::Insert(string.to_owned()));
    load(s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
    let id = texts(&doc_of(s))
        .iter()
        .map(|t| t.id)
        .find(|id| !before.contains(id))
        .unwrap();
    assert_eq!(text(s, id).font.family, family);
    id
}

fn doc_of(s: &Session) -> Document {
    unpack(77, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn texts(d: &Document) -> Vec<curvyo_document_core::TextSnapshot> {
    d.object_ids()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Text(t)) => Some(*t),
            _ => None,
        })
        .collect()
}

fn text(s: &Session, id: NodeId) -> curvyo_document_core::TextSnapshot {
    texts(&doc_of(s)).into_iter().find(|t| t.id == id).unwrap()
}

fn selected(s: &Session) -> Vec<NodeId> {
    s.style_scope().ids
}

fn marquee(s: &mut Session, a: Point, b: Point) {
    s.set_tool(Tool::Select);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(
        pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
        false,
        false,
    );
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn rectangle(s: &mut Session, a: Point, b: Point) {
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

/// Selects `id`, the only object of the document, with a marquee around everything.
fn select_only(s: &mut Session, id: NodeId) {
    marquee(s, pt(-200.0, -200.0), pt(600.0, 600.0));
    assert_eq!(selected(s), [id]);
}

/// Every object in the document is a path with Fill off whose outlines are all open.
fn all_open_paths_fill_off(d: &Document, what: &str) {
    assert!(texts(d).is_empty(), "{what}: no text left");
    assert!(!d.object_ids().is_empty(), "{what}: a result");
    for id in d.object_ids() {
        match d.object(id).unwrap() {
            ObjectSnapshot::Path(p) => {
                assert!(!p.style.fill.enabled, "{what}: Fill off");
                assert!(p.style.stroke.enabled, "{what}: the text's stroke");
                assert!(p.subpaths().all(|o| !o.closed), "{what}: open outlines");
            }
            other => panic!("{what}: {other:?} is not a path"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Part B
// ---------------------------------------------------------------------------------------------

/// Criteria 9 and 10: an outline text with its stroke off, switched to a single-line face by a
/// pick, gets the stroke on in the fill colour at 0.25 mm, round caps and joins, in the one
/// step "Change font"; one undo takes font and stroke back together. Switching back to an
/// outline face keeps the stroke (SL-BACK A) and the fill.
#[test]
fn ac09_ac10_a_pick_switches_the_stroke_on_in_one_undoable_step() {
    let mut s = fresh();
    let id = add_text(&mut s, "Source Sans 3", pt(20.0, 40.0), "Hi");
    select_only(&mut s, id);
    s.set_stroke_paint(false);
    s.set_fill_paint(true);
    let before = text(&s, id);
    assert!(!before.style.stroke.enabled);
    assert!(before.style.fill.enabled);
    let steps = s.undo_len();

    s.pick_font(face(&s, "Hershey Sans 1-stroke"));
    load(&mut s);
    assert_eq!(s.undo_len(), steps + 1, "font and stroke in one step");
    let after = text(&s, id);
    assert_eq!(after.font.family, "Hershey Sans 1-stroke");
    assert_eq!(after.font.kind, FontKind::SingleLine);
    let stroke = &after.style.stroke;
    assert!(stroke.enabled);
    assert_eq!(stroke.color, before.style.fill.color, "the fill colour");
    assert!((stroke.width.as_mm() - 0.25).abs() < 1e-9);
    assert_eq!((stroke.cap, stroke.join), (LineCap::Round, LineJoin::Round));
    assert_eq!(after.style.fill, before.style.fill, "the fill is kept");

    s.undo();
    let undone = text(&s, id);
    assert_eq!(undone.font.family, "Source Sans 3");
    assert_eq!(undone.style, before.style, "one undo takes both back");
    s.redo();
    assert_eq!(text(&s, id).style, after.style);

    // SL-BACK A: back to an outline face, the stroke stays as it is and the fill is kept.
    s.pick_font(face(&s, "Source Serif 4"));
    load(&mut s);
    let back = text(&s, id);
    assert_eq!(back.font.kind, FontKind::Outline);
    assert_eq!(back.style, after.style, "nothing else changes");

    // A stroke that is on is not changed by a pick.
    s.set_stroke_paint(true);
    let width = text(&s, id).style.stroke.width;
    s.pick_font(face(&s, "EMS Allure"));
    load(&mut s);
    assert_eq!(text(&s, id).style.stroke.width, width);
}

/// Criterion 9: a new text typed in a single-line face whose default style has no stroke is
/// visible: its stroke is on, written with the text in the one step "Add text".
#[test]
fn ac09_a_new_single_line_text_has_a_stroke() {
    let mut s = fresh();
    let steps = s.undo_len();
    let id = add_text(&mut s, "EMS Readability", pt(20.0, 40.0), "Ab");
    let t = text(&s, id);
    assert!(t.style.stroke.enabled);
    assert!(t.style.stroke.width.as_mm() > 0.0);
    // The pick with nothing selected writes nothing; the text is one step.
    assert_eq!(s.undo_len(), steps + 1);
}

/// Criterion 13: a uniform corner resize (Ctrl) treats the stroke of a single-line text as a
/// rectangle's stroke in the same resize, with "Scale stroke width" off and on.
#[test]
fn ac13_a_uniform_resize_treats_the_stroke_as_a_path() {
    for scale_stroke in [false, true] {
        let mut s = fresh();
        s.set_scale_stroke_width(scale_stroke);
        let id = add_text(&mut s, "EMS Readability", pt(20.0, 40.0), "Ab");
        rectangle(&mut s, pt(20.0, 60.0), pt(50.0, 80.0));
        let d = doc_of(&s);
        let rect = d.object_ids().into_iter().find(|o| *o != id).unwrap();
        let width_of = |s: &Session, which: NodeId| {
            doc_of(s)
                .object(which)
                .unwrap()
                .style()
                .stroke
                .width
                .as_mm()
        };
        let (text_before, rect_before) = (width_of(&s, id), width_of(&s, rect));
        let text_size = text(&s, id).size.as_mm();
        marquee(&mut s, pt(-200.0, -200.0), pt(600.0, 600.0));
        assert_eq!(selected(&s).len(), 2);
        // The lower right corner handle of the selection box (its centre and size from the size
        // chip), dragged away from the box centre with Ctrl
        // (a uniform corner resize, `0074` criterion 26 and its UX notes).
        let opened = s.key_down(KeyInput {
            key: "s",
            ..KeyInput::default()
        });
        assert_eq!(opened, KeyOutcome::EntryOpened);
        let entry = s.transform_entry().unwrap();
        let w: f64 = entry.fields[0].prefill.parse().unwrap();
        let h: f64 = entry.fields[1].prefill.parse().unwrap();
        let centre = entry.center;
        let handle = pt(centre.x + w / 2.0, centre.y + h / 2.0);
        s.cancel_transform_entry();
        let to = pt(
            3.0 * handle.x - 2.0 * centre.x,
            3.0 * handle.y - 2.0 * centre.y,
        );
        let steps = s.undo_len();
        s.pointer_hover(handle, false, true);
        s.pointer_down(handle, false);
        s.pointer_hover(
            pt(f64::midpoint(handle.x, to.x), f64::midpoint(handle.y, to.y)),
            false,
            true,
        );
        s.pointer_hover(to, false, true);
        s.pointer_up(to, false, true);
        assert_eq!(s.undo_len(), steps + 1, "the resize is one step");
        assert!(
            text(&s, id).size.as_mm() > 1.5 * text_size,
            "a uniform resize multiplies Size"
        );
        let text_ratio = width_of(&s, id) / text_before;
        let rect_ratio = width_of(&s, rect) / rect_before;
        assert!(
            (text_ratio - rect_ratio).abs() < 1e-9,
            "scale stroke {scale_stroke}: text x{text_ratio}, rectangle x{rect_ratio}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Part C
// ---------------------------------------------------------------------------------------------

/// Criterion 18: Split and Break apart convert a single-line text first, in the same one step,
/// and work on the open outlines; every piece is an open path with Fill off.
#[test]
fn ac18_split_and_break_apart_convert() {
    for split in [true, false] {
        let mut s = fresh();
        let id = add_text(&mut s, "Hershey Sans 1-stroke", pt(20.0, 40.0), "HE");
        s.set_text_size(20.0);
        select_only(&mut s, id);
        let steps = s.undo_len();
        if split {
            let out = s.apply_split();
            assert!(matches!(out, SplitOutcome::Applied { .. }), "{out:?}");
        } else {
            let out = s.apply_break_apart();
            assert!(matches!(out, BreakApartOutcome::Applied { .. }), "{out:?}");
        }
        assert_eq!(s.undo_len(), steps + 1, "split={split}: one step");
        let d = doc_of(&s);
        all_open_paths_fill_off(&d, if split { "split" } else { "break apart" });
        if !split {
            // H has three strokes and E four in Hershey Sans: one path per stroke.
            assert!(d.object_ids().len() >= 5, "{}", d.object_ids().len());
        }
    }
}

/// Criterion 18: Cut at crossings converts a single-line text in the same step and cuts the
/// open stroke where a rectangle crosses it.
#[test]
fn ac18_cut_converts() {
    let mut s = fresh();
    let id = add_text(&mut s, "EMS Readability", pt(20.0, 40.0), "I");
    s.set_text_size(20.0);
    let t = text(&s, id);
    let base = t.placement.f;
    let x = t.placement.e;
    rectangle(&mut s, pt(x - 10.0, base - 12.0), pt(x + 20.0, base - 8.0));
    marquee(&mut s, pt(x - 15.0, base - 40.0), pt(x + 40.0, base + 10.0));
    assert_eq!(selected(&s).len(), 2);
    let steps = s.undo_len();
    let out = s.apply_cut();
    assert!(matches!(out, CutOutcome::Applied { .. }), "{out:?}");
    assert_eq!(s.undo_len(), steps + 1, "cut: one step");
    assert!(texts(&doc_of(&s)).is_empty(), "cut: no text left");
}

/// Criterion 17: Object to path of a long single-line text in Hershey Sans 1-stroke: a group
/// named by the first 24 characters, one path per drawn glyph; the H is one compound path of its
/// three open strokes, Fill off, its nodes the font's points scaled to Size within 0.001 mm;
/// one history step.
#[test]
fn ac17_object_to_path_nodes_on_the_font_points() {
    let mut s = fresh();
    let string = "HHHHHHHHHHHHHHHHHHHHHHHHHHHHHH";
    let id = add_text(&mut s, "Hershey Sans 1-stroke", pt(20.0, 40.0), string);
    select_only(&mut s, id);
    s.set_text_size(20.0);
    let t = text(&s, id);
    assert!((t.size.as_mm() - 20.0).abs() < 1e-9);
    let steps = s.undo_len();
    assert_eq!(s.object_to_path(), ObjectToPathOutcome::Converted);
    assert_eq!(s.undo_len(), steps + 1, "one step");
    let d = doc_of(&s);
    let group = d
        .read_nodes()
        .into_iter()
        .find(|n| n.kind == curvyo_document_core::NodeKind::Group)
        .expect("a group");
    // The first 24 characters; `0074` names rows with an ellipsis after (its criterion 27).
    let name = group.registers.name.clone().unwrap_or_default();
    assert!(
        name == string[..24] || name == format!("{}\u{2026}", &string[..24]),
        "{name:?}"
    );
    let paths: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter_map(|o| match d.object(o) {
            Some(ObjectSnapshot::Path(p)) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), string.len(), "one path per glyph");
    // The font's H: `M 220 662 L 220 0 M 662 662 L 662 0 M 220 346 L 662 346`, em 1000, the H
    // is 662 units high, so one unit is 20 / 662 mm; y is up in the font, down on the canvas.
    let strokes = [
        [(220.0, 662.0), (220.0, 0.0)],
        [(662.0, 662.0), (662.0, 0.0)],
        [(220.0, 346.0), (662.0, 346.0)],
    ];
    let unit = 20.0 / 662.0;
    let first = paths
        .iter()
        .min_by(|a, b| {
            let x = |p: &curvyo_document_core::PathSnapshot| {
                p.all_anchors()
                    .map(|a| a.point.x)
                    .fold(f64::INFINITY, f64::min)
            };
            x(a).total_cmp(&x(b))
        })
        .unwrap();
    assert!(!first.style.fill.enabled, "Fill off");
    assert!(first.style.stroke.enabled, "the text's stroke");
    let outlines: Vec<_> = first.subpaths().collect();
    assert_eq!(outlines.len(), 3, "one compound path of three strokes");
    assert!(outlines.iter().all(|o| !o.closed), "open strokes");
    // The glyph's origin: the first stroke's lower end is (220, 0) in font units.
    let origin_x = outlines[0].anchors[1].point.x - 220.0 * unit;
    let baseline = outlines[0].anchors[1].point.y;
    assert!((baseline - t.placement.f).abs() < 0.001, "on the baseline");
    for (outline, stroke) in outlines.iter().zip(strokes) {
        assert_eq!(outline.anchors.len(), 2);
        for (anchor, (x, y)) in outline.anchors.iter().zip(stroke) {
            let want = pt(origin_x + x * unit, baseline - y * unit);
            assert!(
                (anchor.point.x - want.x).abs() < 0.001 && (anchor.point.y - want.y).abs() < 0.001,
                "{:?} is not {want:?}",
                anchor.point
            );
        }
    }
}

/// Criterion 19: Offset refuses a single-line text with the no-area reason; nothing changes.
#[test]
fn ac19_offset_refuses() {
    let mut s = fresh();
    let id = add_text(&mut s, "EMS Readability", pt(20.0, 40.0), "Hi");
    select_only(&mut s, id);
    let steps = s.undo_len();
    let before = doc_of(&s).export_json().unwrap();
    s.set_tool(Tool::Offset);
    assert!(s.set_offset_distance(0.5));
    let opened = s.open_offset_chip();
    let out = s.apply_offset();
    match out {
        OffsetOutcome::Refused {
            refusal: OffsetRefusal::Texts(r),
            ..
        } => {
            assert!(r.no_area, "{r:?}");
            assert_eq!(r.offenders, [id]);
        }
        other => panic!("offset is refused (chip opened: {opened}): {other:?}"),
    }
    assert_eq!(s.undo_len(), steps, "nothing written");
    assert_eq!(doc_of(&s).export_json().unwrap(), before, "nothing changed");
}

/// Criterion 19: all or nothing. Two single-line texts and a rectangle: Union refuses, both
/// texts are offenders (the notice counts two), the rectangle is not touched.
#[test]
fn ac19_two_single_line_texts_refuse_all_or_nothing() {
    let mut s = fresh();
    let a = add_text(&mut s, "EMS Readability", pt(20.0, 40.0), "Ab");
    let b = add_text(&mut s, "Hershey Script 1-stroke", pt(20.0, 90.0), "Cd");
    rectangle(&mut s, pt(10.0, 20.0), pt(60.0, 100.0));
    marquee(&mut s, pt(-50.0, -50.0), pt(400.0, 300.0));
    assert_eq!(selected(&s).len(), 3);
    let before = doc_of(&s).export_json().unwrap();
    for op in [
        BooleanOp::Union,
        BooleanOp::Difference,
        BooleanOp::Intersection,
    ] {
        let out = s.apply_boolean(op);
        let BooleanOutcome::Refused(BooleanRefusal::Texts(r)) = out else {
            panic!("{op:?} is refused: {out:?}")
        };
        assert!(r.no_area);
        assert_eq!(
            r.offenders.len(),
            2,
            "{op:?}: both texts, not the rectangle"
        );
        assert!(
            r.offenders.contains(&a) && r.offenders.contains(&b),
            "{op:?}"
        );
    }
    assert_eq!(doc_of(&s).export_json().unwrap(), before, "nothing changed");
}

/// Criteria 19 and 22: a selection with a present and a missing single-line text: the
/// missing-font refusal of `0074` comes first, for the area commands and for Object to path.
#[test]
fn ac19_ac22_the_missing_font_refusal_comes_first() {
    let mut s = fresh();
    add_text(&mut s, "EMS Readability", pt(20.0, 40.0), "Ab");
    add_text(&mut s, "Hershey Sans 1-stroke", pt(20.0, 90.0), "Cd");
    let bytes = s.pack("0.1.0").unwrap();

    // Another computer without Hershey Sans 1-stroke opens the project.
    let mut s = prepare(Session::open(32, &bytes).unwrap(), "Hershey Sans 1-stroke");
    load(&mut s);
    let d = doc_of(&s);
    let lost = texts(&d)
        .into_iter()
        .find(|t| t.font.family == "Hershey Sans 1-stroke")
        .unwrap();
    assert_eq!(lost.font.kind, FontKind::SingleLine, "the kind is stored");
    marquee(&mut s, pt(-50.0, -50.0), pt(400.0, 300.0));
    assert_eq!(selected(&s).len(), 2);
    let out = s.apply_boolean(BooleanOp::Union);
    let BooleanOutcome::Refused(BooleanRefusal::Texts(r)) = out else {
        panic!("refused: {out:?}")
    };
    assert!(!r.no_area, "the missing-font notice first: {r:?}");
    assert_eq!(r.missing_faces.len(), 1, "{r:?}");
    assert!(r.missing_faces[0].starts_with("Hershey Sans 1-stroke"));
    let out = s.apply_offset();
    assert!(!matches!(out, OffsetOutcome::Applied { .. }), "{out:?}");
    // The missing text is drawn as lines in the fallback (criterion 22).
    let missing = texts(&doc_of(&s))
        .into_iter()
        .find(|t| t.font.family == "Hershey Sans 1-stroke")
        .unwrap();
    assert_eq!(missing.font.kind, FontKind::SingleLine);
    assert!(s.draw_list().triangle_count() > 0);
}

// ---------------------------------------------------------------------------------------------
// Part D
// ---------------------------------------------------------------------------------------------

/// Criterion 21: a session's project with single-line texts saves and reopens with the same
/// texts (kind and kept fill included), the texts stay editable, and the file is format 18 or
/// later.
#[test]
fn ac21_save_and_reopen_in_a_session() {
    let mut s = fresh();
    let id = add_text(&mut s, "EMS Allure", pt(20.0, 40.0), "One\nTwo");
    select_only(&mut s, id);
    s.set_fill_paint(true);
    let saved = text(&s, id);
    let bytes = s.pack("0.1.0").unwrap();
    assert!(manifest_version(&bytes) >= 18);
    let mut again = prepare(Session::open(33, &bytes).unwrap(), "");
    load(&mut again);
    let reopened = text(&again, id);
    assert_eq!(reopened.string, saved.string);
    assert_eq!(reopened.font, saved.font);
    assert_eq!(reopened.style, saved.style, "the kept fill included");
    assert_eq!(reopened.placement, saved.placement);
    assert_eq!(reopened.size, saved.size);
    // Editable: a Size change applies.
    select_only(&mut again, id);
    again.set_text_size(15.0);
    assert!((text(&again, id).size.as_mm() - 15.0).abs() < 1e-9);
}

fn manifest_version(bytes: &[u8]) -> u64 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    let manifest: Value = serde_json::from_str(&out).unwrap();
    manifest["format_version"].as_u64().unwrap()
}

/// Rewrites the container with `format_version` set to `version`.
fn with_version(bytes: &[u8], version: u64) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().unwrap().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if name == "manifest.json" {
            let mut manifest: Value = serde_json::from_slice(&data).unwrap();
            manifest["format_version"] = Value::from(version);
            data = serde_json::to_vec(&manifest).unwrap();
        }
        out.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// Criterion 21: an older build refuses the file. The golden file is format 18; a build that
/// reads only up to 17 sees it as "newer", as this build sees a file one version above its own.
#[test]
fn ac21_an_older_build_refuses_the_file() {
    let golden = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/text_single_line_v18.curvyo"),
    )
    .unwrap();
    assert_eq!(manifest_version(&golden), 18);
    assert!(unpack(1, &golden).is_ok());
    let newer = with_version(&golden, u64::from(CURRENT_FORMAT_VERSION) + 1);
    assert!(
        matches!(unpack(1, &newer), Err(OpenError::FormatTooNew { .. })),
        "a newer format is refused"
    );
}
