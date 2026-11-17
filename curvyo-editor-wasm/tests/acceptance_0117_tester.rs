//! Tester acceptance for `specs/0117-centre-lines` at the editor session: criteria 1 (enabled
//! state only), 2, 3, 4, 12, 13, 16, 17, 18 (the refusal) and 19 end to end. Written from the
//! specification and the public names only, before the implementation was read. The canvas
//! reference is `curvyo-render-core`'s own drawing, rasterised at 20 px per mm.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::cast_possible_wrap,
    clippy::type_complexity,
    missing_docs
)]

mod split_cut_common;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorKind, Color, Document, FontKind, FontRef, Length, LineCap, LineJoin, NewAnchor,
    NewImage, NodeId, ObjectSnapshot, PathSnapshot, StyleEdit, TextLayoutValues, Vec2,
    ViewTransform, unpack,
};
use curvyo_editor_wasm::{RowPress, Session, SvgExportResult};
use curvyo_geometry_core::CentreOptions;
use curvyo_render_core::build_artwork;
use curvyo_ui_core::{
    AnchorIdMinter, CENTRE_LINES_NODE_LIMIT, CentreLinesRefusal, SvgExportOptions,
    plan_stroke_to_path,
};
use serde_json::Value;
use split_cut_common::{aid, circle, compound, corner, json, pt, reread, session_of, v};

const RED: Color = Color {
    r: 200,
    g: 20,
    b: 30,
};

// ---------- building ----------

fn filled(d: &Document, ids: &[NodeId], color: Color) {
    for id in ids {
        d.edit_style(&[*id], &StyleEdit::FillEnabled(true)).unwrap();
        d.edit_style(&[*id], &StyleEdit::FillColor(color)).unwrap();
        d.edit_style(&[*id], &StyleEdit::StrokeEnabled(false))
            .unwrap();
    }
}

fn smooth(x: f64, y: f64, hin: (f64, f64), hout: (f64, f64)) -> NewAnchor {
    NewAnchor {
        id: aid(),
        point: pt(x, y),
        handle_in: v(hin.0, hin.1),
        handle_out: v(hout.0, hout.1),
        kind: AnchorKind::Symmetric,
    }
}

/// A stadium (a 2 mm line with round ends) from `(x, y)` to `(x + len, y)` of width `w`.
fn stadium(x: f64, y: f64, len: f64, w: f64) -> Vec<NewAnchor> {
    let r = w / 2.0;
    let k = 0.552_284_749_830_793_4 * r;
    vec![
        smooth(x, y - r, (-k, 0.0), (0.0, 0.0)),
        smooth(x + len, y - r, (0.0, 0.0), (k, 0.0)),
        smooth(x + len + r, y, (0.0, -k), (0.0, k)),
        smooth(x + len, y + r, (k, 0.0), (0.0, 0.0)),
        smooth(x, y + r, (0.0, 0.0), (-k, 0.0)),
        smooth(x - r, y, (0.0, k), (0.0, -k)),
    ]
}

/// A T outline: bar 40 x 2 mm, stem 20 x 2 mm below its middle.
fn tee(x: f64, y: f64) -> Vec<NewAnchor> {
    [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 2.0),
        (21.0, 2.0),
        (21.0, 22.0),
        (19.0, 22.0),
        (19.0, 2.0),
        (0.0, 2.0),
    ]
    .iter()
    .map(|&(a, b)| corner(x + a, y + b))
    .collect()
}

fn rid(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn select(s: &mut Session, ids: &[NodeId]) {
    for (i, &id) in ids.iter().enumerate() {
        let press = if i == 0 {
            RowPress::Plain
        } else {
            RowPress::Toggle
        };
        s.press_layer_row(&rid(id), press);
    }
}

fn selected(s: &mut Session) -> HashSet<NodeId> {
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 5000)).unwrap();
    v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["mark"] == "selected")
        .map(|r| serde_json::from_value(r["id"].clone()).unwrap())
        .collect()
}

fn apply(s: &mut Session) -> String {
    format!("{:?}", s.apply_centre_lines(CentreOptions::default()))
}

fn apply_with(s: &mut Session, options: CentreOptions) -> String {
    format!("{:?}", s.apply_centre_lines(options))
}

fn counts(outcome: &str) -> (usize, usize, usize, usize) {
    assert!(outcome.starts_with("Applied"), "not applied: {outcome}");
    let field = |name: &str| -> usize {
        let at = outcome.find(&format!("{name}: ")).unwrap() + name.len() + 2;
        outcome[at..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap()
    };
    (
        field("converted"),
        field("kept"),
        field("skipped"),
        field("unlinked"),
    )
}

fn new_ids(d: &Document, before: &HashSet<NodeId>) -> Vec<NodeId> {
    d.object_ids()
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect()
}

fn as_path(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Path(p)) => p,
        other => panic!("{id:?} is not a path: {other:?}"),
    }
}

fn name_of(d: &Document, id: NodeId) -> Option<String> {
    d.read_nodes()
        .into_iter()
        .find(|n| n.id == id)
        .and_then(|n| n.registers.name)
}

fn ids(d: &Document) -> HashSet<NodeId> {
    d.object_ids().into_iter().collect()
}

fn text(d: &Document) -> NodeId {
    let missing = FontRef {
        family: "Nowhere Sans".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "d".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    d.create_text(
        "Hi",
        &missing,
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(80.0, 80.0)),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn image(d: &Document) -> NodeId {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgba_8.png"),
    )
    .unwrap();
    d.add_images(&[NewImage {
        bytes: Arc::from(bytes.as_slice()),
        placement: Affine::translation(Vec2::new(120.0, 10.0)),
        name: "a.png".to_owned(),
    }])
    .unwrap()[0]
}

// ---------- canvas raster ----------

const SCALE: f64 = 20.0;

fn raster(triangles: &[curvyo_render_core::Vertex]) -> HashSet<(i64, i64)> {
    let mut covered = HashSet::new();
    for tri in triangles.as_chunks::<3>().0 {
        let p: Vec<(f64, f64)> = tri
            .iter()
            .map(|v| (v.position.x * SCALE, v.position.y * SCALE))
            .collect();
        let edge = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
        };
        let area = edge(p[0], p[1], p[2]);
        if area == 0.0 {
            continue;
        }
        let xs = p.iter().map(|q| q.0);
        let ys = p.iter().map(|q| q.1);
        let (x0, x1) = (
            xs.clone().fold(f64::MAX, f64::min).floor() as i64,
            xs.fold(f64::MIN, f64::max).ceil() as i64,
        );
        let (y0, y1) = (
            ys.clone().fold(f64::MAX, f64::min).floor() as i64,
            ys.fold(f64::MIN, f64::max).ceil() as i64,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let c = (x as f64 + 0.5, y as f64 + 0.5);
                let w = [
                    edge(p[0], p[1], c) * area.signum(),
                    edge(p[1], p[2], c) * area.signum(),
                    edge(p[2], p[0], c) * area.signum(),
                ];
                if w.iter().all(|&v| v >= 0.0) {
                    covered.insert((x, y));
                }
            }
        }
    }
    covered
}

fn artwork(d: &Document) -> HashSet<(i64, i64)> {
    let objects: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .collect();
    raster(&build_artwork(&objects, ViewTransform::new(SCALE, pt(-300.0, -300.0))).triangles)
}

// ---------- criterion 1 ----------

