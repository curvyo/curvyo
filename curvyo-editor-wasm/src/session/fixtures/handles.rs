//! The handles of the selected fixture (`specs/0102-laser-fixtures` criteria 14, 18, 22, 23 and
//! 38; feature `adrs.md` §6a.1 and §6a.2): the rectangle's box layer per kind, with the leg
//! squircles and knobs of a stop and a screw, and the one hit test of a press on them.

use curvyo_document_core::{Angle, Page, Point};
use curvyo_laser_core::StopPart;
use curvyo_render_core::{TransformGlyphKind, TransformHandleGlyph};
use curvyo_ui_core::{
    ALL_EIGHT, EditHandle, HandleSpec, OrientedBox, ResizeDirection, TransformHandleTolerances,
    hit_transform_handle, is_drawn_handle, transform_handles,
};

use super::placed::{PlacedFixture, Shape};

/// A handle of a fixture, or what a press on its body starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FixtureHandle {
    /// A resize squircle of a holder or a clamp.
    Resize(ResizeDirection),
    /// A rotate glyph at a corner (or a side, while Shift is held).
    Rotate(ResizeDirection),
    /// The centre move handle, the outline or the inside of the selected box.
    Move,
    /// The squircle at the free end of a stop's leg 1 or leg 2.
    Leg(StopPart),
    /// A stop's thickness knob.
    Thickness,
    /// A screw's head knob.
    Head,
    /// A screw's thread knob.
    Thread,
    /// A press on a grid: nothing moves.
    Hold,
}

/// The fixture's box as the rectangle's `OrientedBox`: pivot at its frame origin, turned by its
/// matrix, its local box placed at the origin (`adrs.md` §6a.2; the matrix is rigid).
pub(super) fn fixture_box(fixture: &PlacedFixture, page: &Page) -> OrientedBox {
    let (min, max) = fixture.local_box(page);
    let origin = fixture.global.apply_point(Point::new(0.0, 0.0));
    OrientedBox {
        min: Point::new(origin.x + min.x, origin.y + min.y),
        max: Point::new(origin.x + max.x, origin.y + max.y),
        angle: Angle::from_radians(fixture.global.b.atan2(fixture.global.a)),
        pivot: origin,
    }
}

/// The point `local` of the fixture's frame in document space, through its box.
fn at(box_: &OrientedBox, local: (f64, f64)) -> Point {
    box_.to_document(Point::new(box_.pivot.x + local.0, box_.pivot.y + local.1))
}

/// The handles of the rectangle's layout that `fixture` keeps: resize (holder and clamp only),
/// rotate (not on a screw) and the centre; never skew (D22).
fn layout_handles(
    fixture: &PlacedFixture,
    box_: &OrientedBox,
    tolerances: &TransformHandleTolerances,
    side_rotate: bool,
) -> Vec<(EditHandle, Point)> {
    let resize: &'static [ResizeDirection] = match fixture.shape {
        Shape::Board { .. } | Shape::Clamp { .. } => &ALL_EIGHT,
        _ => &[],
    };
    let spec = HandleSpec {
        resize_directions: resize,
        side_rotate,
    };
    let screw = matches!(fixture.shape, Shape::Screw { .. });
    transform_handles(box_, spec, tolerances)
        .into_iter()
        .filter(|(handle, _)| match handle {
            EditHandle::Skew(_) | EditHandle::Param(_) => false,
            EditHandle::Rotate(_) => !screw,
            EditHandle::Resize(_) | EditHandle::Move => true,
        })
        .collect()
}

/// The own handles of a stop and a screw with their positions: the leg squircles (24 px rule)
/// and the knobs (72 px rule, `PARAM_MIN_SIDE_PX`).
fn own_handles(
    fixture: &PlacedFixture,
    box_: &OrientedBox,
    tolerances: &TransformHandleTolerances,
) -> Vec<(FixtureHandle, Point)> {
    let mut out = Vec::new();
    match &fixture.shape {
        Shape::Stop {
            leg_a,
            leg_b,
            thickness,
        } => {
            let (a, b, t) = (leg_a.as_mm(), leg_b.as_mm(), thickness.as_mm());
            if a >= tolerances.edge_handle_min_side_mm {
                out.push((FixtureHandle::Leg(StopPart::LegA), at(box_, (a, t / 2.0))));
            }
            if b >= tolerances.edge_handle_min_side_mm {
                out.push((FixtureHandle::Leg(StopPart::LegB), at(box_, (t / 2.0, b))));
            }
            if a.min(b) >= tolerances.param_min_side_mm {
                // On the top (outer) edge of leg 1, the edge a thickness change moves (D23).
                out.push((FixtureHandle::Thickness, at(box_, (a / 2.0, 0.0))));
            }
        }
        Shape::Screw { thread, head } if head.as_mm() >= tolerances.param_min_side_mm => {
            out.push((FixtureHandle::Head, at(box_, (head.as_mm() / 2.0, 0.0))));
            out.push((
                FixtureHandle::Thread,
                at(box_, (-thread.as_mm() / 2.0, 0.0)),
            ));
        }
        _ => {}
    }
    out
}

