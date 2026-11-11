//! Curved variable-width artwork shares one contour across editor output paths.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, Vec2, pack, unpack};
use curvyo_editor_wasm::{Session, StrokeToPathOutcome, SvgExportResult};
use curvyo_ui_core::SvgExportOptions;
mod cpu_time;

fn draw_once(session: &mut Session) {
    let _ = session.draw_list();
}

fn open_curve(doc: &Document) -> curvyo_document_core::NodeId {
    let mut a = NewAnchor::corner(AnchorId::new(19, 1), Point::new(20.0, 20.0));
    let mut b = NewAnchor::corner(AnchorId::new(19, 2), Point::new(30.0, 25.0));
    a.handle_out = Vec2::new(4.0, 0.0);
    b.handle_in = Vec2::new(-4.0, 0.0);
    let id = doc.create_path(&[a, b], false);
    doc.set_width_point(id, AnchorId::new(19, 3), 0.0, 0.5)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(19, 4), 1.0, 1.5)
        .unwrap();
    id
}

fn closed_curve(doc: &Document) -> curvyo_document_core::NodeId {
    let k = 5.522_847_498_307_934;
    let data = [
        (
            Point::new(60.0, 40.0),
            Vec2::new(0.0, -k),
            Vec2::new(0.0, k),
        ),
        (
            Point::new(50.0, 50.0),
            Vec2::new(k, 0.0),
            Vec2::new(-k, 0.0),
        ),
        (
            Point::new(40.0, 40.0),
            Vec2::new(0.0, k),
            Vec2::new(0.0, -k),
        ),
        (
            Point::new(50.0, 30.0),
            Vec2::new(-k, 0.0),
            Vec2::new(k, 0.0),
        ),
    ];
    let anchors: Vec<_> = data
        .into_iter()
        .enumerate()
        .map(|(i, (point, incoming, outgoing))| {
            let mut a = NewAnchor::corner(AnchorId::new(20, i as u64 + 1), point);
            a.handle_in = incoming;
            a.handle_out = outgoing;
            a
        })
        .collect();
    let id = doc.create_path(&anchors, true);
    doc.edit_style(
        &[id],
        &curvyo_document_core::StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(2.0)),
    )
    .unwrap();
    for (i, (position, width)) in [(0.0, 1.0), (0.25, 1.4), (0.5, 1.0), (0.75, 0.8)]
        .into_iter()
        .enumerate()
    {
        doc.set_width_point(id, AnchorId::new(20, i as u64 + 5), position, width)
            .unwrap();
    }
    doc.set_width_point(id, AnchorId::new(20, 10), 0.99, 1.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(20, 11), 0.01, 1.0)
        .unwrap();
    id
}

