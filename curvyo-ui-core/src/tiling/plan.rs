//! What the Select bar's Tile button offers and what an Apply sends to the document
//! (`specs/0069-tiling` criteria 1, 3, 5, 26 and 36): the state of the button, the cell box of
//! the sources, the count status and the request with its cells and conversions.

use curvyo_document_core::{
    Document, NodeId, ObjectRead, ObjectSnapshot, TILING_MAX_COUNT, TILING_WARN_COUNT,
    TilingRecipe, TilingRequest, TilingResult,
};

use super::layout::{CellBox, tiling_layout};
use crate::anchor_id_minter::AnchorIdMinter;
use crate::conversion::build_primitive_conversions;
use crate::object_bounds::object_bounds;
use crate::units::Units;

/// What the Tile slot of the Select bar shows (UX notes §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileButton {
    /// Nothing is selected: "Tile", disabled.
    NothingSelected,
    /// A tiling together with other objects: "Tile", disabled.
    TilingWithOthers,
    /// "Tile", enabled, for these top-level sources.
    Tile(Vec<NodeId>),
    /// Exactly one tiling: "Edit tiling" and "Release tiling".
    Edit(NodeId),
}

/// The state of the Tile slot for the selected leaves `selected` (criteria 3 and 26).
#[must_use]
pub fn tile_button(document: &Document, units: &Units, selected: &[NodeId]) -> TileButton {
    let objects = units.selected(selected);
    if objects.is_empty() {
        return TileButton::NothingSelected;
    }
    let tilings = objects
        .iter()
        .filter(|object| object.is_group && document.tiling(object.id).is_some())
        .count();
    match (tilings, objects.len()) {
        (0, _) => TileButton::Tile(objects.iter().map(|object| object.id).collect()),
        (1, 1) => TileButton::Edit(objects[0].id),
        _ => TileButton::TilingWithOthers,
    }
}

/// The box Align and Distribute use for the leaves of `sources` (criterion 5), `None` when they
/// hold no drawn object (only dimensions, or nothing left).
#[must_use]
pub fn cell_box(document: &Document, read: &ObjectRead, sources: &[NodeId]) -> Option<CellBox> {
    document
        .tiling_leaves(sources)
        .iter()
        .filter_map(|leaf| read.get(*leaf))
        .filter(|object| !matches!(object, ObjectSnapshot::Dimension(_)))
        .map(object_bounds)
        .reduce(|(a_min, a_max), (b_min, b_max)| {
            (
                curvyo_document_core::Point::new(a_min.x.min(b_min.x), a_min.y.min(b_min.y)),
                curvyo_document_core::Point::new(a_max.x.max(b_max.x), a_max.y.max(b_max.y)),
            )
        })
        .map(|(min, max)| CellBox { min, max })
}

/// How the count of a recipe stands (criterion 36).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountStatus {
    /// Up to the warning limit.
    Fine(u64),
    /// Above 10,000: the preview draws boxes only.
    Simplified(u64),
    /// Above 100,000: Apply is disabled.
    Refused(u64),
}

/// The count status of `recipe`, the kept source counted as an instance.
#[must_use]
pub fn count_status(recipe: &TilingRecipe) -> CountStatus {
    let count = recipe.instance_count();
    if count > u64::from(TILING_MAX_COUNT) {
        CountStatus::Refused(count)
    } else if count > u64::from(TILING_WARN_COUNT) {
        CountStatus::Simplified(count)
    } else {
        CountStatus::Fine(count)
    }
}

/// Whether the sources hold an image while Clones is chosen ("Images become copies.").
#[must_use]
pub fn images_become_copies(
    document: &Document,
    read: &ObjectRead,
    sources: &[NodeId],
    recipe: &TilingRecipe,
) -> bool {
    recipe.result == TilingResult::Clones
        && document
            .tiling_leaves(sources)
            .iter()
            .any(|leaf| matches!(read.get(*leaf), Some(ObjectSnapshot::Image(_))))
}

/// The request of an Apply: the cells of `recipe` for the box of the sources and the "Object to
/// path" outline of every primitive leaf (criteria 23 and 35: the preview draws the same cells).
#[must_use]
pub fn tiling_request(
    document: &Document,
    read: &ObjectRead,
    minter: &mut AnchorIdMinter,
    (tiling, sources): (Option<NodeId>, &[NodeId]),
    recipe: &TilingRecipe,
    cell_box: CellBox,
) -> TilingRequest {
    let primitives: Vec<NodeId> = document
        .tiling_leaves(sources)
        .into_iter()
        .filter(|leaf| matches!(read.get(*leaf), Some(ObjectSnapshot::Primitive(_))))
        .collect();
    TilingRequest {
        tiling,
        sources: if tiling.is_some() {
            Vec::new()
        } else {
            sources.to_vec()
        },
        recipe: *recipe,
        cells: tiling_layout(cell_box, recipe),
        conversions: build_primitive_conversions(document, minter, &primitives),
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Length, Point, RectBounds};

    use super::*;
    use crate::tiling::default_recipe;

    #[test]
    fn a_rectangle_tiles_and_reopens_as_one_tiling() {
        let d = Document::new(1);
        let rect = d.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(5.0),
        });
        let units = Units::default();
        assert_eq!(tile_button(&d, &units, &[]), TileButton::NothingSelected);
        let TileButton::Tile(sources) = tile_button(&d, &units, &[rect]) else {
            panic!("offered")
        };
        let read = d.read_objects();
        let cell_box = cell_box(&d, &read, &sources).unwrap();
        assert_eq!(cell_box.max, Point::new(10.0, 5.0));
        let recipe = default_recipe(cell_box, 9);
        let mut minter = AnchorIdMinter::new(1);
        let request = tiling_request(&d, &read, &mut minter, (None, &sources), &recipe, cell_box);
        assert_eq!(request.cells.len(), 19);
        assert_eq!(request.conversions.len(), 1);
        let mut mint = || minter.mint();
        let tiling = d.apply_tiling(&request, &mut mint).unwrap().tiling;
        let units = Units::build(&d, None);
        let leaves = d.tiling_leaves(&[tiling]);
        assert_eq!(tile_button(&d, &units, &leaves), TileButton::Edit(tiling));
        assert_eq!(count_status(&recipe), CountStatus::Fine(20));
    }
}
