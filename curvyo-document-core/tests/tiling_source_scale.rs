//! A tiling instance follows the linear part of a source transform (`specs/0069-tiling`
//! criterion 29, ADR 0016 §2 amendment and ADR 0028 §2 of 2026-11-02): one pivot per tiling, the
//! centre of Apply's cell box; the chain rule holds through an instance; flip, skew, a scale
//! with rotation, undo, re-Apply and a concurrent Release behave as the ADRs say.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::collections::HashMap;

use curvyo_document_core::{
    Affine, AnchorId, Angle, CellKey, CircleOptions, CloneLinks, Document, GridOptions, Length,
    NewAnchor, NodeId, ObjectSnapshot, OffsetLines, Percent, Point, SpacingMode, TilingCell,
    TilingLayout, TilingRecipe, TilingRequest, TilingResult, Variance, Vec2, clone_matrices_after,
    pack, unpack,
};

/// The triangle (x, 0), (x + 10, 0), (x + 10, 10).
fn path(document: &Document, x: f64, n: u64) -> NodeId {
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(7, n), Point::new(x, 0.0)),
            NewAnchor::corner(AnchorId::new(7, n + 1), Point::new(x + 10.0, 0.0)),
            NewAnchor::corner(AnchorId::new(7, n + 2), Point::new(x + 10.0, 10.0)),
        ],
        true,
    )
}

/// A grid with a fixed pitch of `PITCH`: Apply's instance affine is then the plain translation
/// by the cell's offset, whatever the cell box.
const PITCH: f64 = 40.0;

fn recipe(columns: u32) -> TilingRecipe {
    TilingRecipe {
        layout: TilingLayout::Grid,
        grid: GridOptions {
            columns,
            rows: 1,
            spacing_x: Length::from_mm(PITCH),
            spacing_y: Length::from_mm(PITCH),
            spacing_mode: SpacingMode::Pitch,
            mirror_rows: false,
            mirror_columns: false,
            offset: OffsetLines::None,
            shift: Percent::new(50.0),
        },
        circle: CircleOptions {
            count: 12,
            centre: Vec2::new(-20.0, 0.0),
            radius: Length::from_mm(20.0),
            start: Angle::from_radians(0.0),
            sweep: Angle::from_radians(std::f64::consts::TAU),
            face_centre: false,
        },
        variance: Variance::default(),
        seed: 42,
        result: TilingResult::Clones,
        keep_source: true,
    }
}

fn request(tiling: Option<NodeId>, sources: Vec<NodeId>, columns: u32) -> TilingRequest {
    TilingRequest {
        tiling,
        sources,
        cells: (1..columns)
            .map(|column| TilingCell {
                key: CellKey { row: 0, column },
                affine: shift(f64::from(column) * PITCH, 0.0),
                opacity: Percent::new(0.0),
                hue: Angle::from_radians(0.0),
            })
            .collect(),
        recipe: recipe(columns),
        conversions: Vec::new(),
    }
}

fn mint() -> AnchorId {
    AnchorId::new(99, 1)
}

fn tile(d: &Document, sources: Vec<NodeId>, columns: u32) -> NodeId {
    d.apply_tiling(&request(None, sources, columns), &mut mint)
        .unwrap()
        .tiling
}

fn shift(x: f64, y: f64) -> Affine {
    Affine::translation(Vec2::new(x, y))
}

/// `linear` applied about `pivot`.
fn about(pivot: Point, linear: Affine) -> Affine {
    shift(pivot.x, pivot.y)
        .compose(linear)
        .compose(shift(-pivot.x, -pivot.y))
}

fn drawn_points(d: &Document, id: NodeId) -> Vec<Point> {
    let path = match d.object(id) {
        Some(ObjectSnapshot::Clone(clone)) => clone.resolved,
        Some(ObjectSnapshot::Path(path)) => path,
        other => panic!("a clone or a path: {other:?}"),
    };
    path.anchors.iter().map(|a| a.point).collect()
}

fn mapped_points(m: Affine, points: &[Point]) -> Vec<Point> {
    points.iter().map(|p| m.apply_point(*p)).collect()
}

fn assert_close(got: &[Point], want: &[Point], what: &str) {
    assert!(
        got.len() == want.len()
            && got
                .iter()
                .zip(want)
                .all(|(p, q)| (p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9),
        "{what}: {got:?} vs {want:?}"
    );
}

/// Every object of `ids` mapped by `g` as a gesture gives it: paths mapped, clones with the
/// matrices of [`clone_matrices_after`].
fn mapped_results(d: &Document, ids: &[NodeId], g: Affine) -> Vec<ObjectSnapshot> {
    let links: CloneLinks = d.clone_links();
    let matrices: HashMap<NodeId, Affine> =
        clone_matrices_after(&links, &d.own_placement_clones(), ids, g)
            .matrices
            .into_iter()
            .collect();
    ids.iter()
        .map(|&id| match d.object(id).unwrap() {
            ObjectSnapshot::Path(path) => ObjectSnapshot::Path(path.mapped(g)),
            ObjectSnapshot::Clone(clone) => {
                let mut moved = clone.mapped_by(g);
                moved.chain.matrix = matrices[&id];
                ObjectSnapshot::Clone(moved)
            }
            other => panic!("a path or a clone: {other:?}"),
        })
        .collect()
}

/// One source tiled 3 x 1; returns the document, the source and the tiling group.
fn one_source() -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let group = tile(&d, vec![source], 3);
    (d, source, group)
}

