//! The operation-only settings of an operation (ADR 0056 §6, `specs/0111` criteria 20, 21, 24):
//! their ranges, defaults, and the stored `op_options` map with its damage.

use serde_json::{Map, Value, json};

use crate::operation_cut_options::{
    CUT_KEYS, CutEdit, CutSettings, cuts_through, read_cut_key, write_cut_keys,
};
use crate::operation_image_options::{
    IMAGE_KEYS, ImageEdit, ImageSettings, read_image_key, write_image_keys,
};
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
    /// Hatch a second time at the scan angle plus 90 degrees (Engrave and Engrave + Outline
    /// only).
    pub crosshatch: bool,
    /// Alternate the hatch or row direction line by line (Engrave kinds and Image).
    pub bidirectional: bool,
    /// The hatch or row direction (Engrave kinds and Image).
    pub scan_angle: ScanAngle,
    /// The run past the edge (Engrave kinds and Image).
    pub overscan: Overscan,
    /// The focus height offset; runs only with a motorised Z axis.
    pub z_offset: ZOffset,
    /// The Image settings of a raster item (Image only, `specs/0123`; ADR 0069 §13).
    pub image: ImageSettings,
    /// The cut path settings (Cut and Cut-out only, `specs/0139` adrs L2).
    pub cut: CutSettings,
}

impl Default for OperationOptions {
    fn default() -> Self {
        Self {
            crosshatch: false,
            bidirectional: true,
            scan_angle: ScanAngle::default(),
            overscan: Overscan::DEFAULT,
            z_offset: ZOffset::default(),
            image: ImageSettings::DEFAULT,
            cut: CutSettings::DEFAULT,
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
    /// A key on a kind that does not take it (ADR 0069 §13): Crosshatch off the Engrave kinds,
    /// a scan key off the Engrave kinds and Image, an Image settings key off Image.
    KeyOnKind,
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
    /// One of the Image settings (Image only, `specs/0123` criteria 11, 12).
    Image(ImageEdit),
    /// One of the cut path settings (Cut and Cut-out only, `specs/0139` criterion 1).
    Cut(CutEdit),
}

const KEY_CROSSHATCH: &str = "crosshatch";
const KEY_BIDIRECTIONAL: &str = "bidirectional";
const KEY_SCAN_ANGLE: &str = "scan_angle_deg";
const KEY_OVERSCAN: &str = "overscan_pct";
const KEY_Z_OFFSET: &str = "z_offset_mm";
const SCAN_KEYS: [&str; 3] = [KEY_BIDIRECTIONAL, KEY_SCAN_ANGLE, KEY_OVERSCAN];

/// Whether operations of `kind` hatch areas, so Crosshatch applies: the Engrave kinds.
#[must_use]
pub const fn hatches(kind: OperationKind) -> bool {
    matches!(kind, OperationKind::Engrave | OperationKind::EngraveOutline)
}

/// Whether operations of `kind` scan in rows, so Bidirectional fill, Scan angle and
/// Overscanning apply: the Engrave kinds and Image (ADR 0069 §13).
#[must_use]
pub const fn scans(kind: OperationKind) -> bool {
    hatches(kind) || matches!(kind, OperationKind::Image)
}

/// Whether `key` is valid on an operation of `kind`; `true` for a key no kind owns.
fn key_on_kind(key: &str, kind: OperationKind) -> bool {
    if key == KEY_CROSSHATCH {
        hatches(kind)
    } else if SCAN_KEYS.contains(&key) {
        scans(kind)
    } else if IMAGE_KEYS.contains(&key) {
        kind == OperationKind::Image
    } else if CUT_KEYS.contains(&key) {
        cuts_through(kind)
    } else {
        true
    }
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
        if !key_on_kind(key, kind) {
            return Err(OptionsDamage::KeyOnKind);
        }
        if read_image_key(&mut options.image, key, value)?
            || read_cut_key(&mut options.cut, key, value)?
        {
            continue;
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
    /// The whole map to store for an operation of kind `kind`: every key the kind takes (the cut
    /// path keys only when one differs from its default), so [`read_options`] reads it back equal.
    #[must_use]
    pub fn to_value(&self, kind: OperationKind) -> Value {
        let mut map = Map::new();
        if hatches(kind) {
            map.insert(KEY_CROSSHATCH.to_owned(), json!(self.crosshatch));
        }
        if scans(kind) {
            map.insert(KEY_BIDIRECTIONAL.to_owned(), json!(self.bidirectional));
            map.insert(KEY_SCAN_ANGLE.to_owned(), json!(self.scan_angle.degrees()));
            map.insert(KEY_OVERSCAN.to_owned(), json!(self.overscan.percent()));
        }
        if kind == OperationKind::Image {
            write_image_keys(&self.image, &mut map);
        }
        // Only settings that differ from the defaults are written, so a project without tabs
        // stores the same map as before `specs/0139` (criterion 24).
        if cuts_through(kind) && self.cut != CutSettings::DEFAULT {
            write_cut_keys(self.cut, &mut map);
        }
        map.insert(KEY_Z_OFFSET.to_owned(), json!(self.z_offset.mm()));
        Value::Object(map)
    }

    /// The options with `edit` applied, for an operation of kind `kind`: `None` when nothing
    /// changes or for an option the kind does not take (ADR 0069 §13).
    #[must_use]
    pub fn with(&self, kind: OperationKind, edit: OptionEdit) -> Option<Self> {
        let mut next = *self;
        match edit {
            OptionEdit::ZOffset(z) => next.z_offset = z,
            OptionEdit::Crosshatch(on) if hatches(kind) => next.crosshatch = on,
            OptionEdit::Bidirectional(on) if scans(kind) => next.bidirectional = on,
            OptionEdit::ScanAngle(angle) if scans(kind) => next.scan_angle = angle,
            OptionEdit::Overscan(overscan) if scans(kind) => next.overscan = overscan,
            OptionEdit::Image(edit) if kind == OperationKind::Image => {
                next.image = next.image.with(edit);
            }
            OptionEdit::Cut(edit) if cuts_through(kind) => next.cut = next.cut.with(edit),
            _ => return None,
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
#[path = "operation_options_tests.rs"]
mod tests;
