//! The Image grid's Picture source (`specs/0135` criterion 51, ADR 0072 §6 and A9, ADR 0073
//! slice 2 decisions a to e): the picture the cells engrave, copied into them as image nodes at
//! the pick, each with the cell's Strength.

use curvyo_document_core::{
    AdjustmentsWrite, Affine, AssetHash, DEFAULT_IMAGE_STRENGTH, EXPERT_PRESET, GridCellImage,
    ImageAdjustments, NodeId, ObjectSnapshot, visible_rect,
};
use curvyo_laser_core::grids::{Param, PictureSource, Recipe, Side};
use curvyo_laser_core::settings::SetKind;
use serde::Serialize;

use super::super::Session;
use super::prepared::Target;

/// The picture a picture grid's cells engrave: the asset, its working size and the registers
/// it showed when it was picked.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct GridPicture {
    asset: AssetHash,
    size: (u32, u32),
    adjustments: ImageAdjustments,
}

impl GridPicture {
    /// Whether the picture has a preset, so the Strength axis means something (P25).
    fn has_preset(&self) -> bool {
        self.adjustments
            .preset
            .as_deref()
            .is_some_and(|p| p != EXPERT_PRESET)
    }

    /// The cell image of the square of `side` mm at `(x, y)`: the picture's visible part (its
    /// crops, ADR 0072 §3) fit inside it and centred, with its registers and `strength` (the
    /// cell's, or the picture's own).
    pub(in crate::session) fn cell(
        &self,
        (x, y): (f64, f64),
        side: f64,
        strength: Option<u8>,
    ) -> GridCellImage {
        let shown = &self.adjustments;
        let part = visible_rect(&shown.steps);
        let (w, h) = (
            f64::from(self.size.0.max(1)) * part.width,
            f64::from(self.size.1.max(1)) * part.height,
        );
        let (cw, ch) = if w >= h {
            (side, side * h / w)
        } else {
            (side * w / h, side)
        };
        // The unit square scaled so its visible part is the fitted box.
        let (sx, sy) = (cw / part.width, ch / part.height);
        let (left, top) = (x + (side - cw) / 2.0, y + (side - ch) / 2.0);
        GridCellImage {
            asset: self.asset.clone(),
            placement: Affine::new(sx, 0.0, 0.0, sy, left - part.left * sx, top - part.top * sy),
            adjustments: AdjustmentsWrite {
                preset: Some(
                    shown
                        .preset
                        .clone()
                        .unwrap_or_else(|| EXPERT_PRESET.to_owned()),
                ),
                strength: Some(strength.unwrap_or(shown.strength)),
                steps: Some(shown.steps.clone()),
                dither: shown.dither.clone(),
            },
        }
    }
}

/// The bar's Picture segments of an Image grid (`ux-notes.md` §11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PictureSourceView {
    /// `ramp` or `selected`.
    pub value: &'static str,
    /// Whether the Selected picture segment is shown: a picture is selected or the grid holds
    /// one (P24).
    pub selected_offered: bool,
    /// Whether the Strength axis is offered: Selected picture with a preset on it (P25).
    pub strength_offered: bool,
}

impl Session {
    /// The picture the grid `group` holds: its first image cell.
    fn held_picture(&self, group: NodeId) -> Option<GridPicture> {
        let read = self.object_read();
        self.document.leaves_of(group).into_iter().find_map(|leaf| {
            let ObjectSnapshot::Image(image) = read.get(leaf)? else {
                return None;
            };
            self.picture_of(&image.asset, image.adjustments.clone())
        })
    }

    fn picture_of(&self, asset: &AssetHash, adjustments: ImageAdjustments) -> Option<GridPicture> {
        let pixels = self.job_pixels(asset)?;
        Some(GridPicture {
            asset: asset.clone(),
            size: (pixels.width, pixels.height),
            adjustments,
        })
    }

    /// The one selected picture, as it shows its adjustments.
    fn selected_picture(&self) -> Option<GridPicture> {
        let target = self.adjust_target()?;
        let leaf = self.picture_leaf(target)?;
        let shown = self.document.shown_adjustments(target).ok()?;
        self.picture_of(&leaf.asset, shown.adjustments)
    }

    /// The picture the cells of `recipe` written as `target` engrave: none for the Ramp; the
    /// regenerated or copied grid's own; for a new grid the selected picture.
    pub(in crate::session) fn grid_picture(
        &self,
        recipe: &Recipe,
        target: &Target,
    ) -> Option<GridPicture> {
        if recipe.test.picture() != PictureSource::Selected {
            return None;
        }
        match target {
            Target::Regenerate(group, _) | Target::Copy(group) => {
                let mut held = self.held_picture(*group)?;
                // The cells hold the axis's Strengths; the picture's own is not kept, so a grid
                // whose Strength axis goes burns at 50 (Proposal; a new pick takes the picture's).
                let had_axis = self.grid_object(*group).is_some_and(|grid| {
                    [Side::Columns, Side::Rows]
                        .iter()
                        .any(|&side| grid.recipe.test.axis(side).param() == Param::Strength)
                });
                if had_axis {
                    held.adjustments.strength = DEFAULT_IMAGE_STRENGTH;
                }
                Some(held)
            }
            Target::Place { .. } => self.selected_picture(),
        }
    }

    /// The Picture segments of the bar's `recipe` (grid `group`, or the draft); `None` for a
    /// grid that is not of the Image kind.
    pub(in crate::session) fn picture_source_view(
        &self,
        group: Option<NodeId>,
        recipe: &Recipe,
    ) -> Option<PictureSourceView> {
        if recipe.test.kind() != SetKind::Image {
            return None;
        }
        let picture = match group {
            Some(group) => self.held_picture(group),
            None => self.selected_picture(),
        };
        let selected = recipe.test.picture() == PictureSource::Selected;
        Some(PictureSourceView {
            value: if selected { "selected" } else { "ramp" },
            selected_offered: picture.is_some(),
            strength_offered: selected && picture.as_ref().is_some_and(GridPicture::has_preset),
        })
    }

    /// The Picture segments (criterion 51): `ramp` or `selected`, one step "Edit test grid" on a
    /// grid; `Ok(false)` for Selected picture while none is offered, an unknown source or the
    /// same one.
    ///
    /// # Errors
    /// The bed cause.
    pub fn set_test_grid_picture(&mut self, source: &str) -> Result<bool, String> {
        let picture = match source {
            "ramp" => PictureSource::Ramp,
            "selected" => PictureSource::Selected,
            _ => return Ok(false),
        };
        let Some((group, recipe, _)) = self.bar_view_recipe() else {
            return Ok(false);
        };
        let offered = self
            .picture_source_view(group, &recipe)
            .is_some_and(|v| v.selected_offered);
        if picture == PictureSource::Selected && !offered {
            return Ok(false);
        }
        self.bar_commit(
            |this, current| {
                let defaults = this.recipe_defaults(current.material.as_ref(), current.test.kind());
                Ok((
                    current.test.with_picture(picture, &defaults),
                    current.material.clone(),
                ))
            },
            "picture",
            source,
            curvyo_document_core::EDIT_TEST_GRID_COMMIT_LABEL,
        )
    }
}
