//! The pen sample record (`specs/0158-pen-display` slice 0, ADR 0094 decision 3): what a host reads
//! from its toolkit's pen events, in normalised units, batched per frame.
//!
//! Pure: the host reads its toolkit's events into [`RawPenEvent`]s and sends [`PenBatch`]es; this
//! module shapes nothing else, so a pen without a device is testable.

mod flat;

use serde::{Deserialize, Serialize};

pub use flat::PEN_SAMPLE_STRIDE;

use crate::ScreenPx;

/// The most samples one batch holds; the oldest are dropped past it and counted (ADR 0094
/// decision 4).
pub const PEN_BATCH_LIMIT: usize = 256;

/// An event older than this at the moment it is sent is taken as a clock mismatch, not an age.
const MAX_AGE_MS: u32 = 10_000;

/// Pen pressure, 0 (none) to 1 (full); built clamped, a missing or not-a-number value is 0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pressure(f64);

impl Pressure {
    /// The pressure of an already normalised axis value, clamped to `0..=1`.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(if value.is_nan() {
            0.0
        } else {
            value.clamp(0.0, 1.0)
        })
    }

    /// The pressure of a raw device axis with range `min..=max`; an empty range reads as 0.
    #[must_use]
    pub fn from_axis(value: f64, min: f64, max: f64) -> Self {
        if max > min {
            Self::new((value - min) / (max - min))
        } else {
            Self::new(0.0)
        }
    }

    /// The value, 0 to 1.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Pen tilt, each axis -1 to 1 as a fraction of 90 degrees (GDK's normalised value).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tilt {
    x: f64,
    y: f64,
}

impl Tilt {
    /// A tilt of normalised axes, each clamped to `-1..=1` (not-a-number reads as 0).
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        let axis = |v: f64| if v.is_nan() { 0.0 } else { v.clamp(-1.0, 1.0) };
        Self {
            x: axis(x),
            y: axis(y),
        }
    }

    /// A tilt of Pointer Event degrees (`tiltX`, `tiltY`, -90 to 90).
    #[must_use]
    pub fn from_degrees(x: f64, y: f64) -> Self {
        Self::new(x / 90.0, y / 90.0)
    }

    /// The x tilt, -1 to 1.
    #[must_use]
    pub const fn x(self) -> f64 {
        self.x
    }

    /// The y tilt, -1 to 1.
    #[must_use]
    pub const fn y(self) -> f64 {
        self.y
    }
}

/// Which end of the pen touches or hovers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PenEnd {
    /// The tip.
    Tip,
    /// The eraser end.
    Eraser,
}

/// Where the pen is relative to the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PenPhase {
    /// In range, not touching.
    Hover,
    /// Touching (button 1 held).
    Contact,
    /// Left the range.
    Out,
}

/// One pen sample (ADR 0094 decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PenSample {
    /// CSS px from the viewport's left edge, the space of `PointerEvent.clientX`.
    pub x: ScreenPx,
    /// CSS px from the viewport's top edge.
    pub y: ScreenPx,
    /// `None` when the device reports no pressure axis.
    pub pressure: Option<Pressure>,
    /// `None` when the device reports no tilt.
    pub tilt: Option<Tilt>,
    /// Tip or eraser end.
    pub end: PenEnd,
    /// Hover, contact or out of range.
    pub phase: PenPhase,
    /// Barrel buttons: bit 0 is button 2, bit 1 is button 3.
    pub barrel: u8,
    /// Milliseconds on the source's own clock; only differences within a stroke mean anything.
    pub time_ms: f64,
}

/// What kind of raw event it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawKind {
    /// The pointer moved.
    Motion,
    /// A button went down (its number, 1 based).
    Press(u32),
    /// A button went up (its number, 1 based).
    Release(u32),
    /// The pen came into range.
    ProximityIn,
    /// The pen left the range.
    ProximityOut,
}

/// What the toolkit says the source device is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSource {
    /// A pen tip.
    Pen,
    /// An eraser end.
    Eraser,
    /// Anything else (a mouse, a touchpad, a device the driver shows as a mouse).
    Other,
}

