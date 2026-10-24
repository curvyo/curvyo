//! The angle entry `R` of a text on an ellipse carrier (`specs/0080-text-on-path` criterion 20).

use curvyo_document_core::{OnPathLabel, OnPathValues};

use super::Session;

impl Session {
    /// Enter in the angle entry `R` of the sole selected text on an ellipse carrier (criterion
    /// 20): the text's middle moves about the centre by the typed angle less the shown one, as
    /// the rotate handles move it, in one step "Move text on path"; the entry closes. `None`
    /// when the entry is no such angle entry or the text does not parse: the generic commit runs.
    pub(super) fn commit_text_path_rotate_entry(
        &mut self,
        typed: &str,
    ) -> Option<curvyo_ui_core::EntryOutcome> {
        let entry = self.open_entry()?;
        if entry.kind() != curvyo_ui_core::EntryKind::Angle {
            return None;
        }
        let shown = entry.object().orientation().as_radians().to_degrees();
        let text = self.sole_text_on_path()?;
        if text.id != entry.object().id() {
            return None;
        }
        let on_path = text.on_path.as_ref()?;
        let ellipse = on_path.curve.as_ref()?.ellipse?;
        let degrees = curvyo_ui_core::parse_entry_number(typed, true)? - shown;
        let on = self.on_carrier(&text)?;
        let values = OnPathValues {
            position: curvyo_ui_core::rotated_position(
                on.placement.measure(),
                &ellipse,
                &on_path.values,
                degrees,
            ),
            ..on_path.values
        };
        self.select.cancel_entry();
        if degrees.abs() < 1e-9 {
            return Some(curvyo_ui_core::EntryOutcome::Unchanged);
        }
        let _ = self
            .document
            .set_text_path(&[(text.id, values)], OnPathLabel::Move);
        Some(curvyo_ui_core::EntryOutcome::Committed)
    }
}
