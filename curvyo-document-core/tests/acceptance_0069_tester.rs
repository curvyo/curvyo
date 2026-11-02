//! Tester's acceptance tests for `specs/0069-tiling` on the stored tiling (ADR 0030), written from
//! the specification before the implementation was read: refusals before the first write
//! (criterion 4), the result group and the kept source (22 to 24), the registers and the format
//! (25, 42), re-apply with minimal writes (27), hand edits (28), source edits and deletes (29,
//! 30), delete, duplicate and undo (31), release, ungroup and concurrent repair (32).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::large_types_passed_by_value,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::semicolon_if_nothing_returned,
    clippy::option_option
)]

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, Angle, CURRENT_FORMAT_VERSION, CellKey, CircleOptions, CopySource,
    DimensionKind, DimensionPlace, DimensionSlots, Document, FontKind, FontRef, GridOptions,
    Length, MAX_CLONE_DEPTH, MAX_GROUP_DEPTH, NewAnchor, NewImage, NodeId, ObjectSnapshot,
    OffsetLines, OpenError, Percent, Point, PointSlot, RectBounds, SpacingMode, StyleEdit,
    TILING_GROUP_NAME, TextLayoutValues, TilingCell, TilingError, TilingLayout, TilingRecipe,
    TilingRequest, TilingResult, Variance, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroMap, LoroValue, TreeID};

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn tri(d: &Document, x: f64, n: u64) -> NodeId {
    d.create_path(
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
            count: 6,
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

/// Translations by `pitch` per column and row (plus `shift` in x), with `opacity` and `hue`.
fn cells_with(r: &TilingRecipe, shift: f64, opacity: f64, hue: f64) -> Vec<TilingCell> {
    let (rows, columns) = match r.layout {
        TilingLayout::Grid => (r.grid.rows, r.grid.columns),
        TilingLayout::Circle => (1, r.circle.count),
    };
    let mut out = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let key = CellKey { row, column };
            if r.keep_source && key == CellKey::ORIGIN {
                continue;
            }
            out.push(TilingCell {
                key,
                affine: Affine::translation(Vec2::new(
                    f64::from(column) * 15.0 + shift,
                    f64::from(row) * 15.0,
                )),
                opacity: Percent::new(opacity),
                hue: Angle::from_radians(hue),
            });
        }
    }
    out
}

fn cells(r: &TilingRecipe) -> Vec<TilingCell> {
    cells_with(r, 0.0, 0.0, 0.0)
}

fn req(tiling: Option<NodeId>, sources: Vec<NodeId>, r: TilingRecipe) -> TilingRequest {
    TilingRequest {
        tiling,
        sources: if tiling.is_some() {
            Vec::new()
        } else {
            sources
        },
        cells: cells(&r),
        recipe: r,
        conversions: Vec::new(),
    }
}

fn minter() -> impl FnMut() -> AnchorId {
    let next = Cell::new(5000_u64);
    move || {
        next.set(next.get() + 1);
        AnchorId::new(77, next.get())
    }
}

fn tile(d: &Document, sources: Vec<NodeId>, r: TilingRecipe) -> NodeId {
    d.apply_tiling(&req(None, sources, r), &mut minter())
        .unwrap()
        .tiling
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn tree_id(id: NodeId) -> TreeID {
    let v = serde_json::to_value(id).unwrap();
    TreeID::new(
        v["peer"].as_str().unwrap().parse().unwrap(),
        v["counter"].as_i64().unwrap() as i32,
    )
}

fn loro_of(d: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro
}

fn meta(loro: &LoroDoc, id: NodeId) -> LoroMap {
    loro.get_tree("paths").get_meta(tree_id(id)).unwrap()
}

fn reg(loro: &LoroDoc, id: NodeId, key: &str) -> Option<LoroValue> {
    meta(loro, id).get(key).map(|v| v.get_deep_value())
}

fn cell_of(loro: &LoroDoc, id: NodeId) -> Option<(i64, i64)> {
    match reg(loro, id, "tiling_cell")? {
        LoroValue::List(l) => match (&l[0], &l[1]) {
            (LoroValue::I64(a), LoroValue::I64(b)) => Some((*a, *b)),
            _ => None,
        },
        _ => None,
    }
}

fn clone_of(d: &Document, id: NodeId) -> curvyo_document_core::CloneSnapshot {
    match d.read_objects().get(id) {
        Some(ObjectSnapshot::Clone(c)) => c.clone(),
        other => panic!("not a clone: {other:?}"),
    }
}

/// The container (a layer, or `None` for the top level) that holds `id` directly.
fn layer_of(d: &Document, id: NodeId) -> Option<Option<NodeId>> {
    if d.children(None).contains(&id) {
        return Some(None);
    }
    d.children(None)
        .into_iter()
        .find(|layer| d.children(Some(*layer)).contains(&id))
        .map(Some)
}

fn all_top(d: &Document) -> Vec<NodeId> {
    d.children(None)
        .into_iter()
        .flat_map(|l| {
            let mut v = vec![l];
            v.extend(d.children(Some(l)));
            v
        })
        .collect()
}

/// Children of a tiling keyed by their cell.
fn keyed(d: &Document, group: NodeId) -> BTreeMap<(i64, i64), NodeId> {
    let loro = loro_of(d);
    d.children(Some(group))
        .into_iter()
        .filter_map(|c| cell_of(&loro, c).map(|k| (k, c)))
        .collect()
}

fn snapshot(d: &Document) -> Vec<u8> {
    pack(d, "t").unwrap()
}

/// The whole drawing as JSON, for "exactly as before" comparisons.
fn drawing(d: &Document) -> String {
    String::from_utf8(d.export_json().unwrap()).unwrap()
}

// ---------------------------------------------------------------------------------------------
// A. Refusals before the first write (criterion 4)
// ---------------------------------------------------------------------------------------------

fn refused(d: &Document, request: &TilingRequest, want: &TilingError) {
    let version = d.version();
    let steps = d.own_steps().len();
    let got = d.apply_tiling(request, &mut minter());
    assert_eq!(
        got.as_ref().err(),
        Some(want),
        "{got:?} (want {want:?}, cells {:?})",
        request.cells.iter().map(|c| c.key).collect::<Vec<_>>()
    );
    assert_eq!(d.version(), version, "nothing written on {want:?}");
    assert_eq!(d.own_steps().len(), steps);
}

#[test]
fn c04_a_source_of_only_dimensions_is_refused() {
    let d = Document::new(1);
    let p = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(30.0, 0.0)),
        ],
        false,
    );
    let dim = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Anchor {
                    node: p,
                    anchor: AnchorId::new(1, 1),
                },
                PointSlot::Anchor {
                    node: p,
                    anchor: AnchorId::new(1, 2),
                },
            ]),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap();
    refused(
        &d,
        &req(None, vec![dim], recipe(2, 1)),
        &TilingError::EmptySource,
    );
    refused(
        &d,
        &req(None, vec![], recipe(2, 1)),
        &TilingError::EmptySource,
    );
    // With a path too, the dimension is skipped and never tiled.
    let g = tile(&d, vec![p, dim], recipe(3, 1));
    let read = d.read_objects();
    for child in d.children(Some(g)) {
        if child == p || child == dim {
            continue;
        }
        assert!(
            !matches!(read.get(child), Some(ObjectSnapshot::Dimension(_))),
            "a dimension was tiled"
        );
        assert!(
            d.children(Some(child)).is_empty(),
            "one leaf: a bare clone, no group"
        );
    }
}

