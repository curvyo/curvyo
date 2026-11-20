//! The Before and after pane of the selected picture (`specs/0135` criteria 11 to 13;
//! `ux-notes.md` §7; ADR 0073 §8): open or closed, Split or Side by side, the operation shown,
//! the subject line, the original's pixels and the dots of the item (`output/pane_picture.rs`).

use curvyo_document_core::NodeId;
use serde::Serialize;

use super::picture::UNASSIGNED_INTERVAL_UM;
use crate::session::operations::image_view::dither_label;

/// The pane's session state, never saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PaneState {
    /// Whether the pane is open.
    pub open: bool,
    /// Split (the default, P20) or Side by side.
    pub split: bool,
    /// The operation picked above the view; the first when `None` or not the picture's.
    pub operation: Option<NodeId>,
}

impl Default for PaneState {
    fn default() -> Self {
        Self {
            open: false,
            split: true,
            operation: None,
        }
    }
}

/// One segment of the operation pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneOperationView {
    /// The operation.
    pub id: NodeId,
    /// Its name.
    pub name: String,
    /// Whether it is the one shown.
    pub pressed: bool,
}

/// One half of the pane: a picture in page mm.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PaneImageView {
    /// Width in pixels of the bytes `before_after_pixels` gives.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The unit square to page mm (`a b c d e f`), so both halves line up.
    pub placement: [f64; 6],
}

/// The After half: the job's dots of the item (criterion 12), or while they are worked a live
/// picture of the current stack on a coarser dot grid (criterion 13).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PaneDotsView {
    /// The dot picture.
    pub image: PaneImageView,
    /// Whether it is the reduced live picture (the exact dots follow when the job settles).
    pub reduced: bool,
}

/// The Before and after pane.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BeforeAfterView {
    /// Whether the pane is open.
    pub open: bool,
    /// `"split"` or `"side_by_side"`.
    pub view: &'static str,
    /// The operation pick: present with two or more Image operations.
    pub operations: Vec<PaneOperationView>,
    /// "photo.jpg · Image 1 · Jarvis · 0.10 mm"; empty without a picture.
    pub subject: String,
    /// "No picture selected." when there is none.
    pub empty: Option<&'static str>,
    /// The original as imported (no step).
    pub before: Option<PaneImageView>,
    /// The dots that will burn; `None` while the picture is in no Image operation.
    pub after: Option<PaneDotsView>,
    /// The Interval of the shown operation in mm (one dot at 100 %).
    pub interval_mm: f64,
}

fn placement(map: curvyo_document_core::Affine) -> [f64; 6] {
    [map.a, map.b, map.c, map.d, map.e, map.f]
}

impl crate::session::Session {
    /// The header button Before and after: opens or closes the pane; whether it is open now.
    pub fn toggle_before_after(&mut self) -> bool {
        let pane = &mut self.image_adjust.pane;
        pane.open = !pane.open;
        pane.open
    }

    /// Escape or Close in the pane.
    pub fn close_before_after(&mut self) {
        self.image_adjust.pane.open = false;
    }

    /// The View strip: `"split"` or `"side_by_side"`; whether it changed.
    pub fn set_before_after_view(&mut self, view: &str) -> bool {
        let split = match view {
            "split" => true,
            "side_by_side" => false,
            _ => return false,
        };
        let changed = self.image_adjust.pane.split != split;
        self.image_adjust.pane.split = split;
        changed
    }

    /// The operation pick above the view (criterion 11); whether `op` runs the picture.
    pub fn pick_before_after_operation(&mut self, op: NodeId) -> bool {
        let Some(target) = self.adjust_target() else {
            return false;
        };
        if !self.picture_operations(target).iter().any(|o| o.id == op) {
            return false;
        }
        self.image_adjust.pane.operation = Some(op);
        true
    }

