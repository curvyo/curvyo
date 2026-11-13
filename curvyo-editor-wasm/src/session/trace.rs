//! The Trace tool's session side (`specs/0025-raster-trace` criteria 1 to 3, 7, 12 to 14 and
//! 46): the images it traces, their pixels for the worker, the results it keeps and Create paths
//! in one step. What it draws is `trace_draw.rs`; the trace itself runs in the host's worker
//! (ADR 0023 §5).

use std::sync::Arc;

use curvyo_document_core::{AssetHash, DrawingNode, ImageSnapshot, NodeId, ObjectSnapshot, Point};
use curvyo_image_core::WorkingImage;
use curvyo_render_core::PreviewAnchor;
use curvyo_ui_core::{TraceTarget, hit_test_objects, plan_trace, read_trace_wire};
use vectrace::Output;
use vectrace::shapes::ShapeRegion;
use vectrace::wire::{Layer, Palette, Status};

use super::select::DoubleClickHint;
use super::trace_colours::with_leftovers;
use super::trace_draw::preview_anchors;
use super::trace_mesh::MeshCache;
use super::trace_text::leftovers_of;
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
    /// A multicolour result's layers and palette (`specs/0089` slice M); empty for one colour.
    pub(super) layers: Vec<Layer>,
    pub(super) palette: Vec<Palette>,
    /// The layers the drawn paths fill in order: a multicolour result's layers, then one per
    /// text line or text in its fill (`specs/0091` slice 2); empty for one colour.
    pub(super) fill_layers: Vec<Layer>,
    /// Colour regions joined to a neighbour.
    pub(super) joined: usize,
    /// Text lines set and found (`specs/0091` UX notes §4).
    pub(super) text_counts: (usize, usize),
    /// Shapes set (`specs/0088` Part B, UX notes B3): the primitives of the recognised regions.
    pub(super) shape_count: usize,
    /// Shapes kept whole across the mask edge after a kernel refusal (`specs/0094`).
    pub(super) unclipped: usize,
    /// The tessellated hollow outline, kept across frames (criterion 56).
    pub(super) outline_mesh: MeshCache,
}

/// What the Trace tool keeps between events; nothing of it is in the document.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TraceState {
    /// The image clicked last: the one the preview shows.
    last_clicked: Option<NodeId>,
    pub(super) preview: Option<TracePreview>,
    /// The full-size results Create paths writes, per image.
    pub(super) results: Vec<(NodeId, Output)>,
    /// The view is moving (a pan or zoom has not rested yet): the kept meshes are drawn as they
    /// are, whatever the zoom (criterion 56).
    pub(super) moving: bool,
    /// The pipette (`trace_pick.rs`).
    pub(super) pick: super::trace_pick::TracePick,
    /// The reduced copy of the last picture asked for: every preview of a large image traces it
    /// (criteria 10 and 52), and making it costs the main thread tens of milliseconds
    /// (criterion 58).
    reduced: Option<(AssetHash, Arc<WorkingImage>)>,
    /// Outside is chosen in the Mask item for the implicit part of an untouched image (`specs/0094`
    /// criterion 17: Inside on every entry; ADR 0061 §3).
    pub(super) mask_outside: bool,
    /// The region of the active part, made once per change of its regions (`trace_region.rs`).
    pub(super) region: std::cell::RefCell<Option<super::trace_region::RegionCache>>,
    /// The parts of a composite trace (`trace_parts.rs`, `specs/0094` slice 2).
    pub(super) parts: super::trace_parts::TraceParts,
    /// The mask's region tools, selection and drag (`trace_mask_tool.rs`, `specs/0114`).
    pub(super) mask: super::trace_mask_tool::TraceMaskTool,
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
    /// The selection holds images or image-root clones; every other object in it is passed
    /// over (`specs/0114` criterion 21).
    Images,
}

/// What the Trace bar and the Select bar's Trace button read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceView {
    /// What is selected.
    pub selection: TraceSelection,
    /// The selected images and image-root clones, in selection order.
    pub images: Vec<NodeId>,
    /// The image the preview shows: the one clicked last, else the last selected.
    pub shown: Option<NodeId>,
    /// The active part's stored regions (`specs/0114`).
    pub regions: usize,
    /// The regions enclose no area of the image: "0 paths: the mask is outside the image."
    /// (`specs/0094` criterion 7).
    pub mask_outside_image: bool,
    /// The kernel refused to combine the regions: "0 paths: the mask paths could not be
    /// combined."
    pub mask_failed: bool,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceCreateOutcome {
    /// Nothing: not the Trace tool, no image, or a result is missing or has no outline.
    #[default]
    Ignored,
    /// The groups were written in one step.
    Created {
        /// Images traced.
        images: usize,
        /// Outlines written.
        paths: usize,
        /// Anchors written.
        nodes: usize,
        /// Colour paths written (multicolour, `specs/0089` slice M); 0 for black paths.
        colours: usize,
    },
}

