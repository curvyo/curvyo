//! The embroidery operation kinds and their stored numbers and thread colour
//! (`specs/0160-embroidery` criteria 5, 6, 20, `adrs.md` EM5): reading `op_settings` with damage
//! as the default, writing it, and checking a typed value against its range.

use curvyo_inkstitch_core::{Angle, Mm};
use serde_json::{Map, Value};

/// The `op_settings` key of the thread colour (Proposal: no colour register exists on an
/// operation node, so the colour rides with the numbers).
pub const KEY_THREAD: &str = "thread_colour";

/// What an operation of an embroidery page does (Frage 428).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpKind {
    /// Single stitches of a set length along each line.
    Running,
    /// Each closed area filled with staggered rows.
    Tatami,
    /// A zigzag column along each line, as wide as its stroke.
    Satin,
}

impl OpKind {
    /// Every kind, in Select-bar order.
    pub const ALL: [Self; 3] = [Self::Running, Self::Tatami, Self::Satin];

    /// The stored `op_kind`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Tatami => "tatami",
            Self::Satin => "satin",
        }
    }

    /// The kind a stored `op_kind` names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }

    /// The name and the start of a new operation's name ("Running stitch 1").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Running => "Running stitch",
            Self::Tatami => "Tatami fill",
            Self::Satin => "Satin",
        }
    }

    /// The fields of the kind, in panel order.
    #[must_use]
    pub const fn fields(self) -> &'static [Field] {
        match self {
            Self::Running => &[Field::StitchLength, Field::Repeats],
            Self::Tatami => &[Field::Angle, Field::RowSpacing, Field::MaxStitch],
            Self::Satin => &[Field::ZigzagSpacing],
        }
    }
}

/// A thread colour as 0xRRGGBB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ThreadColour(u32);

impl ThreadColour {
    /// The colour of `r`, `g`, `b`.
    #[must_use]
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
    }

    /// The colour a "#RRGGBB" text names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        (hex.len() == 6)
            .then(|| u32::from_str_radix(hex, 16).ok())
            .flatten()
            .map(Self)
    }

    /// "#RRGGBB".
    #[must_use]
    pub fn hex(self) -> String {
        format!("#{:06X}", self.0)
    }

    /// The red, green and blue bytes.
    #[must_use]
    pub fn channels(self) -> (u8, u8, u8) {
        let byte = |shift: u32| u8::try_from((self.0 >> shift) & 0xFF).unwrap_or(u8::MAX);
        (byte(16), byte(8), byte(0))
    }
}

/// A field of an operation's numbers (criterion 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    /// Running stitch: Stitch length, 1.0 to 10.0 mm.
    StitchLength,
    /// Running stitch: Repeats 1, 3 or 5.
    Repeats,
    /// Tatami fill: Angle, 0 to 179 degrees.
    Angle,
    /// Tatami fill: Row spacing, 0.10 to 2.00 mm.
    RowSpacing,
    /// Tatami fill: Max stitch, 1.0 to 10.0 mm.
    MaxStitch,
    /// Satin: Zigzag spacing, 0.20 to 2.00 mm.
    ZigzagSpacing,
}

/// How a field reads: its stored key, its raw unit per stored unit, the step, range and default
/// in raw units, and its message.
struct Spec {
    key: &'static str,
    scale: f64,
    step: u16,
    min: u16,
    max: u16,
    default: u16,
    message: &'static str,
}

impl Field {
    /// Every field.
    pub const ALL: [Self; 6] = [
        Self::StitchLength,
        Self::Repeats,
        Self::Angle,
        Self::RowSpacing,
        Self::MaxStitch,
        Self::ZigzagSpacing,
    ];

    const fn spec(self) -> Spec {
        // Lengths are raw hundredths of a mm, the angle whole degrees, repeats a count.
        let (key, scale, step, min, max, default, message) = match self {
            Self::StitchLength => (
                "stitch_length_mm",
                100.0,
                10,
                100,
                1_000,
                250,
                "Stitch length is 1.0 to 10.0 mm.",
            ),
            Self::Repeats => ("repeats", 1.0, 2, 1, 5, 1, "Repeats is 1, 3 or 5."),
            Self::Angle => ("angle_deg", 1.0, 1, 0, 179, 0, "Angle is 0 to 179°."),
            Self::RowSpacing => (
                "row_spacing_mm",
                100.0,
                1,
                10,
                200,
                25,
                "Row spacing is 0.10 to 2.00 mm.",
            ),
            Self::MaxStitch => (
                "max_stitch_mm",
                100.0,
                10,
                100,
                1_000,
                300,
                "Max stitch is 1.0 to 10.0 mm.",
            ),
            Self::ZigzagSpacing => (
                "zigzag_spacing_mm",
                100.0,
                1,
                20,
                200,
                40,
                "Zigzag spacing is 0.20 to 2.00 mm.",
            ),
        };
        Spec {
            key,
            scale,
            step,
            min,
            max,
            default,
            message,
        }
    }

