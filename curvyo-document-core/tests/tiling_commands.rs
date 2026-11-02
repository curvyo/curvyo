//! The tiling commands (`specs/0069-tiling` criteria 4, 18, 22 to 32, ADR 0030 §3 to §7): Apply
//! creates and re-applies a tiling in one commit with minimal writes, refuses before the first
//! write, and Release, Ungroup and Duplicate handle the tiling registers.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::large_types_passed_by_value
)]

use std::cell::Cell;

use curvyo_document_core::{
    Affine, AnchorId, Angle, CellKey, CircleOptions, Document, FontKind, FontRef, GridOptions,
    Length, MovePlan, MoveTarget, NewAnchor, NodeId, ObjectSnapshot, OffsetLines, Percent, Point,
    SpacingMode, TILING_GROUP_NAME, TextLayoutValues, TilingCell, TilingError, TilingLayout,
    TilingRecipe, TilingRequest, TilingResult, Variance, Vec2,
};

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

fn recipe(columns: u32, rows: u32) -> TilingRecipe {
    TilingRecipe {
        layout: TilingLayout::Grid,
        grid: GridOptions {
            columns,
            rows,
            spacing_x: Length::from_mm(5.0),
            spacing_y: Length::from_mm(5.0),
            spacing_mode: SpacingMode::Gap,
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

/// Plain translations by 15 mm per column and row, the kept source's cell left out.
fn cells(recipe: &TilingRecipe, shift: f64) -> Vec<TilingCell> {
    let mut out = Vec::new();
    for row in 0..recipe.grid.rows {
        for column in 0..recipe.grid.columns {
            let key = CellKey { row, column };
            if recipe.keep_source && key == CellKey::ORIGIN {
                continue;
            }
            out.push(TilingCell {
                key,
                affine: Affine::translation(Vec2::new(
                    f64::from(column) * 15.0 + shift,
                    f64::from(row) * 15.0,
                )),
                opacity: Percent::new(0.0),
                hue: Angle::from_radians(0.0),
            });
        }
    }
    out
}

fn request(tiling: Option<NodeId>, sources: Vec<NodeId>, recipe: TilingRecipe) -> TilingRequest {
    TilingRequest {
        tiling,
        sources,
        cells: cells(&recipe, 0.0),
        recipe,
        conversions: Vec::new(),
    }
}

fn minter() -> impl FnMut() -> AnchorId {
    let next = Cell::new(1000_u64);
    move || {
        next.set(next.get() + 1);
        AnchorId::new(99, next.get())
    }
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn tile(d: &Document, sources: Vec<NodeId>, recipe: TilingRecipe) -> NodeId {
    d.apply_tiling(&request(None, sources, recipe), &mut minter())
        .unwrap()
        .tiling
}

#[test]
fn tile_makes_one_group_with_the_source_lowest_and_one_clone_per_cell() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let steps = d.own_steps().len();
    let group = tile(&d, vec![source], recipe(3, 2));
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(last_label(&d), "tile");
    let children = d.children(Some(group));
    assert_eq!(children.len(), 6);
    assert_eq!(children[0], source);
    assert_eq!(
        d.node_registers(group).unwrap().name.as_deref(),
        Some(TILING_GROUP_NAME)
    );
    let read = d.read_objects();
    let Some(ObjectSnapshot::Clone(clone)) = read.get(children[2]) else {
        panic!("a clone instance")
    };
    assert_eq!(clone.chain.root, source);
    // Cell (0, 2) sits two columns right of the source.
    assert_eq!(clone.resolved.anchors[0].point, Point::new(30.0, 0.0));
    let stored = d.tiling(group).unwrap();
    assert_eq!(stored.recipe, recipe(3, 2));
    assert_eq!(stored.sources, vec![source]);
}

#[test]
fn every_refusal_comes_before_the_first_write() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let version = d.version();
    let mut too_many = recipe(1000, 101);
    too_many.keep_source = true;
    let refusals = [
        request(None, vec![], recipe(2, 2)),
        TilingRequest {
            cells: Vec::new(),
            ..request(None, vec![source], too_many)
        },
        request(Some(source), vec![], recipe(2, 2)),
        TilingRequest {
            cells: cells(
                &TilingRecipe {
                    keep_source: false,
                    ..recipe(2, 2)
                },
                0.0,
            ),
            ..request(None, vec![source], recipe(2, 2))
        },
    ];
    let expected = [
        TilingError::EmptySource,
        TilingError::Recipe(curvyo_document_core::RecipeError::TooMany),
        TilingError::NotATiling,
        TilingError::Cells,
    ];
    for (refused, error) in refusals.iter().zip(expected) {
        assert_eq!(d.apply_tiling(refused, &mut minter()), Err(error));
        assert_eq!(d.version(), version);
    }
    d.set_node_locked(&[source], true).unwrap();
    let version = d.version();
    assert_eq!(
        d.apply_tiling(&request(None, vec![source], recipe(2, 2)), &mut minter()),
        Err(TilingError::Locked)
    );
    assert_eq!(d.version(), version);
}

#[test]
fn a_tiling_cannot_be_tiled() {
    let d = Document::new(1);
    let group = tile(&d, vec![path(&d, 0.0, 1)], recipe(2, 1));
    let version = d.version();
    assert_eq!(
        d.apply_tiling(&request(None, vec![group], recipe(2, 1)), &mut minter()),
        Err(TilingError::ContainsTiling)
    );
    assert_eq!(d.version(), version);
}

#[test]
fn a_layer_cannot_be_tiled() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let version = d.version();
    assert_eq!(
        d.apply_tiling(&request(None, vec![layer], recipe(2, 1)), &mut minter()),
        Err(TilingError::IsALayer)
    );
    assert_eq!(d.version(), version);
}

#[test]
fn an_unchanged_apply_writes_nothing_and_a_changed_one_is_one_edit_step() {
    let d = Document::new(1);
    let group = tile(&d, vec![path(&d, 0.0, 1)], recipe(3, 1));
    let version = d.version();
    let steps = d.own_steps().len();
    let outcome = d
        .apply_tiling(&request(Some(group), vec![], recipe(3, 1)), &mut minter())
        .unwrap();
    assert!(!outcome.written);
    assert_eq!(d.version(), version);
    let mut wider = recipe(4, 1);
    wider.seed = 7;
    d.apply_tiling(&request(Some(group), vec![], wider), &mut minter())
        .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(last_label(&d), "edit_tiling");
    assert_eq!(d.children(Some(group)).len(), 4);
}

#[test]
fn a_re_apply_keeps_ids_restores_deleted_cells_and_leaves_hand_work_alone() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let group = tile(&d, vec![source], recipe(4, 1));
    let before = d.children(Some(group));
    // Delete cell (0, 1), unlink cell (0, 2), drop an unkeyed object into the group.
    d.delete_objects(&[before[1]]).unwrap();
    let unlinked = before[2];
    let anchors: Vec<AnchorId> = (0..3).map(|n| AnchorId::new(55, n)).collect();
    d.unlink_clones(&[curvyo_document_core::CopySource {
        id: unlinked,
        anchor_ids: anchors,
    }])
    .unwrap();
    let mut shifted = request(Some(group), vec![], recipe(4, 1));
    shifted.cells = cells(&recipe(4, 1), 1.0);
    d.apply_tiling(&shifted, &mut minter()).unwrap();
    let after = d.children(Some(group));
    assert_eq!(after.len(), 4);
    assert!(after.contains(&before[3]), "cell (0, 3) kept its id");
    assert!(after.contains(&unlinked), "the unlinked cell is left alone");
    let read = d.read_objects();
    let Some(ObjectSnapshot::Path(hand)) = read.get(unlinked) else {
        panic!("still a path")
    };
    assert_eq!(hand.anchors[0].point, Point::new(30.0, 0.0), "not moved");
    let Some(ObjectSnapshot::Clone(kept)) = read.get(before[3]) else {
        panic!("a clone")
    };
    assert_eq!(
        kept.resolved.anchors[0].point,
        Point::new(46.0, 0.0),
        "moved by the shift"
    );
}

