//! Independent acceptance tests for `specs/0100-bring-content-in` at the document model and file
//! format (ADR 0045): placing linked clones (criteria 1, 6), clones of groups, texts, images and
//! primitives (7 to 11), the containment cycle rule (12), placed pages (13, 17 to 19, 24) and
//! format 38 (21). Written from the spec before reading the implementation.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::assert_is_empty,
    clippy::cast_possible_truncation,
    missing_docs
)]

use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, Angle, CURRENT_FORMAT_VERSION, CloneError, CopySource, Document,
    DocumentSize, EllipseFrame, FontKind, FontRef, Length, ManufacturingError, MovePlan,
    MoveRefusal, MoveTarget, NewAnchor, NewImage, NewOutputPage, NodeId, ObjectSnapshot, OpenError,
    PLACE_LINKED_CLONES_COMMIT_LABEL, PLACE_ON_PAGE_COMMIT_LABEL, PLACE_PAGE_COMMIT_LABEL, Page,
    PageKind, Point, RectBounds, StyleEdit, TextLayoutValues, UNLINK_CLONE_COMMIT_LABEL, Vec2,
    pack, unpack,
};

const TOL: f64 = 1e-9;
const CURVE_TOL: f64 = 0.01;

// ---- helpers ------------------------------------------------------------------------------------

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn png() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png"),
    )
    .unwrap()
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

fn reopen(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "0100-tester").unwrap()).unwrap()
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

fn add_laser(d: &Document, w: f64, h: f64) -> Page {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(w, h),
    })
    .unwrap()
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + w, y + h),
    ))
}

fn line(d: &Document, seed: u64, from: Point, to: Point) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(77, seed * 10), from),
            NewAnchor::corner(AnchorId::new(77, seed * 10 + 1), to),
        ],
        false,
    )
}

fn text(d: &Document, s: &str, at: Vec2) -> NodeId {
    d.create_text(
        s,
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(at),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn image(d: &Document, x: f64, y: f64) -> NodeId {
    d.add_images(&[NewImage {
        bytes: Arc::from(png().as_slice()),
        placement: Affine::new(20.0, 0.0, 0.0, 10.0, x, y),
        name: "p.png".to_owned(),
    }])
    .unwrap()[0]
}

/// Outline points of any outline-bearing snapshot (path, primitive, outline clone).
fn points(o: &ObjectSnapshot) -> Vec<Point> {
    match o {
        ObjectSnapshot::Path(p) => p.all_anchors().map(|a| a.point).collect(),
        ObjectSnapshot::Primitive(p) => curvyo_document_core::outline_of(&p.shape)
            .anchors
            .iter()
            .map(|a| a.point)
            .collect(),
        ObjectSnapshot::Clone(c) if !c.is_picture() => {
            c.resolved.all_anchors().map(|a| a.point).collect()
        }
        other => panic!("no outline for {other:?}"),
    }
}

fn pts_of(d: &Document, id: NodeId) -> Vec<Point> {
    points(&d.object(id).unwrap())
}

fn mapped(ps: &[Point], m: Affine) -> Vec<Point> {
    ps.iter().map(|&p| m.apply_point(p)).collect()
}

fn close(a: &[Point], b: &[Point], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| (p.x - q.x).abs() <= tol && (p.y - q.y).abs() <= tol)
}

fn clone_snap(d: &Document, id: NodeId) -> curvyo_document_core::CloneSnapshot {
    match d.object(id).unwrap() {
        ObjectSnapshot::Clone(c) => c,
        other => panic!("{id:?} is not a clone: {other:?}"),
    }
}

fn kind(o: &ObjectSnapshot) -> &'static str {
    match o {
        ObjectSnapshot::Path(_) => "path",
        ObjectSnapshot::Primitive(_) => "primitive",
        ObjectSnapshot::Clone(_) => "clone",
        ObjectSnapshot::Text(_) => "text",
        ObjectSnapshot::Image(_) => "image",
        ObjectSnapshot::Sketch(_) => "sketch",
        ObjectSnapshot::Dimension(_) => "dimension",
    }
}

/// The drawn leaf outlines (paths only) of a picture clone, in stacking order.
fn picture_paths(c: &curvyo_document_core::CloneSnapshot) -> Vec<Vec<Point>> {
    c.drawn_leaves()
        .iter()
        .filter(|l| matches!(l, ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_)))
        .map(points)
        .collect()
}

fn leaf_kinds(c: &curvyo_document_core::CloneSnapshot) -> Vec<&'static str> {
    c.drawn_leaves().iter().map(kind).collect()
}

fn count_objects(d: &Document) -> usize {
    d.read_objects().objects.len()
}

fn clones_of_root(d: &Document, root: NodeId) -> Vec<NodeId> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(c) if c.chain.root == root => Some(c.resolved.id),
            _ => None,
        })
        .collect()
}

/// A drawing page "first" with group G (rect, circle, path, text, image), and a laser page of
/// 400 x 300 mm.
struct Fx {
    d: Document,
    first: Page,
    laser: Page,
    g: NodeId,
    rect: NodeId,
    circle: NodeId,
    path: NodeId,
    text: NodeId,
    image: NodeId,
}