#[test]
fn c1_enabled_only_with_a_subject() {
    let d = Document::new(1);
    let line = d.create_path(&stadium(0.0, 0.0, 30.0, 2.0), true);
    filled(&d, &[line], RED);
    let open = d.create_path(
        &[corner(0.0, 20.0), corner(30.0, 20.0), corner(30.0, 30.0)],
        false,
    );
    filled(&d, &[open], RED);
    let stroke_only = d.create_path(&stadium(0.0, 40.0, 30.0, 2.0), true);
    d.edit_style(&[stroke_only], &StyleEdit::FillEnabled(false))
        .unwrap();
    d.edit_style(&[stroke_only], &StyleEdit::StrokeEnabled(true))
        .unwrap();
    let t = text(&d);
    let img = image(&d);
    let inner = d.create_path(&stadium(0.0, 60.0, 30.0, 2.0), true);
    filled(&d, &[inner], RED);
    let other = d.create_path(&[corner(50.0, 60.0), corner(60.0, 60.0)], false);
    let group = d.group_objects(&[inner, other]).unwrap();
    let mut s = session_of(&d);
    for (what, sel, want) in [
        ("filled stadium", vec![line], true),
        ("open path", vec![open], false),
        ("stroke only", vec![stroke_only], false),
        ("text", vec![t], false),
        ("image", vec![img], false),
        ("group with a filled shape", vec![group], true),
        ("mixed", vec![open, line], true),
    ] {
        select(&mut s, &sel);
        assert_eq!(s.centre_lines_available(), want, "{what}");
    }
}

// ---------- criterion 2 ----------

#[test]
fn c2_one_undo_step_restores_document_and_selection() {
    let d = Document::new(1);
    let below = d.create_path(&stadium(0.0, -20.0, 10.0, 2.0), true);
    let subject = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    let above = d.create_path(&stadium(0.0, 20.0, 10.0, 2.0), true);
    filled(&d, &[below, subject, above], RED);
    let mut s = session_of(&d);
    select(&mut s, &[subject]);
    let before_json = json(&s);
    let before_ids = ids(&reread(&s));
    let history = s.history_len();
    let outcome = apply(&mut s);
    assert_eq!(counts(&outcome), (1, 0, 0, 0), "{outcome}");
    assert_eq!(s.history_len(), history + 1, "exactly one history entry");
    let after = reread(&s);
    let after_json = json(&s);
    assert!(after.object(subject).is_none(), "the subject is replaced");
    let made = new_ids(&after, &before_ids);
    assert_eq!(made.len(), 1, "one centre path");
    // In the subject's place in the stacking order: between below and above.
    let order = after.children(after.parent_node(made[0]));
    let pos = |id| order.iter().position(|&x| x == id).unwrap();
    let (b, m, a) = (pos(below), pos(made[0]), pos(above));
    assert!(
        (b < m && m < a) || (a < m && m < b),
        "stacking place: {b} {m} {a}"
    );
    assert_eq!(
        selected(&mut s),
        HashSet::from([made[0]]),
        "the result is selected"
    );
    s.undo();
    assert_eq!(json(&s), before_json, "undo restores the exact document");
    assert_eq!(
        selected(&mut s),
        HashSet::from([subject]),
        "undo restores the selection"
    );
    s.redo();
    assert_eq!(json(&s), after_json, "redo restores the result");
    assert_eq!(
        selected(&mut s),
        HashSet::from([made[0]]),
        "redo selects the result"
    );
}

// ---------- criterion 3 ----------

#[test]
fn c3_single_result_is_an_ordinary_profiled_stroke() {
    let d = Document::new(1);
    let subject = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    filled(&d, &[subject], RED);
    let mut s = session_of(&d);
    let before = ids(&reread(&s));
    select(&mut s, &[subject]);
    apply(&mut s);
    let after = reread(&s);
    let made = new_ids(&after, &before);
    assert_eq!(made.len(), 1);
    assert!(!after.is_group(made[0]), "a single result is not grouped");
    let p = as_path(&after, made[0]);
    assert!(!p.closed);
    assert!(!p.style.fill.enabled, "Fill off");
    assert!(p.style.stroke.enabled, "Stroke on");
    assert_eq!(
        p.style.stroke.color, RED,
        "stroke colour = subject fill colour"
    );
    assert_eq!(p.style.stroke.cap, LineCap::Round);
    assert_eq!(p.style.stroke.join, LineJoin::Round);
    let w = p.style.stroke.width.as_mm();
    assert!(
        (w - 2.0).abs() <= 0.06,
        "base width {w} = largest local width"
    );
    for wp in &p.width_points {
        assert!(
            wp.multiplier <= 1.0 + 1e-9,
            "multipliers of the largest width"
        );
    }
}

#[test]
fn c3_several_results_are_grouped_and_named() {
    for name in [Some("Tee"), None] {
        let d = Document::new(1);
        let subject = d.create_path(&tee(0.0, 0.0), true);
        filled(&d, &[subject], RED);
        if let Some(n) = name {
            d.rename_node(subject, n).unwrap();
        }
        let mut s = session_of(&d);
        select(&mut s, &[subject]);
        let outcome = apply(&mut s);
        assert_eq!(counts(&outcome).0, 1, "{outcome}");
        let after = reread(&s);
        let parent = after
            .object_ids()
            .into_iter()
            .find_map(|id| after.parent_group(id))
            .expect("the results are in a group");
        let want = name.map_or("Centre lines".to_owned(), |n| {
            format!("Centre lines of {n}")
        });
        assert_eq!(name_of(&after, parent).as_deref(), Some(want.as_str()));
        let kids = after.leaves_of(parent);
        assert_eq!(kids.len(), 2, "bar and stem");
        let mut caps: Vec<LineCap> = kids
            .iter()
            .map(|&k| as_path(&after, k).style.stroke.cap)
            .collect();
        caps.sort_by_key(|c| format!("{c:?}"));
        assert_eq!(
            caps,
            vec![LineCap::Butt, LineCap::Round],
            "branch Butt, main Round"
        );
        assert_eq!(
            selected(&mut s),
            HashSet::from([parent]),
            "the group is selected"
        );
    }
}

#[test]
fn c3_ring_is_closed_with_round_caps() {
    let d = Document::new(1);
    let ring = compound(
        &d,
        vec![circle(0.0, 0.0, 11.0, false), circle(0.0, 0.0, 9.0, true)],
    );
    filled(&d, &[ring], RED);
    let mut s = session_of(&d);
    let before = ids(&reread(&s));
    select(&mut s, &[ring]);
    apply(&mut s);
    let after = reread(&s);
    let made = new_ids(&after, &before);
    assert_eq!(made.len(), 1);
    let p = as_path(&after, made[0]);
    assert!(p.closed);
    assert_eq!(p.style.stroke.cap, LineCap::Round);
}

// ---------- criterion 4 ----------

fn refused(s: &mut Session, sel: &[NodeId], want: &CentreLinesRefusal) {
    select(s, sel);
    let before = json(s);
    let history = s.history_len();
    let outcome = apply(s);
    assert_eq!(outcome, format!("Refused({want:?})"));
    assert_eq!(s.history_len(), history, "no history entry");
    assert_eq!(json(s), before, "nothing changes");
    assert_eq!(s.centre_lines_refusal().as_ref(), Some(want));
}

#[test]
fn c4_refusals_name_the_reason() {
    let d = Document::new(1);
    let open = d.create_path(
        &[corner(0.0, 20.0), corner(30.0, 20.0), corner(30.0, 30.0)],
        false,
    );
    filled(&d, &[open], RED);
    let stroke_only = d.create_path(&stadium(0.0, 40.0, 30.0, 2.0), true);
    d.edit_style(&[stroke_only], &StyleEdit::FillEnabled(false))
        .unwrap();
    let t = text(&d);
    let img = image(&d);
    let a = d.create_path(&[corner(0.0, 60.0), corner(10.0, 60.0)], false);
    let b = d.create_path(&[corner(0.0, 70.0), corner(10.0, 70.0)], false);
    let group = d.group_objects(&[a, b]).unwrap();
    let mut s = session_of(&d);
    refused(&mut s, &[open], &CentreLinesRefusal::NotFilled);
    refused(&mut s, &[stroke_only], &CentreLinesRefusal::NotFilled);
    refused(&mut s, &[t], &CentreLinesRefusal::Text);
    refused(&mut s, &[img], &CentreLinesRefusal::Image);
    refused(&mut s, &[group], &CentreLinesRefusal::NotFilled);
}

