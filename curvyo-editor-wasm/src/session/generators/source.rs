//! Boxes built from a selected outline: the Box from path button, Replace outline and the pick
//! mode of the Generators tool (`specs/0149-path-box` UX notes §1 to §3).

use curvyo_document_core::{NodeId, Point};
use curvyo_ui_core::instance;
use curvyo_ui_core::instance_source;

use super::Tool;
use crate::session::Session;

impl Session {
    /// The kind of the chosen segment when it is built from an outline, else `None`.
    pub(super) fn source_kind(&self) -> Option<String> {
        let kind = self.generator_kind();
        instance::find_kind(&self.manufacturing.extensions, &kind)
            .filter(|found| found.kind.source.is_some())
            .map(|_| kind)
    }

    /// The first generator kind built from an outline (`<extension>/<kind>`), for the Select bar
    /// button.
    pub(super) fn first_source_kind(&self) -> Option<String> {
        instance::enabled_kinds(&self.manufacturing.extensions)
            .into_iter()
            .find(|found| found.kind.source.is_some())
            .map(|found| format!("{}/{}", found.manifest.id, found.kind.id))
    }

    /// Whether the Select bar shows Box from path: exactly one object is selected and it is one
    /// closed outline (`0149` UX notes §1.1; the self-intersection and size tests run on the
    /// press).
    #[must_use]
    pub fn box_from_path_shown(&self) -> bool {
        let ids = self.selection.ids();
        let [id] = ids else {
            return false;
        };
        self.first_source_kind().is_some()
            && !self.document.is_generated(*id)
            && self
                .document
                .object(*id)
                .is_some_and(|object| instance_source::outline_of(&object).is_some())
    }

    /// Box from path on the selected outline, in one step "Add path box": the box is selected,
    /// the source untouched. Empty on success, else the refusal sentence ("Nothing was
    /// changed.").
    pub fn apply_box_from_path(&mut self) -> String {
        let Some(kind) = self.first_source_kind() else {
            return String::new();
        };
        let &[source] = self.selection.ids() else {
            return instance_source::SourceRefusal::NotOneOutline.to_string();
        };
        self.build_from_source(&kind, source)
    }

    /// Replace outline: the pick mode of the Generators tool for the selected path box; the next
    /// click or Enter on a closed outline replaces its stored outline. Escape cancels.
    pub fn begin_replace_outline(&mut self) -> bool {
        let &[id] = self.selection.ids() else {
            return false;
        };
        let Some(snapshot) = self.document.generated(id) else {
            return false;
        };
        if instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind)
            .is_none_or(|found| found.kind.source.is_none())
        {
            return false;
        }
        self.generators.kind = snapshot.record.kind;
        self.set_tool(Tool::Generators);
        self.generators.replace = Some(id);
        true
    }

    /// The outline under `at`, or under the pointer for Enter.
    pub(super) fn outline_under(&self, at: Option<Point>) -> Option<NodeId> {
        let point = at.or(self.pointer_position)?;
        let objects = self.objects();
        curvyo_ui_core::hit_test_objects(&objects, point, self.object_tolerance())
            .first()
            .copied()
    }

    /// A pick while Replace outline is armed.
    pub(super) fn replace_pick(&mut self, target: NodeId, at: Option<Point>) -> bool {
        let Some(source) = self.outline_under(at) else {
            if at.is_none() {
                self.generators.refusal = instance_source::SourceRefusal::NotOneOutline.to_string();
            }
            return false;
        };
        let outcome = self.recording_named_guarded(Some("Replace outline".to_owned()), |this| {
            Some(instance_source::replace_outline(
                &this.document,
                &this.manufacturing.extensions,
                target,
                source,
            ))
        });
        match outcome {
            Some(Ok(())) => {
                self.set_tool(Tool::Select);
                self.selection.set(&[target]);
                true
            }
            Some(Err(refusal)) => {
                self.generators.refusal = refusal.to_string();
                false
            }
            None => false,
        }
    }

    /// A click of the pick mode on the object under `at`, or Enter on the selection.
    pub(super) fn pick_source(&mut self, kind: &str, at: Option<Point>) -> bool {
        if let Some(target) = self.generators.replace {
            return self.replace_pick(target, at);
        }
        let source = match at {
            Some(point) => {
                let objects = self.objects();
                curvyo_ui_core::hit_test_objects(&objects, point, self.object_tolerance())
                    .first()
                    .copied()
            }
            None => match self.selection.ids() {
                &[id] => Some(id),
                _ => None,
            },
        };
        match source {
            Some(id) => self.build_from_source(kind, id).is_empty(),
            // A click on empty canvas says nothing (the Pick mode rule); Enter says why.
            None if at.is_none() => {
                self.generators.refusal = instance_source::SourceRefusal::NotOneOutline.to_string();
                false
            }
            None => false,
        }
    }

    /// Builds from `source`; a refusal is returned and kept for the canvas notice slot.
    pub(super) fn build_from_source(&mut self, kind: &str, source: NodeId) -> String {
        let name = self
            .kind_label(kind)
            .map(|label| format!("Add {}", instance::noun(label)));
        let outcome = self.recording_named_guarded(name, |this| {
            Some(instance_source::place_from_source(
                &this.document,
                &this.manufacturing.extensions,
                kind,
                source,
            ))
        });
        match outcome {
            Some(Ok(id)) => {
                self.selection.set(&[id]);
                self.set_tool(Tool::Select);
                // Warnings about the result go to the canvas notice slot (UX notes §6).
                if let Some(snapshot) = self.document.generated(id) {
                    self.generators.refusal =
                        instance::warning_notices(&self.manufacturing.extensions, &snapshot)
                            .join(" ");
                }
                String::new()
            }
            Some(Err(refusal)) => {
                let text = refusal.to_string();
                self.generators.refusal.clone_from(&text);
                text
            }
            None => String::new(),
        }
    }

    /// The cursor of the pick mode: `pointer` over an outline a box can be built from, the arrow
    /// elsewhere, `None` outside the mode.
    pub(in crate::session) fn pick_cursor(&self) -> Option<&'static str> {
        if self.tool != Tool::Generators {
            return None;
        }
        let kind = self.source_kind()?;
        let eligible = self
            .generators
            .hover
            .and_then(|point| self.outline_under(Some(point)))
            .is_some_and(|id| {
                instance_source::preview(&self.document, &self.manufacturing.extensions, &kind, id)
                    .is_ok()
            });
        Some(if eligible { "pointer" } else { "arrow" })
    }
}
