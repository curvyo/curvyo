//! What the canvas draws for text on path (`specs/0080-text-on-path` slice 1): carrier lines,
//! the pick-mode hover, the previews of drags, the handles of a selected text on path.

use std::sync::Arc;

use curvyo_document_core::{CarrierCurve, ObjectSnapshot, ViewTransform};
use curvyo_render_core::{
    DrawList, TransformDecorationInput, TransformGlyphKind, TransformHandleGlyph,
};

use super::text::edit_id;
use super::text_on_path_drag::{DragKind, PathDrag};
use super::{Session, Tool};

/// The most texts a carrier drag previews.
const PREVIEW_CAP: usize = 500;

impl Session {
    /// The blue preview of the texts on the carriers a drag reshapes (criterion 24): the Select
    /// tool's live objects, or the Node tool's dragged paths that differ from the read.
    pub(super) fn text_path_follower_list(
        &self,
        view: ViewTransform,
        objects: &[ObjectSnapshot],
        paths: &[curvyo_document_core::PathSnapshot],
        live: Option<&[ObjectSnapshot]>,
    ) -> DrawList {
        let changed: Vec<ObjectSnapshot> = match live {
            Some(live) => live.to_vec(),
            None if self.tool == Tool::Node => paths
                .iter()
                .filter(|path| {
                    objects.iter().any(
                        |o| matches!(o, ObjectSnapshot::Path(p) if p.id == path.id && p != *path),
                    )
                })
                .map(|path| ObjectSnapshot::Path(path.clone()))
                .collect(),
            None => Vec::new(),
        };
        let followers = self.text_path_followers(objects, &changed);
        if followers.is_empty() {
            return DrawList::default();
        }
        curvyo_render_core::build_live_edit_preview(&followers, view)
    }

    /// The texts whose carriers `changed` replaces (a drag of the carrier, criterion 24), laid out
    /// on the changed carriers.
    fn text_path_followers(
        &self,
        objects: &[ObjectSnapshot],
        changed: &[ObjectSnapshot],
    ) -> Vec<ObjectSnapshot> {
        if changed.is_empty() || self.document_read().clones.texts_on.is_empty() {
            return Vec::new();
        }
        curvyo_document_core::text_path_previews(objects, changed, PREVIEW_CAP)
            .unwrap_or_default()
            .into_iter()
            .map(|mut text| {
                text.outline = self.fonts.lay_out(&text).map(Arc::new);
                ObjectSnapshot::Text(Box::new(text))
            })
            .collect()
    }

    /// The canvas layer of text on path: the carrier lines of the selected texts on path, the
    /// hover outline and preview of pick mode, the dragged text's preview.
    pub(super) fn text_path_draw_list(&self, view: ViewTransform) -> DrawList {
        let mut list = DrawList::default();
        if let Some(pick) = self.text_path.pick {
            if let Some(carrier) = self.text_path.hover
                && let Some(curve) = self
                    .document_read()
                    .leaves
                    .iter()
                    .find(|o| o.id() == carrier.node)
                    .and_then(|o| curvyo_document_core::carrier_curve(o, carrier.outline))
            {
                list.extend(curvyo_render_core::build_carrier_hover(&curve, view));
                if let Some(mut text) = self.read_text(pick.text) {
                    let values = self.put_values(&text, &curve);
                    text.on_path = Some(curvyo_document_core::TextOnPath {
                        carrier,
                        values,
                        curve: Some(Arc::new(curve)),
                        kept: false,
                        kept_curve: None,
                    });
                    text.outline = self.fonts.lay_out(&text).map(Arc::new);
                    list.extend(curvyo_render_core::build_live_edit_preview(
                        &[ObjectSnapshot::Text(Box::new(text))],
                        view,
                    ));
                }
            }
            return list;
        }
        let read = self.document_read();
        let selected: Vec<Arc<CarrierCurve>> = self
            .selection
            .ids()
            .iter()
            .chain(self.text.edit.as_ref().and_then(edit_id).iter())
            .filter_map(|id| {
                read.leaves.iter().find_map(|o| match o {
                    ObjectSnapshot::Text(t) if t.id == *id => {
                        t.on_path.as_ref().and_then(|p| p.curve.clone())
                    }
                    _ => None,
                })
            })
            .collect();
        let curves: Vec<&CarrierCurve> = selected.iter().map(AsRef::as_ref).collect();
        list.extend(curvyo_render_core::build_carrier_lines(&curves, view));
        if let Some(live) = self.text_path_live() {
            list.extend(curvyo_render_core::build_live_edit_preview(
                &[ObjectSnapshot::Text(Box::new(live))],
                view,
            ));
        }
        list
    }

    /// The handles of one selected text on path in place of the Select tool's (criteria 15, 20,
    /// 21): the position handle with its guide, the four corners and, on an ellipse carrier, the
    /// rotate handles; nothing during a drag but the dragged handle.
    pub(super) fn text_path_handles(&self) -> Option<TransformDecorationInput> {
        let text = self.text_path_live().or_else(|| self.sole_text_on_path())?;
        let on = self.on_carrier(&text)?;
        let drag = self.text_path.drag.filter(|d| d.moved);
        let (handle, on_line) = curvyo_ui_core::handle_points(&on.placement);
        let mut input = TransformDecorationInput {
            device_pixel_ratio: self.device_pixel_ratio,
            ..TransformDecorationInput::default()
        };
        // The handle keeps its dragging look while its entry box is open.
        let sliding =
            drag.is_some_and(|d| d.kind == DragKind::Slide) || self.text_path.entry.is_some();
        if drag.is_none_or(|d| d.kind == DragKind::Slide) {
            input.handles.push(TransformHandleGlyph {
                position: handle,
                kind: TransformGlyphKind::Parameter,
                dragging: sliding,
                hovered: false,
            });
            if handle.vector_to(on_line).length() > 1e-9 {
                input.param_guides.push((handle, on_line));
            }
        }
        // No box handles during a drag or while the handle's entry box is open.
        if self.tool == Tool::Select
            && self.text.edit.is_none()
            && drag.is_none()
            && self.text_path.entry.is_none()
        {
            for (position, rotate) in self.box_handles(&text) {
                input.handles.push(TransformHandleGlyph {
                    position,
                    kind: if rotate {
                        TransformGlyphKind::Rotate
                    } else {
                        TransformGlyphKind::Resize
                    },
                    dragging: false,
                    hovered: false,
                });
            }
        }
        if let Some(PathDrag {
            kind: DragKind::Rotate { .. },
            ..
        }) = drag
        {
            input.pivot_marker = text
                .on_path
                .as_ref()
                .and_then(|p| p.curve.as_ref())
                .and_then(|c| c.ellipse)
                .map(|e| e.centre);
        }
        Some(input)
    }
}
