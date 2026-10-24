//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 1 on the
//! pure rules in `curvyo-ui-core`: relative values times Size (criteria 10, 34), the anchor shift
//! of an alignment change (4), the field views, Mixed and Auto (7, 10, 13), typed values held per
//! text (7, 13), steps and the drag grid (7) and the hit box over turned glyphs (15).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(clippy::unwrap_used, clippy::float_cmp, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, FontKind, FontRef, Length, Point, Style, TextAlign, TextLayoutValues,
    TextSnapshot, UNCREATED_TEXT_ID, Vec2,
};
use curvyo_text_core::{Face, FaceStore, Mm};
use curvyo_ui_core::{
    Grid, LayoutField, LayoutSubject, field_view, lay_out, layout_options, parse_field_text,
    realigned_placement, set_field, stepped, text_layout, value_at,
};

const TOL: f64 = 0.01;
const D: TextLayoutValues = TextLayoutValues::DEFAULT;

fn sans<R>(f: impl FnOnce(&Face) -> R) -> R {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"),
    )
    .unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    f(store.get(&key).unwrap())
}

fn snapshot(string: &str, size: f64, layout: TextLayoutValues, placement: Affine) -> TextSnapshot {
    TextSnapshot {
        id: UNCREATED_TEXT_ID,
        string: string.to_owned(),
        font: FontRef {
            family: "Source Sans 3".to_owned(),
            style: "Regular".to_owned(),
            face_hash: "c".repeat(64),
            face_index: 0,
            kind: FontKind::Outline,
        },
        size: Length::from_mm(size),
        layout,
        placement,
        style: Style::default(),
        outline: None,
    }
}

fn subject(size: f64, values: TextLayoutValues) -> LayoutSubject {
    LayoutSubject {
        size_mm: size,
        values,
        natural_line_mm: size * 1.4,
    }
}

fn deg(d: f64) -> Angle {
    Angle::from_radians(d.to_radians())
}

/// Criteria 10 and 34: the engine gets every relative value times Size, so a doubled Size
/// doubles the mm; the rotation does not scale.
#[test]
fn ac34_relative_values_times_size() {
    let v = TextLayoutValues {
        align: TextAlign::Right,
        letter_spacing: 0.2,
        word_spacing: -0.1,
        line_spacing: Some(1.5),
        rotation: deg(30.0),
        ..TextLayoutValues::DEFAULT
    };
    for (size, line) in [(10.0, 15.0), (20.0, 30.0)] {
        let o = layout_options(Length::from_mm(size), &v);
        assert!((o.letter_spacing.0 - 0.2 * size).abs() < 1e-9);
        assert!((o.word_spacing.0 + 0.1 * size).abs() < 1e-9);
        assert!((o.line_spacing.unwrap().0 - line).abs() < 1e-9);
        assert!((o.rotation_rad - 30f64.to_radians()).abs() < 1e-12);
        assert_eq!(o.align, curvyo_text_core::Align::Right);
    }
    let auto = layout_options(Length::from_mm(10.0), &D);
    assert_eq!(auto, curvyo_text_core::LayoutOptions::default());
    // Criterion 10 end to end: two lines at 1.5 x Size are 15 then 30 mm apart.
    sans(|face| {
        for (size, gap) in [(10.0, 15.0), (20.0, 30.0)] {
            let l = text_layout(
                &snapshot(
                    "A\nB",
                    size,
                    TextLayoutValues {
                        line_spacing: Some(1.5),
                        ..D
                    },
                    Affine::IDENTITY,
                ),
                face,
            )
            .unwrap();
            assert!((l.lines[1].baseline.0 - l.lines[0].baseline.0 - gap).abs() < TOL);
        }
    });
}

