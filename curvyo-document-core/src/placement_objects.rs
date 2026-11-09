//! Placed pages as objects of the one selection (ADR 0045 §7, `specs/0100-bring-content-in`
//! criteria 17 to 19): each reads as a frame object (the unit square mapped onto the source
//! frame under its global matrix), and the object writers hand a placement id to the placement
//! commands, so a move, turn, scale, skew, flip, typed value, align, distribute, copy or delete
//! writes only its `placement_link`.

use std::collections::HashSet;

use crate::affine::Affine;
use crate::assets::AssetHash;
use crate::document::Document;
use crate::image_model::ImageSnapshot;
use crate::manufacturing::ManufacturingError;
use crate::objects::{CopySource, ObjectEditError};
use crate::path_model::NodeId;
use crate::placement_commands::PlacedPage;
use crate::primitive_model::ObjectSnapshot;
use crate::units::Vec2;

/// The copies of placements made by a Duplicate, `(source, copy)`, and the other sources.
type PlacementCopies = (Vec<(NodeId, NodeId)>, Vec<CopySource>);

/// The asset a placed page's frame object names: no picture is stored under it.
const FRAME_ASSET: &[u8] = b"curvyo placed page frame";

/// The map from the unit square onto the source frame of `placed` in the plane.
fn unit_map(placed: &PlacedPage) -> Option<Affine> {
    let source = placed.source.as_ref()?;
    Some(
        placed
            .global_map()?
            .compose(Affine::translation(Vec2::new(
                source.origin.x,
                source.origin.y,
            )))
            .compose(Affine::scale(
                source.size.width.as_mm(),
                source.size.height.as_mm(),
            )),
    )
}

fn refused(error: &ManufacturingError) -> ObjectEditError {
    match error {
        ManufacturingError::OffPage => ObjectEditError::OffPage,
        _ => ObjectEditError::NoSuchObject,
    }
}

impl Document {
    /// Every placed page whose source exists as a frame object: an image-shaped object with the
    /// placement's id whose frame is the source frame under its matrix. Selection, hit, box,
    /// handles and the transform gestures read it; nothing draws it as a picture.
    #[must_use]
    pub fn placed_page_objects(&self) -> Vec<ObjectSnapshot> {
        let asset = AssetHash::of(FRAME_ASSET);
        self.placed_pages()
            .iter()
            .filter_map(|placed| {
                Some(ObjectSnapshot::Image(ImageSnapshot::new(
                    placed.id,
                    asset.clone(),
                    unit_map(placed)?,
                )))
            })
            .collect()
    }

    /// The frame object of placement `id`, `None` when it is not a live placement of a live page.
    #[must_use]
    pub fn placed_page_object(&self, id: NodeId) -> Option<ObjectSnapshot> {
        let placed = self.placed_page(id)?;
        Some(ObjectSnapshot::Image(ImageSnapshot::new(
            id,
            AssetHash::of(FRAME_ASSET),
            unit_map(&placed)?,
        )))
    }

    /// Whether `id` is a live placement.
    #[must_use]
    pub fn is_placement(&self, id: NodeId) -> bool {
        self.placed_pages().iter().any(|p| p.id == id)
    }

    /// `ids` split into the placements and the rest, each in their order.
    pub(crate) fn split_placements(&self, ids: &[NodeId]) -> (Vec<NodeId>, Vec<NodeId>) {
        let placed: HashSet<NodeId> = self.placed_pages().iter().map(|p| p.id).collect();
        if placed.is_empty() {
            return (Vec::new(), ids.to_vec());
        }
        ids.iter().partition(|id| placed.contains(id))
    }

    /// The document map that turns the frame object of placement `id` into `result`.
    fn map_to(&self, id: NodeId, result: &ObjectSnapshot) -> Option<Affine> {
        let ObjectSnapshot::Image(image) = result else {
            return None;
        };
        let placed = self.placed_page(id)?;
        Some(image.placement.compose(unit_map(&placed)?.inverse()?))
    }

    /// Maps the placements of `moves`, inside the caller's step; refused whole when one would
    /// leave its laser page (criterion 18), which the session reads for its notice.
    pub(crate) fn move_placements(
        &self,
        moves: &[(NodeId, Affine)],
        label: &str,
    ) -> Result<(), ObjectEditError> {
        let result = self
            .transform_placements(moves, label)
            .map_err(|error| refused(&error));
        if result == Err(ObjectEditError::OffPage) {
            self.placement_refusal.set(true);
        }
        result
    }

