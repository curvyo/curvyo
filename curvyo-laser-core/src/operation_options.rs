//! The operation-only settings of an operation (ADR 0056 §6, `specs/0111` criteria 20, 21, 24):
//! their ranges, defaults, and the stored `op_options` map with its damage.

use serde_json::{Map, Value, json};

use crate::operation_kind::OperationKind;
use crate::steps::{exact_steps, rounded_steps, typed_steps};

/// The chip text of a typed value that is no number (UX section 10).
pub const ENTER_NUMBER: &str = "Enter a number.";
/// The chip text of a typed value that must be whole (UX section 10).
pub const ENTER_WHOLE_NUMBER: &str = "Enter a whole number.";

/// The scan angle of a hatch, -90.0 to 90.0 degrees with one decimal, kept in tenths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ScanAngle(i16);

impl ScanAngle {
    /// The smallest angle, in tenths of a degree.
    pub const MIN_TENTHS: i16 = -900;
    /// The largest angle, in tenths of a degree.
    pub const MAX_TENTHS: i16 = 900;
    /// The chip text of a typed angle out of range.
    pub const RANGE_TEXT: &'static str = "Scan angle is -90.0 to 90.0°.";

    /// The angle of `tenths` tenths of a degree, `None` outside -90 to 90.
    #[must_use]
    pub fn from_tenths(tenths: i64) -> Option<Self> {
        i16::try_from(tenths)
            .ok()
            .filter(|t| (Self::MIN_TENTHS..=Self::MAX_TENTHS).contains(t))
            .map(Self)
    }

    /// The exact stored angle `degrees`, `None` with more than one decimal or out of range.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Option<Self> {
        Self::from_tenths(exact_steps(degrees, 10.0)?)
    }

    /// A typed angle, rounded to 0.1°.
    ///
    /// # Errors
    /// The chip text.
    pub fn typed(degrees: f64) -> Result<Self, &'static str> {
        if !degrees.is_finite() {
            return Err(ENTER_NUMBER);
        }
        typed_steps(
            degrees,
            10.0,
            f64::from(Self::MIN_TENTHS),
            f64::from(Self::MAX_TENTHS),
        )
        .and_then(Self::from_tenths)
        .ok_or(Self::RANGE_TEXT)
    }

    /// Tenths of a degree.
    #[must_use]
    pub const fn tenths(self) -> i16 {
        self.0
    }

    /// The angle in degrees.
    #[must_use]
    pub fn degrees(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

/// How far the head runs past a shape's edge, 0 to 50 percent of the run-out its speed needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Overscan(u8);

impl Overscan {
    /// The largest overscan in percent.
    pub const MAX: u8 = 50;
    /// The default, 2 %.
    pub const DEFAULT: Self = Self(2);
    /// The chip text of a typed overscan out of range.
    pub const RANGE_TEXT: &'static str = "Overscanning is 0 to 50 %.";

    /// `percent`, `None` above 50.
    #[must_use]
    pub fn new(percent: u64) -> Option<Self> {
        u8::try_from(percent)
            .ok()
            .filter(|p| *p <= Self::MAX)
            .map(Self)
    }

    /// A typed overscan, a whole number.
    ///
    /// # Errors
    /// The chip text.
    pub fn typed(percent: f64) -> Result<Self, &'static str> {
        if !percent.is_finite() || percent.fract() != 0.0 {
            return Err(ENTER_WHOLE_NUMBER);
        }
        rounded_steps(percent, 1.0)
            .and_then(|p| u64::try_from(p).ok())
            .and_then(Self::new)
            .ok_or(Self::RANGE_TEXT)
    }

    /// The percent.
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }
}

impl Default for Overscan {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The focus height offset of a laser with a motorised Z axis, -20.0 to 20.0 mm with one decimal,
/// kept in tenths of a millimetre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ZOffset(i16);

impl ZOffset {
    /// The lowest offset, in tenths of a millimetre.
    pub const MIN_TENTHS: i16 = -200;
    /// The highest offset, in tenths of a millimetre.
    pub const MAX_TENTHS: i16 = 200;
    /// The chip text of a typed offset out of range.
    pub const RANGE_TEXT: &'static str = "Z offset is -20.0 to 20.0 mm.";

    /// The offset of `tenths` tenths of a millimetre, `None` outside -20 to 20 mm.
    #[must_use]
    pub fn from_tenths(tenths: i64) -> Option<Self> {
        i16::try_from(tenths)
            .ok()
            .filter(|t| (Self::MIN_TENTHS..=Self::MAX_TENTHS).contains(t))
            .map(Self)
    }