#[test]
fn fewer_columns_delete_the_cells_that_left() {
    let d = Document::new(1);
    let group = tile(&d, vec![path(&d, 0.0, 1)], recipe(4, 2));
    d.apply_tiling(&request(Some(group), vec![], recipe(2, 1)), &mut minter())
        .unwrap();
    assert_eq!(d.children(Some(group)).len(), 2);
}

#[test]
fn copies_are_independent_paths_and_switching_replaces_the_clones() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let mut copies = recipe(2, 1);
    copies.result = TilingResult::Copies;
    let group = tile(&d, vec![source], recipe(2, 1));
    let clone = d.children(Some(group))[1];
    d.apply_tiling(&request(Some(group), vec![], copies), &mut minter())
        .unwrap();
    let children = d.children(Some(group));
    assert_ne!(children[1], clone);
    let read = d.read_objects();
    let Some(ObjectSnapshot::Path(copy)) = read.get(children[1]) else {
        panic!("a path copy")
    };
    assert_eq!(copy.anchors[2].point, Point::new(25.0, 10.0));
}

#[test]
fn several_sources_give_instance_groups_with_member_keys() {
    let d = Document::new(1);
    let a = path(&d, 0.0, 1);
    let b = path(&d, 20.0, 10);
    let group = tile(&d, vec![b, a], recipe(2, 1));
    let children = d.children(Some(group));
    assert_eq!(children[..2], [a, b], "the sources in z-order, lowest");
    let instance = children[2];
    assert!(d.is_group(instance));
    assert_eq!(d.children(Some(instance)).len(), 2);
    let version = d.version();
    let outcome = d
        .apply_tiling(&request(Some(group), vec![], recipe(2, 1)), &mut minter())
        .unwrap();
    assert!(!outcome.written);
    assert_eq!(d.version(), version);
}