#[test]
fn c04_a_locked_source_or_locked_layer_is_refused() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    d.set_node_locked(&[a], true).unwrap();
    refused(&d, &req(None, vec![a], recipe(2, 1)), &TilingError::Locked);
    d.set_node_locked(&[a], false).unwrap();
    let layer = d.new_layer(None);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![a],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    assert_eq!(layer_of(&d, a), Some(Some(layer)));
    d.set_node_locked(&[layer], true).unwrap();
    refused(&d, &req(None, vec![a], recipe(2, 1)), &TilingError::Locked);
}

#[test]
fn c04_a_clone_chain_at_its_limit_is_refused() {
    let d = Document::new(1);
    let mut top = tri(&d, 0.0, 1);
    for _ in 0..MAX_CLONE_DEPTH {
        top = d.linked_clone(&[top], &[]).unwrap()[0];
    }
    assert_eq!(clone_of(&d, top).chain.depth, MAX_CLONE_DEPTH);
    refused(
        &d,
        &req(None, vec![top], recipe(2, 1)),
        &TilingError::CloneTooDeep,
    );
}

#[test]
fn c04_an_orphan_clone_is_refused() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let c = d.linked_clone(&[a], &[]).unwrap()[0];
    // Deleting the original repairs or orphans the clone; either way tiling must not break.
    d.delete_objects(&[a]).unwrap();
    let read = d.read_objects();
    if matches!(read.get(c), Some(ObjectSnapshot::Clone(_))) {
        // Still an (orphan) clone: refused.
        refused(
            &d,
            &req(None, vec![c], recipe(2, 1)),
            &TilingError::CloneTooDeep,
        );
    } else {
        // Repaired into a plain path: tiles.
        assert!(
            d.apply_tiling(&req(None, vec![c], recipe(2, 1)), &mut minter())
                .is_ok()
        );
    }
}

#[test]
fn c04_too_many_instances_and_bad_recipes_are_refused() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut r = recipe(100_001, 1);
    let request = TilingRequest {
        tiling: None,
        sources: vec![a],
        recipe: r,
        cells: Vec::new(),
        conversions: Vec::new(),
    };
    refused(
        &d,
        &request,
        &TilingError::Recipe(curvyo_document_core::RecipeError::TooMany),
    );
    r = recipe(2, 1);
    r.grid.spacing_x = Length::from_mm(f64::NAN);
    refused(
        &d,
        &req(None, vec![a], r),
        &TilingError::Recipe(curvyo_document_core::RecipeError::NotFinite),
    );
    r = recipe(0, 1);
    let mut request = req(None, vec![a], r);
    request.cells.clear();
    refused(
        &d,
        &request,
        &TilingError::Recipe(curvyo_document_core::RecipeError::CountZero),
    );
    r = recipe(2, 1);
    r.circle.radius = Length::from_mm(-1.0);
    refused(
        &d,
        &req(None, vec![a], r),
        &TilingError::Recipe(curvyo_document_core::RecipeError::OutOfRange),
    );
}

#[test]
fn c04_cells_that_do_not_fit_the_recipe_are_refused() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut request = req(None, vec![a], recipe(3, 1));
    let dup = request.cells[0];
    request.cells.push(dup);
    refused(&d, &request, &TilingError::Cells);
    let mut request = req(None, vec![a], recipe(3, 1));
    request.cells.push(TilingCell {
        key: CellKey::ORIGIN,
        affine: Affine::IDENTITY,
        opacity: Percent::new(0.0),
        hue: Angle::from_radians(0.0),
    });
    refused(&d, &request, &TilingError::Cells);
    // A cell outside the layout.
    let mut request = req(None, vec![a], recipe(3, 1));
    request.cells[0].key = CellKey { row: 5, column: 0 };
    refused(&d, &request, &TilingError::Cells);
}

#[test]
fn c04_a_non_finite_cell_affine_is_refused() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut request = req(None, vec![a], recipe(3, 1));
    request.cells[0].affine = Affine::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0);
    let version = d.version();
    assert!(
        d.apply_tiling(&request, &mut minter()).is_err(),
        "NaN affine accepted"
    );
    assert_eq!(d.version(), version);
}

#[test]
fn c04_a_source_that_contains_a_tiling_is_refused() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let g = tile(&d, vec![a], recipe(2, 1));
    let b = tri(&d, 100.0, 10);
    refused(
        &d,
        &req(None, vec![g, b], recipe(2, 1)),
        &TilingError::ContainsTiling,
    );
    let outer = d.group_objects(&[g, b]).unwrap();
    refused(
        &d,
        &req(None, vec![outer], recipe(2, 1)),
        &TilingError::ContainsTiling,
    );
}

#[test]
fn c04_groups_nested_too_deep_are_refused() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let b = tri(&d, 20.0, 10);
    // Nest the two-leaf source as deep as groups may go; the tiling group plus an instance group
    // on top must exceed the limit.
    let mut top = d.group_objects(&[a, b]).unwrap();
    let mut depth = 1;
    let mut n = 100;
    while depth < MAX_GROUP_DEPTH {
        let filler = tri(&d, 500.0, n);
        n += 10;
        top = d.group_objects(&[top, filler]).unwrap();
        depth += 1;
    }
    assert_eq!(
        depth, MAX_GROUP_DEPTH,
        "a and b sit under the deepest allowed nesting"
    );
    refused(
        &d,
        &req(None, vec![a, b], recipe(2, 1)),
        &TilingError::TooDeep,
    );
}

// ---------------------------------------------------------------------------------------------
// E. Result and organisation (criteria 22 to 24)
// ---------------------------------------------------------------------------------------------

