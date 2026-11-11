//! Independent acceptance tests for `specs/0099-sketch-pages` at the document model and file
//! format (ADR 0043): the fixed sketch (criterion 4), the frame move and resize of a sketch page
//! (5, 6), clones following the sketch (7), placing a clone (8), Convert to paths turning the
//! page into a drawing page (15), deleting a sketch page with clones (17) and the golden round
//! trip without a format bump (19).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, AnchorSnapshot, Angle, CONVERT_SKETCH_PAGE_COMMIT_LABEL,
    CURRENT_FORMAT_VERSION, DELETE_SKETCH_PAGE_COMMIT_LABEL, Document, DocumentSize, Length,
    NewOutputPage, NodeId, ObjectSnapshot, PLACE_SKETCH_CLONE_COMMIT_LABEL, Page, PageKind, Point,
    RectBounds, SketchDelta, Style, Vec2, pack, unpack,
};
use curvyo_sketch_core as sk;

const CURVE_TOL: f64 = 0.01;
const TOL: f64 = 1e-6;

// ---- helpers ------------------------------------------------------------------------------------

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn reopen(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "0099-tester").unwrap()).unwrap()
}

fn exchange(a: &Document, b: &Document) {
    let aa = a.export_updates(&b.version()).unwrap();
    let bb = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bb).unwrap();
    b.import_updates(&aa).unwrap();
}

fn page(d: &Document, id: &str) -> Page {
    d.pages().into_iter().find(|p| p.id == id).unwrap()
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + w, y + h),
    ))
}

/// A sketch of one driven line along x from the origin: length `width - 2 * margin`, with
/// `width` = 80 mm and `margin` = 10 mm (60 mm), the `0066` document fixture.
fn driven_line() -> sk::Sketch {
    let mut s = sk::Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        sk::Entity::Point {
            position: sk::Position::mm(60.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        sk::Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    for (id, value) in [("width", 80.0), ("margin", 10.0)] {
        s.parameters.insert(
            id.into(),
            sk::Parameter::literal(id, sk::Quantity::length(value).unwrap()),
        );
    }
    let formula = sk::Expression::parse("width - 2 * margin", &s.parameters).unwrap();
    s.parameters.insert(
        "spacing".into(),
        sk::Parameter::expression("spacing", sk::Dimension::Length, formula),
    );
    s.constraints.insert(
        "horizontal".into(),
        sk::Constraint::driving(sk::Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        sk::Constraint::driving(sk::Relation::Length {
            curve: "line".into(),
            value: sk::Expression::Reference("spacing".into()),
        }),
    );
    s
}

fn set_width(value: f64) -> SketchDelta {
    let mut delta = SketchDelta::default();
    delta.parameters.insert(
        "width".into(),
        Some(sk::Parameter::literal(
            "width",
            sk::Quantity::length(value).unwrap(),
        )),
    );
    delta
}

/// A closed 40 x 30 mm loop around the sketch origin.
fn closed_box() -> sk::Sketch {
    sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "edge",
        &[
            sk::Position::mm(-20.0, -15.0),
            sk::Position::mm(20.0, -15.0),
            sk::Position::mm(20.0, 15.0),
            sk::Position::mm(-20.0, 15.0),
        ],
        true,
    )
    .unwrap()
}

fn wide_style() -> Style {
    let mut s = Style::default();
    s.stroke.width = Length::from_mm(0.7);
    s
}

/// A drawing page at (0, 0) ("first"), a sketch page right of it with the closed box.
fn with_sketch(sketch: &sk::Sketch) -> (Document, Page, NodeId) {
    let d = Document::new(1);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let id = d.create_page_sketch(&sp.id, sketch, &wide_style()).unwrap();
    let p = page(&d, &sp.id);
    (d, p, id)
}

fn outline(o: &ObjectSnapshot) -> Vec<AnchorSnapshot> {
    match o {
        ObjectSnapshot::Clone(c) => c.resolved.all_anchors().copied().collect(),
        ObjectSnapshot::Sketch(s) => s.resolved.all_anchors().copied().collect(),
        ObjectSnapshot::Path(p) => p.all_anchors().copied().collect(),
        other => panic!("no outline for {other:?}"),
    }
}

fn outline_of(d: &Document, id: NodeId) -> Vec<AnchorSnapshot> {
    outline(&d.object(id).unwrap())
}

fn same_outline(a: &[AnchorSnapshot], b: &[AnchorSnapshot], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            (p.point.x - q.point.x).abs() < tol
                && (p.point.y - q.point.y).abs() < tol
                && (p.handle_in.x - q.handle_in.x).abs() < tol
                && (p.handle_in.y - q.handle_in.y).abs() < tol
                && (p.handle_out.x - q.handle_out.x).abs() < tol
                && (p.handle_out.y - q.handle_out.y).abs() < tol
        })
}

fn shifted(a: &[AnchorSnapshot], by: Vec2) -> Vec<AnchorSnapshot> {
    a.iter()
        .copied()
        .map(|mut x| {
            x.point = Point::new(x.point.x + by.x, x.point.y + by.y);
            x
        })
        .collect()
}

fn fresh_ids(d: &Document, sketch: NodeId, seed: u64) -> Vec<AnchorId> {
    let n = d.sketch(sketch).unwrap().resolved.all_anchors().count();
    (0..n as u64).map(|i| AnchorId::new(seed, i)).collect()
}

