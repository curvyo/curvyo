//! Independent tester cases for `specs/0080-text-on-path/specification.md` slice 1 on
//! `curvyo-document-core` (ADR 0022): the carrier becomes a guide in the same step and Undo gives
//! back the straight text and the old role (criteria 4, 6), a locked carrier is not marked (4, 7),
//! the carrier stays a guide after Release and removal of the text (4a), a linked clone carries its
//! own guide (ADR 0022 §2), the history labels (17, 18), Release (23), the kept outline of a
//! deleted carrier and Undo (25), Duplicate (31), several texts on one carrier (32), save and
//! reopen, the golden file and older files (34), merge residue (35) and two peers (36).
//!
//! Written from the specification, ADR 0022 and the public names before the implementation was
//! read.

// Short names (d, c, t, v) and local `use` of enum variants keep the cases readable; the boxed
// raw-edit closures are a test table.
#![allow(
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::type_complexity,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, CarrierRef,
    CopySource, Document, EllipseFrame, FontKind, FontRef, Length, LineOnPath, NewAnchor, NodeId,
    ObjectSnapshot, OnPathLabel, OnPathValues, PathSide, Point, TextLayoutValues, TextSnapshot,
    Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

const TOL: f64 = 0.01;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn sans() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn circle(d: &Document) -> NodeId {
    d.create_ellipse(EllipseFrame {
        center: pt(100.0, 100.0),
        rx: Length::from_mm(50.0),
        ry: Length::from_mm(50.0),
    })
}

fn open_line(d: &Document, n: u64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, n * 10), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(9, n * 10 + 1), pt(100.0, 0.0)),
        ],
        false,
    )
}

const STRAIGHT: Affine = Affine::translation(Vec2::new(10.0, 20.0));