    /// The writes of `results` that are placements' frame objects, inside the caller's step, and
    /// the remaining results.
    pub(crate) fn transform_placement_results(
        &self,
        results: &[ObjectSnapshot],
        label: &str,
    ) -> Result<Vec<ObjectSnapshot>, ObjectEditError> {
        let ids: Vec<NodeId> = results.iter().map(ObjectSnapshot::id).collect();
        let (placed, _) = self.split_placements(&ids);
        if placed.is_empty() {
            return Ok(results.to_vec());
        }
        let moves = results
            .iter()
            .filter(|r| placed.contains(&r.id()))
            .map(|r| {
                self.map_to(r.id(), r)
                    .map(|t| (r.id(), t))
                    .ok_or(ObjectEditError::NoSuchObject)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.move_placements(&moves, label)?;
        Ok(results
            .iter()
            .filter(|r| !placed.contains(&r.id()))
            .cloned()
            .collect())
    }

    /// The copies of the placements among `sources`, moved by `offset`, inside the caller's
    /// step, each with its source's id; and the remaining sources.
    pub(crate) fn duplicate_placement_sources(
        &self,
        sources: &[CopySource],
        offset: Vec2,
    ) -> Result<PlacementCopies, ObjectEditError> {
        let ids: Vec<NodeId> = sources.iter().map(|s| s.id).collect();
        let (placed, _) = self.split_placements(&ids);
        if placed.is_empty() {
            return Ok((Vec::new(), sources.to_vec()));
        }
        let copies = self
            .duplicate_placements(&placed, offset, "duplicate_objects")
            .map_err(|error| {
                let error = refused(&error);
                if error == ObjectEditError::OffPage {
                    self.placement_refusal.set(true);
                }
                error
            })?;
        let rest = sources
            .iter()
            .filter(|s| !placed.contains(&s.id))
            .cloned()
            .collect();
        Ok((placed.into_iter().zip(copies).collect(), rest))
    }

    /// Whether a writer refused a placed page's move off its laser page since the last call
    /// (criterion 18): the session shows the notice.
    pub fn take_placement_refusal(&self) -> bool {
        self.placement_refusal.take()
    }

    /// Duplicate of `sources` that hold placements: their copies on their own laser pages and
    /// the other sources' copies, in one step, in the order of `sources`; `None` when no source
    /// is a placement.
    pub(crate) fn duplicate_with_placements(
        &self,
        sources: &[CopySource],
        offset: Vec2,
        text_shifts: &std::collections::HashMap<NodeId, crate::units::Length>,
        target_page: Option<&str>,
    ) -> Option<Result<Vec<NodeId>, ObjectEditError>> {
        let ids: Vec<NodeId> = sources.iter().map(|s| s.id).collect();
        if self.split_placements(&ids).0.is_empty() {
            return None;
        }
        Some(self.batch("duplicate_objects", |d| {
            let (copies, rest) = d.duplicate_placement_sources(sources, offset)?;
            let others = if rest.is_empty() {
                Vec::new()
            } else {
                d.duplicate_objects_in_page(&rest, offset, text_shifts, target_page)?
            };
            let mut others = others.into_iter();
            Ok(ids
                .iter()
                .filter_map(|id| {
                    copies
                        .iter()
                        .find(|(source, _)| source == id)
                        .map(|(_, copy)| *copy)
                        .or_else(|| others.next())
                })
                .collect())
        }))
    }

    /// Transform of `results` that hold placed pages' frame objects: their links and the other
    /// results, in one step; `None` when no result is a placed page.
    pub(crate) fn transform_with_placements(
        &self,
        results: &[ObjectSnapshot],
        write_stroke_width: bool,
    ) -> Option<Result<(), ObjectEditError>> {
        let ids: Vec<NodeId> = results.iter().map(ObjectSnapshot::id).collect();
        if self.split_placements(&ids).0.is_empty() {
            return None;
        }
        Some(self.batch("transform_objects", |d| {
            let rest = d.transform_placement_results(results, "transform_objects")?;
            if rest.is_empty() {
                Ok(())
            } else {
                d.transform_objects(&rest, write_stroke_width)
            }
        }))
    }
}