fn clone_of(d: &Document, original: NodeId) -> NodeId {
    d.linked_clone(&[original], &[]).unwrap()[0]
}

fn as_clone(d: &Document, id: NodeId) -> curvyo_document_core::CloneSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(c)) => c,
        other => panic!("{id:?} is no clone: {other:?}"),
    }
}

/// Puts `clone` under `extra · M`, the way a rotate or scale of the clone writes its matrix.
fn transform_clone(d: &Document, clone: NodeId, extra: Affine) {
    let mut c = as_clone(d, clone);
    c.chain.matrix = c.chain.matrix.compose(extra);
    d.transform_objects(&[ObjectSnapshot::Clone(c)], false)
        .unwrap();
}

fn order_of(d: &Document, id: &str) -> usize {
    d.pages().iter().position(|p| p.id == id).unwrap()
}

// ---- criterion 4: the fixed sketch -------------------------------------------------------------

#[test]
fn ac4_only_the_live_owner_of_a_live_sketch_page_is_fixed() {
    let (d, sp, owner) = with_sketch(&closed_box());
    assert!(d.is_fixed_sketch(owner));
    let loose = d
        .create_sketch(&closed_box(), Affine::IDENTITY, &Style::default())
        .unwrap();
    assert!(!d.is_fixed_sketch(loose), "a sketch no page owns");
    let c = clone_of(&d, owner);
    assert!(!d.is_fixed_sketch(c), "a clone of the sketch");
    let r = rect(&d, 1.0, 1.0, 5.0, 5.0);
    assert!(!d.is_fixed_sketch(r));
    d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert!(!d.is_fixed_sketch(owner), "page and sketch deleted");
    undo_last(&d);
    assert!(d.is_fixed_sketch(owner), "undo restores the fixed sketch");
    let ids = fresh_ids(&d, owner, 900);
    d.convert_sketch_page(&sp.id, &ids).unwrap();
    assert!(
        !d.is_fixed_sketch(owner),
        "paths on a drawing page are free"
    );
}

#[test]
fn ac4_a_conflict_object_is_an_ordinary_object_and_is_not_fixed() {
    let a = Document::new(31);
    let sp = a.add_page_of(&PageKind::Sketch).unwrap();
    let b = reopen(&a, 32);
    let left = a
        .create_page_sketch(&sp.id, &closed_box(), &Style::default())
        .unwrap();
    let right = b
        .create_page_sketch(&sp.id, &driven_line(), &Style::default())
        .unwrap();
    exchange(&a, &b);
    let winner = a.page_sketch(&sp.id).unwrap();
    let loser = if winner == left { right } else { left };
    assert!(a.sketch_ownership_conflict(loser));
    assert!(a.is_fixed_sketch(winner));
    assert!(!a.is_fixed_sketch(loser));
}

// ---- criterion 5: the frame move carries the sketch only, clones held --------------------------

#[test]
fn ac5_the_frame_move_moves_the_sketch_and_holds_every_clone_and_foreign_object() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let plain = clone_of(&d, owner);
    d.translate_objects(&[plain], Vec2::new(0.0, 200.0))
        .unwrap();
    let own = clone_of(&d, owner);
    d.set_own_placement(&[own], true).unwrap();
    d.translate_objects(&[own], Vec2::new(0.0, -200.0)).unwrap();
    let over = rect(&d, sp.origin.x + 5.0, sp.origin.y + 5.0, 10.0, 10.0);
    let placement = d.sketch(owner).unwrap().placement;
    let before_plain = outline_of(&d, plain);
    let before_own = outline_of(&d, own);
    let before_over = d.object(over).unwrap();
    let before_sketch = outline_of(&d, owner);
    let steps = d.own_step_count();
    let delta = Vec2::new(37.5, -12.25);
    d.move_page(
        &sp.id,
        delta,
        &curvyo_document_core::PageContents::from([(sp.id.clone(), vec![owner])]),
    )
    .unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one undo step");
    assert_eq!(last_label(&d), "move_page");
    let moved = d.sketch(owner).unwrap().placement;
    assert!(
        moved.approx_eq(Affine::translation(delta).compose(placement), TOL)
            || moved.approx_eq(placement.compose(Affine::translation(delta)), TOL),
        "placement translated by the frame offset: {placement:?} -> {moved:?}"
    );
    assert!(same_outline(
        &outline_of(&d, owner),
        &shifted(&before_sketch, delta),
        TOL
    ));
    assert!(
        same_outline(&outline_of(&d, plain), &before_plain, TOL),
        "a clone without own placement is held back"
    );
    assert!(same_outline(&outline_of(&d, own), &before_own, TOL));
    assert_eq!(d.object(over).unwrap(), before_over, "foreign object stays");
    let f = page(&d, &sp.id);
    assert!((f.origin.x - sp.origin.x - delta.x).abs() < TOL);
    assert!((f.origin.y - sp.origin.y - delta.y).abs() < TOL);
    undo_last(&d);
    assert_eq!(d.sketch(owner).unwrap().placement, placement);
    assert!(same_outline(&outline_of(&d, plain), &before_plain, TOL));
}