/// Criterion 4: an alignment change moves the anchor along the baseline so the widest line's
/// extent keeps its place on the canvas; the other lines take the new alignment. Also with a
/// turned placement.
#[test]
fn ac04_widest_line_keeps_its_place() {
    sans(|face| {
        let turn = Affine::rotation_about(Point::new(0.0, 0.0), deg(30.0))
            .compose(Affine::translation(Vec2::new(40.0, 25.0)));
        for placement in [Affine::translation(Vec2::new(40.0, 25.0)), turn] {
            for (from, to) in [
                (TextAlign::Left, TextAlign::Centre),
                (TextAlign::Left, TextAlign::Right),
                (TextAlign::Right, TextAlign::Justify),
                (TextAlign::Centre, TextAlign::Left),
            ] {
                let old = snapshot(
                    "AA\nAAAAAA\nA",
                    10.0,
                    TextLayoutValues { align: from, ..D },
                    placement,
                );
                let moved = realigned_placement(&old, face, to);
                let new = snapshot(
                    "AA\nAAAAAA\nA",
                    10.0,
                    TextLayoutValues { align: to, ..D },
                    moved,
                );
                let lo = text_layout(&old, face).unwrap();
                let ln = text_layout(&new, face).unwrap();
                let at = |p: Affine, l: &curvyo_text_core::TextLayout, x: f64| {
                    p.apply_point(Point::new(x, l.lines[1].baseline.0))
                };
                let a = at(placement, &lo, lo.lines[1].extent.start.0);
                let b = at(moved, &ln, ln.lines[1].extent.start.0);
                assert!(
                    (a.x - b.x).abs() < TOL && (a.y - b.y).abs() < TOL,
                    "{from:?} -> {to:?}: {a:?} vs {b:?}"
                );
                // The anchor moved along the baseline only: the move is parallel to the
                // placement's x axis.
                let shift = moved.apply_point(Point::new(0.0, 0.0));
                let local = placement.inverse().unwrap().apply_point(shift);
                assert!(local.y.abs() < 1e-9, "{local:?}");
                // Another line takes the new alignment: for Centre its midpoint is the anchor.
                if to == TextAlign::Centre {
                    let e = &ln.lines[0].extent;
                    assert!(f64::midpoint(e.start.0, e.end.0).abs() < TOL);
                }
            }
        }
        // Same alignment: the placement is unchanged.
        let t = snapshot("A\nAAA", 10.0, D, Affine::translation(Vec2::new(5.0, 5.0)));
        assert_eq!(realigned_placement(&t, face, TextAlign::Left), t.placement);
    });
}

/// Criteria 7 and 10: defaults show 0 with no reset; Line spacing Auto shows the natural spacing,
/// marked automatic, with no reset; a set value shows the reset.
#[test]
fn ac07_ac10_field_views_at_default_and_auto() {
    let s = [subject(20.0, D)];
    for f in [
        LayoutField::LetterSpacing,
        LayoutField::WordSpacing,
        LayoutField::CharacterRotation,
    ] {
        let v = field_view(f, &s);
        assert_eq!(
            (v.text.as_str(), v.value, v.mixed, v.resettable),
            ("0", 0.0, false, false),
            "{f:?}"
        );
    }
    let line = field_view(LayoutField::LineSpacing, &s);
    assert!(line.auto && !line.resettable && !line.mixed);
    assert!(
        (line.value - 28.0).abs() < 1e-9,
        "natural spacing shown: {}",
        line.value
    );
    assert_eq!(line.text, "28");
    let set = field_view(
        LayoutField::LineSpacing,
        &[subject(
            20.0,
            TextLayoutValues {
                line_spacing: Some(0.75),
                ..D
            },
        )],
    );
    assert!(!set.auto && set.resettable);
    assert!((set.value - 15.0).abs() < 1e-9);
    // Tracks at Size 20 (criterion 7 table).
    for (f, lo, hi) in [
        (LayoutField::LetterSpacing, -5.0, 20.0),
        (LayoutField::WordSpacing, -10.0, 40.0),
        (LayoutField::LineSpacing, 0.0, 60.0),
        (LayoutField::CharacterRotation, -180.0, 180.0),
    ] {
        let v = field_view(f, &s);
        assert!(
            (v.track_min - lo).abs() < 1e-9 && (v.track_max - hi).abs() < 1e-9,
            "{f:?}: {v:?}"
        );
    }
    // A negative value is shown with U+2212 and up to two decimals.
    let neg = field_view(
        LayoutField::LetterSpacing,
        &[subject(
            10.0,
            TextLayoutValues {
                letter_spacing: -0.12345,
                ..D
            },
        )],
    );
    assert_eq!(neg.text, "\u{2212}1.23");
    let rot = field_view(
        LayoutField::CharacterRotation,
        &[subject(
            10.0,
            TextLayoutValues {
                rotation: deg(-12.5),
                ..D
            },
        )],
    );
    assert_eq!(rot.text, "\u{2212}12.5");
}