#[test]
fn c4_mixed_selection_converts_subjects_and_counts_the_rest() {
    let d = Document::new(1);
    let line = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    let disc = d.create_path(&circle(100.0, 0.0, 5.0, false), true);
    filled(&d, &[line, disc], RED);
    let open = d.create_path(
        &[corner(0.0, 20.0), corner(30.0, 20.0), corner(30.0, 30.0)],
        false,
    );
    let t = text(&d);
    let mut s = session_of(&d);
    let open_before = reread(&s).object(open);
    let text_before = format!("{:?}", reread(&s).object(t));
    select(&mut s, &[line, disc, open, t]);
    let outcome = apply(&mut s);
    assert_eq!(counts(&outcome), (1, 1, 2, 0), "{outcome}");
    let after = reread(&s);
    assert!(after.object(line).is_none());
    assert_eq!(
        after.object(open),
        open_before,
        "the open path is left alone"
    );
    assert_eq!(
        format!("{:?}", after.object(t)),
        text_before,
        "the text is left alone"
    );
    assert!(after.object(disc).is_some(), "the disc is kept filled");
}

// ---------- criterion 12 ----------

#[test]
fn c12_disc_stays_filled_as_it_was() {
    let d = Document::new(1);
    let disc = d.create_path(&circle(0.0, 0.0, 5.0, false), true);
    filled(&d, &[disc], RED);
    let mut s = session_of(&d);
    let shape_before = format!("{:?}", reread(&s).object(disc));
    select(&mut s, &[disc]);
    let outcome = apply_with(
        &mut s,
        CentreOptions {
            thick_parts: None,
            ..CentreOptions::default()
        },
    );
    assert_eq!(counts(&outcome), (0, 1, 0, 0), "{outcome}");
    assert_eq!(format!("{:?}", reread(&s).object(disc)), shape_before);
}

// ---------- criterion 13 ----------

#[test]
fn c13_inner_area_is_a_grey_filled_shape_without_stroke() {
    let d = Document::new(1);
    let ring = compound(
        &d,
        vec![circle(0.0, 0.0, 11.0, false), circle(0.0, 0.0, 9.0, true)],
    );
    filled(&d, &[ring], RED);
    let mut s = session_of(&d);
    let before = ids(&reread(&s));
    select(&mut s, &[ring]);
    apply_with(
        &mut s,
        CentreOptions {
            inner_offset: Some(Length::from_mm(0.3)),
            ..CentreOptions::default()
        },
    );
    let after = reread(&s);
    let made = new_ids(&after, &before);
    let inner: Vec<PathSnapshot> = made
        .iter()
        .map(|&id| as_path(&after, id))
        .filter(|p| p.style.fill.enabled)
        .collect();
    assert_eq!(inner.len(), 1, "one inner area");
    let p = &inner[0];
    assert!(!p.style.stroke.enabled, "Stroke off");
    let c = p.style.fill.color;
    assert!(
        c.r == c.g && c.g == c.b && c.r >= 180,
        "a neutral light grey: {c:?}"
    );
    let (lo, hi) = p
        .all_anchors()
        .map(|a| a.point.x.hypot(a.point.y))
        .fold((f64::INFINITY, 0.0f64), |(l, h), r| (l.min(r), h.max(r)));
    assert!(
        (lo - 8.7).abs() <= 0.01 && (hi - 8.7).abs() <= 0.01,
        "{lo}..{hi}"
    );
    assert!(made.len() >= 2, "the ring's centre line too");
}

// ---------- criterion 16 ----------

#[test]
fn c16_clones_of_a_converted_original_become_ordinary_paths() {
    let d = Document::new(1);
    let subject = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    filled(&d, &[subject], RED);
    let clone = d.linked_clone(&[subject], &[]).unwrap()[0];
    let mut s = session_of(&d);
    assert!(matches!(
        reread(&s).object(clone),
        Some(ObjectSnapshot::Clone(_))
    ));
    select(&mut s, &[subject]);
    let outcome = apply(&mut s);
    assert_eq!(counts(&outcome), (1, 0, 0, 1), "{outcome}");
    assert!(
        matches!(reread(&s).object(clone), Some(ObjectSnapshot::Path(_))),
        "the clone became an ordinary path"
    );
    s.undo();
    assert!(matches!(
        reread(&s).object(clone),
        Some(ObjectSnapshot::Clone(_))
    ));
}

#[test]
fn c16_a_selected_clone_is_converted_as_the_path_it_shows() {
    let d = Document::new(1);
    let subject = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    filled(&d, &[subject], RED);
    let clone = d.linked_clone(&[subject], &[]).unwrap()[0];
    let mut s = session_of(&d);
    let before = ids(&reread(&s));
    select(&mut s, &[clone]);
    assert!(
        s.centre_lines_available(),
        "a clone of a filled shape is a subject"
    );
    let outcome = apply(&mut s);
    assert_eq!(counts(&outcome).0, 1, "{outcome}");
    let after = reread(&s);
    assert!(after.object(clone).is_none(), "the clone is replaced");
    assert!(after.object(subject).is_some(), "the original stays");
    let made = new_ids(&after, &before);
    assert_eq!(made.len(), 1);
    assert_eq!(as_path(&after, made[0]).style.stroke.color, RED);
}

// ---------- criterion 17 ----------

#[test]
fn c17_result_reopens_and_exports() {
    let d = Document::new(1);
    let subject = d.create_path(&tee(0.0, 0.0), true);
    let line = d.create_path(&stadium(0.0, 40.0, 40.0, 2.0), true);
    filled(&d, &[subject, line], RED);
    let mut s = session_of(&d);
    let canvas_subject = artwork(&reread(&s));
    select(&mut s, &[subject, line]);
    apply(&mut s);
    let packed = s.pack("0.1.0").unwrap();
    let first = json(&s);
    let reopened = Session::open(3, &packed).unwrap();
    assert_eq!(
        json(&reopened),
        first,
        "the result opens unchanged after saving"
    );
    let again = unpack(7, &reopened.pack("0.1.0").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), first);
    // The painted result is close to the painted subjects on the canvas (criterion 5 by pixels);
    // the results are painted as the canvas draws them, with their width profiles.
    let doc = reread(&s);
    let canvas_result = canvas(&doc);
    let differ = canvas_subject.symmetric_difference(&canvas_result).count();
    let share = differ as f64 / canvas_subject.len() as f64;
    println!(
        "c17 canvas subject vs result: {:.3} % of pixels differ",
        share * 100.0
    );
    assert!(
        share <= 0.05,
        "the result paints the subject: {:.2} % differ",
        share * 100.0
    );
    // SVG export draws every result.
    match s.export_svg(SvgExportOptions::default()) {
        SvgExportResult::Written { text, objects, .. } => {
            assert_eq!(objects, 3, "bar, stem and line exported");
            assert!(text.contains("<path"));
        }
        other => panic!("not written: {other:?}"),
    }
}

