//! 0066 AC49: a horizontal or vertical distance draws exactly its two extension lines, its
//! dimension line (with the leader to a text set beside it) and nothing else, in every case.
use super::*;

/// Endpoint tolerance of the oracle, screen pixels.
const SLACK_PX: f64 = 0.5;
/// Text box half size plus its gap, as `sketch_dimension_text` estimates it (12px text).
fn text_clear_px(chars: usize) -> (f64, f64) {
    if chars == 0 {
        return (0.0, 0.0);
    }
    #[allow(clippy::cast_precision_loss)]
    let width = 7.0 * chars as f64 + 8.0;
    (width / 2.0 + 4.0, 8.0 + 4.0)
}

/// One case: the measured points and direction, the stored label, the text length and zoom.
struct Case {
    from: Point,
    to: Point,
    along: Vec2,
    label: Option<Point>,
    chars: usize,
    scale: f64,
}

/// Checks the drawn segments of `case` against the AC49 anatomy; the error names the breach.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // One oracle, read top to bottom against the criterion; s, h, u, n are its frame.
fn oracle(case: &Case) -> Result<(), String> {
    let item = DimensionItem {
        geometry: DimensionGeometry::Projected {
            from: case.from,
            to: case.to,
            along: case.along,
        },
        label: case.label,
        text_chars: case.chars,
    };
    let marks = &layout_dimensions(
        &[item],
        ViewTransform::new(case.scale, Point::new(0.0, 0.0)),
    )[0];
    let u = case.along.normalized_to(1.0);
    let n = Vec2::new(-u.y, u.x);
    let s = |p: Point| (p.x * u.x + p.y * u.y) * case.scale;
    let h = |p: Point| (p.x * n.x + p.y * n.y) * case.scale;
    let (w, t) = text_clear_px(case.chars);
    let along_clear = u.x.abs() * w + u.y.abs() * t;
    let level = h(marks.text);
    let ts = s(marks.text);
    let ends = (s(case.from).min(s(case.to)), s(case.from).max(s(case.to)));
    let span = ends.1 - ends.0;
    // The text never lies on an extension line.
    if case.chars > 0 {
        for base in [case.from, case.to] {
            if (ts - s(base)).abs() < along_clear - SLACK_PX {
                return Err(format!("text box over the extension line at {}", s(base)));
            }
        }
    }
    // A span that holds the arrows and the text keeps the text between its extension lines.
    if span >= 56.0 && span >= 2.0 * (along_clear + 8.0) && (ts < ends.0 || ts > ends.1) {
        return Err(format!(
            "text at {ts:.2} outside {:.2}..{:.2}",
            ends.0, ends.1
        ));
    }
    let mut extension = [0, 0];
    let mut pieces = Vec::new();
    for &(p, q) in &marks.lines {
        let (sp, sq, hp, hq) = (s(p), s(q), h(p), h(q));
        if (sp - sq).abs() <= SLACK_PX {
            // An extension line: from 4px off its own point to 4px past the dimension line.
            let Some(i) = [case.from, case.to]
                .iter()
                .position(|b| (s(*b) - sp).abs() <= SLACK_PX && (h(*b) - level).abs() > 4.0)
                .filter(|i| extension[*i] == 0 || (s(case.from) - s(case.to)).abs() <= SLACK_PX)
            else {
                if (hp - level).abs() <= SLACK_PX && (hq - level).abs() <= SLACK_PX {
                    pieces.push((sp.min(sq), sp.max(sq)));
                    continue;
                }
                if span < 24.0 {
                    continue; // a tick, checked below
                }
                return Err(format!("stray segment across the line at s={sp:.2}"));
            };
            let i = if extension[i] == 0 { i } else { 1 - i };
            extension[i] += 1;
            let base = if i == 0 { case.from } else { case.to };
            let side = (level - h(base)).signum();
            let (near, far) = if (hp - h(base)).abs() < (hq - h(base)).abs() {
                (hp, hq)
            } else {
                (hq, hp)
            };
            if (near - (h(base) + 4.0 * side)).abs() > SLACK_PX
                || (far - (level + 4.0 * side)).abs() > SLACK_PX
            {
                return Err(format!(
                    "extension line {i} runs {near:.2}..{far:.2}, level {level:.2}"
                ));
            }
        } else if (hp - level).abs() <= SLACK_PX && (hq - level).abs() <= SLACK_PX {
            pieces.push((sp.min(sq), sp.max(sq)));
        } else if span < 24.0 {
            let tip = |x: f64, y: f64| {
                [s(case.from), s(case.to)]
                    .iter()
                    .any(|e| (x - e).abs() <= 3.0 + SLACK_PX && (y - level).abs() <= 3.0 + SLACK_PX)
            };
            if !(tip(sp, hp) && tip(sq, hq)) {
                return Err(format!(
                    "stray slanted segment {sp:.2},{hp:.2} {sq:.2},{hq:.2}"
                ));
            }
        } else {
            return Err(format!("stray segment {sp:.2},{hp:.2} {sq:.2},{hq:.2}"));
        }
    }
    for (i, base) in [case.from, case.to].iter().enumerate() {
        let wanted = usize::from((h(*base) - level).abs() > 4.0);
        let same = (s(case.from) - s(case.to)).abs() <= SLACK_PX;
        if extension[i] != wanted && !(same && extension[0] + extension[1] == 2 * wanted) {
            return Err(format!(
                "extension line {i}: {} drawn, {wanted} wanted",
                extension[i]
            ));
        }
    }
    // The dimension line: between the extension lines, a 12px tail on a small span, and the
    // leader up to a text set beside it; never under the text.
    let tail = if span < 56.0 { 12.0 } else { 0.0 };
    let lo = (ends.0 - tail).min(ts - along_clear) - SLACK_PX;
    let hi = (ends.1 + tail).max(ts + along_clear) + SLACK_PX;
    if pieces.is_empty() || pieces.len() > 2 {
        return Err(format!("{} dimension line pieces", pieces.len()));
    }
    for &(a, b) in &pieces {
        if a < lo || b > hi {
            return Err(format!(
                "dimension line {a:.2}..{b:.2} outside {lo:.2}..{hi:.2}"
            ));
        }
        if case.chars > 0 && a < ts + along_clear - SLACK_PX && b > ts - along_clear + SLACK_PX {
            return Err(format!(
                "dimension line {a:.2}..{b:.2} under the text at {ts:.2}"
            ));
        }
    }
    // Covered from end to end except for the text box.
    pieces.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut reach = pieces[0].0;
    if reach > ends.0 - tail + SLACK_PX && reach > ts + along_clear {
        return Err(format!(
            "dimension line starts at {reach:.2}, not at {:.2}",
            ends.0
        ));
    }
    for &(a, b) in &pieces {
        if a > reach + SLACK_PX
            && !(reach >= ts - along_clear - SLACK_PX && a <= ts + along_clear + SLACK_PX)
        {
            return Err(format!(
                "dimension line breaks at {reach:.2}..{a:.2} away from the text"
            ));
        }
        reach = reach.max(b);
    }
    Ok(())
}