/// Criterion 13: the same value in mm on texts of different Size is not Mixed; different values
/// are Mixed with no bar; the track uses the smallest Size.
#[test]
fn ac13_mixed_and_smallest_size() {
    let same = [
        subject(
            10.0,
            TextLayoutValues {
                letter_spacing: 0.2,
                ..D
            },
        ),
        subject(
            20.0,
            TextLayoutValues {
                letter_spacing: 0.1,
                ..D
            },
        ),
    ];
    let v = field_view(LayoutField::LetterSpacing, &same);
    assert!(!v.mixed);
    assert_eq!(v.text, "2");
    assert!(
        (v.track_min + 2.5).abs() < 1e-9 && (v.track_max - 10.0).abs() < 1e-9,
        "{v:?}"
    );
    let differ = [
        subject(
            10.0,
            TextLayoutValues {
                letter_spacing: 0.2,
                ..D
            },
        ),
        subject(10.0, D),
    ];
    let m = field_view(LayoutField::LetterSpacing, &differ);
    assert!(m.mixed && m.text.is_empty() && m.resettable);
    // Line spacing: one Auto, one set: Mixed.
    let lines = [
        subject(10.0, D),
        subject(
            10.0,
            TextLayoutValues {
                line_spacing: Some(1.0),
                ..D
            },
        ),
    ];
    assert!(field_view(LayoutField::LineSpacing, &lines).mixed);
}

/// Criteria 7 and 13: a value in mm becomes each text's own factor; past a text's range it is
/// held at that text's limit and counted.
#[test]
fn ac07_ac13_typed_values_held_per_text() {
    let one = [subject(20.0, D)];
    let c = set_field(LayoutField::LetterSpacing, &one, Some(100.0));
    assert!(
        (c.values[0].letter_spacing - 2.0).abs() < 1e-12,
        "40 mm at Size 20"
    );
    assert_eq!(c.limited, 1);
    let (top, mm) = c.limit.unwrap();
    assert!(top && (mm - 40.0).abs() < 1e-9);
    let c = set_field(LayoutField::LetterSpacing, &one, Some(-50.0));
    assert!((c.values[0].letter_spacing + 0.5).abs() < 1e-12);
    assert_eq!(
        c.limit.map(|(t, m)| (t, (m * 100.0).round())),
        Some((false, -1000.0))
    );
    // Two sizes: 30 mm fits Size 20 (limit 40) but not Size 10 (limit 20).
    let two = [subject(10.0, D), subject(20.0, D)];
    let c = set_field(LayoutField::LetterSpacing, &two, Some(30.0));
    assert!((c.values[0].letter_spacing - 2.0).abs() < 1e-12);
    assert!((c.values[1].letter_spacing - 1.5).abs() < 1e-12);
    assert_eq!(c.limited, 1);
    // Inside every range: nothing limited; each stored relative to its own Size.
    let c = set_field(LayoutField::WordSpacing, &two, Some(5.0));
    assert_eq!(c.limited, 0);
    assert!(
        (c.values[0].word_spacing - 0.5).abs() < 1e-12
            && (c.values[1].word_spacing - 0.25).abs() < 1e-12
    );
    // Line spacing: None is Auto; above 10 x Size is held; other values untouched.
    let keep = [subject(
        10.0,
        TextLayoutValues {
            letter_spacing: 0.3,
            ..D
        },
    )];
    let c = set_field(LayoutField::LineSpacing, &keep, Some(500.0));
    assert_eq!(c.values[0].line_spacing, Some(10.0));
    assert_eq!(c.values[0].letter_spacing, 0.3);
    let c = set_field(LayoutField::LineSpacing, &keep, None);
    assert_eq!(c.values[0].line_spacing, None);
    let c = set_field(LayoutField::LineSpacing, &keep, Some(-3.0));
    assert_eq!(c.values[0].line_spacing, Some(0.0));
}

