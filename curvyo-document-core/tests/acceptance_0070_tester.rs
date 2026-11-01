//! Independent tester tests for `specs/0070-measure-and-dimension` at the document model
//! (ADR 0027): format 31 and its goldens, older and newer files, damaged files, linked
//! resolution on read, the kept number on delete and the commands that replace geometry, undo
//! relinking, copy remap, convert to paths and a text, the output filter, history labels and
//! units. Written from the specification and the ADR; only public names were looked up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(clippy::similar_names, clippy::many_single_char_names)]
#![allow(clippy::cast_possible_truncation, clippy::items_after_statements)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, CopySource, CurveSlot,
    DimensionKind, DimensionPlace, DimensionSlots, DimensionSnapshot, DimensionValue, Document,
    EllipseFrame, Length, NewAnchor, NodeId, ObjectEditError, ObjectSnapshot, OpenError, PieceFill,
    Point, PointSlot, RectBounds, ShapeFeature, Vec2, pack, unpack,
};
use loro::LoroDoc;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(path).unwrap()
}

/// An open two-node path from `a` to `b`; anchor ids (1, n) and (1, n + 1).
fn line(d: &Document, n: u64, a: Point, b: Point) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n), a),
            NewAnchor::corner(AnchorId::new(1, n + 1), b),
        ],
        false,
    )
}

fn anchor(node: NodeId, n: u64) -> PointSlot {
    PointSlot::Anchor {
        node,
        anchor: AnchorId::new(1, n),
    }
}

fn place() -> DimensionPlace {
    DimensionPlace::at_offset(Length::from_mm(5.0))
}

/// A 50 mm aligned dimension between the ends of a 30-40-50 path.
fn fifty(d: &Document) -> (NodeId, NodeId) {
    let p = line(d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let dim = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(p, 10), anchor(p, 11)]),
            place(),
        )
        .unwrap();
    (p, dim)
}

fn dim(d: &Document, id: NodeId) -> DimensionSnapshot {
    d.dimension(id).expect("a live dimension")
}

fn mm(v: DimensionValue) -> f64 {
    match v {
        DimensionValue::Length(l) => l.as_mm(),
        DimensionValue::Angle(_) => panic!("expected a length, got an angle"),
    }
}

fn labels(d: &Document) -> Vec<String> {
    d.own_steps().into_iter().map(|s| s.label).collect()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn json(d: &Document) -> String {
    String::from_utf8(d.export_json().unwrap()).unwrap()
}

fn member(zip_bytes: &[u8], name: &str) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
    let mut f = z.by_name(name).unwrap();
    let mut out = Vec::new();
    f.read_to_end(&mut out).unwrap();
    out
}

fn manifest_version(zip_bytes: &[u8]) -> u64 {
    let m: serde_json::Value = serde_json::from_slice(&member(zip_bytes, "manifest.json")).unwrap();
    m["format_version"].as_u64().unwrap()
}