#[test]
fn dashed_curved_profile_worker_svg_conversion_and_undo() {
    use curvyo_document_core::{DashPattern, LineCap, StyleEdit};
    for pattern in [vec![3.0, 2.0], vec![0.0, 4.0]] {
        let doc = Document::new(90);
        let id = open_curve(&doc);
        doc.edit_style(&[id], &StyleEdit::StrokeCap(LineCap::Round))
            .unwrap();
        doc.edit_style(
            &[id],
            &StyleEdit::StrokeDash(DashPattern::new(pattern.clone()).unwrap()),
        )
        .unwrap();
        let before = doc.path(id).unwrap();
        let mut session = Session::open(91, &pack(&doc, "test").unwrap()).unwrap();
        draw_once(&mut session);
        assert!(
            session.take_unsupported_width_notice().is_empty(),
            "{pattern:?}"
        );
        let draw = session.draw_list();
        assert!(!draw.triangles.is_empty(), "{pattern:?}");
        let SvgExportResult::Written { text, .. } = session.export_svg(SvgExportOptions::default())
        else {
            panic!("export {pattern:?}")
        };
        assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
        session.pointer_down(Point::new(20.0, 20.0), false);
        session.pointer_up(Point::new(20.0, 20.0), false, false);
        assert!(
            matches!(
                session.apply_stroke_to_path(),
                StrokeToPathOutcome::Applied { .. }
            ),
            "{pattern:?}"
        );
        let converted = unpack(92, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap();
        assert!(
            converted.closed && !converted.extra_subpaths.is_empty(),
            "{pattern:?}"
        );
        session.undo();
        assert_eq!(
            unpack(93, &session.pack("test").unwrap())
                .unwrap()
                .path(id)
                .unwrap(),
            before
        );
    }
}

#[test]
fn dashed_closed_curve_worker_svg_conversion_and_undo() {
    use curvyo_document_core::{DashPattern, StyleEdit};
    let doc = Document::new(94);
    let id = closed_curve(&doc);
    doc.edit_style(
        &[id],
        &StyleEdit::StrokeDash(DashPattern::new(vec![8.0, 2.0]).unwrap()),
    )
    .unwrap();
    let before = doc.path(id).unwrap();
    let mut session = Session::open(95, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    assert!(session.take_unsupported_width_notice().is_empty());
    assert!(!session.draw_list().triangles.is_empty());
    let SvgExportResult::Written { text, .. } = session.export_svg(SvgExportOptions::default())
    else {
        panic!("closed dashed export")
    };
    assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
    session.pointer_down(Point::new(60.0, 40.0), false);
    session.pointer_up(Point::new(60.0, 40.0), false, false);
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { .. }
    ));
    let converted = unpack(96, &session.pack("test").unwrap())
        .unwrap()
        .path(id)
        .unwrap();
    assert!(converted.closed && !converted.extra_subpaths.is_empty());
    session.undo();
    assert_eq!(
        unpack(97, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        before
    );
}

#[test]
fn a_curve_with_a_profile_exports_at_once_without_changing_the_path() {
    let doc = Document::new(19);
    let id = open_curve(&doc);
    let before = doc.path(id).unwrap();
    let session = Session::open(21, &pack(&doc, "test").unwrap()).unwrap();
    assert!(matches!(
        session.export_svg(SvgExportOptions::default()),
        SvgExportResult::Written { objects: 1, .. }
    ));
    assert!(session.take_unsupported_width_notice().is_empty());
    assert_eq!(
        unpack(22, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        before
    );
}

#[test]
fn closed_cornered_profile_exports_and_conversion_undo_restores_editable_path() {
    let doc = Document::new(40);
    let nodes: Vec<_> = [(10.0, 10.0), (50.0, 10.0), (50.0, 50.0), (10.0, 50.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| NewAnchor::corner(AnchorId::new(40, i as u64 + 1), Point::new(x, y)))
        .collect();
    let id = doc.create_path(&nodes, true);
    doc.set_width_point(id, AnchorId::new(40, 5), 0.0, 1.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(40, 6), 0.5, 2.0)
        .unwrap();
    let before = doc.path(id).unwrap();
    let mut session = Session::open(41, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    let SvgExportResult::Written { text, .. } = session.export_svg(SvgExportOptions::default())
    else {
        panic!("closed profiled rectangle must export")
    };
    assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { .. }
    ));
    let result = unpack(42, &session.pack("test").unwrap()).unwrap();
    let converted = result.path(id).unwrap();
    assert!(converted.closed);
    assert!(
        !converted.extra_subpaths.is_empty(),
        "the inner boundary remains a hole"
    );
    session.undo();
    assert_eq!(
        unpack(43, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        before
    );
}

#[test]
fn zero_width_on_a_cornered_line_exports_two_closed_regions_and_undoes() {
    let doc = Document::new(50);
    let nodes: Vec<_> = [(10.0, 10.0), (50.0, 10.0), (50.0, 50.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| NewAnchor::corner(AnchorId::new(50, i as u64 + 1), Point::new(x, y)))
        .collect();
    let id = doc.create_path(&nodes, false);
    for (i, (position, multiplier)) in [(0.0, 1.0), (0.5, 0.0), (1.0, 1.0)].into_iter().enumerate()
    {
        doc.set_width_point(id, AnchorId::new(50, i as u64 + 4), position, multiplier)
            .unwrap();
    }
    let before = doc.path(id).unwrap();
    let mut session = Session::open(51, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    let SvgExportResult::Written { text, .. } = session.export_svg(SvgExportOptions::default())
    else {
        panic!("zero-width corner must export")
    };
    assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { .. }
    ));
    let converted = unpack(52, &session.pack("test").unwrap())
        .unwrap()
        .path(id)
        .unwrap();
    assert!(converted.closed);
    assert_eq!(converted.extra_subpaths.len(), 1);
    assert!(converted.extra_subpaths[0].closed);
    session.undo();
    assert_eq!(
        unpack(53, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        before
    );
}

#[test]
fn curved_corner_with_zero_width_exports_and_undoes_for_each_join() {
    use curvyo_document_core::{LineJoin, StyleEdit};
    for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
        let doc = Document::new(60);
        let mut a = NewAnchor::corner(AnchorId::new(60, 1), Point::new(10.0, 10.0));
        let mut b = NewAnchor::corner(AnchorId::new(60, 2), Point::new(20.0, 15.0));
        let mut c = NewAnchor::corner(AnchorId::new(60, 3), Point::new(25.0, 25.0));
        a.handle_out = Vec2::new(4.0, 0.0);
        b.handle_in = Vec2::new(-4.0, 0.0);
        b.handle_out = Vec2::new(0.0, 4.0);
        c.handle_in = Vec2::new(0.0, -4.0);
        let id = doc.create_path(&[a, b, c], false);
        doc.edit_style(&[id], &StyleEdit::StrokeJoin(join)).unwrap();
        for (i, (position, multiplier)) in
            [(0.0, 1.0), (0.5, 0.0), (1.0, 1.0)].into_iter().enumerate()
        {
            doc.set_width_point(id, AnchorId::new(60, i as u64 + 4), position, multiplier)
                .unwrap();
        }
        let before = doc.path(id).unwrap();
        let mut session = Session::open(61, &pack(&doc, "test").unwrap()).unwrap();
        draw_once(&mut session);
        let SvgExportResult::Written { text, .. } = session.export_svg(SvgExportOptions::default())
        else {
            panic!("curved zero-width corner export: {join:?}")
        };
        assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
        session.pointer_down(Point::new(10.0, 10.0), false);
        session.pointer_up(Point::new(10.0, 10.0), false, false);
        assert!(
            matches!(
                session.apply_stroke_to_path(),
                StrokeToPathOutcome::Applied { .. }
            ),
            "{join:?}"
        );
        let converted = unpack(62, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap();
        assert!(converted.closed);
        assert_eq!(converted.extra_subpaths.len(), 1, "{join:?}");
        assert!(converted.extra_subpaths[0].closed);
        session.undo();
        assert_eq!(
            unpack(63, &session.pack("test").unwrap())
                .unwrap()
                .path(id)
                .unwrap(),
            before
        );
    }
}

#[test]
fn closed_curved_corners_export_and_undo_for_each_join() {
    use curvyo_document_core::{Length, LineJoin, StyleEdit};
    for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
        let doc = Document::new(70);
        let nodes: Vec<_> = [
            ((10.0, 10.0), (-2.0, 6.0), (6.0, -2.0)),
            ((30.0, 10.0), (-6.0, -2.0), (2.0, 6.0)),
            ((30.0, 30.0), (2.0, -6.0), (-6.0, 2.0)),
            ((10.0, 30.0), (6.0, 2.0), (-2.0, -6.0)),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (p, incoming, outgoing))| {
            let mut node = NewAnchor::corner(AnchorId::new(70, i as u64 + 1), Point::new(p.0, p.1));
            node.handle_in = Vec2::new(incoming.0, incoming.1);
            node.handle_out = Vec2::new(outgoing.0, outgoing.1);
            node
        })
        .collect();
        let id = doc.create_path(&nodes, true);
        doc.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
            .unwrap();
        doc.edit_style(&[id], &StyleEdit::StrokeJoin(join)).unwrap();
        doc.set_width_point(id, AnchorId::new(70, 5), 0.0, 1.0)
            .unwrap();
        doc.set_width_point(id, AnchorId::new(70, 6), 0.5, 1.5)
            .unwrap();
        let before = doc.path(id).unwrap();
        let mut session = Session::open(71, &pack(&doc, "test").unwrap()).unwrap();
        draw_once(&mut session);
        let SvgExportResult::Written { text, .. } = session.export_svg(SvgExportOptions::default())
        else {
            panic!("closed curved corners export: {join:?}")
        };
        assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
        session.pointer_down(Point::new(10.0, 10.0), false);
        session.pointer_up(Point::new(10.0, 10.0), false, false);
        assert!(
            matches!(
                session.apply_stroke_to_path(),
                StrokeToPathOutcome::Applied { .. }
            ),
            "{join:?}"
        );
        let converted = unpack(72, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap();
        assert!(
            converted.closed && converted.extra_subpaths.iter().any(|p| p.closed),
            "{join:?}"
        );
        session.undo();
        assert_eq!(
            unpack(73, &session.pack("test").unwrap())
                .unwrap()
                .path(id)
                .unwrap(),
            before
        );
    }
}

#[test]
fn a_rotated_closed_curve_with_a_seam_profile_is_drawn_and_exported() {
    let doc = Document::new(20);
    let id = closed_curve(&doc);
    let path = doc.path(id).unwrap();
    let mapped = path.mapped(curvyo_document_core::Affine::new(
        0.8, 0.6, -0.6, 0.8, 0.0, 0.0,
    ));
    doc.transform_objects(&[curvyo_document_core::ObjectSnapshot::Path(mapped)], false)
        .unwrap();
    let before = doc.path(id).unwrap();
    let other = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(30, 1), Point::new(90.0, 10.0)),
            NewAnchor::corner(AnchorId::new(30, 2), Point::new(110.0, 10.0)),
        ],
        false,
    );
    let mut session = Session::open(25, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    assert_eq!(session.take_unsupported_width_notice(), "");
    assert!(matches!(
        session.export_svg(curvyo_ui_core::SvgExportOptions::default()),
        SvgExportResult::Written { objects: 2, .. }
    ));
    session.pointer_down(Point::new(100.0, 10.0), false);
    session.pointer_up(Point::new(100.0, 10.0), false, false);
    let options = SvgExportOptions {
        selection_only: true,
        ..Default::default()
    };
    assert!(matches!(
        session.export_svg(options),
        SvgExportResult::Written { objects: 1, .. }
    ));
    assert_eq!(
        unpack(26, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        before
    );
    assert!(doc.path(other).is_some());
}

#[test]
fn open_curve_artwork_can_be_hit_exported_and_converted_with_undo() {
    let doc = Document::new(19);
    let id = open_curve(&doc);
    let before = doc.path(id).unwrap();
    let mut session = Session::open(21, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    assert_eq!(session.take_unsupported_width_notice(), "");
    assert!(matches!(
        session.export_svg(SvgExportOptions::default()),
        SvgExportResult::Written { objects: 1, .. }
    ));
    session.pointer_down(Point::new(25.0, 22.5), false);
    session.pointer_up(Point::new(25.0, 22.5), false, false);
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { .. }
    ));
    session.undo();
    let restored = unpack(22, &session.pack("test").unwrap()).unwrap();
    assert_eq!(restored.path(id).unwrap(), before);
}

#[test]
fn closed_curve_seam_and_clone_export_are_preserved_through_conversion() {
    let doc = Document::new(20);
    let id = closed_curve(&doc);
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    let mut session = Session::open(23, &pack(&doc, "test").unwrap()).unwrap();
    let (_, first_draw_cpu) = cpu_time::cpu_time_of(|| session.draw_list());
    draw_once(&mut session);
    let (_, second_draw_cpu) = cpu_time::cpu_time_of(|| session.draw_list());
    eprintln!(
        "closed contour drawing thread CPU: first {first_draw_cpu:?}, repeated {second_draw_cpu:?}"
    );
    assert_eq!(session.take_unsupported_width_notice(), "");
    let SvgExportResult::Written { text, objects, .. } =
        session.export_svg(SvgExportOptions::default())
    else {
        panic!("closed export")
    };
    assert_eq!(objects, 2);
    assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
    session.pointer_down(Point::new(60.0, 40.0), false);
    session.pointer_up(Point::new(60.0, 40.0), false, false);
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { .. }
    ));
    session.undo();
    let restored = unpack(24, &session.pack("test").unwrap()).unwrap();
    assert_eq!(restored.path(id).unwrap(), doc.path(id).unwrap());
    assert!(restored.object(clone).is_some());
}

