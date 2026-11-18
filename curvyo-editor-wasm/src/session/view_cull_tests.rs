use std::collections::HashSet;

use curvyo_document_core::{
    AnchorId, Length, LineCap, LineJoin, MarkerShape, NewAnchor, NodeId, Point, StyleEdit,
};
use curvyo_render_core::{DrawList, Vertex};

use super::super::Session;

/// A session whose canvas is 800 x 600 CSS pixels, at the default view.
fn session() -> Session {
    let mut session = Session::new(1);
    session.show_default_view();
    session.resize_viewport(800.0, 600.0);
    session
}

/// A path through `points`, stroked `width` mm wide; returns its id.
fn path(session: &Session, points: &[(f64, f64)], closed: bool, width: f64, n: u64) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            NewAnchor::corner(AnchorId::new(9, n * 16 + i as u64), Point::new(x, y))
        })
        .collect();
    let id = session.document.create_path(&anchors, closed);
    session
        .document
        .edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(width)))
        .unwrap();
    id
}

/// The artwork layers of `list`, each as its triangles.
fn layers(list: &DrawList) -> Vec<Vec<Vertex>> {
    let mut start = 0;
    list.layers()
        .iter()
        .map(|&end| {
            let layer = list.triangles[start..end].to_vec();
            start = end;
            layer
        })
        .collect()
}

/// The artwork with and without the view's culling.
fn both(session: &Session) -> (Vec<Vec<Vertex>>, Vec<Vec<Vertex>>) {
    let objects = session.render_objects();
    let view = session.view();
    let guides = HashSet::new();
    let culled = session.artwork_list(&objects, &guides, view, session.visible_rect());
    let whole = session.artwork_list(&objects, &guides, view, None);
    (layers(&culled), layers(&whole))
}

/// A path far outside the canvas adds no triangles; one in view is drawn as without culling.
#[test]
fn a_path_outside_the_canvas_is_not_tessellated() {
    let session = session();
    let rect = session.visible_rect().unwrap();
    let _inside = path(
        &session,
        &[
            (rect.min.x + 5.0, rect.min.y + 5.0),
            (rect.min.x + 20.0, rect.min.y + 9.0),
        ],
        false,
        0.5,
        1,
    );
    let _outside = path(
        &session,
        &[(rect.max.x + 500.0, 0.0), (rect.max.x + 520.0, 10.0)],
        false,
        0.5,
        2,
    );
    let (culled, whole) = both(&session);
    assert_eq!(whole.len(), 2);
    assert_eq!(culled, whole[..1].to_vec());
}

/// No canvas size yet: nothing is left out.
#[test]
fn without_a_canvas_size_nothing_is_left_out() {
    let session = Session::new(1);
    assert!(session.visible_rect().is_none());
}

/// Whether any triangle of `layer` reaches into the rectangle (by its vertices' box).
fn reaches(layer: &[Vertex], min: Point, max: Point) -> bool {
    layer.chunks(3).any(|t| {
        let (lx, hx) = t
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), v| {
                (l.min(v.position.x), h.max(v.position.x))
            });
        let (ly, hy) = t
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), v| {
                (l.min(v.position.y), h.max(v.position.y))
            });
        hx >= min.x && lx <= max.x && hy >= min.y && ly <= max.y
    })
}

/// Every layer that reaches into the view is kept, unchanged and in order, and nothing else
/// is added: the canvas shows the same pixels. Wide strokes, miters, square caps and arrow
/// markers just outside the edges, at several zooms and pans.
#[test]
fn culling_keeps_every_layer_that_reaches_into_the_view() {
    let mut session = session();
    let rect = session.visible_rect().unwrap();
    let mut n = 0;
    for gx in -6..=6 {
        for gy in -6..=6 {
            n += 1;
            let x = rect.min.x + (rect.max.x - rect.min.x) * (f64::from(gx) / 4.0 + 0.5);
            let y = rect.min.y + (rect.max.y - rect.min.y) * (f64::from(gy) / 4.0 + 0.5);
            let width = [0.05, 0.8, 4.0, 15.0][(n % 4) as usize];
            let id = path(
                &session,
                &[(x, y), (x + 9.0, y + 1.0), (x + 1.0, y + 7.0)],
                n % 3 == 0,
                width,
                n,
            );
            let edits = [
                StyleEdit::StrokeJoin(LineJoin::Miter),
                StyleEdit::StrokeCap(if n % 2 == 0 {
                    LineCap::Square
                } else {
                    LineCap::Round
                }),
                StyleEdit::MarkerEnd(if n % 5 == 0 {
                    MarkerShape::Arrow
                } else {
                    MarkerShape::None
                }),
            ];
            for edit in &edits {
                session.document.edit_style(&[id], edit).unwrap();
            }
        }
    }
    let mut culled_any = false;
    for (delta, at) in [
        (0.0, (400.0, 300.0)),
        (-300.0, (10.0, 10.0)),
        (500.0, (790.0, 590.0)),
        (-900.0, (400.0, 300.0)),
    ] {
        session.wheel(0.0, delta, at.0, at.1, false, true);
        let rect = session.visible_rect().unwrap();
        let (culled, whole) = both(&session);
        culled_any |= culled.len() < whole.len();
        let mut remaining = whole.iter();
        for layer in &culled {
            assert!(
                remaining.any(|w| w == layer),
                "a kept layer is one of the whole artwork, in order"
            );
        }
        for layer in whole.iter().filter(|l| reaches(l, rect.min, rect.max)) {
            assert!(
                culled.contains(layer),
                "a layer reaching into the view was left out"
            );
        }
    }
    assert!(culled_any, "the views left something out");
}

/// A gradient fill out of view is still drawn: the frame's gradient spans and stop table
/// carry every shaded leaf (`specs/0124-gradient-fill` criterion 10 reads them).
#[test]
fn a_gradient_fill_out_of_view_is_kept() {
    let session = session();
    let rect = session.visible_rect().unwrap();
    let id = path(
        &session,
        &[
            (rect.max.x + 500.0, 0.0),
            (rect.max.x + 520.0, 10.0),
            (rect.max.x + 500.0, 20.0),
        ],
        true,
        0.5,
        3,
    );
    for edit in [
        StyleEdit::FillEnabled(true),
        StyleEdit::FillPaint(curvyo_document_core::FillPaint::Gradient),
    ] {
        session.document.edit_style(&[id], &edit).unwrap();
    }
    assert!(
        session.render_objects()[0]
            .style()
            .fill
            .painted_gradient()
            .is_some()
    );
    let (culled, whole) = both(&session);
    assert_eq!(whole.len(), 2, "its fill and its stroke");
    assert_eq!(culled, whole);
}

/// A side pane over the canvas's right part does not shrink what is drawn: the GPU draws the
/// whole canvas element, under the pane too.
#[test]
fn a_side_pane_does_not_shrink_the_visible_rectangle() {
    let mut session = session();
    let whole = session.visible_rect().unwrap();
    session.cover_right(300.0);
    let covered = session.visible_rect().unwrap();
    assert!(
        (covered.max.x - whole.max.x).abs() < 1e-9,
        "{covered:?} {whole:?}"
    );
}
