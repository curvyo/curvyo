//! The commands of the Gradient block (`specs/0124-gradient-fill` criteria 3, 7, 8, 18): Kind,
//! Angle, Radius, a stop's colour and position, Add stop, Remove, Reverse and the picker's
//! preview, each one step with its History name.

use curvyo_document_core::{Angle, EDIT_GRADIENT_LABEL, GradientKind, Length, NodeId, StyleEdit};
use curvyo_ui_core::{hsv_to_rgb, parse_hex};

use super::super::Session;

/// A typed Angle in degrees, wrapped into `(-180, 180]` (UX notes §3, P7).
///
/// # Errors
/// `"number"` for text that is no finite number.
pub(super) fn parse_angle(text: &str) -> Result<Angle, &'static str> {
    let value: f64 = text.trim().parse().map_err(|_| "number")?;
    if !value.is_finite() {
        return Err("number");
    }
    let wrapped = value.rem_euclid(360.0);
    let degrees = if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    };
    Ok(Angle::from_radians(degrees.to_radians()))
}

/// A typed Radius in millimetres, above 0 and at most 10,000.
///
/// # Errors
/// `"number"` for text that is no number, `"range"` outside the range.
pub(super) fn parse_radius(text: &str) -> Result<Length, &'static str> {
    let value: f64 = text.trim().parse().map_err(|_| "number")?;
    if !(value > 0.0 && value <= 10_000.0) {
        return Err("range");
    }
    Ok(Length::from_mm(value))
}

/// A typed Position in percent, 0 to 100, as an offset.
///
/// # Errors
/// `"number"` for text that is no number, `"range"` outside 0 to 100.
pub(super) fn parse_position(text: &str) -> Result<f64, &'static str> {
    let trimmed = text.trim().trim_end_matches('%').trim();
    let value: f64 = trimmed.parse().map_err(|_| "number")?;
    if !(0.0..=100.0).contains(&value) {
        return Err("range");
    }
    Ok(value / 100.0)
}

impl Session {
    /// The Kind switch: one step "Gradient: Linear" or "Gradient: Radial"; the stops stay.
    pub fn set_gradient_kind(&mut self, kind: &str) -> bool {
        let kind = match kind {
            "linear" => GradientKind::Linear,
            "radial" => GradientKind::Radial,
            _ => return false,
        };
        self.apply_gradient_edit(&StyleEdit::GradientKind(kind))
    }

    /// Writes per-shape geometry made by `make` from each edited gradient and its box, one step
    /// "Edit gradient"; whether anything changed.
    fn set_each_geometry(
        &mut self,
        make: impl Fn(&super::Shaded) -> curvyo_document_core::GradientGeometry,
    ) -> bool {
        let edits: Vec<(NodeId, _)> = self
            .shaded_objects()
            .iter()
            .map(|shaded| (shaded.object.id(), make(shaded)))
            .collect();
        self.recording(|this| -> bool {
            this.flush_style_preview();
            let before = this.document.version();
            this.document.set_gradient_geometries(&edits).is_ok()
                && this.document.version() != before
        })
    }

