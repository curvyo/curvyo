//! The override commands of the clone block with their notices (`specs/0057-clone-overrides`
//! criteria 2, 7, 8, 14, 18 and 42): the Placement and Style switches, Spread colours, and the
//! notices a transform or a style edit adds when it held clones back or left them alone.

use curvyo_document_core::{NodeId, ObjectSnapshot};

use super::clones::CloneResult;
use super::{Session, Tool};

/// "1 clone" / "3 clones".
fn clones_word(count: usize) -> String {
    format!("{count} {}", if count == 1 { "clone" } else { "clones" })
}

impl Session {
    /// The selected clones, when the selection holds only clones (the clone block's targets).
    fn block_clones(&self) -> Vec<NodeId> {
        let read = self.object_read();
        let ids = self.selection.ids();
        if ids.is_empty()
            || !ids
                .iter()
                .all(|&id| matches!(read.get(id), Some(ObjectSnapshot::Clone(_))))
        {
            return Vec::new();
        }
        ids.to_vec()
    }

    /// The selected linked clones of a selection of clones that holds text clones too.
    fn linked_block_clones(&self) -> Vec<NodeId> {
        let read = self.object_read();
        self.selection
            .ids()
            .iter()
            .copied()
            .filter(|id| matches!(read.get(*id), Some(ObjectSnapshot::Clone(_))))
            .collect()
    }

    /// The Placement row (criterion 2): `own` true is "Own placement", false "Follows original",
    /// for every selected clone in one commit. Pressing the state they already have does nothing
    /// and posts nothing.
    pub fn set_clone_placement(&mut self, own: bool) -> CloneResult {
        self.recording(|this| {
            if this.tool != Tool::Select && this.tool != Tool::Node {
                return CloneResult::ignored();
            }
            let clones = this.block_clones();
            match this.document.set_own_placement(&clones, own) {
                Ok(n) if n > 0 => CloneResult::done(
                    if own {
                        "Own placement. It stays put when the original moves."
                    } else {
                        "Follows its original's placement again."
                    }
                    .to_owned(),
                ),
                _ => CloneResult::ignored(),
            }
        })
    }

    /// The Style row (criteria 11 and 14): `own` true copies the style each clone shows, false
    /// removes the own style; one commit, nothing for the state they already have.
    pub fn set_clone_style(&mut self, own: bool) -> CloneResult {
        self.recording(|this| {
            if this.tool != Tool::Select && this.tool != Tool::Node {
                return CloneResult::ignored();
            }
            this.flush_style_preview();
            let clones = this.block_clones();
            // Text clones have the Style row too (`specs/0080-text-on-path` criterion 40).
            let texts = this.selected_text_clones();
            // One step for both kinds (lead decision 2).
            let changed = if texts.is_empty() {
                this.document.set_own_style(&clones, own)
            } else {
                this.document
                    .set_own_style_and_texts(&this.linked_block_clones(), &texts, own)
            };
            match changed {
                Ok(n) if n > 0 => CloneResult::done(
                    if own {
                        "Own style, copied from the original."
                    } else {
                        "Shows its original's style again. Its own style was removed."
                    }
                    .to_owned(),
                ),
                _ => CloneResult::ignored(),
            }
        })
    }

    /// Whether Spread colours is enabled: two or more clones selected (criterion 42), linked or
    /// text clones (`specs/0080-text-on-path`, lead decision 3).
    #[must_use]
    pub fn spread_colours_ready(&self) -> bool {
        self.selected_clones_of_both_kinds().len() >= 2
    }

    /// Spread colours (criterion 42): evenly spaced hues over the selected clones, back to front,
    /// one commit; nothing with fewer than two clones.
    pub fn spread_colours(&mut self) -> CloneResult {
        self.recording(|this| {
            if this.tool != Tool::Select && this.tool != Tool::Node {
                return CloneResult::ignored();
            }
            let clones = this.selected_clones_of_both_kinds();
            let colours = curvyo_ui_core::spread_colours(&this.object_read(), &clones);
            if colours.is_empty() {
                return CloneResult::ignored();
            }
            match this.document.recolour_clones(&colours) {
                Ok(n) if n > 0 => {
                    CloneResult::done(format!("Spread hues over {}.", clones_word(colours.len())))
                }
                _ => CloneResult::ignored(),
            }
        })
    }