    /// The Before and after pane.
    #[must_use]
    pub fn before_after_view(&self) -> BeforeAfterView {
        let pane = self.image_adjust.pane;
        let view = if pane.split { "split" } else { "side_by_side" };
        let empty = BeforeAfterView {
            open: pane.open,
            view,
            operations: Vec::new(),
            subject: String::new(),
            empty: Some("No picture selected."),
            before: None,
            after: None,
            interval_mm: f64::from(UNASSIGNED_INTERVAL_UM) / 1000.0,
        };
        let Some(target) = self.adjust_target() else {
            return empty;
        };
        // One read of the drawing for the whole pane (each read is a lookup, but a checked one
        // in the tests).
        let read = self.document_read();
        let Some(leaf) = read
            .clones
            .get(target)
            .and_then(super::picture::image_leaf)
            .cloned()
        else {
            return empty;
        };
        let operations = self.picture_operations_in(&read, target);
        let shown = self.shown_operation(&operations);
        let interval_um = shown.map_or(UNASSIGNED_INTERVAL_UM, |op| op.interval_um);
        let name = self
            .document
            .node_registers(target)
            .and_then(|r| r.name)
            .unwrap_or_else(|| curvyo_ui_core::object_name(&read.clones, target));
        let shown_mm = f64::from(interval_um) / 1000.0;
        // The picture's own dither, not the operation's (ADR 0073 note §8).
        let dither = curvyo_laser_core::adjust::AdjustRegisters::read(&leaf.adjustments)
            .dither
            .mode;
        let subject = match shown {
            Some(op) => format!(
                "{name} · {} · {} · {shown_mm:.2} mm",
                op.name,
                dither_label(dither)
            ),
            None => name,
        };
        let origin = self
            .active_manufacturing()
            .and_then(|m| m.page)
            .map_or(curvyo_document_core::Point::new(0.0, 0.0), |p| p.origin);
        let before = self.job_pixels(&leaf.asset).map(|pixels| PaneImageView {
            width: pixels.width,
            height: pixels.height,
            placement: placement(
                curvyo_document_core::Affine::translation(curvyo_document_core::Vec2::new(
                    -origin.x, -origin.y,
                ))
                .compose(leaf.placement),
            ),
        });
        let after = shown
            .and_then(|op| self.pane_after_picture(&read, op.id, target))
            .map(|(picture, reduced)| PaneDotsView {
                image: PaneImageView {
                    width: picture.image.width,
                    height: picture.image.height,
                    placement: placement(picture.placement),
                },
                reduced,
            });
        BeforeAfterView {
            open: pane.open,
            view,
            operations: if operations.len() > 1 {
                operations
                    .iter()
                    .map(|op| PaneOperationView {
                        id: op.id,
                        name: op.name.clone(),
                        pressed: shown.is_some_and(|s| s.id == op.id),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            subject,
            empty: None,
            before,
            after,
            interval_mm: shown_mm,
        }
    }

    /// [`Self::before_after_view`] as JSON.
    #[must_use]
    pub fn before_after_view_json(&self) -> String {
        serde_json::to_string(&self.before_after_view()).unwrap_or_else(|_| "null".to_owned())
    }

    /// The RGBA8 bytes of one half (`"before"`: the working image, straight alpha; `"after"`:
    /// the dot picture, the Engrave colour with the dot coverage in alpha, the one the pane JSON
    /// sizes); empty when there is none.
    #[must_use]
    pub fn before_after_pixels(&self, half: &str) -> Vec<u8> {
        let Some(target) = self.adjust_target() else {
            return Vec::new();
        };
        match half {
            "before" => self
                .picture_pixels(target)
                .map(|p| p.pixels.clone())
                .unwrap_or_default(),
            "after" => {
                let read = self.document_read();
                let operations = self.picture_operations_in(&read, target);
                self.shown_operation(&operations)
                    .and_then(|op| self.pane_after_picture(&read, op.id, target))
                    .map(|(p, _)| p.image.pixels.clone())
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }
}
