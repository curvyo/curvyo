//! The Pen's cue and the Close path command on the `wasm-bindgen` surface
//! (`specs/0034-pen-path-extension`): codes and counts only, the sentences are the host's
//! (ADR 0001 §5). A further `impl WasmSession` block, every method a pass-through to `Session`.

use curvyo_ui_core::{JoinType, PenTarget};
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// What a Pen press at the pointer would do: `kind` is `"none"` (no pointer or not the Pen),
/// `"place"`, `"new_path"` (Shift over an end), `"continue"`, `"join"`, `"place_over_end"` (Shift
/// over a join target) or `"close"`. For `"close"`, `join` and `as_drawn` are `"sharp"` or
/// `"smooth"` (the join a press applies and the one with no Shift), `shift` is the live Shift and
/// `style_differs` is `false`; for `"join"`, `style_differs` tells whether the continued path and
/// the target path differ in style.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct PenCueView {
    /// The target kind code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The identity of the target node (a path and its end), empty for a place or a close: the
    /// host restarts the rest timer of its chip when it changes.
    #[wasm_bindgen(getter_with_clone)]
    pub target: String,
    /// The join a close applies, `"sharp"` or `"smooth"`, else empty.
    #[wasm_bindgen(getter_with_clone)]
    pub join: String,
    /// The join with no Shift, else empty.
    #[wasm_bindgen(getter_with_clone)]
    pub as_drawn: String,
    /// Whether Shift is held.
    pub shift: bool,
    /// Whether a join target's path differs in style from the continued path.
    pub style_differs: bool,
    /// The unit direction (x) away from the path in progress at a close or join target, for the
    /// hint chip; 0 when there is none.
    pub away_x: f64,
    /// The unit direction (y, down) away from the path in progress.
    pub away_y: f64,
}

fn join_code(join: JoinType) -> String {
    match join {
        JoinType::Sharp => "sharp",
        JoinType::Smooth => "smooth",
    }
    .to_string()
}

/// How many open paths and open outlines of compound paths the Close path buttons would close
/// and skip.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct ClosePathStateView {
    /// Open ordinary paths with three or more nodes.
    pub closable: u32,
    /// Open ordinary paths with fewer than three nodes, or three whose ends coincide.
    pub skipped: u32,
    /// How many of the skipped paths have three nodes with coinciding ends.
    pub same_ends: u32,
    /// Open outlines of compound paths with three or more nodes.
    pub closable_outlines: u32,
    /// Open outlines of compound paths skipped, by the same rule as `skipped`.
    pub skipped_outlines: u32,
    /// How many of the skipped outlines have three nodes with coinciding ends.
    pub same_ends_outlines: u32,
}

/// What a Close path press did.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct ClosePathResultView {
    /// How many paths were closed.
    pub closed: u32,
    /// How many open paths were skipped.
    pub skipped: u32,
    /// How many of the skipped paths have three nodes with coinciding ends.
    pub same_ends: u32,
    /// How many outlines of compound paths were closed.
    pub closed_outlines: u32,
    /// How many open outlines of compound paths were skipped.
    pub skipped_outlines: u32,
    /// How many of the skipped outlines have three nodes with coinciding ends.
    pub same_ends_outlines: u32,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[wasm_bindgen]
impl WasmSession {
    /// Starts editing the selected path's width profile.
    pub fn begin_width_edit(&mut self) -> bool {
        self.session.begin_width_edit()
    }

    /// Whether the tool bar offers Edit width (one stroked path selected).
    #[must_use]
    pub fn width_edit_offered(&self) -> bool {
        self.session.width_edit_offered()
    }

    /// Whether width profile editing is active.
    #[must_use]
    pub fn width_edit_active(&self) -> bool {
        self.session.width_edit_active()
    }

    /// Leaves width profile editing.
    pub fn end_width_edit(&mut self) {
        self.session.end_width_edit();
    }

    /// Takes the latest width edit failure, or an empty string.
    pub fn take_width_edit_notice(&mut self) -> String {
        self.session.take_width_edit_notice()
    }

