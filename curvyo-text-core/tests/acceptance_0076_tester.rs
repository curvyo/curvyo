//! Tester white-box case for `specs/0076-single-line-fonts` criterion 16 on the SVG font reader:
//! arcs at the largest Size stay within 0.001 mm.
#![allow(clippy::unwrap_used, missing_docs)]
#![cfg(not(target_arch = "wasm32"))]

use curvyo_text_core::{FaceStore, Mm, layout};
use kurbo::{ParamCurve, PathEl, Point};

/// Criterion 16: a circle of two arcs in a glyph, laid out at Size 2,000 mm (the largest), stays
/// within 0.001 mm of the true circle everywhere along every cubic.
#[test]
fn ac16_arcs_within_a_micrometre_at_the_largest_size() {
    let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font horiz-adv-x="800"><font-face font-family="Arc" units-per-em="1000"/>
<glyph unicode="H" d="M0 0V700M0 350H600M600 0V700"/>
<glyph unicode="O" d="M0 350A350 350 0 1 0 700 350A350 350 0 1 0 0 350"/>
</font></defs></svg>"#
        .to_vec();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    let face = store.get(&key).unwrap();
    let size = 2000.0;
    let laid = layout(face, "O", Mm(size)).unwrap();
    let paths = laid.glyph_paths(face);
    assert_eq!(paths.len(), 1);
    // H is 700 units = Size, so the radius is 350 units = Size / 2.
    let radius = size / 2.0;
    let mut start = Point::ZERO;
    let mut points = Vec::new();
    for element in paths[0].elements() {
        match *element {
            PathEl::MoveTo(p) => start = p,
            PathEl::CurveTo(a, b, p) => {
                let cubic = kurbo::CubicBez::new(start, a, b, p);
                for i in 0..=40 {
                    points.push(cubic.eval(f64::from(i) / 40.0));
                }
                start = p;
            }
            PathEl::LineTo(p) => {
                points.push(p);
                start = p;
            }
            _ => {}
        }
    }
    let (min_x, max_x) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.x), hi.max(p.x))
        });
    let (min_y, max_y) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
    let centre = Point::new(f64::midpoint(min_x, max_x), f64::midpoint(min_y, max_y));
    assert!(
        ((max_x - min_x) / 2.0 - radius).abs() < 0.001,
        "{min_x} {max_x}"
    );
    for p in &points {
        let error = (p.distance(centre) - radius).abs();
        assert!(error < 0.001, "{p:?} is {error} mm off the circle");
    }
}
