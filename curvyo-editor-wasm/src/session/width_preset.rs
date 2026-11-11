//! Applies a ready-made width profile to the selected path as one undo step
//! (`specs/0110-pen-modes-variable-stroke` criterion 5).

use curvyo_ui_core::WidthPreset;

use super::Session;

impl Session {
    /// Replaces the width profile of the one selected stroked path with `preset`, as one undo
    /// step that keeps the centre line; a refusal is also shown in the canvas notice slot.
    ///
    /// # Errors
    /// The sentence the canvas shows when the selection cannot take the preset.
    pub fn apply_width_preset(&mut self, preset: WidthPreset) -> Result<(), &'static str> {
        let result = self.write_width_preset(preset);
        if let Err(sentence) = result {
            *self.unsupported_width_notice.borrow_mut() = Some(sentence.to_owned());
        }
        result
    }

    /// The preset the one selected path's width profile is, for the panel to mark; `None` for
    /// another selection or a profile drawn by hand (`specs/0110` UX review U9).
    #[must_use]
    pub fn selected_width_preset(&self) -> Option<WidthPreset> {
        let [id] = self.selection.ids() else {
            return None;
        };
        let points = self.document.width_points(*id).ok()?;
        let pairs: Vec<_> = points.iter().map(|p| (p.position, p.multiplier)).collect();
        WidthPreset::matching(&pairs)
    }

    fn write_width_preset(&mut self, preset: WidthPreset) -> Result<(), &'static str> {
        let [id] = self.selection.ids() else {
            return Err("Select one path to give it a width profile.");
        };
        let id = *id;
        let Some(path) = self.document.path(id) else {
            return Err("Width profiles need a path; convert this object to a path first.");
        };
        if path.is_compound() {
            return Err("Width profiles are not available for compound paths.");
        }
        if self.units().is_inert(id) {
            return Err("Unlock the path before changing its width.");
        }
        if !path.style.stroke.enabled || path.style.stroke.width.as_mm() <= 0.0 {
            return Err("Give the path a stroke before changing its width.");
        }
        if path.closed && preset.needs_ends() {
            return Err("A taper needs an open path.");
        }
        if self.document.is_shared() || self.document.workspace_share().is_some() {
            return Err("Width profiles cannot be edited while the document is shared.");
        }
        // One batch: the reset and every point are one step.
        let written = self.recording(|this| {
            let ids: Vec<_> = preset.points().iter().map(|_| this.minter.mint()).collect();
            Some(this.document.batch("set_width_profile", |document| {
                document.reset_width_profile(id).is_ok()
                    && ids
                        .iter()
                        .zip(preset.points())
                        .all(|(&point, &(position, multiplier))| {
                            document
                                .set_width_point(id, point, position, multiplier)
                                .is_ok()
                        })
            }))
        });
        match written {
            Some(true) => {
                if let Some(state) = &mut self.width_edit {
                    state.forget_selection();
                }
                Ok(())
            }
            Some(false) => Err("The width profile could not be written."),
            None => Err("Double-click to edit this object first."),
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, NewAnchor, Point};
    use curvyo_ui_core::WidthPreset;

    use super::Session;

    fn stroked_line(session: &mut Session) -> curvyo_document_core::NodeId {
        let id = session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(9, 1), Point::new(10.0, 10.0)),
                NewAnchor::corner(AnchorId::new(9, 2), Point::new(60.0, 10.0)),
            ],
            false,
        );
        session.selection.set(&[id]);
        id
    }

    #[test]
    fn a_preset_is_one_undo_step_and_keeps_the_centre_line() {
        let mut session = Session::new(9);
        let id = stroked_line(&mut session);
        let anchors = session.document.path(id).unwrap().anchors;
        session.apply_width_preset(WidthPreset::TaperBoth).unwrap();
        let points = session.document.width_points(id).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(session.document.path(id).unwrap().anchors, anchors);
        session.apply_width_preset(WidthPreset::TaperStart).unwrap();
        assert_eq!(session.document.width_points(id).unwrap().len(), 2);
        session.undo();
        assert_eq!(
            session.document.width_points(id).unwrap().len(),
            3,
            "one undo step takes back the whole preset"
        );
        session.apply_width_preset(WidthPreset::Uniform).unwrap();
        assert_eq!(session.document.width_points(id).unwrap().len(), 0);
    }

    #[test]
    fn the_applied_preset_is_the_selected_one() {
        let mut session = Session::new(9);
        assert_eq!(session.selected_width_preset(), None, "nothing selected");
        let id = stroked_line(&mut session);
        assert_eq!(session.selected_width_preset(), Some(WidthPreset::Uniform));
        session.apply_width_preset(WidthPreset::TaperEnd).unwrap();
        assert_eq!(session.selected_width_preset(), Some(WidthPreset::TaperEnd));
        session
            .document
            .set_width_point(id, AnchorId::new(9, 77), 0.5, 2.0)
            .unwrap();
        assert_eq!(session.selected_width_preset(), None, "drawn by hand");
    }

    #[test]
    fn a_taper_is_refused_on_a_closed_path_and_without_a_selection() {
        let mut session = Session::new(9);
        assert!(session.apply_width_preset(WidthPreset::TaperEnd).is_err());
        let id = session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(9, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(9, 2), Point::new(10.0, 0.0)),
                NewAnchor::corner(AnchorId::new(9, 3), Point::new(0.0, 10.0)),
            ],
            true,
        );
        session.selection.set(&[id]);
        let version = session.document.version();
        assert_eq!(
            session.apply_width_preset(WidthPreset::TaperEnd),
            Err("A taper needs an open path.")
        );
        assert_eq!(session.document.version(), version, "nothing written");
        assert!(session.apply_width_preset(WidthPreset::Uniform).is_ok());
    }
}