#[test]
fn ac5_a_sketch_whose_outline_lies_outside_its_frame_still_moves_with_it() {
    let far = sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "far",
        &[sk::Position::mm(400.0, 0.0), sk::Position::mm(450.0, 30.0)],
        false,
    )
    .unwrap();
    let (d, sp, owner) = with_sketch(&far);
    let before = outline_of(&d, owner);
    assert!(
        before.iter().all(|a| !sp.contains(a.point)),
        "fixture: the outline lies wholly outside the frame"
    );
    let delta = Vec2::new(-10.0, 25.0);
    d.move_page(
        &sp.id,
        delta,
        &curvyo_document_core::PageContents::from([(sp.id.clone(), vec![owner])]),
    )
    .unwrap();
    assert!(same_outline(
        &outline_of(&d, owner),
        &shifted(&before, delta),
        TOL
    ));
}

// ---- criterion 6: resize keeps the sketch ----------------------------------------------------

#[test]
fn ac6_resizing_a_sketch_page_keeps_the_sketch_and_the_clones_in_place() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let c = clone_of(&d, owner);
    let placement = d.sketch(owner).unwrap().placement;
    let clone_before = outline_of(&d, c);
    for size in [
        DocumentSize::from_mm(300.0, 120.0),
        DocumentSize::from_mm(40.0, 500.0),
    ] {
        let steps = d.own_step_count();
        d.resize_page(&sp.id, size, &curvyo_document_core::PageContents::new())
            .unwrap();
        assert_eq!(d.own_step_count(), steps + 1, "one undo step");
        assert_eq!(page(&d, &sp.id).size, size);
        assert_eq!(
            d.sketch(owner).unwrap().placement,
            placement,
            "not re-centred"
        );
        assert!(same_outline(&outline_of(&d, c), &clone_before, TOL));
    }
}

// ---- criterion 7: clones follow the sketch ---------------------------------------------------

fn line_vector(anchors: &[AnchorSnapshot]) -> Vec2 {
    assert_eq!(anchors.len(), 2, "one line: two anchors");
    Vec2::new(
        anchors[1].point.x - anchors[0].point.x,
        anchors[1].point.y - anchors[0].point.y,
    )
}

#[test]
fn ac7_clones_on_a_drawing_and_a_laser_page_follow_a_driving_dimension_change() {
    let (d, _sp, owner) = with_sketch(&driven_line());
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let on_drawing = d.place_linked_clone(owner, Vec2::new(-150.0, 0.0));
    let on_drawing = on_drawing.unwrap();
    let first = page(&d, "first");
    let sketch_zero = d
        .sketch(owner)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    let to_laser = Vec2::new(
        laser.origin.x + 100.0 - sketch_zero.x,
        laser.origin.y + 100.0 - sketch_zero.y,
    );
    let on_laser = d.place_linked_clone(owner, to_laser).unwrap();
    let pivot = Point::new(laser.origin.x + 100.0, laser.origin.y + 100.0);
    let scale = Affine::translation(Vec2::new(-pivot.x, -pivot.y))
        .compose(Affine::scale(0.5, 0.5))
        .compose(Affine::translation(Vec2::new(pivot.x, pivot.y)));
    transform_clone(
        &d,
        on_laser,
        scale.compose(Affine::rotation_about(
            pivot,
            Angle::from_radians(12.0_f64.to_radians()),
        )),
    );
    let _ = first;
    let a0 = line_vector(&outline_of(&d, on_drawing));
    let b0 = line_vector(&outline_of(&d, on_laser));
    assert!((a0.x.hypot(a0.y) - 60.0).abs() < CURVE_TOL);
    assert!(
        (b0.x.hypot(b0.y) - 30.0).abs() < CURVE_TOL,
        "scaled by 50 %"
    );
    d.apply_sketch_delta(owner, &set_width(100.0)).unwrap();
    let a1 = line_vector(&outline_of(&d, on_drawing));
    let b1 = line_vector(&outline_of(&d, on_laser));
    // 20 mm times the scale, along the clone's (rotated) axis.
    for (before, after, scale) in [(a0, a1, 1.0), (b0, b1, 0.5)] {
        let len0 = before.x.hypot(before.y);
        let ux = before.x / len0;
        let uy = before.y / len0;
        let dx = after.x - before.x;
        let dy = after.y - before.y;
        assert!(
            (dx - 20.0 * scale * ux).abs() < CURVE_TOL
                && (dy - 20.0 * scale * uy).abs() < CURVE_TOL,
            "change ({dx}, {dy}) along ({ux}, {uy}) by {}",
            20.0 * scale
        );
    }
    // The laser clone's axis is rotated by 12 degrees against the drawing clone's.
    let angle = (b0.y.atan2(b0.x) - a0.y.atan2(a0.x)).to_degrees().abs();
    assert!((angle - 12.0).abs() < 1e-6, "{angle}");
}

// ---- criterion 8 / 11: place_linked_clone ---------------------------------------------------

