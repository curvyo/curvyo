use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{
    AnchorId, DashPattern, Length, MarkerShape, NewAnchor, NodeId, Point, StyleEdit, ViewTransform,
};
use curvyo_render_core::DrawList;
use proptest::prelude::*;

use super::super::{Session, Tool};

/// A session on a canvas of 800 x 600 CSS pixels at the default view, with artwork in and
/// around the view: plain, dashed and arrow-marked strokes, a guide, a linked clone and a
/// gradient fill, in a grid that reaches past every edge.
fn session() -> Session {
    let mut session = Session::new(1);
    session.show_default_view();
    session.resize_viewport(800.0, 600.0);
    let min = session.screen_to_document(0.0, 0.0);
    let max = session.screen_to_document(800.0, 600.0);
    let (width, height) = (max.x - min.x, max.y - min.y);
    let mut ids = Vec::new();
    for gx in -3..=3 {
        for gy in -3..=3 {
            let n = ids.len() as u64 + 1;
            let x = min.x + width * (f64::from(gx) / 3.0 + 0.5);
            let y = min.y + height * (f64::from(gy) / 3.0 + 0.5);
            ids.push(path(
                &session,
                &[(x, y), (x + 9.0, y + 1.0), (x + 1.0, y + 7.0)],
                n.is_multiple_of(3),
                n,
            ));
        }
    }
    let dashed = DashPattern::new(vec![2.0, 1.0]).unwrap();
    for (i, &id) in ids.iter().enumerate() {
        let edit = match i % 5 {
            0 => StyleEdit::StrokeDash(dashed.clone()),
            1 => StyleEdit::MarkerEnd(MarkerShape::Arrow),
            2 => StyleEdit::StrokeWidth(Length::from_mm(3.0)),
            _ => continue,
        };
        session.document.edit_style(&[id], &edit).unwrap();
    }
    session.document.set_guide(&[ids[8]], true).unwrap();
    for edit in [
        StyleEdit::FillEnabled(true),
        StyleEdit::FillPaint(curvyo_document_core::FillPaint::Gradient),
    ] {
        session.document.edit_style(&[ids[24]], &edit).unwrap();
    }
    session.selection.set(&[ids[16], ids[30]]);
    assert_eq!(
        session.linked_clone().kind,
        super::super::clones::CloneKind::Done
    );
    session.selection.set(&[]);
    session
}

/// A path through `points`, stroked 0.5 mm wide.
fn path(session: &Session, points: &[(f64, f64)], closed: bool, n: u64) -> NodeId {
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
        .edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(0.5)))
        .unwrap();
    id
}

/// The frame as a session without a kept artwork builds it; the memo is left as it was.
fn fresh_frame(session: &Session) -> DrawList {
    let held = std::mem::take(&mut *session.artwork_memo.borrow_mut());
    let list = session.frame_draw_list();
    *session.artwork_memo.borrow_mut() = held;
    list
}

/// The artwork as the uncached path builds it: the read culled by `in_view` and tessellated.
fn uncached_artwork(session: &Session) -> DrawList {
    let guides = Arc::clone(&session.document_read().guides);
    session.artwork_list(
        &session.render_objects(),
        &guides,
        session.view(),
        session.visible_rect(),
    )
}

/// The artwork the session's own read path builds now (kept or built).
fn read_artwork(session: &Session) -> DrawList {
    let guides = Arc::clone(&session.document_read().guides);
    session.read_artwork_list(
        &session.render_objects(),
        &guides,
        session.view(),
        session.visible_rect(),
    )
}

fn hits(session: &Session) -> u64 {
    session.artwork_memo.borrow().hits
}

fn pan(session: &mut Session, dx: f64, dy: f64) {
    session.begin_pan(400.0, 300.0);
    session.pan_to(400.0 + dx, 300.0 + dy);
    session.end_pan();
}

/// The precondition of the memo: the tessellation reads the view's scale only. The same
/// objects at the same scale and another origin give the same list, bit for bit.
#[test]
fn the_artwork_does_not_depend_on_the_view_origin() {
    let session = session();
    let objects = session.render_objects();
    let guides = Arc::clone(&session.document_read().guides);
    let scale = session.view().scale();
    let at = |x, y| {
        session.artwork_list(
            &objects,
            &guides,
            ViewTransform::new(scale, Point::new(x, y)),
            None,
        )
    };
    let here = at(0.0, 0.0);
    assert!(here.triangle_count() > 0);
    assert_eq!(here, at(123.456_789, -98.765_432_1));
    assert_eq!(here, at(-1.0e5, 3.3e4));
}