    /// The stored key (`stitch_length_mm`).
    #[must_use]
    pub const fn key(self) -> &'static str {
        self.spec().key
    }

    /// The field a stored key names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    /// The raw value of `value` in stored units when it lies on the field's step within its
    /// range: `None` otherwise.
    fn raw(self, value: f64) -> Option<u16> {
        let spec = self.spec();
        let scaled = (value * spec.scale).round();
        if !scaled.is_finite() || scaled < f64::from(spec.min) || scaled > f64::from(spec.max) {
            return None;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: within min..=max, which fit u16
        let raw = scaled as u16;
        (raw - spec.min).is_multiple_of(spec.step).then_some(raw)
    }

    /// The stored value of a raw one.
    fn stored(self, raw: u16) -> Value {
        let spec = self.spec();
        if spec.scale > 1.0 {
            Value::from(f64::from(raw) / spec.scale)
        } else {
            Value::from(raw)
        }
    }
}

/// The numbers of one operation (criterion 6); every kind's fields, the kind's own ones used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbroiderySettings {
    raw: [u16; 6],
}

impl Default for EmbroiderySettings {
    fn default() -> Self {
        Self {
            raw: Field::ALL.map(|f| f.spec().default),
        }
    }
}

impl EmbroiderySettings {
    fn at(self, field: Field) -> u16 {
        self.raw[field as usize]
    }

    fn hundredths(self, field: Field) -> Mm {
        Mm::saturating(f64::from(self.at(field)) / 100.0)
    }

    /// Running stitch: the stitch length.
    #[must_use]
    pub fn stitch_length(self) -> Mm {
        self.hundredths(Field::StitchLength)
    }

    /// Running stitch: how often each line runs (1, 3 or 5).
    #[must_use]
    pub fn repeats(self) -> u8 {
        u8::try_from(self.at(Field::Repeats)).unwrap_or(1)
    }

    /// Tatami fill: the row angle.
    #[must_use]
    pub fn angle(self) -> Angle {
        Angle::from_degrees(f64::from(self.at(Field::Angle))).unwrap_or_default()
    }

    /// Tatami fill: the angle in whole degrees.
    #[must_use]
    pub fn angle_degrees(self) -> u16 {
        self.at(Field::Angle)
    }

    /// Tatami fill: the row spacing.
    #[must_use]
    pub fn row_spacing(self) -> Mm {
        self.hundredths(Field::RowSpacing)
    }

    /// Tatami fill: the longest stitch along a row.
    #[must_use]
    pub fn max_stitch(self) -> Mm {
        self.hundredths(Field::MaxStitch)
    }

    /// Satin: the zigzag spacing.
    #[must_use]
    pub fn zigzag_spacing(self) -> Mm {
        self.hundredths(Field::ZigzagSpacing)
    }

    /// The numbers of `kind` and the thread colour stored in `stored`: a missing value reads as
    /// its default, a damaged one (no number, off the step, out of range, no "#RRGGBB") as its
    /// default and counted (EM5). No stored colour reads as black.
    #[must_use]
    pub fn read(kind: OpKind, stored: &Map<String, Value>) -> (Self, ThreadColour, usize) {
        let mut settings = Self::default();
        let mut damaged = 0;
        for &field in kind.fields() {
            let Some(value) = stored.get(field.key()) else {
                continue;
            };
            match value.as_f64().and_then(|v| field.raw(v)) {
                Some(raw) => settings.raw[field as usize] = raw,
                None => damaged += 1,
            }
        }
        let thread = match stored.get(KEY_THREAD) {
            None => ThreadColour::default(),
            Some(value) => value
                .as_str()
                .and_then(ThreadColour::parse)
                .unwrap_or_else(|| {
                    damaged += 1;
                    ThreadColour::default()
                }),
        };
        (settings, thread, damaged)
    }

