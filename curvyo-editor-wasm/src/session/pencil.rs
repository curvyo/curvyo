//! The Pencil's session glue (`specs/0158-pen-display` slice 1, ADR 0095 decision 3): routes the
//! frame's pen samples to the Pencil tool and writes what it finished into the document.

use curvyo_document_core::{AnchorKind, Length, NewAnchor, NodeId, StyleEdit, Vec2};
use curvyo_geometry_core::OutlineTriple;
use curvyo_geometry_core::fit_stroke;
use curvyo_ui_core::{
    PenEnd, PenSample, PencilOutcome, PencilSettings, PencilStroke, PressureCurve, WidthSource,
};

use super::{Session, Tool, Workbench};

/// The commit label of a Pencil stroke ("Pencil stroke" in the history).
pub(super) const PENCIL_STROKE_LABEL: &str = "pencil_stroke";
/// The commit label of an eraser lift ("Erase" in the history).
pub(super) const ERASE_LABEL: &str = "erase";
/// The notice of the first pressure stroke in a shared project (criterion 23).
pub(super) const SHARED_PRESSURE_NOTICE: &str =
    "Pressure width is not available in a shared project yet.";
/// Handles whose cross product is within this share of their lengths' product are collinear.
const COLLINEAR_SHARE: f64 = 1e-3;

/// The Pencil's state in the session.
#[derive(Debug, Default)]
pub(super) struct PencilState {
    pub(super) tool: curvyo_ui_core::PencilTool,
    pub(super) settings: PencilSettings,
    /// What the eraser end would delete or has touched: the hover box set (criterion 12).
    pub(super) erase_hits: Vec<NodeId>,
    /// Whether the last sample came from the eraser end (the eraser cursor).
    pub(super) eraser_end: bool,
    /// The pen window's Eraser button: every sample erases as the eraser end does
    /// (`specs/0158-pen-display` slice 2b criterion 2).
    pub(super) eraser_mode: bool,
    /// Whether the last finished stroke had pen pressure, `None` before the first.
    pub(super) last_had_pressure: Option<bool>,
    /// The shared-project notice was shown in this session (criterion 23).
    pub(super) shared_notice_shown: bool,
}

impl Session {
    /// Takes one frame's pen samples as flat records (ADR 0095 decision 1); outside the Pencil, or
    /// on a sketch page, they are ignored. Each finished stroke and each erase is its own step.
    pub fn pen_samples(&mut self, flat: &[f64]) {
        // The pen window ignores the pen while it waits, is held or reloads (criteria 13, 18).
        if self.tool != Tool::Pencil || self.link_blocks_input() {
            return;
        }
        if self.workbench() == Workbench::Sketch {
            let pressed = PenSample::decode_flat(flat)
                .iter()
                .any(|s| s.phase == curvyo_ui_core::PenPhase::Contact);
            if pressed && !self.pencil.tool.in_contact() {
                self.link_refuses_sketch_press();
            }
            return;
        }
        for mut sample in PenSample::decode_flat(flat) {
            // The Eraser button takes slice 1's eraser-end path, nothing of its own.
            if self.pencil.eraser_mode {
                sample.end = PenEnd::Eraser;
            }
            self.pencil_sample(&sample);
        }
    }

    /// The pen window's Eraser button on or off: on, the tip (and a mouse) erases as the eraser
    /// end does; off, it draws again. Ends a stroke in progress as Escape does.
    pub fn set_pencil_eraser(&mut self, on: bool) {
        if self.pencil.eraser_mode != on {
            self.pencil_reset();
            self.pencil.eraser_mode = on;
        }
    }

    /// Whether the Eraser button is on.
    #[must_use]
    pub const fn pencil_eraser(&self) -> bool {
        self.pencil.eraser_mode
    }