impl Session {
    /// The images of the selection and the image-root clones in it (ADR 0061 §8, CLONE-MASK A),
    /// in selection order; a clone stands as its picture under its composed matrix, with the
    /// clone's id. Every other selected object is passed over (`specs/0114` criterion 21).
    pub(super) fn trace_images(&self) -> (TraceSelection, Vec<ImageSnapshot>) {
        let ids = self.selection.ids();
        if ids.is_empty() {
            return (TraceSelection::Empty, Vec::new());
        }
        let objects = self.objects();
        let placed = self.placement_ids();
        let images: Vec<ImageSnapshot> = ids
            .iter()
            // A placed page's frame object is no picture (`specs/0100` criterion 17).
            .filter(|id| !placed.contains(id))
            .filter_map(|id| objects.iter().find(|object| object.id() == *id))
            .filter_map(traced_picture)
            .collect();
        if images.is_empty() {
            (TraceSelection::Other, Vec::new())
        } else {
            (TraceSelection::Images, images)
        }
    }

    /// The node whose registers hold the trace mask of the traced picture `image`: the root
    /// image of an image-root clone, else the image itself (ADR 0061 §8).
    pub(super) fn trace_mask_owner(&self, image: NodeId) -> NodeId {
        self.objects()
            .iter()
            .find_map(|object| match object {
                ObjectSnapshot::Clone(clone) if clone.id() == image => Some(clone.chain.root),
                _ => None,
            })
            .unwrap_or(image)
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
        let regions = self
            .trace_active_mask_part()
            .filter(|_| selection == TraceSelection::Images && images.len() == 1)
            .map_or(0, |part| part.regions.len());
        let masked = regions > 0;
        let mask_failed = masked && self.trace_mask_failure().is_some();
        let mask_outside_image = masked && !mask_failed && !self.trace_mask_meets_image();
        TraceView {
            selection,
            images,
            shown,
            regions,
            mask_outside_image,
            mask_failed,
        }
    }

