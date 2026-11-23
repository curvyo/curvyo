//! The writes of Print and cut on a placed sheet (ADR 0081 §4, §7): a measure, the tolerance,
//! Apply and Reset placement, one commit each; the drawing page is never touched.

use loro::LoroMap;

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE};
use crate::manufacturing::ManufacturingError;
use crate::manufacturing_codec::{PlacementLink, write_placement_link};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::placement_commands::{PlacedPage, overlaps_page};
use crate::print_and_cut_codec::{
    APPLY_PRINT_AND_CUT_COMMIT_LABEL, PRINT_AND_CUT_MAX_MEASURE_UM, PRINT_AND_CUT_MAX_TOLERANCE_UM,
    PRINT_AND_CUT_MEASURE_COMMIT_LABEL, PRINT_AND_CUT_MIN_TOLERANCE_UM,
    PRINT_AND_CUT_TOLERANCE_COMMIT_LABEL, PrintAndCutMeasures, PrintAndCutRegister,
    RESET_PRINT_AND_CUT_COMMIT_LABEL, read_register, write_register,
};

impl Document {
    /// The Print and cut register of the placement `id`, `None` when it is not a live placement.
    #[must_use]
    pub fn print_and_cut(&self, id: NodeId) -> Option<PrintAndCutRegister> {
        self.placed_page(id)?;
        Some(read_register(&self.placement_meta(id)?))
    }

    fn placement_meta(&self, id: NodeId) -> Option<LoroMap> {
        self.loro()
            .get_tree(OBJECTS_TREE)
            .get_meta(tree_id_of(id))
            .ok()
    }

    /// The stored measures to edit: an absent or unreadable register starts empty (ADR 0081 §4:
    /// the next measure write replaces an unreadable one).
    fn editable_measures(
        &self,
        id: NodeId,
    ) -> Result<(LoroMap, PrintAndCutMeasures), ManufacturingError> {
        self.placed_page(id).ok_or(ManufacturingError::NotFound)?;
        let meta = self
            .placement_meta(id)
            .ok_or(ManufacturingError::NotFound)?;
        let measures = match read_register(&meta) {
            PrintAndCutRegister::Measures(measures) => measures,
            PrintAndCutRegister::Absent | PrintAndCutRegister::Unreadable => {
                PrintAndCutMeasures::default()
            }
        };
        Ok((meta, measures))
    }

    /// Sets (or with `None` clears) the measured position of mark `number` on the placement
    /// `id`, in µm, in one commit labelled [`PRINT_AND_CUT_MEASURE_COMMIT_LABEL`] (criterion 9).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an id that is not a live placement;
    /// [`ManufacturingError::OutOfRange`] for a number outside 1 to 4 or a coordinate past
    /// ±10,000 mm. Nothing is written.
    pub fn set_print_and_cut_measure(
        &self,
        id: NodeId,
        number: u8,
        position: Option<[i64; 2]>,
    ) -> Result<(), ManufacturingError> {
        if !(1..=4).contains(&number)
            || position.is_some_and(|p| p.iter().any(|v| v.abs() > PRINT_AND_CUT_MAX_MEASURE_UM))
        {
            return Err(ManufacturingError::OutOfRange);
        }
        let (meta, mut measures) = self.editable_measures(id)?;
        match position {
            Some(p) => measures.marks.insert(number, p),
            None => measures.marks.remove(&number),
        };
        write_register(&meta, &measures);
        self.commit_with_label(PRINT_AND_CUT_MEASURE_COMMIT_LABEL);
        Ok(())
    }

    /// Sets the tolerance of the placement `id` in µm, in one commit labelled
    /// [`PRINT_AND_CUT_TOLERANCE_COMMIT_LABEL`] (criterion 13).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] as above; [`ManufacturingError::OutOfRange`] outside 50
    /// to 5000 µm.
    pub fn set_print_and_cut_tolerance(
        &self,
        id: NodeId,
        tolerance_um: i64,
    ) -> Result<(), ManufacturingError> {
        if !(PRINT_AND_CUT_MIN_TOLERANCE_UM..=PRINT_AND_CUT_MAX_TOLERANCE_UM)
            .contains(&tolerance_um)
        {
            return Err(ManufacturingError::OutOfRange);
        }
        let (meta, mut measures) = self.editable_measures(id)?;
        measures.tolerance_um = tolerance_um;
        write_register(&meta, &measures);
        self.commit_with_label(PRINT_AND_CUT_TOLERANCE_COMMIT_LABEL);
        Ok(())
    }

    /// Apply (criterion 16): writes `fit` (source page-local to output page-local mm) as the
    /// placement's matrix and the previous matrix as its Reset target, in one commit labelled
    /// [`APPLY_PRINT_AND_CUT_COMMIT_LABEL`]. The drawing page is not touched.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] as above; [`ManufacturingError::OutOfRange`] for a matrix
    /// that is not finite or past ±1e7; [`ManufacturingError::OffPage`] when the fitted frame
    /// leaves no overlap with the laser page ("The fit puts the sheet off the laser page.").
    pub fn apply_print_and_cut(&self, id: NodeId, fit: Affine) -> Result<(), ManufacturingError> {
        let placed = self.placed_page(id).ok_or(ManufacturingError::NotFound)?;
        check_on_page(&placed, fit)?;
        let (meta, mut measures) = self.editable_measures(id)?;
        measures.reset = Some(placed.matrix);
        write_matrix(&meta, &placed, fit)?;
        write_register(&meta, &measures);
        self.commit_with_label(APPLY_PRINT_AND_CUT_COMMIT_LABEL);
        Ok(())
    }

    /// Reset placement (criterion 16): writes the Reset target back as the matrix and removes it,
    /// in one commit labelled [`RESET_PRINT_AND_CUT_COMMIT_LABEL`].
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an id that is not a live placement or has no Reset
    /// target; [`ManufacturingError::OffPage`] as [`Document::apply_print_and_cut`].
    pub fn reset_print_and_cut(&self, id: NodeId) -> Result<(), ManufacturingError> {
        let placed = self.placed_page(id).ok_or(ManufacturingError::NotFound)?;
        let (meta, mut measures) = self.editable_measures(id)?;
        let reset = measures.reset.take().ok_or(ManufacturingError::NotFound)?;
        check_on_page(&placed, reset)?;
        write_matrix(&meta, &placed, reset)?;
        write_register(&meta, &measures);
        self.commit_with_label(RESET_PRINT_AND_CUT_COMMIT_LABEL);
        Ok(())
    }
}

fn check_on_page(placed: &PlacedPage, matrix: Affine) -> Result<(), ManufacturingError> {
    if !matrix.is_finite() || !matrix.is_within_limit() {
        return Err(ManufacturingError::OutOfRange);
    }
    let moved = PlacedPage {
        matrix,
        ..placed.clone()
    };
    let corners = moved.frame_corners().ok_or(ManufacturingError::NotFound)?;
    if overlaps_page(&corners, &placed.output) {
        Ok(())
    } else {
        Err(ManufacturingError::OffPage)
    }
}

fn write_matrix(
    meta: &LoroMap,
    placed: &PlacedPage,
    matrix: Affine,
) -> Result<(), ManufacturingError> {
    let page = placed
        .source
        .as_ref()
        .ok_or(ManufacturingError::NotFound)?
        .id
        .clone();
    write_placement_link(meta, &PlacementLink { page, matrix });
    Ok(())
}

#[cfg(test)]
#[path = "print_and_cut_commands_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "print_and_cut_format_tests.rs"]
mod format_tests;
