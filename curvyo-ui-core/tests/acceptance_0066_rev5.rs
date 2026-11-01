//! Independent layout oracle for 0066 revision 5 AC49: a horizontal or vertical distance
//! annotation draws only two extension lines, one dimension line (with the text gap) and no
//! other segment, whatever the geometry, label side and zoom. Written from the specification and
//! UX notes "Dimension anatomy" before the implementation diff was read.
#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::manual_midpoint,
    clippy::collapsible_if,
    clippy::assert_is_empty,
    clippy::cast_precision_loss,
    missing_docs
)]
use curvyo_document_core::{Point, Vec2, ViewTransform};
use curvyo_ui_core::{DimensionGeometry, DimensionItem, layout_dimensions};
use proptest::prelude::*;

type Pt = (f64, f64);

fn screen(p: Point, k: f64) -> Pt {
    (p.x * k, p.y * k)
}
fn doc(p: Pt, k: f64) -> Point {
    Point::new(p.0 / k, p.1 / k)
}

/// Lays out one projected dimension in an identity-origin view of scale `k` and returns the
/// failures against the anatomy; coordinates in screen px.
fn check(k: f64, a: Pt, b: Pt, horizontal: bool, label: Option<Pt>, chars: usize) -> Vec<String> {
    let along = if horizontal {
        Vec2 { x: 1.0, y: 0.0 }
    } else {
        Vec2 { x: 0.0, y: 1.0 }
    };
    let item = DimensionItem {
        geometry: DimensionGeometry::Projected {
            from: doc(a, k),
            to: doc(b, k),
            along,
        },
        label: label.map(|p| doc(p, k)),
        text_chars: chars,
    };
    let marks = layout_dimensions(&[item], ViewTransform::new(k, Point::new(0.0, 0.0)));
    verify(k, &marks[0], a, b, horizontal, label, chars)
}

fn verify(
    k: f64,
    m: &curvyo_ui_core::DimensionMarks,
    a: Pt,
    b: Pt,
    horizontal: bool,
    label: Option<Pt>,
    chars: usize,
) -> Vec<String> {
    let mut why = Vec::new();
    let text = screen(m.text, k);
    // (along, across) accessors
    let al = |p: Pt| if horizontal { p.0 } else { p.1 };
    let ac = |p: Pt| if horizontal { p.1 } else { p.0 };
    let line = ac(text);
    let (la, lb) = (al(a), al(b));
    let half = chars as f64 * 7.0 / 2.0 + 8.0;
    let (lo, hi) = (
        la.min(lb).min(al(text) - half) - 14.0,
        la.max(lb).max(al(text) + half) + 14.0,
    );
    let mut ext = 0;
    let span = (lb - la).abs();
    for (p, q) in &m.lines {
        let (p, q) = (screen(*p, k), screen(*q, k));
        let (d_al, d_ac) = ((al(q) - al(p)).abs(), (ac(q) - ac(p)).abs());
        if d_al < 0.5 && d_ac >= 0.5 {
            // extension line: stands across the measured direction at one of the points
            ext += 1;
            let at = al(p);
            let owners: Vec<Pt> = [a, b]
                .into_iter()
                .filter(|o| (al(*o) - at).abs() < 0.5)
                .collect();
            if owners.is_empty() {
                why.push(format!(
                    "extension line at {at:.2} is not at a measured point ({la:.2}, {lb:.2})"
                ));
                continue;
            }
            // the start must be 4 px off one owner and the end 4 px past the dimension line
            let ok = owners.iter().any(|owner| {
                let sg = if line >= ac(*owner) { 1.0 } else { -1.0 };
                [(ac(p), ac(q)), (ac(q), ac(p))]
                    .iter()
                    .any(|(first, last)| {
                        ((first - ac(*owner)) * sg - 4.0).abs() < 0.5
                            && ((last - line) * sg - 4.0).abs() < 0.5
                    })
            });
            let gap_ok = owners.iter().any(|o| (line - ac(*o)).abs() <= 8.5);
            if !ok && !gap_ok {
                why.push(format!(
                    "extension line {p:?}-{q:?} does not run from 4 px off its point to 4 px past the dimension line (line at {line:.2}, owners {owners:?})"
                ));
            }
        } else if d_ac < 0.5 {
            if d_al >= 0.5 && (ac(p) - line).abs() > 0.5 {
                why.push(format!("segment {p:?}-{q:?} runs parallel but off the dimension line (line at {line:.2})"));
            }
            let (s0, s1) = (al(p).min(al(q)), al(p).max(al(q)));
            if d_al >= 0.5 && (s0 < lo - 0.5 || s1 > hi + 0.5) {
                why.push(format!(
                    "dimension segment {s0:.1}..{s1:.1} leaves the hull {lo:.1}..{hi:.1}"
                ));
            }
        } else {
            // oblique: only a tick (under 24 px span), short and centred on the dimension line
            let len = d_al.hypot(d_ac);
            let mid_ac = (ac(p) + ac(q)) / 2.0;
            if !(span < 24.0 && len <= 9.0 && (mid_ac - line).abs() < 0.5) {
                why.push(format!("oblique segment {p:?}-{q:?} (span {span:.1})"));
            }
        }
    }
    if ext > 2 {
        why.push(format!("{ext} extension lines, want at most 2"));
    }
    // the text lies on the dimension line it labels (the label's across coordinate)
    if let Some(l) = label {
        if (ac(text) - ac(l)).abs() > 0.5 {
            why.push(format!(
                "text sits {:.1} px off the placed dimension line",
                ac(text) - ac(l)
            ));
        }
    }
    why
}