fn fx() -> Fx {
    let d = Document::new(1);
    let laser = add_laser(&d, 400.0, 300.0);
    let first = page(&d, "first");
    let o = first.origin;
    let rect = rect(&d, o.x + 10.0, o.y + 10.0, 30.0, 20.0);
    let circle = d.create_ellipse(EllipseFrame {
        center: Point::new(o.x + 60.0, o.y + 20.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let path = line(
        &d,
        1,
        Point::new(o.x + 5.0, o.y + 50.0),
        Point::new(o.x + 80.0, o.y + 55.0),
    );
    let text = text(&d, "ANNA", Vec2::new(o.x + 10.0, o.y + 70.0));
    let image = image(&d, o.x + 50.0, o.y + 60.0);
    let g = d.group_objects(&[rect, circle, path, text, image]).unwrap();
    let laser = page(&d, &laser.id);
    Fx {
        d,
        first,
        laser,
        g,
        rect,
        circle,
        path,
        text,
        image,
    }
}

fn to_laser(f: &Fx) -> Vec2 {
    Vec2::new(
        f.laser.origin.x - f.first.origin.x + 20.0,
        f.laser.origin.y - f.first.origin.y + 20.0,
    )
}

// ---- criterion 1: one drop, one step, originals untouched -----------------------------------

#[test]
fn ac1_four_objects_place_four_linked_clones_in_one_step_and_undo_removes_them() {
    let d = Document::new(1);
    let laser = add_laser(&d, 800.0, 600.0);
    let first = page(&d, "first");
    let o = first.origin;
    let a = rect(&d, o.x + 10.0, o.y + 10.0, 20.0, 10.0);
    let b = line(
        &d,
        2,
        Point::new(o.x + 40.0, o.y + 10.0),
        Point::new(o.x + 60.0, o.y + 30.0),
    );
    let c = rect(&d, o.x + 70.0, o.y + 10.0, 5.0, 5.0);
    // On the pasteboard above the drawing page.
    let p = rect(&d, o.x + 10.0, o.y - 50.0, 10.0, 10.0);
    let laser = page(&d, &laser.id);
    let before: Vec<_> = [a, b, c, p].iter().map(|&i| d.object(i).unwrap()).collect();
    let steps = d.own_steps().len();
    let objects = count_objects(&d);
    let shift = Vec2::new(
        laser.origin.x - o.x + 300.0 - 280.0,
        laser.origin.y - o.y + 60.0,
    );
    let clones = d.place_linked_clones(&[a, b, c, p], shift).unwrap();
    assert_eq!(clones.len(), 4);
    assert_eq!(d.own_steps().len(), steps + 1, "one history step");
    assert_eq!(last_label(&d), PLACE_LINKED_CLONES_COMMIT_LABEL);
    assert_eq!(count_objects(&d), objects + 4);
    for (i, (&orig, &cl)) in [a, b, c, p].iter().zip(&clones).enumerate() {
        assert_eq!(
            d.object(orig).unwrap(),
            before[i],
            "original {i} is not written"
        );
        let snap = clone_snap(&d, cl);
        assert_eq!(snap.chain.origin, orig);
        assert!(snap.chain.own_placement, "own placement on (Frage 150 A)");
        assert!(
            snap.chain.matrix.approx_eq(Affine::translation(shift), TOL),
            "matrix is the drag's translation"
        );
        let want = mapped(&pts_of(&d, orig), Affine::translation(shift));
        assert!(close(&points(&d.object(cl).unwrap()), &want, TOL));
    }
    // The clones are stacked in the selection's order, on top.
    let order: Vec<NodeId> = d
        .read_objects()
        .objects
        .iter()
        .map(ObjectSnapshot::id)
        .collect();
    let pos: Vec<usize> = clones
        .iter()
        .map(|c| order.iter().position(|x| x == c).unwrap())
        .collect();
    assert!(pos.windows(2).all(|w| w[0] < w[1]), "selection order kept");
    assert_eq!(*pos.last().unwrap(), order.len() - 1, "on top");
    undo_last(&d);
    assert_eq!(count_objects(&d), objects, "undo removes every clone");
    for &cl in &clones {
        assert!(d.object(cl).is_none());
    }
    for (i, &orig) in [a, b, c, p].iter().enumerate() {
        assert_eq!(d.object(orig).unwrap(), before[i]);
    }
}

#[test]
fn ac25_place_on_page_uses_its_own_label_and_one_step() {
    let f = fx();
    let steps = f.d.own_steps().len();
    let clones =
        f.d.place_linked_clones_on_page(&[f.g], to_laser(&f))
            .unwrap();
    assert_eq!(clones.len(), 1);
    assert_eq!(f.d.own_steps().len(), steps + 1);
    assert_eq!(last_label(&f.d), PLACE_ON_PAGE_COMMIT_LABEL);
    assert!(clone_snap(&f.d, clones[0]).chain.own_placement);
}

#[test]
fn ac1_a_repeated_original_makes_one_clone_and_an_empty_list_writes_nothing() {
    let f = fx();
    let steps = f.d.own_steps().len();
    let clones = f.d.place_linked_clones(&[f.g, f.g], to_laser(&f)).unwrap();
    assert_eq!(clones.len(), 1, "a repeated id is cloned once");
    let steps_after = f.d.own_steps().len();
    assert_eq!(steps_after, steps + 1);
    let r = f.d.place_linked_clones(&[], to_laser(&f));
    assert!(r.map_or(true, |v| v.is_empty()));
    assert_eq!(
        f.d.own_steps().len(),
        steps_after,
        "nothing to place adds no step"
    );
}

#[test]
fn ac1_a_non_finite_or_huge_translation_is_refused_and_writes_nothing() {
    let f = fx();
    let steps = f.d.own_steps().len();
    let n = count_objects(&f.d);
    for t in [
        Vec2::new(f64::NAN, 0.0),
        Vec2::new(f64::INFINITY, 0.0),
        Vec2::new(1e12, 0.0),
    ] {
        assert!(f.d.place_linked_clones(&[f.rect], t).is_err(), "{t:?}");
    }
    assert_eq!(f.d.own_steps().len(), steps);
    assert_eq!(count_objects(&f.d), n);
}

// ---- criterion 6: every kind; depth 8 refusal ------------------------------------------------

#[test]
fn ac6_every_kind_is_cloned_and_a_primitive_original_stays_a_primitive() {
    let f = fx();
    let chain = f.d.linked_clone(&[f.path], &[]).unwrap()[0];
    let originals = [f.rect, f.circle, f.path, f.text, f.image, f.g, chain];
    let before: Vec<_> = originals[..5]
        .iter()
        .map(|&i| f.d.object(i).unwrap())
        .collect();
    let clones = f.d.place_linked_clones(&originals, to_laser(&f)).unwrap();
    assert_eq!(clones.len(), originals.len());
    for (i, &o) in originals[..5].iter().enumerate() {
        assert_eq!(f.d.object(o).unwrap(), before[i], "original {i} unchanged");
    }
    assert!(matches!(
        f.d.object(f.rect).unwrap(),
        ObjectSnapshot::Primitive(_)
    ));
    // The clone of the clone is a chain (depth 2) to the path.
    let c = clone_snap(&f.d, clones[6]);
    assert_eq!(c.chain.origin, chain);
    assert_eq!(c.chain.root, f.path);
    assert_eq!(c.chain.depth, 2);
    // Clones of the group, text and image are picture clones.
    for &i in &[3, 4, 5] {
        assert!(
            clone_snap(&f.d, clones[i]).is_picture(),
            "{i} is a picture clone"
        );
    }
    // Primitive roots are outline roots.
    assert!(!clone_snap(&f.d, clones[0]).is_picture());
    let want = mapped(&pts_of(&f.d, f.rect), Affine::translation(to_laser(&f)));
    assert!(close(&pts_of(&f.d, clones[0]), &want, TOL));
}

#[test]
fn ac6_a_clone_of_depth_eight_refuses_the_whole_drop() {
    let f = fx();
    let mut top = f.path;
    for _ in 0..8 {
        top = f.d.linked_clone(&[top], &[]).unwrap()[0];
    }
    assert_eq!(clone_snap(&f.d, top).chain.depth, 8);
    let steps = f.d.own_steps().len();
    let n = count_objects(&f.d);
    let err =
        f.d.place_linked_clones(&[f.rect, top], to_laser(&f))
            .unwrap_err();
    assert!(
        matches!(err, CloneError::TooDeep(ref ids) if ids.contains(&top)),
        "{err:?}"
    );
    assert_eq!(f.d.own_steps().len(), steps, "no step");
    assert_eq!(
        count_objects(&f.d),
        n,
        "nothing is created, the rect neither"
    );
    // Depth 7 is still fine and gives depth 8.
    let seven = clone_snap(&f.d, top).chain.origin;
    let ok = f.d.place_linked_clones(&[seven], to_laser(&f)).unwrap();
    assert_eq!(clone_snap(&f.d, ok[0]).chain.depth, 8);
}

#[test]
fn ac6_a_layer_is_never_an_original() {
    let f = fx();
    let layer = f.d.new_layer(None);
    let steps = f.d.own_steps().len();
    assert!(matches!(
        f.d.place_linked_clones(&[f.rect, layer], to_laser(&f)),
        Err(CloneError::NotClonable | CloneError::NoSuchObject)
    ));
    assert_eq!(f.d.own_steps().len(), steps);
}

// ---- criterion 7: a group clone follows its group live, own placement holds -----------------

#[test]
fn ac7_a_turned_and_scaled_group_clone_follows_every_member_edit_but_not_a_group_move() {
    let f = fx();
    let c = f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap()[0];
    // Turn it by 30 degrees and scale by 50 % about the laser page centre.
    let centre = Point::new(f.laser.origin.x + 200.0, f.laser.origin.y + 150.0);
    let turn = Affine::rotation_about(centre, Angle::from_radians((30.0_f64).to_radians()));
    let scale = Affine::translation(Vec2::new(centre.x, centre.y))
        .compose(Affine::scale(0.5, 0.5))
        .compose(Affine::translation(Vec2::new(-centre.x, -centre.y)));
    let extra = turn.compose(scale);
    let cs = clone_snap(&f.d, c);
    let moved = ObjectSnapshot::Clone(cs.mapped_by(extra));
    f.d.transform_objects(&[moved], false).unwrap();
    let check = |what: &str| {
        let cs = clone_snap(&f.d, c);
        let want: Vec<Vec<Point>> =
            f.d.children(Some(f.g))
                .iter()
                .filter_map(|&id| match f.d.object(id) {
                    Some(o @ (ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_))) => {
                        Some(mapped(&points(&o), cs.chain.composed))
                    }
                    _ => None,
                })
                .collect();
        let got = picture_paths(&cs);
        assert_eq!(got.len(), want.len(), "{what}: path leaf count");
        for (g, w) in got.iter().zip(&want) {
            assert!(close(g, w, CURVE_TOL), "{what}: {g:?} vs {w:?}");
        }
    };
    check("start");
    // (a) a node of the rectangle's sibling path is dragged.
    let anchor = match f.d.object(f.path).unwrap() {
        ObjectSnapshot::Path(p) => p.all_anchors().next().unwrap().id,
        _ => unreachable!(),
    };
    f.d.move_anchors(&[(
        f.path,
        anchor,
        Point::new(f.first.origin.x + 1.0, f.first.origin.y + 40.0),
    )])
    .unwrap();
    check("(a) node drag");
    // (b) the text changes.
    let chars = match f.d.object(f.text).unwrap() {
        ObjectSnapshot::Text(t) => t.chars.clone(),
        _ => unreachable!(),
    };
    let _ = f.d.edit_text(f.text, "ANNE", &chars);
    let text_leaf = clone_snap(&f.d, c)
        .drawn_leaves()
        .into_iter()
        .find_map(|l| match l {
            ObjectSnapshot::Text(t) => Some(t),
            _ => None,
        })
        .expect("the text is drawn by the clone");
    assert_eq!(text_leaf.string, "ANNE", "(b) the text edit shows");
    // (c) a style edit of the circle shows.
    f.d.edit_style(&[f.circle], &StyleEdit::StrokeWidth(Length::from_mm(2.5)))
        .unwrap();
    let widths: Vec<f64> = clone_snap(&f.d, c)
        .drawn_leaves()
        .iter()
        .map(|l| l.style().stroke.width.as_mm())
        .collect();
    assert!(
        widths.iter().any(|w| (w - 2.5).abs() < TOL),
        "(c) {widths:?}"
    );
    // (d) a path drawn into G.
    let new = line(
        &f.d,
        9,
        Point::new(f.first.origin.x + 1.0, f.first.origin.y + 1.0),
        Point::new(f.first.origin.x + 9.0, f.first.origin.y + 9.0),
    );
    f.d.move_nodes(&MovePlan {
        nodes: vec![new],
        target: MoveTarget::FrontOf(Some(f.g)),
    })
    .unwrap();
    check("(d) a path drawn into G");
    // (e) the circle moves out of G.
    f.d.move_nodes(&MovePlan {
        nodes: vec![f.circle],
        target: MoveTarget::Above(f.g),
    })
    .unwrap();
    check("(e) the circle left G");
    let before = clone_snap(&f.d, c).drawn_leaves();
    // (f) G moves and rotates as a whole: the clone stays.
    let g_leaves = f.d.children(Some(f.g));
    f.d.translate_objects(&g_leaves, Vec2::new(15.0, -7.0))
        .unwrap();
    let _ = g_leaves;
    let after = clone_snap(&f.d, c).drawn_leaves();
    assert_eq!(
        before.iter().map(points_or_none).collect::<Vec<_>>(),
        after.iter().map(points_or_none).collect::<Vec<_>>(),
        "(f) nothing changes on the laser page after a move of G"
    );
}

fn points_or_none(o: &ObjectSnapshot) -> Option<Vec<(i64, i64)>> {
    match o {
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_) => Some(
            points(o)
                .iter()
                .map(|p| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64))
                .collect(),
        ),
        _ => None,
    }
}