/// Criterion 17, `0085` criterion 11 test: every result painted as an outline (what SVG export
/// writes for a variable width) matches the canvas within 0.5 % of pixels at 20 px per mm.
#[test]
fn c17_outline_of_each_result_matches_the_canvas() {
    let d = Document::new(1);
    let subject = d.create_path(&tee(0.0, 0.0), true);
    let line = d.create_path(&stadium(0.0, 40.0, 40.0, 2.0), true);
    filled(&d, &[subject, line], RED);
    let mut s = session_of(&d);
    select(&mut s, &[subject, line]);
    apply(&mut s);
    let packed = s.pack("0.1.0").unwrap();
    let doc = reread(&s);
    // Every result painted as an outline (the SVG of a variable width) matches the canvas
    // within 0.5 % of pixels (the `0085` criterion 11 test).
    let paint = |objects: Vec<ObjectSnapshot>| {
        raster(&build_artwork(&objects, ViewTransform::new(SCALE, pt(-300.0, -300.0))).triangles)
    };
    let mut bad = Vec::new();
    for id in doc.object_ids() {
        let d2 = unpack(8, &packed).unwrap();
        let old = ids(&d2);
        let p = as_path(&d2, id);
        // The canvas draws a profiled path as its variable-width artwork at 0.01 mm
        // (`session/width_outlines.rs`); `build_artwork` alone strokes the base width and knows
        // no profile, so it is not the canvas for these results.
        let before = if p.width_points.is_empty() {
            paint(d2.object(id).into_iter().collect())
        } else {
            let (fill, outline) = curvyo_ui_core::verified_variable_stroke_artwork_with_tolerance(
                &p,
                curvyo_document_core::Tolerance::from_mm(0.01),
            )
            .unwrap();
            paint(
                std::iter::once(fill)
                    .chain(outline)
                    .map(ObjectSnapshot::Path)
                    .collect(),
            )
        };
        let objects: Vec<_> = d2.object(id).into_iter().collect();
        let plan = plan_stroke_to_path(
            &objects,
            &[id],
            &|_| false,
            &|_| None,
            &mut AnchorIdMinter::new(8),
        )
        .unwrap();
        d2.stroke_to_path(&plan.write).unwrap();
        let made: Vec<ObjectSnapshot> = new_ids(&d2, &old)
            .into_iter()
            .chain(std::iter::once(id))
            .filter_map(|n| d2.object(n))
            .collect();
        let after = paint(made);
        let differing: Vec<_> = before.symmetric_difference(&after).copied().collect();
        let off_edge = differing
            .iter()
            .filter(|&&(x, y)| {
                let inside = before.contains(&(x, y));
                !(-1..=1).any(|dy| (-1..=1).any(|dx| before.contains(&(x + dx, y + dy)) != inside))
            })
            .count();
        let share = differing.len() as f64 / before.len().max(1) as f64;
        println!(
            "c17 {id:?}: cap {:?}, {} width points, {} of {} px differ ({:.3} %), {off_edge} off the edge",
            p.style.stroke.cap,
            p.width_points.len(),
            differing.len(),
            before.len(),
            share * 100.0
        );
        if share > 0.005 || off_edge > 0 {
            bad.push(format!(
                "{:?} cap: {:.3} %, {off_edge} px off the edge",
                p.style.stroke.cap,
                share * 100.0
            ));
        }
    }
    assert!(bad.is_empty(), "outline vs canvas: {bad:?}");
}

// ---------- criterion 18: the node limit ----------

fn polygon(d: &Document, n: usize) -> NodeId {
    let anchors: Vec<NewAnchor> = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            let r = if i % 2 == 0 { 100.0 } else { 98.0 };
            corner(r * a.cos(), r * a.sin())
        })
        .collect();
    let id = d.create_path(&anchors, true);
    filled(d, &[id], RED);
    id
}

#[test]
fn c18_above_the_node_limit_is_refused_with_the_count() {
    // The worker runs the kernel (customer 2026-11-18, reversing question 219 A): the interim
    // 20,000-node limit is gone, the 100,000 refusal stays.
    assert_eq!(CENTRE_LINES_NODE_LIMIT, 100_000);
    let d = Document::new(1);
    let big = polygon(&d, 100_001);
    let mut s = session_of(&d);
    select(&mut s, &[big]);
    assert_eq!(
        s.centre_lines_refusal(),
        Some(CentreLinesRefusal::TooLarge { nodes: 100_001 })
    );
    let history = s.history_len();
    assert_eq!(
        apply(&mut s),
        format!(
            "Refused({:?})",
            CentreLinesRefusal::TooLarge { nodes: 100_001 }
        )
    );
    assert_eq!(s.history_len(), history);
    // Two shapes that together pass the limit are refused too.
    let d = Document::new(1);
    let a = polygon(&d, 60_000);
    let b = polygon(&d, 40_001);
    let mut s = session_of(&d);
    select(&mut s, &[a, b]);
    assert_eq!(
        s.centre_lines_refusal(),
        Some(CentreLinesRefusal::TooLarge { nodes: 100_001 })
    );
    // Exactly the limit is not refused.
    let d = Document::new(1);
    let ok = polygon(&d, 100_000);
    let mut s = session_of(&d);
    select(&mut s, &[ok]);
    assert_eq!(s.centre_lines_refusal(), None);
}

// ---------- criterion 19: the whale end to end ----------

fn whale(d: &Document) -> Vec<NodeId> {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-geometry-core/tests/fixtures/centre_lines/whale_traced.txt"),
    )
    .unwrap();
    let mut subjects: Vec<Vec<Vec<NewAnchor>>> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "subject" {
            subjects.push(Vec::new());
        } else if line.starts_with("outline") {
            assert!(line.contains("true"), "closed outlines");
            subjects.last_mut().unwrap().push(Vec::new());
        } else {
            let n: Vec<f64> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            subjects
                .last_mut()
                .unwrap()
                .last_mut()
                .unwrap()
                .push(NewAnchor {
                    id: aid(),
                    point: pt(n[0], n[1]),
                    handle_in: v(n[2], n[3]),
                    handle_out: v(n[4], n[5]),
                    kind: AnchorKind::Corner,
                });
        }
    }
    subjects
        .into_iter()
        .map(|outlines| {
            let id = if outlines.len() == 1 {
                d.create_path(&outlines[0], true)
            } else {
                compound(d, outlines)
            };
            filled(d, &[id], Color { r: 0, g: 0, b: 0 });
            id
        })
        .collect()
}

#[test]
fn c19_whale_converts_in_one_step_and_is_deterministic() {
    let run = || {
        let d = Document::new(1);
        let subjects = whale(&d);
        let mut s = session_of(&d);
        select(&mut s, &subjects);
        let before = json(&s);
        let history = s.history_len();
        let outcome = apply(&mut s);
        let (converted, kept, skipped, _) = counts(&outcome);
        assert_eq!(converted + kept, subjects.len(), "{outcome}");
        assert_eq!(skipped, 0);
        // `kept` counts subjects; the whale is one compound subject whose pupil region stays
        // filled (reported to the lead: criterion 12 says such a blob is counted).
        let _ = kept;
        assert_eq!(s.history_len(), history + 1);
        let doc = reread(&s);
        let filled_parts = doc
            .object_ids()
            .into_iter()
            .filter_map(|id| doc.object(id))
            .filter(|o| matches!(o, ObjectSnapshot::Path(p) if p.style.fill.enabled))
            .count();
        assert!(
            filled_parts >= 1,
            "the pupil stays a filled shape among the results"
        );
        let shapes: Vec<String> = doc
            .object_ids()
            .into_iter()
            .filter_map(|id| doc.object(id))
            .map(|o| match o {
                ObjectSnapshot::Path(p) => format!(
                    "{:?}",
                    (
                        p.closed,
                        p.style.stroke.cap,
                        (p.style.stroke.width.as_mm() * 1e4).round(),
                        p.all_anchors()
                            .map(|a| ((a.point.x * 1e4).round(), (a.point.y * 1e4).round()))
                            .collect::<Vec<_>>(),
                        p.width_points
                            .iter()
                            .map(|w| ((w.position * 1e6).round(), (w.multiplier * 1e6).round()))
                            .collect::<Vec<_>>()
                    )
                ),
                other => format!("{:?}", std::mem::discriminant(&other)),
            })
            .collect();
        s.undo();
        assert_eq!(json(&s), before, "undo restores the whale");
        (outcome, shapes)
    };
    let (o1, a) = run();
    let (o2, b) = run();
    assert_eq!(o1, o2);
    assert_eq!(a, b, "the same whale gives the same centre lines");
    println!("c19 whale: {o1}, {} objects after", a.len());
}