#[test]
fn hue_variance_gives_instances_an_own_style() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let mut varied = recipe(2, 1);
    varied.variance.hue.random = Angle::from_radians(1.0);
    let mut req = request(None, vec![source], varied);
    req.cells[0].hue = Angle::from_radians(2.0);
    let group = d.apply_tiling(&req, &mut minter()).unwrap().tiling;
    let read = d.read_objects();
    let Some(ObjectSnapshot::Clone(clone)) = read.get(d.children(Some(group))[1]) else {
        panic!("a clone")
    };
    assert!(clone.chain.own_style);
}

#[test]
fn keep_source_off_leaves_the_source_and_puts_the_group_above_it() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let mut off = recipe(2, 1);
    off.keep_source = false;
    let group = tile(&d, vec![source], off);
    assert_eq!(d.children(Some(group)).len(), 2);
    let top = d.children(None);
    let at_source = top.iter().position(|n| *n == source).unwrap();
    assert_eq!(top[at_source + 1], group);
}

#[test]
fn release_and_ungroup_strip_the_tiling() {
    let d = Document::new(1);
    let group = tile(&d, vec![path(&d, 0.0, 1)], recipe(2, 1));
    d.release_tiling(group).unwrap();
    assert_eq!(last_label(&d), "release_tiling");
    assert!(d.tiling(group).is_none());
    assert!(d.is_group(group));
    let other = tile(&d, vec![path(&d, 50.0, 20)], recipe(2, 1));
    let released = d.ungroup(&[other]).unwrap();
    assert_eq!(released.len(), 2);
    assert_eq!(d.release_tiling(group), Err(TilingError::NotATiling));
}

#[test]
fn a_duplicated_tiling_points_at_its_own_source() {
    let d = Document::new(1);
    let group = tile(&d, vec![path(&d, 0.0, 1)], recipe(2, 1));
    let copy = d
        .duplicate_objects(
            &[curvyo_document_core::CopySource {
                id: group,
                anchor_ids: (0..3).map(|n| AnchorId::new(88, n)).collect(),
            }],
            Vec2::new(0.0, 50.0),
        )
        .unwrap()[0];
    let stored = d.tiling(copy).unwrap();
    assert_eq!(stored.sources, vec![d.children(Some(copy))[0]]);
    // The copy re-applies on its own.
    let outcome = d
        .apply_tiling(&request(Some(copy), vec![], recipe(3, 1)), &mut minter())
        .unwrap();
    assert!(outcome.written);
    assert_eq!(d.children(Some(group)).len(), 2);
}

#[test]
fn a_deleted_source_offers_only_release() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let mut off = recipe(2, 1);
    off.keep_source = false;
    let group = tile(&d, vec![source], off);
    d.delete_objects(&[source]).unwrap();
    assert!(!d.tiling(group).unwrap().sources_live);
    assert_eq!(
        d.apply_tiling(&request(Some(group), vec![], off), &mut minter()),
        Err(TilingError::SourceMissing)
    );
}