fn loro_of(d: &Document) -> LoroDoc {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn file_from_loro(loro: &LoroDoc, version: u32) -> Vec<u8> {
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": version,
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

fn tree_id(id: NodeId) -> loro::TreeID {
    let v = serde_json::to_value(id).unwrap();
    loro::TreeID::new(
        v["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(v["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn dimensions(d: &Document) -> Vec<DimensionSnapshot> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Dimension(x) => Some((**x).clone()),
            _ => None,
        })
        .collect()
}

// ---------- criterion 16: format 31, goldens, older and newer files ----------

#[test]
fn c16_format_version_is_31_and_a_saved_file_says_so() {
    assert_eq!(CURRENT_FORMAT_VERSION, 31);
    let d = Document::new(1);
    fifty(&d);
    assert_eq!(manifest_version(&pack(&d, "t").unwrap()), 31);
}

#[test]
fn c16_golden_v31_opens_with_dimensions_of_every_kind_family_and_round_trips() {
    let bytes = fixture("dimension_v31.curvyo");
    assert_eq!(manifest_version(&bytes), 31);
    let d = unpack(3, &bytes).unwrap();
    let dims = dimensions(&d);
    assert!(!dims.is_empty(), "the golden holds dimensions");
    let kinds: std::collections::HashSet<_> = dims.iter().map(|x| x.kind).collect();
    assert!(
        kinds.len() >= 3,
        "the golden covers several kinds, got {kinds:?}"
    );
    for x in &dims {
        assert!(x.shown.is_finite() && x.kept.is_finite());
        assert!(x.slots.fit(x.kind));
    }
    // Open writes nothing; save, open, save is identical.
    assert!(d.own_steps().is_empty(), "opening writes no step");
    let one = pack(&d, "t").unwrap();
    let again = unpack(4, &one).unwrap();
    assert_eq!(json(&d), json(&again));
    assert_eq!(
        member(&pack(&again, "t").unwrap(), "document.json"),
        member(&one, "document.json")
    );
    assert_eq!(dimensions(&again), dims);
}

#[test]
fn c16_save_open_round_trip_keeps_kind_slots_place_kept_and_link() {
    let d = Document::new(1);
    let (p, a) = fifty(&d);
    let c = d.create_ellipse(EllipseFrame {
        center: pt(100.0, 100.0),
        rx: Length::from_mm(25.0),
        ry: Length::from_mm(25.0),
    });
    let r = d
        .add_dimension(
            DimensionKind::Diameter,
            DimensionSlots::Curve(CurveSlot::Circle { node: c }),
            DimensionPlace {
                offset: Length::from_mm(3.0),
                along: 0.3,
                leader: curvyo_document_core::Angle::from_radians(1.0),
                sector: 0,
            },
        )
        .unwrap();
    let free = d
        .add_dimension(
            DimensionKind::Horizontal,
            DimensionSlots::Points(vec![
                PointSlot::Free {
                    point: pt(-5.0, 1.0),
                },
                PointSlot::ShapePoint {
                    node: c,
                    feature: ShapeFeature::Center,
                },
            ]),
            place(),
        )
        .unwrap();
    let back = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    for id in [a, r, free] {
        assert_eq!(dim(&back, id), dim(&d, id), "dimension {id:?} survives");
        assert!(dim(&back, id).linked);
    }
    assert!((mm(dim(&back, r).shown.value) - 50.0).abs() < 1e-9);
    assert!((mm(dim(&back, free).shown.value) - 105.0).abs() < 1e-9);
    let _ = p;
}

#[test]
fn c16_older_files_open_and_a_newer_one_is_refused() {
    for name in ["grid_v30.curvyo", "sketch_dimension_label_v29.curvyo"] {
        let d = unpack(2, &fixture(name)).unwrap();
        assert!(dimensions(&d).is_empty(), "{name} has no dimensions");
    }
    let d = Document::new(1);
    fifty(&d);
    let bytes = file_from_loro(&loro_of(&d), 32);
    assert!(matches!(
        unpack(2, &bytes),
        Err(OpenError::FormatTooNew {
            found: 32,
            supported: 31
        })
    ));
}

#[test]
fn c16_damaged_goldens_are_refused() {
    for name in [
        "dimension_kind_unknown_v31.curvyo",
        "dimension_slot_malformed_v31.curvyo",
        "dimension_slots_count_v31.curvyo",
        "dimension_value_nan_v31.curvyo",
    ] {
        assert!(
            matches!(unpack(2, &fixture(name)), Err(OpenError::Damaged)),
            "{name} must be refused as damaged"
        );
    }
}

/// My own damage, independent of the shipped fixtures: overwrite one register of a valid
/// dimension in the Loro tree and save as version 31.
fn damaged_with(key: &str, value: loro::LoroValue) -> Result<Document, OpenError> {
    let d = Document::new(1);
    let (_, id) = fifty(&d);
    let l = loro_of(&d);
    let tree = l.get_tree("paths");
    let meta = tree.get_meta(tree_id(id)).unwrap();
    assert!(
        meta.get(key).is_some(),
        "register {key} exists on a dimension"
    );
    meta.insert(key, value).unwrap();
    unpack(2, &file_from_loro(&l, 31))
}

#[test]
fn c16_my_own_damaged_registers_are_refused() {
    use loro::LoroValue as V;
    assert!(matches!(
        damaged_with("dimension_kind", V::from("ordinate")),
        Err(OpenError::Damaged)
    ));
    assert!(matches!(
        damaged_with("dimension_kind", V::from(3_i64)),
        Err(OpenError::Damaged)
    ));
    // Aligned with three slots: count does not fit the kind.
    assert!(
        matches!(
            damaged_with("dimension_kind", V::from("angle")),
            Err(OpenError::Damaged)
        ) || matches!(
            damaged_with("dimension_kind", V::from("radius")),
            Err(OpenError::Damaged)
        )
    );
    assert!(matches!(
        damaged_with("dimension_kind", V::from("radius")),
        Err(OpenError::Damaged)
    ));
    assert!(matches!(
        damaged_with("dimension_ref_a", V::from("garbage")),
        Err(OpenError::Damaged)
    ));
    assert!(matches!(
        damaged_with("dimension_kept", V::from(true)),
        Err(OpenError::Damaged)
    ));
    assert!(matches!(
        damaged_with("dimension_place", V::from("x")),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn c16_a_stale_kept_number_in_a_file_shows_the_resolved_value_and_opening_writes_nothing() {
    // Merge residue (ADR 0027 Consequences): the file's kept value is older than the geometry.
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    let l = loro_of(&d);
    // Move the path's second anchor behind the document's back: build the doc again by moving
    // and then restoring the kept register from before the move.
    let tree = l.get_tree("paths");
    let kept_before = tree.get_meta(tree_id(id)).unwrap().get("dimension_kept");
    d.move_anchors(&[(p, AnchorId::new(1, 11), pt(60.0, 80.0))])
        .unwrap();
    assert!((mm(dim(&d, id).kept.value) - 100.0).abs() < 1e-9);
    let l2 = loro_of(&d);
    let tree2 = l2.get_tree("paths");
    let meta = tree2.get_meta(tree_id(id)).unwrap();
    match kept_before.unwrap() {
        loro::ValueOrContainer::Value(v) => {
            meta.insert("dimension_kept", v).unwrap();
        }
        loro::ValueOrContainer::Container(_) => panic!("dimension_kept is a plain value register"),
    }
    let opened = unpack(2, &file_from_loro(&l2, 31)).unwrap();
    let x = dim(&opened, id);
    assert!(x.linked);
    assert!(
        (mm(x.shown.value) - 100.0).abs() < 1e-9,
        "shown is resolved"
    );
    assert!((mm(x.kept.value) - 50.0).abs() < 1e-9, "kept is as stored");
    assert!(opened.own_steps().is_empty(), "opening writes nothing");
}

// ---------- criterion 11: linked on read ----------

#[test]
fn c11_linked_dimension_follows_node_moves_and_object_transforms() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    let x = dim(&d, id);
    assert!(x.linked);
    assert!((mm(x.shown.value) - 50.0).abs() < 1e-12);
    assert_eq!(x.shown.points, vec![pt(0.0, 0.0), pt(30.0, 40.0)]);

    let steps = d.own_steps().len();
    d.move_anchors(&[(p, AnchorId::new(1, 11), pt(60.0, 80.0))])
        .unwrap();
    assert_eq!(
        d.own_steps().len(),
        steps + 1,
        "no extra step for the dimension"
    );
    let x = dim(&d, id);
    assert!(x.linked);
    assert!((mm(x.shown.value) - 100.0).abs() < 1e-9);
    assert!(
        (mm(x.kept.value) - 100.0).abs() < 1e-9,
        "kept refreshed in the same commit"
    );

    d.translate_objects(&[p], Vec2::new(7.0, -3.0)).unwrap();
    let x = dim(&d, id);
    assert!((mm(x.shown.value) - 100.0).abs() < 1e-9);
    assert!((x.shown.points[0].x - 7.0).abs() < 1e-9 && (x.shown.points[0].y + 3.0).abs() < 1e-9);
}

#[test]
fn c11_a_uniform_scale_of_a_measured_circle_updates_the_radius() {
    let d = Document::new(1);
    let c = d.create_ellipse(EllipseFrame {
        center: pt(0.0, 0.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let id = d
        .add_dimension(
            DimensionKind::Radius,
            DimensionSlots::Curve(CurveSlot::Circle { node: c }),
            place(),
        )
        .unwrap();
    assert!((mm(dim(&d, id).shown.value) - 10.0).abs() < 1e-9);
    d.resize_ellipse(
        c,
        EllipseFrame {
            center: pt(0.0, 0.0),
            rx: Length::from_mm(12.5),
            ry: Length::from_mm(12.5),
        },
        None,
    )
    .unwrap();
    let x = dim(&d, id);
    assert!(x.linked);
    assert!((mm(x.shown.value) - 12.5).abs() < 1e-9);
    assert!((mm(x.kept.value) - 12.5).abs() < 1e-9);
}

#[test]
fn c11_rounded_corner_and_shape_points_resolve() {
    let d = Document::new(1);
    let r = d.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(20.0),
    });
    d.set_corner_radius(&[r], Length::from_mm(4.0)).unwrap();
    let rc = d
        .add_dimension(
            DimensionKind::Radius,
            DimensionSlots::Curve(CurveSlot::RoundCorner { node: r, corner: 0 }),
            place(),
        )
        .unwrap();
    assert!((mm(dim(&d, rc).shown.value) - 4.0).abs() < 1e-9);
    let w = d
        .add_dimension(
            DimensionKind::Horizontal,
            DimensionSlots::Points(vec![
                PointSlot::ShapePoint {
                    node: r,
                    feature: ShapeFeature::Corner(0),
                },
                PointSlot::ShapePoint {
                    node: r,
                    feature: ShapeFeature::Corner(2),
                },
            ]),
            place(),
        )
        .unwrap();
    assert!(
        (mm(dim(&d, w).shown.value) - 40.0).abs() < 1e-9,
        "horizontal only"
    );
    let v = d
        .add_dimension(
            DimensionKind::Vertical,
            DimensionSlots::Points(vec![
                PointSlot::ShapePoint {
                    node: r,
                    feature: ShapeFeature::Corner(0),
                },
                PointSlot::ShapePoint {
                    node: r,
                    feature: ShapeFeature::Corner(2),
                },
            ]),
            place(),
        )
        .unwrap();
    assert!(
        (mm(dim(&d, v).shown.value) - 20.0).abs() < 1e-9,
        "vertical only"
    );
}

#[test]
fn c11_angle_of_three_points_is_in_radians() {
    let d = Document::new(1);
    let id = d
        .add_dimension(
            DimensionKind::Angle,
            DimensionSlots::Points(vec![
                PointSlot::Free {
                    point: pt(0.0, 0.0),
                },
                PointSlot::Free {
                    point: pt(10.0, 0.0),
                },
                PointSlot::Free {
                    point: pt(10.0, 10.0),
                },
            ]),
            place(),
        )
        .unwrap();
    match dim(&d, id).shown.value {
        DimensionValue::Angle(a) => {
            assert!((a.as_radians().abs() - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
        }
        DimensionValue::Length(_) => panic!("an angle dimension measures an angle"),
    }
}

// ---------- criterion 12: unlink, last value, undo relinks ----------

fn assert_unlinked_at_fifty(d: &Document, id: NodeId, what: &str) {
    let x = dim(d, id);
    assert!(!x.linked, "{what}: the dimension is unlinked");
    assert!(
        (mm(x.shown.value) - 50.0).abs() < 1e-9,
        "{what}: keeps its last number, got {}",
        mm(x.shown.value)
    );
    assert_eq!(
        x.shown.points,
        vec![pt(0.0, 0.0), pt(30.0, 40.0)],
        "{what}: stays"
    );
}

#[test]
fn c12_delete_unlinks_in_the_same_step_and_one_undo_restores_object_link_and_dimension() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    let before = json(&d);
    let steps = d.own_steps().len();
    d.delete_objects(&[p]).unwrap();
    assert_eq!(d.own_steps().len(), steps + 1, "one step");
    assert_unlinked_at_fifty(&d, id, "delete");
    undo_last(&d);
    assert_eq!(json(&d), before);
    assert!(dim(&d, id).linked, "undo relinks");
    // redo unlinks again
    let step = d.own_steps().last().unwrap().step;
    let _ = step;
}

#[test]
fn c12_a_move_then_delete_keeps_the_last_value_not_the_creation_value() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    d.move_anchors(&[(p, AnchorId::new(1, 11), pt(0.0, 12.5))])
        .unwrap();
    d.delete_objects(&[p]).unwrap();
    let x = dim(&d, id);
    assert!(!x.linked);
    assert!((mm(x.shown.value) - 12.5).abs() < 1e-9);
}

#[test]
fn c12_node_delete_unlinks_and_undo_relinks() {
    let d = Document::new(1);
    let p = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 10), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 11), pt(30.0, 40.0)),
            NewAnchor::corner(AnchorId::new(1, 12), pt(60.0, 0.0)),
        ],
        false,
    );
    let id = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(p, 10), anchor(p, 11)]),
            place(),
        )
        .unwrap();
    d.delete_anchors(&[(p, AnchorId::new(1, 11))]);
    assert!(d.object(p).is_some(), "the path stays");
    assert_unlinked_at_fifty(&d, id, "node delete");
    undo_last(&d);
    assert!(dim(&d, id).linked);
}