#[test]
fn ac8_place_linked_clone_is_one_step_with_own_placement_and_the_translation() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let sketch_before = d.sketch(owner).unwrap();
    let steps = d.own_step_count();
    let by = Vec2::new(-120.0, 33.0);
    let c = d.place_linked_clone(owner, by).unwrap();
    let place_step = d.own_steps().last().unwrap().step;
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), PLACE_SKETCH_CLONE_COMMIT_LABEL);
    assert_eq!(PLACE_SKETCH_CLONE_COMMIT_LABEL, "place_sketch_clone");
    let snap = as_clone(&d, c);
    assert_eq!(snap.chain.root, owner);
    assert_eq!(snap.chain.origin, owner);
    assert!(snap.chain.own_placement, "own placement on (0057)");
    assert!(same_outline(
        &outline_of(&d, c),
        &shifted(&outline_of(&d, owner), by),
        TOL
    ));
    // Top of the paint order.
    let objects = d.read_objects();
    assert_eq!(objects.objects.last().unwrap().id(), c, "on top");
    // The sketch and the page are unchanged.
    let after = d.sketch(owner).unwrap();
    assert_eq!(after.graph, sketch_before.graph);
    assert_eq!(after.placement, sketch_before.placement);
    assert_eq!(page(&d, &sp.id), sp);
    // Moving the sketch page never pushes it.
    let clone_before = outline_of(&d, c);
    d.move_page(
        &sp.id,
        Vec2::new(20.0, 20.0),
        &curvyo_document_core::PageContents::from([(sp.id.clone(), vec![owner])]),
    )
    .unwrap();
    assert!(same_outline(&outline_of(&d, c), &clone_before, TOL));
    undo_last(&d);
    d.undo_step(place_step).unwrap();
    assert!(d.object(c).is_none(), "undo removes the clone");
    assert_eq!(d.sketch(owner).unwrap().graph, sketch_before.graph);
}

#[test]
fn ac8_place_linked_clone_refuses_anything_but_a_fixed_sketch_and_writes_nothing() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let loose = d
        .create_sketch(&closed_box(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let r = rect(&d, 1.0, 1.0, 5.0, 5.0);
    let c = clone_of(&d, owner);
    let version = d.version();
    let steps = d.own_step_count();
    for bad in [loose, r, c] {
        assert!(
            d.place_linked_clone(bad, Vec2::new(1.0, 1.0)).is_err(),
            "{bad:?}"
        );
    }
    for bad in [
        Vec2::new(f64::NAN, 0.0),
        Vec2::new(0.0, f64::INFINITY),
        Vec2::new(f64::NEG_INFINITY, 1.0),
    ] {
        assert!(d.place_linked_clone(owner, bad).is_err());
    }
    assert_eq!(d.version(), version, "nothing written");
    assert_eq!(d.own_step_count(), steps);
    // A deleted sketch page's sketch cannot be placed either.
    d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert!(d.place_linked_clone(owner, Vec2::new(0.0, 0.0)).is_err());
}

// ---- criterion 15: Convert to paths --------------------------------------------------------

#[test]
fn ac15_convert_makes_the_page_a_drawing_page_in_one_step_and_clones_follow_the_paths() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let later = d.add_page_of(&PageKind::Drawing).unwrap();
    let c1 = d.place_linked_clone(owner, Vec2::new(-150.0, 0.0)).unwrap();
    let c2 = clone_of(&d, c1); // a chained clone
    transform_clone(
        &d,
        c2,
        Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(30.0_f64.to_radians()),
        ),
    );
    let over = rect(&d, sp.origin.x + 2.0, sp.origin.y + 2.0, 8.0, 8.0);
    let sketch_before = d.sketch(owner).unwrap();
    let anchors_before = outline_of(&d, owner);
    let c1_before = outline_of(&d, c1);
    let c2_before = outline_of(&d, c2);
    let over_before = d.object(over).unwrap();
    let order = order_of(&d, &sp.id);
    let steps = d.own_step_count();
    let ids = fresh_ids(&d, owner, 700);
    d.convert_sketch_page(&sp.id, &ids).unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one undo step");
    assert_eq!(last_label(&d), CONVERT_SKETCH_PAGE_COMMIT_LABEL);
    assert_eq!(CONVERT_SKETCH_PAGE_COMMIT_LABEL, "convert_sketch_page");
    let p = page(&d, &sp.id);
    assert_eq!(p.kind, PageKind::Drawing);
    assert_eq!(d.page_kind(&sp.id), Some(PageKind::Drawing));
    assert_eq!(p.name, sp.name);
    assert_eq!(p.size, sp.size);
    assert_eq!(p.origin, sp.origin);
    assert_eq!(order_of(&d, &sp.id), order, "same place in the list");
    assert_eq!(d.page_sketch(&sp.id), None);
    let converted = d.object(owner).unwrap();
    let ObjectSnapshot::Path(path) = &converted else {
        panic!("not a path: {converted:?}");
    };
    assert!(path.closed, "closed where the loop closes");
    assert!(same_outline(
        &outline(&converted),
        &anchors_before,
        CURVE_TOL
    ));
    assert_eq!(path.style.stroke.width, Length::from_mm(0.7), "style kept");
    for (c, before) in [(c1, &c1_before), (c2, &c2_before)] {
        let snap = as_clone(&d, c);
        assert_eq!(snap.chain.root, owner, "still a linked clone, of the paths");
        assert!(same_outline(&outline_of(&d, c), before, CURVE_TOL));
    }
    assert_eq!(d.object(over).unwrap(), over_before);
    assert_eq!(page(&d, &later.id).kind, PageKind::Drawing);
    // Undo restores the sketch page with its sketch, constraints and parameters.
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(page(&d, &sp.id).kind, PageKind::Sketch);
    assert_eq!(d.page_sketch(&sp.id), Some(owner));
    let back = d.sketch(owner).unwrap();
    assert_eq!(back.graph, sketch_before.graph);
    assert_eq!(back.placement, sketch_before.placement);
    assert!(d.is_fixed_sketch(owner));
    for (c, before) in [(c1, &c1_before), (c2, &c2_before)] {
        assert_eq!(as_clone(&d, c).chain.root, owner);
        assert!(same_outline(&outline_of(&d, c), before, CURVE_TOL));
    }
    // Redo converts again.
    d.redo_step(step).unwrap();
    assert_eq!(page(&d, &sp.id).kind, PageKind::Drawing);
    assert!(matches!(d.object(owner), Some(ObjectSnapshot::Path(_))));
    assert_eq!(as_clone(&d, c1).chain.root, owner);
}