/// One event of the toolkit, read into plain values; the input of [`RawPenEvent::sample`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawPenEvent {
    /// The event type.
    pub kind: RawKind,
    /// The source device's kind.
    pub source: RawSource,
    /// The device tool says eraser (some drivers show it only there).
    pub eraser_tool: bool,
    /// The pressure axis, already 0 to 1, if the event has one.
    pub pressure: Option<f64>,
    /// The x tilt axis, -1 to 1, if any.
    pub tilt_x: Option<f64>,
    /// The y tilt axis, -1 to 1, if any.
    pub tilt_y: Option<f64>,
    /// Position in the webview's viewport, CSS px.
    pub x: f64,
    /// Position in the webview's viewport, CSS px.
    pub y: f64,
    /// The event time, ms, 32 bit as the toolkit gives it.
    pub time: u32,
    /// Buttons held before the event (bit 0 button 1, bit 1 button 2, bit 2 button 3).
    pub buttons_before: u8,
}

/// The bit of a button number in a button mask; buttons above 3 have none.
fn button_bit(button: u32) -> u8 {
    match button {
        1..=3 => 1 << (button - 1),
        _ => 0,
    }
}

impl RawPenEvent {
    /// Whether this is a pen event: from a pen or eraser device, or with a pressure axis (GDK 3
    /// on X11 guesses the source from the device name, so a driver's device may read as a mouse).
    #[must_use]
    pub fn is_pen(&self) -> bool {
        self.source != RawSource::Other || self.pressure.is_some()
    }

    /// The sample of this event, or `None` for an event that is not a pen's. `clock` unwraps the
    /// 32 bit time.
    #[must_use]
    pub fn sample(&self, clock: &mut TimeUnwrap) -> Option<PenSample> {
        if !self.is_pen() {
            return None;
        }
        let held = match self.kind {
            RawKind::Press(button) => self.buttons_before | button_bit(button),
            RawKind::Release(button) => self.buttons_before & !button_bit(button),
            _ => self.buttons_before,
        };
        let phase = match self.kind {
            RawKind::ProximityOut => PenPhase::Out,
            _ if held & 1 != 0 => PenPhase::Contact,
            _ => PenPhase::Hover,
        };
        let tilt = match (self.tilt_x, self.tilt_y) {
            (None, None) => None,
            (x, y) => Some(Tilt::new(x.unwrap_or(0.0), y.unwrap_or(0.0))),
        };
        let eraser = self.source == RawSource::Eraser || self.eraser_tool;
        Some(PenSample {
            x: ScreenPx(self.x),
            y: ScreenPx(self.y),
            pressure: self.pressure.map(Pressure::new),
            tilt,
            end: if eraser { PenEnd::Eraser } else { PenEnd::Tip },
            phase,
            barrel: (held >> 1) & 0b11,
            time_ms: clock.unwrap(self.time),
        })
    }
}

/// Turns a 32 bit millisecond time that wraps (about every 49.7 days) into a running one.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TimeUnwrap {
    last: Option<u32>,
    wraps: u64,
}

impl TimeUnwrap {
    /// The running time of `time` in ms; a slightly older event after a wrap keeps the earlier
    /// round.
    pub fn unwrap(&mut self, time: u32) -> f64 {
        const HALF: u32 = 1 << 31;
        let wraps = match self.last {
            Some(last) if time < last && last - time > HALF => {
                self.wraps += 1;
                self.last = Some(time);
                self.wraps
            }
            Some(last) if time > last && time - last > HALF => self.wraps.saturating_sub(1),
            _ => {
                self.last = Some(self.last.map_or(time, |last| last.max(time)));
                self.wraps
            }
        };
        #[allow(clippy::cast_precision_loss)] // 2^53 ms is 285,000 years
        {
            wraps as f64 * 4_294_967_296.0 + f64::from(time)
        }
    }
}

/// How long ago the event happened, ms, when sent at `now` (same clock, 32 bit); `None` when it
/// looks like the clocks differ (more than ten seconds).
#[must_use]
pub fn event_age_ms(now: u32, event: u32) -> Option<f64> {
    let age = now.wrapping_sub(event);
    (age <= MAX_AGE_MS).then(|| f64::from(age))
}

/// One frame's samples, in order (ADR 0094 decision 4).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PenBatch {
    /// The samples, oldest first.
    pub samples: Vec<PenSample>,
    /// How many samples were dropped from the front of this burst for want of room.
    pub dropped: u32,
    /// The source device's name, for the probe page only; empty when unknown.
    pub device: String,
    /// The host's age of the oldest sample when sent, ms, when its clock allows one.
    pub age_ms: Option<f64>,
}

