//! The Measure tool's gesture (`specs/0070-measure-and-dimension` criteria 1 to 6): the picks,
//! the pinned measurement, the live Shift swap, and the placement and commit of a dimension.

use curvyo_document_core::{
    Angle, DimensionKind, DimensionPlace, DimensionSlots, DimensionValue, DocumentVersion, Length,
    ObjectSnapshot, Point, PointSlot, Vec2,
};
use curvyo_ui_core::{
    CurveKind, MeasurePick, SelectionDimension, angle_picks, free_pick, pick_point, selection_pick,
};

use super::{Session, Tool};

/// A press that moves less than this many screen pixels is a click.
const CLICK_PX: f64 = 4.0;
/// The offset of a dimension placed without a pointer position (Enter, criterion 5).
const DEFAULT_OFFSET_PX: f64 = 24.0;
/// Snap and hit tolerances of a pick, in screen pixels (UX notes §2).
const SNAP_PX: f64 = 8.0;
const HIT_PX: f64 = 8.0;
/// With snapping off a point feature is picked only by a press right on it: no capture zone
/// (criterion 6, `0060`), only the pixel the pointer covers.
const ON_PX: f64 = 1.0;

/// Where the gesture stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    /// Nothing picked.
    #[default]
    Idle,
    /// The first point is picked; the second follows the pointer.
    Picking,
    /// The measurement stays on the canvas until the next press or edit.
    Pinned,
    /// A dimension follows the pointer; the next click places it.
    Placing,
}

/// The Measure tool's state; nothing of it is stored in the document.
#[derive(Debug, Clone, Default)]
pub(super) struct MeasureState {
    phase: Phase,
    first: Option<MeasurePick>,
    second: Option<MeasurePick>,
    /// Where the button went down, and whether that press made the first pick.
    press: Option<(Point, bool)>,
    curve: Option<CurveKind>,
    /// The document version the measurement was pinned at: any edit ends it.
    pinned_at: Option<DocumentVersion>,
    /// The pick under the pointer while nothing is pressed.
    hover: Option<MeasurePick>,
    /// What the live region says: the value of the last pin or placement (UX notes §3).
    speech: String,
}

/// What the gesture measures right now.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Measuring {
    pub(super) kind: DimensionKind,
    pub(super) slots: DimensionSlots,
    /// The measured points, as `Measured::points`.
    pub(super) points: Vec<Point>,
    pub(super) sector: u8,
}

impl Session {
    /// The capture zone of a point feature and the hit tolerance of a curve or segment: the
    /// capture zone only while snapping is on, Alt held inverting the document setting
    /// (criterion 6, UX notes §2).
    fn measure_snap(&self) -> (Length, Length) {
        let px = 1.0 / self.view().scale();
        let snapping = self.document.snapping().enabled != self.held.alt;
        let capture = if snapping { SNAP_PX } else { ON_PX };
        (Length::from_mm(capture * px), Length::from_mm(HIT_PX * px))
    }

    /// What a press at `point` picks (criteria 3 and 6).
    fn measure_pick_at(&self, point: Point) -> MeasurePick {
        let read = self.document_read();
        curvyo_ui_core::measure_pick(&read.visible, point, self.measure_snap())
    }

    /// The one selected object the tool dimensions at once, when eligible (criterion 5).
    pub(super) fn measure_selection(&self) -> Option<(ObjectSnapshot, SelectionDimension)> {
        let ids = self.selection.ids();
        let [id] = ids else {
            return None;
        };
        let read = self.document_read();
        let object = read.visible.iter().find(|o| o.id() == *id)?.clone();
        let eligible = curvyo_ui_core::selection_dimension(&object)?;
        Some((object, eligible))
    }