#[test]
fn ac15_convert_survives_save_and_reopen_with_the_new_kind() {
    let (d, sp, owner) = with_sketch(&driven_line());
    let ids = fresh_ids(&d, owner, 701);
    d.convert_sketch_page(&sp.id, &ids).unwrap();
    let r = reopen(&d, 9);
    assert_eq!(page(&r, &sp.id).kind, PageKind::Drawing);
    assert!(matches!(r.object(owner), Some(ObjectSnapshot::Path(_))));
}

#[test]
fn ac15_convert_refuses_other_pages_and_bad_ids_and_writes_nothing() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let empty = d.add_page_of(&PageKind::Sketch).unwrap();
    let ids = fresh_ids(&d, owner, 702);
    let version = d.version();
    assert!(
        d.convert_sketch_page("first", &ids).is_err(),
        "drawing page"
    );
    assert!(
        d.convert_sketch_page(&empty.id, &ids).is_err(),
        "no sketch yet"
    );
    assert!(d.convert_sketch_page("no-such-page", &ids).is_err());
    assert!(
        d.convert_sketch_page(&sp.id, &ids[..1]).is_err(),
        "too few ids"
    );
    assert_eq!(d.version(), version, "nothing written");
    assert_eq!(page(&d, &sp.id).kind, PageKind::Sketch);
}

#[test]
fn ac15_converting_an_ownership_conflict_object_writes_no_kind() {
    let a = Document::new(41);
    let sp = a.add_page_of(&PageKind::Sketch).unwrap();
    let b = reopen(&a, 42);
    let left = a
        .create_page_sketch(&sp.id, &closed_box(), &Style::default())
        .unwrap();
    let right = b
        .create_page_sketch(&sp.id, &closed_box(), &Style::default())
        .unwrap();
    exchange(&a, &b);
    let winner = a.page_sketch(&sp.id).unwrap();
    let loser = if winner == left { right } else { left };
    let ids = fresh_ids(&a, loser, 703);
    a.convert_sketch_to_path(loser, &ids).unwrap();
    assert_eq!(page(&a, &sp.id).kind, PageKind::Sketch, "kind unchanged");
    assert_eq!(a.page_sketch(&sp.id), Some(winner));
}

// ---- criterion 17: deleting a sketch page --------------------------------------------------

#[test]
fn ac17_delete_turns_every_clone_into_paths_and_undo_restores_them_as_clones() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let p2 = d.add_page_of(&PageKind::Drawing).unwrap();
    let first = page(&d, "first");
    let zero = d
        .sketch(owner)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    let to =
        |p: &Page, dx: f64| Vec2::new(p.origin.x + 50.0 + dx - zero.x, p.origin.y + 50.0 - zero.y);
    let a = d.place_linked_clone(owner, to(&first, 0.0)).unwrap();
    let b = d.place_linked_clone(owner, to(&first, 60.0)).unwrap();
    let c = d.place_linked_clone(owner, to(&p2, 0.0)).unwrap();
    transform_clone(
        &d,
        c,
        Affine::rotation_about(
            Point::new(p2.origin.x + 50.0, p2.origin.y + 50.0),
            Angle::from_radians(12.0_f64.to_radians()),
        ),
    );
    let pasteboard = d
        .place_linked_clone(owner, Vec2::new(0.0, -2000.0))
        .unwrap();
    let chained = clone_of(&d, a);
    d.translate_objects(&[chained], Vec2::new(0.0, 80.0))
        .unwrap();
    let over = rect(&d, sp.origin.x + 3.0, sp.origin.y + 3.0, 6.0, 6.0);
    let clones = [a, b, c, pasteboard, chained];
    let before: Vec<_> = clones.iter().map(|&id| outline_of(&d, id)).collect();
    let sketch_before = d.sketch(owner).unwrap();
    let over_before = d.object(over).unwrap();
    let pages_before = d.pages();
    let steps = d.own_step_count();
    d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one undo step");
    assert_eq!(last_label(&d), DELETE_SKETCH_PAGE_COMMIT_LABEL);
    assert_eq!(DELETE_SKETCH_PAGE_COMMIT_LABEL, "delete_sketch_page");
    assert!(d.pages().iter().all(|p| p.id != sp.id), "page gone");
    assert!(d.sketch(owner).is_none(), "sketch gone");
    assert!(d.object(owner).is_none());
    for (id, outline_before) in clones.iter().zip(&before) {
        let o = d.object(*id).unwrap_or_else(|| panic!("{id:?} kept"));
        match &o {
            ObjectSnapshot::Path(p) => {
                assert_eq!(
                    p.style.stroke.width,
                    Length::from_mm(0.7),
                    "style it showed"
                );
            }
            ObjectSnapshot::Clone(cl) => {
                assert_ne!(cl.chain.root, owner, "no clone follows the deleted sketch");
                assert!(d.object(cl.chain.root).is_some());
            }
            other => panic!("{other:?}"),
        }
        assert!(
            same_outline(&outline(&o), outline_before, CURVE_TOL),
            "{id:?} keeps its outline"
        );
    }
    for id in [a, b, c, pasteboard] {
        assert!(
            matches!(d.object(id), Some(ObjectSnapshot::Path(_))),
            "a direct clone becomes a path"
        );
    }
    assert_eq!(d.object(over).unwrap(), over_before, "foreign object stays");
    // Undo restores page, sketch and clones as clones.
    undo_last(&d);
    assert_eq!(d.pages(), pages_before);
    assert_eq!(d.page_sketch(&sp.id), Some(owner));
    let back = d.sketch(owner).unwrap();
    assert_eq!(back.graph, sketch_before.graph);
    assert_eq!(back.graph.constraints, sketch_before.graph.constraints);
    assert_eq!(back.graph.parameters, sketch_before.graph.parameters);
    for (id, outline_before) in clones.iter().zip(&before) {
        let snap = as_clone(&d, *id);
        assert_eq!(
            snap.chain.root, owner,
            "{id:?} a linked clone of the sketch again"
        );
        assert!(same_outline(
            &outline_of(&d, *id),
            outline_before,
            CURVE_TOL
        ));
    }
    assert!(as_clone(&d, a).chain.own_placement);
}