fn text(d: &Document, s: &str) -> NodeId {
    d.create_text(
        s,
        &sans(),
        Length::from_mm(10.0),
        STRAIGHT,
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn on(node: NodeId) -> CarrierRef {
    CarrierRef { node, outline: 0 }
}

fn put(d: &Document, t: NodeId, carrier: NodeId, values: &OnPathValues) {
    d.put_text_on_path(t, on(carrier), values, true).unwrap();
}

fn guide_of(d: &Document, id: NodeId) -> bool {
    d.read_nodes().iter().find(|n| n.id == id).unwrap().guide
}

fn labels(d: &Document) -> Vec<String> {
    d.own_steps().into_iter().map(|s| s.label).collect()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn snap(d: &Document, id: NodeId) -> TextSnapshot {
    d.text(id).unwrap()
}

fn values(d: &Document, id: NodeId) -> OnPathValues {
    snap(d, id).on_path.unwrap().values
}

/// The carrier outline a text resolves, as flat numbers (points and handles).
fn curve_numbers(t: &TextSnapshot) -> Vec<f64> {
    let p = t.on_path.as_ref().unwrap();
    let c = p.curve.as_ref().expect("a resolved curve");
    c.anchors
        .iter()
        .flat_map(|a| {
            [
                a.point.x,
                a.point.y,
                a.handle_in.x,
                a.handle_in.y,
                a.handle_out.x,
                a.handle_out.y,
            ]
        })
        .collect()
}

fn same_numbers(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < TOL)
}

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn file_from_loro(loro: &LoroDoc) -> Vec<u8> {
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        (
            "document.loro",
            loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        w.start_file(name, o).unwrap();
        w.write_all(&bytes).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// Criteria 4 and 6: one step "Put text on path" that also makes the carrier a guide; the
/// ellipse stays an ellipse with its frame; Undo gives back the straight text with its old
/// placement and the carrier with its old role. Guide off later leaves the text on the carrier.
#[test]
fn c4_c6_put_marks_the_carrier_in_the_same_step_and_undo_restores_both() {
    let d = Document::new(1);
    let c = circle(&d);
    let t = text(&d, "Badge");
    let shape_before = d.primitive(c).unwrap();
    let n = d.own_steps().len();
    put(&d, t, c, &OnPathValues::at(Length::from_mm(12.0)));
    assert_eq!(d.own_steps().len(), n + 1, "one step, the guide included");
    assert_eq!(labels(&d).last().unwrap(), "put_text_on_path");
    assert!(guide_of(&d, c), "the carrier became a guide");
    assert_eq!(
        d.primitive(c).unwrap().shape,
        shape_before.shape,
        "still the same ellipse"
    );
    let v = values(&d, t);
    assert_eq!((v.side, v.line), (PathSide::Above, LineOnPath::Baseline));
    assert!((v.position.as_mm() - 12.0).abs() < 1e-9);
    assert_eq!(v.distance.as_mm(), 0.0);

    undo_last(&d);
    let back = snap(&d, t);
    assert!(back.on_path.is_none(), "straight again");
    assert_eq!(back.placement, STRAIGHT, "with its old transform");
    assert!(!guide_of(&d, c), "the carrier's old role");

    // Again, then Guide off: the text stays on the carrier.
    put(&d, t, c, &OnPathValues::at(Length::from_mm(12.0)));
    d.set_guide(&[c], false).unwrap();
    assert!(!guide_of(&d, c));
    assert!(
        snap(&d, t)
            .on_path
            .as_ref()
            .is_some_and(|p| p.curve.is_some())
    );
}

/// Criterion 4: a carrier that is a guide already stays one, and Undo of the put leaves it one.
#[test]
fn c4_a_guide_carrier_stays_a_guide_through_put_and_undo() {
    let d = Document::new(1);
    let c = circle(&d);
    d.set_guide(&[c], true).unwrap();
    let t = text(&d, "A");
    put(&d, t, c, &OnPathValues::at(Length::from_mm(0.0)));
    assert!(guide_of(&d, c));
    undo_last(&d);
    assert!(guide_of(&d, c), "its role from before: a guide");
}

/// Criteria 4 and 7: a carrier on a locked layer is only read: the text goes on it, the carrier
/// is not made a guide.
#[test]
fn c4_c7_carrier_on_a_locked_layer_is_not_marked_but_carries() {
    let d = Document::new(1);
    let locked_layer = d.new_layer(None);
    d.set_draw_parent(Some(locked_layer));
    let c = circle(&d);
    let open_layer = d.new_layer(None);
    d.set_draw_parent(Some(open_layer));
    let t = text(&d, "Lid");
    d.set_node_locked(&[locked_layer], true).unwrap();
    d.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(3.0)), true)
        .unwrap();
    assert!(!guide_of(&d, c), "a locked carrier is not written");
    assert!(
        snap(&d, t)
            .on_path
            .as_ref()
            .is_some_and(|p| p.curve.is_some()),
        "the text is put on it anyway"
    );
}

/// Criterion 4a: Release from path and removing the text leave the carrier a guide.
#[test]
fn c4a_release_and_removal_of_the_text_leave_the_carrier_a_guide() {
    let d = Document::new(1);
    let c = circle(&d);
    let t1 = text(&d, "One");
    let t2 = text(&d, "Two");
    put(&d, t1, c, &OnPathValues::at(Length::from_mm(0.0)));
    put(&d, t2, c, &OnPathValues::at(Length::from_mm(50.0)));
    d.release_from_path(&[(t1, STRAIGHT)]).unwrap();
    assert!(guide_of(&d, c), "after Release");
    d.delete_objects(&[t2]).unwrap();
    assert!(guide_of(&d, c), "after removing the text");
}

/// Criterion 23 (document side): Release deletes the link and writes the straight placement in
/// one step "Release from path"; the carrier is not written.
#[test]
fn c23_release_writes_the_placement_and_drops_the_link_in_one_step() {
    let d = Document::new(1);
    let c = circle(&d);
    let t = text(&d, "Name");
    put(&d, t, c, &OnPathValues::at(Length::from_mm(7.0)));
    let carrier_before = d.primitive(c).unwrap();
    let n = d.own_steps().len();
    let released = Affine::rotation_about(
        pt(40.0, 60.0),
        curvyo_document_core::Angle::from_radians(0.3),
    );
    d.release_from_path(&[(t, released)]).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert_eq!(labels(&d).last().unwrap(), "release_from_path");
    let s = snap(&d, t);
    assert!(s.on_path.is_none());
    assert_eq!(s.placement, released);
    assert_eq!(d.primitive(c).unwrap(), carrier_before);
}

/// Criteria 17 and 18: every option change is one step with its own label; Mirror is "Mirror
/// text on path" and swaps Above + Baseline <-> Below + Top edge, Above + Top edge <-> Below +
/// Baseline with the position kept.
#[test]
fn c17_c18_option_steps_and_mirror_mapping() {
    let d = Document::new(1);
    let c = circle(&d);
    let t = text(&d, "Opt");
    put(&d, t, c, &OnPathValues::at(Length::from_mm(20.0)));
    let base = values(&d, t);
    let cases = [
        (
            OnPathValues {
                side: PathSide::Below,
                ..base
            },
            OnPathLabel::Side,
            "change_text_side",
        ),
        (
            OnPathValues {
                side: PathSide::Below,
                line: LineOnPath::TopEdge,
                ..base
            },
            OnPathLabel::Line,
            "change_line_on_path",
        ),
        (
            OnPathValues {
                position: Length::from_mm(33.0),
                ..base
            },
            OnPathLabel::Move,
            "move_text_on_path",
        ),
        (
            OnPathValues {
                distance: Length::from_mm(-2.5),
                ..base
            },
            OnPathLabel::Distance,
            "change_distance_on_path",
        ),
    ];
    for (v, label, expected) in cases {
        let n = d.own_steps().len();
        d.set_text_path(&[(t, v)], label).unwrap();
        assert_eq!(d.own_steps().len(), n + 1, "{expected}: one step");
        assert_eq!(labels(&d).last().unwrap(), expected);
        assert_eq!(values(&d, t), v);
    }
    assert_eq!(OnPathLabel::Mirror.label(), "mirror_text_on_path");

    let mk = |side, line| OnPathValues {
        side,
        line,
        ..OnPathValues::at(Length::from_mm(20.0))
    };
    use LineOnPath::{Baseline, TopEdge};
    use PathSide::{Above, Below};
    for (from, to) in [
        (mk(Above, Baseline), mk(Below, TopEdge)),
        (mk(Below, TopEdge), mk(Above, Baseline)),
        (mk(Above, TopEdge), mk(Below, Baseline)),
        (mk(Below, Baseline), mk(Above, TopEdge)),
    ] {
        assert_eq!(from.mirrored(), to, "{from:?}");
        assert_eq!(
            from.reads_with_carrier(),
            to.reads_with_carrier(),
            "Mirror keeps the reading direction"
        );
    }
    // Reading direction of the Terms: Above + Baseline and Below + Top edge read with the carrier.
    assert!(mk(Above, Baseline).reads_with_carrier());
    assert!(mk(Below, TopEdge).reads_with_carrier());
    assert!(!mk(Above, TopEdge).reads_with_carrier());
    assert!(!mk(Below, Baseline).reads_with_carrier());
}

/// Criterion 25: removing the carrier keeps its outline on the text (same step), the text still
/// resolves the same curve; Undo brings the carrier and the live link back.
#[test]
fn c25_deleted_carrier_is_kept_on_the_text_and_undo_relinks() {
    let d = Document::new(1);
    let c = circle(&d);
    let t = text(&d, "Kept");
    put(&d, t, c, &OnPathValues::at(Length::from_mm(40.0)));
    let before = curve_numbers(&snap(&d, t));
    let n = d.own_steps().len();
    d.delete_objects(&[c]).unwrap();
    assert_eq!(d.own_steps().len(), n + 1, "the same step as the removal");
    assert!(d.object(c).is_none(), "the carrier is gone");
    let kept = snap(&d, t);
    let p = kept.on_path.as_ref().unwrap();
    assert!(p.kept, "the text sits on its kept outline");
    assert!(same_numbers(&curve_numbers(&kept), &before), "same curve");
    assert_eq!(d.take_kept_carriers(), 1, "notice: 1 text kept its path");

    undo_last(&d);
    assert!(d.object(c).is_some(), "carrier back");
    let relinked = snap(&d, t);
    let p = relinked.on_path.as_ref().unwrap();
    assert!(!p.kept, "the live link wins again");
    assert_eq!(p.carrier.node, c);
    assert!(same_numbers(&curve_numbers(&relinked), &before));
}

/// Criterion 25: the kept outline survives save and reopen (criterion 34 names it).
#[test]
fn c25_c34_kept_outline_survives_save_and_reopen() {
    let d = Document::new(1);
    let c = circle(&d);
    let t = text(&d, "Kept");
    put(&d, t, c, &OnPathValues::at(Length::from_mm(40.0)));
    let before = curve_numbers(&snap(&d, t));
    d.delete_objects(&[c]).unwrap();
    let again = unpack(2, &pack(&d, "tester").unwrap()).unwrap();
    let s = again.text(t).unwrap();
    assert!(s.on_path.as_ref().unwrap().kept);
    assert!(same_numbers(&curve_numbers(&s), &before));
}

/// ADR 0022 §2 with criterion 4: a linked clone as carrier gets its own guide flag; its original
/// is not made a guide. The text sits on the clone's resolved outline.
#[test]
fn adr0022_clone_carrier_carries_its_own_guide() {
    let d = Document::new(1);
    let src = open_line(&d, 1);
    let clone = d.linked_clone(&[src], &[]).unwrap()[0];
    let t = text(&d, "Clone");
    put(&d, t, clone, &OnPathValues::at(Length::from_mm(0.0)));
    assert!(guide_of(&d, clone), "the clone is a guide");
    assert!(!guide_of(&d, src), "the original is not");
    assert!(
        snap(&d, t)
            .on_path
            .as_ref()
            .is_some_and(|p| p.curve.is_some()),
        "a resolved clone carries"
    );
    // Guide off on the clone works (ADR 0022 §2).
    d.set_guide(&[clone], false).unwrap();
    assert!(!guide_of(&d, clone));
}

/// Criterion 31: Duplicate of a text on path alone gives a text on the same carrier with
/// Position + 5 mm; of the text with its carrier, the copy sits on the carrier's copy.
#[test]
fn c31_duplicate_alone_shifts_five_mm_and_with_carrier_goes_on_the_copy() {
    let d = Document::new(1);
    let c = circle(&d);
    let t = text(&d, "Dup");
    put(&d, t, c, &OnPathValues::at(Length::from_mm(10.0)));
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: t,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap()[0];
    let p = snap(&d, copy).on_path.unwrap();
    assert_eq!(p.carrier.node, c, "same carrier");
    assert!(
        (p.values.position.as_mm() - 15.0).abs() < 1e-6,
        "Position + 5 mm, got {}",
        p.values.position.as_mm()
    );

    let both = d
        .duplicate_objects(
            &[
                CopySource {
                    id: c,
                    anchor_ids: Vec::new(),
                },
                CopySource {
                    id: t,
                    anchor_ids: Vec::new(),
                },
            ],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    let carrier_copy = both
        .iter()
        .copied()
        .find(|&id| matches!(d.object(id), Some(ObjectSnapshot::Primitive(_))))
        .unwrap();
    let text_copy = both
        .iter()
        .copied()
        .find(|&id| d.text(id).is_some())
        .unwrap();
    let p = snap(&d, text_copy).on_path.unwrap();
    assert_eq!(p.carrier.node, carrier_copy, "the copy is on the copy");
    assert!((p.values.position.as_mm() - 10.0).abs() < 1e-6);
}

/// Criterion 32: two texts on one circle keep their own string, side, line and position.
#[test]
fn c32_several_texts_on_one_carrier_are_independent() {
    let d = Document::new(1);
    let c = circle(&d);
    let top = text(&d, "TOP");
    let bottom = text(&d, "BOTTOM");
    put(&d, top, c, &OnPathValues::at(Length::from_mm(200.0)));
    let b = OnPathValues {
        side: PathSide::Below,
        line: LineOnPath::Baseline,
        ..OnPathValues::at(Length::from_mm(80.0))
    };
    put(&d, bottom, c, &b);
    d.set_text_path(
        &[(
            top,
            OnPathValues {
                position: Length::from_mm(210.0),
                ..values(&d, top)
            },
        )],
        OnPathLabel::Move,
    )
    .unwrap();
    assert_eq!(values(&d, bottom), b, "the other text is untouched");
    assert!((values(&d, top).position.as_mm() - 210.0).abs() < 1e-9);
    assert_eq!(snap(&d, top).string, "TOP");
    assert_eq!(snap(&d, bottom).string, "BOTTOM");
}

/// Criterion 34: save, close, reopen keeps every value, the guide role and the curve.
#[test]
fn c34_round_trip_keeps_values_guide_and_curve() {
    let d = Document::new(1);
    let c = circle(&d);
    let t = text(&d, "Round");
    let v = OnPathValues {
        side: PathSide::Below,
        line: LineOnPath::TopEdge,
        position: Length::from_mm(123.25),
        distance: Length::from_mm(-1.5),
        mirror_letters: true,
    };
    put(&d, t, c, &v);
    let before = curve_numbers(&snap(&d, t));
    let again = unpack(2, &pack(&d, "tester").unwrap()).unwrap();
    assert!(guide_of(&again, c));
    let s = again.text(t).unwrap();
    assert_eq!(s.on_path.as_ref().unwrap().values, v);
    assert!(same_numbers(&curve_numbers(&s), &before));
}

/// Criterion 34: format 22 and the golden file with the cases the criterion lists.
#[test]
fn c34_golden_file_opens_with_texts_on_every_carrier_kind() {
    assert_eq!(CURRENT_FORMAT_VERSION, 22);
    let d = unpack(3, &fixture("text_on_path_v22.curvyo")).expect("golden opens");
    let texts: Vec<TextSnapshot> = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Text(t) => Some((**t).clone()),
            _ => None,
        })
        .collect();
    let on_path: Vec<&TextSnapshot> = texts
        .iter()
        .filter(|t| t.on_path.as_ref().is_some_and(|p| p.curve.is_some()))
        .collect();
    // Open path, four circle cases, rectangle, star, ellipse arc, compound outline, kept.
    assert!(
        on_path.len() >= 10,
        "texts on a curve in the golden file: {}",
        on_path.len()
    );
    let mut cases: Vec<(PathSide, LineOnPath)> = on_path
        .iter()
        .map(|t| {
            let v = t.on_path.as_ref().unwrap().values;
            (v.side, v.line)
        })
        .collect();
    cases.sort_by_key(|c| format!("{c:?}"));
    cases.dedup();
    assert_eq!(cases.len(), 4, "all four Side and Line cases");
    assert!(
        on_path.iter().any(|t| t.on_path.as_ref().unwrap().kept),
        "a text with a kept outline"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.on_path.as_ref().is_none_or(|p| p.curve.is_none())),
        "the text whose carrier names nothing reads straight"
    );
    assert!(
        on_path
            .iter()
            .any(|t| t.on_path.as_ref().unwrap().carrier.outline > 0),
        "a compound outline"
    );
    // Reopened and saved again, it reads the same.
    let again = unpack(4, &pack(&d, "tester").unwrap()).unwrap();
    for t in &on_path {
        let s = again.text(t.id).unwrap();
        assert_eq!(
            s.on_path.as_ref().unwrap().values,
            t.on_path.as_ref().unwrap().values
        );
    }
}