/// Criterion 7: a typed rotation outside -180..180 wraps (270 becomes -90), nothing is limited.
#[test]
fn ac07_rotation_wraps() {
    let one = [subject(10.0, D)];
    for (typed, want) in [
        (270.0, -90.0),
        (-270.0, 90.0),
        (180.0, 180.0),
        (540.0, 180.0),
        (-180.0, 180.0),
        (725.0, 5.0),
    ] {
        let c = set_field(LayoutField::CharacterRotation, &one, Some(typed));
        let got = c.values[0].rotation.as_radians().to_degrees();
        assert!((got - want).abs() < 1e-9, "{typed} -> {got}");
        assert_eq!(c.limited, 0);
    }
}

/// Criterion 7 (UX §0.1): arrows step 0.1 mm, Shift 1 mm, Ctrl 0.01 mm; rotation 1°, 15°, 0.1°.
#[test]
fn ac07_arrow_steps() {
    for f in [
        LayoutField::LetterSpacing,
        LayoutField::WordSpacing,
        LayoutField::LineSpacing,
    ] {
        assert!(
            (stepped(f, 1.0, 1, Grid::Normal) - 1.1).abs() < 1e-9,
            "{f:?}"
        );
        assert!((stepped(f, 1.0, 1, Grid::Coarse) - 2.0).abs() < 1e-9);
        assert!((stepped(f, 1.0, -1, Grid::Fine) - 0.99).abs() < 1e-9);
    }
    let r = LayoutField::CharacterRotation;
    assert!((stepped(r, 10.0, 1, Grid::Normal) - 11.0).abs() < 1e-9);
    // Steps land on the step grid, as every value field of `0047` does.
    assert!((stepped(r, 15.0, -1, Grid::Coarse)).abs() < 1e-9);
    assert!((stepped(r, 15.0, 1, Grid::Coarse) - 30.0).abs() < 1e-9);
    assert!((stepped(r, 10.0, 1, Grid::Fine) - 10.1).abs() < 1e-9);
}

/// Criterion 7: a drag lands on the 1-2-5 grid of at least the track span over the field width;
/// a rotation drag stops at -180 and 180; spacing continues past the track up to the typed limit.
#[test]
fn ac07_drag_grid_and_ends() {
    let s = [subject(10.0, D)];
    let r = LayoutField::CharacterRotation;
    assert_eq!(value_at(r, &s, 1.7, Grid::Normal), 180.0);
    assert_eq!(value_at(r, &s, -0.7, Grid::Normal), -180.0);
    let is_on_125 = |v: f64| {
        // Some step of the form {1,2,5} x 10^k divides v.
        let a = v.abs();
        a < 1e-12
            || (-4..4).any(|k| {
                [1.0, 2.0, 5.0].iter().any(|m| {
                    let step = m * 10f64.powi(k);
                    ((a / step) - (a / step).round()).abs() < 1e-6 && step >= 12.5 / 1000.0
                })
            })
    };
    for p in [0.0, 0.123, 0.377, 0.5, 0.731, 0.999] {
        let v = value_at(LayoutField::LetterSpacing, &s, p, Grid::Normal);
        assert!(is_on_125(v), "p {p} -> {v}");
        assert!((-2.5 - 0.2..=10.0 + 0.2).contains(&v), "{p} -> {v}");
    }
    // Track ends at Size 10: -2.5 and 10 mm.
    assert!((value_at(LayoutField::LetterSpacing, &s, 0.0, Grid::Normal) + 2.5).abs() < 0.11);
    assert!((value_at(LayoutField::LetterSpacing, &s, 1.0, Grid::Normal) - 10.0).abs() < 0.11);
    // Overshoot: far past the right end stops at the typed limit, 2 x Size.
    let far = value_at(LayoutField::LetterSpacing, &s, 50.0, Grid::Normal);
    assert!((far - 20.0).abs() < 1e-9, "{far}");
    let far = value_at(LayoutField::LetterSpacing, &s, -50.0, Grid::Normal);
    assert!((far + 5.0).abs() < 1e-9, "{far}");
}