#[test]
fn ac17_delete_keeps_the_parameters_of_a_driven_sketch_through_undo() {
    let (d, sp, owner) = with_sketch(&driven_line());
    let graph = d.sketch(owner).unwrap().graph;
    d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    undo_last(&d);
    assert_eq!(d.sketch(owner).unwrap().graph, graph);
    d.apply_sketch_delta(owner, &set_width(100.0)).unwrap();
    let v = line_vector(&outline_of(&d, owner));
    assert!((v.x.hypot(v.y) - 80.0).abs() < CURVE_TOL, "still driven");
}

#[test]
fn ac17_the_last_page_cannot_be_deleted_even_a_sketch_page() {
    let d = Document::new(5);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    d.delete_page("first", &curvyo_document_core::PageContents::new())
        .unwrap();
    let owner = d
        .create_page_sketch(&sp.id, &closed_box(), &Style::default())
        .unwrap();
    let version = d.version();
    assert!(
        d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
            .is_err()
    );
    assert_eq!(d.version(), version, "nothing written");
    assert!(d.sketch(owner).is_some());
}

#[test]
fn ac17_an_empty_sketch_page_is_deleted_like_any_page() {
    let d = Document::new(6);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let r = rect(&d, sp.origin.x + 1.0, sp.origin.y + 1.0, 4.0, 4.0);
    let steps = d.own_step_count();
    d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert!(d.object(r).is_some());
    undo_last(&d);
    assert!(d.pages().iter().any(|p| p.id == sp.id));
}

#[test]
fn ac17_delete_leaves_a_conflict_object_on_the_page_untouched() {
    let a = Document::new(51);
    let sp = a.add_page_of(&PageKind::Sketch).unwrap();
    let b = reopen(&a, 52);
    let left = a
        .create_page_sketch(&sp.id, &closed_box(), &Style::default())
        .unwrap();
    let right = b
        .create_page_sketch(&sp.id, &driven_line(), &Style::default())
        .unwrap();
    exchange(&a, &b);
    let winner = a.page_sketch(&sp.id).unwrap();
    let loser = if winner == left { right } else { left };
    let loser_before = a.sketch(loser).unwrap().graph;
    a.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert!(a.sketch(winner).is_none(), "the owner goes");
    assert_eq!(
        a.sketch(loser).map(|s| s.graph),
        Some(loser_before),
        "the conflict object stays"
    );
}

#[test]
fn ac17_a_clone_made_by_a_peer_while_the_page_is_deleted_is_removed_as_an_orphan() {
    let a = Document::new(61);
    let sp = a.add_page_of(&PageKind::Sketch).unwrap();
    let owner = a
        .create_page_sketch(&sp.id, &closed_box(), &Style::default())
        .unwrap();
    let b = reopen(&a, 62);
    let late = b.place_linked_clone(owner, Vec2::new(-100.0, 0.0)).unwrap();
    a.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    exchange(&a, &b);
    a.remove_orphan_clones();
    assert!(
        !matches!(a.object(late), Some(ObjectSnapshot::Clone(_))),
        "no clone of a deleted sketch survives"
    );
}

// ---- criterion 19: files ---------------------------------------------------------------------

#[test]
// `0099` made no bump; `specs/0100-bring-content-in` bumps to 38 (ADR 0045 §9),
// `specs/0101-laser-operations` to 39 (ADR 0046 §4),
// `specs/0110-pen-modes-variable-stroke` to 42 (ADR 0053).
fn ac19_the_format_stays_37_until_0100_bumps_it_to_38() {
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    assert_eq!(CURRENT_FORMAT_VERSION, 42);
}

