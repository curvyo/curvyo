//! Independent tester cases for `specs/0080-text-on-path/specification.md` slice 1 on the pure
//! layout and edit rules in `curvyo-ui-core`: default placement (criterion 5), rigid letters on a
//! straight carrier (9), the four circle cases (10), corners (11), several lines (12), single-line
//! faces (13), the substitute face (14), Ctrl snap stops and the drag across (15, 16), Mirror and
//! Mirror letters (18, 19), rotation on an ellipse (20), the nudge (21), Release (23), following a
//! carrier edit (24), overflow (26), Position read modulo (35), the per-character kerns (47),
//! vertical texts (48) and the budgets (43, 45).
//!
//! Written from the specification, ADR 0022 and the public names before the implementation was
//! read.

// Short names and local `use` of enum variants keep the geometry cases readable; the f64 -> i64
// casts round micrometre keys of finite test points.
#![allow(
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::cast_possible_truncation,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::f64::consts::PI;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, CarrierRef, CharValues, Document, EllipseFrame, FontKind,
    FontRef, InnerRatio, Length, LineOnPath, NewAnchor, NodeId, ObjectSnapshot, OnPathValues,
    PathSide, PathSnapshot, Point, PointCount, RectBounds, StarFrame, TextChars, TextLayoutValues,
    TextOutline, TextSnapshot, TextWritingMode, Vec2, carrier_curve, text_path_previews,
};
use curvyo_text_core::{Face, FaceStore};
use curvyo_ui_core::{
    PathPlacement, default_position, dragged_values, lay_out, measure_of, nudged_values,
    operation_name, position_at_degrees, release_placement, rotated_position, snap_stops,
    stored_vertical_layout, text_layout,
};

const TOL: f64 = 0.01;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn face_from(file: &str) -> std::sync::Arc<Face> {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(file),
    )
    .unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn sans() -> std::sync::Arc<Face> {
    face_from("source-sans-3/SourceSans3-Regular.otf")
}

fn font(kind: FontKind) -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind,
    }
}

struct Fx {
    d: Document,
    carrier: NodeId,
}

impl Fx {
    fn circle() -> Self {
        let d = Document::new(1);
        let carrier = d.create_ellipse(EllipseFrame {
            center: pt(100.0, 100.0),
            rx: Length::from_mm(50.0),
            ry: Length::from_mm(50.0),
        });
        Self { d, carrier }
    }

    fn line(from: Point, to: Point) -> Self {
        let d = Document::new(1);
        let carrier = d.create_path(
            &[
                NewAnchor::corner(AnchorId::new(9, 1), from),
                NewAnchor::corner(AnchorId::new(9, 2), to),
            ],
            false,
        );
        Self { d, carrier }
    }

    fn with(d: Document, carrier: NodeId) -> Self {
        Self { d, carrier }
    }

    /// A text put on the carrier with `values`, read back resolved.
    fn text(
        &self,
        s: &str,
        values: &OnPathValues,
        layout: TextLayoutValues,
        chars: &TextChars,
        kind: FontKind,
    ) -> TextSnapshot {
        let t = self
            .d
            .create_text_with_chars(
                s,
                &font(kind),
                Length::from_mm(10.0),
                Affine::translation(Vec2::new(3.0, 4.0)),
                &layout,
                chars,
            )
            .unwrap();
        self.d
            .put_text_on_path(
                t,
                CarrierRef {
                    node: self.carrier,
                    outline: 0,
                },
                values,
                true,
            )
            .unwrap();
        self.d.text(t).unwrap()
    }

    fn plain(&self, s: &str, values: &OnPathValues) -> TextSnapshot {
        self.text(
            s,
            values,
            TextLayoutValues::DEFAULT,
            &TextChars::default(),
            FontKind::Outline,
        )
    }
}

fn with_values(t: &TextSnapshot, values: OnPathValues) -> TextSnapshot {
    let mut t = t.clone();
    t.on_path.as_mut().unwrap().values = values;
    t
}

fn straight(t: &TextSnapshot, placement: Affine) -> TextSnapshot {
    let mut t = t.clone();
    t.on_path = None;
    t.placement = placement;
    t
}

fn outlines(path: &PathSnapshot) -> Vec<Vec<Point>> {
    let one = |anchors: &[NewAnchor]| {
        anchors
            .iter()
            .flat_map(|a| {
                [
                    a.point,
                    a.point.translated(a.handle_in),
                    a.point.translated(a.handle_out),
                ]
            })
            .collect::<Vec<Point>>()
    };
    let mut all = vec![one(&path.anchors)];
    all.extend(path.extra_subpaths.iter().map(|s| one(&s.anchors)));
    all.retain(|o| !o.is_empty());
    all
}

/// The points of each drawn glyph.
fn glyphs(o: &TextOutline) -> Vec<Vec<Point>> {
    let all = outlines(&o.glyphs);
    let mut out = Vec::new();
    let mut at = 0;
    for &n in &o.glyph_outlines {
        out.push(all[at..at + n].concat());
        at += n;
    }
    out
}

fn points(o: &TextOutline) -> Vec<Point> {
    outlines(&o.glyphs).concat()
}

