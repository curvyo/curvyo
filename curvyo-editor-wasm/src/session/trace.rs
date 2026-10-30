//! The Trace tool's session side (`specs/0025-raster-trace` criteria 1 to 3, 7, 12 to 14 and
//! 46): the images it traces, their pixels for the worker, the results it keeps and Create paths
//! in one step. What it draws is `trace_draw.rs`; the trace itself runs in the host's worker
//! (ADR 0023 §5).

use std::sync::Arc;

use curvyo_document_core::{AssetHash, DrawingNode, ImageSnapshot, NodeId, ObjectSnapshot, Point};
use curvyo_image_core::WorkingImage;
use curvyo_render_core::PreviewAnchor;
use curvyo_ui_core::{
    TraceStatus, TraceTarget, TraceWire, hit_test_objects, plan_trace, read_trace_wire,
};

use super::select::DoubleClickHint;
use super::trace_draw::{TraceViewMode, filled_result, preview_anchors};
use super::trace_mesh::MeshCache;
use super::{Session, Tool};

/// The preview of one image (criterion 8): its outlines and what is tessellated from them.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TracePreview {
    pub(super) image: NodeId,
    pub(super) outlines: Vec<Vec<PreviewAnchor>>,
    /// Each outline's box in document millimetres, for the cull.
    pub(super) boxes: Vec<(Point, Point)>,
    /// A stopped job left it out of date: drawn at half strength.
    pub(super) stale: bool,
    /// The result as Create paths would write it, black and filled, for the Trace view
    /// (criterion 51); built once per result.
    pub(super) fill: curvyo_document_core::PathSnapshot,
    /// The tessellated hollow outline, kept across frames (criterion 56).
    pub(super) outline_mesh: MeshCache,
    /// The tessellated filled result of the Trace view, kept across frames (criterion 56).
    pub(super) fill_mesh: MeshCache,
}

/// What the Trace tool keeps between events; nothing of it is in the document.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TraceState {
    /// The image clicked last: the one the preview shows.
    last_clicked: Option<NodeId>,
    pub(super) preview: Option<TracePreview>,
    /// The full-size results Create paths writes, per image.
    results: Vec<(NodeId, TraceWire)>,
    /// The chosen view; Overlay on every entry into the tool (UX notes section 13).
    pub(super) view_mode: TraceViewMode,
    /// `\` is held: the Image view shows until it is released (criterion 51).
    pub(super) peek: bool,
    /// The view is moving (a pan or zoom has not rested yet): the kept meshes are drawn as they
    /// are, whatever the zoom (criterion 56).
    pub(super) moving: bool,
    /// The pipette (`trace_pick.rs`).
    pub(super) pick: super::trace_pick::TracePick,
    /// The mask of the Mask view (`trace_mask.rs`).
    pub(super) mask: super::trace_mask::TraceMaskState,
    /// The reduced copy of the last picture asked for: every preview of a large image traces it
    /// (criteria 10 and 52), and making it costs the main thread tens of milliseconds
    /// (criterion 58).
    reduced: Option<(AssetHash, Arc<WorkingImage>)>,
}

/// The counts of each group Create paths wrote in this session, for its Layers row's tooltip
/// (criteria 30 and 68 a): group, outlines, anchors. Not in the document; kept when the tool is
/// left.
pub(super) type TracedGroups = Vec<(NodeId, usize, usize)>;

/// What the selection is to the Trace tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceSelection {
    /// Nothing is selected.
    Empty,
    /// Something that is not an image is selected.
    Other,
    /// Only images are selected.
    Images,
}

/// What the Trace bar and the Select bar's Trace button read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceView {
    /// What is selected.
    pub selection: TraceSelection,
    /// The selected images, in selection order.
    pub images: Vec<NodeId>,
    /// The image the preview shows: the one clicked last, else the last selected.
    pub shown: Option<NodeId>,
}

/// One image's pixels and pixel map for the worker.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceSource {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight-alpha RGBA8.
    pub rgba: Vec<u8>,
    /// Pixel corners to document millimetres, `[a, b, c, d, e, f]`.
    pub map: [f64; 6],
}

/// What [`Session::create_traced_paths`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceCreateOutcome {
    /// Nothing: not the Trace tool, no image, or a result is missing or has no outline.
    Ignored,
    /// The groups were written in one step.
    Created {
        /// Images traced.
        images: usize,
        /// Outlines written.
        paths: usize,
        /// Anchors written.
        nodes: usize,
    },
}

