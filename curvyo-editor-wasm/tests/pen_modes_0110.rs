//! Session-level Pen mode interactions.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

use curvyo_document_core::{Point, unpack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::PenMode;

fn draw_once(session: &mut Session) {
    let _ = session.draw_list();
}

#[test]
fn freehand_pointer_drag_commits_one_path() {
    let mut session = Session::new(1);
    session.set_tool(Tool::Pen);
    assert!(session.set_pen_mode(PenMode::Freehand));
    session.pointer_down(Point::new(10.0, 10.0), false);
    session.pointer_hover(Point::new(20.0, 20.0), false, false);
    session.pointer_hover(Point::new(30.0, 10.0), false, false);
    session.pointer_up(Point::new(40.0, 10.0), false, false);
    let document = unpack(2, &session.pack("test").unwrap()).unwrap();
    let paths = document.object_ids();
    assert_eq!(paths.len(), 1);
    assert!(document.path(paths[0]).unwrap().anchors.len() >= 3);
}

#[test]
fn bspline_preview_and_committed_artwork_are_drawable() {
    use curvyo_document_core::CurveKind;
    let mut session = Session::new(3);
    session.set_tool(Tool::Pen);
    assert!(session.set_pen_mode(PenMode::BSpline));
    for point in [
        Point::new(10.0, 10.0),
        Point::new(30.0, 10.0),
        Point::new(30.0, 30.0),
    ] {
        session.pointer_down(point, false);
        session.pointer_up(point, false, false);
    }
    session.pointer_hover(Point::new(10.0, 30.0), false, false);
    let preview = session.draw_list();
    assert!(preview.triangle_count() > 0);
    session.finish_pen();
    let document = unpack(4, &session.pack("test").unwrap()).unwrap();
    let id = document.object_ids()[0];
    assert_eq!(document.path(id).unwrap().curve_kind, CurveKind::BSpline);
    assert!(session.draw_list().triangle_count() > 0);
}

#[test]
fn bspline_artwork_differs_from_control_polygon() {
    fn draw(mode: PenMode) -> curvyo_render_core::DrawList {
        let mut session = Session::new(9);
        session.set_tool(Tool::Pen);
        assert!(session.set_pen_mode(mode));
        for point in [
            Point::new(10.0, 10.0),
            Point::new(30.0, 10.0),
            Point::new(30.0, 30.0),
            Point::new(10.0, 30.0),
        ] {
            session.pointer_down(point, false);
            session.pointer_up(point, false, false);
        }
        session.finish_pen();
        session.set_tool(Tool::Select);
        session.draw_list()
    }
    assert_ne!(draw(PenMode::BSpline), draw(PenMode::Lines));
}

#[test]
fn svg_exports_bspline_as_its_evaluated_cubic() {
    use curvyo_editor_wasm::SvgExportResult;
    let mut session = Session::new(10);
    session.set_tool(Tool::Pen);
    assert!(session.set_pen_mode(PenMode::BSpline));
    for point in [
        Point::new(10.0, 10.0),
        Point::new(30.0, 10.0),
        Point::new(30.0, 30.0),
        Point::new(10.0, 30.0),
    ] {
        session.pointer_down(point, false);
        session.pointer_up(point, false, false);
    }
    session.finish_pen();
    let SvgExportResult::Written { text, .. } =
        session.export_svg(curvyo_ui_core::SvgExportOptions::default())
    else {
        panic!("SVG not written")
    };
    assert!(text.contains("C 30 10 30 30 10 30"), "{text}");
}

#[test]
fn svg_exports_a_supported_variable_width_as_a_filled_outline() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    use curvyo_editor_wasm::SvgExportResult;
    let doc = Document::new(8);
    let path = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(8, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(8, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(path, AnchorId::new(8, 3), 0.5, 2.0)
        .unwrap();
    doc.set_width_point(path, AnchorId::new(8, 4), 0.0, 1.0)
        .unwrap();
    let mut session = Session::open(9, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    let SvgExportResult::Written { text: svg, .. } =
        session.export_svg(curvyo_ui_core::SvgExportOptions::default())
    else {
        panic!("the straight variable stroke should export");
    };
    assert!(svg.contains("<path"));
    assert!(!svg.contains("stroke-width"));
}

#[test]
fn svg_refuses_a_variable_stroke_without_length() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    use curvyo_editor_wasm::SvgExportResult;
    let doc = Document::new(8);
    // Only degenerate input has no outline (ADR 0053 amendment item 5).
    let path = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(8, 1), Point::new(5.0, 5.0)),
            NewAnchor::corner(AnchorId::new(8, 2), Point::new(5.0, 5.0)),
        ],
        false,
    );
    doc.set_width_point(path, AnchorId::new(8, 3), 0.5, 2.0)
        .unwrap();
    doc.set_width_point(path, AnchorId::new(8, 4), 0.0, 1.0)
        .unwrap();
    let session = Session::open(9, &pack(&doc, "test").unwrap()).unwrap();
    assert!(matches!(
        session.export_svg(curvyo_ui_core::SvgExportOptions::default()),
        SvgExportResult::UnsupportedExtendedPath
    ));
}