#[test]
fn variable_curved_dash_with_marker_exports_converts_and_undoes() {
    use curvyo_document_core::{DashPattern, MarkerShape, StyleEdit};
    let doc = Document::new(101);
    let id = open_curve(&doc);
    doc.edit_style(
        &[id],
        &StyleEdit::StrokeDash(DashPattern::new(vec![3.0, 2.0]).unwrap()),
    )
    .unwrap();
    doc.edit_style(&[id], &StyleEdit::MarkerEnd(MarkerShape::Arrow))
        .unwrap();
    let before = doc.path(id).unwrap();
    let mut session = Session::open(102, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    assert!(matches!(
        session.export_svg(SvgExportOptions::default()),
        SvgExportResult::Written { .. }
    ));
    session.pointer_down(Point::new(20.0, 20.0), false);
    session.pointer_up(Point::new(20.0, 20.0), false, false);
    assert!(matches!(
        session.apply_stroke_to_path(),
        StrokeToPathOutcome::Applied { .. }
    ));
    session.undo();
    assert_eq!(
        unpack(103, &session.pack("test").unwrap())
            .unwrap()
            .path(id)
            .unwrap(),
        before
    );
}

/// A group of the open profiled curve and a plain line, hidden, and a linked clone of it in
/// place: the clone draws what the group would.
fn hidden_group_and_its_clone(
    doc: &Document,
) -> (curvyo_document_core::NodeId, curvyo_document_core::NodeId) {
    let id = open_curve(doc);
    let plain = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(19, 8), Point::new(20.0, 40.0)),
            NewAnchor::corner(AnchorId::new(19, 9), Point::new(30.0, 40.0)),
        ],
        false,
    );
    let group = doc.group_objects(&[id, plain]).unwrap();
    let clone = doc
        .place_linked_clones(&[group], Vec2::new(0.0, 0.0))
        .unwrap()[0];
    doc.set_node_visible(&[group], false).unwrap();
    (group, clone)
}

