//! The entry box of the position handle of a text on path (`specs/0080-text-on-path` criteria
//! 17, 18; customer 2026-10-24): a double-click on the handle opens it with Position, Distance,
//! Side, Line on path and Mirror; its buttons preview, Enter writes one step, Escape nothing.

use curvyo_document_core::{
    Length, LineOnPath, NodeId, OnPathLabel, OnPathValues, PathSide, Point, TextSnapshot,
};

use super::Session;

/// The position handle's hit radius for the double-click, screen px ("Parameter handle").
const HANDLE_HIT_PX: f64 = 12.0;

/// The open entry box: the text it edits, its values at the open and the pending ones.
#[derive(Debug, Clone, Copy)]
pub(super) struct HandleEntry {
    text: NodeId,
    start: OnPathValues,
    pub(super) values: OnPathValues,
}

/// What the entry box shows.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPathEntryView {
    /// The position handle, document mm: the box sits next to it.
    pub handle: Point,
    /// The pending side.
    pub side: PathSide,
    /// The pending line on path.
    pub line: LineOnPath,
    /// The Position at the open: mm, or degrees on an ellipse carrier.
    pub position: f64,
    /// Whether Position reads degrees.
    pub degrees: bool,
    /// The Distance at the open, mm.
    pub distance_mm: f64,
}

impl Session {
    /// A double-click at `point`: on the position handle of the one selected (or edited) text on
    /// path it opens the entry box. Returns whether it opened.
    pub(super) fn open_text_path_entry_at(&mut self, point: Point) -> bool {
        let Some(text) = self.sole_text_on_path() else {
            return false;
        };
        let Some(on) = self.on_carrier(&text) else {
            return false;
        };
        let Some(values) = text.on_path.as_ref().map(|p| p.values) else {
            return false;
        };
        let (handle, _) = curvyo_ui_core::handle_points(&on.placement);
        if handle.vector_to(point).length() > HANDLE_HIT_PX / self.view().scale() {
            return false;
        }
        self.cancel_text_path_drag();
        self.text_path.entry = Some(HandleEntry {
            text: text.id,
            start: values,
            values,
        });
        true
    }

    /// The open entry box, or `None`; it closes when its text is no longer the one selected.
    #[must_use]
    pub fn text_path_entry(&self) -> Option<TextPathEntryView> {
        let entry = self.text_path.entry?;
        let text = self.sole_text_on_path().filter(|t| t.id == entry.text)?;
        let on = self.on_carrier(&text)?;
        let (handle, _) = curvyo_ui_core::handle_points(&on.placement);
        let ellipse = ellipse_of(&text);
        let position = ellipse.map_or(entry.start.position.as_mm(), |e| {
            curvyo_ui_core::ellipse_degrees(&e, handle)
        });
        Some(TextPathEntryView {
            handle,
            side: entry.values.side,
            line: entry.values.line,
            position,
            degrees: ellipse.is_some(),
            distance_mm: entry.start.distance.as_mm(),
        })
    }

    /// Side in the entry box: previewed, written on Enter.
    pub fn text_path_entry_side(&mut self, side: PathSide) {
        if let Some(entry) = self.text_path.entry.as_mut() {
            entry.values.side = side;
        }
    }

    /// Line on path in the entry box: previewed, written on Enter.
    pub fn text_path_entry_line(&mut self, line: LineOnPath) {
        if let Some(entry) = self.text_path.entry.as_mut() {
            entry.values.line = line;
        }
    }

    /// Mirror in the entry box (criterion 18): across the carrier, reading direction kept.
    pub fn text_path_entry_mirror(&mut self) {
        if let Some(entry) = self.text_path.entry.as_mut() {
            entry.values = entry.values.mirrored();
        }
    }

    /// Escape: the entry box closes, nothing is written.
    pub fn cancel_text_path_entry(&mut self) {
        self.text_path.entry = None;
    }

    /// Enter with the typed Position (mm, or degrees on an ellipse carrier) and Distance (mm):
    /// `"committed"` (one step), `"unchanged"` or `"invalid:<field>"` (0 Position, 1 Distance;
    /// the box stays open).
    pub fn commit_text_path_entry(&mut self, position: &str, distance: &str) -> String {
        let Some(entry) = self.text_path.entry else {
            return "unchanged".to_owned();
        };
        let Some(view) = self.text_path_entry() else {
            self.text_path.entry = None;
            return "unchanged".to_owned();
        };
        let Some(typed_position) = curvyo_ui_core::parse_entry_number(position, view.degrees)
        else {
            return "invalid:0".to_owned();
        };
        let Some(typed_distance) = curvyo_ui_core::parse_entry_number(distance, false) else {
            return "invalid:1".to_owned();
        };
        self.text_path.entry = None;
        let Some(text) = self.read_text(entry.text) else {
            return "unchanged".to_owned();
        };
        let mut values = entry.values;
        // An untouched field keeps the stored value exactly, not its rounded readout.
        if (typed_position - view.position).abs() >= 0.05 {
            values.position = self.typed_position(&text, typed_position);
        }
        if (typed_distance - view.distance_mm).abs() >= 0.05 {
            values.distance = Length::from_mm(typed_distance.clamp(-1e5, 1e5));
        }
        let Some(label) = entry_label(entry.start, values) else {
            return "unchanged".to_owned();
        };
        self.recording(|this| {
            let _ = this.document.set_text_path(&[(text.id, values)], label);
        });
        "committed".to_owned()
    }

    /// The arc length of a typed Position: mm, or degrees on an ellipse carrier.
    fn typed_position(&self, text: &TextSnapshot, value: f64) -> Length {
        match (ellipse_of(text), self.on_carrier(text)) {
            (Some(ellipse), Some(on)) => {
                curvyo_ui_core::position_at_degrees(on.placement.measure(), &ellipse, value)
            }
            _ => Length::from_mm(value),
        }
    }

    /// The text of the open entry box at its pending values, for the preview.
    pub(super) fn text_path_entry_values(&self) -> Option<(NodeId, OnPathValues)> {
        let entry = self.text_path.entry?;
        (entry.values != entry.start).then_some((entry.text, entry.values))
    }
}

/// The ellipse frame of `text`'s carrier, if it is one.
fn ellipse_of(text: &TextSnapshot) -> Option<curvyo_document_core::EllipseCarrier> {
    text.on_path.as_ref()?.curve.as_ref()?.ellipse
}

/// The one step's label for the change from `start` to `values`: the change's own when only one
/// thing changed (both Side and Line swapped is Mirror), else "Change text on path"; `None` when
/// nothing changed.
fn entry_label(start: OnPathValues, values: OnPathValues) -> Option<OnPathLabel> {
    let side = start.side != values.side;
    let line = start.line != values.line;
    let position = start.position != values.position;
    let distance = start.distance != values.distance;
    match (side, line, position, distance) {
        (false, false, false, false) => None,
        (true, false, false, false) => Some(OnPathLabel::Side),
        (false, true, false, false) => Some(OnPathLabel::Line),
        (true, true, false, false) => Some(OnPathLabel::Mirror),
        (false, false, true, false) => Some(OnPathLabel::Move),
        (false, false, false, true) => Some(OnPathLabel::Distance),
        _ => Some(OnPathLabel::Values),
    }
}
