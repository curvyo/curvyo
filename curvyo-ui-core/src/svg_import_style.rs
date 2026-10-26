//! An imported leaf's style and fill rule (`specs/0024-svg-import-export` criteria 37 to 41, 50
//! and 51): the reader's resolved style as a model style, and an even-odd fill re-wound by depth
//! so the nonzero fill of the compound path looks the same, where that is possible.

use curvyo_document_core::{
    Color, DashPattern, DrawingAnchor, DrawingOutline, Fill, Length, LineCap, LineJoin, Opacity,
    Stroke, Style,
};
use curvyo_geometry_core::{
    Outline, OutlineTriple, outline_area_mm2, outline_has_area, outline_nesting, touching_outlines,
};
use curvyo_svg_core::{Cap, Join, LeafStyle, Paint};

/// The model style of a leaf: the file's stroke width, colour, caps and joins kept also when the
/// stroke is off, the application defaults where the file states none (criterion 38).
pub(crate) fn model_style(style: &LeafStyle) -> Style {
    let defaults = Stroke::default();
    let colour = |paint: Option<Paint>| {
        paint.map_or(Color::BLACK, |p| Color {
            r: p.rgb[0],
            g: p.rgb[1],
            b: p.rgb[2],
        })
    };
    let opacity = |paint: Option<Paint>| {
        paint
            .and_then(|p| Opacity::new(p.opacity).ok())
            .unwrap_or(Opacity::OPAQUE)
    };
    Style {
        fill: Fill {
            enabled: style.fill.is_some(),
            color: colour(style.fill),
            opacity: opacity(style.fill),
        },
        stroke: Stroke {
            enabled: style.stroke.is_some(),
            width: style
                .stroke_width_mm
                .filter(|w| w.is_finite() && *w > 0.0)
                .map_or(defaults.width, Length::from_mm),
            color: style.stroke.map_or(defaults.color, |p| colour(Some(p))),
            opacity: opacity(style.stroke),
            dash: DashPattern::new(style.dash.clone()).unwrap_or_default(),
            join: style.join.map_or(defaults.join, |join| match join {
                Join::Miter => LineJoin::Miter,
                Join::Round => LineJoin::Round,
                Join::Bevel => LineJoin::Bevel,
            }),
            cap: style.cap.map_or(defaults.cap, |cap| match cap {
                Cap::Butt => LineCap::Butt,
                Cap::Round => LineCap::Round,
                Cap::Square => LineCap::Square,
            }),
            markers: defaults.markers,
        },
    }
}

fn triples(anchors: &[DrawingAnchor]) -> Vec<OutlineTriple> {
    anchors
        .iter()
        .map(|a| (a.point, a.handle_in, a.handle_out))
        .collect()
}

/// The outline walked the other way: the node order reversed and each node's handles swapped
/// (the reversal of Combine, `0054`).
fn reversed(anchors: &[DrawingAnchor]) -> Vec<DrawingAnchor> {
    anchors
        .iter()
        .rev()
        .map(|a| DrawingAnchor {
            handle_in: a.handle_out,
            handle_out: a.handle_in,
            ..*a
        })
        .collect()
}

/// EVENODD A (criteria 50 and 51): with a fill that is on, closed outlines only (open outlines
/// without area take no part) and no two outlines touching or crossing, each outline gets the
/// direction that makes the nonzero fill equal to the even-odd fill: depth 0 keeps its
/// direction, an even depth takes the direction of its outermost container, an odd depth the
/// opposite one. Otherwise the outlines stay as written.
pub(crate) fn rewind_even_odd(outlines: &mut [DrawingOutline]) {
    let owned: Vec<Vec<OutlineTriple>> = outlines.iter().map(|o| triples(&o.anchors)).collect();
    let all: Vec<Outline> = owned
        .iter()
        .zip(outlines.iter())
        .map(|(t, o)| Outline::new(t, o.closed))
        .collect();
    let open_with_area = outlines
        .iter()
        .zip(&all)
        .any(|(o, outline)| !o.closed && outline_has_area(outline));
    let closed: Vec<usize> = (0..outlines.len())
        .filter(|&i| outlines[i].closed)
        .collect();
    if closed.len() < 2 || open_with_area {
        return;
    }
    let shapes: Vec<Outline> = closed.iter().map(|&i| all[i]).collect();
    if !touching_outlines(&shapes).is_empty() {
        return;
    }
    let nesting = outline_nesting(&shapes);
    let positive: Vec<bool> = shapes.iter().map(|s| outline_area_mm2(s) > 0.0).collect();
    let mut reverse = vec![false; shapes.len()];
    for (index, place) in nesting.iter().enumerate() {
        if place.depth == 0 {
            continue;
        }
        let mut outermost = index;
        while let Some(parent) = nesting[outermost].parent {
            outermost = parent;
        }
        let wanted = if place.depth % 2 == 0 {
            positive[outermost]
        } else {
            !positive[outermost]
        };
        reverse[index] = positive[index] != wanted;
    }
    drop(all);
    for (slot, &index) in closed.iter().enumerate() {
        if reverse[slot] {
            outlines[index].anchors = reversed(&outlines[index].anchors);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorKind, Point, Vec2};

    fn square(x: f64, side: f64, clockwise: bool) -> DrawingOutline {
        let corner = |px: f64, py: f64| DrawingAnchor {
            point: Point::new(px, py),
            handle_in: Vec2::ZERO,
            handle_out: Vec2::ZERO,
            kind: AnchorKind::Corner,
        };
        let mut anchors = vec![
            corner(x, x),
            corner(x + side, x),
            corner(x + side, x + side),
            corner(x, x + side),
        ];
        if !clockwise {
            anchors.reverse();
        }
        DrawingOutline {
            closed: true,
            anchors,
        }
    }

    fn signs(outlines: &[DrawingOutline]) -> Vec<bool> {
        outlines
            .iter()
            .map(|o| {
                let t = triples(&o.anchors);
                outline_area_mm2(&Outline::new(&t, true)) > 0.0
            })
            .collect()
    }

    #[test]
    fn ac50_nested_rings_alternate() {
        let mut rings = vec![
            square(0.0, 40.0, true),
            square(10.0, 20.0, true),
            square(15.0, 10.0, true),
        ];
        rewind_even_odd(&mut rings);
        let s = signs(&rings);
        assert_eq!(s, [s[0], !s[0], s[0]]);
    }

    #[test]
    fn ac51_crossing_outlines_stay() {
        let mut crossing = vec![square(0.0, 20.0, true), square(10.0, 20.0, true)];
        let before = crossing.clone();
        rewind_even_odd(&mut crossing);
        assert_eq!(crossing, before);
    }
}