#[test]
fn c12_boolean_replacement_unlinks() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    d.replace_with_pieces(&[p], &[], PieceFill::Keep, "boolean")
        .unwrap();
    assert_unlinked_at_fifty(&d, id, "boolean");
    undo_last(&d);
    assert!(dim(&d, id).linked);
}

#[test]
fn c12_join_that_removes_the_measured_path_unlinks() {
    let d = Document::new(1);
    let a = line(&d, 20, pt(-10.0, 0.0), pt(0.0, 0.0));
    let (p, id) = fifty(&d);
    let (joined, _) = d
        .join_endpoints(a, AnchorId::new(1, 21), p, AnchorId::new(1, 10))
        .unwrap();
    if d.object(p).is_none() || joined != p {
        // The measured path, or its first anchor, is gone.
        assert_unlinked_at_fifty(&d, id, "join");
    } else {
        let x = dim(&d, id);
        assert!((mm(x.shown.value) - 50.0).abs() < 1e-9);
    }
    undo_last(&d);
    assert!(dim(&d, id).linked);
}

#[test]
fn c12_convert_to_path_of_a_circle_and_non_uniform_scale_unlink() {
    let mk = |d: &Document| {
        let c = d.create_ellipse(EllipseFrame {
            center: pt(0.0, 0.0),
            rx: Length::from_mm(25.0),
            ry: Length::from_mm(25.0),
        });
        let id = d
            .add_dimension(
                DimensionKind::Radius,
                DimensionSlots::Curve(CurveSlot::Circle { node: c }),
                place(),
            )
            .unwrap();
        (c, id)
    };
    // Non-uniform scale.
    let d = Document::new(1);
    let (c, id) = mk(&d);
    d.resize_ellipse(
        c,
        EllipseFrame {
            center: pt(0.0, 0.0),
            rx: Length::from_mm(40.0),
            ry: Length::from_mm(25.0),
        },
        None,
    )
    .unwrap();
    let x = dim(&d, id);
    assert!(!x.linked, "a non-uniform scale unlinks");
    assert!((mm(x.shown.value) - 25.0).abs() < 1e-9);
    undo_last(&d);
    assert!(dim(&d, id).linked);

    // Convert to path of the shape.
    let d = Document::new(1);
    let (c, id) = mk(&d);
    let anchors: Vec<NewAnchor> = (0..8_u32)
        .map(|i| {
            let t = f64::from(i) * std::f64::consts::FRAC_PI_4;
            NewAnchor::corner(
                AnchorId::new(5, u64::from(i)),
                pt(25.0 * t.cos(), 25.0 * t.sin()),
            )
        })
        .collect();
    d.convert_to_paths(&[(c, anchors)]).unwrap();
    let x = dim(&d, id);
    assert!(!x.linked, "convert to path unlinks");
    assert!((mm(x.shown.value) - 25.0).abs() < 1e-9);
    undo_last(&d);
    assert!(dim(&d, id).linked);
}