    /// The exact stored offset `mm`, `None` with more than one decimal or out of range.
    #[must_use]
    pub fn from_mm(mm: f64) -> Option<Self> {
        Self::from_tenths(exact_steps(mm, 10.0)?)
    }

    /// A typed offset, rounded to 0.1 mm.
    ///
    /// # Errors
    /// The chip text.
    pub fn typed(mm: f64) -> Result<Self, &'static str> {
        if !mm.is_finite() {
            return Err(ENTER_NUMBER);
        }
        typed_steps(
            mm,
            10.0,
            f64::from(Self::MIN_TENTHS),
            f64::from(Self::MAX_TENTHS),
        )
        .and_then(Self::from_tenths)
        .ok_or(Self::RANGE_TEXT)
    }

    /// Tenths of a millimetre.
    #[must_use]
    pub const fn tenths(self) -> i16 {
        self.0
    }

    /// The offset in mm.
    #[must_use]
    pub fn mm(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

/// The settings that belong to an operation and never to a material (`specs/0111` "Words").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationOptions {
    /// Hatch a second time at the scan angle plus 90 degrees (Engrave kinds only).
    pub crosshatch: bool,
    /// Alternate the hatch direction line by line (Engrave kinds only).
    pub bidirectional: bool,
    /// The hatch direction (Engrave kinds only).
    pub scan_angle: ScanAngle,
    /// The run past the edge (Engrave kinds only).
    pub overscan: Overscan,
    /// The focus height offset; runs only with a motorised Z axis.
    pub z_offset: ZOffset,
}

impl Default for OperationOptions {
    fn default() -> Self {
        Self {
            crosshatch: false,
            bidirectional: true,
            scan_angle: ScanAngle::default(),
            overscan: Overscan::DEFAULT,
            z_offset: ZOffset::default(),
        }
    }
}

/// Why a stored `op_options` does not read: the operation reads Damaged settings, runs nothing
/// and the value is kept and never repaired (ADR 0056 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsDamage {
    /// Not a map.
    NotAMap,
    /// A key this build does not know.
    UnknownKey,
    /// A value of the wrong type.
    WrongType,
    /// A number out of range or with too many decimals.
    BadValue,
    /// A hatch key on a kind other than Engrave or Engrave + Outline.
    HatchOnKind,
}

/// One edit of the lower part of an unfolded row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionEdit {
    /// Crosshatch on or off.
    Crosshatch(bool),
    /// Bidirectional fill on or off.
    Bidirectional(bool),
    /// A scan angle.
    ScanAngle(ScanAngle),
    /// An overscan.
    Overscan(Overscan),
    /// A Z offset.
    ZOffset(ZOffset),
}

const KEY_CROSSHATCH: &str = "crosshatch";
const KEY_BIDIRECTIONAL: &str = "bidirectional";
const KEY_SCAN_ANGLE: &str = "scan_angle_deg";
const KEY_OVERSCAN: &str = "overscan_pct";
const KEY_Z_OFFSET: &str = "z_offset_mm";
const HATCH_KEYS: [&str; 4] = [
    KEY_CROSSHATCH,
    KEY_BIDIRECTIONAL,
    KEY_SCAN_ANGLE,
    KEY_OVERSCAN,
];

/// Whether operations of `kind` hatch, so the four hatch options apply.
#[must_use]
pub const fn hatches(kind: OperationKind) -> bool {
    matches!(kind, OperationKind::Engrave | OperationKind::EngraveOutline)
}

fn flag(value: &Value) -> Result<bool, OptionsDamage> {
    value.as_bool().ok_or(OptionsDamage::WrongType)
}

fn number(value: &Value) -> Result<f64, OptionsDamage> {
    value.as_f64().ok_or(OptionsDamage::WrongType)
}