// ---------- re-verification (2026-11-16) ----------

/// The objects as the canvas draws them: a profiled path as its variable-width artwork at the
/// canvas tolerance of 0.01 mm (`session/width_outlines.rs`, `push_width_expanded`), anything
/// else as it is.
fn canvas_objects(d: &Document) -> Vec<ObjectSnapshot> {
    let mut out = Vec::new();
    for id in d.object_ids() {
        let Some(object) = d.object(id) else { continue };
        match &object {
            ObjectSnapshot::Path(p) if !p.width_points.is_empty() => {
                let (fill, outline) =
                    curvyo_ui_core::verified_variable_stroke_artwork_with_tolerance(
                        p,
                        curvyo_document_core::Tolerance::from_mm(0.01),
                    )
                    .expect("the canvas has an outline for every result");
                out.push(ObjectSnapshot::Path(fill));
                out.extend(outline.map(ObjectSnapshot::Path));
            }
            _ => out.push(object),
        }
    }
    out
}

fn canvas(d: &Document) -> HashSet<(i64, i64)> {
    raster(
        &build_artwork(
            &canvas_objects(d),
            ViewTransform::new(SCALE, pt(-300.0, -300.0)),
        )
        .triangles,
    )
}

/// Pixels of `b` not within `reach` pixels of the same state in `a`'s picture: the differences
/// that are not along an edge.
fn off_edge(a: &HashSet<(i64, i64)>, b: &HashSet<(i64, i64)>, reach: i64) -> Vec<(i64, i64)> {
    a.symmetric_difference(b)
        .filter(|&&(x, y)| {
            let inside = a.contains(&(x, y));
            !(-reach..=reach)
                .any(|dy| (-reach..=reach).any(|dx| a.contains(&(x + dx, y + dy)) != inside))
        })
        .copied()
        .collect()
}

/// Criteria 5 and 17 by the canvas (stricter than the outline-vs-outline export check): what
/// the canvas paints for the results of a T, a stadium and a ring differs from the painted
/// subject by at most 2 % of the subject's area, and nothing lies farther than 0.1 mm (2 px)
/// from the subject's edge, except in the square corners of the T's flat ends, which are drawn
/// round ("a flat end counts as rounded", criterion 8 Proposal).
#[test]
fn c5_c17_canvas_of_the_results_paints_the_subject() {
    type Make = Box<dyn Fn(&Document) -> NodeId>;
    type Allowed = Box<dyn Fn(f64, f64) -> bool>;
    let mut bad = Vec::new();
    let shapes: Vec<(&str, Make, Allowed)> = vec![
        (
            "T",
            Box::new(|d: &Document| d.create_path(&tee(0.0, 0.0), true)),
            // Within w/2 = 1 mm of the bar's two flat ends and the stem's flat end.
            Box::new(|x, y| x < 1.0 || x > 39.0 || y > 21.0),
        ),
        (
            "stadium",
            Box::new(|d: &Document| d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true)),
            Box::new(|_, _| false),
        ),
        (
            "ring",
            Box::new(|d: &Document| {
                compound(
                    d,
                    vec![circle(0.0, 0.0, 11.0, false), circle(0.0, 0.0, 9.0, true)],
                )
            }),
            Box::new(|_, _| false),
        ),
    ];
    for (name, make, allowed) in &shapes {
        let d = Document::new(1);
        let subject = make(&d);
        filled(&d, &[subject], RED);
        let mut s = session_of(&d);
        let before = artwork(&reread(&s));
        select(&mut s, &[subject]);
        let outcome = apply(&mut s);
        assert_eq!(counts(&outcome).0, 1, "{name}: {outcome}");
        let after = canvas(&reread(&s));
        let differ = before.symmetric_difference(&after).count();
        let share = differ as f64 / before.len() as f64;
        let far = off_edge(&before, &after, 2);
        let far_elsewhere: Vec<(f64, f64)> = far
            .iter()
            .map(|&(x, y)| (x as f64 / SCALE, y as f64 / SCALE))
            .filter(|&(x, y)| !allowed(x, y))
            .collect();
        println!(
            "{name}: canvas of the results vs subject {:.3} % of {} px, {} px farther than 0.1 mm ({} outside the flat-end corners)",
            share * 100.0,
            before.len(),
            far.len(),
            far_elsewhere.len()
        );
        if share > 0.02 || !far_elsewhere.is_empty() {
            bad.push(format!(
                "{name}: {:.3} %, far at {:?}",
                share * 100.0,
                &far_elsewhere[..far_elsewhere.len().min(5)]
            ));
        }
    }
    assert!(bad.is_empty(), "canvas vs subject: {bad:?}");
}

/// Criterion 13 with a negative offset at the session: the inner area is the hole grown outward
/// (radius 9 + 0.3 = 9.3 within 0.01 mm), grey, Stroke off, and drawn under the ring's stroke.
#[test]
fn c13_negative_offset_inner_area_lies_under_the_stroke() {
    let d = Document::new(1);
    let ring = compound(
        &d,
        vec![circle(0.0, 0.0, 11.0, false), circle(0.0, 0.0, 9.0, true)],
    );
    filled(&d, &[ring], RED);
    let mut s = session_of(&d);
    let before = ids(&reread(&s));
    select(&mut s, &[ring]);
    let outcome = apply_with(
        &mut s,
        CentreOptions {
            inner_offset: Some(Length::from_mm(-0.3)),
            ..CentreOptions::default()
        },
    );
    assert_eq!(counts(&outcome).0, 1, "{outcome}");
    let after = reread(&s);
    let order = after.object_ids();
    let made: Vec<NodeId> = order
        .iter()
        .copied()
        .filter(|id| !before.contains(id))
        .collect();
    let inner: Vec<NodeId> = made
        .iter()
        .copied()
        .filter(|&id| as_path(&after, id).style.fill.enabled)
        .collect();
    assert_eq!(inner.len(), 1, "one inner area: {made:?}");
    let p = as_path(&after, inner[0]);
    assert!(!p.style.stroke.enabled);
    let (lo, hi) = p
        .all_anchors()
        .map(|a| a.point.x.hypot(a.point.y))
        .fold((f64::INFINITY, 0.0f64), |(l, h), r| (l.min(r), h.max(r)));
    assert!(
        (lo - 9.3).abs() <= 0.01 && (hi - 9.3).abs() <= 0.01,
        "{lo}..{hi}"
    );
    let line = made
        .iter()
        .copied()
        .find(|&id| as_path(&after, id).style.stroke.enabled)
        .expect("the ring's centre line");
    let at = |id| order.iter().position(|&o| o == id).unwrap();
    assert!(
        at(inner[0]) < at(line),
        "the inner area is under the stroke (stacking {order:?})"
    );
}

/// The geometry of a path without its ids, for comparing two runs.
fn shape_of(p: &PathSnapshot) -> String {
    let anchors: Vec<String> = p
        .all_anchors()
        .map(|a| format!("{:?} {:?} {:?}", a.point, a.handle_in, a.handle_out))
        .collect();
    let widths: Vec<(f64, f64)> = p
        .width_points
        .iter()
        .map(|w| (w.position, w.multiplier))
        .collect();
    format!(
        "{} {:?} {:?} {anchors:?} {widths:?}",
        p.closed, p.style, p.curve_kind
    )
}