/// A pan at the scale of the frames before draws the kept artwork, and the frame equals the
/// one a session without a kept list builds: the same triangles, layers, depths and overlay.
#[test]
fn a_pan_draws_the_kept_artwork_as_a_fresh_build_does() {
    let mut session = session();
    // Zoom out so that every object is in view: a small pan keeps them all.
    session.wheel(0.0, 300.0, 400.0, 300.0, false, true);
    // The first frame of a view keeps nothing (a zoom would copy its list in vain), the second
    // keeps its list.
    let _ = session.frame_draw_list();
    let _ = session.frame_draw_list();
    assert_eq!(
        hits(&session),
        0,
        "nothing was kept before the second frame"
    );
    pan(&mut session, 7.25, -3.5);
    let frame = session.frame_draw_list();
    assert_eq!(hits(&session), 1, "a pan draws the kept list");
    let fresh = fresh_frame(&session);
    assert_eq!(frame, fresh);
    assert_eq!(frame.vertex_depths(), fresh.vertex_depths());
    assert_eq!(read_artwork(&session), uncached_artwork(&session));
    pan(&mut session, -30.0, 12.5);
    let frame = session.frame_draw_list();
    assert_eq!(
        hits(&session),
        3,
        "the read path above and the next pan draw the kept list"
    );
    assert_eq!(frame, fresh_frame(&session));
}

/// Anything but a pan builds the artwork afresh: a zoom, a document edit, a selection, a tool,
/// a hover or a canvas size; each frame still equals a fresh build.
#[test]
fn anything_but_a_pan_builds_the_artwork_afresh() {
    type Change = fn(&mut Session, NodeId);
    let mut session = session();
    session.wheel(0.0, 300.0, 400.0, 300.0, false, true);
    let first = session.render_objects()[0].id();
    let changes: [(&str, Change); 6] = [
        ("zoom", |s, _| {
            s.wheel(0.0, -100.0, 400.0, 300.0, false, true);
        }),
        ("edit", |s, id| {
            s.document
                .edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.25)))
                .unwrap();
        }),
        ("selection", |s, id| s.selection.set(&[id])),
        ("tool", |s, _| s.tool = Tool::Node),
        ("hover", |s, _| {
            let at = s.screen_to_document(401.0, 301.0);
            s.pointer_hover(at, false, false);
        }),
        ("size", |s, _| s.resize_viewport(790.0, 600.0)),
    ];
    for (name, change) in changes {
        let start = hits(&session);
        for _ in 0..3 {
            let _ = session.frame_draw_list();
        }
        let before = hits(&session);
        assert!(before > start, "a list was kept before the {name}");
        change(&mut session, first);
        let frame = session.frame_draw_list();
        assert_eq!(hits(&session), before, "{name} drew the kept artwork");
        assert_eq!(frame, fresh_frame(&session), "{name}");
        session.tool = Tool::Select;
    }
}

/// A style preview (a panel drag) draws other objects than the read: never the kept list.
#[test]
fn a_style_preview_never_draws_the_kept_artwork() {
    let mut session = session();
    let id = session.render_objects()[0].id();
    let _ = session.frame_draw_list();
    let _ = session.frame_draw_list();
    let before = hits(&session);
    session.selection.set(&[id]);
    for _ in 0..3 {
        let _ = session.frame_draw_list();
    }
    let before_preview = hits(&session);
    assert!(before_preview > before);
    session.preview_style_hsv(curvyo_ui_core::StyleField::StrokeColor, 200.0, 0.8, 0.6);
    let frame = session.frame_draw_list();
    assert_eq!(
        hits(&session),
        before_preview,
        "the preview drew the kept artwork"
    );
    assert_eq!(frame, fresh_frame(&session));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Over random pans at random zooms, every frame equals a fresh build bit for bit, and
    /// the read path's artwork equals the uncached one (`in_view` and the tessellation). Two
    /// small pans follow each jump, so the kept list is drawn and compared too.
    #[test]
    fn every_pan_frame_equals_a_fresh_build(
        zoom in -400.0..300.0_f64,
        pans in prop::collection::vec((-900.0..900.0_f64, -700.0..700.0_f64), 1..10),
    ) {
        let mut session = session();
        session.wheel(0.0, zoom, 400.0, 300.0, false, true);
        let _ = session.frame_draw_list();
        let start = hits(&session);
        for (dx, dy) in pans {
            pan(&mut session, dx, dy);
            let frame = session.frame_draw_list();
            prop_assert_eq!(&frame, &fresh_frame(&session));
            prop_assert_eq!(read_artwork(&session), uncached_artwork(&session));
            // Small pans of the same view: the case the memo serves most.
            for _ in 0..2 {
                pan(&mut session, dx.signum() * 0.75, dy.signum() * 0.5);
                let frame = session.frame_draw_list();
                prop_assert_eq!(&frame, &fresh_frame(&session));
            }
        }
        prop_assert!(hits(&session) > start, "no frame drew the kept list");
    }
}

/// The read holds one guide set between frames, so the memo's key can compare it by identity.
#[test]
fn the_read_keeps_one_guide_set_between_frames() {
    let session = Session::new(1);
    let guides: Arc<HashSet<NodeId>> = Arc::clone(&session.document_read().guides);
    assert!(Arc::ptr_eq(&guides, &session.document_read().guides));
}