/// The centre of the single source's cell box (0, 0)..(10, 10).
const CENTRE: Point = Point::new(5.0, 5.0);

/// Blocking 1 (a): an own-placement clone of an instance keeps its outline when the source is
/// scaled, though the instance it follows grows (ADR 0016 §2, `0057` criterion 3).
#[test]
fn an_own_placement_clone_of_an_instance_keeps_its_outline_when_the_source_scales() {
    let (d, source, group) = one_source();
    let instance = d.children(Some(group))[1];
    let clone = d.linked_clone(&[instance], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(0.0, 30.0)).unwrap();
    d.set_own_placement(&[clone], true).unwrap();
    let before = drawn_points(&d, clone);
    let source_before = drawn_points(&d, source);
    let scale = about(CENTRE, Affine::scale(2.0, 2.0));
    d.transform_objects(&mapped_results(&d, &[source], scale), false)
        .unwrap();
    assert_close(
        &drawn_points(&d, instance),
        &mapped_points(shift(PITCH, 0.0).compose(scale), &source_before),
        "the instance follows the scale",
    );
    assert_close(&drawn_points(&d, clone), &before, "the clone of it stays");
}

/// Blocking 1 (b): a clone of an instance selected with the source draws its old outline mapped
/// by the gesture (`0049` criterion 16: `G · M · T_Y⁻¹` with the instance's real change).
#[test]
fn a_selected_clone_of_an_instance_gets_exactly_the_gesture_map() {
    let (d, source, group) = one_source();
    let instance = d.children(Some(group))[1];
    let clone = d.linked_clone(&[instance], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(0.0, 30.0)).unwrap();
    let before = drawn_points(&d, clone);
    let scale = about(CENTRE, Affine::scale(2.0, 2.0));
    d.transform_objects(&mapped_results(&d, &[source, clone], scale), false)
        .unwrap();
    assert_close(
        &drawn_points(&d, clone),
        &mapped_points(scale, &before),
        "the selected clone",
    );
}

/// Blocking 2: the members of a two-source tiling scale about the one cell box centre, so their
/// spacing scales with the sources and the result is what Apply writes: re-Apply writes nothing.
#[test]
fn two_sources_scaled_together_keep_their_member_spacing_and_re_apply_writes_nothing() {
    let d = Document::new(1);
    let a = path(&d, 0.0, 1);
    let b = path(&d, 20.0, 10);
    let group = tile(&d, vec![a, b], 2);
    let instance = d.children(Some(group))[2];
    let members = d.children(Some(instance));
    assert_eq!(members.len(), 2);
    let before: Vec<Vec<Point>> = [a, b].iter().map(|s| drawn_points(&d, *s)).collect();
    // The cell box is (0, 0)..(30, 10).
    let scale = about(Point::new(15.0, 5.0), Affine::scale(2.0, 2.0));
    d.transform_objects(&mapped_results(&d, &[a, b], scale), false)
        .unwrap();
    for (member, source) in members.iter().zip(&before) {
        assert_close(
            &drawn_points(&d, *member),
            &mapped_points(shift(PITCH, 0.0).compose(scale), source),
            "a member",
        );
    }
    let outcome = d
        .apply_tiling(&request(Some(group), vec![], 2), &mut mint)
        .unwrap();
    assert!(!outcome.written, "re-Apply after the scale writes nothing");
}

/// Non-blocking 6: a single source scaled about its cell box centre is what Apply writes.
#[test]
fn re_apply_after_a_scale_writes_nothing() {
    let (d, source, group) = one_source();
    let scale = about(CENTRE, Affine::scale(1.5, 3.0));
    d.transform_objects(&mapped_results(&d, &[source], scale), false)
        .unwrap();
    let outcome = d
        .apply_tiling(&request(Some(group), vec![], 3), &mut mint)
        .unwrap();
    assert!(!outcome.written);
}

/// Criterion 29 (customer 2026-11-02): a stretch and a rotation in one gesture reach the
/// instance whole, about its own cell position; only the move of the pivot is held back.
#[test]
fn a_stretch_with_a_rotation_in_one_gesture_turns_and_stretches_the_instance_in_place() {
    let (d, source, group) = one_source();
    let instance = d.children(Some(group))[1];
    let source_before = drawn_points(&d, source);
    let (sin, cos) = 0.6_f64.sin_cos();
    let turn = Affine::new(cos, sin, -sin, cos, 0.0, 0.0);
    let stretch = Affine::scale(2.0, 1.0);
    let gesture = shift(7.0, -3.0).compose(about(CENTRE, turn.compose(stretch)));
    d.transform_objects(&mapped_results(&d, &[source], gesture), false)
        .unwrap();
    assert_close(
        &drawn_points(&d, instance),
        &mapped_points(
            shift(PITCH, 0.0).compose(about(CENTRE, turn.compose(stretch))),
            &source_before,
        ),
        "turned and stretched in its cell, not moved",
    );
}