fn assert_same(a: &[Point], b: &[Point], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: point count");
    let worst = a
        .iter()
        .zip(b)
        .map(|(p, q)| (p.x - q.x).hypot(p.y - q.y))
        .fold(0.0, f64::max);
    assert!(worst < TOL, "{what}: worst {worst} mm");
}

fn lay(t: &TextSnapshot, face: &Face) -> TextOutline {
    lay_out(t, face, false).unwrap()
}

fn centre_x(t: &TextSnapshot, face: &Face) -> f64 {
    text_layout(&straight(t, Affine::IDENTITY), face)
        .unwrap()
        .lines[0]
        .glyphs[0]
        .centre_x
        .0
}

fn measure(t: &TextSnapshot) -> curvyo_geometry_core::PathMeasure {
    measure_of(t.on_path.as_ref().unwrap().curve.as_ref().unwrap())
}

fn vals(side: PathSide, line: LineOnPath, position: f64) -> OnPathValues {
    OnPathValues {
        side,
        line,
        ..OnPathValues::at(Length::from_mm(position))
    }
}

fn circumference(t: &TextSnapshot) -> f64 {
    measure(t).length().as_mm()
}

/// Criterion 5 (Position is the text's middle, customer 2026-10-24): on an open carrier the
/// default puts the text's start at the carrier's start, its middle half its width along; on a
/// closed one the text's middle at the topmost point (12 o'clock on a circle).
#[test]
fn c5_default_position_open_start_and_circle_top() {
    let face = sans();
    let line = Fx::line(pt(0.0, 0.0), pt(100.0, 0.0));
    let t = line.plain("Name", &OnPathValues::at(Length::from_mm(0.0)));
    let laid = text_layout(&t, &face).unwrap();
    assert!(
        (default_position(&laid, &measure(&t)).as_mm() - laid.lines[0].width.0 / 2.0).abs() < 1e-9
    );

    let c = Fx::circle();
    let t = c.plain("Name", &OnPathValues::at(Length::from_mm(0.0)));
    let laid = text_layout(&t, &face).unwrap();
    let v = t.on_path.as_ref().unwrap().values;
    let s = default_position(&laid, &measure(&t));
    let placed = lay(&with_values(&t, OnPathValues { position: s, ..v }), &face);
    // The line box's middle at the carrier sits at the top: its corners are symmetric about x=100.
    let xs: Vec<f64> = points(&placed).iter().map(|p| p.x).collect();
    let (lo, hi) = (
        xs.iter().copied().fold(f64::MAX, f64::min),
        xs.iter().copied().fold(f64::MIN, f64::max),
    );
    assert!(
        (f64::midpoint(lo, hi) - 100.0).abs() < 1.0,
        "middle near x = 100: {lo}..{hi}"
    );
    assert!(
        (s.as_mm() - 0.75 * circumference(&t)).abs() < 0.05,
        "middle of the line at 3/4 of the clockwise circle"
    );
}

/// The text `t` on its open carrier at the default Position: its start at the carrier's start.
fn at_start(t: &TextSnapshot, face: &Face) -> TextSnapshot {
    let laid = text_layout(t, face).unwrap();
    let position = default_position(&laid, &measure(t));
    let v = t.on_path.as_ref().unwrap().values;
    with_values(t, OnPathValues { position, ..v })
}

/// Criterion 9: a straight carrier from (0, 0) to (100, 0), Above, Baseline, at the default
/// Position (half the text's width, customer 2026-10-24) draws the outlines of the straight text
/// anchored at (0, 0).
#[test]
fn c9_straight_carrier_equals_straight_text() {
    let face = sans();
    let fx = Fx::line(pt(0.0, 0.0), pt(100.0, 0.0));
    let t = fx.plain("Hallo Welt", &OnPathValues::at(Length::from_mm(0.0)));
    let on = lay(&at_start(&t, &face), &face);
    let flat = lay(&straight(&t, Affine::IDENTITY), &face);
    assert_same(&points(&on), &points(&flat), "straight carrier");
}

/// Criterion 10: the four cases of a round badge with an "H" of Size 10 mm on the circle of
/// radius 50 around (100, 100).
#[test]
fn c10_four_cases_on_the_circle() {
    let face = sans();
    let fx = Fx::circle();
    let t = fx.plain("H", &OnPathValues::at(Length::from_mm(0.0)));
    let cx = centre_x(&t, &face);
    let c = circumference(&t);
    let laid = text_layout(&t, &face).unwrap();
    // H upright with its advance middle at x = 100 and its baseline at `y`.
    let upright = |y: f64| {
        points(&lay(
            &straight(&t, Affine::translation(Vec2::new(100.0 - cx, y))),
            &face,
        ))
    };
    use LineOnPath::{Baseline, TopEdge};
    use PathSide::{Above, Below};

    // Top: default position (criterion 5) for each case.
    for (side, line, baseline, what) in [
        (
            Above,
            Baseline,
            50.0,
            "Above + Baseline: outside, baseline at 50",
        ),
        (
            Below,
            TopEdge,
            60.0,
            "Below + Top edge: inside, top at 50, baseline at 60",
        ),
    ] {
        let v = vals(side, line, 0.0);
        let s = default_position(&laid, &measure(&t));
        let got = lay(&with_values(&t, OnPathValues { position: s, ..v }), &face);
        assert_same(&points(&got), &upright(baseline), what);
    }
    // Bottom: the H's middle (Position) at the 90 degree point, arc length C / 4.
    for (side, line, baseline, what) in [
        (
            Above,
            TopEdge,
            160.0,
            "Above + Top edge: outside, top at 150, baseline at 160",
        ),
        (
            Below,
            Baseline,
            150.0,
            "Below + Baseline: inside, baseline at 150",
        ),
    ] {
        let v = vals(side, line, c / 4.0);
        let got = lay(&with_values(&t, v), &face);
        assert_same(&points(&got), &upright(baseline), what);
    }
}