#[test]
fn c12_clone_unlink_unlinks_dimensions_on_the_clone() {
    let d = Document::new(1);
    let p = line(&d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let clones = d.linked_clone(&[p], &[]).unwrap();
    let c = clones[0];
    let id = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(c, 10), anchor(c, 11)]),
            place(),
        )
        .unwrap();
    let x = dim(&d, id);
    assert!(
        x.linked,
        "a clone resolves by its own id and the root's anchor ids"
    );
    let shown = x.shown.clone();
    d.unlink_clones(&[CopySource {
        id: c,
        anchor_ids: vec![AnchorId::new(7, 1), AnchorId::new(7, 2)],
    }])
    .unwrap();
    let x = dim(&d, id);
    assert!(!x.linked, "clone unlink mints new anchor ids");
    assert_eq!(x.shown, shown);
    undo_last(&d);
    assert!(dim(&d, id).linked);
}

#[test]
fn c13_unlinked_does_not_follow_and_its_kept_points_move_without_changing_the_value() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    let q = line(&d, 30, pt(0.0, 0.0), pt(30.0, 40.0));
    d.delete_objects(&[p]).unwrap();
    d.translate_objects(&[q], Vec2::new(100.0, 0.0)).unwrap();
    assert_unlinked_at_fifty(&d, id, "other geometry moved");

    let n = d.own_steps().len();
    d.move_dimension_point(id, 1, pt(300.0, 400.0)).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert_eq!(labels(&d).last().unwrap(), "edit_dimension_nodes");
    let x = dim(&d, id);
    assert!(
        (mm(x.shown.value) - 50.0).abs() < 1e-9,
        "value never changes"
    );
    assert_eq!(x.shown.points[1], pt(300.0, 400.0));
    // A non-finite point is refused and writes nothing.
    let n = d.own_steps().len();
    assert!(d.move_dimension_point(id, 0, pt(f64::NAN, 0.0)).is_err());
    assert!(d.move_dimension_point(id, 5, pt(0.0, 0.0)).is_err());
    assert_eq!(d.own_steps().len(), n);
}