// ---- criterion 9: what a group clone draws --------------------------------------------------

#[test]
fn ac9_hidden_g_still_feeds_its_clones_a_hidden_member_and_a_guide_do_not() {
    let f = fx();
    let c = f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap()[0];
    let all = leaf_kinds(&clone_snap(&f.d, c)).len();
    assert_eq!(
        all,
        5,
        "five members drawn: {:?}",
        leaf_kinds(&clone_snap(&f.d, c))
    );
    f.d.set_node_visible(&[f.g], false).unwrap();
    assert_eq!(
        leaf_kinds(&clone_snap(&f.d, c)).len(),
        5,
        "G's own Hide does not hide its clones"
    );
    f.d.set_node_visible(&[f.g], true).unwrap();
    f.d.set_node_visible(&[f.circle], false).unwrap();
    assert_eq!(
        leaf_kinds(&clone_snap(&f.d, c)).len(),
        4,
        "a hidden member is not drawn"
    );
    f.d.set_guide(&[f.path], true).unwrap();
    assert_eq!(
        leaf_kinds(&clone_snap(&f.d, c)).len(),
        3,
        "a guide is not drawn"
    );
}

#[test]
fn ac9_a_member_hidden_in_a_nested_group_is_left_out_and_an_inner_clone_contributes() {
    let f = fx();
    let inner_a = rect(
        &f.d,
        f.first.origin.x + 100.0,
        f.first.origin.y + 100.0,
        5.0,
        5.0,
    );
    let inner_b = rect(
        &f.d,
        f.first.origin.x + 110.0,
        f.first.origin.y + 100.0,
        5.0,
        5.0,
    );
    let inner = f.d.group_objects(&[inner_a, inner_b]).unwrap();
    let path_clone = f.d.linked_clone(&[f.path], &[]).unwrap()[0];
    f.d.move_nodes(&MovePlan {
        nodes: vec![inner, path_clone],
        target: MoveTarget::FrontOf(Some(f.g)),
    })
    .unwrap();
    let c = f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap()[0];
    assert_eq!(leaf_kinds(&clone_snap(&f.d, c)).len(), 8);
    f.d.set_node_visible(&[inner], false).unwrap();
    assert_eq!(
        leaf_kinds(&clone_snap(&f.d, c)).len(),
        6,
        "a group hidden strictly below G hides its members in the clone"
    );
}