/// Criterion 34: an older file (format 21) opens with every text straight.
#[test]
fn c34_older_file_opens_with_straight_texts() {
    let d = unpack(3, &fixture("text_layout_v21.curvyo")).expect("v21 opens");
    let objects = d.read_objects();
    let mut any = false;
    for o in objects.objects.iter() {
        if let ObjectSnapshot::Text(t) = o {
            any = true;
            assert!(t.on_path.is_none());
        }
    }
    assert!(any, "the fixture has texts");
}

/// Criterion 35: a carrier id that names nothing, a carrier that is not a carrier kind (a
/// text), and non-finite values open as a straight text at the stored anchor; nothing crashes.
#[test]
fn c35_merge_residue_opens_as_straight_text() {
    let d = Document::new(1);
    let c = circle(&d);
    let other = text(&d, "Other");
    let t = text(&d, "Residue");
    put(&d, t, c, &OnPathValues::at(Length::from_mm(10.0)));
    let base = LoroDoc::new();
    base.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let bytes = base.export(loro::ExportMode::Snapshot).unwrap();

    let raw = |edit: &dyn Fn(&loro::LoroMap)| {
        let l = LoroDoc::new();
        l.import(&bytes).unwrap();
        l.set_peer_id(7).unwrap();
        let meta = l.get_tree("paths").get_meta(tree_id(t)).unwrap();
        edit(&meta);
        unpack(5, &file_from_loro(&l)).expect("opens, never refused")
    };
    let other_json = serde_json::to_value(other).unwrap();
    let nothing = serde_json::json!({"peer": "424242", "counter": 99});
    let cases: Vec<(&str, Box<dyn Fn(&loro::LoroMap)>)> = vec![
        (
            "carrier names nothing",
            Box::new(move |m: &loro::LoroMap| {
                let v: LoroValue = serde_json::from_value::<LoroValue>(
                    serde_json::json!({"node": nothing.clone(), "outline": 0}),
                )
                .unwrap();
                m.insert("text_carrier", v).unwrap();
            }),
        ),
        (
            "carrier is a text",
            Box::new(move |m: &loro::LoroMap| {
                let v: LoroValue = serde_json::from_value::<LoroValue>(
                    serde_json::json!({"node": other_json.clone(), "outline": 0}),
                )
                .unwrap();
                m.insert("text_carrier", v).unwrap();
            }),
        ),
        (
            "position NaN",
            Box::new(|m: &loro::LoroMap| {
                m.insert("text_path_position", f64::NAN).unwrap();
            }),
        ),
        (
            "distance infinite",
            Box::new(|m: &loro::LoroMap| {
                m.insert("text_path_distance", f64::INFINITY).unwrap();
            }),
        ),
    ];
    for (name, edit) in cases {
        let opened = raw(edit.as_ref());
        let s = opened.text(t).expect(name);
        assert!(
            s.on_path.as_ref().is_none_or(|p| p.curve.is_none()),
            "{name}: reads straight, got {:?}",
            s.on_path.as_ref().map(|p| p.values)
        );
        assert_eq!(s.placement, STRAIGHT, "{name}: at its stored anchor");
        let _ = opened.read_objects();
    }
}