    /// What the gesture measures with the pointer at `pointer`.
    pub(super) fn measuring(&self, pointer: Point) -> Option<Measuring> {
        let state = &self.measure;
        // Selection first: the preview follows the pointer over empty canvas (criterion 5).
        let selected = (state.phase == Phase::Idle && state.first.is_none())
            .then(|| self.measure_selection())
            .flatten()
            .map(|(object, eligible)| selection_pick(&object, eligible));
        let first = state.first.as_ref().or(selected.as_ref())?;
        match first {
            MeasurePick::Curve {
                slot,
                center,
                radius,
                ..
            } => Some(Measuring {
                kind: state.curve.map_or(DimensionKind::Radius, |c| c.kind),
                slots: DimensionSlots::Curve(*slot),
                points: vec![*center, Point::new(center.x + radius.as_mm(), center.y)],
                sector: 0,
            }),
            MeasurePick::Segment { start, end } => {
                if let Some(second) = &state.second
                    && let Some((three, (r1, r2))) = angle_picks(first, second)
                {
                    let sector = curvyo_ui_core::angle_sector(three[0].1, r1, r2, pointer);
                    return Some(Measuring {
                        kind: DimensionKind::Angle,
                        slots: DimensionSlots::Points(three.iter().map(|p| p.0).collect()),
                        points: three.iter().map(|p| p.1).collect(),
                        sector,
                    });
                }
                Some(Measuring {
                    kind: DimensionKind::Aligned,
                    slots: DimensionSlots::Points(vec![start.0, end.0]),
                    points: vec![start.1, end.1],
                    sector: 0,
                })
            }
            MeasurePick::Point { .. } => {
                let (a_slot, a) = pick_point(first)?;
                let (b_slot, b) = match &state.second {
                    Some(second) => pick_point(second)?,
                    None => pick_point(&self.measure_live_second(pointer))?,
                };
                let kind = if state.phase == Phase::Placing {
                    curvyo_ui_core::two_point_kind(a, b, pointer)
                } else {
                    DimensionKind::Aligned
                };
                Some(Measuring {
                    kind,
                    slots: DimensionSlots::Points(vec![a_slot, b_slot]),
                    points: vec![a, b],
                    sector: 0,
                })
            }
        }
    }

    /// The second point while it follows the pointer: the pick under it, Ctrl locking the
    /// direction from the first point to the Pen's angle stops (criterion 4).
    fn measure_live_second(&self, pointer: Point) -> MeasurePick {
        let first = self.measure.first.as_ref().and_then(pick_point);
        if self.held.ctrl
            && let Some((_, a)) = first
        {
            let d = a.vector_to(pointer);
            let snapped = curvyo_ui_core::snap_angle(Angle::from_radians(d.y.atan2(d.x)));
            let r = snapped.as_radians();
            return free_pick(a.translated(Vec2::new(r.cos(), r.sin()).scaled(d.length())));
        }
        match self.measure_pick_at(pointer) {
            pick @ MeasurePick::Point { .. } => pick,
            _ => free_pick(pointer),
        }
    }

    /// Whether the gesture shows a dimension instead of a measurement: placing, or Shift held
    /// while the second pick is open (criterion 4).
    pub(super) fn measure_leaves_dimension(&self) -> bool {
        match self.measure.phase {
            Phase::Placing => true,
            Phase::Picking => self.held.shift,
            Phase::Idle => self.measure.first.is_none() && self.measure_selection().is_some(),
            Phase::Pinned => false,
        }
    }

    /// The place of the dimension the gesture would leave with the pointer at `pointer`.
    pub(super) fn measure_place(&self, measuring: &Measuring, pointer: Point) -> DimensionPlace {
        let px = 1.0 / self.view().scale();
        let mut start = DimensionPlace::at_offset(Length::from_mm(DEFAULT_OFFSET_PX * px));
        start.sector = measuring.sector;
        if self.measure.phase == Phase::Picking {
            return start;
        }
        let mut place = curvyo_ui_core::place_at(measuring.kind, &measuring.points, start, pointer);
        if measuring.kind == DimensionKind::Angle {
            place.along = 0.5;
        }
        place
    }

    /// The gesture is over: nothing picked.
    pub(super) fn measure_reset(&mut self) {
        let hover = self.measure.hover;
        let speech = std::mem::take(&mut self.measure.speech);
        self.measure = MeasureState {
            hover,
            speech,
            ..MeasureState::default()
        };
    }

    /// Ends a pinned measurement when the document changed since it was pinned (criterion 1).
    pub(super) fn measure_drop_stale_pin(&mut self) {
        if self.measure.phase == Phase::Pinned
            && self.measure.pinned_at.as_ref() != Some(&self.document.version())
        {
            self.measure_reset();
        }
    }