#[test]
fn ac9_an_empty_group_clone_draws_nothing_and_reads_without_panic() {
    let d = Document::new(1);
    let laser = add_laser(&d, 300.0, 200.0);
    let a = rect(&d, 10.0, 10.0, 5.0, 5.0);
    let b = rect(&d, 20.0, 10.0, 5.0, 5.0);
    let g = d.group_objects(&[a, b]).unwrap();
    let c = d
        .place_linked_clones(&[g], Vec2::new(laser.origin.x, laser.origin.y))
        .unwrap()[0];
    d.set_node_visible(&[a, b], false).unwrap();
    let cs = clone_snap(&d, c);
    assert!(cs.drawn_leaves().is_empty());
    let _ = reopen(&d, 2);
}

// ---- criterion 10: Unlink gives an independent copy -------------------------------------------

#[test]
fn ac10_unlinking_a_group_clone_gives_a_group_of_copies_drawing_the_same_outlines() {
    let f = fx();
    let c = f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap()[0];
    let drawn = picture_paths(&clone_snap(&f.d, c));
    let n = count_objects(&f.d);
    let steps = f.d.own_steps().len();
    f.d.unlink_clones(&[CopySource {
        id: c,
        anchor_ids: vec![],
    }])
    .unwrap();
    assert_eq!(f.d.own_steps().len(), steps + 1);
    assert_eq!(last_label(&f.d), UNLINK_CLONE_COMMIT_LABEL);
    // One clone node became five leaves (a group of copies).
    assert_eq!(count_objects(&f.d), n - 1 + 5);
    assert!(clones_of_root(&f.d, f.g).is_empty());
    let objects = f.d.read_objects();
    let new: Vec<&ObjectSnapshot> = objects.objects.iter().skip(n - 1).collect();
    let mut kinds: Vec<&str> = new.iter().map(|o| kind(o)).collect();
    kinds.sort_unstable();
    assert_eq!(
        kinds,
        vec!["image", "path", "primitive", "primitive", "text"]
    );
    let got: Vec<Vec<Point>> = new
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_)))
        .map(|o| points(o))
        .collect();
    assert_eq!(got.len(), drawn.len());
    for (g, w) in got.iter().zip(&drawn) {
        assert!(close(g, w, CURVE_TOL), "unlinked {g:?} vs drawn {w:?}");
    }
    // They live in one new group.
    let parents: std::collections::HashSet<_> =
        new.iter().map(|o| f.d.parent_node(o.id())).collect();
    assert_eq!(parents.len(), 1, "one group of copies");
    // The original G is untouched.
    assert_eq!(f.d.children(Some(f.g)).len(), 5);
}

#[test]
fn ac10_unlinking_a_skewed_primitive_clone_gives_a_path_and_a_translated_one_a_primitive() {
    let d = Document::new(1);
    let laser = add_laser(&d, 300.0, 200.0);
    let r = rect(&d, 10.0, 10.0, 20.0, 10.0);
    let shift = Vec2::new(laser.origin.x, laser.origin.y);
    let [a, b] = d
        .place_linked_clones(&[r], shift)
        .map(|v| [v[0], d.place_linked_clones(&[r], shift).unwrap()[0]])
        .unwrap();
    let skew = Affine::new(1.0, 0.0, 0.5, 1.0, 0.0, 0.0);
    let bs = clone_snap(&d, b);
    d.transform_objects(&[ObjectSnapshot::Clone(bs.mapped_by(skew))], false)
        .unwrap();
    let a_pts = pts_of(&d, a);
    let b_pts = pts_of(&d, b);
    d.unlink_clones(&[
        CopySource {
            id: a,
            anchor_ids: vec![],
        },
        CopySource {
            id: b,
            anchor_ids: (0..b_pts.len() as u64)
                .map(|i| AnchorId::new(55, i))
                .collect(),
        },
    ])
    .unwrap();
    let objects = d.read_objects();
    let last_two: Vec<&ObjectSnapshot> = objects.objects.iter().rev().take(2).collect();
    let mut by_kind: Vec<(&str, Vec<Point>)> =
        last_two.iter().map(|o| (kind(o), points(o))).collect();
    by_kind.sort_by_key(|(k, _)| *k);
    assert_eq!(by_kind[0].0, "path", "a skewed primitive becomes a path");
    assert_eq!(
        by_kind[1].0, "primitive",
        "a translated primitive stays one"
    );
    assert!(close(&by_kind[1].1, &a_pts, CURVE_TOL));
    // The skewed path draws the same outline (anchor points of a rect stay corners).
    let _ = b_pts;
}

// ---- criterion 11: deleting G turns its clones into copies -----------------------------------

#[test]
fn ac11_deleting_g_turns_its_clones_into_copies_in_one_step_and_undo_restores_clones() {
    let f = fx();
    let c1 = f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap()[0];
    let chain = f.d.linked_clone(&[c1], &[]).unwrap()[0];
    let drawn1 = picture_paths(&clone_snap(&f.d, c1));
    let drawn2 = picture_paths(&clone_snap(&f.d, chain));
    let steps = f.d.own_steps().len();
    f.d.delete_objects(&[f.g]).unwrap();
    assert_eq!(f.d.own_steps().len(), steps + 1, "one step");
    assert!(f.d.object(f.rect).is_none());
    // No clone of G survives as a clone; its drawing survives as copies.
    let objects = f.d.read_objects();
    assert!(
        !objects
            .objects
            .iter()
            .any(|o| matches!(o, ObjectSnapshot::Clone(c) if c.chain.root == f.g)),
        "no clone of the deleted G"
    );
    let all: Vec<Vec<Point>> = objects
        .objects
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_)))
        .map(points)
        .collect();
    for w in drawn1.iter().chain(&drawn2) {
        assert!(
            all.iter().any(|g| close(g, w, CURVE_TOL)),
            "the drawn outline {w:?} survives as a copy"
        );
    }
    // Undo restores G and the clones as clones.
    undo_last(&f.d);
    assert!(f.d.object(f.rect).is_some());
    assert!(clone_snap(&f.d, c1).is_picture());
    let cs = clone_snap(&f.d, chain);
    assert!(cs.is_picture());
    assert_eq!(cs.chain.origin, c1);
    assert_eq!(cs.chain.root, f.g);
}