fn mm(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// The customer's quadrilateral (screenshot `customer-glitch.png`), origin inside, at 100 %.
const PX_PER_MM: f64 = 188.6 / 50.0;
fn customer() -> [(Point, Point, Vec2); 3] {
    let (top, right, bottom, left) = (
        mm(-2.4, -63.6),
        mm(57.5, 27.0),
        mm(-2.6, 63.6),
        mm(-46.9, -41.4),
    );
    [
        (left, right, Vec2::new(1.0, 0.0)),
        // A sketch Y axis pointing up on the page, and pointing down.
        (bottom, top, Vec2::new(0.0, -1.0)),
        (top, bottom, Vec2::new(0.0, 1.0)),
    ]
}

#[test]
fn the_customer_axis_distances_draw_only_their_own_marks() {
    for (from, to, along) in customer() {
        for chars in [11, 0] {
            let case = Case {
                from,
                to,
                along,
                label: None,
                chars,
                scale: PX_PER_MM,
            };
            oracle(&case).unwrap_or_else(|e| panic!("{from:?} {to:?} chars {chars}: {e}"));
        }
    }
}

/// Screenshot `customer-symmetry.png`: the text stored far beyond the right point after the
/// geometry moved stays between the extension lines, without a stray line out to it.
#[test]
fn a_stored_text_beyond_an_extension_line_comes_back_between_them() {
    for (from, to, along) in customer() {
        let beyond = to.translated(along.normalized_to(1.0).scaled(100.0));
        let before = from.translated(along.normalized_to(1.0).scaled(-60.0));
        for label in [beyond, before] {
            let case = Case {
                from,
                to,
                along,
                label: Some(label),
                chars: 11,
                scale: PX_PER_MM,
            };
            oracle(&case).unwrap_or_else(|e| panic!("{label:?}: {e}"));
        }
    }
}

#[test]
fn every_case_draws_only_its_own_marks_from_ten_to_a_thousand_percent() {
    let x = Vec2::new(1.0, 0.0);
    let y = Vec2::new(0.0, -1.0);
    let cases: Vec<(Point, Point, Vec2, Option<Point>)> = vec![
        // Wide span, the line on either side and between the points.
        (mm(-40.0, 10.0), mm(50.0, -20.0), x, None),
        (mm(-40.0, 10.0), mm(50.0, -20.0), x, Some(mm(0.0, 40.0))),
        (mm(-40.0, 10.0), mm(50.0, -20.0), x, Some(mm(5.0, -5.0))),
        (mm(-40.0, 10.0), mm(50.0, -20.0), y, None),
        (mm(-40.0, 10.0), mm(50.0, -20.0), y, Some(mm(80.0, 0.0))),
        // A point on an axis, the origin itself, the span across the origin.
        (mm(0.0, 0.0), mm(30.0, 12.0), x, None),
        (mm(0.0, -25.0), mm(0.0, 25.0), y, None),
        // Short spans: text beside, arrows flipped, ticks; a zero value.
        (mm(0.0, 0.0), mm(12.0, 5.0), x, None),
        (mm(0.0, 0.0), mm(5.0, 5.0), x, None),
        (mm(0.0, 0.0), mm(1.0, 5.0), x, None),
        (mm(0.0, 0.0), mm(0.0, 20.0), x, None),
        (mm(0.0, 0.0), mm(12.0, 5.0), x, Some(mm(-30.0, -20.0))),
    ];
    for scale in [0.1, 0.25, 1.0, 4.0, 10.0].map(|z| z * PX_PER_MM) {
        for &(from, to, along, label) in &cases {
            for chars in [5, 11, 0] {
                let case = Case {
                    from,
                    to,
                    along,
                    label,
                    chars,
                    scale,
                };
                oracle(&case).unwrap_or_else(|e| {
                    panic!(
                        "scale {scale:.3} {from:?}->{to:?} {along:?} {label:?} chars {chars}: {e}"
                    )
                });
            }
        }
    }
}