    /// Takes the warning about a variable stroke that cannot be rendered yet.
    pub fn take_unsupported_width_notice(&self) -> String {
        self.session.take_unsupported_width_notice()
    }

    /// Applies a ready-made width profile (`uniform`, `taper-start`, `taper-end`, `taper-both`)
    /// to the selected path as one undo step: empty when written, else the refusal sentence.
    pub fn apply_width_preset(&mut self, code: &str) -> String {
        let Some(preset) = curvyo_ui_core::WidthPreset::from_code(code) else {
            return "Unknown width profile.".to_owned();
        };
        self.session
            .apply_width_preset(preset)
            .err()
            .unwrap_or_default()
            .to_owned()
    }

    /// The code of the preset the selected path's width profile is, or an empty string.
    #[must_use]
    pub fn selected_width_preset(&self) -> String {
        self.session
            .selected_width_preset()
            .map_or("", curvyo_ui_core::WidthPreset::code)
            .to_owned()
    }

    /// The current Pen mode.
    #[must_use]
    pub fn pen_mode(&self) -> String {
        match self.session.pen_mode() {
            curvyo_ui_core::PenMode::Bezier => "bezier",
            curvyo_ui_core::PenMode::Lines => "lines",
            curvyo_ui_core::PenMode::Freehand => "freehand",
            curvyo_ui_core::PenMode::BSpline => "bspline",
            curvyo_ui_core::PenMode::Spiro => "spiro",
        }
        .to_owned()
    }

    /// Selects a Pen mode when no path is in progress.
    pub fn set_pen_mode(&mut self, mode: &str) -> bool {
        pen_mode(mode).is_some_and(|mode| self.session.set_pen_mode(mode))
    }

    /// Why the Pen mode `mode` cannot be chosen now, or an empty string when it can.
    #[must_use]
    pub fn pen_mode_refusal(&self, mode: &str) -> String {
        pen_mode(mode)
            .and_then(|mode| self.session.pen_mode_refusal(mode))
            .unwrap_or_default()
            .to_owned()
    }

    /// The full width of the selected width point in millimetres, or NaN when none is selected.
    #[must_use]
    pub fn width_point_mm(&self) -> f64 {
        self.session.width_point_mm().unwrap_or(f64::NAN)
    }

    /// Sets the full width of the selected width point, as one undo step.
    pub fn set_width_point_mm(&mut self, mm: f64) -> bool {
        self.session.set_width_point_mm(mm)
    }

    /// Where the selected width point's diamond sits, canvas px as `[x, y]`; empty without one.
    #[must_use]
    pub fn width_point_screen(&self) -> Vec<f64> {
        self.session
            .width_point_screen()
            .map_or_else(Vec::new, |(x, y)| vec![x, y])
    }

    /// Whether the screen point (`x`, `y`) is over a width point in width editing: the host's
    /// wheel goes to [`WasmSession::width_wheel`] there instead of zooming (`specs/0114`).
    #[must_use]
    pub fn width_point_under(&self, x: f64, y: f64) -> bool {
        self.session
            .width_point_under(self.session.screen_to_document(x, y))
    }

    /// The wheel turned `notches` (positive widens) over the screen point (`x`, `y`): a width
    /// point there changes live, 0.5 mm per notch (0.1 mm with `fine`). `false` when the wheel
    /// is not the width's: the host zooms.
    pub fn width_wheel(&mut self, x: f64, y: f64, notches: f64, fine: bool) -> bool {
        let point = self.session.screen_to_document(x, y);
        self.session.width_wheel(point, notches, fine)
    }

    /// Ends a wheel burst over a width point: its width is written as one undo step.
    pub fn end_width_wheel(&mut self) -> bool {
        self.session.end_width_wheel()
    }

    /// Sets the freehand simplification tolerance in millimetres.
    pub fn set_pen_smoothing(&mut self, millimetres: f64) -> bool {
        self.session
            .set_pen_smoothing(curvyo_document_core::Length::from_mm(millimetres))
    }