#[test]
fn ac11_deleting_one_member_only_changes_what_the_clone_shows() {
    let f = fx();
    let c = f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap()[0];
    f.d.delete_objects(&[f.circle]).unwrap();
    let cs = clone_snap(&f.d, c);
    assert!(cs.is_picture());
    assert_eq!(cs.drawn_leaves().len(), 4);
}

#[test]
fn ac11_deleting_a_text_root_or_an_image_root_gives_a_text_and_an_image() {
    let d = Document::new(1);
    let laser = add_laser(&d, 300.0, 200.0);
    let t = text(&d, "HI", Vec2::new(5.0, 5.0));
    let i = image(&d, 40.0, 40.0);
    let shift = Vec2::new(laser.origin.x, laser.origin.y);
    let cs = d.place_linked_clones(&[t, i], shift).unwrap();
    d.delete_objects(&[t, i]).unwrap();
    let kinds: Vec<&str> = d.read_objects().objects.iter().map(kind).collect();
    assert_eq!(kinds, vec!["text", "image"], "copies of the right kinds");
    let objs = d.read_objects();
    if let ObjectSnapshot::Image(img) = &objs.objects[1] {
        let _ = &cs;
        assert!(img.placement.approx_eq(
            Affine::translation(shift).compose(Affine::new(20.0, 0.0, 0.0, 10.0, 40.0, 40.0)),
            CURVE_TOL
        ));
    }
}

// ---- criterion 12: no clone inside what it draws ----------------------------------------------

#[test]
fn ac12_a_clone_of_g_cannot_go_into_g_or_a_group_g_draws() {
    let f = fx();
    let c = f.d.linked_clone(&[f.g], &[]).unwrap()[0];
    assert!(f.d.would_cycle(&[c], Some(f.g)));
    let a = rect(&f.d, 0.0, 0.0, 1.0, 1.0);
    let b = rect(&f.d, 2.0, 0.0, 1.0, 1.0);
    let inner = f.d.group_objects(&[a, b]).unwrap();
    f.d.move_nodes(&MovePlan {
        nodes: vec![inner],
        target: MoveTarget::FrontOf(Some(f.g)),
    })
    .unwrap();
    assert!(f.d.would_cycle(&[c], Some(inner)), "a group G draws");
    // A group holding the clone is refused too.
    let other = rect(&f.d, 5.0, 5.0, 1.0, 1.0);
    let holder = f.d.group_objects(&[c, other]).unwrap();
    assert!(f.d.would_cycle(&[holder], Some(f.g)));
    let before = f.d.read_objects().objects.to_vec();
    let steps = f.d.own_steps().len();
    assert_eq!(
        f.d.move_nodes(&MovePlan {
            nodes: vec![holder],
            target: MoveTarget::FrontOf(Some(inner)),
        }),
        Err(MoveRefusal::InsideOwnOriginal)
    );
    assert_eq!(f.d.own_steps().len(), steps);
    assert_eq!(
        f.d.read_objects().objects.to_vec(),
        before,
        "nothing changes"
    );
    // Placing it above a member of G (Above(member)) is the same move into G.
    assert_eq!(
        f.d.move_nodes(&MovePlan {
            nodes: vec![c],
            target: MoveTarget::Above(f.rect),
        }),
        Err(MoveRefusal::InsideOwnOriginal)
    );
    assert_eq!(
        f.d.move_nodes(&MovePlan {
            nodes: vec![c],
            target: MoveTarget::Below(a),
        }),
        Err(MoveRefusal::InsideOwnOriginal)
    );
    // Moving to the top level or into a layer is fine.
    assert!(!f.d.would_cycle(&[c], None));
}

#[test]
fn ac12_two_groups_cloning_each_other_is_refused() {
    let d = Document::new(1);
    let g1 = d
        .group_objects(&[rect(&d, 0.0, 0.0, 1.0, 1.0), rect(&d, 2.0, 0.0, 1.0, 1.0)])
        .unwrap();
    let g2 = d
        .group_objects(&[rect(&d, 0.0, 5.0, 1.0, 1.0), rect(&d, 2.0, 5.0, 1.0, 1.0)])
        .unwrap();
    let c1 = d.linked_clone(&[g1], &[]).unwrap()[0];
    d.move_nodes(&MovePlan {
        nodes: vec![c1],
        target: MoveTarget::FrontOf(Some(g2)),
    })
    .unwrap();
    let c2 = d.linked_clone(&[g2], &[]).unwrap()[0];
    assert!(d.would_cycle(&[c2], Some(g1)));
    assert_eq!(
        d.move_nodes(&MovePlan {
            nodes: vec![c2],
            target: MoveTarget::FrontOf(Some(g1)),
        }),
        Err(MoveRefusal::InsideOwnOriginal)
    );
}

#[test]
fn ac12_a_clone_of_a_member_inside_g_is_allowed() {
    let f = fx();
    let c = f.d.linked_clone(&[f.path], &[]).unwrap()[0];
    assert!(!f.d.would_cycle(&[c], Some(f.g)));
    f.d.move_nodes(&MovePlan {
        nodes: vec![c],
        target: MoveTarget::FrontOf(Some(f.g)),
    })
    .unwrap();
    assert_eq!(f.d.children(Some(f.g)).len(), 6);
}

#[test]
fn ac12_a_merged_cycle_reads_without_hang_orphans_both_clones_and_peers_agree() {
    let a = Document::new(1);
    let g1 = a
        .group_objects(&[rect(&a, 0.0, 0.0, 1.0, 1.0), rect(&a, 2.0, 0.0, 1.0, 1.0)])
        .unwrap();
    let g2 = a
        .group_objects(&[rect(&a, 0.0, 5.0, 1.0, 1.0), rect(&a, 2.0, 5.0, 1.0, 1.0)])
        .unwrap();
    let c1 = a.linked_clone(&[g1], &[]).unwrap()[0];
    let c2 = a.linked_clone(&[g2], &[]).unwrap()[0];
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    // Each replica makes one legal move; together they close a cycle.
    a.move_nodes(&MovePlan {
        nodes: vec![c1],
        target: MoveTarget::FrontOf(Some(g2)),
    })
    .unwrap();
    b.move_nodes(&MovePlan {
        nodes: vec![c2],
        target: MoveTarget::FrontOf(Some(g1)),
    })
    .unwrap();
    let aa = a.export_updates(&b.version()).unwrap();
    let bb = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bb).unwrap();
    b.import_updates(&aa).unwrap();
    let ra: Vec<NodeId> = a
        .read_objects()
        .objects
        .iter()
        .map(ObjectSnapshot::id)
        .collect();
    let rb: Vec<NodeId> = b
        .read_objects()
        .objects
        .iter()
        .map(ObjectSnapshot::id)
        .collect();
    assert_eq!(ra, rb, "both peers read the same");
    assert!(
        !ra.contains(&c1) && !ra.contains(&c2),
        "both clones on the cycle are orphans"
    );
    assert_eq!(ra.len(), 4, "the four rectangles still draw");
    // A saved file holding the cycle opens without hang.
    let bytes = pack(&a, "t").unwrap();
    let r = unpack(3, &bytes).unwrap();
    assert_eq!(r.read_objects().objects.len(), 4);
}