    fn pencil_sample(&mut self, sample: &PenSample) {
        let point = self.screen_to_document(sample.x.0, sample.y.0);
        self.pointer_position = Some(point);
        self.pencil.eraser_end = sample.end == PenEnd::Eraser;
        let settings = self.pencil.settings;
        match self.pencil.tool.sample(point, sample, &settings) {
            PencilOutcome::Hover(end) => {
                self.pencil.erase_hits = if end == PenEnd::Eraser {
                    self.erase_hits_along(&[point])
                        .into_iter()
                        .last()
                        .into_iter()
                        .collect()
                } else {
                    Vec::new()
                };
            }
            PencilOutcome::Started | PencilOutcome::EraseStarted => {
                // A hidden or locked target layer starts nothing and says so (`0050` crit. 23).
                if self.refuse_drawing_into_layer() {
                    self.pencil.tool.escape();
                    self.pencil.erase_hits.clear();
                } else if sample.end == PenEnd::Eraser {
                    self.pencil.erase_hits = self.erase_hits_along(&[point]);
                }
            }
            PencilOutcome::Moved => {
                if let Some(path) = self.pencil.tool.eraser_path() {
                    let tail = path[path.len().saturating_sub(2)..].to_vec();
                    for id in self.erase_hits_along(&tail) {
                        if !self.pencil.erase_hits.contains(&id) {
                            self.pencil.erase_hits.push(id);
                        }
                    }
                }
            }
            PencilOutcome::Finished(stroke) => {
                self.pencil.last_had_pressure = Some(stroke.has_pressure());
                self.commit_pencil_stroke(&stroke);
            }
            PencilOutcome::Erased(path) => {
                let hits = self.erase_hits_along(&path);
                self.pencil.erase_hits.clear();
                self.erase_objects(&hits);
            }
            PencilOutcome::TooShort | PencilOutcome::Ignored => {
                self.pencil.erase_hits.clear();
            }
        }
    }

    /// The visible, unlocked objects whose line comes within the 8 px hit rule of `0014` of
    /// `line`, in z-order (criterion 12).
    fn erase_hits_along(&self, line: &[curvyo_document_core::Point]) -> Vec<NodeId> {
        curvyo_ui_core::hit_test_objects_along(
            &self.pickable_objects(),
            line,
            self.object_tolerance(),
        )
    }

    /// Deletes `hits` in one step "Erase"; they leave the selection, the rest stays. Nothing hit
    /// writes nothing.
    fn erase_objects(&mut self, hits: &[NodeId]) {
        if hits.is_empty() {
            return;
        }
        self.recording_unguarded(|this| {
            let deleted = this
                .document
                .batch(ERASE_LABEL, |doc| doc.delete_objects(hits).is_ok());
            if deleted {
                let kept: Vec<NodeId> = this
                    .selection
                    .ids()
                    .iter()
                    .copied()
                    .filter(|id| !hits.contains(id))
                    .collect();
                this.selection.set(&kept);
            }
        });
    }

    /// Fits a finished stroke and writes it as one path in one step "Pencil stroke": the new Pen
    /// path's style and layer, its stroke width Max with the profile on it, or the style's width
    /// and no profile for a stroke without pressure (criteria 1, 5, 7, 13). In a shared project,
    /// where width profiles are refused (ADR 0053), a pressure stroke is a uniform stroke at Max
    /// and the first one says so once (slice 2a criterion 23). The path is selected; the tool
    /// stays the Pencil.
    fn commit_pencil_stroke(&mut self, stroke: &PencilStroke) {
        let settings = *stroke.settings();
        let profiled = stroke.source() != WidthSource::Uniform;
        let tolerance = profiled.then(|| settings.widths.profile_tolerance());
        let Some(fitted) = fit_stroke(&stroke.fit_samples(), settings.smoothing, tolerance) else {
            return;
        };
        let anchors: Vec<NewAnchor> = fitted
            .anchors
            .iter()
            .map(|triple| new_anchor(self.minter.mint(), triple))
            .collect();
        let max = settings.widths.max();
        // The profile on the path's width Max: each point's multiplier is its width over Max.
        let scale = fitted.width.as_mm() / max.as_mm();
        let points: Vec<(curvyo_document_core::AnchorId, f64, f64)> = fitted
            .profile
            .iter()
            .flat_map(|profile| {
                profile
                    .positions()
                    .map(|at| (at, profile.sample(at).unwrap_or(1.0) * scale))
                    .collect::<Vec<_>>()
            })
            .map(|(at, multiplier)| (self.minter.mint(), at, multiplier))
            .collect();
        let uniform_at_max = !points.is_empty() && self.extended_edits_blocked();
        if uniform_at_max && !self.pencil.shared_notice_shown {
            self.pencil.shared_notice_shown = true;
            *self.unsupported_width_notice.borrow_mut() = Some(SHARED_PRESSURE_NOTICE.to_owned());
        }
        self.recording_unguarded(|this| {
            let created = this.document.batch(PENCIL_STROKE_LABEL, |doc| {
                let id = doc.create_path(&anchors, false);
                if !points.is_empty() {
                    doc.edit_style(&[id], &StyleEdit::StrokeWidth(max)).ok()?;
                    if !uniform_at_max {
                        doc.set_width_profiles(&[(id, points)]).ok()?;
                    }
                }
                Some(id)
            });
            if let Some(id) = created {
                this.selection.select_single(id);
            }
        });
    }