/// Criterion 11: a letter whose middle falls on a corner takes the incoming side's tangent; on a
/// star and a polygon every letter is drawn, nothing is refused.
#[test]
fn c11_corner_takes_the_incoming_side_and_shapes_with_corners_carry() {
    let face = sans();
    let d = Document::new(1);
    let rect = d.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(100.0),
        height: Length::from_mm(50.0),
    });
    let fx = Fx::with(d, rect);
    let t = fx.plain("I", &OnPathValues::at(Length::from_mm(0.0)));
    let m = measure(&t);
    let at_corner = m.at(Length::from_mm(100.0));
    assert!((at_corner.point.x - 100.0).abs() < TOL && at_corner.point.y.abs() < TOL);
    assert!(
        (at_corner.tangent.x - 1.0).abs() < 1e-6,
        "incoming top edge: {:?}",
        at_corner.tangent
    );
    let past = m.at(Length::from_mm(100.5));
    assert!(
        (past.tangent.y - 1.0).abs() < 1e-6,
        "right edge after the corner"
    );
    // The I's middle on the top-right corner: drawn as on the top edge, unturned.
    let cx = centre_x(&t, &face);
    let got = lay(
        &with_values(&t, OnPathValues::at(Length::from_mm(100.0))),
        &face,
    );
    let want = lay(
        &straight(&t, Affine::translation(Vec2::new(100.0 - cx, 0.0))),
        &face,
    );
    assert_same(&points(&got), &points(&want), "letter on the corner");

    for kind in ["star", "polygon"] {
        let d = Document::new(1);
        let frame = StarFrame {
            center: pt(50.0, 50.0),
            radius: Length::from_mm(30.0),
            angle: curvyo_document_core::Angle::from_radians(0.0),
        };
        let id = if kind == "star" {
            d.create_star(
                frame,
                PointCount::new(5).unwrap(),
                InnerRatio::new(0.4).unwrap(),
            )
        } else {
            d.create_polygon(frame, PointCount::new(6).unwrap())
        };
        let fx = Fx::with(d, id);
        let t = fx.plain("STARSTARSTAR", &OnPathValues::at(Length::from_mm(0.0)));
        assert!(
            t.on_path.as_ref().unwrap().curve.is_some(),
            "{kind} carries"
        );
        let o = lay(&t, &face);
        assert_eq!(o.glyph_outlines.len(), 12, "{kind}: every letter drawn");
        assert!(
            points(&o)
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite())
        );
    }
}

/// Criterion 12: line k lies k line heights further from the carrier in the direction from top
/// edge to baseline: "H" / "H" on the circle, Above + Baseline at the top, the second H's baseline
/// middle at (100, 50 + line height).
#[test]
fn c12_second_line_stacks_one_line_height_further() {
    let face = sans();
    let fx = Fx::circle();
    let t = fx.plain("H\nH", &OnPathValues::at(Length::from_mm(0.0)));
    let laid = text_layout(&t, &face).unwrap();
    let lh = laid.lines[1].baseline.0 - laid.lines[0].baseline.0;
    assert!(lh > 10.0);
    let v = t.on_path.as_ref().unwrap().values;
    let s = default_position(&laid, &measure(&t));
    let got = glyphs(&lay(
        &with_values(&t, OnPathValues { position: s, ..v }),
        &face,
    ));
    assert_eq!(got.len(), 2);
    let one = TextSnapshot {
        string: "H".to_owned(),
        ..t.clone()
    };
    let cx = centre_x(&one, &face);
    for (k, glyph) in got.iter().enumerate() {
        let want = points(&lay(
            &straight(
                &one,
                Affine::translation(Vec2::new(100.0 - cx, 50.0 + k as f64 * lh)),
            ),
            &face,
        ));
        assert_same(glyph, &want, &format!("line {k}"));
    }
}