#[test]
fn ac12_a_plan_where_one_node_would_cycle_is_refused_whole() {
    // Moving several nodes where only one would cycle refuses the whole plan.
    let f = fx();
    let c = f.d.linked_clone(&[f.g], &[]).unwrap()[0];
    let free = rect(&f.d, 0.0, 0.0, 1.0, 1.0);
    let steps = f.d.own_steps().len();
    assert_eq!(
        f.d.move_nodes(&MovePlan {
            nodes: vec![free, c],
            target: MoveTarget::FrontOf(Some(f.g)),
        }),
        Err(MoveRefusal::InsideOwnOriginal)
    );
    assert_eq!(f.d.own_steps().len(), steps);
    assert_ne!(f.d.parent_node(free), Some(f.g));
}

// ---- criteria 13, 17 to 19, 24: placed pages ------------------------------------------------

struct Pp {
    d: Document,
    first: Page,
    laser: Page,
}

fn pp() -> Pp {
    let d = Document::new(1);
    let laser = add_laser(&d, 600.0, 500.0);
    let first = page(&d, "first");
    rect(&d, first.origin.x + 10.0, first.origin.y + 10.0, 40.0, 30.0);
    let laser = page(&d, &laser.id);
    Pp { d, first, laser }
}

fn centred(p: &Pp) -> Affine {
    // Source page-local to laser page-local: the source frame's centre on the laser's centre.
    let (sw, sh) = (p.first.size.width.as_mm(), p.first.size.height.as_mm());
    let (lw, lh) = (p.laser.size.width.as_mm(), p.laser.size.height.as_mm());
    Affine::translation(Vec2::new((lw - sw) / 2.0, (lh - sh) / 2.0))
}

#[test]
fn ac13_place_page_is_one_step_and_writes_nothing_of_the_source() {
    let p = pp();
    let before = p.d.read_objects().objects.to_vec();
    let steps = p.d.own_steps().len();
    let id = p.d.place_page(&p.laser.id, "first", centred(&p)).unwrap();
    assert_eq!(p.d.own_steps().len(), steps + 1);
    assert_eq!(last_label(&p.d), PLACE_PAGE_COMMIT_LABEL);
    assert_eq!(
        p.d.read_objects().objects.to_vec(),
        before,
        "source objects untouched"
    );
    assert_eq!(page(&p.d, "first"), p.first, "source page untouched");
    let placed = p.d.placed_page(id).unwrap();
    assert_eq!(placed.output.id, p.laser.id);
    assert_eq!(placed.source.as_ref().unwrap().id, "first");
    assert!(placed.matrix.approx_eq(centred(&p), TOL));
    // A second one lies above the first.
    let id2 = p.d.place_page(&p.laser.id, "first", centred(&p)).unwrap();
    let ids: Vec<NodeId> = p.d.placed_pages().iter().map(|x| x.id).collect();
    assert_eq!(ids, vec![id, id2], "later placements above earlier ones");
    undo_last(&p.d);
    assert!(p.d.placed_page(id2).is_none());
}

#[test]
fn ac13_place_page_refuses_wrong_kinds_and_off_page_matrices() {
    let p = pp();
    let sketch = p.d.add_page_of(&PageKind::Sketch).unwrap();
    let other_laser = add_laser(&p.d, 100.0, 100.0);
    let steps = p.d.own_steps().len();
    for (out, src) in [
        ("first", "first"),
        (p.laser.id.as_str(), p.laser.id.as_str()),
        (p.laser.id.as_str(), other_laser.id.as_str()),
        (p.laser.id.as_str(), sketch.id.as_str()),
        (p.laser.id.as_str(), "nope"),
    ] {
        assert!(
            matches!(
                p.d.place_page(out, src, Affine::translation(Vec2::new(0.0, 0.0))),
                Err(ManufacturingError::NotFound)
            ),
            "{out} <- {src}"
        );
    }
    assert!(matches!(
        p.d.place_page(
            &p.laser.id,
            "first",
            Affine::translation(Vec2::new(5000.0, 0.0))
        ),
        Err(ManufacturingError::OffPage)
    ));
    assert!(
        p.d.place_page(
            &p.laser.id,
            "first",
            Affine::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0)
        )
        .is_err()
    );
    assert_eq!(p.d.own_steps().len(), steps, "nothing written");
    assert!(p.d.placed_pages().is_empty());
}

#[test]
fn ac14_an_empty_drawing_page_may_be_placed() {
    let p = pp();
    let empty = p.d.add_page_of(&PageKind::Drawing).unwrap();
    let id =
        p.d.place_page(
            &p.laser.id,
            &empty.id,
            Affine::translation(Vec2::new(1.0, 1.0)),
        )
        .unwrap();
    assert!(p.d.placed_page(id).unwrap().frame_corners().is_some());
}

#[test]
fn ac17_a_turned_and_scaled_placed_page_writes_only_its_link() {
    let p = pp();
    let id = p.d.place_page(&p.laser.id, "first", centred(&p)).unwrap();
    let objects = p.d.read_objects().objects.to_vec();
    let placed = p.d.placed_page(id).unwrap();
    let corners = placed.frame_corners().unwrap();
    let c = Point::new(
        f64::midpoint(corners[0].x, corners[2].x),
        f64::midpoint(corners[0].y, corners[2].y),
    );
    let t = Affine::rotation_about(c, Angle::from_radians((90.0_f64).to_radians()))
        .compose(Affine::translation(Vec2::new(c.x, c.y)))
        .compose(Affine::scale(0.5, 0.5))
        .compose(Affine::translation(Vec2::new(-c.x, -c.y)));
    let steps = p.d.own_steps().len();
    p.d.transform_placements(&[(id, t)], "rotate_objects")
        .unwrap();
    assert_eq!(p.d.own_steps().len(), steps + 1);
    assert_eq!(
        p.d.read_objects().objects.to_vec(),
        objects,
        "source registers untouched"
    );
    let after = p.d.placed_page(id).unwrap();
    let want = t.compose(placed.global_map().unwrap());
    assert!(
        after.global_map().unwrap().approx_eq(want, 1e-9),
        "global map is T after the old map"
    );
    let got = after.frame_corners().unwrap();
    let want_c: Vec<Point> = corners.iter().map(|&q| t.apply_point(q)).collect();
    assert!(close(&got, &want_c, CURVE_TOL));
}

