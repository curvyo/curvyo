//! What the frame shows of linked clones (`specs/0049-linked-clone` criteria 11 and 57 to 62):
//! the marks of a selected clone and of what it follows, and the preview of the clones behind an
//! original a drag edits. Read from the frame's one document read.

use std::cell::RefCell;
use std::sync::Arc;

use curvyo_document_core::{HeldClones, ObjectSnapshot, PathSnapshot, clone_previews};
use curvyo_render_core::CloneTrace;
use curvyo_ui_core::{CloneMarks, MarkForm, MarkFrame, MarkRect, MarkRole, clone_marks};

use super::{Session, Tool};

/// With more clones than this behind an edited original, no clone preview is drawn and the
/// clones jump on release (criterion 11).
const MAX_CLONE_PREVIEWS: usize = 500;

/// The screen size a handle glyph takes for tag placement (px): the largest glyph.
const HANDLE_GLYPH_PX: f64 = 18.0;

/// At most this many targets of Select original get the hover box (criterion 25).
const MAX_HOVER_TARGETS: usize = 50;

/// [`curvyo_document_core::Document::own_placement_clones`] of one document read: a drag reads
/// it once, not every frame.
#[derive(Debug, Default)]
pub(super) struct HeldCache {
    held: RefCell<Option<HeldOfRead>>,
}

/// The leaves of a document read and how its clones with own placement hold back.
type HeldOfRead = (Arc<[ObjectSnapshot]>, Arc<HeldClones>);

impl Session {
    /// The marks for the frame: empty with a tool other than Select or Node, during a canvas drag
    /// and while a preview outline is drawn (`previewing`, criterion 59).
    pub(super) fn clone_marks_in(
        &self,
        objects: &[ObjectSnapshot],
        previewing: bool,
    ) -> CloneMarks {
        if !matches!(self.tool, Tool::Select | Tool::Node)
            || self.selection.ids().is_empty()
            || !objects.iter().any(|object| match object {
                ObjectSnapshot::Clone(_) => true,
                ObjectSnapshot::Text(text) => text.source.is_some(),
                _ => false,
            })
        {
            return CloneMarks::default();
        }
        let view = self.view();
        let decoration = self.select_transform_decoration_input_in(objects);
        // A tag meets no drawn handle glyph, no reference mark of the typed move and no DOM chip
        // the host reported (criterion 60).
        let square = |at: curvyo_document_core::Point| {
            let (x, y) = view.document_to_screen(at);
            MarkRect {
                x: x - HANDLE_GLYPH_PX / 2.0,
                y: y - HANDLE_GLYPH_PX / 2.0,
                w: HANDLE_GLYPH_PX,
                h: HANDLE_GLYPH_PX,
            }
        };
        let mut obstacles: Vec<MarkRect> = decoration
            .handles
            .iter()
            .map(|handle| square(handle.position))
            .collect();
        if let Some(reference) = self.reference_decoration() {
            obstacles.extend(reference.marks.iter().map(|mark| square(mark.at)));
        }
        obstacles.extend(self.dom_chips.iter().copied());
        let suppressed = previewing || self.select.drag_in_flight() || self.button_down;
        let read = self.document_read();
        let hidden: std::collections::HashSet<curvyo_document_core::NodeId> = read
            .nodes
            .iter()
            .filter(|node| !node.visible)
            .map(|node| node.id)
            .collect();
        clone_marks(
            objects,
            self.selection.ids(),
            &MarkFrame {
                view,
                viewport: self.viewport.canvas_size(),
                obstacles: &obstacles,
                suppressed,
                hidden: &hidden,
            },
        )
    }

    /// The preview layer of a Node-tool drag: the clones behind the paths it edits, as the blue
    /// outline (criterion 11), over the artwork and under the boxes and handles, and whether a
    /// live edit is in the frame (the marks then stay away). The tiling instances among those
    /// clones are returned apart: the artwork draws them as themselves, as a release leaves them
    /// (`specs/0069-tiling` criterion 29, live during the gesture). A Select drag
    /// (`select_drag`) draws its clones as themselves ([`super::drag_swap`]). The marks are
    /// [`Session::clone_mark_layer`].
    pub(super) fn clone_layers(
        &self,
        view: curvyo_document_core::ViewTransform,
        objects: &[ObjectSnapshot],
        paths: &[PathSnapshot],
        select_drag: bool,
    ) -> (curvyo_render_core::DrawList, bool, Vec<ObjectSnapshot>) {
        let mut list = curvyo_render_core::DrawList::default();
        let changed = if !select_drag && self.tool == Tool::Node {
            Self::changed_paths(objects, paths)
        } else {
            Vec::new()
        };
        let mut tiled = Vec::new();
        if !changed.is_empty() {
            // A Node drag edits the shape: every clone behind it follows.
            let held = self.own_placement_clones();
            let (instances, behind): (Vec<ObjectSnapshot>, Vec<ObjectSnapshot>) = self
                .clone_previews_in(objects, &changed, false)
                .into_iter()
                .partition(|clone| {
                    matches!(
                        held.get(&clone.id()),
                        Some(curvyo_document_core::Hold::Translation { .. })
                    )
                });
            list.extend(curvyo_render_core::build_live_edit_preview(&behind, view));
            tiled = instances;
        }
        (list, select_drag || !changed.is_empty(), tiled)
    }

