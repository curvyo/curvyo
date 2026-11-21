//! The Crop bar of the selected picture (`specs/0135` criterion 5, `ux-notes.md` §3.9): which
//! Crop row it edits, where the whole picture lies on screen for the frame and handles, and the
//! writes of a drag or a field, one step "Edit adjustments" each.

use curvyo_document_core::{NodeId, Point};
use curvyo_laser_core::adjust::{ManualStep, StepKind};
use serde::Serialize;

use super::view::{StepFieldView, field_views};

/// The bar and its canvas frame, as the host draws them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CropBarView {
    /// The Crop row of the stack it edits.
    pub row: usize,
    /// Left, top, width and height in % of the whole picture.
    pub pct: [u8; 4],
    /// The map of the whole picture's unit square to canvas pixels: `[a, b, c, d, e, f]` with
    /// `x = a·u + c·v + e`, `y = b·u + d·v + f`.
    pub quad: [f64; 6],
    /// The working image's size in pixels (the bytes of `before_after_pixels("before")`).
    pub width: u32,
    /// Its height.
    pub height: u32,
    /// The four bar fields, Left, Top, Width, Height.
    pub fields: Vec<StepFieldView>,
}

impl crate::session::Session {
    /// The Crop row `index` of the selected picture with its fields, when it is one.
    fn crop_row(&self, index: usize) -> Option<ManualStep> {
        let steps = self.shown_steps()?;
        let step = steps.get(index)?.step()?;
        let read = ManualStep::read(&step.kind, |key| step.fields.get(key).copied()).ok()?;
        (read.kind() == StepKind::Crop).then_some(read)
    }

    /// Crop on the canvas of Crop row `index` (also after adding a Crop): the bar replaces the
    /// Select bar; `false` when the row is no Crop row of the selected picture.
    pub fn open_crop_bar(&mut self, index: usize) -> bool {
        let Some(target) = self.adjust_target() else {
            return false;
        };
        if self.crop_row(index).is_none() {
            return false;
        }
        self.image_adjust.crop = Some((target, index));
        true
    }

    /// Done, Enter, Escape: the bar leaves (no step).
    pub fn close_crop_bar(&mut self) {
        self.image_adjust.crop = None;
    }

    /// The row the bar edits, while it is open on the selected picture.
    pub(super) fn crop_bar_row(&self) -> Option<usize> {
        let (target, row) = self.image_adjust.crop?;
        (self.tool == crate::session::Tool::Select
            && self.adjust_target() == Some(target)
            && self.crop_row(row).is_some())
        .then_some(row)
    }

    /// Whether the Crop bar is open on the selected picture.
    pub(in crate::session) fn crop_bar_open(&self) -> bool {
        self.crop_bar_row().is_some()
    }

    /// A Select press at `point` while the Crop bar is open: on the picture it is taken (the
    /// picture does not move, its frame is the overlay's); elsewhere the bar leaves and the press
    /// goes on. `false` while the bar is closed.
    pub(in crate::session) fn crop_bar_takes_press(&mut self, point: Point) -> bool {
        if !self.crop_bar_open() {
            return false;
        }
        let on_picture = self
            .adjust_target()
            .and_then(|target| self.picture_leaf(target))
            .is_some_and(|leaf| in_quad(&leaf.full_corners(), point));
        if !on_picture {
            self.close_crop_bar();
        }
        on_picture
    }

    /// The bar and its frame; `None` while it is closed, the selection changed or the row is no
    /// longer a Crop row (it then leaves, `ux-notes.md` §3.9).
    #[must_use]
    pub fn crop_bar_view(&self) -> Option<CropBarView> {
        let row = self.crop_bar_row()?;
        let target = self.adjust_target()?;
        let step = self.crop_row(row)?;
        let ManualStep::Crop { pct } = step else {
            return None;
        };
        let leaf = self.picture_leaf(target)?;
        let (width, height) = self.picture_size(target)?;
        let view = self.view();
        let at =
            |u: f64, v: f64| view.document_to_screen(leaf.placement.apply_point(Point::new(u, v)));
        let (o, x, y) = (at(0.0, 0.0), at(1.0, 0.0), at(0.0, 1.0));
        Some(CropBarView {
            row,
            pct,
            quad: [x.0 - o.0, x.1 - o.1, y.0 - o.0, y.1 - o.1, o.0, o.1],
            width,
            height,
            fields: field_views(step)
                .into_iter()
                .map(|mut field| {
                    if let Some(text) = curvyo_ui_core::crop_range_text(pct, field.key) {
                        field.range = text;
                    }
                    field
                })
                .collect(),
        })
    }

    /// [`Self::crop_bar_view`] as JSON, `null` while closed.
    #[must_use]
    pub fn crop_bar_json(&self) -> String {
        serde_json::to_string(&self.crop_bar_view()).unwrap_or_else(|_| "null".to_owned())
    }

    /// The release of a drag of the frame or a handle: the crop `left, top, width, height` in
    /// whole % written on the bar's row as one step; `false` when it is refused (past the edge,
    /// under 1 %) or unchanged.
    pub fn set_crop_rect(&mut self, left: u8, top: u8, width: u8, height: u8) -> bool {
        let Some(row) = self.crop_bar_row() else {
            return false;
        };
        let Some(steps) = self.shown_steps() else {
            return false;
        };
        let Some(enabled) = steps[row].step().map(|s| s.enabled) else {
            return false;
        };
        let fields = [
            ("left_pct", left),
            ("top_pct", top),
            ("width_pct", width),
            ("height_pct", height),
        ];
        let Ok(step) = ManualStep::read("crop", |key| {
            fields
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| i64::from(*v))
        }) else {
            return false;
        };
        if self.crop_row(row) == Some(step) {
            return false;
        }
        let mut next = steps;
        next[row] = step.entry(enabled);
        self.write_steps(next)
    }
}

/// Whether `p` lies inside the convex quad `q` (its corners in order, either winding).
fn in_quad(q: &[Point; 4], p: Point) -> bool {
    let sides: Vec<f64> = (0..4)
        .map(|i| {
            let (a, b) = (q[i], q[(i + 1) % 4]);
            (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
        })
        .collect();
    sides.iter().all(|s| *s >= 0.0) || sides.iter().all(|s| *s <= 0.0)
}

/// The node a Crop bar was opened on and its row.
pub(super) type CropTarget = (NodeId, usize);