    /// The sentence of the last transform that held clones with own placement back (criteria 7
    /// and 8), once: "3 clones with own placement stayed in place.", "2 clones became paths.",
    /// or empty.
    pub fn take_clone_hold_notice(&mut self) -> String {
        let hold = self.document.take_clone_hold();
        let mut parts = Vec::new();
        if hold.unlinked > 0 {
            // The words of criterion 7 ("2 clones became paths."), as the repair says them.
            parts.push(if hold.unlinked == 1 {
                "1 clone became a path.".to_owned()
            } else {
                format!("{} clones became paths.", hold.unlinked)
            });
        }
        if hold.held > 0 {
            parts.push(format!(
                "{} with own placement stayed in place.",
                clones_word(hold.held)
            ));
        }
        parts.join(" ")
    }

    /// Remembers the notice of criterion 18 for an edit that went to `targets` while the
    /// selection also held clones that show their original's style.
    pub(super) fn note_skipped_clones(&mut self, wrote_any: bool) {
        if !wrote_any {
            return;
        }
        let skipped = curvyo_ui_core::skipped_clones(&self.objects(), self.selection.ids());
        self.skip_notice = (skipped > 0).then(|| {
            if skipped == 1 {
                "1 clone shows its original's style and was not changed.".to_owned()
            } else {
                format!("{skipped} clones show their original's style and were not changed.")
            }
        });
    }

    /// Adds the sentence of a fill edit that left single-line texts alone
    /// (`specs/0076-single-line-fonts`, UX notes §4) after the clone sentence, if any.
    pub(super) fn note_skipped_single_line(
        &mut self,
        edit: &curvyo_document_core::StyleEdit,
        wrote_any: bool,
    ) {
        if !wrote_any || !edit.is_fill_edit() {
            return;
        }
        let skipped =
            curvyo_ui_core::skipped_single_line_texts(&self.objects(), &self.style_scope().ids);
        if skipped == 0 {
            return;
        }
        let sentence = if skipped == 1 {
            "1 single-line text has no fill and was not changed.".to_owned()
        } else {
            format!("{skipped} single-line texts have no fill and were not changed.")
        };
        self.skip_notice = Some(match self.skip_notice.take() {
            Some(clones) => format!("{clones} {sentence}"),
            None => sentence,
        });
    }

    /// The notice of criterion 18 of the last style or opacity edit, once; empty when it left no
    /// clone alone.
    pub fn take_style_skip_notice(&mut self) -> String {
        self.skip_notice.take().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, Color, Document, NewAnchor, Point, Vec2, pack};

    use super::*;
    use crate::session::KeyInput;

    fn session_with(clones: usize) -> (Session, NodeId, Vec<NodeId>) {
        let d = Document::new(1);
        let corner =
            |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
        let p = d.create_path(
            &[
                corner(0, 0.0, 0.0),
                corner(1, 10.0, 0.0),
                corner(2, 10.0, 10.0),
                corner(3, 0.0, 10.0),
            ],
            true,
        );
        let made: Vec<NodeId> = (0..clones)
            .map(|k| {
                let c = d.linked_clone(&[p], &[]).unwrap()[0];
                #[allow(clippy::cast_precision_loss)]
                d.translate_objects(&[c], Vec2::new(30.0 * (k + 1) as f64, 0.0))
                    .unwrap();
                c
            })
            .collect();
        let mut session = Session::open(2, &pack(&d, "t").unwrap()).unwrap();
        session.set_tool(Tool::Select);
        (session, p, made)
    }

