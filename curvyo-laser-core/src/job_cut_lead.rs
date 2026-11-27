//! The lead-in and lead-out of one closed cut outline (`specs/0139` criterion 14, adrs L9): the
//! waste side from the nesting depth and the outline's orientation, and the Line or Arc points.

use crate::job_flatten::FLATTEN_MM;
use crate::job_input::PointUm;
use crate::operation_path_options::LeadKind;

/// A point or a direction in µm, unrounded.
pub(crate) type Xy = (f64, f64);

/// Whether the inside of the closed outline `points` lies left of its drawn direction, from the
/// exact integer shoelace sum; `None` for an outline of zero area (it gets no lead).
pub(crate) fn inside_left(points: &[PointUm]) -> Option<bool> {
    let n = points.len();
    if n < 3 {
        return None;
    }
    let twice: i128 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            i128::from(a.x) * i128::from(b.y) - i128::from(b.x) * i128::from(a.y)
        })
        .sum();
    (twice != 0).then_some(twice > 0)
}

/// The unit normal to `direction` that points to the waste side: outside an outline at even
/// `depth` (a part edge), inside one at odd depth (a hole).
pub(crate) fn waste_normal(direction: Xy, inside_left: bool, depth: usize) -> Xy {
    let left = (-direction.1, direction.0);
    let inside = if inside_left {
        left
    } else {
        (-left.0, -left.1)
    };
    if depth % 2 == 1 {
        inside
    } else {
        (-inside.0, -inside.1)
    }
}

fn add(a: Xy, b: Xy, k: f64) -> Xy {
    (a.0 + b.0 * k, a.1 + b.1 * k)
}

/// The quarter circle of radius `r` around `centre` from the offset `from` to the offset `to`
/// (both of length `r`, a quarter turn apart), as points from `from` to `to` whose chords keep
/// their sagitta within [`FLATTEN_MM`]: midpoints pushed out to the radius, so only `+ - * /`
/// and `sqrt` are used and every target writes the same bytes (adrs L9).
fn quarter(centre: Xy, from: Xy, to: Xy, r: f64) -> Vec<Xy> {
    let tolerance = FLATTEN_MM * 1000.0;
    let mut offsets = vec![from, to];
    loop {
        let (a, b) = (offsets[0], offsets[1]);
        let (dx, dy) = (a.0 - b.0, a.1 - b.1);
        let half = (dx * dx + dy * dy).sqrt() / 2.0;
        let sagitta = r - (r * r - half * half).max(0.0).sqrt();
        if sagitta <= tolerance || offsets.len() > 1 << 12 {
            break;
        }
        let mut next = Vec::with_capacity(offsets.len() * 2 - 1);
        for pair in offsets.windows(2) {
            let m = (
                f64::midpoint(pair[0].0, pair[1].0),
                f64::midpoint(pair[0].1, pair[1].1),
            );
            let norm = (m.0 * m.0 + m.1 * m.1).sqrt();
            next.push(pair[0]);
            next.push((m.0 / norm * r, m.1 / norm * r));
        }
        next.push(offsets[offsets.len() - 1]);
        offsets = next;
    }
    offsets
        .into_iter()
        .map(|o| (centre.0 + o.0, centre.1 + o.1))
        .collect()
}

/// The lead-in of `kind` and `length` µm that ends at `at`, where the path leaves in the unit
/// `direction`, on the waste side `normal`: its points from its start to `at`, empty for None.
pub(crate) fn lead_in(kind: LeadKind, length: f64, at: Xy, direction: Xy, normal: Xy) -> Vec<Xy> {
    match kind {
        LeadKind::None => Vec::new(),
        LeadKind::Line => vec![add(at, normal, length), at],
        LeadKind::Arc => {
            let centre = add(at, normal, length);
            let from = (-direction.0 * length, -direction.1 * length);
            let to = (-normal.0 * length, -normal.1 * length);
            quarter(centre, from, to, length)
        }
    }
}