/// Criterion 13: a single-line face on path is drawn as centre lines with the text's stroke,
/// never filled, exactly as the straight single-line text.
#[test]
fn c13_single_line_face_on_path_draws_like_straight() {
    let face = face_from("hershey-sans-1/HersheySans1.svg");
    let fx = Fx::line(pt(0.0, 0.0), pt(200.0, 0.0));
    let t = fx.text(
        "Hello",
        &OnPathValues::at(Length::from_mm(0.0)),
        TextLayoutValues::DEFAULT,
        &TextChars::default(),
        FontKind::SingleLine,
    );
    let on = lay(&at_start(&t, &face), &face);
    let flat = lay(&straight(&t, Affine::IDENTITY), &face);
    assert_eq!(on.glyphs.style, flat.glyphs.style, "same stroke, no fill");
    let closed = |p: &PathSnapshot| {
        std::iter::once(p.closed)
            .chain(p.extra_subpaths.iter().map(|s| s.closed))
            .collect::<Vec<bool>>()
    };
    assert_eq!(
        closed(&on.glyphs),
        closed(&flat.glyphs),
        "open centre lines"
    );
    assert_same(
        &points(&on),
        &points(&flat),
        "single-line on a straight carrier",
    );
}

/// Criterion 14: with the substitute face the text is laid out on the carrier and tagged.
#[test]
fn c14_substitute_face_lays_out_on_the_carrier() {
    let face = sans();
    let fx = Fx::circle();
    let t = fx.plain("Missing", &OnPathValues::at(Length::from_mm(30.0)));
    let real = lay_out(&t, &face, false).unwrap();
    let sub = lay_out(&t, &face, true).unwrap();
    assert!(sub.substitute && !real.substitute);
    assert_same(&points(&sub), &points(&real), "laid out on the carrier");
}

/// Criteria 15 and 16: Ctrl snap stops (open: start, middle, end; closed: start, quarters and
/// the default point; ellipse: 15 and 22.5 degree stops), a drag snaps to them, a drag more than
/// `cross` to the other side switches Side and Line, back switches back.
#[test]
fn c15_c16_snap_stops_and_drag_across() {
    let face = sans();
    let fx = Fx::line(pt(0.0, 0.0), pt(100.0, 0.0));
    let t = fx.plain("Ab", &OnPathValues::at(Length::from_mm(0.0)));
    let m = measure(&t);
    let has = |stops: &[Length], s: f64| stops.iter().any(|x| (x.as_mm() - s).abs() < 1e-6);
    let stops = snap_stops(&m, None, Length::from_mm(0.0));
    for s in [0.0, 50.0, 100.0] {
        assert!(has(&stops, s), "open stop {s}: {stops:?}");
    }
    let start = t.on_path.as_ref().unwrap().values;
    let snapped = dragged_values(&m, start, pt(48.0, -1.0), 2.0, Some(&stops));
    assert!(
        (snapped.position.as_mm() - 50.0).abs() < 1e-6,
        "snaps to the middle"
    );
    let free = dragged_values(&m, start, pt(48.0, -1.0), 2.0, None);
    assert!((free.position.as_mm() - 48.0).abs() < TOL, "nearest point");
    assert_eq!((free.side, free.line), (start.side, start.line));
    // Above lies at y < 0 for a carrier drawn left to right; within `cross` below: no switch.
    let near = dragged_values(&m, start, pt(30.0, 1.5), 2.0, None);
    assert_eq!(
        (near.side, near.line),
        (PathSide::Above, LineOnPath::Baseline)
    );
    let across = dragged_values(&m, start, pt(30.0, 5.0), 2.0, None);
    assert_eq!(
        (across.side, across.line),
        (PathSide::Below, LineOnPath::TopEdge),
        "crossed: both switch, reading direction kept"
    );
    let back = dragged_values(&m, across, pt(30.0, -5.0), 2.0, None);
    assert_eq!(
        (back.side, back.line),
        (PathSide::Above, LineOnPath::Baseline)
    );

    // Closed carrier: start, quarters and the default point.
    let c = Fx::circle();
    let t = c.plain("Top", &OnPathValues::at(Length::from_mm(0.0)));
    let m = measure(&t);
    let len = m.length().as_mm();
    let laid = text_layout(&t, &face).unwrap();
    let def = default_position(&laid, &m);
    let ellipse = t.on_path.as_ref().unwrap().curve.as_ref().unwrap().ellipse;
    let stops = snap_stops(&m, None, def);
    for s in [0.0, len / 4.0, len / 2.0, 0.75 * len, def.as_mm()] {
        assert!(has(&stops, s), "closed stop {s}");
    }
    // On the ellipse: 15 and 22.5 degree stops.
    let e = ellipse.expect("a circle is an ellipse carrier");
    let stops = snap_stops(&m, Some(&e), def);
    for deg in [15.0, 22.5, 30.0, 45.0, 67.5, 90.0, 345.0] {
        let s = position_at_degrees(&m, &e, deg).as_mm();
        assert!(
            has(&stops, s) || stops.iter().any(|x| (x.as_mm() - s).abs() < 1e-3),
            "{deg} deg stop"
        );
    }
}

/// Criterion 18: Mirror keeps the reference point: the H at the top moves inside, hanging with
/// its top middle at (100, 50).
#[test]
fn c18_mirror_moves_across_and_keeps_the_reference_point() {
    let face = sans();
    let fx = Fx::circle();
    let t = fx.plain("H", &OnPathValues::at(Length::from_mm(0.0)));
    let laid = text_layout(&t, &face).unwrap();
    let v = t.on_path.as_ref().unwrap().values;
    let v = OnPathValues {
        position: default_position(&laid, &measure(&t)),
        ..v
    };
    let mirrored = lay(&with_values(&t, v.mirrored()), &face);
    let cx = centre_x(&t, &face);
    let want = lay(
        &straight(&t, Affine::translation(Vec2::new(100.0 - cx, 60.0))),
        &face,
    );
    assert_same(&points(&mirrored), &points(&want), "mirrored to the inside");
}