    fn measure_finish_second(&mut self) {
        if self.held.shift {
            self.measure.phase = Phase::Placing;
        } else {
            self.measure.phase = Phase::Pinned;
            self.measure.pinned_at = Some(self.document.version());
            self.measure.speech = self.measure_spoken_value().unwrap_or_default();
        }
    }

    /// The pinned value as spoken: "50 millimetres, 0 degrees", "Radius 25 millimetres",
    /// "45 degrees".
    fn measure_spoken_value(&self) -> Option<String> {
        let measuring = self.measuring(self.pointer_position?)?;
        let unit = self.document.display_unit();
        let speak = |kind, value| curvyo_ui_core::dimension_speech(kind, value, unit);
        let mm = |value| DimensionValue::Length(Length::from_mm(value));
        Some(match measuring.kind {
            DimensionKind::Radius | DimensionKind::Diameter => {
                let r = measuring.points[0].vector_to(measuring.points[1]).length();
                format!("Radius {}", speak(DimensionKind::Radius, mm(r)))
            }
            DimensionKind::Angle => {
                let value = curvyo_document_core::measure(
                    measuring.kind,
                    &measuring.points,
                    measuring.sector,
                )?;
                speak(DimensionKind::Angle, value)
            }
            _ => {
                let d = measuring.points[0].vector_to(measuring.points[1]);
                format!(
                    "{}, {}",
                    speak(DimensionKind::Aligned, mm(d.length())),
                    speak(
                        DimensionKind::Angle,
                        DimensionValue::Angle(Angle::from_radians(d.y.atan2(d.x)))
                    )
                )
            }
        })
    }

    /// What the live region says now.
    pub(super) fn measure_speech(&self) -> &str {
        &self.measure.speech
    }

    /// Whether Ctrl locks the second point's direction right now: the Lock badge (UX notes §5).
    #[must_use]
    pub fn measure_angle_lock_badge(&self) -> bool {
        self.tool == Tool::Measure
            && self.held.ctrl
            && self.measure.phase == Phase::Picking
            && matches!(self.measure.first, Some(MeasurePick::Point { .. }))
    }

    /// Leaves the dimension the gesture measures, placed for `pointer`, as one step "Dimension".
    fn measure_commit(&mut self, pointer: Point) {
        if let Some(measuring) = self.measuring(pointer) {
            let place = self.measure_place(&measuring, pointer);
            let value =
                curvyo_document_core::measure(measuring.kind, &measuring.points, place.sector);
            let placed = self.recording(|this| {
                this.document
                    .add_dimension(measuring.kind, measuring.slots, place)
            });
            if let (Ok(_), Some(value)) = (placed, value) {
                self.measure.speech = format!(
                    "Dimension {}",
                    curvyo_ui_core::dimension_speech(
                        measuring.kind,
                        value,
                        self.document.display_unit()
                    )
                );
            }
        }
        self.measure_reset();
    }

    /// Selection first: places the dimension of the selected object at `pointer` (criterion 5).
    fn measure_place_selection(&mut self, pointer: Option<Point>) -> bool {
        let Some((object, eligible)) = self.measure_selection() else {
            return false;
        };
        self.measure.first = Some(selection_pick(&object, eligible));
        self.measure.curve = Some(CurveKind::picked(self.held.alt));
        self.measure.phase = if pointer.is_some() {
            Phase::Placing
        } else {
            Phase::Picking
        };
        let at = pointer
            .or(self.pointer_position)
            .unwrap_or(Point::new(0.0, 0.0));
        self.measure_commit(at);
        true
    }