#[test]
fn a_clone_of_a_group_draws_its_profiled_member_by_the_width_outline() {
    let grouped = Document::new(19);
    let _ = hidden_group_and_its_clone(&grouped);
    let cloned = Session::open(31, &pack(&grouped, "test").unwrap()).unwrap();
    let alone = Document::new(19);
    let _ = open_curve(&alone);
    let _ = alone.create_path(
        &[
            NewAnchor::corner(AnchorId::new(19, 8), Point::new(20.0, 40.0)),
            NewAnchor::corner(AnchorId::new(19, 9), Point::new(30.0, 40.0)),
        ],
        false,
    );
    let direct = Session::open(32, &pack(&alone, "test").unwrap()).unwrap();
    let a = cloned.draw_list();
    let b = direct.draw_list();
    assert_eq!(cloned.take_unsupported_width_notice(), "");
    assert_eq!(
        a.triangle_count(),
        b.triangle_count(),
        "the clone draws the outline the member draws"
    );
    for (p, q) in a.triangles.iter().zip(&b.triangles) {
        assert!(
            (p.position.x - q.position.x).abs() < 1e-3
                && (p.position.y - q.position.y).abs() < 1e-3,
            "same place: {p:?} vs {q:?}"
        );
    }
}

#[test]
fn a_clone_of_a_group_is_hit_and_exported_by_the_width_outline() {
    let doc = Document::new(19);
    let (_, clone) = hidden_group_and_its_clone(&doc);
    let mut session = Session::open(33, &pack(&doc, "test").unwrap()).unwrap();
    draw_once(&mut session);
    // 0.7 mm above the curve's end at (30, 25): inside the 1.5 x widened end, outside a plain
    // stroke of half a width.
    let at = Point::new(29.9, 24.3);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    assert_eq!(
        session.selected_object_count(),
        1,
        "the widened end picks the clone"
    );
    let SvgExportResult::Written { text, objects, .. } =
        session.export_svg(SvgExportOptions::default())
    else {
        panic!("the clone is written")
    };
    assert!(objects >= 1, "the clone is written ({objects})");
    let _ = clone;
    // The outline is written as a closed filled shape, not as a stroked centre line: once for
    // the hidden member, once for the clone.
    assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
    assert_eq!(
        text.matches(r#"Z" sodipodi"#).count(),
        2,
        "the member's outline and the clone's: {text}"
    );
}