    /// The freehand simplification tolerance in millimetres.
    #[must_use]
    pub fn pen_smoothing(&self) -> f64 {
        self.session.pen_smoothing().as_mm()
    }

    /// What a Pen press at the pointer would do now, for the cursor class and the hint chip.
    #[must_use]
    pub fn pen_cue(&self) -> PenCueView {
        let mut view = PenCueView {
            kind: "none".to_string(),
            target: String::new(),
            join: String::new(),
            as_drawn: String::new(),
            shift: false,
            style_differs: false,
            away_x: 0.0,
            away_y: 0.0,
        };
        let Some(target) = self.session.pen_target() else {
            return view;
        };
        match target {
            PenTarget::Place => "place",
            PenTarget::NewPathAt => "new_path",
            PenTarget::Continue(end) => {
                view.target = format!("{:?}:{:?}", end.path, end.anchor.id);
                "continue"
            }
            PenTarget::Join(end) => {
                view.target = format!("{:?}:{:?}", end.path, end.anchor.id);
                view.style_differs = self.session.pen_join_style_differs();
                "join"
            }
            PenTarget::PlaceOverEnd => "place_over_end",
            PenTarget::Close {
                join,
                as_drawn,
                shift,
            } => {
                view.join = join_code(join);
                view.as_drawn = join_code(as_drawn);
                view.shift = shift;
                "close"
            }
        }
        .clone_into(&mut view.kind);
        (view.away_x, view.away_y) = self.session.pen_cue_away();
        view
    }

    /// The sequence number of the latest Pen join that moved outlines of the removed object into
    /// the survivor (`specs/0055-compound-path-node-editing` criterion 9), 0 when there was none:
    /// a new number after a release means the host shows the Join notice there.
    #[must_use]
    pub fn pen_join_notice_seq(&self) -> u32 {
        self.session
            .pen_join_notice()
            .map_or(0, |notice| notice.seq)
    }

    /// How many outlines that latest Pen join moved besides the joined one.
    #[must_use]
    pub fn pen_join_moved_outlines(&self) -> u32 {
        self.session
            .pen_join_notice()
            .map_or(0, |notice| count(notice.moved_outlines))
    }

    /// What the Close path buttons would do now.
    #[must_use]
    pub fn close_path_state(&self) -> ClosePathStateView {
        let state = self.session.close_path_state();
        ClosePathStateView {
            closable: count(state.closable),
            skipped: count(state.skipped),
            same_ends: count(state.same_ends),
            closable_outlines: count(state.closable_outlines),
            skipped_outlines: count(state.skipped_outlines),
            same_ends_outlines: count(state.same_ends_outlines),
        }
    }

    /// The Close path buttons: `join` is `"sharp"` or `"smooth"`.
    ///
    /// # Errors
    /// A `JsValue` if `join` is neither.
    pub fn close_path(&mut self, join: &str) -> Result<ClosePathResultView, JsValue> {
        let join = match join {
            "sharp" => JoinType::Sharp,
            "smooth" => JoinType::Smooth,
            other => return Err(JsValue::from_str(&format!("unknown join: {other}"))),
        };
        let outcome = self.session.close_paths(join);
        Ok(ClosePathResultView {
            closed: count(outcome.closed),
            skipped: count(outcome.skipped),
            same_ends: count(outcome.same_ends),
            closed_outlines: count(outcome.closed_outlines),
            skipped_outlines: count(outcome.skipped_outlines),
            same_ends_outlines: count(outcome.same_ends_outlines),
        })
    }
}

/// The Pen mode of a wasm `code`.
fn pen_mode(code: &str) -> Option<curvyo_ui_core::PenMode> {
    Some(match code {
        "bezier" => curvyo_ui_core::PenMode::Bezier,
        "lines" => curvyo_ui_core::PenMode::Lines,
        "freehand" => curvyo_ui_core::PenMode::Freehand,
        "bspline" => curvyo_ui_core::PenMode::BSpline,
        "spiro" => curvyo_ui_core::PenMode::Spiro,
        _ => return None,
    })
}