impl Session {
    /// The selected images when only images are selected.
    pub(super) fn trace_images(&self) -> (TraceSelection, Vec<ImageSnapshot>) {
        let ids = self.selection.ids();
        if ids.is_empty() {
            return (TraceSelection::Empty, Vec::new());
        }
        let objects = self.objects();
        let mut images = Vec::with_capacity(ids.len());
        for id in ids {
            match objects.iter().find(|object| object.id() == *id) {
                Some(ObjectSnapshot::Image(image)) => images.push(image.clone()),
                _ => return (TraceSelection::Other, Vec::new()),
            }
        }
        (TraceSelection::Images, images)
    }

    /// What the Trace bar shows (criteria 1 to 4, 46).
    #[must_use]
    pub fn trace_view(&self) -> TraceView {
        let (selection, images) = self.trace_images();
        let images: Vec<NodeId> = images.iter().map(|image| image.id).collect();
        let shown = self
            .trace
            .last_clicked
            .filter(|id| images.contains(id))
            .or_else(|| images.last().copied());
        TraceView {
            selection,
            images,
            shown,
        }
    }

    /// Whether the Select bar shows the Trace button: the Select tool with only images selected
    /// (criterion 1).
    #[must_use]
    pub fn trace_available(&self) -> bool {
        self.tool == Tool::Select && self.trace_images().0 == TraceSelection::Images
    }

    /// The working image of `image` (a reduced copy with `reduced`, criterion 10) and its pixel
    /// map, for the worker; `None` when it is not a selected image or its picture is unreadable.
    pub fn trace_source(&mut self, image: NodeId, reduced: bool) -> Option<TraceSource> {
        let (_, images) = self.trace_images();
        let snapshot = images.into_iter().find(|candidate| candidate.id == image)?;
        let working = self.working_image(&snapshot.asset)?;
        let picture = if reduced {
            match &self.trace.reduced {
                Some((asset, copy)) if *asset == snapshot.asset => Arc::clone(copy),
                _ => {
                    let copy =
                        Arc::new(working.reduced_copy(curvyo_image_core::TRACE_PREVIEW_SIDE));
                    self.trace.reduced = Some((snapshot.asset.clone(), Arc::clone(&copy)));
                    copy
                }
            }
        } else {
            working
        };
        Some(TraceSource {
            width: picture.width,
            height: picture.height,
            rgba: picture.pixels.clone(),
            map: snapshot.pixel_map(picture.width, picture.height).to_array(),
        })
    }

    /// Shows the worker's result `wire` for `image` as the preview (criterion 8); `false` when the
    /// numbers are not a result.
    pub fn set_trace_preview(&mut self, image: NodeId, wire: &[f64]) -> bool {
        let Some(read) = read_trace_wire(wire) else {
            return false;
        };
        let mut outlines = Vec::with_capacity(read.outlines.len());
        let mut boxes = Vec::with_capacity(read.outlines.len());
        for path in &read.outlines {
            let b = kurbo::Shape::bounding_box(path);
            boxes.push((Point::new(b.x0, b.y0), Point::new(b.x1, b.y1)));
            outlines.push(preview_anchors(path));
        }
        let fill = filled_result(image, &outlines);
        self.trace.preview = Some(TracePreview {
            image,
            outlines,
            boxes,
            stale: false,
            fill,
            outline_mesh: MeshCache::default(),
            fill_mesh: MeshCache::default(),
        });
        true
    }

    /// Marks the preview and the mask out of date (a stopped job) or current again.
    pub fn set_trace_preview_stale(&mut self, stale: bool) {
        if let Some(preview) = &mut self.trace.preview {
            preview.stale = stale;
        }
        self.set_trace_mask_stale(stale);
    }

    /// Keeps the worker's full-size result `wire` for `image` for Create paths; `false` when the
    /// numbers are not a result.
    pub fn set_trace_result(&mut self, image: NodeId, wire: &[f64]) -> bool {
        let Some(read) = read_trace_wire(wire) else {
            return false;
        };
        self.trace.results.retain(|(id, _)| *id != image);
        self.trace.results.push((image, read));
        true
    }

    /// Forgets the full-size results (a value changed).
    pub fn clear_trace_results(&mut self) {
        self.trace.results.clear();
    }

    /// Drops everything the tool keeps (leaving it, criterion 7).
    pub(super) fn leave_trace_tool(&mut self) {
        self.trace = TraceState::default();
    }