fn criterion_19_project() -> (Document, Vec<NodeId>) {
    let d = Document::new(19);
    let p2 = d.add_page_of(&PageKind::Drawing).unwrap();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let mut graph = sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "edge",
        &[
            sk::Position::mm(-30.0, -20.0),
            sk::Position::mm(30.0, -20.0),
            sk::Position::mm(30.0, 20.0),
            sk::Position::mm(-30.0, 20.0),
        ],
        false,
    )
    .unwrap();
    let lines: Vec<_> = graph
        .entities
        .iter()
        .filter(|(_, e)| matches!(e, sk::Entity::Line { .. }))
        .map(|(id, _)| id.clone())
        .collect();
    assert_eq!(lines.len(), 3);
    for (n, l) in lines.iter().enumerate() {
        graph.constraints.insert(
            format!("c{n}").as_str().into(),
            sk::Constraint::driving(if n % 2 == 0 {
                sk::Relation::Horizontal(l.clone())
            } else {
                sk::Relation::Vertical(l.clone())
            }),
        );
    }
    let graph =
        sk::create_circle(&graph, "hole", sk::Position::mm(0.0, 0.0), sk::Length(5.0)).unwrap();
    let owner = d
        .create_page_sketch(&sp.id, &graph, &Style::default())
        .unwrap();
    let zero = d
        .sketch(owner)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    let on_drawing = d
        .place_linked_clone(
            owner,
            Vec2::new(p2.origin.x + 80.0 - zero.x, p2.origin.y + 80.0 - zero.y),
        )
        .unwrap();
    let pivot = Point::new(laser.origin.x + 100.0, laser.origin.y + 100.0);
    let on_laser = d
        .place_linked_clone(owner, Vec2::new(pivot.x - zero.x, pivot.y - zero.y))
        .unwrap();
    transform_clone(
        &d,
        on_laser,
        Affine::rotation_about(pivot, Angle::from_radians(12.0_f64.to_radians())),
    );
    let over = rect(&d, sp.origin.x + 4.0, sp.origin.y + 4.0, 10.0, 10.0);
    let sp2 = d.add_page_of(&PageKind::Sketch).unwrap();
    let second = d
        .create_page_sketch(&sp2.id, &closed_box(), &Style::default())
        .unwrap();
    let ids = fresh_ids(&d, second, 719);
    d.convert_sketch_page(&sp2.id, &ids).unwrap();
    (d, vec![owner, on_drawing, on_laser, over, second])
}

fn assert_same_project(a: &Document, b: &Document) {
    assert_eq!(a.pages(), b.pages(), "kind, name, order, size, place");
    assert_eq!(a.read_objects().objects, b.read_objects().objects);
    for p in a.pages() {
        assert_eq!(a.page_sketch(&p.id), b.page_sketch(&p.id));
    }
}

#[test]
fn ac19_a_criterion_project_saves_and_reopens_identically() {
    let (d, ids) = criterion_19_project();
    let r = reopen(&d, 20);
    assert_same_project(&d, &r);
    assert!(r.is_fixed_sketch(ids[0]));
    assert!(as_clone(&r, ids[1]).chain.own_placement);
    assert!(as_clone(&r, ids[2]).chain.own_placement);
    assert!(matches!(r.object(ids[4]), Some(ObjectSnapshot::Path(_))));
    let rr = reopen(&r, 21);
    assert_same_project(&r, &rr);
}