    /// A press of the Measure tool.
    pub(super) fn measure_pointer_down(&mut self, point: Point) {
        self.measure_drop_stale_pin();
        let pick = self.measure_pick_at(point);
        let segment_follows = matches!(self.measure.first, Some(MeasurePick::Segment { .. }))
            && self.measure.second.is_none()
            && matches!(pick, MeasurePick::Segment { .. })
            && self
                .measure
                .first
                .as_ref()
                .and_then(|first| angle_picks(first, &pick))
                .is_some();
        match self.measure.phase {
            // The second segment of an angle: Shift held now decides (criterion 4).
            Phase::Placing | Phase::Pinned if segment_follows => {
                self.measure.second = Some(pick);
                self.measure_finish_second();
                return;
            }
            Phase::Placing => {
                self.measure_commit(point);
                return;
            }
            Phase::Picking => {
                self.measure.press = Some((point, false));
                return;
            }
            Phase::Pinned | Phase::Idle => self.measure_reset(),
        }
        let empty = matches!(
            pick,
            MeasurePick::Point {
                slot: PointSlot::Free { .. },
                ..
            }
        );
        if empty && self.measure_place_selection(Some(point)) {
            return;
        }
        self.measure.first = Some(pick);
        match pick {
            MeasurePick::Curve { .. } => {
                self.measure.curve = Some(CurveKind::picked(self.held.alt));
                self.measure_finish_second();
            }
            MeasurePick::Segment { .. } => self.measure_finish_second(),
            MeasurePick::Point { .. } => {
                self.measure.phase = Phase::Picking;
                self.measure.press = Some((point, true));
            }
        }
    }

    /// A release of the Measure tool: the second pick of a drag or of a second click.
    pub(super) fn measure_pointer_up(&mut self, point: Point) {
        let Some((pressed, first_press)) = self.measure.press.take() else {
            return;
        };
        if self.measure.phase != Phase::Picking {
            return;
        }
        let moved = pressed.vector_to(point).length() * self.view().scale() > CLICK_PX;
        if first_press && !moved {
            // A click: the second point is the next click.
            return;
        }
        self.measure.second = Some(self.measure_live_second(point));
        self.measure_finish_second();
    }

    /// The pointer moved: the pick under it, for the marker and the hover highlight.
    pub(super) fn measure_pointer_move(&mut self, point: Point) {
        self.measure_drop_stale_pin();
        self.measure.hover = Some(self.measure_pick_at(point));
    }

    /// Escape: ends the gesture or the pinned measurement; `false` when there was none.
    pub(super) fn measure_escape(&mut self) -> bool {
        let had = self.measure.phase != Phase::Idle;
        self.measure_reset();
        had
    }

    /// Enter: with an eligible selection, places its dimension at the default position.
    pub fn measure_enter(&mut self) -> bool {
        self.tool == Tool::Measure
            && self.measure.phase == Phase::Idle
            && self.measure_place_selection(None)
    }

    /// An Alt keydown while a radius is placed switches Radius and Diameter (criterion 6).
    pub fn measure_alt(&mut self, repeat: bool, other_modifier: bool) -> bool {
        if self.tool != Tool::Measure
            || !matches!(self.measure.first, Some(MeasurePick::Curve { .. }))
        {
            return false;
        }
        self.measure
            .curve
            .as_mut()
            .is_some_and(|curve| curve.alt_down(repeat, other_modifier))
    }

    /// Alt (`alt`) or Enter in the Measure tool, as the key table decided.
    pub(super) fn measure_key(
        &mut self,
        alt: bool,
        input: &super::keys::KeyInput<'_>,
    ) -> super::keys::KeyOutcome {
        let acted = if alt {
            self.measure_alt(input.repeat, input.ctrl || input.shift)
        } else {
            self.measure_enter()
        };
        if acted {
            super::keys::KeyOutcome::Consumed
        } else {
            super::keys::KeyOutcome::Ignored
        }
    }

    /// The pick under the pointer, for the snap marker.
    pub(super) fn measure_hover(&self) -> Option<MeasurePick> {
        self.measure.hover
    }

    /// The first pick, for the anchor dot.
    pub(super) fn measure_first(&self) -> Option<MeasurePick> {
        self.measure.first
    }

    /// The first picked point while the second pick is open, for its dot under a Shift preview.
    pub(super) fn measure_open_anchor(&self) -> Option<Point> {
        if self.measure.phase != Phase::Picking || self.measure.second.is_some() {
            return None;
        }
        self.measure
            .first
            .as_ref()
            .and_then(pick_point)
            .map(|p| p.1)
    }

    /// Whether the measurement is pinned (its readout stays at the second point).
    pub(super) fn measure_pinned(&self) -> bool {
        self.measure.phase == Phase::Pinned
    }

    /// Whether a gesture or a pinned measurement is on the canvas.
    pub(super) fn measure_active(&self) -> bool {
        self.measure.phase != Phase::Idle
    }
}

#[cfg(test)]
#[path = "measure_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "measure_review_tests.rs"]
mod review_tests;