#[test]
fn c22_keep_source_moves_the_source_into_the_tiling_as_its_lowest_child() {
    let d = Document::new(1);
    let below = tri(&d, -50.0, 40);
    let a = tri(&d, 0.0, 1);
    let above = tri(&d, 50.0, 20);
    let layer = layer_of(&d, a).unwrap();
    let steps = d.own_steps().len();
    let g = tile(&d, vec![a], recipe(3, 2));
    assert_eq!(d.own_steps().len(), steps + 1, "one step");
    assert_eq!(last_label(&d), "tile");
    assert!(d.is_group(g));
    assert_eq!(
        d.node_registers(g).unwrap().name.as_deref(),
        Some(TILING_GROUP_NAME)
    );
    assert_eq!(layer_of(&d, g), Some(layer), "in the source's layer");
    let children = d.children(Some(g));
    assert_eq!(children[0], a, "the source is the lowest child");
    assert_eq!(children.len(), 6, "source + 5 generated");
    let loro = loro_of(&d);
    assert_eq!(
        cell_of(&loro, a),
        None,
        "the kept source carries no cell key"
    );
    let keys: Vec<_> = children[1..]
        .iter()
        .map(|c| cell_of(&loro, *c).unwrap())
        .collect();
    assert!(!keys.contains(&(0, 0)), "cell (0,0) is not generated");
    // z-order: the tiling takes the source's place between its neighbours.
    let order = d.children(layer);
    let pos = |id| order.iter().position(|x| *x == id).unwrap();
    assert!(pos(below) < pos(g) && pos(g) < pos(above));
}

#[test]
fn c22_without_keep_source_the_sources_stay_and_the_tiling_sits_above_the_topmost() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mid = tri(&d, 200.0, 40);
    let b = tri(&d, 20.0, 10);
    let top = tri(&d, 300.0, 20);
    let layer = layer_of(&d, a).unwrap();
    let before_a = clone_free_points(&d, a);
    let mut r = recipe(2, 2);
    r.keep_source = false;
    let g = tile(&d, vec![a, b], r);
    let order = d.children(layer);
    let pos = |id| order.iter().position(|x| *x == id).unwrap();
    assert!(
        order.contains(&a) && order.contains(&b),
        "sources stay in the layer"
    );
    assert_eq!(pos(g), pos(b) + 1, "directly above the topmost source");
    assert!(pos(mid) < pos(g) && pos(g) < pos(top));
    assert_eq!(clone_free_points(&d, a), before_a, "sources untouched");
    assert_eq!(
        d.children(Some(g)).len(),
        4,
        "every cell generated, (0,0) too"
    );
    assert!(keyed(&d, g).contains_key(&(0, 0)));
}

fn clone_free_points(d: &Document, id: NodeId) -> Vec<(f64, f64)> {
    d.read_objects()
        .get(id)
        .unwrap()
        .drawn_path()
        .unwrap()
        .anchors
        .iter()
        .map(|a| (a.point.x, a.point.y))
        .collect()
}

#[test]
fn c23_clones_have_own_placement_and_the_cell_affine_as_matrix() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let r = recipe(3, 1);
    let g = tile(&d, vec![a], r);
    for (key, id) in keyed(&d, g) {
        let c = clone_of(&d, id);
        assert_eq!(c.chain.origin, a);
        assert!(c.chain.own_placement, "own placement");
        assert!(!c.chain.own_style, "no own style without style variance");
        let want = Affine::translation(Vec2::new(key.1 as f64 * 15.0, 0.0));
        assert_eq!(c.chain.matrix, want);
    }
    // Moving the source alone does not scatter the instances.
    let before: Vec<_> = keyed(&d, g)
        .values()
        .map(|id| clone_of(&d, *id).resolved.anchors[0].point)
        .collect();
    d.translate_objects(&[a], Vec2::new(7.0, 3.0)).unwrap();
    let after: Vec<_> = keyed(&d, g)
        .values()
        .map(|id| clone_of(&d, *id).resolved.anchors[0].point)
        .collect();
    assert_eq!(before, after);
}

#[test]
fn c23_primitives_are_converted_in_the_same_commit() {
    let d = Document::new(1);
    let rect = d.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(5.0),
    });
    let anchors: Vec<NewAnchor> = [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (0.0, 5.0)]
        .iter()
        .enumerate()
        .map(|(i, (x, y))| NewAnchor::corner(AnchorId::new(9, i as u64), Point::new(*x, *y)))
        .collect();
    let r = recipe(2, 1);
    let mut request = req(None, vec![rect], r);
    // Missing conversion: refused.
    refused(&d, &request, &TilingError::Conversions);
    request.conversions = vec![(rect, anchors)];
    let steps = d.own_steps().len();
    let g = d.apply_tiling(&request, &mut minter()).unwrap().tiling;
    assert_eq!(
        d.own_steps().len(),
        steps + 1,
        "conversion and tile in one commit"
    );
    let read = d.read_objects();
    assert!(
        matches!(read.get(rect), Some(ObjectSnapshot::Path(_))),
        "converted to a path"
    );
    for id in keyed(&d, g).values() {
        assert_eq!(clone_of(&d, *id).chain.origin, rect);
    }
}

#[test]
fn c23_copies_make_independent_leaves_mapped_by_the_affine() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut r = recipe(3, 1);
    r.result = TilingResult::Copies;
    let g = tile(&d, vec![a], r);
    let read = d.read_objects();
    for (key, id) in keyed(&d, g) {
        let Some(ObjectSnapshot::Path(p)) = read.get(id) else {
            panic!("a path copy")
        };
        let dx = key.1 as f64 * 15.0;
        let pts: Vec<_> = p.anchors.iter().map(|a| (a.point.x, a.point.y)).collect();
        assert_eq!(pts, vec![(dx, 0.0), (dx + 10.0, 0.0), (dx + 10.0, 10.0)]);
    }
    // A source edit does not flow into copies.
    let copy = keyed(&d, g)[&(0, 1)];
    let before = clone_free_points(&d, copy);
    d.move_anchors(&[(a, AnchorId::new(7, 1), Point::new(-5.0, -5.0))])
        .unwrap();
    assert_eq!(clone_free_points(&d, copy), before);
    // The next Apply refreshes them.
    d.apply_tiling(&req(Some(g), vec![], r), &mut minter())
        .unwrap();
    let copy = keyed(&d, g)[&(0, 1)];
    assert_eq!(clone_free_points(&d, copy)[0], (10.0, -5.0));
}

#[test]
fn c29_a_source_edit_flows_into_clone_instances() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let g = tile(&d, vec![a], recipe(3, 1));
    d.move_anchors(&[(a, AnchorId::new(7, 1), Point::new(-5.0, -5.0))])
        .unwrap();
    let c = clone_of(&d, keyed(&d, g)[&(0, 2)]);
    assert_eq!(c.resolved.anchors[0].point, Point::new(25.0, -5.0));
    // Style edits flow too, while the instance has no own style.
    d.edit_style(&[a], &StyleEdit::StrokeWidth(Length::from_mm(2.5)))
        .unwrap();
    let c = clone_of(&d, keyed(&d, g)[&(0, 2)]);
    assert_eq!(c.resolved.style.stroke.width.as_mm(), 2.5);
}