/// Criterion 19: Mirror letters draws each letter mirrored about its own vertical axis with the
/// reading order reversed: on a straight carrier the whole line is the mirror image of the
/// unmirrored line about one vertical axis.
#[test]
fn c19_mirror_letters_is_a_mirror_image_of_the_line() {
    let face = sans();
    let fx = Fx::line(pt(0.0, 0.0), pt(200.0, 0.0));
    let t = fx.plain("LF7R", &OnPathValues::at(Length::from_mm(20.0)));
    let v = t.on_path.as_ref().unwrap().values;
    let normal = points(&lay(&t, &face));
    let mirror = points(&lay(
        &with_values(
            &t,
            OnPathValues {
                mirror_letters: true,
                ..v
            },
        ),
        &face,
    ));
    let bc = |ps: &[Point]| {
        let lo = ps.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        let hi = ps.iter().map(|p| p.x).fold(f64::MIN, f64::max);
        f64::midpoint(lo, hi)
    };
    let axis = f64::midpoint(bc(&normal), bc(&mirror));
    let mut reflected: Vec<(i64, i64)> = normal
        .iter()
        .map(|p| ((2.0 * axis - p.x) * 1000.0).round() as i64)
        .zip(normal.iter().map(|p| (p.y * 1000.0).round() as i64))
        .collect();
    let mut got: Vec<(i64, i64)> = mirror
        .iter()
        .map(|p| ((p.x * 1000.0).round() as i64, (p.y * 1000.0).round() as i64))
        .collect();
    reflected.sort_unstable();
    got.sort_unstable();
    assert_eq!(got.len(), reflected.len());
    let worst = got
        .iter()
        .zip(&reflected)
        .map(|(a, b)| (a.0 - b.0).abs().max((a.1 - b.1).abs()))
        .max()
        .unwrap();
    assert!(worst <= 10, "mirror image within 0.01 mm, worst {worst} um");
    assert_ne!(normal, mirror, "something changed");
}

/// Criterion 20: a rotation by 30 degrees about the circle's centre moves the reference point 30
/// degrees along it; on an ellipse to the point at that parametric angle.
#[test]
fn c20_rotation_on_an_ellipse_carrier_moves_the_reference_point() {
    let fx = Fx::circle();
    let t = fx.plain("Rot", &OnPathValues::at(Length::from_mm(10.0)));
    let m = measure(&t);
    let e = t
        .on_path
        .as_ref()
        .unwrap()
        .curve
        .as_ref()
        .unwrap()
        .ellipse
        .unwrap();
    let v = t.on_path.as_ref().unwrap().values;
    let s = rotated_position(&m, &e, &v, 30.0).as_mm();
    assert!(
        (s - (10.0 + 50.0 * PI / 6.0)).abs() < 0.02,
        "30 degrees clockwise is {} mm further, got {s}",
        50.0 * PI / 6.0
    );

    let d = Document::new(1);
    let el = d.create_ellipse(EllipseFrame {
        center: pt(100.0, 100.0),
        rx: Length::from_mm(80.0),
        ry: Length::from_mm(40.0),
    });
    let fx = Fx::with(d, el);
    let t = fx.plain("Rot", &OnPathValues::at(Length::from_mm(0.0)));
    let m = measure(&t);
    let e = t
        .on_path
        .as_ref()
        .unwrap()
        .curve
        .as_ref()
        .unwrap()
        .ellipse
        .unwrap();
    let start = position_at_degrees(&m, &e, 40.0);
    let v = OnPathValues::at(start);
    let s = rotated_position(&m, &e, &v, 30.0);
    let want = m.at(position_at_degrees(&m, &e, 70.0)).point;
    let got = m.at(s).point;
    assert!(
        (got.x - want.x).hypot(got.y - want.y) < TOL,
        "parametric 70 degrees: {got:?} vs {want:?}"
    );
}

/// Criterion 21: Right moves Position in reading direction (not carrier direction), Up changes
/// Distance towards the top of the letters.
#[test]
fn c21_nudge_in_reading_direction_and_up_to_the_letter_tops() {
    use LineOnPath::{Baseline, TopEdge};
    use PathSide::{Above, Below};
    let with = vals(Above, Baseline, 20.0);
    let against = vals(Above, TopEdge, 20.0);
    assert!((nudged_values(with, 1.0, 0.0).position.as_mm() - 21.0).abs() < 1e-9);
    assert!((nudged_values(against, 1.0, 0.0).position.as_mm() - 19.0).abs() < 1e-9);
    assert!((nudged_values(with, -10.0, 0.0).position.as_mm() - 10.0).abs() < 1e-9);
    // Baseline on the carrier: the letters' top lies away from it, Distance grows.
    assert!((nudged_values(with, 0.0, 1.0).distance.as_mm() - 1.0).abs() < 1e-9);
    // Top edge on the carrier: the letters' top faces the carrier, Up brings them closer.
    assert!((nudged_values(against, 0.0, 1.0).distance.as_mm() + 1.0).abs() < 1e-9);
    assert!(
        (nudged_values(vals(Below, TopEdge, 0.0), 0.0, 1.0)
            .distance
            .as_mm()
            + 1.0)
            .abs()
            < 1e-9
    );
}