    #[test]
    fn the_switches_post_their_notices_once_and_nothing_for_the_same_state() {
        let (mut s, _, clones) = session_with(1);
        s.selection.set(&clones);
        let own = s.set_clone_placement(true);
        assert_eq!(
            own.notice,
            "Own placement. It stays put when the original moves."
        );
        assert_eq!(s.set_clone_placement(true).code(), "ignored");
        assert_eq!(
            s.set_clone_placement(false).notice,
            "Follows its original's placement again."
        );
        assert_eq!(
            s.set_clone_style(true).notice,
            "Own style, copied from the original."
        );
        assert_eq!(
            s.set_clone_style(false).notice,
            "Shows its original's style again. Its own style was removed."
        );
    }

    #[test]
    fn a_move_of_the_original_says_how_many_stayed() {
        let (mut s, p, clones) = session_with(3);
        s.selection.set(&clones);
        s.set_clone_placement(true);
        s.selection.set(&[p]);
        let _ = s.key_down(KeyInput {
            key: "ArrowRight",
            ..KeyInput::default()
        });
        assert_eq!(
            s.take_clone_hold_notice(),
            "3 clones with own placement stayed in place."
        );
        assert_eq!(s.take_clone_hold_notice(), "");
    }

    #[test]
    fn spread_colours_needs_two_clones_and_is_one_step() {
        let (mut s, _, clones) = session_with(3);
        s.selection.set(&clones[..1]);
        assert!(!s.spread_colours_ready());
        assert_eq!(s.spread_colours().code(), "ignored");
        s.selection.set(&clones);
        assert!(s.spread_colours_ready());
        assert_eq!(s.spread_colours().notice, "Spread hues over 3 clones.");
        assert_eq!(
            s.document.own_steps().last().unwrap().label,
            "spread_colours"
        );
        let read = s.object_read();
        let colours: Vec<Color> = clones
            .iter()
            .map(|id| read.get(*id).unwrap().style().stroke.color)
            .collect();
        assert!(
            colours.iter().all(|c| *c == Color::BLACK),
            "black stays black"
        );
    }

    #[test]
    fn a_style_edit_on_a_mixed_selection_names_the_clones_left_alone() {
        let (mut s, p, clones) = session_with(2);
        s.selection.set(&clones[..1]);
        s.set_clone_style(true);
        let mut all = vec![p];
        all.extend(&clones);
        s.selection.set(&all);
        s.set_stroke_paint(false);
        assert_eq!(
            s.take_style_skip_notice(),
            "1 clone shows its original's style and was not changed."
        );
        let read = s.object_read();
        assert!(
            !read.get(clones[0]).unwrap().style().stroke.enabled,
            "own style written"
        );
        assert!(!read.get(p).unwrap().style().stroke.enabled);
    }

    #[test]
    fn a_colour_drag_on_a_mixed_selection_names_the_clones_left_alone_on_release() {
        // UX review must-fix 4: a drag commit sets the notice like a typed edit.
        let (mut s, p, clones) = session_with(2);
        let mut all = vec![p];
        all.extend(&clones);
        s.selection.set(&all);
        s.preview_style_hsv(curvyo_ui_core::StyleField::StrokeColor, 120.0, 1.0, 1.0);
        assert_eq!(s.take_style_skip_notice(), "", "nothing written yet");
        s.commit_style_preview();
        assert_eq!(
            s.take_style_skip_notice(),
            "2 clones show their original's style and were not changed."
        );
        s.commit_style_preview();
        assert_eq!(
            s.take_style_skip_notice(),
            "",
            "a release without a drag says nothing"
        );
    }

    #[test]
    fn a_collapse_says_how_many_clones_became_paths_in_the_words_of_criterion_7() {
        let (mut s, p, clones) = session_with(2);
        s.selection.set(&clones);
        s.set_clone_placement(true);
        let read = s.object_read();
        let ObjectSnapshot::Path(path) = read.get(p).unwrap() else {
            panic!("a path")
        };
        let flat: Vec<_> = path
            .anchors
            .iter()
            .map(|a| (a.id, Point::new(0.0, a.point.y), a.handle_in, a.handle_out))
            .collect();
        s.document.resize_path(p, &flat, None).unwrap();
        assert_eq!(s.take_clone_hold_notice(), "2 clones became paths.");
    }
}