#[test]
fn c23_c24_multi_source_with_image_and_text() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let img = d
        .add_images(&[NewImage {
            bytes: Arc::from(&b"tester picture bytes"[..]),
            placement: Affine::new(10.0, 0.0, 0.0, 10.0, 20.0, 0.0),
            name: "p.png".to_owned(),
        }])
        .unwrap()[0];
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
            Affine::translation(Vec2::new(40.0, 0.0)),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let mut r = recipe(2, 2);
    r.keep_source = true;
    let g = tile(&d, vec![a, img, text], r);
    let read = d.read_objects();
    let loro = loro_of(&d);
    let instances = keyed(&d, g);
    assert_eq!(instances.len(), 3);
    for id in instances.values() {
        assert!(d.is_group(*id), "a group of members");
        let members = d.children(Some(*id));
        assert_eq!(members.len(), 3);
        let member_of = |m: NodeId| match reg(&loro, m, "tiling_member") {
            Some(LoroValue::String(_)) => true,
            other => panic!("member key missing: {other:?}"),
        };
        assert!(members.iter().all(|m| member_of(*m)));
        // In the sources' z-order: clone of path, image copy, text clone.
        match read.get(members[0]) {
            Some(ObjectSnapshot::Clone(c)) => {
                assert_eq!(c.chain.origin, a);
                assert!(c.chain.own_placement);
            }
            other => panic!("path member: {other:?}"),
        }
        match read.get(members[1]) {
            Some(ObjectSnapshot::Image(i)) => {
                let src = match read.get(img) {
                    Some(ObjectSnapshot::Image(s)) => s.asset.clone(),
                    _ => panic!(),
                };
                assert_eq!(i.asset, src, "the copy keeps the asset");
            }
            other => panic!("image member: {other:?}"),
        }
        match read.get(members[2]) {
            Some(ObjectSnapshot::Text(t)) => {
                assert_eq!(
                    t.source.as_ref().map(|s| s.node),
                    Some(text),
                    "a linked text clone"
                );
            }
            other => panic!("text member: {other:?}"),
        }
    }
}

#[test]
fn c23_sketches_tile_as_linked_clones() {
    let d = Document::new(1);
    let graph = curvyo_sketch_core::create_rectangle(
        &curvyo_sketch_core::Sketch::new("origin".into()),
        "plate",
        curvyo_sketch_core::Position::mm(0.0, 0.0),
        curvyo_sketch_core::Position::mm(20.0, 10.0),
    )
    .unwrap();
    let s = d
        .create_sketch(
            &graph,
            Affine::IDENTITY,
            &curvyo_document_core::Style::default(),
        )
        .unwrap();
    let g = tile(&d, vec![s], recipe(2, 1));
    let id = keyed(&d, g)[&(0, 1)];
    let c = clone_of(&d, id);
    assert_eq!(c.chain.origin, s);
}

// ---------------------------------------------------------------------------------------------
// D. Own style only with style variance (criterion 18)
// ---------------------------------------------------------------------------------------------

