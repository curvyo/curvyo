//! `Session`'s glue for Object to path of texts and the marks of missing faces
//! (`specs/0074-text-objects` criteria 22 and 29).

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_ui_core::TextRefusal;

use super::Session;

/// What Object to path did, for its notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectToPathOutcome {
    /// Nothing to convert, or the tool is not ready.
    Ignored,
    /// Converted.
    Converted,
    /// Texts of the selection could not be converted; nothing was changed (criterion 29).
    Refused(TextRefusal),
}

impl Session {
    /// Object to path (criterion 22): the selected primitives become paths, the selected texts
    /// groups of one compound path per glyph, in one step. A text with a missing face refuses the
    /// whole command (criterion 29).
    pub fn object_to_path(&mut self) -> ObjectToPathOutcome {
        self.recording(|this| {
            this.flush_select_bar_preview();
            this.select.cancel_entry();
            let ids = this.selection.ids().to_vec();
            if ids.is_empty() || !this.selected_group_leaves().is_empty() {
                return ObjectToPathOutcome::Ignored;
            }
            let objects = this.objects();
            let nodes = std::sync::Arc::clone(&this.document_read().nodes);
            let names = |id: NodeId| {
                nodes
                    .iter()
                    .find(|node| node.id == id)
                    .and_then(|node| node.registers.name.clone())
            };
            let texts = match curvyo_ui_core::build_text_conversions(
                &objects,
                &ids,
                &names,
                &mut this.minter,
            ) {
                Ok(texts) => texts,
                Err(refusal) => {
                    this.command_refusal = Some(super::refusal::RefusalMarks::new(
                        refusal.offenders.clone(),
                        &this.selection,
                    ));
                    return ObjectToPathOutcome::Refused(refusal);
                }
            };
            if texts.is_empty() {
                this.convert_selected_to_paths();
                return ObjectToPathOutcome::Converted;
            }
            let primitives =
                curvyo_ui_core::build_primitive_conversions(&this.document, &mut this.minter, &ids);
            let converted: Vec<NodeId> = primitives.iter().map(|(id, _)| *id).collect();
            match this
                .document
                .convert_to_paths_with_texts(&primitives, &texts)
            {
                Ok(groups) => {
                    // A selected group is held as its leaves, as after Ctrl+G
                    // (`grouping.rs`), so the Style tab and the arrow keys act on the result.
                    let mut selected = converted;
                    for group in groups.into_iter().flatten() {
                        selected.extend(this.document.leaves_of(group));
                    }
                    this.node.clear_selection();
                    this.selection.set(&selected);
                    this.command_refusal = None;
                    ObjectToPathOutcome::Converted
                }
                Err(_) => ObjectToPathOutcome::Ignored,
            }
        })
    }

    /// The face a path command's refusal of the selected texts names, when there is one.
    #[must_use]
    pub fn text_refusal_face(&self) -> String {
        curvyo_ui_core::texts_refusal(&self.objects(), self.selection.ids())
            .and_then(|refusal| match refusal.missing_faces.as_slice() {
                [one] => Some(one.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// The canvas tags "Font missing" (criterion 29, UX notes §4): the screen point above the
    /// top-left of each drawn text whose face is missing, at most 12.
    #[must_use]
    pub fn missing_font_marks(&self) -> Vec<(f64, f64)> {
        let view = self.view();
        self.render_objects()
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Text(text) => text.outline.as_ref().filter(|o| o.substitute),
                _ => None,
            })
            .take(12)
            .map(|outline| view.document_to_screen(outline.line_box[0]))
            .collect()
    }
}
