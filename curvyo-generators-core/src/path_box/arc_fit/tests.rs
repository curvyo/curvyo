//! Tests of `arc_fit.rs`.

use super::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn end_point(shape: &Shape) -> Point {
    match *shape {
        Shape::Straight { to, .. } => to,
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => crate::outline::polar(centre, radius, from + sweep),
    }
}

fn start_point(shape: &Shape) -> Point {
    match *shape {
        Shape::Straight { from, .. } => from,
        Shape::Bend {
            centre,
            radius,
            from,
            ..
        } => crate::outline::polar(centre, radius, from),
    }
}

/// A lobe of the heart: bends one way by about 130 degrees.
fn lobe() -> (Point, Cubic) {
    (
        p(0.0, 40.0),
        Cubic {
            c1: p(-10.0, 25.0),
            c2: p(-40.0, 10.0),
            to: p(-40.0, -10.0),
        },
    )
}

fn tangent_of(shape: &Shape, at_end: bool) -> V {
    match *shape {
        Shape::Straight { from, to } => {
            let d = sub(v(to), v(from));
            scale(d, 1.0 / len(d))
        }
        Shape::Bend { from, sweep, .. } => {
            let angle = if at_end { from + sweep } else { from };
            let s = sweep.signum();
            (-s * libm::sin(angle), s * libm::cos(angle))
        }
    }
}

#[test]
fn the_arcs_stay_within_001_mm_of_the_cubic_both_ways() {
    let (from, cubic) = lobe();
    let shapes = arcs(from, &cubic, (None, None));
    let bez = Bez::of(from, &cubic);
    let dense: Vec<V> = (0..=2000).map(|k| bez.at(f64::from(k) / 2000.0)).collect();
    for k in 0..=64 {
        let p = bez.at(f64::from(k) / 64.0);
        let d = shapes
            .iter()
            .map(|s| distance_to(s, p))
            .fold(f64::MAX, f64::min);
        assert!(d <= FIT_TOLERANCE_MM, "cubic to arcs {d}");
    }
    for shape in &shapes {
        for k in 0..=64 {
            let along = f64::from(k) / 64.0;
            let q = match *shape {
                Shape::Straight { from, to } => lerp(v(from), v(to), along),
                Shape::Bend {
                    centre,
                    radius,
                    from,
                    sweep,
                } => v(crate::outline::polar(centre, radius, from + sweep * along)),
            };
            let d = dense
                .windows(2)
                .map(|c| {
                    let chord = Shape::Straight {
                        from: pt(c[0]),
                        to: pt(c[1]),
                    };
                    distance_to(&chord, q)
                })
                .fold(f64::MAX, f64::min);
            assert!(d <= FIT_TOLERANCE_MM, "arcs to cubic {d}");
        }
    }
}

#[test]
fn the_arcs_join_end_to_end_and_tangent_within_1e_9() {
    let (from, cubic) = lobe();
    let shapes = arcs(from, &cubic, (None, None));
    assert!(shapes.len() >= 2);
    let first = start_point(&shapes[0]);
    assert!(len(sub(v(first), v(from))) < 1e-9);
    let last = end_point(&shapes[shapes.len() - 1]);
    assert!(len(sub(v(last), v(cubic.to))) < 1e-9);
    for pair in shapes.windows(2) {
        assert!(len(sub(v(end_point(&pair[0])), v(start_point(&pair[1])))) < 1e-9);
        let (a, b) = (tangent_of(&pair[0], true), tangent_of(&pair[1], false));
        assert!(turn_between(a, b).abs() < 1e-9, "tangent join");
    }
}

#[test]
fn a_forced_direction_half_a_degree_off_makes_the_join_exact_and_still_fits() {
    let (from, cubic) = lobe();
    let own = Bez::of(from, &cubic).tangent(true);
    let (sin, cos) = (
        libm::sin(0.4_f64.to_radians()),
        libm::cos(0.4_f64.to_radians()),
    );
    let want = (own.0 * cos - own.1 * sin, own.0 * sin + own.1 * cos);
    let shapes = arcs(from, &cubic, (None, Some(want)));
    let last = tangent_of(&shapes[shapes.len() - 1], true);
    assert!(turn_between(last, want).abs() < 1e-9);
    let bez = Bez::of(from, &cubic);
    for k in 0..=64 {
        let p = bez.at(f64::from(k) / 64.0);
        let d = shapes
            .iter()
            .map(|s| distance_to(s, p))
            .fold(f64::MAX, f64::min);
        assert!(d <= FIT_TOLERANCE_MM, "{d}");
    }
}

#[test]
fn an_s_shaped_cubic_is_split_at_its_inflection() {
    let from = p(0.0, 0.0);
    let cubic = Cubic {
        c1: p(20.0, 0.0),
        c2: p(-10.0, 20.0),
        to: p(10.0, 20.0),
    };
    let shapes = arcs(from, &cubic, (None, None));
    let signs: Vec<f64> = shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Bend { sweep, .. } => Some(sweep.signum()),
            Shape::Straight { .. } => None,
        })
        .collect();
    assert!(signs.contains(&1.0) && signs.contains(&-1.0));
}

#[test]
fn a_cusp_is_found_where_the_derivative_vanishes() {
    let from = p(0.0, 0.0);
    // B' vanishes at t = 0.5 for this symmetric loop.
    let cubic = Cubic {
        c1: p(10.0, 10.0),
        c2: p(0.0, 10.0),
        to: p(10.0, 0.0),
    };
    let found = cusps(from, &cubic);
    assert_eq!(found.len(), 1);
    assert!((found[0] - 0.5).abs() < 1e-9);
    let smooth = Cubic {
        c1: p(3.0, 5.0),
        c2: p(7.0, 5.0),
        to: p(10.0, 0.0),
    };
    assert_eq!(cusps(from, &smooth), Vec::<f64>::new());
}