#[test]
fn c13_a_linked_dimension_refuses_a_kept_point_move() {
    let d = Document::new(1);
    let (_, id) = fifty(&d);
    let n = d.own_steps().len();
    assert!(d.move_dimension_point(id, 0, pt(1.0, 1.0)).is_err());
    assert_eq!(d.own_steps().len(), n);
}

// ---------- criterion 4 / 8: create and move ----------

#[test]
fn c4_create_is_one_step_and_undo_removes_it() {
    let d = Document::new(1);
    let p = line(&d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let before = json(&d);
    let n = d.own_steps().len();
    let id = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(p, 10), anchor(p, 11)]),
            place(),
        )
        .unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert_eq!(labels(&d).last().unwrap(), "create_dimension");
    undo_last(&d);
    assert!(d.dimension(id).is_none());
    assert_eq!(json(&d), before);
}

#[test]
fn c4_bad_creations_write_nothing() {
    let d = Document::new(1);
    let p = line(&d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let n = d.own_steps().len();
    let two = DimensionSlots::Points(vec![anchor(p, 10), anchor(p, 11)]);
    assert!(matches!(
        d.add_dimension(DimensionKind::Angle, two.clone(), place()),
        Err(ObjectEditError::InvalidConversion)
    ));
    assert!(
        d.add_dimension(DimensionKind::Radius, two.clone(), place())
            .is_err()
    );
    let mut bad = place();
    bad.offset = Length::from_mm(f64::INFINITY);
    assert!(
        d.add_dimension(DimensionKind::Aligned, two.clone(), bad)
            .is_err()
    );
    let mut bad = place();
    bad.sector = 4;
    assert!(d.add_dimension(DimensionKind::Aligned, two, bad).is_err());
    // A slot naming a missing anchor does not resolve.
    assert!(
        d.add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(p, 10), anchor(p, 99)]),
            place()
        )
        .is_err()
    );
    // A circle slot on a path.
    assert!(
        d.add_dimension(
            DimensionKind::Radius,
            DimensionSlots::Curve(CurveSlot::Circle { node: p }),
            place()
        )
        .is_err()
    );
    // A free-point dimension with non-finite points.
    assert!(
        d.add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Free {
                    point: pt(f64::NAN, 0.0)
                },
                PointSlot::Free {
                    point: pt(1.0, 0.0)
                },
            ]),
            place()
        )
        .is_err(),
        "non-finite free point is refused"
    );
    assert_eq!(d.own_steps().len(), n, "nothing written");
}