/// The lead-out of `kind` and `length` µm that starts at `at`, where the path arrives in the
/// unit `direction`, on the waste side `normal`: its points from `at` on, empty for None.
pub(crate) fn lead_out(kind: LeadKind, length: f64, at: Xy, direction: Xy, normal: Xy) -> Vec<Xy> {
    match kind {
        LeadKind::None => Vec::new(),
        LeadKind::Line => vec![at, add(at, normal, length)],
        LeadKind::Arc => {
            let centre = add(at, normal, length);
            let from = (-normal.0 * length, -normal.1 * length);
            let to = (direction.0 * length, direction.1 * length);
            quarter(centre, from, to, length)
        }
    }
}

/// `points` rounded to µm, repeats removed.
pub(crate) fn rounded(points: &[Xy]) -> Vec<PointUm> {
    let mut out: Vec<PointUm> = points
        .iter()
        .map(|&(x, y)| PointUm::new(round(x), round(y)))
        .collect();
    out.dedup();
    out
}

fn round(value: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; page coordinates are far inside the range
    let um = value.round() as i64;
    um
}

#[cfg(test)]
mod tests {
    use super::*;

    fn length(points: &[Xy]) -> f64 {
        points
            .windows(2)
            .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
            .sum()
    }

    #[test]
    fn the_waste_side_is_outside_a_part_and_inside_a_hole() {
        // Drawn +x then +y: the inside is left of the direction.
        let square = [
            PointUm::new(0, 0),
            PointUm::new(40_000, 0),
            PointUm::new(40_000, 40_000),
            PointUm::new(0, 40_000),
        ];
        assert_eq!(inside_left(&square), Some(true));
        let mut back = square;
        back.reverse();
        assert_eq!(inside_left(&back), Some(false));
        assert_eq!(
            inside_left(&[PointUm::new(0, 0), PointUm::new(1, 0), PointUm::new(2, 0)]),
            None
        );
        // Along the bottom side (+x) the part's inside is +y: the waste is -y at depth 0, +y at 1.
        assert_eq!(waste_normal((1.0, 0.0), true, 0), (0.0, -1.0));
        assert_eq!(waste_normal((1.0, 0.0), true, 1), (0.0, 1.0));
        assert_eq!(waste_normal((1.0, 0.0), false, 0), (0.0, 1.0));
    }

    #[test]
    fn a_line_lead_is_perpendicular_and_an_arc_is_a_tangent_quarter_circle() {
        let line = lead_in(
            LeadKind::Line,
            2_000.0,
            (20_000.0, 0.0),
            (1.0, 0.0),
            (0.0, -1.0),
        );
        assert_eq!(line, [(20_000.0, -2_000.0), (20_000.0, 0.0)]);
        let arc = lead_in(LeadKind::Arc, 1_000.0, (0.0, 0.0), (1.0, 0.0), (0.0, -1.0));
        // From one radius before and one radius out, to the junction.
        assert!((arc[0].0 + 1_000.0).abs() < 1e-9 && (arc[0].1 + 1_000.0).abs() < 1e-9);
        assert_eq!(*arc.last().unwrap(), (0.0, 0.0));
        let centre = (0.0, -1_000.0);
        for p in &arc {
            let r = ((p.0 - centre.0).powi(2) + (p.1 - centre.1).powi(2)).sqrt();
            assert!((r - 1_000.0).abs() < 1e-6);
        }
        // Tangent at the junction: the last chord runs nearly along +x.
        let n = arc.len();
        let (dx, dy) = (arc[n - 1].0 - arc[n - 2].0, arc[n - 1].1 - arc[n - 2].1);
        assert!(dy.abs() / dx < 0.2, "{dx} {dy}");
        let quarter_length = std::f64::consts::FRAC_PI_2 * 1_000.0;
        assert!((length(&arc) - quarter_length).abs() < 5.0);
        assert!(n.is_power_of_two() || (n - 1).is_power_of_two());
        let out = lead_out(LeadKind::Arc, 1_000.0, (0.0, 0.0), (1.0, 0.0), (0.0, -1.0));
        assert_eq!(out[0], (0.0, 0.0));
        let end = *out.last().unwrap();
        assert!((end.0 - 1_000.0).abs() < 1e-9 && (end.1 + 1_000.0).abs() < 1e-9);
        assert_eq!(
            lead_out(LeadKind::None, 1.0, (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)).len(),
            0
        );
    }
}