/// Criterion 23: Release writes a straight placement under which the first letter does not move.
#[test]
fn c23_release_keeps_the_first_letter_in_place() {
    let face = sans();
    let fx = Fx::circle();
    let t = fx.plain("Wood", &OnPathValues::at(Length::from_mm(33.0)));
    let laid = text_layout(&t, &face).unwrap();
    let placement = PathPlacement::new(&laid, t.on_path.as_ref().unwrap()).unwrap();
    let before = glyphs(&lay(&t, &face));
    let affine = release_placement(&t, &laid, &placement, None);
    fx.d.release_from_path(&[(t.id, affine)]).unwrap();
    let after_text = fx.d.text(t.id).unwrap();
    assert!(after_text.on_path.is_none());
    let after = glyphs(&lay(&after_text, &face));
    assert_same(&after[0], &before[0], "first letter stays");
    assert!((after_text.size.as_mm() - 10.0).abs() < 1e-12, "Size kept");
}

/// Criterion 24: a carrier edit is followed without writing the text: Position kept, the letters
/// on the edited outline; the drag preview gives the same text as the commit.
#[test]
fn c24_text_follows_a_carrier_edit_in_preview_and_commit() {
    let face = sans();
    let fx = Fx::circle();
    let t = fx.plain("H", &OnPathValues::at(Length::from_mm(0.0)));
    let laid = text_layout(&t, &face).unwrap();
    let v = t.on_path.as_ref().unwrap().values;
    let s = default_position(&laid, &measure(&t));
    fx.d.set_text_path(
        &[(t.id, OnPathValues { position: s, ..v })],
        curvyo_document_core::OnPathLabel::Move,
    )
    .unwrap();
    let steps = fx.d.own_steps().len();
    let objects = fx.d.read_objects().objects;
    let Some(ObjectSnapshot::Primitive(prim)) = fx.d.object(fx.carrier) else {
        panic!("the circle")
    };
    // The radius grows to 60 mm: as the drag preview, then committed.
    let mut changed = prim.clone();
    let frame = EllipseFrame {
        center: pt(100.0, 100.0),
        rx: Length::from_mm(60.0),
        ry: Length::from_mm(60.0),
    };
    fx.d.set_ellipse_frame(fx.carrier, frame).unwrap();
    assert_eq!(fx.d.own_steps().len(), steps + 1, "one step: the carrier's");
    let Some(ObjectSnapshot::Primitive(after)) = fx.d.object(fx.carrier) else {
        panic!()
    };
    changed.shape = after.shape;
    let previews = text_path_previews(&objects, &[ObjectSnapshot::Primitive(changed)], 100)
        .expect("one text follows");
    let preview = previews.iter().find(|p| p.id == t.id).expect("our text");
    let committed = fx.d.text(t.id).unwrap();
    assert_eq!(
        committed.on_path.as_ref().unwrap().values.position,
        s,
        "Position kept, the text not written"
    );
    let a = points(&lay(preview, &face));
    let b = points(&lay(&committed, &face));
    assert_same(&a, &b, "preview equals commit");
    // On the bigger circle at the same arc length: the letter middle is at angle s / 60 from 0 deg.
    let theta = s.as_mm() / 60.0;
    let curve = committed.on_path.as_ref().unwrap().curve.clone().unwrap();
    let sample = measure_of(&curve).at(s);
    let want = pt(100.0 + 60.0 * theta.cos(), 100.0 + 60.0 * theta.sin());
    assert!(
        (sample.point.x - want.x).hypot(sample.point.y - want.y) < 0.05,
        "{:?} vs {want:?}",
        sample.point
    );
    assert!(carrier_curve(&fx.d.object(fx.carrier).unwrap(), 0).is_some());
}

/// Criterion 26: nothing is hidden: past the end of an open carrier the letters run on the
/// straight extension, before its start too; on a closed one they go round and overlap.
#[test]
fn c26_overflow_draws_every_letter() {
    let face = sans();
    let fx = Fx::line(pt(0.0, 0.0), pt(10.0, 0.0));
    let t = fx.plain("HHHHHHHHHH", &OnPathValues::at(Length::from_mm(0.0)));
    let o = lay(&t, &face);
    assert_eq!(o.glyph_outlines.len(), 10, "all ten drawn");
    let g = glyphs(&o);
    let last = &g[9];
    assert!(last.iter().all(|p| p.x > 10.0), "past the end");
    assert!(
        last.iter().all(|p| p.y <= 0.01 && p.y >= -10.0),
        "on the straight extension, upright"
    );
    let before = glyphs(&lay(
        &with_values(&t, OnPathValues::at(Length::from_mm(-40.0))),
        &face,
    ));
    assert!(
        before[0]
            .iter()
            .all(|p| p.x < 0.0 && p.y <= 0.01 && p.y >= -10.0)
    );

    let d = Document::new(1);
    let small = d.create_ellipse(EllipseFrame {
        center: pt(0.0, 0.0),
        rx: Length::from_mm(5.0),
        ry: Length::from_mm(5.0),
    });
    let fx = Fx::with(d, small);
    let t = fx.plain("HHHHHHHHHH", &OnPathValues::at(Length::from_mm(0.0)));
    let o = lay(&t, &face);
    assert_eq!(o.glyph_outlines.len(), 10);
    assert!(
        points(&o).iter().all(|p| p.x.hypot(p.y) < 5.0 + 10.0 + 1.0),
        "round and round, near the circle"
    );
}