    /// Escape during a stroke or an erase: dropped, nothing written (criterion 15).
    pub(super) fn pencil_escape(&mut self) -> bool {
        self.pencil.erase_hits.clear();
        self.pencil.tool.escape()
    }

    /// Leaving the Pencil, or a cancelled pointer: nothing in progress stays.
    pub(super) fn pencil_reset(&mut self) {
        self.pencil.tool.reset();
        self.pencil.erase_hits.clear();
        self.pencil.eraser_end = false;
    }

    /// The objects the eraser end would delete or has touched, for their hover boxes.
    pub(super) fn pencil_erase_hits(&self) -> &[NodeId] {
        &self.pencil.erase_hits
    }

    /// Whether a Pencil contact is in progress.
    pub(super) fn pencil_in_contact(&self) -> bool {
        self.pencil.tool.in_contact()
    }

    /// The cursor over the canvas with the Pencil: the eraser cursor for the eraser end.
    pub(super) fn pencil_cursor(&self) -> &'static str {
        if self.pencil.eraser_end {
            "pencil-eraser"
        } else {
            "crosshair"
        }
    }

    /// The preview of the stroke in progress (criterion 10).
    pub(super) fn pencil_preview_list(
        &self,
        view: curvyo_document_core::ViewTransform,
    ) -> curvyo_render_core::DrawList {
        let Some(stroke) = self.pencil.tool.stroke() else {
            return curvyo_render_core::DrawList::default();
        };
        let widths: Option<Vec<f64>> = (stroke.source() != WidthSource::Uniform)
            .then(|| stroke.widths().iter().map(|w| w.as_mm()).collect());
        curvyo_render_core::build_pencil_preview(stroke.points(), widths.as_deref(), view)
    }

    /// The Pencil's settings.
    #[must_use]
    pub const fn pencil_settings(&self) -> PencilSettings {
        self.pencil.settings
    }

    /// Sets Smoothing as the fit accuracy (Smoothing percent / 100 mm, criterion 9); from the
    /// next stroke. A negative or not-a-number value is refused.
    pub fn set_pencil_smoothing(&mut self, accuracy: Length) -> bool {
        let mm = accuracy.as_mm();
        if !mm.is_finite() || mm < 0.0 {
            return false;
        }
        self.pencil.settings.smoothing = accuracy;
        true
    }

    /// Turns "Simulate pressure" on or off.
    pub fn set_pencil_simulate(&mut self, on: bool) {
        self.pencil.settings.simulate = on;
    }

    /// Sets Min (Max is raised to it if below).
    pub fn set_pencil_min(&mut self, width: Length) {
        self.pencil.settings.widths.set_min(width);
    }

    /// Sets Max (raised to Min if below it).
    pub fn set_pencil_max(&mut self, width: Length) {
        self.pencil.settings.widths.set_max(width);
    }

    /// Sets the pressure curve.
    pub fn set_pencil_curve(&mut self, curve: PressureCurve) {
        self.pencil.settings.curve = curve;
    }

    /// Whether the last finished stroke had pen pressure, `None` before the first.
    #[must_use]
    pub const fn pencil_last_had_pressure(&self) -> Option<bool> {
        self.pencil.last_had_pressure
    }
}

/// A fitted anchor as a new anchor: Asymmetric where both handles are collinear and opposite,
/// so the Node tool keeps the curve smooth there, else Corner.
fn new_anchor(id: curvyo_document_core::AnchorId, triple: &OutlineTriple) -> NewAnchor {
    let (point, handle_in, handle_out) = *triple;
    let smooth = smooth_handles(handle_in, handle_out);
    NewAnchor {
        id,
        point,
        handle_in,
        handle_out,
        kind: if smooth {
            AnchorKind::Asymmetric
        } else {
            AnchorKind::Corner
        },
    }
}

fn smooth_handles(a: Vec2, b: Vec2) -> bool {
    let (la, lb) = (a.x.hypot(a.y), b.x.hypot(b.y));
    la > 0.0
        && lb > 0.0
        && (a.x * b.y - a.y * b.x).abs() <= COLLINEAR_SHARE * la * lb
        && a.x * b.x + a.y * b.y < 0.0
}
