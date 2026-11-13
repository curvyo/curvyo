//! `specs/0114-image-trace-mask`, the document side (ADR 0061): the mask stored on the image,
//! one History step per gesture with its label, undo and redo, delete and undo, Duplicate, the
//! asset swap, and two peers merging (criteria 12, 14, 16, 17, 18, 20).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::sync::Arc;

use curvyo_document_core::{
    Affine, CopySource, Document, ImplicitTracePart, NewImage, NewTraceRegion, NodeId, TraceMask,
    TraceMaskError, TracePartRef, TracePartSettings, TraceRegionEdit, TraceRegionKind, UnitPoint,
    Vec2, pack, unpack,
};
use serde_json::json;

fn image(d: &Document, bytes: &'static [u8]) -> NodeId {
    d.add_images(&[NewImage {
        bytes: Arc::from(bytes),
        placement: Affine::new(100.0, 0.0, 0.0, 100.0, 0.0, 0.0),
        name: "M1".into(),
    }])
    .unwrap()[0]
}

fn rect(subtract: bool, a: (f64, f64), b: (f64, f64)) -> NewTraceRegion {
    NewTraceRegion {
        kind: TraceRegionKind::Rect,
        subtract,
        points: vec![UnitPoint::new(a.0, a.1), UnitPoint::new(b.0, b.1)],
    }
}

fn settings(value: serde_json::Value) -> TracePartSettings {
    serde_json::from_value(value).unwrap()
}

fn implicit() -> ImplicitTracePart {
    ImplicitTracePart {
        outside: false,
        settings: settings(json!({"threshold": 50, "smooth": 80})),
    }
}

fn mask(d: &Document, id: NodeId) -> TraceMask {
    d.trace_mask(id).unwrap()
}

fn labels(d: &Document) -> Vec<String> {
    d.own_steps().iter().map(|s| s.label.clone()).collect()
}

fn undo(d: &Document) {
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
}

fn json_of(d: &Document) -> serde_json::Value {
    serde_json::from_slice(&d.export_json().unwrap()).unwrap()
}

/// R1 of the spec on M1 (100 x 100 mm): x 0 to 50 mm, the full height.
fn r1() -> NewTraceRegion {
    rect(false, (0.0, 0.0), (0.5, 1.0))
}

#[test]
fn c22_an_untouched_image_stores_nothing() {
    let d = Document::new(1);
    let m1 = image(&d, b"m1");
    assert!(mask(&d, m1).is_empty());
    assert!(json_of(&d)["objects"][0].get("trace_mask").is_none());
    let path = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        curvyo_document_core::Point::new(0.0, 0.0),
        curvyo_document_core::Point::new(1.0, 1.0),
    ));
    assert_eq!(d.trace_mask(path), None, "not an image");
    assert_eq!(
        d.draw_trace_region(path, TracePartRef::Implicit(&implicit()), &r1()),
        Err(TraceMaskError::NotAnImage)
    );
}

/// Criterion 17 and ADR 0061 §3: the first gesture stores part `0` with the tool's mode and
/// settings in its own one step; every command is one step with its label, undo and redo work.
#[test]
fn c17_each_mask_command_is_one_step_with_its_label() {
    let d = Document::new(1);
    let m1 = image(&d, b"m1");
    let before = d.own_step_count();
    let r = d
        .draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    let stored = mask(&d, m1);
    assert_eq!(stored.parts.len(), 1);
    assert_eq!(stored.parts[0].id, "0");
    assert_eq!(stored.parts[0].settings, implicit().settings);
    assert_eq!(stored.parts[0].regions[0].id, r);
    let moved = [UnitPoint::new(0.1, 0.0), UnitPoint::new(0.6, 1.0)];
    assert!(
        d.set_trace_region_points(m1, &r, &moved, TraceRegionEdit::Move)
            .unwrap()
    );
    assert!(
        !d.set_trace_region_points(m1, &r, &moved, TraceRegionEdit::Move)
            .unwrap(),
        "unchanged points write no step"
    );
    let resized = [UnitPoint::new(0.1, 0.0), UnitPoint::new(0.7, 1.0)];
    d.set_trace_region_points(m1, &r, &resized, TraceRegionEdit::Resize)
        .unwrap();
    let cut = d
        .draw_trace_region(
            m1,
            TracePartRef::Stored("0"),
            &rect(true, (0.2, 0.2), (0.3, 0.3)),
        )
        .unwrap();
    d.delete_trace_region(m1, &cut).unwrap();
    assert!(d.set_trace_part_outside(m1, "0", true).unwrap());
    assert!(!d.set_trace_part_outside(m1, "0", true).unwrap());
    d.set_trace_part_outside(m1, "0", false).unwrap();
    let two = d
        .add_trace_part(m1, &implicit(), &settings(json!({"smooth": 10})))
        .unwrap();
    assert!(
        d.set_trace_part_settings(m1, &two, &settings(json!({"smooth": 20})))
            .unwrap()
    );
    assert!(d.clear_trace_mask(m1, "0").unwrap());
    d.remove_trace_part(m1, &two).unwrap();
    assert_eq!(d.remove_trace_part(m1, "0"), Err(TraceMaskError::LastPart));
    assert_eq!(
        labels(&d)[before..],
        [
            "draw_trace_rectangle",
            "move_trace_region",
            "resize_trace_region",
            "draw_trace_rectangle",
            "delete_trace_region",
            "trace_mask_outside",
            "trace_mask_inside",
            "add_trace_part",
            "edit_trace_settings",
            "clear_trace_mask",
            "remove_trace_part",
        ]
    );
    // Undo walks back one step at a time; redo reapplies.
    undo(&d);
    assert_eq!(mask(&d, m1).parts.len(), 2, "Remove part undone");
    let steps = d.own_steps();
    let clear = steps[steps.len() - 2].step;
    d.undo_step(clear).unwrap();
    assert_eq!(mask(&d, m1).parts[0].regions.len(), 1, "Clear mask undone");
    assert_eq!(mask(&d, m1).parts[0].regions[0].points, resized);
    d.redo_step(clear).unwrap();
    assert_eq!(mask(&d, m1).parts[0].regions.len(), 0);
}