#[test]
fn ac19_the_golden_holds_the_criterion_project_and_round_trips() {
    let bytes = fixture("sketch_pages_v37.curvyo");
    let d = unpack(3, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let pages = d.pages();
    let kinds = |k: fn(&PageKind) -> bool| pages.iter().filter(|p| k(&p.kind)).count();
    assert_eq!(
        kinds(|k| matches!(k, PageKind::Output { .. })),
        1,
        "a laser page"
    );
    assert_eq!(kinds(|k| *k == PageKind::Sketch), 1, "one sketch page left");
    assert!(
        kinds(|k| *k == PageKind::Drawing) >= 3,
        "two drawing pages and the converted one"
    );
    let sketch_page = pages.iter().find(|p| p.kind == PageKind::Sketch).unwrap();
    let owner = d
        .page_sketch(&sketch_page.id)
        .expect("the sketch page owns its sketch");
    let s = d.sketch(owner).unwrap();
    let lines = s
        .graph
        .entities
        .values()
        .filter(|e| matches!(e, sk::Entity::Line { .. }))
        .count();
    let circles = s
        .graph
        .entities
        .values()
        .filter(|e| matches!(e, sk::Entity::Circle { .. }))
        .count();
    assert_eq!((lines, circles), (3, 1), "three lines and a circle");
    assert!(!s.graph.constraints.is_empty(), "constrained");
    let objects = d.read_objects().objects.to_vec();
    let clones: Vec<_> = objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(c) if c.chain.root == owner => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(
        clones.len(),
        2,
        "a clone on a drawing page and one on the laser page"
    );
    assert!(clones.iter().all(|c| c.chain.own_placement));
    let rotated = clones.iter().any(|c| {
        let m = c.chain.matrix;
        (m.b.atan2(m.a).to_degrees().abs() - 12.0).abs() < 1e-6
    });
    assert!(rotated, "the laser clone is rotated by 12 degrees");
    let inside = |p: &Page, o: &ObjectSnapshot| match o {
        ObjectSnapshot::Primitive(_) => {
            let pts: Vec<_> = outline_points(o);
            !pts.is_empty() && pts.iter().all(|pt| p.contains(*pt))
        }
        _ => false,
    };
    assert!(
        objects.iter().any(|o| inside(sketch_page, o)),
        "a rectangle lies over the sketch page"
    );
    // The converted page: a drawing page holding paths and no sketch.
    assert!(objects.iter().any(|o| matches!(o, ObjectSnapshot::Path(_))));
    let r = reopen(&d, 4);
    assert_same_project(&d, &r);
}

fn outline_points(o: &ObjectSnapshot) -> Vec<Point> {
    match o {
        ObjectSnapshot::Primitive(p) => curvyo_document_core::outline_of(&p.shape)
            .anchors
            .iter()
            .map(|a| a.point)
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn ac19_the_golden_file_says_format_37() {
    let bytes = fixture("sketch_pages_v37.curvyo");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let mut manifest = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("manifest.json").unwrap(),
        &mut manifest,
    )
    .unwrap();
    let m: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(m["format_version"], 37);
}

// ---- white-box edges -----------------------------------------------------------------------

#[test]
fn edge_a_locked_sketch_still_goes_with_its_page() {
    let (d, sp, owner) = with_sketch(&closed_box());
    let c = d.place_linked_clone(owner, Vec2::new(-200.0, 0.0)).unwrap();
    d.set_node_locked(&[owner], true).unwrap();
    let steps = d.own_step_count();
    d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert!(d.sketch(owner).is_none());
    assert!(matches!(d.object(c), Some(ObjectSnapshot::Path(_))));
}

#[test]
fn edge_a_huge_placement_is_refused_or_reads_back_finite() {
    let mut broken = Vec::new();
    for by in [
        Vec2::new(1e300, 0.0),
        Vec2::new(-1e15, 1e15),
        Vec2::new(1e9, 0.0),
    ] {
        let (d, _sp, owner) = with_sketch(&closed_box());
        let version = d.version();
        match d.place_linked_clone(owner, by) {
            Err(_) => assert_eq!(d.version(), version, "{by:?}: a refusal writes nothing"),
            Ok(c) => match unpack(78, &pack(&d, "x").unwrap()) {
                Err(e) => broken.push(format!("{by:?}: {e:?}")),
                Ok(r) => {
                    let o = r.object(c).expect("the clone reads back");
                    if !outline(&o)
                        .iter()
                        .all(|a| a.point.x.is_finite() && a.point.y.is_finite())
                    {
                        broken.push(format!("{by:?}: a non-finite outline"));
                    }
                }
            },
        }
    }
    assert!(broken.is_empty(), "{broken:?}");
}

#[test]
fn edge_a_peer_placing_a_clone_while_the_page_is_converted_follows_the_paths() {
    let (a, sp, owner) = with_sketch(&closed_box());
    let b = reopen(&a, 81);
    let c = b.place_linked_clone(owner, Vec2::new(-150.0, 0.0)).unwrap();
    let before = outline_of(&b, c);
    let ids = fresh_ids(&a, owner, 801);
    a.convert_sketch_page(&sp.id, &ids).unwrap();
    exchange(&a, &b);
    for d in [&a, &b] {
        assert_eq!(page(d, &sp.id).kind, PageKind::Drawing);
        assert_eq!(as_clone(d, c).chain.root, owner);
        assert!(same_outline(&outline_of(d, c), &before, CURVE_TOL));
    }
}

#[test]
fn edge_a_peer_editing_the_sketch_while_it_is_converted_leaves_no_fixed_sketch_on_a_drawing_page() {
    let (a, sp, owner) = with_sketch(&driven_line());
    let b = reopen(&a, 82);
    b.apply_sketch_delta(owner, &set_width(100.0)).unwrap();
    let ids = fresh_ids(&a, owner, 802);
    a.convert_sketch_page(&sp.id, &ids).unwrap();
    exchange(&a, &b);
    for d in [&a, &b] {
        assert_eq!(page(d, &sp.id).kind, PageKind::Drawing);
        assert!(!d.is_fixed_sketch(owner), "nothing fixed on a drawing page");
        assert!(d.object(owner).is_some(), "the object stays visible");
        assert_eq!(d.page_sketch(&sp.id), None);
    }
}

#[test]
fn edge_a_peer_editing_the_sketch_while_its_page_is_deleted_converges() {
    let (a, sp, owner) = with_sketch(&driven_line());
    let c = a.place_linked_clone(owner, Vec2::new(-150.0, 0.0)).unwrap();
    let b = reopen(&a, 83);
    b.apply_sketch_delta(owner, &set_width(100.0)).unwrap();
    a.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    exchange(&a, &b);
    assert_eq!(a.pages(), b.pages());
    assert!(a.pages().iter().all(|p| p.id != sp.id));
    assert_eq!(
        a.read_objects().objects,
        b.read_objects().objects,
        "both peers converge"
    );
    assert!(a.object(c).is_some(), "the clone (as a path) stays");
}