/// Reads a stored `op_options` of an operation of kind `kind` (ADR 0056 §6). `None` (absent)
/// reads every default; a register that is no plain value is handed in as `null` and reads as
/// not a map.
///
/// # Errors
/// The damage of a value that does not read.
pub fn read_options(
    value: Option<&Value>,
    kind: OperationKind,
) -> Result<OperationOptions, OptionsDamage> {
    let mut options = OperationOptions::default();
    let Some(value) = value else {
        return Ok(options);
    };
    let map = value.as_object().ok_or(OptionsDamage::NotAMap)?;
    for (key, value) in map {
        let key = key.as_str();
        if HATCH_KEYS.contains(&key) && !hatches(kind) {
            return Err(OptionsDamage::HatchOnKind);
        }
        match key {
            KEY_CROSSHATCH => options.crosshatch = flag(value)?,
            KEY_BIDIRECTIONAL => options.bidirectional = flag(value)?,
            KEY_SCAN_ANGLE => {
                options.scan_angle =
                    ScanAngle::from_degrees(number(value)?).ok_or(OptionsDamage::BadValue)?;
            }
            KEY_OVERSCAN => {
                let percent = value.as_u64().ok_or(OptionsDamage::WrongType)?;
                options.overscan = Overscan::new(percent).ok_or(OptionsDamage::BadValue)?;
            }
            KEY_Z_OFFSET => {
                options.z_offset =
                    ZOffset::from_mm(number(value)?).ok_or(OptionsDamage::BadValue)?;
            }
            _ => return Err(OptionsDamage::UnknownKey),
        }
    }
    Ok(options)
}

impl OperationOptions {
    /// The whole map to store for an operation of kind `kind`: every key the kind takes, so
    /// [`read_options`] reads it back equal.
    #[must_use]
    pub fn to_value(&self, kind: OperationKind) -> Value {
        let mut map = Map::new();
        if hatches(kind) {
            map.insert(KEY_CROSSHATCH.to_owned(), json!(self.crosshatch));
            map.insert(KEY_BIDIRECTIONAL.to_owned(), json!(self.bidirectional));
            map.insert(KEY_SCAN_ANGLE.to_owned(), json!(self.scan_angle.degrees()));
            map.insert(KEY_OVERSCAN.to_owned(), json!(self.overscan.percent()));
        }
        map.insert(KEY_Z_OFFSET.to_owned(), json!(self.z_offset.mm()));
        Value::Object(map)
    }

    /// The options with `edit` applied, for an operation of kind `kind`: `None` when nothing
    /// changes or for a hatch option on a kind that does not hatch.
    #[must_use]
    pub fn with(&self, kind: OperationKind, edit: OptionEdit) -> Option<Self> {
        let mut next = *self;
        match edit {
            OptionEdit::ZOffset(z) => next.z_offset = z,
            _ if !hatches(kind) => return None,
            OptionEdit::Crosshatch(on) => next.crosshatch = on,
            OptionEdit::Bidirectional(on) => next.bidirectional = on,
            OptionEdit::ScanAngle(angle) => next.scan_angle = angle,
            OptionEdit::Overscan(overscan) => next.overscan = overscan,
        }
        (next != *self).then_some(next)
    }