#[test]
fn ungrouping_a_group_around_a_tiling_keeps_the_tiling() {
    let d = Document::new(1);
    let inner = tile(&d, vec![path(&d, 0.0, 1)], recipe(2, 1));
    let frame = path(&d, 80.0, 20);
    let outer = d.group_objects(&[inner, frame]).unwrap();
    d.ungroup(&[outer]).unwrap();
    assert!(d.tiling(inner).is_some());
    let keyed = d.children(Some(inner));
    assert_eq!(keyed.len(), 2);
    // An Apply finds its kept cells, so nothing is written.
    let outcome = d
        .apply_tiling(&request(Some(inner), vec![], recipe(2, 1)), &mut minter())
        .unwrap();
    assert!(!outcome.written);
}

#[test]
fn releasing_a_tiling_keeps_a_tiling_put_into_it() {
    let d = Document::new(1);
    let inner = tile(&d, vec![path(&d, 80.0, 20)], recipe(2, 1));
    let outer = tile(&d, vec![path(&d, 0.0, 1)], recipe(2, 1));
    d.move_nodes(&MovePlan {
        nodes: vec![inner],
        target: MoveTarget::FrontOf(Some(outer)),
    })
    .unwrap();
    d.release_tiling(outer).unwrap();
    assert!(d.tiling(outer).is_none());
    assert!(d.tiling(inner).is_some());
}

#[test]
fn a_re_applied_text_copy_keeps_its_lock() {
    let d = Document::new(1);
    let text = d
        .create_text(
            "Hi",
            &FontRef {
                family: "Source Sans 3".to_owned(),
                style: "Regular".to_owned(),
                face_hash: "c".repeat(64),
                face_index: 0,
                kind: FontKind::Outline,
            },
            Length::from_mm(10.0),
            Affine::translation(Vec2::new(0.0, 0.0)),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let mut copies = recipe(2, 1);
    copies.result = TilingResult::Copies;
    let group = tile(&d, vec![text], copies);
    let instance = d.children(Some(group))[1];
    d.set_node_locked(&[instance], true).unwrap();
    d.set_text_size(&[text], Length::from_mm(12.0)).unwrap();
    let outcome = d
        .apply_tiling(&request(Some(group), vec![], copies), &mut minter())
        .unwrap();
    assert!(outcome.written);
    assert!(d.node_registers(instance).unwrap().locked);
    let read = d.read_objects();
    let Some(ObjectSnapshot::Text(copy)) = read.get(instance) else {
        panic!("a text copy")
    };
    assert_eq!(copy.size, Length::from_mm(12.0));
}

#[test]
fn a_clone_of_a_removed_instance_is_repointed_to_the_source() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let group = tile(&d, vec![source], recipe(2, 1));
    let instance = d.children(Some(group))[1];
    let clone = d.linked_clone(&[instance], &[]).unwrap()[0];
    let outcome = d
        .apply_tiling(&request(Some(group), vec![], recipe(1, 1)), &mut minter())
        .unwrap();
    assert!(outcome.written);
    assert!(!d.children(Some(group)).contains(&instance));
    // As for any deleted original (`0049` criterion 30): it follows the next original that
    // stays, with the skipped matrix composed, so it draws where it did.
    let read = d.read_objects();
    let Some(ObjectSnapshot::Clone(repaired)) = read.get(clone) else {
        panic!("still a clone")
    };
    assert_eq!(repaired.chain.origin, source);
    assert_eq!(repaired.resolved.anchors[0].point, Point::new(15.0, 0.0));
    assert_eq!(repaired.resolved.anchors[2].point, Point::new(25.0, 10.0));
}

/// The anchor points of the clone `id` as drawn.
fn drawn_points(d: &Document, id: NodeId) -> Vec<Point> {
    let Some(ObjectSnapshot::Clone(clone)) = d.object(id) else {
        panic!("a clone instance")
    };
    clone.resolved.anchors.iter().map(|a| a.point).collect()
}

fn close_points(a: &[Point], b: &[Point]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| (p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9)
}