/// Every handle the selected `fixture` shows, in document space: the rectangle's set it keeps
/// and its own; the centre yields to a knob within 20 px (`CENTRE_YIELD_PX`). None for a grid.
pub(super) fn fixture_handles(
    fixture: &PlacedFixture,
    page: &Page,
    tolerances: &TransformHandleTolerances,
    side_rotate: bool,
) -> Vec<(FixtureHandle, Point)> {
    if !super::commands::movable(fixture) {
        return Vec::new();
    }
    let box_ = fixture_box(fixture, page);
    let own = own_handles(fixture, &box_, tolerances);
    let centre = box_.to_document(box_.local_center());
    let yields = own.iter().any(|(handle, p)| {
        !matches!(handle, FixtureHandle::Leg(_))
            && p.vector_to(centre).length() < tolerances.param_centre_yield_mm
    });
    let mut out: Vec<(FixtureHandle, Point)> =
        layout_handles(fixture, &box_, tolerances, side_rotate)
            .into_iter()
            .filter(|(handle, _)| !(yields && *handle == EditHandle::Move))
            .map(|(handle, p)| (of_edit(handle), p))
            .collect();
    out.extend(own);
    out
}

fn of_edit(handle: EditHandle) -> FixtureHandle {
    match handle {
        EditHandle::Resize(direction) => FixtureHandle::Resize(direction),
        EditHandle::Rotate(direction) => FixtureHandle::Rotate(direction),
        EditHandle::Move | EditHandle::Skew(_) | EditHandle::Param(_) => FixtureHandle::Move,
    }
}

/// The handle of the selected `fixture` a press at `p` takes, the rectangle's rule: the nearest
/// centre within its radius wins (a knob 12 px, a squircle 16 px, a rotate glyph 16 px), a knob
/// on a tie; the centre handle last, within its hover radius.
pub(super) fn hit_fixture_handle(
    fixture: &PlacedFixture,
    page: &Page,
    p: Point,
    tolerances: &TransformHandleTolerances,
    side_rotate: bool,
) -> Option<FixtureHandle> {
    let handles = fixture_handles(fixture, page, tolerances, side_rotate);
    if handles.is_empty() {
        return None;
    }
    let box_ = fixture_box(fixture, page);
    let layout: Vec<(EditHandle, Point)> = handles
        .iter()
        .filter_map(|(handle, at)| edit_of(*handle).map(|h| (h, *at)))
        .collect();
    let own = handles
        .iter()
        .filter(|(handle, _)| edit_of(*handle).is_none())
        .filter_map(|(handle, at)| {
            let radius = if matches!(handle, FixtureHandle::Leg(_)) {
                tolerances.resize.as_mm()
            } else {
                tolerances.param_hit.as_mm()
            };
            let distance = at.vector_to(p).length();
            (distance <= radius).then_some((distance, *handle))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let layout_hit = hit_transform_handle(&layout, &box_, p, tolerances, true);
    let layout_distance = layout_hit.and_then(|hit| {
        layout
            .iter()
            .find(|(h, _)| *h == hit)
            .map(|(_, at)| (hit, at.vector_to(p).length()))
    });
    match (own, layout_distance) {
        (Some((d, _)), Some((hit, layout_d))) if hit != EditHandle::Move && layout_d < d => {
            Some(of_edit(hit))
        }
        (Some((_, own)), _) => Some(own),
        (None, Some((hit, _))) => Some(of_edit(hit)),
        (None, None) => None,
    }
}

/// The rectangle's handle a fixture handle is, for the layout's own hit test.
fn edit_of(handle: FixtureHandle) -> Option<EditHandle> {
    match handle {
        FixtureHandle::Resize(direction) => Some(EditHandle::Resize(direction)),
        FixtureHandle::Rotate(direction) => Some(EditHandle::Rotate(direction)),
        FixtureHandle::Move => Some(EditHandle::Move),
        _ => None,
    }
}

/// The glyphs of the selected fixture's handles for `curvyo-render-core`: squircles for resize
/// and legs, the rotate and move glyphs, the Parameter handle look for the knobs. `active` is
/// the handle being dragged or whose chip is open.
pub(super) fn handle_glyphs(
    fixture: &PlacedFixture,
    page: &Page,
    tolerances: &TransformHandleTolerances,
    active: Option<FixtureHandle>,
    side_rotate: bool,
) -> Vec<TransformHandleGlyph> {
    let box_ = fixture_box(fixture, page);
    fixture_handles(fixture, page, tolerances, side_rotate)
        .into_iter()
        .filter(|(handle, _)| {
            edit_of(*handle).is_none_or(|edit| is_drawn_handle(edit, &box_, tolerances))
        })
        .map(|(handle, position)| TransformHandleGlyph {
            position,
            kind: match handle {
                FixtureHandle::Resize(_) | FixtureHandle::Leg(_) => TransformGlyphKind::Resize,
                FixtureHandle::Rotate(_) => TransformGlyphKind::Rotate,
                FixtureHandle::Move | FixtureHandle::Hold => TransformGlyphKind::Move,
                FixtureHandle::Thickness | FixtureHandle::Head | FixtureHandle::Thread => {
                    TransformGlyphKind::Parameter
                }
            },
            dragging: active == Some(handle),
            hovered: false,
        })
        .collect()
}