    /// The violet traces of a selected clone and of what it follows (criteria 57 and 60), drawn
    /// over the selection and hover boxes and under the handles, so a box that coincides with an
    /// outline never hides its trace (design call on UX review must-fix 3). `previewing` says a
    /// live edit is in the frame, which suppresses the marks.
    pub(super) fn clone_mark_layer(
        &self,
        view: curvyo_document_core::ViewTransform,
        objects: &[ObjectSnapshot],
        previewing: bool,
    ) -> curvyo_render_core::DrawList {
        let marks = self.clone_marks_in(objects, previewing);
        if marks.traces.is_empty() {
            return curvyo_render_core::DrawList::default();
        }
        curvyo_render_core::build_clone_marks(objects, &Self::clone_traces(&marks), view)
    }

    /// The screen rectangles of the DOM chips over the canvas (entry chips, hints, notices), in
    /// canvas px as `[x, y, w, h, ...]`: no tag of the marks may meet one (criterion 60). The
    /// host reports them before it reads the tags.
    pub fn set_clone_tag_obstacles(&mut self, flat: &[f64]) {
        self.dom_chips = flat
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[x, y, w, h]| MarkRect { x, y, w, h })
            .collect();
    }

    /// The pointer or the keyboard focus rests on Select original (`"original"`) or Select root
    /// original (`"root"`), or on neither (`""`): each target shows the hover box of `0014`, at
    /// most 50 (criterion 25).
    pub fn set_clone_hover(&mut self, kind: &str) {
        let read = self.object_read();
        let targets = match kind {
            "original" => curvyo_ui_core::select_originals(&read, self.selection.ids()),
            "root" => curvyo_ui_core::select_root_originals(&read, self.selection.ids()),
            _ => Vec::new(),
        };
        self.history_hover = targets.into_iter().take(MAX_HOVER_TARGETS).collect();
    }

    /// The tags and edge chips of the marks, for the host to place as DOM chips (criterion 60).
    #[must_use]
    pub fn clone_marks(&self) -> CloneMarks {
        let objects = self.objects();
        let previewing = self.select_live_edit_in(&objects).is_some();
        self.clone_marks_in(&objects, previewing)
    }

    /// How the renderer draws each traced object.
    pub(super) fn clone_traces(
        marks: &CloneMarks,
    ) -> Vec<(curvyo_document_core::NodeId, CloneTrace)> {
        marks
            .traces
            .iter()
            .map(|trace| {
                let drawn = match (trace.role, trace.form) {
                    (MarkRole::Clone | MarkRole::FaintClone, MarkForm::Ring) => {
                        CloneTrace::CloneRing
                    }
                    (_, MarkForm::Ring) => CloneTrace::OriginalRing,
                    (MarkRole::Clone, MarkForm::Trace) => CloneTrace::Clone,
                    (MarkRole::Original, MarkForm::Trace) => CloneTrace::Original,
                    (MarkRole::Root, MarkForm::Trace) => CloneTrace::Root,
                    (MarkRole::FaintClone, MarkForm::Trace) => CloneTrace::Faint,
                };
                (trace.id, drawn)
            })
            .collect()
    }

    /// The outlines the clones behind `changed` would have on release, for the preview of a
    /// drag that edits an original, by the commit's own rule (`transform`: a Select drag);
    /// a tiling's instances included, the other clones only up to [`MAX_CLONE_PREVIEWS`].
    pub(super) fn clone_previews_in(
        &self,
        objects: &[ObjectSnapshot],
        changed: &[ObjectSnapshot],
        transform: bool,
    ) -> Vec<ObjectSnapshot> {
        clone_previews(
            objects,
            changed,
            MAX_CLONE_PREVIEWS,
            &self.own_placement_clones(),
            transform,
        )
        .unwrap_or_default()
        .into_iter()
        .map(ObjectSnapshot::Path)
        .collect()
    }

    /// How each clone with own placement holds back its original's change, read once per
    /// document read; empty, without reading the document, when no clone has own placement.
    pub(super) fn own_placement_clones(&self) -> Arc<HeldClones> {
        let leaves = Arc::clone(&self.document_read().leaves);
        let mut cache = self.held_clones.held.borrow_mut();
        if let Some((read, held)) = cache.as_ref()
            && Arc::ptr_eq(read, &leaves)
        {
            return Arc::clone(held);
        }
        let any = leaves
            .iter()
            .any(|o| matches!(o, ObjectSnapshot::Clone(c) if c.chain.own_placement));
        let held = Arc::new(if any {
            self.document.own_placement_clones()
        } else {
            HeldClones::new()
        });
        *cache = Some((leaves, Arc::clone(&held)));
        held
    }

    /// The paths of a Node-tool drag that differ from the read, as changed objects.
    pub(super) fn changed_paths(
        objects: &[ObjectSnapshot],
        paths: &[PathSnapshot],
    ) -> Vec<ObjectSnapshot> {
        paths
            .iter()
            .filter(|path| {
                objects
                    .iter()
                    .any(|object| matches!(object, ObjectSnapshot::Path(p) if p.id == path.id && p != *path))
            })
            .cloned()
            .map(ObjectSnapshot::Path)
            .collect()
    }
}
