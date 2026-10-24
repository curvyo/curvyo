//! The letters of a text on path (`specs/0080-text-on-path` criteria 9, 10, 12, 18, 19, 26, 28,
//! 47, 48) with the included Source Sans 3 face, through the read of a real document.

#![allow(clippy::unwrap_used, clippy::many_single_char_names, missing_docs)]

use curvyo_document_core::{
    Affine, AnchorId, Angle, CarrierRef, CharValues, Curve, Document, EllipseFrame, FontKind,
    FontRef, Length, LineOnPath, NewAnchor, NodeId, ObjectSnapshot, OnPathValues, PathSide, Point,
    Shape, TextChars, TextLayoutValues, TextSnapshot, TextWritingMode, Vec2,
};
use curvyo_text_core::{Face, FaceStore};
use curvyo_ui_core::{
    PathPlacement, default_position, lay_out, line_box_contains, measure_of, projected_values,
    release_placement, shift_along, slid_values, stored_vertical_layout, text_layout,
};

fn sans() -> std::sync::Arc<Face> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
    let mut store = FaceStore::new();
    let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn font() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn circle(d: &Document) -> NodeId {
    d.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(100.0, 100.0),
                rx: Length::from_mm(50.0),
                ry: Length::from_mm(50.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    )
}

fn line(d: &Document, to: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(9, 2), Point::new(to, 0.0)),
        ],
        false,
    )
}

fn create(d: &Document, string: &str, layout: &TextLayoutValues) -> NodeId {
    d.create_text(
        string,
        &font(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(0.0, 0.0)),
        layout,
    )
    .unwrap()
}

fn read(d: &Document, id: NodeId) -> TextSnapshot {
    match d.read_objects().get(id) {
        Some(ObjectSnapshot::Text(text)) => (**text).clone(),
        _ => panic!("a text"),
    }
}

/// Puts `id` on `carrier` with `values`; `None` position means the default of criterion 5.
fn put(
    d: &Document,
    id: NodeId,
    carrier: NodeId,
    values: OnPathValues,
    default: bool,
) -> TextSnapshot {
    let carrier = CarrierRef {
        node: carrier,
        outline: 0,
    };
    d.put_text_on_path(id, carrier, &values, true).unwrap();
    let text = read(d, id);
    if !default {
        return text;
    }
    let face = sans();
    let laid = text_layout(&text, &face).unwrap();
    let measure = measure_of(text.on_path.as_ref().unwrap().curve.as_ref().unwrap());
    let position = default_position(&laid, &measure, &values);
    d.set_text_path(
        &[(id, OnPathValues { position, ..values })],
        curvyo_document_core::OnPathLabel::Move,
    )
    .unwrap();
    read(d, id)
}

fn bounds(text: &TextSnapshot) -> (Point, Point) {
    let outline = lay_out(text, &sans(), false).unwrap();
    outline.glyphs.all_anchors().fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), a| {
            (
                Point::new(lo.x.min(a.point.x), lo.y.min(a.point.y)),
                Point::new(hi.x.max(a.point.x), hi.y.max(a.point.y)),
            )
        },
    )
}