fn results_shape(d: &Document, before: &HashSet<NodeId>) -> Vec<String> {
    new_ids(d, before)
        .into_iter()
        .map(|id| shape_of(&as_path(d, id)))
        .collect()
}

/// ADR 0063 §9 at the session: a compound frame with two holes, its subpaths in another order
/// and each ring started at another anchor, gives bit-identical results in the same order.
#[test]
fn determinism_permuted_subpaths_and_anchors_at_the_session() {
    let rotated = |mut ring: Vec<NewAnchor>, k: usize| {
        ring.rotate_left(k);
        ring
    };
    let frame = |order: [usize; 3], starts: [usize; 3]| {
        let mut rings = [
            rotated(
                vec![
                    corner(0.0, 0.0),
                    corner(40.0, 0.0),
                    corner(40.0, 20.0),
                    corner(0.0, 20.0),
                ],
                starts[0],
            ),
            rotated(circle(10.0, 10.0, 8.0, true), starts[1]),
            rotated(circle(30.0, 10.0, 8.0, true), starts[2]),
        ];
        let rings: Vec<Vec<NewAnchor>> = order
            .iter()
            .map(|&i| std::mem::take(&mut rings[i]))
            .collect();
        let d = Document::new(1);
        let subject = compound(&d, rings);
        filled(&d, &[subject], RED);
        let mut s = session_of(&d);
        let before = ids(&reread(&s));
        select(&mut s, &[subject]);
        let outcome = apply(&mut s);
        assert!(outcome.starts_with("Applied"), "{outcome}");
        results_shape(&reread(&s), &before)
    };
    let base = frame([0, 1, 2], [0, 0, 0]);
    assert!(!base.is_empty(), "the frame gives results");
    for (order, starts) in [
        ([2, 0, 1], [1, 2, 3]),
        ([1, 2, 0], [3, 1, 2]),
        ([0, 2, 1], [2, 3, 1]),
    ] {
        assert_eq!(
            base,
            frame(order, starts),
            "order {order:?}, starts {starts:?}"
        );
    }
}

/// Criterion 2 with groups: a selected group's filled leaves are replaced in their places in the
/// group's stacking order, the group and its name stay, a text inside is left alone, and a
/// nested group's leaves convert too; undo restores the document.
#[test]
fn groups_convert_their_leaves_in_place() {
    let d = Document::new(1);
    let a = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    let t = d.create_path(&tee(0.0, 20.0), true);
    let b = d.create_path(&stadium(0.0, 60.0, 40.0, 2.0), true);
    filled(&d, &[a, t, b], RED);
    let label = text(&d);
    let inner = d.group_objects(&[t, b]).unwrap();
    let outer = d.group_objects(&[a, inner, label]).unwrap();
    d.rename_node(outer, "Whale").unwrap();
    let mut s = session_of(&d);
    let json_before = json(&s);
    select(&mut s, &[outer]);
    assert!(s.centre_lines_available());
    let outcome = apply(&mut s);
    println!("group outcome: {outcome}");
    let (converted, _, _, _) = counts(&outcome);
    assert_eq!(converted, 3, "{outcome}");
    let after = reread(&s);
    assert!(after.is_group(outer), "the group stays");
    assert_eq!(name_of(&after, outer).as_deref(), Some("Whale"));
    assert!(after.object(label).is_some(), "the text is left alone");
    let leaves = after.leaves_of(outer);
    // Bottom to top: a's line, then the T's main and branch, then b's line, then the text.
    let ys: Vec<String> = leaves
        .iter()
        .map(|&id| match after.object(id) {
            Some(ObjectSnapshot::Path(p)) => {
                assert!(
                    !p.style.fill.enabled && p.style.stroke.enabled && p.style.stroke.color == RED
                );
                let y = p
                    .all_anchors()
                    .map(|q| q.point.y)
                    .fold(f64::INFINITY, f64::min);
                if y < 10.0 {
                    "a"
                } else if y < 50.0 {
                    "t"
                } else {
                    "b"
                }
                .to_owned()
            }
            _ => "text".to_owned(),
        })
        .collect();
    assert_eq!(ys, ["a", "t", "t", "b", "text"], "stacking {leaves:?}");
    s.undo();
    assert_eq!(json(&s), json_before, "undo restores the document");
}

/// The 100,000-node refusal counts the nodes a selected group holds and the nodes a clone shows.
#[test]
fn c18_refusal_counts_group_leaves_and_clones() {
    let d = Document::new(1);
    let a = polygon(&d, 60_000);
    let b = polygon(&d, 40_001);
    let group = d.group_objects(&[a, b]).unwrap();
    let mut s = session_of(&d);
    select(&mut s, &[group]);
    assert_eq!(
        s.centre_lines_refusal(),
        Some(CentreLinesRefusal::TooLarge { nodes: 100_001 })
    );
    let d = Document::new(1);
    let a = polygon(&d, 60_000);
    let clone = d.linked_clone(&[a], &[]).unwrap()[0];
    let mut s = session_of(&d);
    select(&mut s, &[a, clone]);
    let history = s.history_len();
    let refusal = s.centre_lines_refusal();
    println!("original + clone: {refusal:?}");
    assert_eq!(
        refusal,
        Some(CentreLinesRefusal::TooLarge { nodes: 120_000 })
    );
    assert!(apply(&mut s).starts_with("Refused"));
    assert_eq!(s.history_len(), history, "no history entry on a refusal");
}

/// Criterion 16 with the original and its clone selected together: both become results, the
/// clone as the path it shows, and undo brings the clone back.
#[test]
fn c16_original_and_its_clone_selected_together() {
    let d = Document::new(1);
    let subject = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    filled(&d, &[subject], RED);
    let clone = d.linked_clone(&[subject], &[]).unwrap()[0];
    let mut s = session_of(&d);
    let json_before = json(&s);
    let before = ids(&reread(&s));
    select(&mut s, &[subject, clone]);
    let outcome = apply(&mut s);
    println!("original + clone: {outcome}");
    assert_eq!(counts(&outcome).0, 2, "{outcome}");
    let after = reread(&s);
    let paths: Vec<PathSnapshot> = after
        .object_ids()
        .into_iter()
        .map(|id| as_path(&after, id))
        .collect();
    assert_eq!(paths.len(), 2, "two ordinary results, no clone left");
    assert!(
        paths
            .iter()
            .all(|p| !p.width_points.is_empty() && p.style.stroke.color == RED)
    );
    assert!(before.iter().all(|id| after.object(*id).is_none()
        || !matches!(after.object(*id), Some(ObjectSnapshot::Clone(_)))));
    s.undo();
    assert_eq!(json(&s), json_before);
}