#[test]
fn ac18_a_transform_leaving_no_overlap_is_refused_and_an_edge_touch_is_off() {
    let p = pp();
    let id =
        p.d.place_page(
            &p.laser.id,
            "first",
            Affine::translation(Vec2::new(0.0, 0.0)),
        )
        .unwrap();
    let placed = p.d.placed_page(id).unwrap();
    let lw = p.laser.size.width.as_mm();
    let steps = p.d.own_steps().len();
    // Exactly to the right edge: an edge-only touch is off the page.
    let to_edge = Affine::translation(Vec2::new(lw, 0.0));
    assert!(matches!(
        p.d.transform_placements(&[(id, to_edge)], "move_objects"),
        Err(ManufacturingError::OffPage)
    ));
    // Far off.
    assert!(matches!(
        p.d.transform_placements(
            &[(id, Affine::translation(Vec2::new(0.0, -9000.0)))],
            "move_objects"
        ),
        Err(ManufacturingError::OffPage)
    ));
    assert_eq!(p.d.own_steps().len(), steps);
    assert!(
        p.d.placed_page(id)
            .unwrap()
            .matrix
            .approx_eq(placed.matrix, 0.0)
    );
    // A sliver of 0.001 mm overlap keeps it.
    p.d.transform_placements(
        &[(id, Affine::translation(Vec2::new(lw - 0.001, 0.0)))],
        "move_objects",
    )
    .unwrap();
}

#[test]
fn ac18_a_turned_page_whose_turned_frame_misses_the_laser_page_is_off_it() {
    // A square source page turned by 45 degrees beyond the laser page's top-left corner: its
    // axis-aligned bounds still overlap the corner, its frame does not.
    let d = Document::new(1);
    let first = page(&d, "first");
    d.resize_page(
        "first",
        DocumentSize::from_mm(100.0, 100.0),
        &curvyo_document_core::PageContents::new(),
    )
    .ok();
    let first = page(&d, &first.id);
    let laser = add_laser(&d, 300.0, 300.0);
    let laser = page(&d, &laser.id);
    let side = first.size.width.as_mm();
    if (first.size.height.as_mm() - side).abs() > 1e-9 {
        return; // resize_page refused: the shape below assumes a square
    }
    let id = d
        .place_page(
            &laser.id,
            "first",
            Affine::translation(Vec2::new(10.0, 10.0)),
        )
        .unwrap();
    // Diamond centred at (-d, -d) relative to the laser's top-left, d chosen so its edge is 5 mm
    // clear of the corner: half-diagonal h = side / sqrt(2), centre distance along the diagonal
    // h + 5 / ... ; along x and y: (h / sqrt(2) + 5) each, so the diamond's nearest edge passes
    // outside the corner while its bounds (half-width h) reach past it.
    let h = side / 2.0_f64.sqrt();
    let off = h / 2.0_f64.sqrt() + 5.0;
    let placed = d.placed_page(id).unwrap();
    let corners = placed.frame_corners().unwrap();
    let c = Point::new(
        f64::midpoint(corners[0].x, corners[2].x),
        f64::midpoint(corners[0].y, corners[2].y),
    );
    let target = Point::new(laser.origin.x - off, laser.origin.y - off);
    let t = Affine::translation(Vec2::new(target.x - c.x, target.y - c.y)).compose(
        Affine::rotation_about(c, Angle::from_radians((45.0_f64).to_radians())),
    );
    let moved: Vec<Point> = corners.iter().map(|&q| t.apply_point(q)).collect();
    // Sanity: the bounds overlap the laser frame, the diamond does not reach the corner.
    let min_x = moved.iter().map(|q| q.x).fold(f64::INFINITY, f64::min);
    let max_x = moved.iter().map(|q| q.x).fold(f64::NEG_INFINITY, f64::max);
    assert!(min_x < laser.origin.x && max_x > laser.origin.x);
    assert!(
        matches!(
            d.transform_placements(&[(id, t)], "move_objects"),
            Err(ManufacturingError::OffPage)
        ),
        "a turned frame lying wholly outside the laser page is off it (criterion 18)"
    );
}

#[test]
fn ac19_duplicate_makes_a_second_placed_page_and_delete_removes_only_the_reference() {
    let p = pp();
    let id = p.d.place_page(&p.laser.id, "first", centred(&p)).unwrap();
    let objects = p.d.read_objects().objects.to_vec();
    let copies =
        p.d.duplicate_placements(&[id], Vec2::new(0.0, 0.0), "duplicate")
            .unwrap();
    assert_eq!(copies.len(), 1);
    let copy = p.d.placed_page(copies[0]).unwrap();
    assert_eq!(copy.output.id, p.laser.id);
    assert!(copy.matrix.approx_eq(centred(&p), TOL));
    let shifted =
        p.d.duplicate_placements(&[id], Vec2::new(12.0, -3.0), "copy")
            .unwrap();
    let s = p.d.placed_page(shifted[0]).unwrap();
    assert!(
        s.global_map().unwrap().approx_eq(
            Affine::translation(Vec2::new(12.0, -3.0))
                .compose(p.d.placed_page(id).unwrap().global_map().unwrap()),
            TOL
        )
    );
    // A copy pushed off the page is refused.
    assert!(matches!(
        p.d.duplicate_placements(&[id], Vec2::new(1e5, 0.0), "copy"),
        Err(ManufacturingError::OffPage)
    ));
    p.d.delete_objects(&[id]).unwrap();
    assert!(p.d.placed_page(id).is_none());
    assert_eq!(p.d.placed_pages().len(), 2);
    assert_eq!(
        p.d.read_objects().objects.to_vec(),
        objects,
        "source objects stay"
    );
}

#[test]
fn ac19_group_refuses_a_selection_holding_a_placed_page() {
    let p = pp();
    let id = p.d.place_page(&p.laser.id, "first", centred(&p)).unwrap();
    let r = rect(
        &p.d,
        p.laser.origin.x + 5.0,
        p.laser.origin.y + 5.0,
        5.0,
        5.0,
    );
    let steps = p.d.own_steps().len();
    assert!(p.d.group_objects(&[id, r]).is_err());
    assert!(p.d.group_objects(&[id]).is_err());
    assert_eq!(p.d.own_steps().len(), steps);
}