/// Criterion 29 (customer findings on PR #247): a scale or a rotation of the source reaches every
/// instance, each scaled or turned about its own centre in its cell; a move of the source alone
/// scatters none.
#[test]
fn a_scale_or_rotation_of_the_source_reaches_the_instances_and_a_move_does_not() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let group = tile(&d, vec![source], recipe(3, 1));
    let instance = d.children(Some(group))[1];
    let before = drawn_points(&d, instance);
    assert!(close_points(
        &before,
        &[
            Point::new(15.0, 0.0),
            Point::new(25.0, 0.0),
            Point::new(25.0, 10.0)
        ]
    ));
    let mapped = |g: Affine| {
        let Some(ObjectSnapshot::Path(path)) = d.object(source) else {
            panic!("a path source")
        };
        ObjectSnapshot::Path(path.mapped(g))
    };
    let _ = d.take_clone_hold();
    d.translate_objects(&[source], Vec2::new(0.0, 40.0))
        .unwrap();
    assert!(close_points(&drawn_points(&d, instance), &before), "move");
    assert_eq!(
        d.take_clone_hold().held,
        2,
        "the move notice counts the held instances"
    );
    let turn = Affine::rotation_about(Point::new(5.0, 45.0), Angle::from_radians(0.7));
    d.transform_objects(&[mapped(turn)], false).unwrap();
    let in_place = Affine::rotation_about(Point::new(20.0, 5.0), Angle::from_radians(0.7));
    let turned: Vec<Point> = before.iter().map(|p| in_place.apply_point(*p)).collect();
    assert!(
        close_points(&drawn_points(&d, instance), &turned),
        "rotation turns the instance about its own centre"
    );
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert!(
        close_points(&drawn_points(&d, instance), &before),
        "undo of the rotation"
    );
    // Twice the size about the source's top-left corner (0, 40).
    let scale = Affine::translation(Vec2::new(0.0, 40.0))
        .compose(Affine::scale(2.0, 2.0))
        .compose(Affine::translation(Vec2::new(0.0, -40.0)));
    d.transform_objects(&[mapped(scale)], false).unwrap();
    assert_eq!(
        d.take_clone_hold().held,
        0,
        "instances that follow a scale are not reported as stayed in place"
    );
    // The instance's centre (20, 5) stays; its outline is the source's, twice the size.
    assert!(
        close_points(
            &drawn_points(&d, instance),
            &[
                Point::new(10.0, -5.0),
                Point::new(30.0, -5.0),
                Point::new(30.0, 15.0)
            ]
        ),
        "{:?}",
        drawn_points(&d, instance)
    );
}

/// The held-back rule of a plain clone with own placement is unchanged (`0057` criterion 3):
/// a scale of its original keeps its outline.
#[test]
fn a_plain_own_placement_clone_stays_fully_held_on_a_scale() {
    let d = Document::new(1);
    let source = path(&d, 0.0, 1);
    let clone = d.linked_clone(&[source], &[]).unwrap()[0];
    d.set_own_placement(&[clone], true).unwrap();
    let before = drawn_points(&d, clone);
    let Some(ObjectSnapshot::Path(path)) = d.object(source) else {
        panic!("a path")
    };
    d.transform_objects(
        &[ObjectSnapshot::Path(path.mapped(Affine::scale(2.0, 2.0)))],
        false,
    )
    .unwrap();
    assert!(close_points(&drawn_points(&d, clone), &before));
}

/// A patterned source (`0071`): clone instances, clones with an own style from the hue variance
/// and copy instances all carry its pattern fill with the same generator and seed.
#[test]
fn instances_of_a_patterned_source_carry_its_pattern_fill() {
    use curvyo_document_core::{FillPaint, GeneratorKind, StyleEdit};
    let pattern_of = |d: &Document, id: NodeId| {
        let fill = d.object(id).unwrap().style().fill.clone();
        (fill.paint, fill.pattern)
    };
    for (result, hue) in [
        (TilingResult::Clones, 0.0),
        (TilingResult::Clones, 0.5),
        (TilingResult::Copies, 0.0),
    ] {
        let d = Document::new(1);
        let source = path(&d, 0.0, 1);
        d.edit_style(&[source], &StyleEdit::FillPaint(FillPaint::Pattern))
            .unwrap();
        d.edit_style(&[source], &StyleEdit::PatternGenerator(GeneratorKind::Dots))
            .unwrap();
        let mut recipe = recipe(2, 2);
        recipe.result = result;
        recipe.variance.hue.random = Angle::from_radians(hue);
        let mut request = request(None, vec![source], recipe);
        for cell in &mut request.cells {
            cell.hue = Angle::from_radians(hue);
        }
        let group = d.apply_tiling(&request, &mut minter()).unwrap().tiling;
        let expected = pattern_of(&d, source);
        assert_eq!(expected.0, FillPaint::Pattern);
        let instances: Vec<NodeId> = d
            .children(Some(group))
            .into_iter()
            .filter(|id| *id != source)
            .collect();
        assert_eq!(instances.len(), 3, "{result:?} hue {hue}");
        for id in instances {
            assert_eq!(pattern_of(&d, id), expected, "{result:?} hue {hue}");
        }
    }
}