/// Criterion 35: a Position outside a closed carrier's length reads modulo the length.
#[test]
fn c35_position_past_the_length_reads_modulo_on_a_closed_carrier() {
    let face = sans();
    let fx = Fx::circle();
    let t = fx.plain("Mod", &OnPathValues::at(Length::from_mm(10.0)));
    let c = circumference(&t);
    let base = points(&lay(&t, &face));
    for s in [10.0 + c, 10.0 - 2.0 * c] {
        let got = points(&lay(
            &with_values(&t, OnPathValues::at(Length::from_mm(s))),
            &face,
        ));
        assert_same(&got, &base, &format!("position {s}"));
    }
}

/// Criterion 47: Vertical kerning moves that letter alone along the normal (+2 mm on the middle
/// H at the top: up by 2 mm); Horizontal kerning on the last H moves it 3 mm further along the
/// circle.
#[test]
fn c47_kerns_along_tangent_and_normal() {
    let face = sans();
    let fx = Fx::circle();
    let plain = fx.plain("HHH", &OnPathValues::at(Length::from_mm(0.0)));
    let laid = text_layout(&plain, &face).unwrap();
    let v = plain.on_path.as_ref().unwrap().values;
    let s = default_position(&laid, &measure(&plain));
    let at = OnPathValues { position: s, ..v };
    let kerned = |chars: TextChars| {
        let t = fx.text(
            "HHH",
            &at,
            TextLayoutValues::DEFAULT,
            &chars,
            FontKind::Outline,
        );
        glyphs(&lay(&t, &face))
    };
    let base = kerned(TextChars::default());
    let up = kerned(TextChars::new(vec![(
        1,
        CharValues {
            kern_y: 0.2,
            ..CharValues::DEFAULT
        },
    )]));
    assert_same(&up[0], &base[0], "first H as without the kern");
    assert_same(&up[2], &base[2], "last H as without the kern");
    let lifted: Vec<Point> = base[1].iter().map(|p| pt(p.x, p.y - 2.0)).collect();
    assert_same(
        &up[1],
        &lifted,
        "middle H 2 mm up: baseline middle at (100, 48)",
    );
    // The middle H of the unkerned text stands upright at x = 100 (the text centred at the top).
    let mid = &base[1];
    let lo = mid.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    let hi = mid.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!(
        (f64::midpoint(lo, hi) - 100.0).abs() < 0.05,
        "centred: {lo}..{hi}"
    );

    let along = kerned(TextChars::new(vec![(
        2,
        CharValues {
            kern_x: 0.3,
            ..CharValues::DEFAULT
        },
    )]));
    let angle = |ps: &[Point]| {
        let n = ps.len() as f64;
        let (sx, sy) = ps.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
        (sy / n - 100.0).atan2(sx / n - 100.0)
    };
    // The same glyph turned about the same circle: its centroid's angle is its middle's angle.
    // Position is the text's middle (customer 2026-10-24): the kern widens the text by 3 mm about
    // its middle, so the last H lies 3 mm further from the first one along the circle.
    let moved = |g: usize| (angle(&along[g]) - angle(&base[g])) * 50.0;
    assert!(
        (moved(2) - moved(0) - 3.0).abs() < TOL,
        "last H 3 mm further clockwise from the first, got {} and {}",
        moved(0),
        moved(2)
    );
}

/// Criterion 48: a vertical text on a path is laid out as the horizontal one; Release keeps its
/// writing mode.
#[test]
fn c48_vertical_text_runs_horizontally_and_keeps_its_mode() {
    let face = sans();
    let fx = Fx::circle();
    let at = OnPathValues::at(Length::from_mm(200.0));
    let h = fx.text(
        "AB",
        &at,
        TextLayoutValues::DEFAULT,
        &TextChars::default(),
        FontKind::Outline,
    );
    let vertical = TextLayoutValues {
        writing_mode: TextWritingMode::VerticalRl,
        ..TextLayoutValues::DEFAULT
    };
    let v = fx.text(
        "AB",
        &at,
        vertical,
        &TextChars::default(),
        FontKind::Outline,
    );
    assert_eq!(
        v.layout.writing_mode,
        TextWritingMode::VerticalRl,
        "stored as is"
    );
    assert_same(
        &points(&lay(&v, &face)),
        &points(&lay(&h, &face)),
        "as horizontal",
    );

    let laid = text_layout(&v, &face).unwrap();
    let placement = PathPlacement::new(&laid, v.on_path.as_ref().unwrap()).unwrap();
    let vertical_layout = stored_vertical_layout(&v, &face);
    assert!(vertical_layout.is_some());
    let affine = release_placement(&v, &laid, &placement, vertical_layout.as_ref());
    fx.d.release_from_path(&[(v.id, affine)]).unwrap();
    let released = fx.d.text(v.id).unwrap();
    assert!(released.on_path.is_none());
    assert_eq!(released.layout.writing_mode, TextWritingMode::VerticalRl);
}