fn placement(text: &TextSnapshot) -> PathPlacement {
    let laid = text_layout(text, &sans()).unwrap();
    PathPlacement::new(&laid, text.on_path.as_ref().unwrap()).unwrap()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

/// Criterion 9: on a straight carrier from (0, 0), Above with Baseline at Position 0 draws the
/// straight text with its anchor at (0, 0).
#[test]
fn c09_a_straight_carrier_draws_the_straight_text() {
    let d = Document::new(1);
    let t = create(&d, "Curvyo", &TextLayoutValues::DEFAULT);
    let straight = lay_out(&read(&d, t), &sans(), false).unwrap();
    let c = line(&d, 100.0);
    let on = put(&d, t, c, OnPathValues::at(Length::from_mm(0.0)), false);
    let laid = lay_out(&on, &sans(), false).unwrap();
    let a: Vec<Point> = straight.glyphs.all_anchors().map(|a| a.point).collect();
    let b: Vec<Point> = laid.glyphs.all_anchors().map(|a| a.point).collect();
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(&b) {
        assert!(p.vector_to(*q).length() < 0.01, "{p:?} {q:?}");
    }
}

fn values(side: PathSide, line: LineOnPath) -> OnPathValues {
    OnPathValues {
        side,
        line,
        ..OnPathValues::at(Length::from_mm(0.0))
    }
}

/// Criterion 10: the four cases of a round badge, upright, within 0.01 mm.
#[test]
fn c10_the_four_cases_on_a_circle() {
    let face = sans();
    let mut checked = 0;
    for (side, line_on, top_half, (y_top, y_base)) in [
        (PathSide::Above, LineOnPath::Baseline, true, (40.0, 50.0)),
        (PathSide::Below, LineOnPath::TopEdge, true, (50.0, 60.0)),
        (PathSide::Above, LineOnPath::TopEdge, false, (150.0, 160.0)),
        (PathSide::Below, LineOnPath::Baseline, false, (140.0, 150.0)),
    ] {
        let d = Document::new(1);
        let t = create(&d, "H", &TextLayoutValues::DEFAULT);
        let c = circle(&d);
        let mut text = put(&d, t, c, values(side, line_on), true);
        if !top_half {
            // The H's middle at the 90° point: a quarter of the length from 3 o'clock.
            let laid = text_layout(&text, &face).unwrap();
            let measure = measure_of(text.on_path.as_ref().unwrap().curve.as_ref().unwrap());
            let middle = f64::midpoint(laid.lines[0].extent.start.0, laid.lines[0].extent.end.0);
            let sigma = if values(side, line_on).reads_with_carrier() {
                1.0
            } else {
                -1.0
            };
            let position = measure.length().as_mm() / 4.0 - sigma * middle;
            let on_path = text.on_path.as_mut().unwrap();
            on_path.values.position = Length::from_mm(position);
        }
        let (lo, hi) = bounds(&text);
        assert!(
            near(lo.y, y_top) && near(hi.y, y_base),
            "{side:?} {line_on:?}: {lo:?} {hi:?}"
        );
        assert!(near(f64::midpoint(lo.x, hi.x), 100.0), "{lo:?} {hi:?}");
        // Upright and reading left to right: text x to screen right, text up to screen up.
        let laid = text_layout(&text, &face).unwrap();
        let frame = placement(&text)
            .frame(0, laid.lines[0].glyphs[0].centre_x.0)
            .unwrap();
        let x = frame.apply_vector(Vec2::new(1.0, 0.0));
        let up = frame.apply_vector(Vec2::new(0.0, -1.0));
        assert!(
            near(x.x, 1.0) && near(up.y, -1.0),
            "{side:?} {line_on:?}: {x:?} {up:?}"
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
}

/// Criterion 12: line k is laid out as line 0 one line height further from the letters' top.
#[test]
fn c12_lines_stack_as_in_a_straight_text() {
    let d = Document::new(1);
    let t = create(&d, "H\nH", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let text = put(&d, t, c, OnPathValues::at(Length::from_mm(0.0)), true);
    let laid = text_layout(&text, &sans()).unwrap();
    let line_height = laid.line_height.0;
    let place = placement(&text);
    let second = &laid.lines[1].glyphs[0];
    let p = place
        .point(1, second.centre_x.0, laid.lines[1].baseline.0)
        .unwrap();
    assert!(near(p.x, 100.0) && near(p.y, 50.0 + line_height), "{p:?}");
}

/// Criterion 18: Mirror keeps the reading direction and the reference point.
#[test]
fn c18_mirror_crosses_and_keeps_the_reading_direction() {
    let d = Document::new(1);
    let t = create(&d, "AB", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let text = put(&d, t, c, OnPathValues::at(Length::from_mm(0.0)), true);
    let mut mirrored = text.clone();
    let on_path = mirrored.on_path.as_mut().unwrap();
    on_path.values = on_path.values.mirrored();
    let x_of = |t: &TextSnapshot| {
        placement(t)
            .frame(0, 1.0)
            .unwrap()
            .apply_vector(Vec2::new(1.0, 0.0))
    };
    let (a, b) = (x_of(&text), x_of(&mirrored));
    assert!(near(a.x, b.x) && near(a.y, b.y));
    let (lo, _) = bounds(&mirrored);
    // Rigid letters hanging inside: their outer top corners stand a little above the circle.
    assert!(lo.y >= 49.5, "inside the circle now: {lo:?}");
}

/// Criterion 19: Mirror letters draws each letter mirrored about its own axis.
#[test]
fn c19_mirror_letters() {
    let d = Document::new(1);
    let t = create(&d, "L", &TextLayoutValues::DEFAULT);
    let c = line(&d, 100.0);
    let plain = put(&d, t, c, OnPathValues::at(Length::from_mm(0.0)), false);
    let mut mirrored = plain.clone();
    mirrored.on_path.as_mut().unwrap().values.mirror_letters = true;
    let laid = text_layout(&plain, &sans()).unwrap();
    let centre = laid.lines[0].glyphs[0].centre_x.0;
    let a = lay_out(&plain, &sans(), false).unwrap();
    let b = lay_out(&mirrored, &sans(), false).unwrap();
    for (p, q) in a.glyphs.all_anchors().zip(b.glyphs.all_anchors()) {
        assert!(near(2.0 * centre - p.point.x, q.point.x) && near(p.point.y, q.point.y));
    }
}

/// Criterion 26: past the end of an open carrier the letters continue straight; all are drawn.
#[test]
fn c26_overflow_continues_straight() {
    let d = Document::new(1);
    let t = create(&d, "HHHHHHHH", &TextLayoutValues::DEFAULT);
    let c = line(&d, 10.0);
    let text = put(&d, t, c, OnPathValues::at(Length::from_mm(0.0)), false);
    let outline = lay_out(&text, &sans(), false).unwrap();
    assert_eq!(outline.glyph_outlines.len(), 8);
    let (lo, hi) = bounds(&text);
    assert!(near(lo.y, -10.0) && near(hi.y, 0.0) && hi.x > 40.0);
}

/// Criterion 28: hit on the turned letter boxes, not on the axis-parallel box.
#[test]
fn c28_hit_on_the_letter_boxes() {
    let d = Document::new(1);
    let t = create(&d, "HELLO WORLD", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let mut text = put(&d, t, c, OnPathValues::at(Length::from_mm(0.0)), true);
    text.outline = lay_out(&text, &sans(), false).map(std::sync::Arc::new);
    let outline = text.outline.clone().unwrap();
    assert_eq!(outline.letter_boxes.len(), 11);
    assert!(line_box_contains(&text, Point::new(100.0, 45.0), 0.0));
    assert!(
        !line_box_contains(&text, Point::new(100.0, 100.0), 0.5),
        "the circle's centre lies in no letter"
    );
}

/// Criterion 47: kerns along the tangent and the normal.
#[test]
fn c47_kerning_along_the_carrier() {
    let face = sans();
    let d = Document::new(1);
    let t = create(&d, "HHH", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let text = put(&d, t, c, OnPathValues::at(Length::from_mm(0.0)), true);
    let base = |t: &TextSnapshot, glyph: usize| {
        let laid = text_layout(t, &face).unwrap();
        let g = laid.lines[0].glyphs[glyph];
        placement(t).point(0, g.centre_x.0, g.origin.y).unwrap()
    };
    let before: Vec<Point> = (0..3).map(|g| base(&text, g)).collect();
    let mut up = text.clone();
    up.chars = TextChars::new([(
        1,
        CharValues {
            kern_x: 0.0,
            kern_y: 0.2,
        },
    )]);
    let middle = base(&up, 1);
    assert!(near(middle.x, 100.0) && near(middle.y, 48.0), "{middle:?}");
    assert!(base(&up, 0).vector_to(before[0]).length() < 0.01);
    assert!(base(&up, 2).vector_to(before[2]).length() < 0.01);
    let mut along = text.clone();
    along.chars = TextChars::new([(
        2,
        CharValues {
            kern_x: 0.3,
            kern_y: 0.0,
        },
    )]);
    let measure = measure_of(text.on_path.as_ref().unwrap().curve.as_ref().unwrap());
    let moved = measure.nearest(base(&along, 2)).as_mm() - measure.nearest(before[2]).as_mm();
    assert!(near(moved, 3.0), "{moved}");
}

/// Criterion 48: a vertical text is set horizontally on a path; Release keeps its first
/// character in place in its stored vertical layout.
#[test]
fn c48_vertical_text_runs_along_the_path() {
    let face = sans();
    let vertical = TextLayoutValues {
        writing_mode: TextWritingMode::VerticalRl,
        ..TextLayoutValues::DEFAULT
    };
    let d = Document::new(1);
    let v = create(&d, "AB", &vertical);
    let h = create(&d, "AB", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let on_v = put(&d, v, c, OnPathValues::at(Length::from_mm(0.0)), true);
    let on_h = put(&d, h, c, OnPathValues::at(Length::from_mm(0.0)), true);
    let a = lay_out(&on_v, &face, false).unwrap();
    let b = lay_out(&on_h, &face, false).unwrap();
    for (p, q) in a.glyphs.all_anchors().zip(b.glyphs.all_anchors()) {
        assert!(p.point.vector_to(q.point).length() < 0.01);
    }
    assert_eq!(on_v.layout.writing_mode, TextWritingMode::VerticalRl);
    let laid = text_layout(&on_v, &face).unwrap();
    let column = stored_vertical_layout(&on_v, &face).unwrap();
    let straight = release_placement(&on_v, &laid, &placement(&on_v), Some(&column));
    let first_on_path = {
        let g = laid.lines[0].glyphs[0];
        placement(&on_v)
            .frame(0, g.centre_x.0)
            .unwrap()
            .apply_point(Point::new(g.origin.x, g.origin.y))
    };
    let g = column.lines[0].glyphs[0];
    let p = column.frame() * g.origin;
    let first_released = straight.apply_point(Point::new(p.x, p.y));
    assert!(first_on_path.vector_to(first_released).length() < 0.01);
}

/// Criterion 23: Release keeps the first letter where it was drawn.
#[test]
fn c23_release_keeps_the_first_letter() {
    let face = sans();
    let d = Document::new(1);
    let t = create(&d, "Hello", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let on = put(&d, t, c, OnPathValues::at(Length::from_mm(20.0)), false);
    let laid = text_layout(&on, &face).unwrap();
    let placement = release_placement(&on, &laid, &placement(&on), None);
    let drawn = lay_out(&on, &face, false).unwrap();
    let mut straight = on.clone();
    straight.on_path = None;
    straight.placement = placement;
    let released = lay_out(&straight, &face, false).unwrap();
    let first = drawn.glyph_outlines[0];
    let a: Vec<Point> = drawn
        .glyphs
        .subpaths()
        .take(first)
        .flat_map(|s| s.anchors.iter().map(|a| a.point))
        .collect();
    let b: Vec<Point> = released
        .glyphs
        .subpaths()
        .take(first)
        .flat_map(|s| s.anchors.iter().map(|a| a.point))
        .collect();
    for (p, q) in a.iter().zip(&b) {
        assert!(p.vector_to(*q).length() < 0.01);
    }
}

/// Criteria 21 and 31: a body drag keeps the grab offset: the reference point moves by the
/// pointer's travel along the carrier, with no jump to the grab point, across the closed
/// carrier's seam too; `shift_along` is that travel, signed and short way round.
#[test]
fn c21_a_body_drag_keeps_the_grab_offset() {
    let d = Document::new(1);
    let t = create(&d, "Ring", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let on = put(&d, t, c, OnPathValues::at(Length::from_mm(2.0)), false);
    let start = on.on_path.as_ref().unwrap().values;
    let placement = placement(&on);
    let m = placement.measure();
    let length = m.length().as_mm();
    // Grabbed 15 mm further along than the reference point, dragged 1 mm on.
    let press = m.at(Length::from_mm(17.0)).point;
    let grab = shift_along(m, start.position, m.nearest(press));
    assert!(near(grab.as_mm(), 15.0), "{grab:?}");
    let to = m.at(Length::from_mm(18.0)).point;
    let v = slid_values(m, start, grab, to, 2.0, None);
    assert!(near(v.position.as_mm(), 3.0), "{:?}", v.position);
    assert_eq!(v.side, start.side);
    // Dragged 5 mm back past the seam: Position wraps to the end, the travel reads -5 mm.
    let back = m.at(Length::from_mm(12.0)).point;
    let v = slid_values(m, start, grab, back, 2.0, None);
    assert!(near(v.position.as_mm(), length - 3.0), "{:?}", v.position);
    assert!(near(
        shift_along(m, start.position, v.position).as_mm(),
        -5.0
    ));
}

/// Criterion 31: a Ctrl-copy move shifts Position by the drag projected on the carrier's
/// tangent at the grab, not by the nearest point (which lags off the carrier).
#[test]
fn c31_the_copy_shift_is_the_drag_projected_on_the_carrier() {
    let d = Document::new(1);
    let t = create(&d, "Ring", &TextLayoutValues::DEFAULT);
    let c = circle(&d);
    let on = put(&d, t, c, OnPathValues::at(Length::from_mm(2.0)), false);
    let start = on.on_path.as_ref().unwrap().values;
    let placement = placement(&on);
    let m = placement.measure();
    let at = m.at(Length::from_mm(10.0));
    // 4 mm off the carrier, dragged 20 mm along its tangent there and 3 mm across.
    let normal = Vec2::new(-at.tangent.y, at.tangent.x);
    let press = Point::new(at.point.x + normal.x * 4.0, at.point.y + normal.y * 4.0);
    let to = Point::new(
        press.x + at.tangent.x * 20.0 + normal.x * 3.0,
        press.y + at.tangent.y * 20.0 + normal.y * 3.0,
    );
    let v = projected_values(m, start, press, to);
    assert!(near(v.position.as_mm(), 22.0), "{:?}", v.position);
    assert_eq!((v.side, v.line), (start.side, start.line));
}