/// Criterion 7: typed lengths take the units of `0052`; other text is not a number.
#[test]
fn ac07_parse_typed_values() {
    let f = LayoutField::LetterSpacing;
    assert_eq!(parse_field_text(f, "2"), Some(2.0));
    assert_eq!(parse_field_text(f, "2 mm"), Some(2.0));
    assert!((parse_field_text(f, "0.1 in").unwrap() - 2.54).abs() < 1e-9);
    assert_eq!(parse_field_text(f, "abc"), None);
    assert_eq!(parse_field_text(f, ""), None);
    assert_eq!(
        parse_field_text(LayoutField::CharacterRotation, "12°"),
        Some(12.0)
    );
    assert!(
        parse_field_text(f, "\u{2212}1.5").is_some_and(|v| (v + 1.5).abs() < 1e-9),
        "U+2212 minus as shown"
    );
}

/// Criterion 15: the hit area of a text with turned, spaced glyphs covers their outlines.
#[test]
fn ac15_hit_box_covers_turned_glyphs() {
    sans(|face| {
        for values in [
            TextLayoutValues {
                letter_spacing: 0.5,
                align: TextAlign::Centre,
                rotation: deg(40.0),
                ..D
            },
            TextLayoutValues {
                rotation: deg(90.0),
                line_spacing: Some(0.2),
                ..D
            },
            TextLayoutValues {
                word_spacing: -1.0,
                rotation: deg(-135.0),
                align: TextAlign::Justify,
                ..D
            },
        ] {
            let t = snapshot(
                "Ijg W\nWWW",
                10.0,
                values,
                Affine::translation(Vec2::new(50.0, 50.0)),
            );
            let out = lay_out(&t, face, false).unwrap();
            let (xs, ys): (Vec<f64>, Vec<f64>) = out.line_box.iter().map(|p| (p.x, p.y)).unzip();
            let (x0, x1) = (
                xs.iter().copied().fold(f64::INFINITY, f64::min),
                xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            );
            let (y0, y1) = (
                ys.iter().copied().fold(f64::INFINITY, f64::min),
                ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            );
            for a in out.glyphs.all_anchors() {
                let p = a.point;
                assert!(
                    p.x >= x0 - TOL && p.x <= x1 + TOL && p.y >= y0 - TOL && p.y <= y1 + TOL,
                    "{values:?}: {p:?} outside {x0}..{x1} x {y0}..{y1}"
                );
            }
        }
    });
}

/// Criterion 35 at the mapping level: the drawn outlines are the engine's glyph paths with the
/// mm values, mapped by the placement.
#[test]
fn ac35_drawn_outline_is_the_engine_layout() {
    sans(|face| {
        let values = TextLayoutValues {
            align: TextAlign::Right,
            letter_spacing: 0.1,
            word_spacing: 0.3,
            line_spacing: Some(1.2),
            rotation: deg(20.0),
            ..TextLayoutValues::DEFAULT
        };
        let placement = Affine::translation(Vec2::new(30.0, 40.0));
        let t = snapshot("Ab c\nde", 10.0, values, placement);
        let out = lay_out(&t, face, false).unwrap();
        let engine = curvyo_text_core::layout(
            face,
            "Ab c\nde",
            Mm(10.0),
            &curvyo_text_core::LayoutOptions {
                align: curvyo_text_core::Align::Right,
                letter_spacing: Mm(1.0),
                word_spacing: Mm(3.0),
                line_spacing: Some(Mm(12.0)),
                rotation_rad: 20f64.to_radians(),
                ..curvyo_text_core::LayoutOptions::default()
            },
        )
        .unwrap();
        let mut want: Vec<(f64, f64)> = Vec::new();
        for path in engine.glyph_paths(face) {
            for el in path.elements() {
                if let Some(p) = el.end_point() {
                    want.push((p.x + 30.0, p.y + 40.0));
                }
            }
        }
        let got: Vec<(f64, f64)> = out
            .glyphs
            .all_anchors()
            .map(|a| (a.point.x, a.point.y))
            .collect();
        for g in &got {
            assert!(
                want.iter()
                    .any(|w| (w.0 - g.0).abs() < TOL && (w.1 - g.1).abs() < TOL),
                "anchor {g:?} is not on the engine's outline"
            );
        }
        assert_eq!(out.glyph_outlines.len(), 5, "one outline per drawn glyph");
    });
}