/// Criterion 17: the history names of the on-path steps.
#[test]
fn c17_history_names() {
    for (label, name) in [
        ("put_text_on_path", "Put text on path"),
        ("change_text_side", "Change text side"),
        ("change_line_on_path", "Change line on path"),
        ("move_text_on_path", "Move text on path"),
        ("change_distance_on_path", "Change distance on path"),
        ("mirror_text_on_path", "Mirror text on path"),
        ("release_from_path", "Release from path"),
    ] {
        assert_eq!(operation_name(label), name);
    }
}

/// A path of `n` cubic segments, a gentle wave from x = 0 rightwards.
fn wave(d: &Document, n: usize, step: f64) -> NodeId {
    let anchors: Vec<NewAnchor> = (0..=n)
        .map(|i| {
            let x = i as f64 * step;
            let y = if i % 2 == 0 { 0.0 } else { 3.0 };
            NewAnchor {
                id: AnchorId::new(9, i as u64 + 100),
                point: pt(x, y),
                handle_in: Vec2::new(-step / 3.0, 0.0),
                handle_out: Vec2::new(step / 3.0, 0.0),
                kind: AnchorKind::Symmetric,
            }
        })
        .collect();
    d.create_path(&anchors, false)
}

fn two_hundred() -> String {
    "The quick brown fox jumps over the lazy dog. ".repeat(5)[..200].to_owned()
}

/// Criterion 43: 200 characters on 1,000 cubics: a position-handle drag frame (drag values,
/// layout) and a carrier node drag frame (preview, layout) within 33 ms. Release build:
/// `cargo nextest run --release -p curvyo-ui-core --test acceptance_0080_tester --run-ignored only`.
#[test]
#[ignore = "release-mode benchmark"]
fn c43_budget_drag_frames_within_33_ms() {
    let face = sans();
    let d = Document::new(1);
    let carrier = wave(&d, 1000, 2.0);
    let fx = Fx::with(d, carrier);
    let t = fx.plain(&two_hundred(), &OnPathValues::at(Length::from_mm(10.0)));
    let m = measure(&t);
    let start = t.on_path.as_ref().unwrap().values;
    let mut worst = Duration::ZERO;
    for i in 0..30 {
        let t0 = Instant::now();
        let v = dragged_values(&m, start, pt(10.0 + f64::from(i) * 30.0, -2.0), 1.0, None);
        let o = lay(&with_values(&t, v), &face);
        assert!(o.glyph_outlines.len() > 150);
        worst = worst.max(t0.elapsed());
    }
    println!("position drag worst frame {worst:?}");
    assert!(worst < Duration::from_millis(33), "position drag {worst:?}");

    let objects = fx.d.read_objects().objects;
    let mut path = fx.d.path(carrier).unwrap();
    let mut worst = Duration::ZERO;
    for i in 0..30 {
        path.anchors[500].point = pt(1000.0, f64::from(i));
        let t0 = Instant::now();
        let previews =
            text_path_previews(&objects, &[ObjectSnapshot::Path(path.clone())], 100).unwrap();
        for p in &previews {
            let _ = lay(p, &face);
        }
        worst = worst.max(t0.elapsed());
    }
    println!("carrier node drag worst frame {worst:?}");
    assert!(worst < Duration::from_millis(33), "node drag {worst:?}");
}

/// Criterion 45: a carrier of 100,000 segments neither hangs nor crashes; letters that would
/// land beyond 1e7 mm do not crash the layout.
#[test]
fn c45_huge_carrier_and_far_letters_do_not_hang_or_crash() {
    let face = sans();
    let d = Document::new(1);
    let carrier = wave(&d, 100_000, 1.0);
    let fx = Fx::with(d, carrier);
    let t0 = Instant::now();
    let t = fx.plain(&two_hundred(), &OnPathValues::at(Length::from_mm(50_000.0)));
    let o = lay(&t, &face);
    assert_ne!(o.glyph_outlines.len(), 0);
    assert!(t0.elapsed() < Duration::from_secs(20), "{:?}", t0.elapsed());

    let fx = Fx::line(pt(0.0, 0.0), pt(10.0, 0.0));
    for s in [2e7, -2e7, 1e300, f64::MAX] {
        let t = fx.plain("Far away", &OnPathValues::at(Length::from_mm(s)));
        let o = lay_out(&t, &face, false);
        if let Some(o) = o {
            assert!(
                points(&o)
                    .iter()
                    .all(|p| p.x.is_finite() && p.y.is_finite()),
                "{s}: no non-finite point drawn"
            );
        }
    }
}
