//! The `curvyo:` shadow data of an imported leaf, kept only where the standard SVG next to it
//! still agrees (`specs/0024-svg-import-export` criteria 57, 66, 140, 142 and 149): exact stroke
//! width and dash list, the stored markers, and the primitive a `<path>` or Inkscape star stands
//! for. Otherwise the standard SVG wins and the shadow value is dropped.

use curvyo_document_core::{
    Angle, CornerRadii, Curve, DashPattern, EllipseFrame, InnerRatio, Length, MarkerCount,
    MarkerPlace, Markers, Point, PointCount, RectBounds, Shape, StarFrame, Style,
};
use curvyo_svg_core::{LeafKind, MarkerShape, ShadowKind, ShadowMarkers, SvgLeaf};
use kurbo::{Affine, BezPath};

use crate::svg_import_match::draws;

/// How far an exact length may be from the standard one and still be kept (criterion 57).
pub(crate) const LENGTH_AGREEMENT_MM: f64 = 1e-6;

/// Applies the agreeing shadow values of `leaf` to `style` (criteria 57, 142).
pub(crate) fn apply_shadow_style(leaf: &SvgLeaf, style: &mut Style) {
    let shadow = &leaf.shadow;
    let standard_width = leaf.style.stroke_width_mm;
    if let (Some(exact), Some(standard)) = (shadow.stroke_width, standard_width)
        && exact > 0.0
        && (exact - standard).abs() <= LENGTH_AGREEMENT_MM
    {
        style.stroke.width = Length::from_mm(exact);
    }
    let width = style.stroke.width.as_mm();
    let standard = standard_width.unwrap_or(width);
    if let Some(exact) = &shadow.dash
        && exact.len() == leaf.style.dash.len()
        && exact
            .iter()
            .zip(&leaf.style.dash)
            .all(|(a, b)| (a * width - b * standard).abs() <= LENGTH_AGREEMENT_MM)
        && let Ok(dash) = DashPattern::new(exact.clone())
    {
        style.stroke.dash = dash;
    }
    if let Some(markers) = shadow.markers
        && leaf.style.stroke.is_some()
    {
        style.stroke.markers = agreeing_markers(markers, leaf);
    }
}

fn shape_of(shape: Option<MarkerShape>) -> curvyo_document_core::MarkerShape {
    match shape {
        None => curvyo_document_core::MarkerShape::None,
        Some(MarkerShape::Arrow) => curvyo_document_core::MarkerShape::Arrow,
        Some(MarkerShape::Dot) => curvyo_document_core::MarkerShape::Dot,
    }
}

/// The stored markers with every slot that SVG draws (criterion 141 (b)) and whose `marker-*`
/// reference is gone set to none (criterion 142).
fn agreeing_markers(markers: ShadowMarkers, leaf: &SvgLeaf) -> Markers {
    let representable = match &leaf.kind {
        LeafKind::Path { path, .. } => single_open_outline(path),
        _ => false,
    };
    let [start_ref, mid_ref, end_ref] = leaf.marker_refs;
    let keep = |shape: Option<MarkerShape>, present: bool| {
        if representable && !present {
            None
        } else {
            shape
        }
    };
    let mid_drawn_by_svg = markers.at_nodes;
    Markers {
        start: shape_of(keep(markers.start, start_ref)),
        mid: shape_of(if mid_drawn_by_svg {
            keep(markers.middle, mid_ref)
        } else {
            markers.middle
        }),
        end: shape_of(keep(markers.end, end_ref)),
        mid_place: if markers.at_nodes {
            MarkerPlace::AtNodes
        } else {
            MarkerPlace::Spaced
        },
        mid_count: MarkerCount::new(markers.count).unwrap_or_default(),
    }
}

