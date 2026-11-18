//! What the host reads of the gradient (`specs/0124-gradient-fill` criteria 3, 6 to 8): the
//! Style tab's Gradient block and the canvas handles with their readout and entry box, each one
//! JSON text whose shape the shared fixtures `tests/fixtures/gradient_view/` pin.

use curvyo_document_core::{GradientKind, ObjectSnapshot};
use curvyo_ui_core::{GradientHandle, gradient_panel, takes_gradient};
use serde::Serialize;

use super::super::Session;
use super::Shaded;

/// One stop row of the block.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientStopJson {
    /// `#RRGGBBAA`, empty when the shapes differ.
    pub hex: String,
    /// The position in percent ("50", "33.3"), empty when the shapes differ.
    pub position: String,
    /// The first or last row: the position is fixed text and there is no Remove.
    pub fixed: bool,
}

/// The Gradient block of the Style tab.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[allow(clippy::struct_excessive_bools)] // independent shown flags, read by name
#[serde(rename_all = "camelCase")]
pub struct GradientViewJson {
    /// Some edited object can take a gradient: the Gradient item is in the Fill switch.
    pub offered: bool,
    /// Every edited object paints a gradient: the block is shown.
    pub shown: bool,
    /// `"linear"`, `"radial"` or `"mixed"`; empty when the block is not shown.
    pub kind: String,
    /// The Angle in degrees, one decimal; empty when mixed or not every gradient is Linear.
    pub angle: String,
    /// The Angle field is shown (every gradient is Linear).
    pub angle_shown: bool,
    /// The Radius in millimetres, up to three decimals; empty when mixed.
    pub radius: String,
    /// The Radius field is shown (every gradient is Radial).
    pub radius_shown: bool,
    /// The stop rows, first to last; empty with "Mixed gradients".
    pub stops: Vec<GradientStopJson>,
    /// The kinds or stop counts differ: the line "Mixed gradients" and Reverse alone.
    pub mixed_gradients: bool,
    /// The active stop's row.
    pub active_stop: usize,
    /// The skip line of a selection with objects that cannot have a gradient.
    pub skip: String,
}

/// One handle on the canvas, in screen pixels.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientHandleJson {
    /// `"start"`, `"end"`, `"centre"`, `"radius"` or `"stop"`.
    pub kind: &'static str,
    /// The index of its stop.
    pub stop: usize,
    /// Screen x.
    pub x: f64,
    /// Screen y.
    pub y: f64,
    /// Its stop's colour, `#RRGGBBAA`.
    pub hex: String,
    /// Its stop is the active stop (the active halo).
    pub active: bool,
    /// The pointer is over it (the hover halo).
    pub hovered: bool,
}

/// The value chip at the pointer during a drag.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GradientReadoutJson {
    /// "30.0°", "r 24.5 mm" or "42 %".
    pub text: String,
    /// Screen x of the pointer.
    pub x: f64,
    /// Screen y of the pointer.
    pub y: f64,
}

/// The entry box open at a handle.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientEntryJson {
    /// `"end"` (Angle or Radius, swatch, hex) or `"stop"` (swatch, hex, Position for an inner
    /// stop).
    pub kind: &'static str,
    /// The stop it edits.
    pub stop: usize,
    /// Screen x of the handle.
    pub x: f64,
    /// Screen y of the handle.
    pub y: f64,
    /// `"angle"`, `"radius"`, `"position"` or empty (the Radial centre).
    pub field: &'static str,
    /// The field's text.
    pub field_text: String,
    /// The stop's colour, `#RRGGBBAA`.
    pub hex: String,
}

/// The gradient handles on the canvas.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientHandlesJson {
    /// The handles are drawn.
    pub shown: bool,
    /// Radial: the circle is drawn and the first handle is the centre.
    pub radial: bool,
    /// The gradient line, screen `x1, y1, x2, y2`.
    pub line: [f64; 4],
    /// A Radial's ellipse: screen centre and its two axis vectors `cx, cy, ax, ay, bx, by`.
    pub ellipse: Option<[f64; 6]>,
    /// The handles, ends first.
    pub handles: Vec<GradientHandleJson>,
    /// The readout of a drag.
    pub readout: Option<GradientReadoutJson>,
    /// The open entry box.
    pub entry: Option<GradientEntryJson>,
}

/// `#RRGGBBAA` of a colour and its alpha.
pub(super) fn hex(colour: curvyo_document_core::Color, alpha: u8) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{alpha:02X}",
        colour.r, colour.g, colour.b
    )
}