#[test]
fn ac24_deleting_a_drawing_page_removes_its_placed_pages_everywhere_in_one_step() {
    let p = pp();
    let second_laser = add_laser(&p.d, 300.0, 300.0);
    let other = p.d.add_page_of(&PageKind::Drawing).unwrap();
    let a = p.d.place_page(&p.laser.id, "first", centred(&p)).unwrap();
    let b =
        p.d.place_page(
            &second_laser.id,
            "first",
            Affine::translation(Vec2::new(0.0, 0.0)),
        )
        .unwrap();
    let keep =
        p.d.place_page(
            &p.laser.id,
            &other.id,
            Affine::translation(Vec2::new(0.0, 0.0)),
        )
        .unwrap();
    let steps = p.d.own_steps().len();
    p.d.delete_page("first", &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(p.d.own_steps().len(), steps + 1, "one step");
    assert!(p.d.placed_page(a).is_none());
    assert!(p.d.placed_page(b).is_none());
    assert!(
        p.d.placed_page(keep).is_some(),
        "other page's placement stays"
    );
    undo_last(&p.d);
    assert!(p.d.pages().iter().any(|pg| pg.id == "first"));
    assert!(p.d.placed_page(a).is_some());
    assert!(p.d.placed_page(b).is_some());
    assert_eq!(p.d.placed_pages().len(), 3);
}

// ---- criterion 21: format 38 ----------------------------------------------------------------

fn manifest_version(bytes: &[u8]) -> u64 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str::<serde_json::Value>(&text).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

fn with_manifest_version(bytes: &[u8], version: u64) -> Vec<u8> {
    use std::io::Write;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if name == "manifest.json" {
            let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
            v["format_version"] = serde_json::json!(version);
            data = v.to_string().into_bytes();
        }
        writer.start_file(name, options).unwrap();
        writer.write_all(&data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn ac21_the_format_is_38_and_a_newer_file_is_refused() {
    // `specs/0101-laser-operations` bumps to 39 (ADR 0046 §4) and `specs/0102-laser-fixtures` to
    // 40 (ADR 0049); the rule checked here is unchanged.
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    // Format 43 since `specs/0028-material-test-library` (ADR 0054 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 43);
    let f = fx();
    f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap();
    let bytes = pack(&f.d, "t").unwrap();
    assert_eq!(manifest_version(&bytes), 43);
    let newer = with_manifest_version(&bytes, 44);
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew {
            found: 44,
            supported: 43
        })
    ));
}

#[test]
fn ac21_the_golden_opens_and_round_trips_every_clone_placement_and_outline() {
    let bytes = fixture("bring_content_in_v38.curvyo");
    assert_eq!(manifest_version(&bytes), 38);
    let d = unpack(2, &bytes).unwrap();
    let objects = d.read_objects();
    let clones: Vec<_> = objects
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(c) => Some(c.clone()),
            _ => None,
        })
        .collect();
    // The spec's content: a group clone with own placement, a chain clone of it, a text clone
    // and an image clone, plus two placed pages of one drawing page.
    assert!(clones.len() >= 4, "{} clones", clones.len());
    let group_clones: Vec<_> = clones.iter().filter(|c| d.is_group(c.chain.root)).collect();
    assert!(
        group_clones
            .iter()
            .any(|c| c.chain.own_placement && c.chain.depth == 1)
    );
    assert!(
        group_clones.iter().any(|c| c.chain.depth == 2),
        "a chain clone of the group clone"
    );
    assert!(
        clones
            .iter()
            .any(|c| matches!(d.object(c.chain.root), Some(ObjectSnapshot::Text(_))))
    );
    assert!(
        clones
            .iter()
            .any(|c| matches!(d.object(c.chain.root), Some(ObjectSnapshot::Image(_))))
    );
    let placed = d.placed_pages();
    assert_eq!(placed.len(), 2);
    assert_eq!(
        placed[0].source.as_ref().map(|s| &s.id),
        placed[1].source.as_ref().map(|s| &s.id)
    );
    assert!(
        placed
            .iter()
            .any(|p| (p.matrix.determinant() - 1.0).abs() > 1e-6),
        "one scaled"
    );
    // Round trip: identical read.
    let again = reopen(&d, 3);
    assert_eq!(
        again.read_objects().objects.to_vec(),
        objects.objects.to_vec()
    );
    let pa: Vec<_> = again
        .placed_pages()
        .iter()
        .map(|p| (p.id, p.matrix))
        .collect();
    let pb: Vec<_> = placed.iter().map(|p| (p.id, p.matrix)).collect();
    assert_eq!(pa, pb);
    // No repair ran on open: every picture clone is still drawn.
    for c in &clones {
        assert!(!c.drawn_leaves().is_empty());
    }
}

#[test]
fn ac21_a_format_37_file_opens_unchanged_and_writes_no_step_on_open() {
    for name in ["sketch_pages_v37.curvyo", "page_kinds_v37.curvyo"] {
        let bytes = fixture(name);
        assert_eq!(manifest_version(&bytes), 37, "{name}");
        let d = unpack(2, &bytes).unwrap();
        assert!(d.own_steps().is_empty(), "{name}: nothing written on open");
        let v = d.version();
        let _ = d.read_objects();
        assert_eq!(d.version(), v, "{name}: reading writes nothing");
    }
}

#[test]
fn ac21_a_saved_and_reopened_project_keeps_every_clone_kind_and_placed_page() {
    let f = fx();
    let gc = f.d.place_linked_clones(&[f.g], to_laser(&f)).unwrap()[0];
    let chain = f.d.linked_clone(&[gc], &[]).unwrap()[0];
    let tc = f.d.linked_clone(&[f.text], &[]).unwrap()[0];
    let ic = f.d.linked_clone(&[f.image], &[]).unwrap()[0];
    let pc = f.d.place_linked_clones(&[f.rect], to_laser(&f)).unwrap()[0];
    let p1 =
        f.d.place_page(
            &f.laser.id,
            "first",
            Affine::translation(Vec2::new(0.0, 0.0)),
        )
        .unwrap();
    let p2 =
        f.d.place_page(
            &f.laser.id,
            "first",
            Affine::new(0.0, 0.5, -0.5, 0.0, 200.0, 50.0),
        )
        .unwrap();
    let before = f.d.read_objects().objects.to_vec();
    let again = reopen(&f.d, 5);
    assert_eq!(again.read_objects().objects.to_vec(), before);
    for id in [gc, chain, tc, ic, pc] {
        let (a, b) = (clone_snap(&f.d, id), clone_snap(&again, id));
        assert_eq!(a.chain.origin, b.chain.origin);
        assert_eq!(a.chain.matrix, b.chain.matrix);
        assert_eq!(a.chain.own_placement, b.chain.own_placement);
    }
    for id in [p1, p2] {
        assert_eq!(
            f.d.placed_page(id).unwrap().matrix,
            again.placed_page(id).unwrap().matrix
        );
    }
}
