//! The entry boxes at a gradient handle (`specs/0124-gradient-fill` criterion 7, UX notes §7):
//! what the open box shows and its commit, the whole box as one step "Edit gradient".

use curvyo_document_core::{EDIT_GRADIENT_LABEL, GradientGeometry, StyleEdit};
use curvyo_ui_core::{GradientHandle, GradientLayout, parse_hex};

use super::super::Session;
use super::Shaded;
use super::commands::{parse_angle, parse_position, parse_radius};
use super::view::{GradientEntryJson, angle_text, hex, radius_text, trimmed};

impl Session {
    /// The open entry box of `shaded`'s handles, `None` when none is open on it.
    pub(super) fn gradient_entry_view(
        &self,
        shaded: &Shaded,
        layout: &GradientLayout,
    ) -> Option<GradientEntryJson> {
        let entry = self
            .gradient
            .entry
            .filter(|entry| entry.id == shaded.object.id())?;
        let stops = shaded.fill.stops.as_slice();
        let stop = entry.handle.stop(stops.len());
        let at = layout.handles.iter().find(|(h, _)| *h == entry.handle)?.1;
        let (x, y) = self.view().document_to_screen(at);
        let (kind, field, field_text) = match entry.handle {
            GradientHandle::Start | GradientHandle::End => {
                ("end", "angle", angle_text(shaded).unwrap_or_default())
            }
            GradientHandle::Radius => ("end", "radius", radius_text(shaded).unwrap_or_default()),
            GradientHandle::Stop(index) => (
                "stop",
                "position",
                trimmed(stops.get(index)?.offset.get() * 100.0, 1),
            ),
            GradientHandle::Centre => ("stop", "", String::new()),
        };
        Some(GradientEntryJson {
            kind,
            stop,
            x,
            y,
            field,
            field_text,
            hex: hex(stops[stop].colour, stops[stop].alpha),
        })
    }

    /// Enter in the entry box: its field (Angle, Radius or Position; ignored for the centre) and
    /// the stop's hex, written together as one step "Edit gradient"; the box closes.
    ///
    /// # Errors
    /// The refused field (`"number"`, `"range"`, `"hex"`); nothing is written and the box stays.
    pub fn commit_gradient_entry(
        &mut self,
        field_text: &str,
        hex_text: &str,
    ) -> Result<bool, &'static str> {
        let Some(entry) = self.gradient.entry else {
            return Ok(false);
        };
        let Some((shaded, _)) = self
            .handle_subject()
            .filter(|(s, _)| s.object.id() == entry.id)
        else {
            self.gradient.entry = None;
            return Ok(false);
        };
        let stops = shaded.fill.stops.as_slice();
        let stop_index = entry.handle.stop(stops.len());
        let typed = parse_hex(hex_text).map_err(|_| "hex")?;
        let (geometry, offset) = entry_field(entry.handle, &shaded, field_text)?;
        // invariant: an opacity lies in [0, 1], so the rounded byte lies in [0, 255].
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let alpha = typed
            .opacity
            .map_or(stops[stop_index].alpha, |o| (o.get() * 255.0).round() as u8);
        let id = entry.id;
        self.gradient.entry = None;
        Ok(self.recording(|this| -> bool {
            this.flush_style_preview();
            let before = this.document.version();
            this.document.batch(EDIT_GRADIENT_LABEL, |document| {
                if let Some(geometry) = geometry {
                    let _ = document.set_gradient_geometries(&[(id, geometry)]);
                }
                let colour = StyleEdit::GradientStopColour(stop_index, typed.color, alpha);
                let _ = document.edit_style(&[id], &colour);
                if let Some(offset) = offset {
                    let _ = document
                        .edit_style(&[id], &StyleEdit::GradientStopOffset(stop_index, offset));
                }
            });
            this.document.version() != before
        }))
    }

    /// Escape, a press outside or a tool change: the box closes and nothing is written.
    pub fn close_gradient_entry(&mut self) {
        self.gradient.entry = None;
    }
}

/// The entry box's field typed on `handle` of `shaded`: the new geometry of an end or radius
/// handle, or the new offset of an inner stop.
///
/// # Errors
/// `"number"` or `"range"` for a refused field.
fn entry_field(
    handle: GradientHandle,
    shaded: &Shaded,
    field_text: &str,
) -> Result<(Option<GradientGeometry>, Option<f64>), &'static str> {
    let geometry = &shaded.fill.geometry;
    Ok(match handle {
        GradientHandle::Start | GradientHandle::End => (
            Some(geometry.with_angle(&shaded.frame, parse_angle(field_text)?)),
            None,
        ),
        GradientHandle::Radius => (
            Some(geometry.with_radius(&shaded.frame, parse_radius(field_text)?)),
            None,
        ),
        GradientHandle::Stop(_) => (None, Some(parse_position(field_text)?)),
        GradientHandle::Centre => (None, None),
    })
}