/// Criterion 10 at the session (CENTRE-JUNCTION A): the Overlap option moves the start of the
/// branch of a T by `(0.5 - p) x w` from the bar's centre line, so each step of 0.2 in `p` moves
/// it by 0.4 mm at `w` = 2 mm, within 0.01 mm, and the document keeps one result per subject.
#[test]
fn c10_overlap_option_moves_the_branch_start_at_the_session() {
    let mut starts = Vec::new();
    for overlap in [0.2, 0.0, -0.2] {
        let d = Document::new(1);
        let t = d.create_path(&tee(0.0, 0.0), true);
        filled(&d, &[t], RED);
        let mut s = session_of(&d);
        select(&mut s, &[t]);
        let before = ids(&reread(&s));
        let outcome = apply_with(
            &mut s,
            CentreOptions {
                overlap,
                ..CentreOptions::default()
            },
        );
        assert_eq!(counts(&outcome).0, 1, "{outcome}");
        let after = reread(&s);
        let made = new_ids(&after, &before);
        assert_eq!(made.len(), 2, "bar and stem: {overlap}");
        // The branch is the path with an end on the stem's axis (x = 20).
        let branch = made
            .iter()
            .map(|&id| as_path(&after, id))
            .find(|p| {
                let pts: Vec<_> = p.all_anchors().map(|a| a.point).collect();
                let on_stem = |q: &curvyo_document_core::Point| (q.x - 20.0).abs() < 0.5;
                on_stem(pts.first().unwrap()) || on_stem(pts.last().unwrap())
            })
            .expect("a branch on the stem");
        let pts: Vec<_> = branch.all_anchors().map(|a| a.point).collect();
        let start = pts
            .iter()
            .filter(|q| (q.x - 20.0).abs() < 0.5)
            .map(|q| q.y)
            .fold(f64::INFINITY, f64::min);
        let want = 1.0 + (0.5 - overlap) * 2.0;
        assert!(
            (start - want).abs() <= 0.01,
            "overlap {overlap}: branch starts at y {start:.4}, want {want:.4}"
        );
        starts.push(start);
    }
    assert!((starts[1] - starts[0] - 0.4).abs() <= 0.02, "{starts:?}");
    assert!((starts[2] - starts[1] - 0.4).abs() <= 0.02, "{starts:?}");
}

// ---------- Round B (customer 2026-11-18): the bar, the worker's preview and the sliced Apply ----------

use curvyo_editor_wasm::centre_lines_glue::{CentreLinesWorker, centre_options};
use curvyo_editor_wasm::{KeyInput, Tool};

/// The worker's side of a bar session: its subjects and a preview of `options`.
fn worker_for(subjects_wire: &[f64], options: &CentreOptions) -> (CentreLinesWorker, Vec<f64>) {
    let mut worker = CentreLinesWorker::new();
    worker
        .set_subjects(subjects_wire, &mut |_| {})
        .expect("the session's subjects wire is well formed");
    let preview = worker.finish(options, true);
    (worker, preview)
}

/// Runs the sliced Apply to its end; the outcome and the number of slices.
fn apply_slices(s: &mut Session, wire: &[f64]) -> (String, usize) {
    s.centre_lines_apply_begin(wire).expect("apply begins");
    let mut slices = 0;
    loop {
        slices += 1;
        assert!(slices < 10_000, "apply never ends");
        if let Some(outcome) = s.centre_lines_apply_step() {
            return (format!("{outcome:?}"), slices);
        }
    }
}

fn two_strokes_and_a_tee(d: &Document) -> Vec<NodeId> {
    let a = d.create_path(&stadium(0.0, 0.0, 40.0, 2.0), true);
    let b = d.create_path(&stadium(0.0, 20.0, 30.0, 3.0), true);
    let t = d.create_path(&tee(60.0, 0.0), true);
    filled(d, &[a, b, t], RED);
    vec![a, b, t]
}

/// Criteria 15b and 2: the preview never changes the document or History; Apply from the worker's
/// result writes one undo step, equal to an Apply of the same options in one call, and undo and
/// redo restore the exact documents.
#[test]
fn c15b_preview_writes_nothing_and_apply_from_the_worker_is_one_step() {
    let options = CentreOptions {
        overlap: 0.0,
        optimise: 0.5,
        ..CentreOptions::default()
    };
    // Reference: the same Apply in one call.
    let d = Document::new(1);
    let subjects = two_strokes_and_a_tee(&d);
    let mut r = session_of(&d);
    select(&mut r, &subjects);
    let before_ids = ids(&reread(&r));
    let want_outcome = apply_with(&mut r, options);
    let want = results_shape(&reread(&r), &before_ids);

    let d = Document::new(1);
    let subjects = two_strokes_and_a_tee(&d);
    let mut s = session_of(&d);
    select(&mut s, &subjects);
    let before_json = json(&s);
    let before_ids = ids(&reread(&s));
    let history = s.history_len();
    let wire = s.centre_lines_open().expect("the bar opens");
    assert!(s.centre_lines_is_open());
    let (worker, preview) = worker_for(&wire, &options);
    let shown = s.centre_lines_show(&preview).expect("the preview fits");
    println!("preview counts: {shown:?}");
    s.centre_lines_set_stale(true);
    s.centre_lines_set_stale(false);
    assert_eq!(json(&s), before_json, "the preview is not in the document");
    assert_eq!(s.history_len(), history, "the preview is no history step");
    assert!(s.centre_lines_current());
    let (outcome, slices) = apply_slices(&mut s, &worker.result());
    println!("apply: {outcome} in {slices} slices");
    assert_eq!(outcome, want_outcome, "the same outcome as one Apply");
    assert!(!s.centre_lines_is_open(), "Apply closes the bar");
    assert_eq!(s.history_len(), history + 1, "exactly one history entry");
    let after_json = json(&s);
    assert_eq!(
        results_shape(&reread(&s), &before_ids),
        want,
        "the same results as one Apply"
    );
    s.undo();
    assert_eq!(json(&s), before_json, "undo restores the exact document");
    assert_eq!(selected(&mut s), subjects.iter().copied().collect());
    s.redo();
    assert_eq!(json(&s), after_json, "redo restores the result");
}

/// Criteria 15 and 15b: Cancel / Escape (`close`) writes nothing, also when Apply had begun; an
/// Apply after close is ignored.
#[test]
fn c15b_close_writes_nothing_even_after_apply_began() {
    let d = Document::new(1);
    let subjects = two_strokes_and_a_tee(&d);
    let mut s = session_of(&d);
    select(&mut s, &subjects);
    let before = json(&s);
    let history = s.history_len();
    let wire = s.centre_lines_open().unwrap();
    let (worker, preview) = worker_for(&wire, &CentreOptions::default());
    s.centre_lines_show(&preview).unwrap();
    s.centre_lines_close();
    assert!(!s.centre_lines_is_open());
    assert_eq!(json(&s), before);
    assert_eq!(s.history_len(), history);
    assert!(
        s.centre_lines_apply_begin(&worker.result()).is_err(),
        "closed: ignored"
    );
    let step = s.centre_lines_apply_step().map(|o| format!("{o:?}"));
    assert!(
        matches!(step.as_deref(), None | Some("Ignored")),
        "closed: {step:?}"
    );
    // Begin, then close before the slices end.
    let wire = s.centre_lines_open().unwrap();
    let (worker, preview) = worker_for(&wire, &CentreOptions::default());
    s.centre_lines_show(&preview).unwrap();
    s.centre_lines_apply_begin(&worker.result()).unwrap();
    s.centre_lines_close();
    for _ in 0..5 {
        let step = s.centre_lines_apply_step().map(|o| format!("{o:?}"));
        assert!(
            matches!(step.as_deref(), None | Some("Ignored")),
            "after close: {step:?}"
        );
    }
    assert_eq!(json(&s), before, "a dropped Apply writes nothing");
    assert_eq!(s.history_len(), history);
    assert_eq!(selected(&mut s), subjects.iter().copied().collect());
}

