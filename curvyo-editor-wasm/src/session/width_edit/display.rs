//! How width editing is drawn: the diamonds, the live outline of a gesture and its readout.

use curvyo_document_core::Length;
use curvyo_ui_core::WidthGesturePart;

use super::{Session, measure, width_path};

impl Session {
    /// "Width 3.25 mm" at the pointer while a drag sets a width, in the display unit.
    pub(in crate::session) fn width_live_readout(
        &self,
    ) -> Option<crate::session::shapes::LiveReadout> {
        let state = self.width_edit.as_ref()?;
        let drag = state.drag.as_ref()?;
        if !drag.wheel && drag.gesture.axis() != Some(WidthGesturePart::Width) {
            return None;
        }
        let path = width_path(&self.document, &self.spiro_cache, state.path)?;
        let width =
            Length::from_mm(path.style.stroke.width.as_mm() * drag.gesture.preview().multiplier);
        let unit = self.display_unit();
        Some(crate::session::shapes::LiveReadout {
            text: format!(
                "Width {} {}",
                curvyo_ui_core::format_field_length(width, unit),
                unit.symbol()
            ),
            anchor: drag.pointer,
        })
    }

    /// The diamonds on the centre line, the selected one filled, and the ghost under the pointer.
    pub(in crate::session) fn width_handle_overlay(&self) -> curvyo_render_core::DrawList {
        let Some((id, points)) = self.width_points_for_display() else {
            return curvyo_render_core::DrawList::default();
        };
        let Some(path) = width_path(&self.document, &self.spiro_cache, id) else {
            return curvyo_render_core::DrawList::default();
        };
        let Some(measure) = measure(&path) else {
            return curvyo_render_core::DrawList::default();
        };
        let state = self.width_edit.as_ref();
        let selected = state.and_then(|state| state.selected);
        let mut diamonds: Vec<_> = points
            .iter()
            .filter_map(|point| {
                let at = curvyo_ui_core::width_handle(
                    &measure,
                    point.position,
                    Length::from_mm(0.0),
                    0.0,
                )?;
                Some(curvyo_render_core::WidthDiamond {
                    at,
                    selected: selected == Some(point.id),
                    ghost: false,
                })
            })
            .collect();
        if let Some(at) = state.and_then(|state| state.ghost) {
            diamonds.push(curvyo_render_core::WidthDiamond {
                at,
                selected: false,
                ghost: true,
            });
        }
        curvyo_render_core::build_width_diamonds(&diamonds, self.view())
    }

    /// `objects` with the path in width editing drawn with the drag's preview profile.
    pub(in crate::session) fn width_preview_artwork(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
    ) -> Option<Vec<curvyo_document_core::ObjectSnapshot>> {
        self.width_edit.as_ref()?.drag.as_ref()?;
        let (id, points) = self.width_points_for_display()?;
        Some(
            objects
                .iter()
                .map(|object| match object {
                    curvyo_document_core::ObjectSnapshot::Path(path) if path.id == id => {
                        let mut preview = path.clone();
                        preview.width_points.clone_from(&points);
                        curvyo_document_core::ObjectSnapshot::Path(preview)
                    }
                    _ => object.clone(),
                })
                .collect(),
        )
    }
}
