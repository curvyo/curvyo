//! `Session`'s per-frame draw-list assembly: every tool's decorations and
//! previews combined into the one list the host submits. Split out of
//! `session/mod.rs` (`docs/technical-debt.md`, "`Session` is one module
//! past the size limit").

use std::borrow::Cow;
use std::sync::Arc;

use curvyo_render_core::{
    DrawList, TransformDecorationInput, build_artwork_with_guides, build_decorations,
    build_group_draw_list, build_marquee_overlay, build_pen_preview, build_select_draw_list,
    build_transform_draw_list,
};

use curvyo_document_core::{ObjectSnapshot, PathSnapshot, ViewTransform};

use super::reference_view::{reference_box_list, reference_marks_list};
use super::{Session, Tool};

impl Session {
    /// `object` with an editable curve replaced by the Bézier curve it draws: a Spiro from the
    /// memo (a Node drag's live controls solved as they move), a B-spline evaluated; a curve that
    /// cannot be evaluated is drawn through its controls with straight lines, never hidden.
    fn evaluate_for_artwork(&self, object: &mut ObjectSnapshot) {
        use curvyo_document_core::CurveKind;
        if !object
            .drawn_path()
            .is_some_and(|path| path.curve_kind != CurveKind::Bezier)
        {
            return;
        }
        if let ObjectSnapshot::Clone(clone) = object
            && clone.chain.root_outline.curve_kind == CurveKind::Spiro
        {
            let matrix = clone.chain.composed;
            let root = self.spiro_cache.borrow_mut().lookup_with_tolerance(
                &self.document,
                &clone.chain.root_outline,
                curvyo_ui_core::SPIRO_TOLERANCE.as_mm() / curvyo_ui_core::stretch(matrix),
            );
            if let Some(root) = root {
                let mut mapped = root.mapped(matrix);
                mapped.id = clone.resolved.id;
                mapped.style = clone.resolved.style.clone();
                mapped.rotation = clone.resolved.rotation;
                *clone.resolved = mapped;
                clone.chain.root_outline = std::sync::Arc::new(root);
                return;
            }
        }
        if let ObjectSnapshot::Path(path) = object
            && path.curve_kind == CurveKind::Spiro
        {
            let node_drag = self.tool == Tool::Node
                && self
                    .document
                    .path(path.id)
                    .is_some_and(|source| source.anchors != path.anchors);
            let mut cache = self.spiro_cache.borrow_mut();
            let evaluated = if node_drag {
                cache.node_preview(&self.document, path)
            } else {
                cache.lookup(&self.document, path)
            };
            if let Some(evaluated) = evaluated {
                *path = evaluated;
                return;
            }
        }
        match curvyo_ui_core::evaluate_object(object) {
            Ok(evaluated) => *object = evaluated,
            Err(_) => match object {
                ObjectSnapshot::Path(path) => control_polygon(path),
                ObjectSnapshot::Clone(clone) => {
                    control_polygon(&mut clone.resolved);
                    control_polygon(std::sync::Arc::make_mut(&mut clone.chain.root_outline));
                }
                _ => {}
            },
        }
    }