    /// The `op_settings` map of `kind` with the thread colour.
    #[must_use]
    pub fn write(self, kind: OpKind, thread: ThreadColour) -> Map<String, Value> {
        let mut map = Map::new();
        for &field in kind.fields() {
            map.insert(field.key().to_owned(), field.stored(self.at(field)));
        }
        map.insert(KEY_THREAD.to_owned(), Value::from(thread.hex()));
        map
    }
}

/// The settings `settings` with the field `key` set to the typed `text`, or the field message
/// (`ux-notes.md` 10).
///
/// # Errors
/// The message to show when the text is no number, off the field's step or out of range, or the
/// field is not one of `kind`'s.
pub fn edit_setting(
    kind: OpKind,
    settings: EmbroiderySettings,
    key: &str,
    text: &str,
) -> Result<EmbroiderySettings, String> {
    let field = Field::parse(key)
        .filter(|f| kind.fields().contains(f))
        .ok_or_else(|| format!("{} has no such number.", kind.label()))?;
    let value: f64 = text
        .trim()
        .parse()
        .map_err(|_| "Enter a number.".to_owned())?;
    let spec = field.spec();
    // A typed value is taken at the field's precision (Stitch length 2.56 is 2.6).
    let step = f64::from(spec.step) / spec.scale;
    let snapped = if field == Field::Repeats {
        value
    } else {
        (value / step).round() * step
    };
    let raw = field.raw(snapped).ok_or_else(|| spec.message.to_owned())?;
    let mut out = settings;
    out.raw[field as usize] = raw;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_ink_stitch_s_and_write_and_read_back() {
        let s = EmbroiderySettings::default();
        assert_eq!(s.stitch_length().get(), 2.5);
        assert_eq!(s.repeats(), 1);
        assert_eq!(s.angle_degrees(), 0);
        assert_eq!(s.row_spacing().get(), 0.25);
        assert_eq!(s.max_stitch().get(), 3.0);
        assert_eq!(s.zigzag_spacing().get(), 0.4);
        let red = ThreadColour::parse("#C8102E").unwrap();
        let map = s.write(OpKind::Tatami, red);
        assert_eq!(
            serde_json::to_string(&map).unwrap(),
            r##"{"angle_deg":0,"max_stitch_mm":3.0,"row_spacing_mm":0.25,"thread_colour":"#C8102E"}"##
        );
        assert_eq!(EmbroiderySettings::read(OpKind::Tatami, &map), (s, red, 0));
    }

    #[test]
    fn a_damaged_number_reads_as_its_default_with_a_count() {
        let mut map = Map::new();
        map.insert("row_spacing_mm".into(), Value::from(5.0));
        map.insert("angle_deg".into(), Value::from(45));
        map.insert(KEY_THREAD.into(), Value::from("red"));
        let (s, thread, damaged) = EmbroiderySettings::read(OpKind::Tatami, &map);
        assert_eq!(damaged, 2);
        assert_eq!(s.row_spacing().get(), 0.25);
        assert_eq!(s.angle_degrees(), 45);
        assert_eq!(thread, ThreadColour::default());
    }

    #[test]
    fn a_typed_value_outside_its_range_is_refused_with_the_range() {
        let s = EmbroiderySettings::default();
        let edit = |kind, key, text| edit_setting(kind, s, key, text);
        assert_eq!(
            edit(OpKind::Running, "stitch_length_mm", "11"),
            Err("Stitch length is 1.0 to 10.0 mm.".to_owned())
        );
        assert_eq!(
            edit(OpKind::Running, "repeats", "2"),
            Err("Repeats is 1, 3 or 5.".to_owned())
        );
        assert_eq!(
            edit(OpKind::Tatami, "angle_deg", "180"),
            Err("Angle is 0 to 179°.".to_owned())
        );
        assert_eq!(
            edit(OpKind::Tatami, "row_spacing_mm", "0.05"),
            Err("Row spacing is 0.10 to 2.00 mm.".to_owned())
        );
        assert_eq!(
            edit(OpKind::Satin, "zigzag_spacing_mm", "x"),
            Err("Enter a number.".to_owned())
        );
        let edited = edit(OpKind::Running, "stitch_length_mm", "2.56").unwrap();
        assert_eq!(edited.stitch_length().get(), 2.6);
        assert_eq!(edit(OpKind::Running, "repeats", "3").unwrap().repeats(), 3);
        assert!(edit(OpKind::Satin, "repeats", "3").is_err());
    }
}