    /// Create paths (criteria 13, 14, 46): for every selected image with a full-size result, one
    /// group "Trace of" and the image name with one black compound path directly above it, all in one step
    /// "Trace image"; the groups are selected and the Select tool is chosen. The images stay.
    pub fn create_traced_paths(&mut self) -> TraceCreateOutcome {
        self.recording(|this| -> TraceCreateOutcome {
            if this.tool != Tool::Trace {
                return TraceCreateOutcome::Ignored;
            }
            let (_, images) = this.trace_images();
            if images.is_empty() {
                return TraceCreateOutcome::Ignored;
            }
            let names = Arc::clone(&this.document_read().nodes);
            let mut targets = Vec::with_capacity(images.len());
            for image in &images {
                let Some((_, wire)) = this.trace.results.iter().find(|(id, _)| *id == image.id)
                else {
                    return TraceCreateOutcome::Ignored;
                };
                if wire.status != TraceStatus::Traced {
                    return TraceCreateOutcome::Ignored;
                }
                let name = names
                    .iter()
                    .find(|node| node.id == image.id)
                    .and_then(|node| node.registers.name.clone());
                targets.push(TraceTarget {
                    image: image.id,
                    name,
                    outlines: wire.outlines.clone(),
                });
            }
            let plan = plan_trace(&targets, &mut this.minter);
            if plan.items.is_empty() {
                return TraceCreateOutcome::Ignored;
            }
            let Ok(groups) = this
                .document
                .insert_drawing_above(&plan.items, &plan.anchor_ids)
            else {
                // invariant: the images were just read and the ids are fresh.
                debug_assert!(false, "a planned trace could not be written");
                return TraceCreateOutcome::Ignored;
            };
            let leaves: Vec<NodeId> = groups
                .iter()
                .flat_map(|&group| this.document.leaves_of(group))
                .collect();
            let images = plan.items.len();
            for (&group, (_, node)) in groups.iter().zip(&plan.items) {
                let (paths, nodes) = drawn_counts(node);
                this.traced_groups.retain(|(id, _, _)| *id != group);
                this.traced_groups.push((group, paths, nodes));
            }
            this.set_tool(Tool::Select);
            this.selection.set(&leaves);
            this.hovered_object = None;
            TraceCreateOutcome::Created {
                images,
                paths: plan.paths,
                nodes: plan.nodes,
            }
        })
    }

    /// A press in the Trace tool selects what it hits, as a click of the Select tool does, and
    /// nothing else; on empty canvas it clears the selection (criterion 3).
    pub(super) fn trace_pointer_down(&mut self, point: Point, shift: bool) {
        let pickable = self.pickable_objects();
        let tolerance = self.object_tolerance();
        let Some(&hit) = hit_test_objects(&pickable, point, tolerance).first() else {
            if !shift {
                self.selection.clear();
            }
            return;
        };
        let units = self.units();
        let members = units.leaves_with(&hit);
        if shift {
            self.selection.toggle_all(members);
        } else {
            self.selection.set(members);
        }
        if self.selection.contains(hit) {
            self.trace.last_clicked = Some(hit);
        }
    }

    /// A double-click on a traced image asks the host to create the paths (criterion 12).
    pub(super) fn trace_double_click(&mut self, point: Point) -> DoubleClickHint {
        let pickable = self.pickable_objects();
        let tolerance = self.object_tolerance();
        let hit = hit_test_objects(&pickable, point, tolerance)
            .first()
            .copied();
        let view = self.trace_view();
        if hit.is_some_and(|hit| view.images.contains(&hit)) {
            DoubleClickHint::CreateTrace
        } else {
            DoubleClickHint::None
        }
    }

    /// The outlines and anchors Create paths wrote into `group` in this session, if it did.
    #[must_use]
    pub fn traced_counts(&self, group: NodeId) -> Option<(usize, usize)> {
        self.traced_groups
            .iter()
            .find(|(id, _, _)| *id == group)
            .map(|&(_, paths, nodes)| (paths, nodes))
    }
}

/// The outlines and anchors of a planned trace group's compound path.
fn drawn_counts(node: &DrawingNode) -> (usize, usize) {
    let DrawingNode::Group { children, .. } = node else {
        return (0, 0);
    };
    children
        .iter()
        .filter_map(|child| match child {
            DrawingNode::Path { outlines, .. } => Some(outlines),
            _ => None,
        })
        .flatten()
        .fold((0, 0), |(paths, nodes), outline| {
            (paths + 1, nodes + outline.anchors.len())
        })
}