#[test]
fn c18_hue_variance_gives_own_style_and_leaves_the_source_untouched() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    d.edit_style(&[a], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(
        &[a],
        &StyleEdit::FillColor(curvyo_document_core::Color { r: 255, g: 0, b: 0 }),
    )
    .unwrap();
    let source_style = d.read_objects().get(a).unwrap().style().clone();
    let mut r = recipe(3, 1);
    r.variance.hue.random = Angle::from_radians(1.0);
    let mut request = req(None, vec![a], r);
    request.cells = cells_with(&r, 0.0, 0.0, std::f64::consts::FRAC_PI_2 * 4.0 / 3.0);
    let g = d.apply_tiling(&request, &mut minter()).unwrap().tiling;
    assert_eq!(d.read_objects().get(a).unwrap().style(), &source_style);
    for id in keyed(&d, g).values() {
        let c = clone_of(&d, *id);
        assert!(c.chain.own_style, "own style with hue variance");
        // A 120 degree shift turns red into green.
        let fill = c.resolved.style.fill.color;
        assert_eq!((fill.r, fill.g, fill.b), (0, 255, 0), "hue shifted fill");
    }
    // It stops following the source's style edits until the next Apply.
    d.edit_style(&[a], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    let c = clone_of(&d, keyed(&d, g)[&(0, 1)]);
    assert_ne!(c.resolved.style.stroke.width.as_mm(), 3.0);
    d.apply_tiling(
        &TilingRequest {
            tiling: Some(g),
            sources: vec![],
            ..request.clone()
        },
        &mut minter(),
    )
    .unwrap();
    let c = clone_of(&d, keyed(&d, g)[&(0, 1)]);
    assert_eq!(
        c.resolved.style.stroke.width.as_mm(),
        3.0,
        "refreshed by Apply"
    );
}

#[test]
fn c18_opacity_variance_is_clamped_to_0_to_100_percent() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut r = recipe(3, 1);
    r.variance.opacity.per_column = Percent::new(-80.0);
    let mut request = req(None, vec![a], r);
    request.cells = cells_with(&r, 0.0, -150.0, 0.0);
    let g = d.apply_tiling(&request, &mut minter()).unwrap().tiling;
    for id in keyed(&d, g).values() {
        let c = clone_of(&d, *id);
        assert!(c.chain.own_style);
        let o = d.node_registers(*id).unwrap().opacity.get();
        assert!((0.0..=1.0).contains(&o), "opacity {o}");
        assert_eq!(o, 0.0, "1.0 - 1.5 clamps to 0");
    }
    let mut up = request.clone();
    up.tiling = Some(g);
    up.sources = vec![];
    up.cells = cells_with(&r, 0.0, 50.0, 0.0);
    d.apply_tiling(&up, &mut minter()).unwrap();
    for id in keyed(&d, g).values() {
        assert_eq!(
            d.node_registers(*id).unwrap().opacity.get(),
            1.0,
            "clamped at 100 %"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// F. Stored tiling: registers, re-apply, hand edits (criteria 25, 27, 28)
// ---------------------------------------------------------------------------------------------

#[test]
fn c25_the_group_stores_recipe_and_sources_and_instances_their_cell() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let b = tri(&d, 20.0, 10);
    let mut r = recipe(2, 3);
    r.seed = u32::MAX;
    r.variance.rotation.per_row = Angle::from_radians(0.25);
    r.grid.mirror_columns = true;
    r.grid.offset = OffsetLines::Columns;
    let g = tile(&d, vec![a, b], r);
    let stored = d.tiling(g).unwrap();
    assert_eq!(stored.recipe, r, "the whole recipe, seed u32::MAX included");
    assert_eq!(stored.sources, vec![a, b]);
    assert!(stored.sources_live);
    let loro = loro_of(&d);
    assert!(reg(&loro, g, "tiling").is_some());
    assert!(matches!(
        reg(&loro, g, "tiling_sources"),
        Some(LoroValue::List(_))
    ));
    let mut keys: Vec<_> = keyed(&d, g).keys().copied().collect();
    keys.sort_unstable();
    assert_eq!(keys, vec![(0, 1), (1, 0), (1, 1), (2, 0), (2, 1)]);
    // Survives save and reopen with the same recipe and keys.
    let reopened = unpack(2, &snapshot(&d)).unwrap();
    assert_eq!(reopened.tiling(g).unwrap().recipe, r);
    let mut k2: Vec<_> = keyed(&reopened, g).keys().copied().collect();
    k2.sort_unstable();
    assert_eq!(k2, keys);
}

#[test]
fn c25_format_version_is_33_and_a_newer_file_is_refused() {
    // `0071` took 32 for pattern fills before this slice merged.
    assert_eq!(CURRENT_FORMAT_VERSION, 33);
    let d = Document::new(1);
    tile(&d, vec![tri(&d, 0.0, 1)], recipe(2, 1));
    let bytes = snapshot(&d);
    let manifest = read_member(&bytes, "manifest.json");
    let v: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(v["format_version"].as_u64(), Some(33));
    // A build one version older is simulated by the newer-version check: a 34 is refused here.
    let mut newer = v.clone();
    newer["format_version"] = 34.into();
    let bumped = replace_member(
        &bytes,
        "manifest.json",
        &serde_json::to_vec(&newer).unwrap(),
    );
    assert!(matches!(
        unpack(2, &bumped),
        Err(OpenError::FormatTooNew { .. })
    ));
}

#[test]
fn c25_documents_without_tilings_open_unchanged() {
    let d = Document::new(1);
    tri(&d, 0.0, 1);
    let reopened = unpack(2, &snapshot(&d)).unwrap();
    assert_eq!(drawing(&reopened), drawing(&d));
}

fn read_member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn replace_member(bytes: &[u8], name: &str, with: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let n = file.name().unwrap().to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        writer.start_file(&n, options).unwrap();
        writer
            .write_all(if n == name { with } else { &data })
            .unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A saved tiling whose raw registers `edit` changes.
fn damaged(edit: impl Fn(&LoroDoc, NodeId, &[NodeId])) -> Vec<u8> {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let b = tri(&d, 20.0, 10);
    let g = tile(&d, vec![a, b], recipe(2, 2));
    let instances: Vec<NodeId> = keyed(&d, g).values().copied().collect();
    let bytes = snapshot(&d);
    let loro = loro_of(&d);
    loro.set_peer_id(9).unwrap();
    edit(&loro, g, &instances);
    loro.commit();
    replace_member(
        &bytes,
        "document.loro",
        &loro.export(loro::ExportMode::Snapshot).unwrap(),
    )
}

fn set_recipe_field(loro: &LoroDoc, g: NodeId, section: Option<&str>, field: &str, v: LoroValue) {
    let m = meta(loro, g);
    let LoroValue::Map(root) = m.get("tiling").unwrap().get_deep_value() else {
        panic!("recipe is a map value")
    };
    let mut root: HashMap<String, LoroValue> =
        root.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    match section {
        None => {
            root.insert(field.to_owned(), v);
        }
        Some(s) => {
            let LoroValue::Map(inner) = root[s].clone() else {
                panic!("{s}")
            };
            let mut inner: HashMap<String, LoroValue> =
                inner.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            inner.insert(field.to_owned(), v);
            root.insert(s.to_owned(), LoroValue::Map(inner.into()));
        }
    }
    m.insert("tiling", LoroValue::Map(root.into())).unwrap();
}

#[test]
fn c25_malformed_registers_make_the_file_damaged() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "cell with three parts",
            damaged(|l, _, inst| {
                meta(l, inst[0])
                    .insert(
                        "tiling_cell",
                        LoroValue::List(vec![1_i64.into(), 1_i64.into(), 1_i64.into()].into()),
                    )
                    .unwrap();
            }),
        ),
        (
            "negative cell",
            damaged(|l, _, inst| {
                meta(l, inst[0])
                    .insert(
                        "tiling_cell",
                        LoroValue::List(vec![(-1_i64).into(), 0_i64.into()].into()),
                    )
                    .unwrap();
            }),
        ),
        (
            "cell beyond u32",
            damaged(|l, _, inst| {
                meta(l, inst[0])
                    .insert(
                        "tiling_cell",
                        LoroValue::List(vec![0_i64.into(), (1_i64 << 33).into()].into()),
                    )
                    .unwrap();
            }),
        ),
        (
            "cell as text",
            damaged(|l, _, inst| {
                meta(l, inst[0]).insert("tiling_cell", "0,1").unwrap();
            }),
        ),
        (
            "member as number",
            damaged(|l, _, inst| {
                let m = l
                    .get_tree("paths")
                    .children(Some(tree_id(inst[0])))
                    .unwrap()[0];
                l.get_tree("paths")
                    .get_meta(m)
                    .unwrap()
                    .insert("tiling_member", 3_i64)
                    .unwrap();
            }),
        ),
        (
            "member not an id",
            damaged(|l, _, inst| {
                let m = l
                    .get_tree("paths")
                    .children(Some(tree_id(inst[0])))
                    .unwrap()[0];
                l.get_tree("paths")
                    .get_meta(m)
                    .unwrap()
                    .insert("tiling_member", "nonsense")
                    .unwrap();
            }),
        ),
        (
            "sources with a number",
            damaged(|l, g, _| {
                meta(l, g)
                    .insert("tiling_sources", LoroValue::List(vec![5_i64.into()].into()))
                    .unwrap();
            }),
        ),
        (
            "recipe on a leaf",
            damaged(|l, g, inst| {
                let v = meta(l, g).get("tiling").unwrap().get_deep_value();
                let m = l
                    .get_tree("paths")
                    .children(Some(tree_id(inst[0])))
                    .unwrap()[0];
                l.get_tree("paths")
                    .get_meta(m)
                    .unwrap()
                    .insert("tiling", v)
                    .unwrap();
            }),
        ),
        (
            "recipe not a map",
            damaged(|l, g, _| {
                meta(l, g).insert("tiling", "grid").unwrap();
            }),
        ),
        (
            "unknown layout",
            damaged(|l, g, _| set_recipe_field(l, g, None, "layout", "hexagon".into())),
        ),
        (
            "unknown result",
            damaged(|l, g, _| set_recipe_field(l, g, None, "result", "stamps".into())),
        ),
        (
            "rows 0",
            damaged(|l, g, _| set_recipe_field(l, g, Some("grid"), "rows", 0_i64.into())),
        ),
        (
            "circle over limit",
            damaged(|l, g, _| set_recipe_field(l, g, Some("circle"), "count", 100_001_i64.into())),
        ),
        (
            "grid product over limit",
            damaged(|l, g, _| {
                set_recipe_field(l, g, Some("grid"), "rows", 1000_i64.into());
                set_recipe_field(l, g, Some("grid"), "columns", 1000_i64.into());
            }),
        ),
        (
            "infinite radius",
            damaged(|l, g, _| {
                set_recipe_field(l, g, Some("circle"), "radius", f64::INFINITY.into())
            }),
        ),
        (
            "NaN shift",
            damaged(|l, g, _| set_recipe_field(l, g, Some("grid"), "shift", f64::NAN.into())),
        ),
        (
            "seed negative",
            damaged(|l, g, _| set_recipe_field(l, g, None, "seed", (-1_i64).into())),
        ),
        (
            "seed over u32",
            damaged(|l, g, _| set_recipe_field(l, g, None, "seed", (1_i64 << 32).into())),
        ),
    ];
    for (name, bytes) in cases {
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}: {:?}",
            unpack(2, &bytes).map(|_| ())
        );
    }
}