/// Criterion 20: Clear mask removes the active part's regions in one step; the part and its
/// settings stay; a part without regions writes no step.
#[test]
fn c20_clear_mask_keeps_the_part_and_its_settings() {
    let d = Document::new(1);
    let m1 = image(&d, b"m1");
    d.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    d.draw_trace_region(
        m1,
        TracePartRef::Stored("0"),
        &rect(true, (0.1, 0.1), (0.2, 0.2)),
    )
    .unwrap();
    let steps = d.own_step_count();
    assert!(d.clear_trace_mask(m1, "0").unwrap());
    assert_eq!(d.own_step_count(), steps + 1);
    let stored = mask(&d, m1);
    assert_eq!(stored.parts.len(), 1);
    assert_eq!(stored.parts[0].regions.len(), 0);
    assert_eq!(stored.parts[0].settings, implicit().settings);
    assert!(!d.clear_trace_mask(m1, "0").unwrap());
    assert_eq!(d.own_step_count(), steps + 1);
}

/// Criterion 16: deleting the image deletes its mask; one undo brings both back exactly.
#[test]
fn c16_delete_and_undo_restore_the_image_with_its_mask() {
    let d = Document::new(1);
    let m1 = image(&d, b"m1");
    d.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    d.add_trace_part(m1, &implicit(), &settings(json!({"smooth": 3})))
        .unwrap();
    let before = mask(&d, m1);
    let json_before = json_of(&d);
    d.delete_objects(&[m1]).unwrap();
    assert_eq!(d.trace_mask(m1), None);
    let saved = json_of(&unpack(2, &pack(&d, "t").unwrap()).unwrap());
    assert!(
        !saved.to_string().contains("trace_mask"),
        "the reopened file's document.json holds no mask"
    );
    undo(&d);
    assert_eq!(mask(&d, m1), before);
    assert_eq!(json_of(&d), json_before);
}

/// Criterion 14 (Duplicate) and ADR 0061 §8: the copy has the same parts and regions with the
/// same ids; editing the copy's mask changes only the copy.
#[test]
fn c14_a_duplicate_carries_the_mask_and_edits_stay_on_it() {
    let d = Document::new(1);
    let m1 = image(&d, b"m1");
    d.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: m1,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(10.0, 0.0),
        )
        .unwrap()[0];
    assert_eq!(mask(&d, copy), mask(&d, m1));
    let original = mask(&d, m1);
    let id = &mask(&d, copy).parts[0].regions[0].id.clone();
    d.delete_trace_region(copy, id).unwrap();
    assert_eq!(mask(&d, copy).parts[0].regions.len(), 0);
    assert_eq!(mask(&d, m1), original);
}