/// Criterion 29: a rotation about a point off the pivot turns the instance about its own cell
/// position; the move it gives the pivot is held back.
#[test]
fn a_rotation_of_the_source_turns_the_instance_in_its_cell() {
    let (d, source, group) = one_source();
    let instance = d.children(Some(group))[1];
    let source_before = drawn_points(&d, source);
    let turn = |centre: Point| Affine::rotation_about(centre, Angle::from_radians(0.7));
    d.transform_objects(
        &mapped_results(&d, &[source], turn(Point::new(0.0, 0.0))),
        false,
    )
    .unwrap();
    assert_close(
        &drawn_points(&d, instance),
        &mapped_points(shift(PITCH, 0.0).compose(turn(CENTRE)), &source_before),
        "turned about its cell centre",
    );
    assert_eq!(d.take_clone_hold().held, 0);
}

/// Non-blocking 1 and 6: a flip (negative determinant) passes whole to the instance, about the
/// cell box centre.
#[test]
fn a_flip_of_the_source_flips_the_instance_in_its_cell() {
    let (d, source, group) = one_source();
    let instance = d.children(Some(group))[1];
    let source_before = drawn_points(&d, source);
    let flip = about(CENTRE, Affine::scale(-1.0, 1.0));
    d.transform_objects(&mapped_results(&d, &[source], flip), false)
        .unwrap();
    assert_close(
        &drawn_points(&d, instance),
        &mapped_points(shift(PITCH, 0.0).compose(flip), &source_before),
        "flipped in its cell",
    );
    assert_eq!(d.take_clone_hold().held, 0);
}

/// Criterion 29: a skew of the source reaches the instance whole, about the cell box centre.
#[test]
fn a_skew_of_the_source_skews_the_instance_in_its_cell() {
    let (d, source, group) = one_source();
    let instance = d.children(Some(group))[1];
    let source_before = drawn_points(&d, source);
    let shear = about(CENTRE, Affine::new(1.0, 0.0, 0.5, 1.0, 0.0, 0.0));
    d.transform_objects(&mapped_results(&d, &[source], shear), false)
        .unwrap();
    assert_close(
        &drawn_points(&d, instance),
        &mapped_points(shift(PITCH, 0.0).compose(shear), &source_before),
        "skewed in its cell",
    );
}

/// Non-blocking 6: one undo of the scale restores the instance matrices.
#[test]
fn undo_of_a_scale_restores_the_instances() {
    let (d, source, group) = one_source();
    let instances = d.tiling_instances(group);
    let before: Vec<Vec<Point>> = instances.iter().map(|i| drawn_points(&d, *i)).collect();
    let scale = about(CENTRE, Affine::scale(2.0, 2.0));
    d.transform_objects(&mapped_results(&d, &[source], scale), false)
        .unwrap();
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    for (instance, want) in instances.iter().zip(&before) {
        assert_close(&drawn_points(&d, *instance), want, "after undo");
    }
}

/// Non-blocking 5 (ADR 0028 §6): a peer releases the tiling while this peer scales the source.
/// The cell key was read here, so after the merge the instance is a plain clone with own
/// placement whose matrix passed the scale; the next scale holds it fully.
#[test]
fn a_concurrent_release_leaves_a_plain_clone_that_followed_the_scale() {
    let (a, source, group) = one_source();
    let instance = a.children(Some(group))[1];
    let b = unpack(2, &pack(&a, "test").unwrap()).unwrap();
    let source_before = drawn_points(&a, source);
    let (av, bv) = (a.version(), b.version());
    b.release_tiling(group).unwrap();
    let scale = about(CENTRE, Affine::scale(2.0, 2.0));
    a.transform_objects(&mapped_results(&a, &[source], scale), false)
        .unwrap();
    let to_a = b.export_updates(&av).unwrap();
    let to_b = a.export_updates(&bv).unwrap();
    a.import_updates(&to_a).unwrap();
    b.import_updates(&to_b).unwrap();
    for d in [&a, &b] {
        assert!(d.tiling(group).is_none(), "released");
        assert!(d.own_placement_clones().contains_key(&instance));
        assert_close(
            &drawn_points(d, instance),
            &mapped_points(shift(PITCH, 0.0).compose(scale), &source_before),
            "followed the scale",
        );
    }
    let held = drawn_points(&a, instance);
    a.transform_objects(&mapped_results(&a, &[source], scale), false)
        .unwrap();
    assert_close(
        &drawn_points(&a, instance),
        &held,
        "a plain clone is held fully",
    );
}
