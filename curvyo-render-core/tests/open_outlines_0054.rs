//! Open outlines of a compound path on the canvas (`specs/0054-combine-open-paths` criteria 2
//! and 3): the stroke stays open, the fill closes an open outline with its closing line and
//! follows its stored direction (nonzero).

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
use curvyo_render_core::{
    DrawList, build_artwork, build_live_edit_preview, build_refusal_outlines,
};

fn view() -> ViewTransform {
    ViewTransform::new(4.0, Point::new(0.0, 0.0))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// A closed square outline with ids from `first`, reversed when `reversed`.
fn square(first: u64, x: f64, y: f64, side: f64, reversed: bool) -> (Vec<NewAnchor>, bool) {
    let mut corners = vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
    if reversed {
        corners.reverse();
    }
    let anchors = corners
        .into_iter()
        .enumerate()
        .map(|(k, (cx, cy))| NewAnchor::corner(AnchorId::new(3, first + k as u64), pt(cx, cy)))
        .collect();
    (anchors, true)
}

/// A compound path of the given outlines, with a stroke of width 1 and no
/// fill unless asked for.
fn compound(document: &Document, outlines: &[(Vec<NewAnchor>, bool)], fill: bool) -> NodeId {
    let seed = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), pt(500.0, 500.0)),
            NewAnchor::corner(AnchorId::new(9, 2), pt(501.0, 500.0)),
        ],
        false,
    );
    let id = document
        .replace_with_path(&[seed], seed, outlines, "boolean_union")
        .unwrap();
    document
        .edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
        .unwrap();
    if fill {
        document
            .edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
            .unwrap();
        document
            .edit_style(&[id], &StyleEdit::FillEnabled(true))
            .unwrap();
        document
            .edit_style(&[id], &StyleEdit::StrokeEnabled(false))
            .unwrap();
    }
    id
}

fn artwork(document: &Document) -> DrawList {
    let objects: Vec<ObjectSnapshot> = document
        .object_ids()
        .into_iter()
        .map(|id| document.object(id).unwrap())
        .collect();
    build_artwork(&objects, view())
}

/// Whether the triangles of `layer` cover `p`.
fn covers(list: &DrawList, layer: usize, p: (f64, f64)) -> bool {
    let start = if layer == 0 {
        0
    } else {
        list.layers()[layer - 1]
    };
    let end = list.layers()[layer];
    list.triangles[start..end].chunks(3).any(|t| {
        let [a, b, c] = [&t[0], &t[1], &t[2]].map(|v| (v.position.x, v.position.y));
        let side =
            |u: (f64, f64), v: (f64, f64)| (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0);
        let (s1, s2, s3) = (side(a, b), side(b, c), side(c, a));
        (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
    })
}

/// An open outline through `points` with ids from `first`.
fn open(first: u64, points: &[(f64, f64)]) -> (Vec<NewAnchor>, bool) {
    let anchors = points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(3, first + k as u64), pt(x, y)))
        .collect();
    (anchors, false)
}

/// The U of criterion 2, moved by `(dx, dy)`.
fn u_shape(first: u64, dx: f64, dy: f64) -> (Vec<NewAnchor>, bool) {
    open(
        first,
        &[
            (dx, dy),
            (dx, 10.0 + dy),
            (10.0 + dx, 10.0 + dy),
            (10.0 + dx, dy),
        ],
    )
}

/// Criterion 2: the open U is stroked from its first to its last node; nothing along its
/// closing line.
#[test]
fn c2_an_open_outline_is_stroked_open() {
    let document = Document::new(1);
    compound(
        &document,
        &[
            square(100, 200.0, 200.0, 20.0, false),
            u_shape(200, 0.0, 0.0),
        ],
        false,
    );
    let list = artwork(&document);
    assert!(
        covers(&list, 0, (5.0, 10.0)),
        "the bottom of the U is stroked"
    );
    assert!(
        !covers(&list, 0, (5.0, 0.0)),
        "the closing line is not stroked"
    );
}

/// Criterion 3: the fill closes the U with its closing line; inside a square run the same way the
/// overlap is painted, run the other way it is not.
#[test]
fn c3_the_fill_closes_an_open_outline_and_follows_its_direction() {
    let document = Document::new(1);
    compound(
        &document,
        &[
            square(100, 200.0, 200.0, 20.0, false),
            u_shape(200, 0.0, 0.0),
        ],
        true,
    );
    assert!(covers(&artwork(&document), 0, (5.0, 5.0)));

    // `square(.., false)` runs (0,0) (40,0) (40,40) (0,40); the U runs (15,15) (15,25) (25,25)
    // (25,15), the other way round. Reversed, the square runs as the U does.
    for (square_reversed, painted) in [(true, true), (false, false)] {
        let document = Document::new(1);
        compound(
            &document,
            &[
                square(100, 0.0, 0.0, 40.0, square_reversed),
                u_shape(200, 15.0, 15.0),
            ],
            true,
        );
        assert_eq!(
            covers(&artwork(&document), 0, (20.0, 20.0)),
            painted,
            "square reversed: {square_reversed}"
        );
        assert!(
            covers(&artwork(&document), 0, (5.0, 5.0)),
            "the square itself"
        );
    }
}

/// UX notes, "The closing line is never drawn by an overlay": the live preview of a drag and the
/// refusal outline draw the U open, without a line from (10, 0) to (0, 0).
#[test]
fn overlays_draw_an_open_outline_open() {
    let document = Document::new(1);
    let id = compound(
        &document,
        &[
            square(100, 200.0, 200.0, 20.0, false),
            u_shape(200, 0.0, 0.0),
        ],
        false,
    );
    let object = document.object(id).unwrap();
    let whole = |list: &DrawList, p: (f64, f64)| {
        list.triangles.chunks(3).any(|t| {
            let [a, b, c] = [&t[0], &t[1], &t[2]].map(|v| (v.position.x, v.position.y));
            let side = |u: (f64, f64), v: (f64, f64)| {
                (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0)
            };
            let (s1, s2, s3) = (side(a, b), side(b, c), side(c, a));
            (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
        })
    };
    for (name, list) in [
        (
            "preview",
            build_live_edit_preview(std::slice::from_ref(&object), view()),
        ),
        (
            "refusal",
            build_refusal_outlines(std::slice::from_ref(&object), view()),
        ),
    ] {
        assert!(whole(&list, (5.0, 10.0)), "{name}: the U is drawn");
        assert!(!whole(&list, (5.0, 0.0)), "{name}: no closing line");
    }
}