/// Whether `path` is one open outline of two or more nodes (criterion 141 (b)).
pub(crate) fn single_open_outline(path: &BezPath) -> bool {
    let elements = path.elements();
    let moves = elements
        .iter()
        .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
        .count();
    let closed = elements
        .iter()
        .any(|e| matches!(e, kurbo::PathEl::ClosePath));
    moves == 1 && !closed && elements.len() >= 2
}

/// The primitive the shadow data of `leaf` names, after `placement`, when its outline draws the
/// leaf's `d` within 0.001 mm (criteria 57, 140); `None` otherwise.
pub(crate) fn shadow_primitive(leaf: &SvgLeaf, placement: Affine) -> Option<(Shape, Angle)> {
    let drawn = match &leaf.kind {
        LeafKind::Path { path, .. } | LeafKind::Star { path, .. } => placement * path.clone(),
        LeafKind::Arc { fine, .. } => placement * fine.clone()?,
        other => placement * other.outline_mm()?,
    };
    let kind = match (&leaf.kind, leaf.shadow.kind) {
        (_, Some(kind)) => kind,
        (LeafKind::Star { star, .. }, None) if star.flat => ShadowKind::Polygon,
        (LeafKind::Star { .. }, None) => ShadowKind::Star,
        (LeafKind::Rect { .. }, None) => ShadowKind::Rectangle,
        (LeafKind::Ellipse { .. }, None) => ShadowKind::Ellipse,
        (LeafKind::Arc { arc, .. }, None) => {
            let frame = leaf.shadow.frame.as_deref()?;
            return arc_by_frame(arc, frame, placement, &drawn);
        }
        (_, None) => return None,
    };
    let frame = leaf.shadow.frame.as_deref()?;
    let shift = placement.translation();
    let center = |x: f64, y: f64| Point::new(x + shift.x, y + shift.y);
    let curve = |value: f64| Curve::new(value);
    let (shape, rotation) = match (kind, frame) {
        (ShadowKind::Rectangle, &[cx, cy, w, h, rotation]) => {
            let [tl, tr, br, bl] = match (&leaf.kind, leaf.shadow.radii) {
                // A `<rect>` draws one radius: the exact radii only when all four are that one
                // (ADR 0018 point 9).
                (LeafKind::Rect { rx, matrix, .. }, radii) => {
                    let drawn = rx * matrix.determinant().abs().sqrt();
                    match radii {
                        Some(exact)
                            if exact.iter().all(|r| {
                                (r - drawn).abs() <= crate::svg_import_match::MATCH_MM
                            }) =>
                        {
                            exact
                        }
                        _ => [drawn; 4],
                    }
                }
                (_, Some(radii)) => radii,
                _ => return None,
            };
            let corner_radii = CornerRadii {
                tl: Length::from_mm(tl),
                tr: Length::from_mm(tr),
                br: Length::from_mm(br),
                bl: Length::from_mm(bl),
            };
            let c = center(cx, cy);
            let bounds = RectBounds {
                origin: Point::new(c.x - w / 2.0, c.y - h / 2.0),
                width: Length::from_mm(w),
                height: Length::from_mm(h),
            };
            (
                Shape::Rect {
                    bounds,
                    corner_radii,
                },
                rotation,
            )
        }
        (ShadowKind::Ellipse, &[cx, cy, rx, ry, rotation, k]) => (
            Shape::Ellipse {
                frame: EllipseFrame {
                    center: center(cx, cy),
                    rx: Length::from_mm(rx),
                    ry: Length::from_mm(ry),
                },
                arc: None,
                curve: curve(k)?,
            },
            rotation,
        ),
        (ShadowKind::Polygon, &[cx, cy, radius, sides, angle, rotation, k]) => (
            Shape::Polygon {
                frame: star_frame(center(cx, cy), radius, angle),
                point_count: count(sides)?,
                curve: curve(k)?,
            },
            rotation,
        ),
        (ShadowKind::Star, &[cx, cy, radius, points, angle, rotation, k, ratio]) => (
            Shape::Star {
                frame: star_frame(center(cx, cy), radius, angle),
                point_count: count(points)?,
                inner_ratio: InnerRatio::new(ratio).ok()?,
                curve: curve(k)?,
            },
            rotation,
        ),
        _ => return None,
    };
    let finite = frame.iter().all(|v| v.is_finite() && v.abs() <= 1e7);
    let rotation = Angle::from_radians(rotation);
    let agrees = match &leaf.kind {
        // A `<rect>` draws true circular corners, the model's outline cubic ones: its numbers are
        // compared instead (criterion 57: geometry within 1e-3 mm).
        LeafKind::Rect { rect, matrix, .. } => {
            rect_agrees(&shape, rotation, *rect, placement * *matrix)
        }
        _ => draws(&shape, rotation, &drawn),
    };
    (finite && agrees).then_some((shape, rotation))
}