/// Criterion 12 (the document half): a save and reopen gives the same parts, modes, regions and
/// settings, and moving the image keeps the regions in its unit square (criterion 13's premise).
#[test]
fn c12_a_reopened_file_has_the_same_mask() {
    let d = Document::new(1);
    let m1 = image(&d, b"m1");
    d.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    let lasso = NewTraceRegion {
        kind: TraceRegionKind::Lasso,
        subtract: false,
        points: vec![
            UnitPoint::new(0.0, 0.0),
            UnitPoint::new(0.6, 0.0),
            UnitPoint::new(0.0, 1.0),
        ],
    };
    d.draw_trace_region(m1, TracePartRef::Stored("0"), &lasso)
        .unwrap();
    let two = d
        .add_trace_part(
            m1,
            &implicit(),
            &settings(json!({"method": "pick_colours"})),
        )
        .unwrap();
    d.draw_trace_region(
        m1,
        TracePartRef::Stored(&two),
        &rect(false, (0.6, 0.6), (0.9, 0.9)),
    )
    .unwrap();
    d.set_trace_part_outside(m1, &two, true).unwrap();
    d.translate_objects(&[m1], Vec2::new(5.0, 5.0)).unwrap();
    let reopened = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(mask(&reopened, m1), mask(&d, m1));
    let stored = mask(&reopened, m1);
    assert_eq!(stored.parts.len(), 2);
    assert!(stored.parts[1].outside);
    assert_eq!(stored.parts[0].regions.len(), 2);
    assert_eq!(
        stored.parts[0].regions[0].points[1],
        UnitPoint::new(0.5, 1.0)
    );
}

/// Two peers forked from one file.
fn peers() -> (Document, Document, NodeId) {
    let a = Document::new(1);
    let m1 = image(&a, b"m1");
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    (a, b, m1)
}

fn sync(a: &Document, b: &Document) {
    let (av, bv) = (a.version(), b.version());
    let to_b = a.export_updates(&bv).unwrap();
    let to_a = b.export_updates(&av).unwrap();
    a.import_updates(&to_a).unwrap();
    b.import_updates(&to_b).unwrap();
}

fn same_bytes(a: &Document, b: &Document) {
    assert_eq!(a.export_json().unwrap(), b.export_json().unwrap());
}

/// Criterion 18 (target): two peers each draw a region offline on an untouched image; after the
/// sync both regions are in the one part `0` on both, byte-identical.
#[test]
fn c18_two_peers_keep_both_regions_in_part_zero() {
    let (a, b, m1) = peers();
    a.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    b.draw_trace_region(
        m1,
        TracePartRef::Implicit(&implicit()),
        &rect(false, (0.6, 0.0), (1.0, 0.4)),
    )
    .unwrap();
    sync(&a, &b);
    same_bytes(&a, &b);
    let stored = mask(&a, m1);
    assert_eq!(stored.parts.len(), 1);
    assert_eq!(stored.parts[0].id, "0");
    assert_eq!(stored.parts[0].regions.len(), 2);
    assert_eq!(mask(&b, m1), stored);
}

/// ADR 0061 §4: concurrent edits of one region keep one peer's whole shape on both.
#[test]
fn c18_concurrent_edits_of_one_region_converge_to_one_shape() {
    let (a, b, m1) = peers();
    let r = a
        .draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    sync(&a, &b);
    let moved_a = [UnitPoint::new(0.1, 0.1), UnitPoint::new(0.4, 0.4)];
    let moved_b = [UnitPoint::new(0.5, 0.5), UnitPoint::new(0.9, 0.9)];
    a.set_trace_region_points(m1, &r, &moved_a, TraceRegionEdit::Move)
        .unwrap();
    b.set_trace_region_points(m1, &r, &moved_b, TraceRegionEdit::Resize)
        .unwrap();
    sync(&a, &b);
    same_bytes(&a, &b);
    let points = mask(&a, m1).parts[0].regions[0].points.clone();
    assert!(points == moved_a || points == moved_b, "{points:?}");
}

/// ADR 0061 §4: Remove part on one peer against a draw into that part on the other: the part
/// stays removed, the drawn region is dangling (not read) and the next Clear mask sweeps it.
#[test]
fn c18_remove_part_wins_against_a_concurrent_draw() {
    let (a, b, m1) = peers();
    a.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    let two = a
        .add_trace_part(m1, &implicit(), &TracePartSettings::new())
        .unwrap();
    sync(&a, &b);
    a.remove_trace_part(m1, &two).unwrap();
    b.draw_trace_region(
        m1,
        TracePartRef::Stored(&two),
        &rect(false, (0.0, 0.0), (0.2, 0.2)),
    )
    .unwrap();
    sync(&a, &b);
    same_bytes(&a, &b);
    let stored = mask(&a, m1);
    assert_eq!(stored.parts.len(), 1);
    assert_eq!(mask(&b, m1), stored);
    assert!(
        a.export_json()
            .unwrap()
            .windows(two.len())
            .all(|w| w != two.as_bytes())
    );
    assert!(a.clear_trace_mask(m1, "0").unwrap());
    sync(&a, &b);
    same_bytes(&a, &b);
}