    /// The Z offset that runs: the stored one with a motorised Z axis, else none (criterion 21;
    /// the stored value is kept).
    #[must_use]
    pub const fn running_z_offset(&self, motorised_z: bool) -> Option<ZOffset> {
        if motorised_z {
            Some(self.z_offset)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(value: &Value, kind: OperationKind) -> Result<OperationOptions, OptionsDamage> {
        read_options(Some(value), kind)
    }

    #[test]
    fn absent_options_read_every_default() {
        let options = read_options(None, OperationKind::Engrave).unwrap();
        assert!(!options.crosshatch);
        assert!(options.bidirectional);
        assert_eq!(options.scan_angle.degrees(), 0.0);
        assert_eq!(options.overscan.percent(), 2);
        assert_eq!(options.z_offset.mm(), 0.0);
        assert_eq!(read(&json!({}), OperationKind::Cut), Ok(options));
    }

    #[test]
    fn a_full_engrave_map_reads_and_writes_back_the_same_plain_values() {
        let value = json!({
            "crosshatch": true, "bidirectional": false, "scan_angle_deg": 15.0,
            "overscan_pct": 50, "z_offset_mm": -1.5,
        });
        let options = read(&value, OperationKind::EngraveOutline).unwrap();
        assert!(options.crosshatch);
        assert!(!options.bidirectional);
        assert_eq!(options.scan_angle.tenths(), 150);
        assert_eq!(options.overscan.percent(), 50);
        assert_eq!(options.z_offset.tenths(), -15);
        assert_eq!(options.to_value(OperationKind::EngraveOutline), value);
    }

    #[test]
    fn a_cut_takes_the_z_offset_only() {
        let value = json!({"z_offset_mm": 20.0});
        let options = read(&value, OperationKind::Cut).unwrap();
        assert_eq!(options.z_offset.tenths(), 200);
        assert_eq!(options.to_value(OperationKind::Cut), value);
        for hatch in HATCH_KEYS {
            let mut map = Map::new();
            map.insert(hatch.to_owned(), json!(false));
            for kind in [
                OperationKind::Cut,
                OperationKind::Score,
                OperationKind::CutOut,
            ] {
                assert_eq!(
                    read(&Value::Object(map.clone()), kind),
                    Err(OptionsDamage::HatchOnKind),
                    "{hatch} on {kind:?}"
                );
            }
        }
    }

    #[test]
    fn every_damage_reads_as_damage() {
        let engrave = OperationKind::Engrave;
        let cases = [
            (json!([1]), OptionsDamage::NotAMap),
            (json!(null), OptionsDamage::NotAMap),
            (json!({"dither": true}), OptionsDamage::UnknownKey),
            (json!({"crosshatch": 1}), OptionsDamage::WrongType),
            (json!({"scan_angle_deg": "15"}), OptionsDamage::WrongType),
            (json!({"overscan_pct": 2.5}), OptionsDamage::WrongType),
            (json!({"overscan_pct": -1}), OptionsDamage::WrongType),
            (json!({"scan_angle_deg": 90.1}), OptionsDamage::BadValue),
            (json!({"scan_angle_deg": -90.1}), OptionsDamage::BadValue),
            (json!({"scan_angle_deg": 12.25}), OptionsDamage::BadValue),
            (json!({"overscan_pct": 51}), OptionsDamage::BadValue),
            (json!({"z_offset_mm": 20.1}), OptionsDamage::BadValue),
            (json!({"z_offset_mm": 0.05}), OptionsDamage::BadValue),
        ];
        for (value, damage) in cases {
            assert_eq!(read(&value, engrave), Err(damage), "{value}");
        }
        assert_eq!(
            read(
                &json!({"scan_angle_deg": -90.0, "z_offset_mm": -20.0}),
                engrave
            )
            .unwrap()
            .scan_angle
            .tenths(),
            -900
        );
    }

    #[test]
    fn typed_values_round_to_their_decimals_and_name_their_range() {
        assert_eq!(ScanAngle::typed(15.04).unwrap().tenths(), 150);
        assert_eq!(ScanAngle::typed(90.1), Err("Scan angle is -90.0 to 90.0°."));
        assert_eq!(ScanAngle::typed(f64::NAN), Err("Enter a number."));
        assert_eq!(Overscan::typed(50.0).unwrap().percent(), 50);
        assert_eq!(Overscan::typed(51.0), Err("Overscanning is 0 to 50 %."));
        assert_eq!(Overscan::typed(-1.0), Err("Overscanning is 0 to 50 %."));
        assert_eq!(Overscan::typed(2.5), Err("Enter a whole number."));
        assert_eq!(ZOffset::typed(1.54).unwrap().tenths(), 15);
        assert_eq!(ZOffset::typed(-20.5), Err("Z offset is -20.0 to 20.0 mm."));
        // Out of range before rounding is refused, never rounded into the range (criterion 24).
        assert_eq!(
            ScanAngle::typed(90.04),
            Err("Scan angle is -90.0 to 90.0°.")
        );
        assert_eq!(ScanAngle::typed(-90.0).unwrap().tenths(), -900);
        assert_eq!(ZOffset::typed(-20.04), Err("Z offset is -20.0 to 20.0 mm."));
        assert_eq!(ZOffset::typed(20.0).unwrap().tenths(), 200);
        for text in [
            ScanAngle::RANGE_TEXT,
            Overscan::RANGE_TEXT,
            ZOffset::RANGE_TEXT,
        ] {
            assert!(text.chars().count() <= 70);
        }
    }

    #[test]
    fn an_edit_changes_one_value_and_a_hatch_edit_on_a_cut_is_refused() {
        let options = OperationOptions::default();
        let on = options
            .with(OperationKind::Engrave, OptionEdit::Crosshatch(true))
            .unwrap();
        assert!(on.crosshatch);
        assert_eq!(
            options.with(OperationKind::Engrave, OptionEdit::Bidirectional(true)),
            None,
            "unchanged"
        );
        assert_eq!(
            options.with(OperationKind::Cut, OptionEdit::Crosshatch(true)),
            None
        );
        let z = ZOffset::typed(1.5).unwrap();
        let cut = options
            .with(OperationKind::Cut, OptionEdit::ZOffset(z))
            .unwrap();
        assert_eq!(
            cut.to_value(OperationKind::Cut),
            json!({"z_offset_mm": 1.5})
        );
        assert_eq!(cut.running_z_offset(false), None, "kept, runs nothing");
        assert_eq!(cut.running_z_offset(true), Some(z));
    }
}