#[test]
fn c25_tolerated_residue_opens() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "dangling source",
            damaged(|l, g, _| {
                let LoroValue::List(list) =
                    meta(l, g).get("tiling_sources").unwrap().get_deep_value()
                else {
                    panic!()
                };
                let mut v = list.to_vec();
                v.push("999999:3".into());
                meta(l, g).insert("tiling_sources", v).unwrap();
            }),
        ),
        (
            "missing key",
            damaged(|l, _, inst| {
                meta(l, inst[0]).delete("tiling_cell").unwrap();
            }),
        ),
        (
            "duplicate key",
            damaged(|l, _, inst| {
                let v = meta(l, inst[0])
                    .get("tiling_cell")
                    .unwrap()
                    .get_deep_value();
                meta(l, inst[1]).insert("tiling_cell", v).unwrap();
            }),
        ),
        (
            "keyed children of a group without recipe",
            damaged(|l, g, _| {
                meta(l, g).delete("tiling").unwrap();
                meta(l, g).delete("tiling_sources").unwrap();
            }),
        ),
    ];
    for (name, bytes) in cases {
        let opened = unpack(2, &bytes);
        assert!(opened.is_ok(), "{name}: {:?}", opened.map(|_| ()));
        let d = opened.unwrap();
        let version = d.version();
        let _ = d.read_objects();
        assert_eq!(d.version(), version, "{name}: reading writes nothing");
    }
}

#[test]
fn c42_shipped_goldens_open_and_damage_files_are_refused() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let golden = std::fs::read(dir.join("tiling_v33.curvyo")).unwrap();
    let d = unpack(2, &golden).unwrap();
    let tilings: Vec<NodeId> = all_top(&d)
        .into_iter()
        .filter(|id| d.tiling(*id).is_some())
        .collect();
    let recipes: Vec<TilingRecipe> = tilings
        .iter()
        .map(|t| d.tiling(*t).unwrap().recipe)
        .collect();
    assert!(
        recipes.iter().any(|r| r.layout == TilingLayout::Circle),
        "a circle"
    );
    assert!(
        recipes
            .iter()
            .any(|r| r.variance.hue.random.as_radians() != 0.0
                || r.variance.hue.per_row.as_radians() != 0.0
                || r.variance.hue.per_column.as_radians() != 0.0),
        "hue variance"
    );
    assert!(
        recipes.iter().any(|r| r.result == TilingResult::Copies),
        "a copies tiling"
    );
    assert!(recipes.iter().any(|r| r.keep_source) && recipes.iter().any(|r| !r.keep_source));
    let read = d.read_objects();
    let has_image_copy = tilings.iter().any(|t| {
        d.children(Some(*t)).iter().any(|i| {
            d.children(Some(*i))
                .iter()
                .any(|m| matches!(read.get(*m), Some(ObjectSnapshot::Image(_))))
        })
    });
    assert!(has_image_copy, "a multi-source tiling with an image copy");
    let mut damaged = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if !name.starts_with("tiling_") || !name.ends_with("_v33.curvyo") {
            continue;
        }
        let bytes = std::fs::read(dir.join(&name)).unwrap();
        if name == "tiling_v33.curvyo" || name == "tiling_residue_v33.curvyo" {
            assert!(unpack(2, &bytes).is_ok(), "{name}");
        } else {
            assert!(
                matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
                "{name}"
            );
            damaged += 1;
        }
    }
    assert!(damaged >= 9, "one damage file per rule: {damaged}");
}

#[test]
fn c27_an_unchanged_apply_writes_nothing() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut r = recipe(3, 2);
    r.variance.hue.random = Angle::from_radians(0.4);
    let mut request = req(None, vec![a], r);
    request.cells = cells_with(&r, 0.0, -10.0, 0.4);
    let g = d.apply_tiling(&request, &mut minter()).unwrap().tiling;
    let version = d.version();
    let steps = d.own_steps().len();
    request.tiling = Some(g);
    request.sources = vec![];
    let out = d.apply_tiling(&request, &mut minter()).unwrap();
    assert!(!out.written);
    assert_eq!(d.version(), version);
    assert_eq!(d.own_steps().len(), steps);
    // Differences below 1e-9 mm write nothing either.
    for c in &mut request.cells {
        c.affine.e += 1e-11;
    }
    assert!(!d.apply_tiling(&request, &mut minter()).unwrap().written);
    assert_eq!(d.version(), version);
}

#[test]
fn c27_reapply_keeps_ids_rewrites_only_changes_creates_and_deletes() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let g = tile(&d, vec![a], recipe(3, 2));
    let before = keyed(&d, g);
    let steps = d.own_steps().len();
    // 4 columns, 1 row: (0,1),(0,2) stay, (0,3) is new, row 1 leaves.
    let r = recipe(4, 1);
    let out = d
        .apply_tiling(&req(Some(g), vec![], r), &mut minter())
        .unwrap();
    assert!(out.written);
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(last_label(&d), "edit_tiling");
    let after = keyed(&d, g);
    let mut keys: Vec<_> = after.keys().copied().collect();
    keys.sort_unstable();
    assert_eq!(keys, vec![(0, 1), (0, 2), (0, 3)]);
    assert_eq!(after[&(0, 1)], before[&(0, 1)], "kept id");
    assert_eq!(after[&(0, 2)], before[&(0, 2)], "kept id");
    for gone in [(1, 0), (1, 1), (1, 2)] {
        assert!(
            d.read_objects().get(before[&gone]).is_none(),
            "{gone:?} deleted"
        );
    }
    assert_eq!(d.tiling(g).unwrap().recipe, r);
    // Undo restores the earlier layout in one step.
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    let mut back: Vec<_> = keyed(&d, g).keys().copied().collect();
    back.sort_unstable();
    assert_eq!(back.len(), 5);
    assert_eq!(d.tiling(g).unwrap().recipe, recipe(3, 2));
}