#[test]
fn c8_move_dimension_is_one_step_and_moving_with_shapes_writes_nothing_on_it() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    let n = d.own_steps().len();
    let mut new = place();
    new.along = 0.8;
    new.offset = Length::from_mm(-12.0);
    d.set_dimension_place(id, new).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert_eq!(labels(&d).last().unwrap(), "move_dimension");
    assert_eq!(dim(&d, id).place, new);
    // Unchanged place writes nothing.
    d.set_dimension_place(id, new).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    // Non-finite refused.
    let mut bad = new;
    bad.along = f64::NAN;
    assert!(d.set_dimension_place(id, bad).is_err());
    assert!(d.set_dimension_place(p, new).is_err(), "not a dimension");
    // Moving the shape and the dimension together keeps its place.
    d.translate_objects(&[p, id], Vec2::new(10.0, 10.0))
        .unwrap();
    let x = dim(&d, id);
    assert_eq!(x.place, new);
    assert!(x.linked);
    assert!((mm(x.shown.value) - 50.0).abs() < 1e-9);
}

// ---------- copy remap (ADR 0027 §5) ----------

#[test]
fn copy_with_its_path_remaps_to_the_copy_and_alone_becomes_free_points() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    let copies = d
        .duplicate_objects(
            &[
                CopySource {
                    id: p,
                    anchor_ids: vec![AnchorId::new(9, 1), AnchorId::new(9, 2)],
                },
                CopySource {
                    id,
                    anchor_ids: vec![],
                },
            ],
            Vec2::new(100.0, 0.0),
        )
        .unwrap();
    assert_eq!(copies.len(), 2);
    let (pc, dc) = (copies[0], copies[1]);
    let x = dim(&d, dc);
    assert!(x.linked);
    assert_eq!(x.slots.nodes(), vec![pc], "remapped to the copied path");
    match &x.slots {
        DimensionSlots::Points(s) => {
            assert_eq!(
                s[0],
                PointSlot::Anchor {
                    node: pc,
                    anchor: AnchorId::new(9, 1)
                }
            );
        }
        DimensionSlots::Curve(_) => panic!(),
    }
    assert!((x.shown.points[0].x - 100.0).abs() < 1e-9);
    // Moving the copy moves the copied dimension, not the original.
    d.move_anchors(&[(pc, AnchorId::new(9, 2), pt(100.0, 10.0))])
        .unwrap();
    assert!((mm(dim(&d, dc).shown.value) - 10.0).abs() < 1e-9);
    assert!((mm(dim(&d, id).shown.value) - 50.0).abs() < 1e-9);

    // The dimension alone: free points at the resolved position, shifted with the copy.
    let alone = d
        .duplicate_objects(
            &[CopySource {
                id,
                anchor_ids: vec![],
            }],
            Vec2::new(0.0, 5.0),
        )
        .unwrap()[0];
    let y = dim(&d, alone);
    assert!(
        y.slots.nodes().is_empty(),
        "no references outside the copied set"
    );
    assert!((mm(y.shown.value) - 50.0).abs() < 1e-9);
    d.translate_objects(&[p], Vec2::new(1.0, 1.0)).unwrap();
    assert!((mm(dim(&d, alone).shown.value) - 50.0).abs() < 1e-9);
    assert!(
        dim(&d, alone).shown.points[0] != dim(&d, id).shown.points[0],
        "the free copy does not follow the original path"
    );
}