/// Whether the rectangle `shape` turned by `rotation` is the `<rect>` `rect` under `m` within
/// 0.001 mm and 1e-6 rad.
fn rect_agrees(shape: &Shape, rotation: Angle, rect: kurbo::Rect, m: Affine) -> bool {
    let Shape::Rect { bounds, .. } = shape else {
        return false;
    };
    let [a, b, _, _, _, _] = m.as_coeffs();
    let scale = m.determinant().abs().sqrt();
    let c = m * rect.center();
    let near = |x: f64, y: f64| (x - y).abs() <= crate::svg_import_match::MATCH_MM;
    let turn = (b.atan2(a) - rotation.as_radians()).rem_euclid(std::f64::consts::TAU);
    let turn_ok = turn.min(std::f64::consts::TAU - turn) <= 1e-6;
    near(c.x, bounds.origin.x + bounds.width.as_mm() / 2.0)
        && near(c.y, bounds.origin.y + bounds.height.as_mm() / 2.0)
        && near(rect.width() * scale, bounds.width.as_mm())
        && near(rect.height() * scale, bounds.height.as_mm())
        && turn_ok
        && m.determinant() > 0.0
}

/// An arc by its frame `cx cy rx ry rotation start end` (radians), its kind from the Inkscape
/// data, when it draws `drawn` within 0.001 mm.
fn arc_by_frame(
    arc: &curvyo_svg_core::ArcData,
    frame: &[f64],
    placement: Affine,
    drawn: &BezPath,
) -> Option<(Shape, Angle)> {
    let &[cx, cy, rx, ry, rotation, start, end] = frame else {
        return None;
    };
    let kind = match (arc.arc_type.as_deref(), arc.open) {
        (Some("slice"), _) | (None, false) => curvyo_document_core::ArcKind::Pie,
        (Some("arc"), _) | (None, true) => curvyo_document_core::ArcKind::Arc,
        (Some("chord"), _) => curvyo_document_core::ArcKind::Chord,
        _ => return None,
    };
    if !frame.iter().all(|v| v.is_finite() && v.abs() <= 1e7) {
        return None;
    }
    let shift = placement.translation();
    let shape = Shape::Ellipse {
        frame: EllipseFrame {
            center: Point::new(cx + shift.x, cy + shift.y),
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        },
        arc: Some(curvyo_document_core::EllipseArc::new(
            Angle::from_radians(start),
            Angle::from_radians(end),
            kind,
        )?),
        curve: Curve::ELLIPSE_DEFAULT,
    };
    let rotation = Angle::from_radians(rotation);
    draws(&shape, rotation, drawn).then_some((shape, rotation))
}

fn star_frame(center: Point, radius: f64, angle: f64) -> StarFrame {
    StarFrame {
        center,
        radius: Length::from_mm(radius),
        angle: Angle::from_radians(angle),
    }
}

/// A point count written as a number.
fn count(value: f64) -> Option<PointCount> {
    if value.fract() != 0.0 || !(0.0..=f64::from(u32::MAX)).contains(&value) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked just above
    PointCount::new(value as u32).ok()
}