    /// The artwork of `objects`.
    pub(super) fn artwork_list(
        &self,
        objects: &[ObjectSnapshot],
        guides: &std::collections::HashSet<curvyo_document_core::NodeId>,
        view: ViewTransform,
    ) -> DrawList {
        // A pattern fill is drawn as its pieces under the shape's stroke (ADR 0029 §7).
        fn drawn<'o>(session: &Session, part: &'o [ObjectSnapshot]) -> Cow<'o, [ObjectSnapshot]> {
            if curvyo_ui_core::PatternCache::any_pattern(part) {
                {
                    let previewed = session.style.previewed_ids();
                    Cow::Owned(
                        session
                            .patterns
                            .borrow_mut()
                            .expand(part, &|id| previewed.contains(&id)),
                    )
                }
            } else {
                Cow::Borrowed(part)
            }
        }
        // Entries of shapes gone from the read are dropped once, before the pieces expand
        // (ADR 0029 §6).
        self.patterns
            .borrow_mut()
            .retain_only(&objects.iter().map(ObjectSnapshot::id).collect());
        build_artwork_with_guides(&drawn(self, objects), guides, view)
    }

    /// Builds this frame's draw list from the document's current state,
    /// the active view transform, and the node tool's selection/hover —
    /// plus the pen tool's in-progress preview
    /// (`specification.md`'s UX notes) when it is active, every
    /// primitive's own stroke, the Select tool's boxes and handles, and (when a
    /// creation-tool drag is in flight) its live preview outline
    /// (`specs/0003-primitive-shapes/specification.md`, "Live creation
    /// feedback").
    ///
    /// When the node tool has a node/handle drag in flight
    /// (acceptance criteria 8, 9, 10's "update live during the drag"),
    /// `live_node_drag_paths_in` (private: this module's own internal step,
    /// not part of its public surface) substitutes that drag's live,
    /// not-yet-committed position/handle values into the snapshot before
    /// anything downstream ever sees it — `curvyo-render-core` needs no
    /// drag-specific code of its own for this: it already draws whatever
    /// `PathSnapshot` it is handed, so a locally live-overridden one
    /// reshapes the stroke and every decoration exactly as if it had
    /// already committed.
    #[must_use]
    #[allow(clippy::too_many_lines)] // Explicit layer ordering keeps overlays auditable.
    pub fn draw_list(&self) -> DrawList {
        if self.workbench() == super::Workbench::Sketch {
            let mut list = self.sketch_draw_list();
            // Drawing objects selected over the sketch page: the box and handles of `0019`
            // (`specs/0106` criterion 7).
            list.extend(self.sketch_page_drawing_overlay());
            // A sketch row dragged onto the canvas: its ghost over everything (`specs/0099`).
            list.extend(self.row_drag_ghost_list());
            list.extend(self.clone_drop_ghost_list());
            return list;
        }
        let view = self.view();
        // The document is read once per frame and every step below works on
        // that read: reading the objects out of the document is by far the
        // largest cost of a frame with many objects.
        let objects = self.objects();
        // Outside an entered group the artwork recedes; the decorations only see the inside.
        // Another participant's move drag in flight is drawn where it has the objects (`0092`
        // criterion 7).
        let render = self.with_remote_drags(self.render_objects());
        // A reopened tiling hides its instances under the preview (`0069` criterion 34).
        let render: std::sync::Arc<[ObjectSnapshot]> = if self.tiling_open() {
            render
                .iter()
                .filter(|o| !self.tiling_hides(o.id()))
                .cloned()
                .collect()
        } else {
            render
        };
        let live = self.select_live_edit_in(&objects);
        // A Select drag draws what it moves as itself at the pointer (customer 2026-10-27).
        let paths = self.live_node_drag_paths_in(&objects);
        // A Node drag: the linked clones behind it in blue, and whether the link marks stay
        // away (`0049`).
        let (swap, clone_previews, previewing) =
            self.frame_swap(view, &objects, live.as_ref(), &paths);
        // The artwork: every object in tree order, paths and primitives
        // interleaved, each with its fill then its stroke
        // (`specs/0007-stroke-and-fill-styling` criterion 26). The Node
        // tool's live drag reshapes the paths it moves.
        // A segment bend leaves the artwork as committed ("black old"): its blue outline is
        // drawn over it below, and only the decorations read the live paths.
        let mut artwork_objects = if self.tool == Tool::Node && self.live_bend().is_none() {
            Cow::Owned(Self::with_paths(&render, &paths))
        } else {
            Cow::Borrowed(&render[..])
        };
        if !swap.is_empty() {
            swap.apply_to(artwork_objects.to_mut());
        }
        // A text in edit mode is drawn as typed (`specs/0074-text-objects` criterion 3).
        let mut artwork_objects = self.page_drag_artwork(self.with_text_edit(artwork_objects));
        // A panel drag draws the objects in the style being previewed
        // (criterion 36); nothing is written until the release.
        if self.style.is_active() {
            self.style.apply_to(artwork_objects.to_mut());
        }
        if let Some(preview) = self.width_preview_artwork(&artwork_objects) {
            artwork_objects = Cow::Owned(preview);
        }
        // Artwork consumes evaluated curves; the node decorations below continue to use the
        // original editable controls.
        if artwork_objects.iter().any(|object| {
            object
                .drawn_path()
                .is_some_and(|path| path.curve_kind != curvyo_document_core::CurveKind::Bezier)
        }) {
            for object in artwork_objects.to_mut().iter_mut() {
                self.evaluate_for_artwork(object);
            }
        }
        let mut unsupported_width = false;
        let mut width_ids = std::collections::HashSet::new();
        if artwork_objects
            .iter()
            .any(|object| !curvyo_ui_core::profiled_paths(object).is_empty())
        {
            let mut expanded = Vec::with_capacity(artwork_objects.len() + 1);
            for object in artwork_objects.iter() {
                width_ids.extend(
                    curvyo_ui_core::profiled_paths(object)
                        .into_iter()
                        .map(|path| path.id),
                );
                unsupported_width |= super::width_outlines::push_width_expanded(
                    object,
                    &|path| self.width_artwork(path),
                    &mut expanded,
                );
            }
            artwork_objects = Cow::Owned(expanded);
        }
        self.width_outlines.borrow_mut().retain(&width_ids);
        if unsupported_width && !self.unsupported_width_shown.replace(true) {
            *self.unsupported_width_notice.borrow_mut() = Some(
                "A variable stroke has no outline: its path has no length or is out of range."
                    .to_owned(),
            );
        } else if !unsupported_width {
            self.unsupported_width_shown.set(false);
        }
        // The Trace tool washes the images it traces (`specs/0025-raster-trace` criteria 2, 68).
        if self.tool == Tool::Trace {
            self.trace_artwork(artwork_objects.to_mut());
        }
        // Guides in the marking style (`specs/0046-document-shapes` criterion 5).
        let guides = Arc::clone(&self.document_read().guides);
        // "View: Operations" draws the output page's own items by their operations' looks
        // (`0101` criterion 18); "View: Drawn" leaves the artwork as it is.
        let mut list = match self.operation_looked_artwork(&artwork_objects, view) {
            Some((looked, overlay)) => {
                let mut artwork = self.artwork_list(&looked, &guides, view);
                artwork.extend(overlay);
                artwork
            }
            None => self.artwork_list(&artwork_objects, &guides, view),
        };
        // The dimensions over the artwork, following a drag in the same frame (`0070`).
        list.extend(self.dimension_layer(&artwork_objects));
        list.extend(self.kept_point_list());
        list.extend(self.width_handle_overlay());
        // The blue half of a segment bend, over the artwork and under the nodes and handles.
        list.extend(self.bend_preview_list(view, &objects));
        // The Node tool draws the nodes and handles of its editing set only (customer 2026-10-26).
        let control_paths: Vec<_> = paths.iter().map(super::node::control_view).collect();
        let decorated = if self.tool == Tool::Node {
            self.in_node_set(&control_paths)
        } else {
            Cow::Borrowed(&control_paths[..])
        };
        list.extend(build_decorations(
            &decorated,
            view,
            &self.decoration_input(),
        ));
        // The origin axes of an axis-locked move: above the artwork, below the
        // blue outline, the boxes and the handles (criterion 27).
        if let Some(axes) = self.move_axes_in(&objects) {
            list.extend(curvyo_render_core::build_move_axes(view, &axes));
        }
        // The ghost a Select drag leaves at the origin, where the blue outline of
        // `specs/0009-unified-object-editing` criterion 10 was until 2026-10-27: over the
        // artwork, under the boxes and handles.
        if objects.iter().any(|object| {
            object
                .drawn_path()
                .is_some_and(|path| path.curve_kind == curvyo_document_core::CurveKind::Spiro)
        }) {
            let ghost_objects: Vec<_> = self
                .spiro_selection_objects(&objects)
                .into_iter()
                .filter(|object| {
                    !object.drawn_path().is_some_and(|path| {
                        path.curve_kind == curvyo_document_core::CurveKind::Spiro
                    })
                })
                .collect();
            list.extend(swap.ghost(&ghost_objects, view));
        } else {
            list.extend(swap.ghost(&objects, view));
        }
        list.extend(clone_previews);
        // The texts on a carrier the Node tool drags follow it in the same frame (`0080`
        // criterion 24).
        list.extend(self.text_path_follower_list(view, &objects, &paths, live.is_some()));
        // Carrier lines, pick mode's hover and preview, a text dragged along its carrier.
        list.extend(self.text_path_draw_list(view));
        // The Offset tool: the blue hollow result, the box and its four handles
        // (`specs/0038-path-offset` criteria 9, 14, 18).
        list.extend(self.offset_draw_list(view));
        // The open tiling bar: instance outlines, circle path and handles (`0069` criterion 34).
        list.extend(self.tiling_draw_list(view));
        // The Trace tool's hollow preview outline (`specs/0025-raster-trace` criterion 8).
        self.trace_draw_into(view, &mut list);
        // The pale red box of a move drag or the open typed move (`0047` criteria 33, 36): above
        // the blue preview, whose white casing would otherwise cover it where the two coincide
        // (an unrotated object), and below the selection box and the handles.
        let reference = self.reference_decoration();
        list.extend(reference_box_list(view, reference.as_ref()));
        list.extend(self.snap_marker_list());
        let live_objects = Self::live_objects_in(&objects, live.as_ref());
        // A selected dimension has no box and no handles (`0070` UX notes §7).
        let live_objects = self.without_selected_dimensions(live_objects);
        let selection_visuals = if live_objects.iter().any(|object| {
            object
                .drawn_path()
                .is_some_and(|path| path.curve_kind == curvyo_document_core::CurveKind::Spiro)
        }) {
            // A Spiro without solution shows its box around its control polygon.
            Cow::Owned(self.spiro_pick_objects(&live_objects))
        } else {
            Cow::Borrowed(&live_objects[..])
        };
        list.extend(build_select_draw_list(
            view,
            &self.select_decoration_input_in(&selection_visuals),
        ));
        // The marks of a selected clone: over the selection and hover boxes, so a box that
        // coincides with an outline never hides its trace, and under the handles (`0049`).
        list.extend(self.clone_mark_layer(view, &objects, previewing));
        // The red hollow outline of the objects a refused command names: over the clone marks,
        // so a refusal at depth 8 stays visible (`0049` UX re-check).
        list.extend(curvyo_render_core::build_refusal_outlines(
            &self.refusal_objects(&objects),
            view,
        ));
        if let Some(group) = self.group_decoration_input_in(&selection_visuals) {
            list.extend(build_group_draw_list(view, &group));
        }
        // The open tiling bar draws the dashed selection box only: transform handles would
        // collide with the circle handles (`0069` UX review).
        if self.tiling.is_none() {
            list.extend(build_transform_draw_list(
                view,
                &self.select_transform_decoration_input_in(&selection_visuals),
            ));
        }
        // The nine marks of the typed move, above the handles.
        list.extend(reference_marks_list(view, reference.as_ref()));
        // The marquee box or lasso line: above the boxes and handles, the
        // topmost layer of a selection drag.
        if let Some(overlay) = self.marquee_overlay() {
            list.extend(build_marquee_overlay(
                view,
                &overlay,
                self.device_pixel_ratio,
            ));
        }
        list.extend(self.creation_preview_list(view));
        // The text in edit mode: line box, selection, composition and caret (`0074`).
        list.extend(self.text_overlay_list(view, self.text_caret_on()));
        if self.tool != Tool::Node {
            self.spiro_cache.borrow_mut().clear_node_previews();
        }
        if self.tool != Tool::Pen
            || self.pen.mode() != curvyo_ui_core::PenMode::Spiro
            || self.pen.in_progress_nodes().is_none()
        {
            self.spiro_cache.borrow_mut().clear_preview();
        }
        if self.tool == Tool::Pen
            && let Some(nodes) = self.pen.in_progress_nodes()
        {
            // The id this pending anchor would actually get if the
            // gesture ended right now — `peek`, never `mint`: a preview
            // must not advance the minter's own counter out of step with
            // what might still be escaped or turn into a close gesture
            // instead (`AnchorIdMinter::peek`'s own doc comment).
            let pending = self.pointer_position.and_then(|cursor| {
                // Ctrl snaps the handle direction, read live (`0072` criterion 3).
                let handle = self.pen_handle_point(cursor, self.held.ctrl);
                self.pen
                    .pending_anchor(self.minter.peek(), handle, self.drag_threshold())
            });
            if matches!(
                self.pen.mode(),
                curvyo_ui_core::PenMode::BSpline | curvyo_ui_core::PenMode::Spiro
            ) {
                let mut controls = nodes.to_vec();
                let closed = self.pen.mode() == curvyo_ui_core::PenMode::Spiro
                    && nodes.len() >= 3
                    && matches!(
                        self.pen_target(),
                        Some(curvyo_ui_core::PenTarget::Close { .. })
                    );
                if let Some(next) = pending {
                    controls.push(next);
                } else if let Some(point) = self.pen_rubber_band_end()
                    && (self.pen.mode() != curvyo_ui_core::PenMode::Spiro
                        || (!closed && controls.last().is_none_or(|last| last.point != point)))
                {
                    let mut next =
                        curvyo_document_core::NewAnchor::corner(self.minter.peek(), point);
                    next.kind = curvyo_document_core::AnchorKind::Symmetric;
                    controls.push(next);
                }
                let curve = if self.pen.mode() == curvyo_ui_core::PenMode::Spiro {
                    // Controls without a solution near the last curve keep that curve and the
                    // points it goes through (customer 2026-11-11).
                    let shown = self.spiro_cache.borrow_mut().preview(&controls, closed);
                    shown.map(|(curve, through)| {
                        controls = through;
                        curve
                    })
                } else {
                    curvyo_ui_core::evaluate_bspline_nodes(&controls, false).ok()
                };
                if curve.is_some() || self.pen.mode() == curvyo_ui_core::PenMode::Spiro {
                    let kind = if self.pen.mode() == curvyo_ui_core::PenMode::Spiro {
                        curvyo_document_core::CurveKind::Spiro
                    } else {
                        curvyo_document_core::CurveKind::BSpline
                    };
                    list.extend(curvyo_render_core::build_curve_pen_preview(
                        nodes,
                        pending.as_ref(),
                        curvyo_render_core::CurvePreview {
                            curve: curve.as_deref().unwrap_or_default(),
                            kind,
                            controls: &controls,
                        },
                        closed,
                        view,
                        self.document.size(),
                        self.shown_background(),
                    ));
                }
            } else {
                list.extend(build_pen_preview(
                    nodes,
                    self.pen_rubber_band_end(),
                    pending.as_ref(),
                    view,
                    self.document.size(),
                    self.shown_background(),
                ));
            }
        }
        // What a press would continue, join or close onto, before the click (`0034`).
        if self.tool == Tool::Pen {
            list.extend(curvyo_render_core::build_pen_cue(
                &self.pen_cue_data(),
                view,
                self.document.size(),
                self.shown_background(),
            ));
        }
        list.extend(self.row_drag_ghost_list());
        list.extend(self.clone_drop_ghost_list());
        list
    }

    /// A placement's or a create-drag's preview, with the pivot marker where it applies
    /// (`specs/0052-click-to-create-shapes/` criteria 2, 15, 28; `0011` for the drag's marker).
    fn creation_preview_list(&self, view: ViewTransform) -> DrawList {
        let mut list = DrawList::default();
        if let Some(placement) = self.placement() {
            // The placement's preview, turned by its angle, and the pivot marker at A for its
            // whole duration (`0052` criteria 2, 15, 28).
            if let Some(shown) = placement.shown() {
                list.extend(curvyo_render_core::build_shape_live_preview(
                    &shown.shape,
                    shown.rotation,
                    view,
                ));
            }
            list.extend(build_transform_draw_list(
                view,
                &TransformDecorationInput {
                    pivot_marker: Some(placement.a()),
                    ..TransformDecorationInput::default()
                },
            ));
        } else if let Some(preview) = self.live_preview() {
            list.extend(curvyo_render_core::build_shape_live_preview(
                &preview.shape,
                curvyo_document_core::Angle::from_radians(0.0),
                view,
            ));
            // Shift makes the press point the centre: the pivot marker says so
            // (`docs/design-system.md`, "Modifiers in a rectangle or ellipse
            // create-drag").
            if preview.pivot.is_some() {
                list.extend(build_transform_draw_list(
                    view,
                    &TransformDecorationInput {
                        pivot_marker: preview.pivot,
                        ..TransformDecorationInput::default()
                    },
                ));
            }
        }
        list
    }

    /// The blue outline of the segment bend in flight, nothing without one.
    fn bend_preview_list(&self, view: ViewTransform, objects: &[ObjectSnapshot]) -> DrawList {
        let bend_preview = self.bend_preview_objects(objects);
        if bend_preview.is_empty() {
            return DrawList::default();
        }
        curvyo_render_core::build_live_edit_preview(&bend_preview, view)
    }

    /// `objects` with each path replaced by the path of the same id in `paths`
    /// (possibly reshaped by a live drag); a path with no match stays as it is.
    fn with_paths(objects: &[ObjectSnapshot], paths: &[PathSnapshot]) -> Vec<ObjectSnapshot> {
        objects
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Path(original) => ObjectSnapshot::Path(
                    paths
                        .iter()
                        .find(|path| path.id == original.id)
                        .unwrap_or(original)
                        .clone(),
                ),
                ObjectSnapshot::Sketch(_)
                | ObjectSnapshot::Primitive(_)
                | ObjectSnapshot::Clone(_)
                | ObjectSnapshot::Text(_)
                | ObjectSnapshot::Image(_)
                | ObjectSnapshot::Dimension(_) => object.clone(),
            })
            .collect()
    }
}