// ---------- criterion 9 / 16: output filter, guide by kind ----------

#[test]
fn c9_a_dimension_is_never_an_output_node_and_is_a_guide() {
    let d = Document::new(1);
    let (p, id) = fifty(&d);
    let nodes = d.read_nodes();
    let n = nodes.iter().find(|n| n.id == id).unwrap();
    assert!(!n.is_output_node());
    assert!(n.guide, "every guide filter treats it as one");
    assert!(nodes.iter().find(|n| n.id == p).unwrap().is_output_node());
    let outputs: Vec<_> = nodes
        .iter()
        .filter(|n| n.is_output_node())
        .map(|n| n.id)
        .collect();
    assert_eq!(outputs, vec![p]);
    // No guide register is written on the dimension (ADR 0027 §1).
    let l = loro_of(&d);
    let meta = l.get_tree("paths").get_meta(tree_id(id)).unwrap();
    assert!(meta.get("guide").is_none());
}

#[test]
fn c9_make_clone_skips_a_dimension() {
    let d = Document::new(1);
    let (_, id) = fifty(&d);
    let r = d.linked_clone(&[id], &[]);
    assert!(r.map_or(true, |v| v.is_empty()), "never a clone source");
}

// ---------- criterion 10: convert ----------

#[test]
fn c10_convert_replaces_the_dimension_with_plain_paths_in_one_step_and_back_to_guides() {
    use curvyo_document_core::{ConvertedPath, DimensionConversion};
    let d = Document::new(1);
    let (_, id) = fifty(&d);
    let style = match d.object(line(&d, 40, pt(0.0, 0.0), pt(1.0, 1.0))).unwrap() {
        ObjectSnapshot::Path(p) => p.style,
        _ => panic!(),
    };
    let n = d.own_steps().len();
    let groups = d
        .convert_dimensions(&[DimensionConversion {
            id,
            paths: vec![
                ConvertedPath {
                    anchors: vec![
                        NewAnchor::corner(AnchorId::new(8, 1), pt(0.0, 0.0)),
                        NewAnchor::corner(AnchorId::new(8, 2), pt(30.0, 40.0)),
                    ],
                    closed: false,
                    style: style.clone(),
                },
                ConvertedPath {
                    anchors: vec![
                        NewAnchor::corner(AnchorId::new(8, 3), pt(0.0, 0.0)),
                        NewAnchor::corner(AnchorId::new(8, 4), pt(1.0, 2.0)),
                        NewAnchor::corner(AnchorId::new(8, 5), pt(2.0, 0.0)),
                    ],
                    closed: true,
                    style,
                },
            ],
            text: None,
        }])
        .unwrap();
    assert_eq!(d.own_steps().len(), n + 1, "one step");
    let convert_step = d.own_steps().last().unwrap().step;
    assert_eq!(labels(&d).last().unwrap(), "convert_dimension");
    assert!(d.dimension(id).is_none(), "the dimension is gone");
    assert_eq!(groups.len(), 1);
    let nodes = d.read_nodes();
    let leaves: Vec<_> = nodes
        .iter()
        .filter(|n| n.parent == Some(groups[0]))
        .collect();
    assert_eq!(leaves.len(), 2, "paths in the group");
    assert!(
        leaves.iter().all(|n| !n.guide && n.is_output_node()),
        "plain, ordinary objects"
    );
    // Guide again: plain guides, not a dimension.
    let ids: Vec<NodeId> = leaves.iter().map(|n| n.id).collect();
    d.set_guide(&ids, true).unwrap();
    let guide_step = d.own_steps().last().unwrap().step;
    assert!(dimensions(&d).is_empty(), "the link is not rebuilt");
    // Undo twice restores the dimension.
    d.undo_step(guide_step).unwrap();
    d.undo_step(convert_step).unwrap();
    assert!(dim(&d, id).linked);
}