#[test]
fn customer_quadrilateral_every_zoom_and_side() {
    let mut fails = Vec::new();
    for k in [0.378, 0.945, 3.78, 15.1, 37.8] {
        for (a, b, horizontal) in [
            ((100.0, 250.0), (480.0, 360.0), true),
            ((280.0, 100.0), (330.0, 530.0), false),
        ] {
            let mid = if horizontal {
                (a.1 + b.1) / 2.0
            } else {
                (a.0 + b.0) / 2.0
            };
            for offset in [
                None,
                Some(-80.0),
                Some(60.0),
                Some(-300.0),
                Some(0.0),
                Some(300.0),
            ] {
                let label = offset.map(|o| {
                    if horizontal {
                        ((a.0 + b.0) / 2.0 + 17.0, mid + o)
                    } else {
                        (mid + o, (a.1 + b.1) / 2.0 - 11.0)
                    }
                });
                for f in check(k, a, b, horizontal, label, 12) {
                    fails.push(format!(
                        "k {k} {} label {label:?}: {f}",
                        if horizontal { "H" } else { "V" }
                    ));
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn zero_value_short_spans_and_points_on_a_line() {
    let mut fails = Vec::new();
    for k in [0.378, 3.78, 37.8] {
        // zero value: same coordinate along the measured direction
        for (a, b, horizontal) in [
            ((200.0, 100.0), (200.0, 300.0), true),
            ((100.0, 200.0), (300.0, 200.0), false),
            ((200.0, 100.0), (210.0, 300.0), true),
            ((200.0, 100.0), (214.0, 300.0), true),
            ((200.0, 100.0), (230.0, 300.0), true),
            ((200.0, 100.0), (250.0, 300.0), true),
            ((100.0, 200.0), (300.0, 212.0), false),
            ((100.0, 200.0), (300.0, 240.0), false),
        ] {
            for label in [
                None,
                Some((a.0 + 5.0, a.1 - 70.0)),
                Some((b.0 - 5.0, b.1 + 70.0)),
            ] {
                for f in check(k, a, b, horizontal, label, 9) {
                    fails.push(format!(
                        "k {k} {a:?}-{b:?} {}: {f}",
                        if horizontal { "H" } else { "V" }
                    ));
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    fn any_geometry_draws_only_the_anatomy(
        zoom in 0.1f64..10.0,
        ax in -400.0f64..400.0, ay in -400.0f64..400.0,
        bx in -400.0f64..400.0, by in -400.0f64..400.0,
        horizontal in any::<bool>(),
        lx in -500.0f64..500.0, ly in -500.0f64..500.0,
        placed in any::<bool>(),
        chars in 3usize..14,
    ) {
        let k = zoom * 96.0 / 25.4;
        let label = placed.then_some((lx, ly));
        let why = check(k, (ax, ay), (bx, by), horizontal, label, chars);
        prop_assert!(why.is_empty(), "{why:?}");
    }
}

#[test]
fn oracle_negative_control_rejects_tampered_marks() {
    let k = 3.78;
    let (a, b) = ((100.0, 250.0), (480.0, 360.0));
    let item = DimensionItem {
        geometry: DimensionGeometry::Projected {
            from: doc(a, k),
            to: doc(b, k),
            along: Vec2 { x: 1.0, y: 0.0 },
        },
        label: None,
        text_chars: 12,
    };
    let good = layout_dimensions(&[item], ViewTransform::new(k, Point::new(0.0, 0.0)));
    let good = &good[0];
    assert!(
        good.lines.len() >= 3,
        "two extension lines and a dimension line"
    );
    assert!(verify(k, good, a, b, true, None, 12).is_empty());
    // a stray oblique segment
    let mut stray = good.clone();
    stray
        .lines
        .push((doc((100.0, 100.0), k), doc((300.0, 180.0), k)));
    assert!(!verify(k, &stray, a, b, true, None, 12).is_empty());
    // a stray horizontal segment far outside the hull
    let mut far = good.clone();
    far.lines
        .push((doc((900.0, 200.0), k), doc((1000.0, 200.0), k)));
    assert!(!verify(k, &far, a, b, true, None, 12).is_empty());
    // an extension line not at a measured point
    let mut off = good.clone();
    off.lines[0].0.x += 30.0 / k;
    off.lines[0].1.x += 30.0 / k;
    assert!(!verify(k, &off, a, b, true, None, 12).is_empty());
}