/// Collects samples between two frames.
#[derive(Debug, Default)]
pub struct PenBatcher {
    samples: Vec<PenSample>,
    dropped: u32,
}

impl PenBatcher {
    /// Adds a sample; past [`PEN_BATCH_LIMIT`] the oldest one is dropped and counted.
    pub fn push(&mut self, sample: PenSample) {
        if self.samples.len() >= PEN_BATCH_LIMIT {
            self.samples.remove(0);
            self.dropped = self.dropped.saturating_add(1);
        }
        self.samples.push(sample);
    }

    /// Whether nothing waits to be sent.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty() && self.dropped == 0
    }

    /// Empties the buffer into a batch, `None` when it was empty.
    pub fn take(&mut self) -> Option<PenBatch> {
        if self.is_empty() {
            return None;
        }
        Some(PenBatch {
            samples: std::mem::take(&mut self.samples),
            dropped: std::mem::take(&mut self.dropped),
            device: String::new(),
            age_ms: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(kind: RawKind) -> RawPenEvent {
        RawPenEvent {
            kind,
            source: RawSource::Pen,
            eraser_tool: false,
            pressure: Some(0.5),
            tilt_x: None,
            tilt_y: None,
            x: 10.0,
            y: 20.0,
            time: 1000,
            buttons_before: 0,
        }
    }

    fn sample_of(event: &RawPenEvent) -> PenSample {
        event
            .sample(&mut TimeUnwrap::default())
            .expect("a pen event")
    }

    #[test]
    fn pressure_is_clamped_and_nan_is_zero() {
        assert_eq!(Pressure::new(1.7).value(), 1.0);
        assert_eq!(Pressure::new(-0.2).value(), 0.0);
        assert_eq!(Pressure::new(f64::NAN).value(), 0.0);
        assert!((Pressure::new(0.25).value() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn pressure_from_a_device_axis_range() {
        assert!((Pressure::from_axis(4096.0, 0.0, 8192.0).value() - 0.5).abs() < 1e-12);
        assert!((Pressure::from_axis(100.0, 100.0, 200.0).value()).abs() < 1e-12);
        assert_eq!(Pressure::from_axis(300.0, 100.0, 200.0).value(), 1.0);
        assert_eq!(Pressure::from_axis(5.0, 10.0, 10.0).value(), 0.0);
    }

    #[test]
    fn tilt_axes_are_clamped_and_degrees_scale_to_a_fraction() {
        let tilt = Tilt::new(2.0, -3.0);
        assert_eq!((tilt.x(), tilt.y()), (1.0, -1.0));
        let tilt = Tilt::from_degrees(45.0, -90.0);
        assert!((tilt.x() - 0.5).abs() < 1e-12);
        assert!((tilt.y() + 1.0).abs() < 1e-12);
        assert_eq!(Tilt::new(f64::NAN, 0.1).x(), 0.0);
    }

    #[test]
    fn a_mouse_without_pressure_is_not_a_pen_event() {
        let mut event = raw(RawKind::Motion);
        event.source = RawSource::Other;
        event.pressure = None;
        assert!(!event.is_pen());
        assert!(event.sample(&mut TimeUnwrap::default()).is_none());
    }

    #[test]
    fn a_pressure_axis_makes_a_pen_even_when_the_source_reads_as_a_mouse() {
        let mut event = raw(RawKind::Motion);
        event.source = RawSource::Other;
        assert!(event.is_pen());
        let mut event = raw(RawKind::Motion);
        event.source = RawSource::Eraser;
        event.pressure = None;
        assert!(event.is_pen());
    }

    #[test]
    fn motion_is_contact_while_button_one_is_held_else_hover() {
        let mut event = raw(RawKind::Motion);
        assert_eq!(sample_of(&event).phase, PenPhase::Hover);
        event.buttons_before = 0b001;
        assert_eq!(sample_of(&event).phase, PenPhase::Contact);
    }

    #[test]
    fn press_and_release_of_button_one_switch_the_phase_at_the_event() {
        let mut event = raw(RawKind::Press(1));
        assert_eq!(sample_of(&event).phase, PenPhase::Contact);
        event.kind = RawKind::Release(1);
        event.buttons_before = 0b001;
        assert_eq!(sample_of(&event).phase, PenPhase::Hover);
    }

    #[test]
    fn proximity_out_is_out_and_in_is_hover() {
        assert_eq!(sample_of(&raw(RawKind::ProximityOut)).phase, PenPhase::Out);
        assert_eq!(sample_of(&raw(RawKind::ProximityIn)).phase, PenPhase::Hover);
    }

    #[test]
    fn barrel_bits_follow_buttons_two_and_three_including_the_event_itself() {
        let mut event = raw(RawKind::Motion);
        event.buttons_before = 0b110;
        assert_eq!(sample_of(&event).barrel, 0b11);
        event.kind = RawKind::Press(2);
        event.buttons_before = 0;
        assert_eq!(sample_of(&event).barrel, 0b01);
        event.kind = RawKind::Release(2);
        event.buttons_before = 0b010;
        assert_eq!(sample_of(&event).barrel, 0);
        event.kind = RawKind::Press(8);
        assert_eq!(sample_of(&event).barrel, 0b01);
    }

    #[test]
    fn the_eraser_comes_from_the_source_or_the_tool() {
        let mut event = raw(RawKind::Motion);
        assert_eq!(sample_of(&event).end, PenEnd::Tip);
        event.source = RawSource::Eraser;
        assert_eq!(sample_of(&event).end, PenEnd::Eraser);
        event.source = RawSource::Pen;
        event.eraser_tool = true;
        assert_eq!(sample_of(&event).end, PenEnd::Eraser);
    }

    #[test]
    fn tilt_and_pressure_are_absent_when_the_event_has_none() {
        let mut event = raw(RawKind::Motion);
        event.pressure = None;
        let sample = sample_of(&event);
        assert!(sample.pressure.is_none() && sample.tilt.is_none());
        event.tilt_x = Some(0.5);
        let tilt = sample_of(&event).tilt.expect("tilt");
        assert_eq!((tilt.x(), tilt.y()), (0.5, 0.0));
    }

    #[test]
    fn time_unwrap_runs_on_across_the_32_bit_wrap() {
        let mut clock = TimeUnwrap::default();
        let first = clock.unwrap(u32::MAX - 5);
        let second = clock.unwrap(10);
        assert!((second - first - 16.0).abs() < 1e-9);
        // A late event from before the wrap keeps the earlier round.
        let late = clock.unwrap(u32::MAX - 2);
        assert!((late - first - 3.0).abs() < 1e-9);
        assert!((clock.unwrap(12) - second - 2.0).abs() < 1e-9);
    }

    #[test]
    fn age_is_the_wrapping_difference_or_none_on_a_clock_mismatch() {
        assert_eq!(event_age_ms(1005, 1000), Some(5.0));
        assert_eq!(event_age_ms(3, u32::MAX - 1), Some(5.0));
        assert_eq!(event_age_ms(1000, 2000), None);
        assert_eq!(event_age_ms(1_000_000, 5), None);
    }

    #[test]
    fn the_batcher_empties_on_take_and_keeps_order() {
        let mut batcher = PenBatcher::default();
        assert!(batcher.take().is_none());
        for t in 0..3 {
            let mut event = raw(RawKind::Motion);
            event.time = t;
            batcher.push(sample_of(&event));
        }
        let batch = batcher.take().expect("batch");
        let times: Vec<f64> = batch.samples.iter().map(|s| s.time_ms).collect();
        assert_eq!(times, [0.0, 1.0, 2.0]);
        assert_eq!(batch.dropped, 0);
        assert!(batcher.is_empty() && batcher.take().is_none());
    }

    #[test]
    fn the_batcher_drops_the_oldest_past_the_limit_and_counts_them() {
        let mut batcher = PenBatcher::default();
        for t in 0..u32::try_from(PEN_BATCH_LIMIT + 3).expect("small") {
            let mut event = raw(RawKind::Motion);
            event.time = t;
            batcher.push(sample_of(&event));
        }
        let batch = batcher.take().expect("batch");
        assert_eq!(batch.samples.len(), PEN_BATCH_LIMIT);
        assert_eq!(batch.dropped, 3);
        assert!((batch.samples[0].time_ms - 3.0).abs() < 1e-9);
    }

    #[test]
    fn a_sample_serialises_flat_numbers() {
        let json = serde_json::to_value(sample_of(&raw(RawKind::Motion))).expect("json");
        assert_eq!(json["x"], 10.0);
        assert_eq!(json["pressure"], 0.5);
        assert_eq!(json["end"], "tip");
        assert_eq!(json["phase"], "hover");
        assert!(json["tilt"].is_null());
    }
}