/// `path` drawn through its controls with straight lines (an editable curve that cannot be
/// evaluated stays visible).
fn control_polygon(path: &mut curvyo_document_core::PathSnapshot) {
    for anchor in &mut path.anchors {
        anchor.handle_in = curvyo_document_core::Vec2::ZERO;
        anchor.handle_out = curvyo_document_core::Vec2::ZERO;
    }
    path.curve_kind = curvyo_document_core::CurveKind::Bezier;
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Point};

    use super::*;

    /// The live drag's paths replace the artwork's by id, whatever their
    /// order, and a path with no match keeps its own geometry.
    #[test]
    fn dragged_paths_replace_objects_by_id_not_by_position() {
        let document = Document::new(1);
        let make = |y: f64, n: u64| {
            document.create_path(
                &[
                    NewAnchor::corner(AnchorId::new(1, n), Point::new(0.0, y)),
                    NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(10.0, y)),
                ],
                false,
            )
        };
        let (first, second) = (make(0.0, 1), make(50.0, 3));
        let objects: Vec<ObjectSnapshot> = [first, second]
            .iter()
            .map(|id| document.object(*id).unwrap())
            .collect();
        let mut moved = document.path(second).unwrap();
        moved.anchors[0].point = Point::new(0.0, 99.0);
        // Only the second path is in the list, and it is listed alone.
        let replaced = Session::with_paths(&objects, std::slice::from_ref(&moved));
        let ObjectSnapshot::Path(a) = &replaced[0] else {
            panic!("a path");
        };
        let ObjectSnapshot::Path(b) = &replaced[1] else {
            panic!("a path");
        };
        assert_eq!(a.anchors[0].point, Point::new(0.0, 0.0), "untouched");
        assert_eq!(b.anchors[0].point, Point::new(0.0, 99.0), "the dragged one");
    }
}