#[test]
fn c27_switching_layout_keeps_variance_seed_and_last_values() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut r = recipe(3, 2);
    r.variance.rotation.random = Angle::from_radians(0.2);
    r.seed = 1234;
    r.circle.count = 9;
    let g = tile(&d, vec![a], r);
    let mut c = r;
    c.layout = TilingLayout::Circle;
    d.apply_tiling(&req(Some(g), vec![], c), &mut minter())
        .unwrap();
    let stored = d.tiling(g).unwrap().recipe;
    assert_eq!(stored.seed, 1234);
    assert_eq!(stored.variance, r.variance);
    assert_eq!(
        (stored.grid.columns, stored.grid.rows),
        (3, 2),
        "grid values kept"
    );
    let mut keys: Vec<_> = keyed(&d, g).keys().copied().collect();
    keys.sort_unstable();
    assert_eq!(keys, (1..9).map(|i| (0, i)).collect::<Vec<_>>());
}

#[test]
fn c28_hand_edits_are_overwritten_and_deleted_cells_restored() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let r = recipe(4, 1);
    let g = tile(&d, vec![a], r);
    let k = keyed(&d, g);
    d.translate_objects(&[k[&(0, 1)]], Vec2::new(3.0, 3.0))
        .unwrap();
    d.delete_objects(&[k[&(0, 2)]]).unwrap();
    // An unlinked instance is left alone and its cell counts as taken.
    d.unlink_clones(&[CopySource {
        id: k[&(0, 3)],
        anchor_ids: (0..3).map(|n| AnchorId::new(66, n)).collect(),
    }])
    .unwrap();
    // An object without a cell key put into the group is left alone.
    let stray = tri(&d, 500.0, 30);
    let stray_pts = clone_free_points(&d, stray);
    let moved = move_into(&d, stray, g);
    let unlinked_pts = clone_free_points(&d, k[&(0, 3)]);
    let steps = d.own_steps().len();
    d.apply_tiling(&req(Some(g), vec![], r), &mut minter())
        .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    let after = keyed(&d, g);
    assert_eq!(
        clone_of(&d, after[&(0, 1)]).chain.matrix,
        Affine::translation(Vec2::new(15.0, 0.0)),
        "placement overwritten"
    );
    assert!(after.contains_key(&(0, 2)), "deleted cell restored");
    assert_eq!(after[&(0, 3)], k[&(0, 3)], "unlinked instance kept");
    assert_eq!(
        clone_free_points(&d, k[&(0, 3)]),
        unlinked_pts,
        "and left alone"
    );
    assert_eq!(
        d.children(Some(g))
            .iter()
            .filter(|c| keyed(&d, g).values().any(|v| v == *c))
            .count(),
        3
    );
    if moved {
        assert!(d.children(Some(g)).contains(&stray), "stray object kept");
        assert_eq!(clone_free_points(&d, stray), stray_pts);
    }
}

#[test]
fn c28_own_style_set_by_hand_is_kept_without_style_variance_and_rewritten_with_it() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let r = recipe(3, 1);
    let g = tile(&d, vec![a], r);
    let one = keyed(&d, g)[&(0, 1)];
    d.set_own_style(&[one], true).unwrap();
    d.edit_style(&[one], &StyleEdit::StrokeWidth(Length::from_mm(4.0)))
        .unwrap();
    let mut moved = req(Some(g), vec![], r);
    moved.cells = cells_with(&r, 2.0, 0.0, 0.0);
    d.apply_tiling(&moved, &mut minter()).unwrap();
    let c = clone_of(&d, one);
    assert!(c.chain.own_style);
    assert_eq!(
        c.resolved.style.stroke.width.as_mm(),
        4.0,
        "hand style kept"
    );
    // With hue variance on, Apply owns the style again.
    let mut hue = r;
    hue.variance.hue.random = Angle::from_radians(0.3);
    let mut request = req(Some(g), vec![], hue);
    request.cells = cells_with(&hue, 2.0, 0.0, 0.3);
    d.apply_tiling(&request, &mut minter()).unwrap();
    let c = clone_of(&d, keyed(&d, g)[&(0, 1)]);
    assert_ne!(
        c.resolved.style.stroke.width.as_mm(),
        4.0,
        "rewritten from the source"
    );
}

fn move_into(d: &Document, id: NodeId, group: NodeId) -> bool {
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![id],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(group)),
    })
    .is_ok()
}

// ---------------------------------------------------------------------------------------------
// Source delete, whole tiling delete, duplicate, release (criteria 30 to 32)
// ---------------------------------------------------------------------------------------------

#[test]
fn c30_deleting_a_source_repairs_clones_keeps_the_dangling_id_and_undo_restores() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut r = recipe(3, 1);
    r.keep_source = false;
    let g = tile(&d, vec![a], r);
    let instances = keyed(&d, g);
    d.delete_objects(&[a]).unwrap();
    let stored = d.tiling(g).unwrap();
    assert_eq!(stored.sources, vec![a], "dangling id kept");
    assert!(!stored.sources_live);
    let read = d.read_objects();
    for id in instances.values() {
        assert!(read.get(*id).is_some(), "instance still drawn after repair");
    }
    assert_eq!(
        d.apply_tiling(&req(Some(g), vec![], r), &mut minter()),
        Err(TilingError::SourceMissing)
    );
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert!(d.tiling(g).unwrap().sources_live);
    for id in instances.values() {
        assert_eq!(clone_of(&d, *id).chain.origin, a, "linked again after undo");
    }
}

#[test]
fn c31_deleting_the_tiling_removes_everything_in_one_step_and_undo_restores_it() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let mut r = recipe(3, 2);
    r.seed = 99;
    let g = tile(&d, vec![a], r);
    let count = d.children(Some(g)).len();
    let steps = d.own_steps().len();
    d.delete_objects(&[g]).unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    let read = d.read_objects();
    assert!(read.get(a).is_none(), "the kept source goes too");
    assert!(d.tiling(g).is_none());
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.tiling(g).unwrap().recipe, r, "options and seed restored");
    assert_eq!(d.children(Some(g)).len(), count);
    assert_eq!(keyed(&d, g).len(), count - 1);
}