    /// Enter or Tab in the Angle field: each Linear turned about its midpoint, one step.
    ///
    /// # Errors
    /// `"number"`; nothing is written.
    pub fn set_gradient_angle_text(&mut self, text: &str) -> Result<bool, &'static str> {
        let angle = parse_angle(text)?;
        Ok(self.set_each_geometry(|s| s.fill.geometry.with_angle(&s.frame, angle)))
    }

    /// Enter or Tab in the Radius field: each Radial's radius handle moved along its direction.
    ///
    /// # Errors
    /// `"number"` or `"range"`; nothing is written.
    pub fn set_gradient_radius_text(&mut self, text: &str) -> Result<bool, &'static str> {
        let radius = parse_radius(text)?;
        Ok(self.set_each_geometry(|s| s.fill.geometry.with_radius(&s.frame, radius)))
    }

    /// Enter or Tab in a stop's hex field: that stop of every edited gradient, one step "Edit
    /// gradient". A 3 or 6 digit form keeps each stop's alpha.
    ///
    /// # Errors
    /// `"hex"`; nothing is written.
    pub fn set_gradient_stop_hex(
        &mut self,
        index: usize,
        text: &str,
    ) -> Result<bool, &'static str> {
        let typed = parse_hex(text).map_err(|_| "hex")?;
        let edits: Vec<(NodeId, StyleEdit)> = self
            .shaded_objects()
            .iter()
            .filter_map(|shaded| {
                let stop = shaded.fill.stops.as_slice().get(index)?;
                // invariant: an opacity lies in [0, 1], so the rounded byte lies in [0, 255].
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let alpha = typed
                    .opacity
                    .map_or(stop.alpha, |o| (o.get() * 255.0).round() as u8);
                Some((
                    shaded.object.id(),
                    StyleEdit::GradientStopColour(index, typed.color, alpha),
                ))
            })
            .collect();
        Ok(self.write_each(&edits))
    }

    /// Writes each object its own edit in one step "Edit gradient".
    fn write_each(&mut self, edits: &[(NodeId, StyleEdit)]) -> bool {
        if edits.is_empty() {
            return false;
        }
        self.recording(|this| -> bool {
            this.flush_style_preview();
            let before = this.document.version();
            this.document.batch(EDIT_GRADIENT_LABEL, |document| {
                for (id, edit) in edits {
                    let _ = document.edit_style(&[*id], edit);
                }
            });
            this.document.version() != before
        })
    }

    /// Enter or Tab in an inner stop's Position: clamped between its neighbours, one step.
    ///
    /// # Errors
    /// `"number"` or `"range"`; nothing is written.
    pub fn set_gradient_stop_position_text(
        &mut self,
        index: usize,
        text: &str,
    ) -> Result<bool, &'static str> {
        let offset = parse_position(text)?;
        Ok(self.apply_gradient_edit(&StyleEdit::GradientStopOffset(index, offset)))
    }

    /// A tick of a picker drag on stop `index`: hue (degrees), saturation and value (`0..=1`),
    /// previewed; [`Session::commit_style_preview`] writes it as one step "Edit gradient".
    pub fn preview_gradient_stop_hsv(
        &mut self,
        index: usize,
        hue: f64,
        saturation: f64,
        value: f64,
    ) {
        let Some(alpha) = self
            .shaded_objects()
            .first()
            .and_then(|s| s.fill.stops.as_slice().get(index).map(|stop| stop.alpha))
        else {
            return;
        };
        let colour = hsv_to_rgb(hue, saturation, value);
        self.preview_style_edit(StyleEdit::GradientStopColour(index, colour, alpha));
    }

    /// Add stop: a stop in the middle of the widest gap, in the shade's colour there, one step
    /// "Add gradient stop"; the new stop is active.
    pub fn add_gradient_stop(&mut self) -> bool {
        let added = self.shaded_objects().first().map(|s| {
            let stops = &s.fill.stops;
            (
                s.object.id(),
                stops.with_stop_added(stops.widest_gap_middle()).1,
            )
        });
        let wrote = self.apply_gradient_edit(&StyleEdit::AddGradientStop(None));
        if let (true, Some((id, index))) = (wrote, added) {
            self.gradient.subject = Some(id);
            self.gradient.active = index;
        }
        wrote
    }

    /// Remove on an inner stop row: one step "Remove gradient stop"; the next stop becomes
    /// active, the previous one after the last inner stop.
    pub fn remove_gradient_stop(&mut self, index: usize) -> bool {
        let count = self.panel_stop_count();
        if index == 0 || index + 1 >= count {
            return false;
        }
        let wrote = self.apply_gradient_edit(&StyleEdit::RemoveGradientStop(index));
        if wrote {
            self.gradient.active = if index + 2 == count { index - 1 } else { index };
        }
        wrote
    }

    /// Reverse: each gradient's stops mirrored, one step "Reverse gradient"; the active stop stays
    /// the same stop, in its mirrored row.
    pub fn reverse_gradient(&mut self) -> bool {
        let count = self.panel_stop_count();
        let wrote = self.apply_gradient_edit(&StyleEdit::ReverseGradient);
        if wrote && count > 0 {
            self.gradient.active = count - 1 - self.gradient.active.min(count - 1);
        }
        wrote
    }
}
