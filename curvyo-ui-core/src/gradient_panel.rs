//! What the Style tab's Gradient block shows for the edited objects (`specs/0124-gradient-fill`
//! criteria 3 and 8, UX notes §3 to §5): the kind, the Angle or Radius, the stop rows with
//! "Mixed" where the objects differ, or the "Mixed gradients" line.

use curvyo_document_core::{Color, GradientFill, GradientKind, ObjectSnapshot, StopOffset};

use crate::style_scope::is_single_line_text;

/// Whether `object` can take a gradient fill (criterion 1): a path, a primitive, a linked clone
/// of a shape or a multi-line text.
#[must_use]
pub fn takes_gradient(object: &ObjectSnapshot) -> bool {
    match object {
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_) => true,
        ObjectSnapshot::Clone(clone) => !clone.is_picture(),
        ObjectSnapshot::Text(_) => !is_single_line_text(object),
        ObjectSnapshot::Image(_) | ObjectSnapshot::Sketch(_) | ObjectSnapshot::Dimension(_) => {
            false
        }
    }
}

/// One stop row: its colour and alpha and its offset, `None` where the objects differ.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStopRow {
    /// The colour and alpha the row shows.
    pub colour: Option<(Color, u8)>,
    /// The offset the row shows.
    pub offset: Option<StopOffset>,
}

/// The Gradient block of the edited objects.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientPanel {
    /// The kind every gradient has; `None` when they differ.
    pub kind: Option<GradientKind>,
    /// The stop rows, when every gradient has the same kind and number of stops; else `None`
    /// and the block shows "Mixed gradients".
    pub rows: Option<Vec<GradientStopRow>>,
}

/// The one value all `items` share, or `None` when they differ (or there are none).
fn common<T: PartialEq>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let first = items.next()?;
    items.all(|item| item == first).then_some(first)
}

/// The block for `gradients` (the painted gradients of the edited objects); `None` when there
/// are none.
#[must_use]
pub fn gradient_panel(gradients: &[&GradientFill]) -> Option<GradientPanel> {
    let first = gradients.first()?;
    let kind = common(gradients.iter().map(|g| g.geometry.kind()));
    let count = common(gradients.iter().map(|g| g.stops.len()));
    let rows = (kind.is_some() && count.is_some()).then(|| {
        (0..first.stops.len())
            .map(|index| GradientStopRow {
                colour: common(gradients.iter().map(|g| {
                    let stop = g.stops.as_slice()[index];
                    (stop.colour, stop.alpha)
                })),
                offset: common(gradients.iter().map(|g| g.stops.as_slice()[index].offset)),
            })
            .collect()
    });
    Some(GradientPanel { kind, rows })
}