/// Criterion 36: one peer moves the carrier while the other edits the string; after the merge
/// both show the new string on the moved carrier.
#[test]
fn c36_peers_converge_on_new_string_and_moved_carrier() {
    let a = Document::new(1);
    let c = circle(&a);
    let t = text(&a, "Old");
    put(&a, t, c, &OnPathValues::at(Length::from_mm(10.0)));
    let b = Document::new(2);
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    a.set_ellipse_frame(
        c,
        EllipseFrame {
            center: pt(150.0, 120.0),
            rx: Length::from_mm(50.0),
            ry: Length::from_mm(50.0),
        },
    )
    .unwrap();
    b.edit_text(t, "New", &curvyo_document_core::TextChars::default())
        .unwrap();
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
    for d in [&a, &b] {
        let s = d.text(t).unwrap();
        assert_eq!(s.string, "New");
        let first = curve_numbers(&s);
        // The rightmost point of the moved circle is at (200, 120).
        let xs: Vec<f64> = first.chunks(6).map(|a| a[0]).collect();
        let max_x = xs.iter().copied().fold(f64::MIN, f64::max);
        assert!((max_x - 200.0).abs() < TOL, "moved carrier, max x {max_x}");
    }
    assert_eq!(
        curve_numbers(&a.text(t).unwrap()),
        curve_numbers(&b.text(t).unwrap())
    );
}