/// Criterion 18: concurrent Add part on both peers gives two parts in the same order on both.
#[test]
fn c18_concurrent_add_part_gives_the_same_order_on_both() {
    let (a, b, m1) = peers();
    a.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &r1())
        .unwrap();
    sync(&a, &b);
    a.add_trace_part(m1, &implicit(), &TracePartSettings::new())
        .unwrap();
    b.add_trace_part(m1, &implicit(), &TracePartSettings::new())
        .unwrap();
    sync(&a, &b);
    same_bytes(&a, &b);
    assert_eq!(mask(&a, m1).parts.len(), 3);
    assert_eq!(mask(&a, m1), mask(&b, m1));
}

/// Invalid points never reach the file.
#[test]
fn points_that_do_not_fit_their_kind_are_refused() {
    let d = Document::new(1);
    let m1 = image(&d, b"m1");
    let steps = d.own_step_count();
    for bad in [
        rect(false, (0.0, 0.0), (f64::NAN, 1.0)),
        rect(false, (0.0, 0.0), (1000.5, 1.0)),
        NewTraceRegion {
            kind: TraceRegionKind::Lasso,
            subtract: false,
            points: vec![UnitPoint::new(0.0, 0.0), UnitPoint::new(1.0, 1.0)],
        },
    ] {
        assert_eq!(
            d.draw_trace_region(m1, TracePartRef::Implicit(&implicit()), &bad),
            Err(TraceMaskError::InvalidPoints)
        );
    }
    assert_eq!(d.own_step_count(), steps);
    assert!(mask(&d, m1).is_empty());
}

/// Criterion 14, Ctrl+C / Ctrl+V (ADR 0061 §8): a copied image's mask goes through its JSON (the
/// page's memo) and is written whole onto the pasted image inside the paste's own step; the copy's
/// mask is its own afterwards.
#[test]
fn c14_a_pasted_image_gets_the_copied_mask_in_the_paste_step() {
    let d = Document::new(1);
    let a = image(&d, b"pixels-a");
    d.draw_trace_region(
        a,
        TracePartRef::Implicit(&implicit()),
        &rect(false, (0.0, 0.0), (0.5, 1.0)),
    )
    .unwrap();
    d.add_trace_part(a, &implicit(), &settings(json!({"smooth": 20})))
        .unwrap();
    let lasso = NewTraceRegion {
        kind: TraceRegionKind::Lasso,
        subtract: true,
        points: vec![
            UnitPoint::new(0.1, 0.1),
            UnitPoint::new(0.6, 0.1),
            UnitPoint::new(0.1, 0.9),
        ],
    };
    let part = d.trace_mask(a).unwrap().parts[1].id.clone();
    d.draw_trace_region(a, TracePartRef::Stored(&part), &lasso)
        .unwrap();
    let copied = d.trace_mask(a).unwrap();
    let json = serde_json::to_value(&copied).unwrap();
    let back = TraceMask::from_json(&json).expect("the memo reads back");
    assert_eq!(back, copied);
    let steps = d.history_page(0, 1000).len();
    let b = d
        .batch("paste_image", |document| {
            let b = document
                .write_images(&[NewImage {
                    bytes: Arc::from(&b"pixels-b"[..]),
                    placement: Affine::new(50.0, 0.0, 0.0, 50.0, 200.0, 0.0),
                    name: "pasted".into(),
                }])
                .into_iter()
                .next()?;
            document.write_trace_mask(b, &back).ok()?;
            Some(b)
        })
        .unwrap();
    assert_eq!(
        d.history_page(0, 1000).len(),
        steps + 1,
        "the paste is one step"
    );
    assert_eq!(
        d.trace_mask(b).unwrap(),
        copied,
        "the same parts, settings and regions"
    );
    // Editing the paste's mask leaves the copied image's alone.
    d.clear_trace_mask(b, "0").unwrap();
    assert_eq!(d.trace_mask(a).unwrap(), copied);
    // A damaged memo writes nothing.
    assert_eq!(TraceMask::from_json(&json!({"parts": [{"id": "x"}]})), None);
    d.delete_objects(&[b]).unwrap();
    assert_eq!(
        d.write_trace_mask(b, &copied),
        Err(TraceMaskError::NotAnImage)
    );
}