    /// Whether the Select bar shows the Trace button: the Select tool with an image or an
    /// image-root clone in the selection (criterion 1, `specs/0114` UX notes §8).
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
        self.show_trace_read(image, read);
        true
    }

    /// Shows the result `read` as the preview of `image`.
    pub(super) fn show_trace_read(&mut self, image: NodeId, read: Output) {
        // `specs/0091`: until the lines are fitted, their own outlines stand where their texts
        // will be, in the texts' fills.
        // A line whose fill the maker excluded is left out (slice 2).
        let shown = || read.texts.lines.iter().filter(|l| !l.excluded);
        let paths: Vec<kurbo::BezPath> = read
            .outlines
            .iter()
            .chain(shown().flat_map(|l| &l.outlines))
            .cloned()
            .collect();
        let extra: Vec<Layer> = shown()
            .map(|l| super::trace_colours::text_layer(l.fill, l.outlines.len()))
            .collect();
        self.show_trace_preview(image, read, &paths, &extra, false);
    }

    /// Shows `paths` (document millimetres) as the preview of `image` with the palette, layers
    /// and counts of `read`, the paths past its layers filled by the `extra` layers (a
    /// multicolour result's texts), out of date with `stale`.
    pub(super) fn show_trace_preview(
        &mut self,
        image: NodeId,
        read: Output,
        paths: &[kurbo::BezPath],
        extra: &[Layer],
        stale: bool,
    ) {
        let mut outlines = Vec::with_capacity(paths.len());
        let mut boxes = Vec::with_capacity(paths.len());
        for path in paths {
            let b = kurbo::Shape::bounding_box(path);
            boxes.push((Point::new(b.x0, b.y0), Point::new(b.x1, b.y1)));
            outlines.push(preview_anchors(path));
        }
        let fill_layers: Vec<Layer> = if read.layers.is_empty() {
            Vec::new()
        } else {
            read.layers.iter().chain(extra).copied().collect()
        };
        self.trace.preview = Some(TracePreview {
            image,
            outlines,
            boxes,
            stale,
            layers: read.layers,
            palette: read.palette,
            fill_layers,
            joined: read.joined,
            text_counts: (read.texts.lines.len(), read.texts.found),
            shape_count: read.shapes.iter().map(ShapeRegion::primitives).sum(),
            unclipped: read.unclipped,
            outline_mesh: MeshCache::default(),
        });
    }

    /// Marks the preview out of date (a stopped job) or current again.
    pub fn set_trace_preview_stale(&mut self, stale: bool) {
        if let Some(preview) = &mut self.trace.preview {
            preview.stale = stale;
        }
    }

    /// Keeps the worker's full-size result `wire` for `image` for Create paths; `false` when the
    /// numbers are not a result.
    pub fn set_trace_result(&mut self, image: NodeId, wire: &[f64]) -> bool {
        let Some(read) = read_trace_wire(wire) else {
            return false;
        };
        self.keep_trace_result(image, &read);
        true
    }

    /// Keeps the full-size result `read` of `image` for Create paths.
    pub(super) fn keep_trace_result(&mut self, image: NodeId, read: &Output) {
        self.trace.results.retain(|(id, _)| *id != image);
        self.trace.results.push((image, read.clone()));
        self.keep_trace_fit(image, read);
    }

    /// The text lines of the shown preview: set and found (`specs/0091` UX notes §4).
    #[must_use]
    pub fn trace_text_counts(&self) -> (usize, usize) {
        self.trace
            .preview
            .as_ref()
            .map_or((0, 0), |p| p.text_counts)
    }

    /// The muted notice of the shown preview's shapes kept whole across the mask edge
    /// (`specs/0094`): "1 shape crosses the mask edge unclipped.", or empty.
    #[must_use]
    pub fn trace_unclipped_notice(&self) -> String {
        curvyo_ui_core::unclipped_notice(self.trace.preview.as_ref().map_or(0, |p| p.unclipped))
    }

    /// The shapes the shown preview sets (`specs/0088` Part B, UX notes B3).
    #[must_use]
    pub fn trace_shape_count(&self) -> usize {
        self.trace.preview.as_ref().map_or(0, |p| p.shape_count)
    }

    /// Forgets the full-size results (a value changed).
    pub fn clear_trace_results(&mut self) {
        self.trace.results.clear();
        self.drop_trace_fit();
    }

    /// Drops everything the tool keeps (leaving it, criterion 7).
    pub(super) fn leave_trace_tool(&mut self) {
        // A lasso in progress is closed, not lost (customer decision 4 of 2026-11-13).
        self.close_trace_lasso();
        self.trace = TraceState::default();
        self.drop_trace_fit();
    }

    /// Create paths (criteria 13, 14, 46; `specs/0089` criterion 29): for every selected image
    /// with a full-size result, one group "Trace of" and the image name with one black compound
    /// path (or one filled compound path per colour) directly above it, all in one step
    /// "Trace image"; the groups are selected and the Select tool is chosen. The images stay.
    pub fn create_traced_paths(&mut self) -> TraceCreateOutcome {
        // The Trace button closes a lasso in progress first (UX notes §6); the host traced
        // after it closed.
        self.close_trace_lasso();
        self.recording(|this| -> TraceCreateOutcome {
            if this.tool != Tool::Trace {
                return TraceCreateOutcome::Ignored;
            }
            let (_, images) = this.trace_images();
            if images.is_empty() {
                return TraceCreateOutcome::Ignored;
            }
            let names = Arc::clone(&this.document_read().nodes);
            let fitted = this.trace_text.fitted.clone().unwrap_or_default();
            let mut targets = Vec::with_capacity(images.len());
            for image in &images {
                let Some((_, wire)) = this.trace.results.iter().find(|(id, _)| *id == image.id)
                else {
                    return TraceCreateOutcome::Ignored;
                };
                if wire.status != Status::Traced {
                    return TraceCreateOutcome::Ignored;
                }
                let name = names
                    .iter()
                    .find(|node| node.id == image.id)
                    .and_then(|node| node.registers.name.clone());
                // `specs/0091` decision 6: the lines' outlines no text replaces join the paths
                // (in Multicolour the layer of their fill).
                let (outlines, colours, places) = with_leftovers(
                    &wire.outlines,
                    &wire.layers,
                    &leftovers_of(&fitted, image.id, wire),
                );
                targets.push(TraceTarget {
                    image: image.id,
                    name,
                    outlines,
                    colours,
                    shapes: shapes_at(&wire.shapes, &places),
                });
            }
            let plan = plan_trace(&targets, &mut this.minter);
            let has_texts = fitted.iter().any(|f| !f.texts.is_empty());
            if plan.items.is_empty() && !has_texts {
                return TraceCreateOutcome::Ignored;
            }
            let written =
                this.document
                    .batch(curvyo_document_core::TRACE_IMAGE_COMMIT_LABEL, |document| {
                        let groups = if plan.items.is_empty() {
                            Vec::new()
                        } else {
                            document
                                .insert_drawing_above(&plan.items, &plan.anchor_ids)
                                .ok()?
                        };
                        let above: Vec<(NodeId, NodeId)> = images
                            .iter()
                            .map(|image| {
                                let group = plan
                                    .items
                                    .iter()
                                    .zip(&groups)
                                    .find(|((id, _), _)| *id == image.id)
                                    .map_or(image.id, |(_, &g)| g);
                                (image.id, group)
                            })
                            .collect();
                        let texts =
                            super::trace_text_write::write_texts(document, &fitted, &above)?;
                        Some((groups, texts))
                    });
            let Some((groups, texts)) = written else {
                // invariant: the images were just read and the ids are fresh; a failed write is
                // reverted by the history as a whole.
                debug_assert!(false, "a planned trace could not be written");
                return TraceCreateOutcome::Ignored;
            };
            let mut leaves: Vec<NodeId> = groups
                .iter()
                .flat_map(|&group| this.document.leaves_of(group))
                .collect();
            leaves.extend(texts.iter().map(|t| t.0));
            let images = plan.items.len();
            for (&group, (_, node)) in groups.iter().zip(&plan.items) {
                let (paths, nodes) = drawn_counts(node);
                this.traced_groups.retain(|(id, _, _)| *id != group);
                this.traced_groups.push((group, paths, nodes));
            }
            this.set_tool(Tool::Select);
            this.selection.set(&leaves);
            this.hovered_object = None;
            this.record_trace_texts(&texts);
            TraceCreateOutcome::Created {
                images,
                paths: plan.paths,
                nodes: plan.nodes,
                colours: plan.colours,
            }
        })
    }

    /// A press in the Trace tool: the mask takes it (`specs/0114`), else it selects what it hits,
    /// as a click of the Select tool does, and nothing else; on empty canvas it clears the
    /// selection (criterion 3).
    pub(super) fn trace_pointer_down(&mut self, point: Point, shift: bool) {
        if self.trace_mask_press(point) {
            return;
        }
        // An object out of context is no operand (`specs/0106` criterion 10).
        let pickable = self.edit_targets();
        let tolerance = self.object_tolerance();
        let Some(&hit) = hit_test_objects(&pickable, point, tolerance).first() else {
            if !shift {
                self.selection.clear();
            }
            return;
        };
        self.trace_select_hit(hit, shift);
    }

    /// Selects `hit` with its group, or with `toggle` (Shift+click) adds or removes them.
    pub(super) fn trace_select_hit(&mut self, hit: NodeId, toggle: bool) {
        let units = self.units();
        let members = units.leaves_with(&hit);
        if toggle {
            self.selection.toggle_all(members);
        } else {
            self.selection.set(members);
        }
        if self.selection.contains(hit) {
            self.trace.last_clicked = Some(hit);
        }
    }

    /// A double-click on a traced image asks the host to create the paths (criterion 12); with
    /// a region tool pressed it never does: it finishes a lasso in progress (`specs/0114`
    /// criterion 4, UX notes §6).
    pub(super) fn trace_double_click(&mut self, point: Point) -> DoubleClickHint {
        if self.trace_mask_double_click() {
            return DoubleClickHint::None;
        }
        // An object out of context is no operand (`specs/0106` criterion 10).
        let pickable = self.edit_targets();
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

/// The picture `object` stands for in the Trace tool: an image, or an image-root clone's picture
/// under its composed matrix with the clone's id.
pub(super) fn traced_picture(object: &ObjectSnapshot) -> Option<ImageSnapshot> {
    match object {
        ObjectSnapshot::Image(image) => Some(image.clone()),
        ObjectSnapshot::Clone(clone) => match clone.chain.picture.as_deref() {
            Some([ObjectSnapshot::Image(image)]) if image.id == clone.chain.root => {
                let mut shown = image.clone();
                shown.id = clone.id();
                Some(shown)
            }
            _ => None,
        },
        _ => None,
    }
}

/// The recognised shapes of a result (`specs/0088` Part B) at their outlines' new `places`.
fn shapes_at(shapes: &[ShapeRegion], places: &[usize]) -> Vec<ShapeRegion> {
    shapes
        .iter()
        .map(|region| ShapeRegion {
            outlines: region.outlines.iter().map(|&i| places[i]).collect(),
            shapes: region.shapes.clone(),
            band: region.band,
            bands: region.bands.clone(),
        })
        .collect()
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