#[test]
fn svg_selection_ignores_unsupported_profiles_outside_selection() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    use curvyo_editor_wasm::SvgExportResult;
    let doc = Document::new(79);
    let _ordinary = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(79, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(79, 2), Point::new(30.0, 10.0)),
        ],
        false,
    );
    // No length, so no outline: the one refused case (ADR 0053 amendment item 5).
    let unsupported = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(79, 3), Point::new(80.0, 80.0)),
            NewAnchor::corner(AnchorId::new(79, 4), Point::new(80.0, 80.0)),
        ],
        false,
    );
    doc.set_width_point(unsupported, AnchorId::new(79, 5), 0.5, 2.0)
        .unwrap();
    doc.set_width_point(unsupported, AnchorId::new(79, 6), 0.0, 1.0)
        .unwrap();
    let mut session = Session::open(80, &pack(&doc, "test").unwrap()).unwrap();
    session.pointer_down(Point::new(20.0, 10.0), false);
    session.pointer_up(Point::new(20.0, 10.0), false, false);
    let options = curvyo_ui_core::SvgExportOptions {
        selection_only: true,
        ..Default::default()
    };
    assert!(matches!(
        session.export_svg(options),
        SvgExportResult::Written { .. }
    ));
    assert!(matches!(
        session.export_svg(curvyo_ui_core::SvgExportOptions::default()),
        SvgExportResult::UnsupportedExtendedPath
    ));
}

#[test]
fn svg_active_page_ignores_unsupported_profiles_on_another_page() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    use curvyo_editor_wasm::SvgExportResult;
    let doc = Document::new(80);
    let _ = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(80, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(80, 2), Point::new(30.0, 10.0)),
        ],
        false,
    );
    let second = doc.add_page();
    // No length, so no outline: the one refused case (ADR 0053 amendment item 5).
    let at = Point::new(second.origin.x + 10.0, second.origin.y + 10.0);
    let unsupported = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(80, 3), at),
            NewAnchor::corner(AnchorId::new(80, 4), at),
        ],
        false,
    );
    doc.set_width_point(unsupported, AnchorId::new(80, 5), 0.5, 2.0)
        .unwrap();
    doc.set_width_point(unsupported, AnchorId::new(80, 6), 0.0, 1.0)
        .unwrap();
    let session = Session::open(81, &pack(&doc, "test").unwrap()).unwrap();
    let options = curvyo_ui_core::SvgExportOptions {
        active_page_only: true,
        ..Default::default()
    };
    assert!(matches!(
        session.export_svg(options),
        SvgExportResult::Written { .. }
    ));
    assert!(matches!(
        session.export_svg(curvyo_ui_core::SvgExportOptions::default()),
        SvgExportResult::UnsupportedExtendedPath
    ));
}

