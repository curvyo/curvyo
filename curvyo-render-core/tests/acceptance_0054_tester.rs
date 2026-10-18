//! Independent render tests for `0054-combine-open-paths` criteria 2 and 3: an open outline of a
//! compound path is stroked from first to last node only, and the fill area closes it with a
//! straight line, nonzero over all outlines in stored direction.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::similar_names
)]

use curvyo_document_core::{
    AnchorId, Color, Document, Length, NewAnchor, NodeId, ObjectSnapshot, Point, StyleEdit,
    ViewTransform,
};
use curvyo_render_core::{DrawList, build_artwork};

fn view() -> ViewTransform {
    ViewTransform::new(4.0, Point::new(0.0, 0.0))
}

fn poly(first: u64, pts: &[(f64, f64)]) -> Vec<NewAnchor> {
    pts.iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(4, first + k as u64), Point::new(x, y)))
        .collect()
}

fn u_nodes(dx: f64, dy: f64, reversed: bool) -> Vec<(f64, f64)> {
    let mut v = vec![(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
    if reversed {
        v.reverse();
    }
    v.into_iter().map(|(x, y)| (x + dx, y + dy)).collect()
}

fn compound(d: &Document, outlines: &[(Vec<NewAnchor>, bool)], fill: bool) -> NodeId {
    let seed = d.create_path(&poly(900, &[(800.0, 800.0), (801.0, 800.0)]), false);
    let id = d
        .replace_with_path(&[seed], seed, outlines, "boolean_union")
        .unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeColor(Color { r: 0, g: 0, b: 0 }))
        .unwrap();
    if fill {
        d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
            .unwrap();
        d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
        d.edit_style(&[id], &StyleEdit::StrokeEnabled(false))
            .unwrap();
    }
    id
}

fn artwork(d: &Document) -> DrawList {
    let objects: Vec<ObjectSnapshot> = d
        .object_ids()
        .into_iter()
        .map(|id| d.object(id).unwrap())
        .collect();
    build_artwork(&objects, view())
}

fn covers(list: &DrawList, p: (f64, f64)) -> bool {
    list.triangles.chunks(3).any(|t| {
        let [a, b, c] = [&t[0], &t[1], &t[2]].map(|v| (v.position.x, v.position.y));
        let side =
            |u: (f64, f64), v: (f64, f64)| (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0);
        let (s1, s2, s3) = (side(a, b), side(b, c), side(c, a));
        (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
    })
}

/// The draw list is in document millimetres; the view only scales at paint time.
fn px(x: f64, y: f64) -> (f64, f64) {
    (x, y)
}

#[test]
fn c2_the_open_u_is_stroked_along_its_nodes_and_not_along_its_closing_line() {
    let d = Document::new(1);
    compound(
        &d,
        &[
            (
                poly(
                    100,
                    &[
                        (100.0, 100.0),
                        (120.0, 100.0),
                        (120.0, 120.0),
                        (100.0, 120.0),
                    ],
                ),
                true,
            ),
            (poly(200, &u_nodes(0.0, 0.0, false)), false),
        ],
        false,
    );
    let list = artwork(&d);
    assert!(covers(&list, px(5.0, 10.0)), "the bottom of the U is black");
    assert!(covers(&list, px(0.0, 5.0)), "the left leg");
    assert!(
        !covers(&list, px(5.0, 0.0)),
        "no stroke along the closing line"
    );
    assert!(!covers(&list, px(5.0, 5.0)), "fill is off");
}

#[test]
fn c3_fill_closes_the_open_outline_with_a_straight_line() {
    let d = Document::new(1);
    compound(
        &d,
        &[
            (
                poly(
                    100,
                    &[
                        (100.0, 100.0),
                        (120.0, 100.0),
                        (120.0, 120.0),
                        (100.0, 120.0),
                    ],
                ),
                true,
            ),
            (poly(200, &u_nodes(0.0, 0.0, false)), false),
        ],
        true,
    );
    let list = artwork(&d);
    assert!(covers(&list, px(5.0, 5.0)), "inside the U");
    assert!(!covers(&list, px(5.0, 12.0)), "outside the U");
}

#[test]
fn c3_inside_a_square_the_u_is_a_hole_or_not_by_its_direction() {
    // The square (0,0)-(40,40) runs (0,0),(40,0),(40,40),(0,40): positive shoelace area.
    // The U as written (0,0),(0,10),(10,10),(10,0) runs the other way (negative area).
    let square = [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)];
    let build = |u_reversed: bool| {
        let d = Document::new(1);
        compound(
            &d,
            &[
                (poly(100, &square), true),
                (poly(200, &u_nodes(15.0, 15.0, u_reversed)), false),
            ],
            true,
        );
        artwork(&d)
    };
    let opposite = build(false);
    assert!(
        !covers(&opposite, px(20.0, 20.0)),
        "opposite direction: winding 0, a hole"
    );
    assert!(covers(&opposite, px(5.0, 5.0)));
    let same = build(true);
    assert!(
        covers(&same, px(20.0, 20.0)),
        "same direction: winding 2, painted"
    );
}