#[test]
fn c10_convert_of_a_non_dimension_writes_nothing() {
    use curvyo_document_core::DimensionConversion;
    let d = Document::new(1);
    let (p, _) = fifty(&d);
    let n = d.own_steps().len();
    assert!(
        d.convert_dimensions(&[DimensionConversion {
            id: p,
            paths: vec![],
            text: None
        }])
        .is_err()
    );
    assert_eq!(d.own_steps().len(), n);
}

// ---------- criterion 14: units ----------

#[test]
fn c14_true_value_is_stored_unrounded_in_mm() {
    let d = Document::new(1);
    let p = line(&d, 10, pt(0.0, 0.0), pt(1.0 / 3.0, 0.0));
    let id = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(p, 10), anchor(p, 11)]),
            place(),
        )
        .unwrap();
    let back = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(
        mm(dim(&back, id).kept.value),
        1.0 / 3.0,
        "not rounded in storage"
    );
}

// ---------- criterion 15: hidden / locked layer ----------

#[test]
fn c15_dimension_follows_its_layer_state_like_any_guide() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    d.set_draw_parent(Some(layer));
    let (_, id) = fifty(&d);
    assert_eq!(
        d.read_nodes().iter().find(|n| n.id == id).unwrap().parent,
        Some(layer)
    );
    d.set_node_visible(&[layer], false).unwrap();
    let n = d.read_nodes().into_iter().find(|n| n.id == id).unwrap();
    assert!(!n.visible);
    d.set_node_visible(&[layer], true).unwrap();
    d.set_node_locked(&[layer], true).unwrap();
    let n = d.read_nodes().into_iter().find(|n| n.id == id).unwrap();
    assert!(n.locked);
}

// ---------- white-box-ish edges ----------

#[test]
fn edge_zero_length_and_huge_values_stay_finite() {
    let d = Document::new(1);
    let p = line(&d, 10, pt(5.0, 5.0), pt(5.0, 5.0));
    if let Ok(id) = d.add_dimension(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![anchor(p, 10), anchor(p, 11)]),
        place(),
    ) {
        assert_eq!(mm(dim(&d, id).shown.value), 0.0);
    }
    let q = line(&d, 20, pt(-1e6, -1e6), pt(1e6, 1e6));
    let id = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(q, 20), anchor(q, 21)]),
            place(),
        )
        .unwrap();
    assert!(dim(&d, id).shown.is_finite());
    // Angle with coincident points: no NaN in the snapshot.
    let a = d.add_dimension(
        DimensionKind::Angle,
        DimensionSlots::Points(vec![
            PointSlot::Free {
                point: pt(0.0, 0.0),
            },
            PointSlot::Free {
                point: pt(0.0, 0.0),
            },
            PointSlot::Free {
                point: pt(1.0, 0.0),
            },
        ]),
        place(),
    );
    if let Ok(a) = a {
        assert!(dim(&d, a).shown.is_finite(), "degenerate angle is finite");
    }
}