#[test]
fn variable_stroke_to_path_preserves_fill_and_undo_redo() {
    use curvyo_document_core::{AnchorId, Color, Document, NewAnchor, StyleEdit, pack};
    use curvyo_editor_wasm::StrokeToPathOutcome;
    let doc = Document::new(71);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(71, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(71, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.edit_style(&[id], &StyleEdit::FillEnabled(true))
        .unwrap();
    doc.edit_style(
        &[id],
        &StyleEdit::StrokeColor(Color {
            r: 200,
            g: 10,
            b: 0,
        }),
    )
    .unwrap();
    doc.set_width_point(id, AnchorId::new(71, 3), 0.5, 2.0)
        .unwrap();
    let before = doc.path(id).unwrap();
    let mut session = Session::open(72, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { .. }
    ));
    let after = unpack(73, &session.pack("test").unwrap()).unwrap();
    assert_eq!(after.object_ids().len(), 2);
    let fill = after.path(id).unwrap();
    assert_eq!(fill.style.fill, before.style.fill);
    assert!(!fill.style.stroke.enabled);
    assert!(fill.width_points.is_empty());
    let outline = after
        .path(
            *after
                .object_ids()
                .iter()
                .find(|&&other| other != id)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(outline.style.fill.color, before.style.stroke.color);
    assert!(!outline.style.stroke.enabled);
    let _ = session.undo();
    let restored = unpack(74, &session.pack("test").unwrap()).unwrap();
    assert_eq!(restored.path(id).unwrap(), before);
    let _ = session.redo();
    let redone = unpack(75, &session.pack("test").unwrap()).unwrap();
    assert_eq!(redone.object_ids().len(), 2);
    assert!(redone.path(id).unwrap().width_points.is_empty());
}

#[test]
fn a_curved_variable_stroke_is_drawn_and_converts_to_its_outline() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Vec2, pack};
    use curvyo_editor_wasm::StrokeToPathOutcome;
    let doc = Document::new(76);
    let mut a = NewAnchor::corner(AnchorId::new(76, 1), Point::new(10.0, 10.0));
    a.handle_out = Vec2::new(5.0, 5.0);
    let id = doc.create_path(
        &[
            a,
            NewAnchor::corner(AnchorId::new(76, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(76, 3), 0.5, 2.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(76, 4), 0.0, 1.0)
        .unwrap();
    let before = doc.path(id).unwrap();
    let mut session = Session::open(77, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    let _ = session.draw_list();
    assert!(session.take_unsupported_width_notice().is_empty());
    session.key_down(curvyo_editor_wasm::KeyInput {
        key: "a",
        ctrl: true,
        ..Default::default()
    });
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { strokes: 1, .. }
    ));
    let after = unpack(78, &session.pack("test").unwrap()).unwrap();
    assert_ne!(after.path(id), Some(before));
}

#[test]
fn width_canvas_click_creates_a_point_and_reset_is_undoable() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    let doc = Document::new(20);
    let _ = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(20, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(20, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    let mut session = Session::open(21, &pack(&doc, "test").unwrap()).unwrap();
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert!(session.begin_width_edit());
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    draw_once(&mut session);
    let saved = unpack(22, &session.pack("test").unwrap()).unwrap();
    let id = saved.object_ids()[0];
    assert_eq!(saved.width_points(id).unwrap().len(), 1);
    assert!(session.reset_width_edit());
    let saved = unpack(23, &session.pack("test").unwrap()).unwrap();
    assert!(saved.width_points(id).unwrap().is_empty());
}

#[test]
fn width_canvas_drag_commits_width_and_delete_removes_point() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    let doc = Document::new(30);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(30, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(30, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(30, 3), 0.5, 1.0)
        .unwrap();
    let mut session = Session::open(31, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert!(session.begin_width_edit());
    // The diamond sits on the centre line; a drag across adds twice the distance moved to the
    // width at the press (UX review U3).
    let base = doc.path(id).unwrap().style.stroke.width.as_mm();
    let edge = 10.0 + 1.5 * base.max(1.0);
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_hover(Point::new(30.0, edge), false, false);
    session.pointer_up(Point::new(30.0, edge), false, false);
    draw_once(&mut session);
    let saved = unpack(32, &session.pack("test").unwrap()).unwrap();
    let expected = 1.0 + 2.0 * (edge - 10.0) / base;
    assert!((saved.width_points(id).unwrap()[0].multiplier - expected).abs() < 1e-6);
    session.delete_selected();
    let saved = unpack(33, &session.pack("test").unwrap()).unwrap();
    assert!(saved.width_points(id).unwrap().is_empty());
    assert!(saved.path(id).is_some());
}

#[test]
fn width_drag_escape_discards_preview_and_preserves_path() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    let doc = Document::new(40);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(40, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(40, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(40, 3), 0.5, 1.0)
        .unwrap();
    let mut session = Session::open(41, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert!(session.begin_width_edit());
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_hover(Point::new(30.0, 12.0), false, false);
    session.escape();
    session.pointer_up(Point::new(30.0, 12.0), false, false);
    let saved = unpack(42, &session.pack("test").unwrap()).unwrap();
    assert_eq!(saved.width_points(id).unwrap()[0].multiplier, 1.0);
    assert!(session.width_edit_active());
    session.escape();
    assert!(!session.width_edit_active());
}

#[test]
fn width_drag_changes_artwork_before_the_document_is_written() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    let doc = Document::new(80);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(80, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(80, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(80, 3), 0.5, 1.0)
        .unwrap();
    let mut session = Session::open(81, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert!(session.begin_width_edit());
    let before = session.draw_list();
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_hover(Point::new(30.0, 12.0), false, false);
    draw_once(&mut session);
    let during = session.draw_list();
    let before_end = *before.layers().last().unwrap();
    let during_end = *during.layers().last().unwrap();
    assert_ne!(
        &before.triangles[..before_end],
        &during.triangles[..during_end]
    );
    let saved = unpack(82, &session.pack("test").unwrap()).unwrap();
    assert_eq!(saved.width_points(id).unwrap()[0].multiplier, 1.0);
}

#[test]
fn a_zero_width_point_stays_when_the_zoom_changes_during_a_press() {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, pack};
    let doc = Document::new(92);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(92, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(92, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(92, 3), 0.5, 0.0)
        .unwrap();
    let mut session = Session::open(93, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    session.key_down(curvyo_editor_wasm::KeyInput {
        key: "a",
        ctrl: true,
        ..Default::default()
    });
    assert!(session.begin_width_edit());
    session.pointer_down(Point::new(30.0, 10.0), false);
    assert!(session.restore_view_text("2 0 0"));
    session.pointer_hover(Point::new(30.0, 10.0), false, false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    draw_once(&mut session);
    let saved = unpack(94, &session.pack("test").unwrap()).unwrap();
    assert_eq!(saved.width_points(id).unwrap()[0].multiplier, 0.0);
}

#[test]
fn bspline_control_node_drag_preserves_curve_and_updates_artwork() {
    use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, pack};
    let doc = Document::new(101);
    let controls = [(10., 10.), (30., 10.), (30., 30.), (10., 30.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(101, i as u64), Point::new(x, y));
            a.kind = AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let mut session = Session::open(102, &pack(&doc, "test").unwrap()).unwrap();
    session.pointer_down(Point::new(25., 20.), false);
    session.pointer_up(Point::new(25., 20.), false, false);
    session.set_tool(Tool::Node);
    let before = session.draw_list();
    session.pointer_down(Point::new(30., 10.), false);
    session.pointer_hover(Point::new(35., 5.), false, false);
    session.pointer_up(Point::new(35., 5.), false, false);
    let after = session.draw_list();
    let saved = unpack(103, &session.pack("test").unwrap()).unwrap();
    let path = saved.path(id).unwrap();
    assert_eq!(path.curve_kind, CurveKind::BSpline);
    assert_eq!(path.anchors[1].point, Point::new(35., 5.));
    assert_ne!(before, after);
}

#[test]
fn bspline_conversion_preserves_artwork_identity_and_undo() {
    use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, pack};
    let doc = Document::new(110);
    let controls = [(10., 10.), (30., 10.), (30., 30.), (10., 30.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(110, i as u64), Point::new(x, y));
            a.kind = AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let original = doc.path(id).unwrap();
    let expected = curvyo_ui_core::evaluate_path(&original).unwrap();
    let mut session = Session::open(111, &pack(&doc, "test").unwrap()).unwrap();
    session.pointer_down(Point::new(25., 20.), false);
    session.pointer_up(Point::new(25., 20.), false, false);
    assert!(session.can_convert_selected_bspline_to_bezier());
    assert!(session.convert_selected_bspline_to_bezier());
    assert!(!session.can_convert_selected_bspline_to_bezier());
    let saved = unpack(112, &session.pack("test").unwrap()).unwrap();
    let result = saved.path(id).unwrap();
    assert_eq!(result.curve_kind, CurveKind::Bezier);
    assert_eq!(result.style, original.style);
    for (a, b) in result.anchors.iter().zip(&expected.anchors) {
        assert_eq!(
            (a.point, a.handle_in, a.handle_out),
            (b.point, b.handle_in, b.handle_out)
        );
    }
    session.undo();
    assert_eq!(
        unpack(113, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        original
    );
    session.redo();
    assert_eq!(
        unpack(114, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        result
    );
}

#[test]
fn bspline_conversion_keeps_relative_width_points() {
    use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, pack};
    let doc = Document::new(120);
    let controls = [(10., 10.), (30., 10.), (30., 30.), (10., 30.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(120, i as u64), Point::new(x, y));
            a.kind = AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(120, 20), 0.4, 2.0)
        .unwrap();
    let before = doc.width_points(id).unwrap();
    let mut session = Session::open(121, &pack(&doc, "test").unwrap()).unwrap();
    session.key_down(curvyo_editor_wasm::KeyInput {
        key: "a",
        ctrl: true,
        ..Default::default()
    });
    assert!(session.convert_selected_bspline_to_bezier());
    let saved = unpack(122, &session.pack("test").unwrap()).unwrap();
    assert_eq!(saved.path(id).unwrap().curve_kind, CurveKind::Bezier);
    assert_eq!(saved.width_points(id).unwrap(), before);
    session.undo();
    let restored = unpack(123, &session.pack("test").unwrap()).unwrap();
    assert_eq!(restored.path(id).unwrap().curve_kind, CurveKind::BSpline);
    assert_eq!(restored.width_points(id).unwrap(), before);
}

#[test]
fn closed_bspline_conversion_preserves_closed_geometry() {
    use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, pack};
    let doc = Document::new(130);
    let controls = [(10., 10.), (30., 10.), (30., 30.), (10., 30.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(130, i as u64), Point::new(x, y));
            a.kind = AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, true, CurveKind::BSpline)
        .unwrap();
    let source = doc.path(id).unwrap();
    let expected = curvyo_ui_core::evaluate_path(&source).unwrap();
    let mut session = Session::open(131, &pack(&doc, "test").unwrap()).unwrap();
    session.pointer_down(Point::new(20., 10.833_333_333_333_3), false);
    session.pointer_up(Point::new(20., 10.833_333_333_333_3), false, false);
    assert!(session.convert_selected_bspline_to_bezier());
    let saved = unpack(132, &session.pack("test").unwrap()).unwrap();
    let result = saved.path(id).unwrap();
    assert!(result.closed);
    assert_eq!(result.anchors.len(), expected.anchors.len());
    for (a, b) in result.anchors.iter().zip(&expected.anchors) {
        assert_eq!(
            (a.point, a.handle_in, a.handle_out),
            (b.point, b.handle_in, b.handle_out)
        );
    }
}

#[test]
fn deleting_required_closed_spline_control_reports_reason() {
    use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, pack};
    let d = Document::new(140);
    let controls = [(10., 10.), (30., 10.), (20., 30.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(140, i as u64), Point::new(x, y));
            a.kind = AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = d
        .create_editable_curve(&controls, true, CurveKind::BSpline)
        .unwrap();
    let mut session = Session::open(141, &pack(&d, "test").unwrap()).unwrap();
    session.pointer_down(Point::new(20., 13.333_333_333_333_3), false);
    session.pointer_up(Point::new(20., 13.333_333_333_333_3), false, false);
    session.set_tool(Tool::Node);
    session.pointer_down(Point::new(10., 10.), false);
    session.pointer_up(Point::new(10., 10.), false, false);
    session.delete_selected();
    assert!(
        session
            .take_history_notice()
            .unwrap_or_default()
            .contains("at least three")
    );
    assert_eq!(
        unpack(142, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap()
            .anchors
            .len(),
        3
    );
}

/// Whether `list` has a vertex in the box, in document mm.
fn ink_in(list: &curvyo_render_core::DrawList, x: (f64, f64), y: (f64, f64)) -> bool {
    list.triangles
        .iter()
        .any(|v| (x.0..x.1).contains(&v.position.x) && (y.0..y.1).contains(&v.position.y))
}

/// Criterion 2 (demo finding of 2026-11-10): a Spiro is drawn while its points are placed and
/// after Enter, with nothing computed elsewhere, and it is selected and moved like any path.
#[test]
fn spiro_is_drawn_live_and_after_finishing_without_any_background_job() {
    let mut session = Session::new(31);
    session.set_tool(Tool::Pen);
    assert!(session.set_pen_mode(PenMode::Spiro));
    let points = [
        Point::new(10.0, 20.0),
        Point::new(30.0, 10.0),
        Point::new(50.0, 20.0),
        Point::new(70.0, 10.0),
    ];
    for point in points {
        session.pointer_down(point, false);
        session.pointer_up(point, false, false);
    }
    session.pointer_hover(Point::new(90.0, 20.0), false, false);
    // Between the first two nodes, away from their markers: only the curve can put ink there.
    let between = ((16.0, 24.0), (5.0, 25.0));
    assert!(
        ink_in(&session.draw_list(), between.0, between.1),
        "the curve is drawn while points are placed"
    );
    session.finish_pen();
    let document = unpack(32, &session.pack("test").unwrap()).unwrap();
    let id = document.object_ids()[0];
    assert_eq!(
        document.path(id).unwrap().curve_kind,
        curvyo_document_core::CurveKind::Spiro
    );
    assert!(
        ink_in(&session.draw_list(), between.0, between.1),
        "the finished curve is drawn"
    );
    session.set_tool(Tool::Select);
    session.pointer_down(points[1], false);
    session.pointer_up(points[1], false, false);
    assert_eq!(
        session.selected_object_count(),
        1,
        "a click on the curve selects it"
    );
}

/// Criterion 2: Spiro exports as its evaluated curve, as B-spline does.
#[test]
fn svg_exports_spiro_as_its_evaluated_cubic() {
    use curvyo_document_core::{AnchorId, CurveKind, Document, NewAnchor, pack};
    use curvyo_editor_wasm::SvgExportResult;
    let doc = Document::new(11);
    let controls = [
        NewAnchor::corner(AnchorId::new(11, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(11, 2), Point::new(10.0, 5.0)),
        NewAnchor::corner(AnchorId::new(11, 3), Point::new(20.0, 0.0)),
    ]
    .map(|mut anchor| {
        anchor.kind = curvyo_document_core::AnchorKind::Symmetric;
        anchor
    });
    doc.create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let session = Session::open(12, &pack(&doc, "test").unwrap()).unwrap();
    let SvgExportResult::Written { text, .. } =
        session.export_svg(curvyo_ui_core::SvgExportOptions::default())
    else {
        panic!("a Spiro exports");
    };
    assert!(text.contains(" C"), "{text}");
}

/// Places Spiro points at `points` and hovers at `pointer`, drawing each step.
fn spiro_placed(points: &[(f64, f64)], pointer: (f64, f64)) -> Session {
    let mut session = Session::new(41);
    session.set_tool(Tool::Pen);
    assert!(session.set_pen_mode(PenMode::Spiro));
    for &(x, y) in points {
        let at = Point::new(x, y);
        session.pointer_hover(at, false, false);
        session.pointer_down(at, false);
        session.pointer_up(at, false, false);
        draw_once(&mut session);
    }
    session.pointer_hover(Point::new(pointer.0, pointer.1), false, false);
    draw_once(&mut session);
    session
}

/// Five zigzag points that solve into loops; with the pointer as a sixth point they have no
/// solution (the UX review's case of 2026-11-10).
const ZIGZAG: [(f64, f64); 5] = [
    (10.0, 60.0),
    (50.0, 30.0),
    (85.0, 65.0),
    (120.0, 35.0),
    (143.0, 70.0),
];

/// Lead decision 2026-11-11 (Proposal, replaces the rule that a click without a near solution
/// places nothing): every Spiro click places a control, the zigzag's looping fifth one too, with
/// the curve solved from nothing; there is no notice, and Enter commits a Spiro that solves.
#[test]
fn every_spiro_click_places_a_control_even_where_the_curve_loops() {
    let mut session = spiro_placed(&ZIGZAG, (160.0, 98.0));
    let placed: Vec<_> = session
        .pen_in_progress()
        .unwrap()
        .iter()
        .map(|a| (a.point.x, a.point.y))
        .collect();
    assert_eq!(placed, ZIGZAG.to_vec(), "every click is a control");
    assert_eq!(session.take_unsupported_width_notice(), "", "no notice");
    session.finish_pen();
    let document = unpack(42, &session.pack("test").unwrap()).unwrap();
    let id = document.object_ids()[0];
    let path = document.path(id).unwrap();
    assert_eq!(path.curve_kind, curvyo_document_core::CurveKind::Spiro);
    assert!(
        curvyo_ui_core::evaluate_spiro_nodes(&path.anchors, false, curvyo_ui_core::SPIRO_TOLERANCE)
            .is_ok(),
        "the stored curve solves"
    );
}

/// A Spiro that solves at Enter is committed as a Spiro.
#[test]
fn a_solvable_spiro_is_committed_as_a_spiro() {
    let mut session = spiro_placed(&ZIGZAG[..3], (85.0, 65.0));
    assert_eq!(session.take_unsupported_width_notice(), "");
    session.finish_pen();
    let document = unpack(43, &session.pack("test").unwrap()).unwrap();
    let id = document.object_ids()[0];
    assert_eq!(
        document.path(id).unwrap().curve_kind,
        curvyo_document_core::CurveKind::Spiro
    );
}

/// `specs/0115` criterion 1: the drag that stopped at a fold of the G4 curve (at 31 mm) follows
/// the pointer to its end with the curvature-continuous curve; the release writes the pointer's
/// place, and the stored curve is the one shown.
#[test]
fn a_node_drag_of_a_spiro_point_follows_past_the_old_fold() {
    use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, pack};
    let doc = Document::new(121);
    let controls: Vec<_> = [(10.0, 40.0), (30.0, 50.0), (50.0, 31.0), (70.0, 50.0)]
        .into_iter()
        .zip(1..)
        .map(|((x, y), i)| {
            let mut a = NewAnchor::corner(AnchorId::new(121, i), Point::new(x, y));
            a.kind = AnchorKind::Symmetric;
            a
        })
        .collect();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let mut session = Session::open(122, &pack(&doc, "test").unwrap()).unwrap();
    session.set_tool(Tool::Node);
    draw_once(&mut session);
    session.pointer_down(Point::new(50.0, 31.0), false);
    for step in 1..=40 {
        let at = Point::new(50.0, 31.0 - 0.5 * f64::from(step));
        session.pointer_hover(at, false, false);
        draw_once(&mut session);
    }
    session.pointer_up(Point::new(50.0, 11.0), false, false);
    let stored = unpack(123, &session.pack("test").unwrap()).unwrap();
    let moved = stored.path(id).unwrap().anchors[2].point;
    assert!(
        (moved.x - 50.0).abs() < 1e-9 && (moved.y - 11.0).abs() < 1e-9,
        "{moved:?}"
    );
    draw_once(&mut session);
}

/// UX review U10: a Spiro or B-spline point is a corner or smooth; it has no handles to make
/// asymmetric, so the Node tool does not offer it.
#[test]
fn a_curve_point_cannot_be_made_asymmetric() {
    use curvyo_document_core::{AnchorId, AnchorKind, CurveKind, Document, NewAnchor, pack};
    for kind in [CurveKind::Spiro, CurveKind::BSpline] {
        let doc = Document::new(111);
        let controls = [(10., 10.), (30., 10.), (30., 30.), (10., 30.)]
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| {
                let mut a = NewAnchor::corner(AnchorId::new(111, i as u64), Point::new(x, y));
                a.kind = AnchorKind::Symmetric;
                a
            })
            .collect::<Vec<_>>();
        let _ = doc.create_editable_curve(&controls, false, kind).unwrap();
        let mut session = Session::open(112, &pack(&doc, "test").unwrap()).unwrap();
        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(30., 10.), false);
        session.pointer_up(Point::new(30., 10.), false, false);
        let state = session.node_toolbar_state();
        assert!(state.can_convert_to_corner, "{kind:?}: a node is selected");
        assert!(!state.can_convert_to_asymmetric, "{kind:?}");
    }
}