#[test]
fn c31_a_duplicate_is_an_independent_tiling_with_remapped_sources_and_members() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let b = tri(&d, 20.0, 10);
    let r = recipe(2, 1);
    let g = tile(&d, vec![a, b], r);
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: g,
                anchor_ids: (0..6).map(|n| AnchorId::new(88, n)).collect(),
            }],
            Vec2::new(0.0, 100.0),
        )
        .unwrap()[0];
    let stored = d.tiling(copy).unwrap();
    assert_eq!(stored.recipe, r, "identical options and seed");
    let kids = d.children(Some(copy));
    assert_eq!(
        stored.sources,
        vec![kids[0], kids[1]],
        "remapped to the copied source"
    );
    assert!(!stored.sources.contains(&a) && !stored.sources.contains(&b));
    let loro = loro_of(&d);
    let inst = keyed(&d, copy)[&(0, 1)];
    for m in d.children(Some(inst)) {
        let LoroValue::String(s) = reg(&loro, m, "tiling_member").unwrap() else {
            panic!()
        };
        let s = s.to_string();
        assert!(
            !encodings(a).contains(&s) && !encodings(b).contains(&s),
            "member names the original: {s}"
        );
        assert!(
            encodings(kids[0]).contains(&s) || encodings(kids[1]).contains(&s),
            "member names the copy: {s}"
        );
        if let Some(ObjectSnapshot::Clone(c)) = d.read_objects().get(m) {
            assert!(
                c.chain.origin == kids[0] || c.chain.origin == kids[1],
                "clone of the copy"
            );
        }
    }
    // Independent: re-applying the copy leaves the original alone.
    let orig = drawing_of_group(&d, g);
    d.apply_tiling(&req(Some(copy), vec![], recipe(3, 1)), &mut minter())
        .unwrap();
    assert_eq!(drawing_of_group(&d, g), orig);
    assert_eq!(keyed(&d, copy).len(), 2);
}

fn encodings(id: NodeId) -> Vec<String> {
    let v = serde_json::to_value(id).unwrap();
    let (p, c) = (
        v["peer"].as_str().unwrap().to_owned(),
        v["counter"].as_i64().unwrap(),
    );
    vec![
        format!("{p}:{c}"),
        format!("{c}:{p}"),
        format!("{c}@{p}"),
        format!("{p}@{c}"),
    ]
}

fn drawing_of_group(d: &Document, g: NodeId) -> Vec<NodeId> {
    let mut v = d.children(Some(g));
    v.sort_by_key(|id| format!("{id:?}"));
    v
}

#[test]
fn c32_release_strips_every_tiling_register_in_one_step() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let b = tri(&d, 20.0, 10);
    let g = tile(&d, vec![a, b], recipe(2, 2));
    let drawn = drawing(&d);
    let steps = d.own_steps().len();
    d.release_tiling(g).unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(last_label(&d), "release_tiling");
    assert!(d.tiling(g).is_none());
    assert!(d.is_group(g));
    assert_eq!(drawing(&d), drawn, "exactly as drawn");
    assert_no_tiling_registers(&d);
    assert_eq!(d.release_tiling(g), Err(TilingError::NotATiling));
    assert_eq!(d.release_tiling(a), Err(TilingError::NotATiling));
}

#[test]
fn c32_ungroup_strips_the_registers_too() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let b = tri(&d, 20.0, 10);
    let g = tile(&d, vec![a, b], recipe(2, 2));
    d.ungroup(&[g]).unwrap();
    // The instances are groups of members; their member keys must be gone too.
    assert_no_tiling_registers(&d);
}

fn assert_no_tiling_registers(d: &Document) {
    let loro = loro_of(d);
    let tree = loro.get_tree("paths");
    for n in tree.get_nodes(false) {
        if tree.is_node_deleted(&n.id).unwrap_or(true) {
            continue;
        }
        let m = tree.get_meta(n.id).unwrap();
        for key in ["tiling", "tiling_sources", "tiling_cell", "tiling_member"] {
            assert!(m.get(key).is_none(), "{key} left on {:?}", n.id);
        }
    }
}

#[test]
fn c32_concurrent_applies_leave_duplicates_that_the_next_apply_repairs() {
    let a_doc = Document::new(1);
    let src = tri(&a_doc, 0.0, 1);
    let g = tile(&a_doc, vec![src], recipe(2, 1));
    let b_doc = unpack(2, &snapshot(&a_doc)).unwrap();
    let base_a = a_doc.version();
    let base_b = b_doc.version();
    // Both peers add the same column concurrently.
    a_doc
        .apply_tiling(&req(Some(g), vec![], recipe(3, 1)), &mut minter())
        .unwrap();
    b_doc
        .apply_tiling(&req(Some(g), vec![], recipe(3, 1)), &mut minter())
        .unwrap();
    let from_b = b_doc.export_updates(&base_b).unwrap();
    let from_a = a_doc.export_updates(&base_a).unwrap();
    a_doc.import_updates(&from_b).unwrap();
    b_doc.import_updates(&from_a).unwrap();
    let loro = loro_of(&a_doc);
    let dup = a_doc
        .children(Some(g))
        .into_iter()
        .filter(|c| cell_of(&loro, *c) == Some((0, 2)))
        .count();
    assert_eq!(dup, 2, "both creations survive the merge");
    let _ = a_doc.read_objects();
    // The next apply keeps one child per key.
    a_doc
        .apply_tiling(&req(Some(g), vec![], recipe(3, 1)), &mut minter())
        .unwrap();
    let loro = loro_of(&a_doc);
    let mut keys: Vec<_> = a_doc
        .children(Some(g))
        .into_iter()
        .filter_map(|c| cell_of(&loro, c))
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, vec![(0, 1), (0, 2)]);
    // Both peers converge on the same survivor (the lowest node id).
    b_doc
        .import_updates(&a_doc.export_updates(&base_a).unwrap())
        .unwrap();
    assert_eq!(keyed(&a_doc, g), keyed(&b_doc, g));
}

#[test]
fn c31_an_instance_copied_out_of_its_tiling_is_a_plain_object() {
    let d = Document::new(1);
    let a = tri(&d, 0.0, 1);
    let g = tile(&d, vec![a], recipe(3, 1));
    let inst = keyed(&d, g)[&(0, 1)];
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: inst,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(0.0, 50.0),
        )
        .unwrap()[0];
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![copy],
        target: curvyo_document_core::MoveTarget::FrontOf(layer_of(&d, g).unwrap()),
    })
    .unwrap();
    let loro = loro_of(&d);
    assert_eq!(
        cell_of(&loro, copy),
        None,
        "outside its tiling it has no cell key"
    );
    assert!(unpack(2, &snapshot(&d)).is_ok());
    // The next Apply leaves it alone and keeps one instance per cell.
    d.apply_tiling(&req(Some(g), vec![], recipe(3, 1)), &mut minter())
        .unwrap();
    assert!(d.read_objects().get(copy).is_some());
    assert_eq!(keyed(&d, g).len(), 2);
}