/// Criterion 15b and ADR 0063 amendment 2026-11-18, A: a selection or document change after the
/// subjects were sent makes the worker's result stale; Apply refuses it and writes nothing. A
/// preview of other subjects does not show.
#[test]
fn c15b_stale_subjects_are_refused_and_foreign_previews_do_not_show() {
    let d = Document::new(1);
    let subjects = two_strokes_and_a_tee(&d);
    let mut s = session_of(&d);
    select(&mut s, &subjects);
    let wire = s.centre_lines_open().unwrap();
    let (worker, _) = worker_for(&wire, &CentreOptions::default());
    // A preview made for another selection does not fit.
    let other = Document::new(1);
    let o = two_strokes_and_a_tee(&other);
    let mut so = session_of(&other);
    select(&mut so, &o[..1]);
    let other_wire = so.centre_lines_open().unwrap();
    let (_, foreign) = worker_for(&other_wire, &CentreOptions::default());
    assert!(
        s.centre_lines_show(&foreign).is_none(),
        "a foreign preview is not shown"
    );
    assert!(
        s.centre_lines_show(&[1.0, 2.0, 3.0]).is_none(),
        "a malformed wire is not shown"
    );
    // Selection change.
    select(&mut s, &subjects[..2]);
    let before = json(&s);
    let history = s.history_len();
    assert!(!s.centre_lines_current(), "the selection changed");
    let got = s.centre_lines_apply_begin(&worker.result());
    assert!(
        format!("{got:?}") == "Err(Stale)",
        "stale after a selection change: {got:?}"
    );
    assert_eq!(json(&s), before);
    assert_eq!(s.history_len(), history);
    // Document change: reopen, then move a subject by an edit outside the bar.
    let wire = s.centre_lines_open().unwrap();
    let (worker, _) = worker_for(&wire, &CentreOptions::default());
    let _ = &d;
    let before_nudge = json(&s);
    s.key_down(KeyInput {
        key: "ArrowRight",
        ..KeyInput::default()
    });
    let before = json(&s);
    assert_ne!(before, before_nudge, "the nudge changed the document");
    let history = s.history_len();
    let got = s.centre_lines_apply_begin(&worker.result());
    println!(
        "after a document change: current {}, {got:?}",
        s.centre_lines_current()
    );
    assert!(
        format!("{got:?}") == "Err(Stale)",
        "stale after a document change: {got:?}"
    );
    assert_eq!(json(&s), before);
    assert_eq!(s.history_len(), history);
}

/// Criterion 15b: deselecting (a press on empty canvas in the Node tool), another tool, Undo and
/// Redo cancel the bar and write nothing.
#[test]
fn c15b_deselect_tool_change_and_undo_cancel() {
    let d = Document::new(1);
    let subjects = two_strokes_and_a_tee(&d);
    let mut s = session_of(&d);
    s.set_tool(Tool::Node);
    select(&mut s, &subjects);
    let before = json(&s);
    let history = s.history_len();
    let _ = s.centre_lines_open().unwrap();
    let empty = curvyo_document_core::Point::new(500.0, 500.0);
    s.pointer_down(empty, false);
    s.pointer_up(empty, false, false);
    assert!(selected(&mut s).is_empty(), "the press deselects");
    assert!(!s.centre_lines_is_open(), "deselecting cancels the bar");
    assert_eq!(json(&s), before);
    assert_eq!(s.history_len(), history);
    select(&mut s, &subjects);
    let _ = s.centre_lines_open().unwrap();
    s.set_tool(Tool::Select);
    assert!(!s.centre_lines_is_open(), "another tool cancels");
    s.set_tool(Tool::Node);
    let _ = s.centre_lines_open().unwrap();
    s.undo();
    assert!(!s.centre_lines_is_open(), "Undo cancels");
}

/// Criteria 12, 15 and 15a at the binding: Thick parts 100 mm (the field's maximum) is no limit,
/// just under it is a limit; options that are not numbers take their defaults, but `+inf` Thick
/// parts is no limit like 100 mm (lead, 2026-11-19).
#[test]
fn c12_c15_binding_options_thick_parts_maximum_and_non_numbers() {
    let mm = |o: &CentreOptions| o.thick_parts.map(Length::as_mm);
    assert_eq!(
        mm(&centre_options(0.2, 100.0, 0.75, false, 0.3, true)),
        None
    );
    assert_eq!(
        mm(&centre_options(0.2, 250.0, 0.75, false, 0.3, true)),
        None
    );
    assert_eq!(
        mm(&centre_options(0.2, 99.9, 0.75, false, 0.3, true)),
        Some(99.9)
    );
    assert_eq!(
        mm(&centre_options(0.2, 6.0, 0.75, false, 0.3, true)),
        Some(6.0)
    );
    let defaults = CentreOptions::default();
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let o = centre_options(x, x, x, true, x, true);
        assert_eq!(o.overlap, defaults.overlap, "overlap {x}");
        let thick = if x == f64::INFINITY { None } else { Some(6.0) };
        assert_eq!(mm(&o), thick, "thick parts {x}");
        assert_eq!(o.optimise, defaults.optimise, "optimise {x}");
        assert_eq!(o.inner_offset.map(Length::as_mm), Some(0.3), "offset {x}");
    }
    let o = centre_options(0.9, 0.0, 2.0, false, 9.0, true);
    assert_eq!(o.overlap, 0.5, "overlap clamped");
    assert_eq!(o.optimise, 1.0, "optimise clamped");
    assert_eq!(o.inner_offset, None, "keep inner areas off");
    assert!(
        mm(&o).unwrap() > 0.0,
        "a zero limit is raised to the minimum"
    );
}

/// Criterion 18: the refusal at the limit stands for the bar too: nothing opens above 100,000.
#[test]
fn c18_the_bar_does_not_open_above_the_limit() {
    let d = Document::new(1);
    let big = polygon(&d, 100_001);
    let mut s = session_of(&d);
    select(&mut s, &[big]);
    assert_eq!(
        s.centre_lines_open().err(),
        Some(CentreLinesRefusal::TooLarge { nodes: 100_001 })
    );
    assert!(!s.centre_lines_is_open());
    let d = Document::new(1);
    let t = text(&d);
    let mut s = session_of(&d);
    select(&mut s, &[t]);
    assert!(s.centre_lines_open().is_err(), "a text opens no bar");
    assert!(!s.centre_lines_is_open());
}

/// Criteria 2 and 15b (architect review, blocker 1): an edit between two slices of Apply on the
/// whale (a nudge of the subjects, Undo, a selection change) writes nothing: no partial commit, no history entry, the document is the edited one.
#[test]
fn c15b_an_edit_between_apply_slices_on_the_whale_commits_nothing() {
    for disturbance in ["nudge", "undo", "selection"] {
        let d = Document::new(1);
        let mut subjects = whale(&d);
        let extra = d.create_path(&stadium(300.0, 300.0, 20.0, 2.0), true);
        filled(&d, &[extra], RED);
        subjects.push(extra);
        let mut s = session_of(&d);
        select(&mut s, &subjects);
        let wire = s.centre_lines_open().unwrap();
        let (worker, preview) = worker_for(&wire, &CentreOptions::default());
        s.centre_lines_show(&preview).unwrap();
        let slices = s.centre_lines_apply_begin(&worker.result()).unwrap();
        assert!(slices >= 2, "{disturbance}: the whale takes several slices");
        assert_eq!(
            s.centre_lines_apply_step(),
            None,
            "{disturbance}: first slice"
        );
        match disturbance {
            "nudge" => {
                let _ = s.key_down(KeyInput {
                    key: "ArrowRight",
                    ..KeyInput::default()
                });
            }
            "undo" => {
                let _ = s.undo();
            }
            _ => select(&mut s, &subjects[..1]),
        }
        let before = json(&s);
        let history = s.history_len();
        let mut outcome = None;
        for _ in 0..=slices {
            outcome = s.centre_lines_apply_step();
            if outcome.is_some() {
                break;
            }
        }
        let outcome = format!("{outcome:?}");
        assert!(
            outcome == "Some(Stale)" || outcome == "Some(Ignored)",
            "{disturbance}: {outcome}"
        );
        assert_eq!(json(&s), before, "{disturbance}: no partial commit");
        assert_eq!(s.history_len(), history, "{disturbance}: no history entry");
        for _ in 0..3 {
            let later = format!("{:?}", s.centre_lines_apply_step());
            assert!(
                later == "None" || later == "Some(Ignored)",
                "{disturbance}: later step {later}"
            );
        }
        assert_eq!(json(&s), before, "{disturbance}: later steps write nothing");
    }
}