/// `value` with at most `decimals` decimals, trailing zeros trimmed ("50", "33.3").
pub(super) fn trimmed(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

/// The Angle of `shaded` in degrees as the field shows it ("30.0").
pub(super) fn angle_text(shaded: &Shaded) -> Option<String> {
    let angle = shaded.fill.geometry.angle_in(&shaded.frame)?;
    let degrees = angle.as_radians().to_degrees();
    // -0.0 shows as 0.0.
    Some(format!("{:.1}", degrees + 0.0))
}

/// The Radius of `shaded` in millimetres as the field shows it ("24.5").
pub(super) fn radius_text(shaded: &Shaded) -> Option<String> {
    Some(trimmed(
        shaded.fill.geometry.radius_in(&shaded.frame)?.as_mm(),
        3,
    ))
}

/// The one text all `texts` share, or empty.
fn common_text(mut texts: impl Iterator<Item = Option<String>>) -> String {
    let Some(Some(first)) = texts.next() else {
        return String::new();
    };
    if texts.all(|t| t.as_ref() == Some(&first)) {
        first
    } else {
        String::new()
    }
}

/// The skip line of `count` objects that cannot have a gradient (UX notes §2, P16).
fn skip_line(images: usize, others: usize) -> String {
    match (images, others) {
        (0, 0) => String::new(),
        (1, 0) => "1 image cannot have a gradient and was not changed.".to_owned(),
        (n, 0) => format!("{n} images cannot have a gradient and were not changed."),
        (i, o) => format!(
            "{} objects cannot have a gradient and were not changed.",
            i + o
        ),
    }
}

/// The stop rows of the block: hex and position, or empty where the selection differs; the
/// first and last fixed.
fn stop_rows(rows: &[curvyo_ui_core::GradientStopRow]) -> Vec<GradientStopJson> {
    let last = rows.len().saturating_sub(1);
    rows.iter()
        .enumerate()
        .map(|(index, row)| GradientStopJson {
            hex: row
                .colour
                .map_or_else(String::new, |(colour, alpha)| hex(colour, alpha)),
            position: row
                .offset
                .map_or_else(String::new, |offset| trimmed(offset.get() * 100.0, 1)),
            fixed: index == 0 || index == last,
        })
        .collect()
}

impl Session {
    /// What the Gradient block shows (UX notes §3 to §5).
    #[must_use]
    pub fn gradient_view(&self) -> GradientViewJson {
        let objects = self.gradient_objects();
        let shaded: Vec<Shaded> = objects.iter().filter_map(Shaded::of).collect();
        let mut view = GradientViewJson {
            offered: !objects.is_empty(),
            shown: !objects.is_empty() && shaded.len() == objects.len(),
            ..GradientViewJson::default()
        };
        if !view.shown {
            return view;
        }
        let (images, others) = self.gradient_skips();
        view.skip = skip_line(images, others);
        let fills: Vec<_> = shaded.iter().map(|s| &s.fill).collect();
        let Some(panel) = gradient_panel(&fills) else {
            return view;
        };
        match panel.kind {
            Some(GradientKind::Linear) => "linear",
            Some(GradientKind::Radial) => "radial",
            None => "mixed",
        }
        .clone_into(&mut view.kind);
        view.angle_shown = panel.kind == Some(GradientKind::Linear);
        view.radius_shown = panel.kind == Some(GradientKind::Radial);
        if view.angle_shown {
            view.angle = common_text(shaded.iter().map(angle_text));
        }
        if view.radius_shown {
            view.radius = common_text(shaded.iter().map(radius_text));
        }
        match panel.rows {
            Some(rows) => {
                view.stops = stop_rows(&rows);
                view.active_stop = self.active_stop(Some(shaded[0].object.id()), rows.len());
            }
            None => view.mixed_gradients = true,
        }
        view
    }

    /// How many selected images, and how many other selected objects, cannot take a gradient
    /// (the skip line, criterion 21).
    fn gradient_skips(&self) -> (usize, usize) {
        let ids = self.style_scope().ids;
        let objects = self.objects();
        let selected = || objects.iter().filter(|o| ids.contains(&o.id()));
        let images = selected()
            .filter(|o| matches!(o, ObjectSnapshot::Image(_)))
            .count();
        let others = selected()
            .filter(|o| !takes_gradient(o) && !matches!(o, ObjectSnapshot::Image(_)))
            .count();
        (images, others)
    }

    /// What the canvas draws of the gradient handles (UX notes §6, §7).
    #[must_use]
    pub fn gradient_handles(&self) -> GradientHandlesJson {
        let Some((shaded, layout)) = self.handle_subject() else {
            return GradientHandlesJson::default();
        };
        let view = self.view();
        let screen = |p: curvyo_document_core::Point| view.document_to_screen(p);
        let (x1, y1) = screen(layout.from);
        let (x2, y2) = screen(layout.to);
        let stops = shaded.fill.stops.as_slice();
        let active = self.active_stop(Some(shaded.object.id()), stops.len());
        let handles = layout
            .handles
            .iter()
            .map(|(handle, at)| {
                let stop = handle.stop(stops.len());
                let (x, y) = screen(*at);
                GradientHandleJson {
                    kind: match handle {
                        GradientHandle::Start => "start",
                        GradientHandle::End => "end",
                        GradientHandle::Centre => "centre",
                        GradientHandle::Radius => "radius",
                        GradientHandle::Stop(_) => "stop",
                    },
                    stop,
                    x,
                    y,
                    hex: hex(stops[stop].colour, stops[stop].alpha),
                    active: stop == active,
                    hovered: self.gradient.hovered == Some(*handle),
                }
            })
            .collect();
        let scale = view.scale();
        GradientHandlesJson {
            shown: true,
            radial: layout.second_axis.is_some(),
            line: [x1, y1, x2, y2],
            ellipse: layout.second_axis.map(|b| {
                [
                    x1,
                    y1,
                    (layout.to.x - layout.from.x) * scale,
                    (layout.to.y - layout.from.y) * scale,
                    b.x * scale,
                    b.y * scale,
                ]
            }),
            handles,
            readout: self.gradient_readout(&shaded),
            entry: self.gradient_entry_view(&shaded, &layout),
        }
    }
}
