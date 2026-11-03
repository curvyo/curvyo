//! The laser machine profile record with its limits and the built-in (ADR 0036 §2, §4, §5;
//! `specs/0026-machine-profile` criteria 1, 2, 3 and 9).

use std::fmt::Write as _;

use curvyo_document_core::{DocumentSize, Length};
use serde::{Deserialize, Serialize};

/// The id of the built-in profile; not a UUID, so no record file can carry it.
pub const GENERIC_LASER_ID: &str = "generic-laser";
/// The name of the built-in profile and the name a damaged one reads as.
pub const GENERIC_LASER_NAME: &str = "Generic laser";
/// The longest profile name, in characters.
pub const MAX_NAME_CHARS: usize = 60;
/// The longest connector address or controller port, in characters.
pub const MAX_CONNECTION_CHARS: usize = 200;
/// The smallest work area side in mm (`0093`'s range, unchanged).
pub const MIN_WORK_MM: f64 = 1.0;
/// The largest work area side in mm.
pub const MAX_WORK_MM: f64 = 10_000.0;
/// The work area of a new profile in mm.
pub const DEFAULT_WORK_MM: (f64, f64) = (400.0, 300.0);

/// The corner of the bed where the machine's origin is (its home).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginCorner {
    /// Top left.
    TopLeft,
    /// Top right.
    TopRight,
    /// Bottom left, the usual GRBL home.
    #[default]
    BottomLeft,
    /// Bottom right.
    BottomRight,
}

impl OriginCorner {
    /// Every corner, in the order of the corner control (UX notes §6).
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    /// The stored spelling (`bottom_left`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
        }
    }

    /// The corner a stored spelling names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|corner| corner.name() == name)
    }

    /// Whether the corner lies on the page's right edge.
    #[must_use]
    pub const fn is_right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    /// Whether the corner lies on the page's bottom edge.
    #[must_use]
    pub const fn is_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }
}

/// Defines an integer newtype with its range and default.
macro_rules! bounded_integer {
    ($(#[$doc:meta])* $name:ident($int:ty), $min:expr, $max:expr, $default:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(try_from = "u64", into = "u64")]
        pub struct $name($int);

        impl $name {
            /// The smallest value.
            pub const MIN: $int = $min;
            /// The largest value.
            pub const MAX: $int = $max;
            /// The value of a new profile.
            pub const DEFAULT: Self = Self($default);

            /// The value `n`, `None` outside the range.
            #[must_use]
            pub fn new(n: u64) -> Option<Self> {
                <$int>::try_from(n)
                    .ok()
                    .filter(|n| (Self::MIN..=Self::MAX).contains(n))
                    .map(Self)
            }

            /// The plain number.
            #[must_use]
            pub const fn get(self) -> $int {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::DEFAULT
            }
        }

        impl TryFrom<u64> for $name {
            type Error = &'static str;
            fn try_from(n: u64) -> Result<Self, Self::Error> {
                Self::new(n).ok_or("out of range")
            }
        }

        impl From<$name> for u64 {
            fn from(value: $name) -> Self {
                Self::from(value.0)
            }
        }
    };
}

bounded_integer!(
    /// The controller's maximum feed rate in mm/min.
    FeedRate(u32), 1, 100_000, 6_000
);
bounded_integer!(
    /// The controller's maximum power value (GRBL's S-max).
    PowerValue(u16), 1, 65_535, 1_000
);
bounded_integer!(
    /// The serial baud rate of the controller.
    BaudRate(u32), 1, 4_000_000, 115_200
);

/// The values a manufacturing copies from a profile (ADR 0037 §1), the work area aside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileValues {
    /// The profile name, 1 to 60 characters; not unique.
    pub name: String,
    /// The origin corner.
    pub origin: OriginCorner,
    /// The maximum feed rate.
    pub max_feed: FeedRate,
    /// The maximum power value.
    pub max_power: PowerValue,
    /// Laser mode (dynamic power, GRBL `$32`).
    pub laser_mode: bool,
}

impl Default for ProfileValues {
    fn default() -> Self {
        Self {
            name: GENERIC_LASER_NAME.to_owned(),
            origin: OriginCorner::default(),
            max_feed: FeedRate::DEFAULT,
            max_power: PowerValue::DEFAULT,
            laser_mode: true,
        }
    }
}

/// How the profile reaches its machine: stored data only, read by `0064` (criterion 9).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connection {
    /// The connector's address, up to 200 characters.
    pub connector_address: String,
    /// The controller port (`/dev/ttyUSB0`, `COM3`), up to 200 characters.
    pub controller_port: String,
    /// The baud rate.
    pub baud_rate: BaudRate,
}

/// One machine profile of the library (ADR 0036 §2).
#[derive(Debug, Clone, PartialEq)]
pub struct MachineProfile {
    /// A lowercase UUID v4, or [`GENERIC_LASER_ID`] for the built-in.
    pub id: String,
    /// The last change in Unix milliseconds, passed in by the caller.
    pub modified: i64,
    /// The copied values.
    pub values: ProfileValues,
    /// The work area, each side within [`MIN_WORK_MM`]`..=`[`MAX_WORK_MM`].
    pub work_area: DocumentSize,
    /// The connection.
    pub connection: Connection,
}

impl MachineProfile {
    /// The built-in Generic laser with the defaults of criterion 1.
    #[must_use]
    pub fn generic_laser() -> Self {
        Self {
            id: GENERIC_LASER_ID.to_owned(),
            modified: 0,
            values: ProfileValues::default(),
            work_area: DocumentSize::from_mm(DEFAULT_WORK_MM.0, DEFAULT_WORK_MM.1),
            connection: Connection::default(),
        }
    }

    /// Whether this is the built-in.
    #[must_use]
    pub fn is_builtin(&self) -> bool {
        self.id == GENERIC_LASER_ID
    }
}

/// An accepted value and the notice that says it was limited or rounded, if it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Limited<T> {
    /// The value that is set.
    pub value: T,
    /// "Max feed limited to 100,000 mm/min.", `None` when the value is as typed.
    pub notice: Option<String>,
}

/// `n` with a comma between thousands ("100,000").
#[must_use]
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A typed whole number rounded and set into `min..=max` (criterion 1: permit over refuse);
/// `None` for a number that is not finite.
fn limited_integer(
    label: &str,
    unit: &str,
    typed: f64,
    min: u64,
    max: u64,
) -> Option<Limited<u64>> {
    if !typed.is_finite() {
        return None;
    }
    let rounded = typed.round();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    // The rounded number is clamped into the u64 range before the cast.
    let value = rounded.clamp(min as f64, max as f64) as u64;
    #[allow(clippy::cast_precision_loss)] // the bounds are far below 2^53
    let notice = if (value as f64 - rounded).abs() > 0.0 {
        Some(format!("{label} limited to {}{unit}.", grouped(value)))
    } else if (rounded - typed).abs() > 0.0 {
        Some(format!("{label} rounded to {}.", grouped(value)))
    } else {
        None
    };
    Some(Limited { value, notice })
}

/// A typed max feed, rounded and set into its range.
#[must_use]
pub fn clamp_feed(typed: f64) -> Option<Limited<FeedRate>> {
    let limited = limited_integer(
        "Max feed",
        " mm/min",
        typed,
        FeedRate::MIN.into(),
        FeedRate::MAX.into(),
    )?;
    Some(Limited {
        value: FeedRate::new(limited.value)?,
        notice: limited.notice,
    })
}

/// A typed max power value, rounded and set into its range.
#[must_use]
pub fn clamp_power(typed: f64) -> Option<Limited<PowerValue>> {
    let limited = limited_integer(
        "Max power",
        "",
        typed,
        PowerValue::MIN.into(),
        PowerValue::MAX.into(),
    )?;
    Some(Limited {
        value: PowerValue::new(limited.value)?,
        notice: limited.notice,
    })
}

/// A typed baud rate, rounded and set into its range.
#[must_use]
pub fn clamp_baud(typed: f64) -> Option<Limited<BaudRate>> {
    let limited = limited_integer(
        "Baud rate",
        "",
        typed,
        BaudRate::MIN.into(),
        BaudRate::MAX.into(),
    )?;
    Some(Limited {
        value: BaudRate::new(limited.value)?,
        notice: limited.notice,
    })
}

/// A typed work area side (`label` "Width" or "Height") set into its range; `None` for a number
/// that is not finite.
#[must_use]
pub fn clamp_work_side(label: &str, typed: Length) -> Option<Limited<Length>> {
    let mm = typed.as_mm();
    if !mm.is_finite() {
        return None;
    }
    let value = mm.clamp(MIN_WORK_MM, MAX_WORK_MM);
    // Exact: `clamp` returns its input when it lies inside; the bounds are whole numbers.
    #[allow(
        clippy::float_cmp,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let notice = (value != mm).then(|| format!("{label} limited to {} mm.", grouped(value as u64)));
    Some(Limited {
        value: Length::from_mm(value),
        notice,
    })
}

/// The work area `area` set into the profile range, with the first notice.
#[must_use]
pub fn clamp_work_area(area: DocumentSize) -> Limited<DocumentSize> {
    let default = DocumentSize::from_mm(DEFAULT_WORK_MM.0, DEFAULT_WORK_MM.1);
    let width = clamp_work_side("Width", area.width);
    let height = clamp_work_side("Height", area.height);
    let notice = [&width, &height]
        .into_iter()
        .find_map(|side| side.as_ref().and_then(|s| s.notice.clone()));
    Limited {
        value: DocumentSize::new(
            width.map_or(default.width, |w| w.value),
            height.map_or(default.height, |h| h.value),
        ),
        notice,
    }
}

/// A text cut to `max` characters, with "<label> limited to <max> characters." when it was.
fn cut_text(label: &str, typed: &str, max: usize) -> Limited<String> {
    if typed.chars().count() <= max {
        return Limited {
            value: typed.to_owned(),
            notice: None,
        };
    }
    Limited {
        value: typed.chars().take(max).collect(),
        notice: Some(format!("{label} limited to {max} characters.")),
    }
}

/// A typed profile name, trimmed and cut to 60 characters; `None` for an empty one, which
/// reverts the field.
#[must_use]
pub fn clamp_name(typed: &str) -> Option<Limited<String>> {
    let trimmed = typed.trim();
    (!trimmed.is_empty()).then(|| cut_text("Name", trimmed, MAX_NAME_CHARS))
}

/// A typed connector address cut to 200 characters.
#[must_use]
pub fn clamp_address(typed: &str) -> Limited<String> {
    cut_text("Address", typed, MAX_CONNECTION_CHARS)
}

/// A typed controller port cut to 200 characters.
#[must_use]
pub fn clamp_port(typed: &str) -> Limited<String> {
    cut_text("Port", typed, MAX_CONNECTION_CHARS)
}

/// Whether `name` is a valid stored name: 1 to 60 characters, not blank.
#[must_use]
pub fn is_valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= MAX_NAME_CHARS
}

/// Whether `mm` is a valid stored work area side.
#[must_use]
pub fn is_valid_work_side(mm: f64) -> bool {
    mm.is_finite() && (MIN_WORK_MM..=MAX_WORK_MM).contains(&mm)
}

/// "<base> copy", then "<base> copy 2", "<base> copy 3", ... until `taken` refuses it; the base
/// is cut so the whole name fits 60 characters and the suffix is kept (criterion 3).
#[must_use]
pub fn copy_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    (1_u32..=u32::MAX)
        .map(|n| {
            let suffix = if n == 1 {
                " copy".to_owned()
            } else {
                format!(" copy {n}")
            };
            let room = MAX_NAME_CHARS.saturating_sub(suffix.chars().count());
            let cut: String = base.chars().take(room).collect();
            format!("{}{suffix}", cut.trim_end())
        })
        .find(|name| !taken(name))
        .unwrap_or_default()
}

/// The name Save as profile gives: `name` itself, or its copy name when a profile has it.
#[must_use]
pub fn save_as_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    if taken(name) {
        copy_name(name, taken)
    } else {
        name.to_owned()
    }
}

/// A new profile id: a lowercase UUID v4 from 16 random bytes the caller passes in (core has no
/// random source; ADR 0036 §5).
#[must_use]
pub fn new_profile_id(random: [u8; 16]) -> String {
    let mut bytes = random;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes.iter().fold(String::with_capacity(32), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Whether `id` is a lowercase UUID v4 (the id of a user profile and its file's name).
#[must_use]
pub fn is_profile_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, &b)| match index {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'4',
            19 => matches!(b, b'8' | b'9' | b'a' | b'b'),
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_has_the_defaults_of_criterion_1() {
        let generic = MachineProfile::generic_laser();
        assert!(generic.is_builtin());
        assert_eq!(generic.values.name, "Generic laser");
        assert_eq!(generic.work_area, DocumentSize::from_mm(400.0, 300.0));
        assert_eq!(generic.values.origin, OriginCorner::BottomLeft);
        assert_eq!(generic.values.max_feed.get(), 6_000);
        assert_eq!(generic.values.max_power.get(), 1_000);
        assert!(generic.values.laser_mode);
        assert_eq!(generic.connection.baud_rate.get(), 115_200);
        assert_eq!(generic.connection.connector_address, "");
    }

    #[test]
    fn integers_are_rounded_and_limited_with_a_short_notice() {
        let feed = clamp_feed(200_000.0).unwrap();
        assert_eq!(feed.value.get(), 100_000);
        assert_eq!(
            feed.notice.as_deref(),
            Some("Max feed limited to 100,000 mm/min.")
        );
        let feed = clamp_feed(1_250.4).unwrap();
        assert_eq!(feed.value.get(), 1_250);
        assert_eq!(feed.notice.as_deref(), Some("Max feed rounded to 1,250."));
        assert_eq!(clamp_feed(3_000.0).unwrap().notice, None);
        assert!(clamp_feed(f64::NAN).is_none());
        let power = clamp_power(0.0).unwrap();
        assert_eq!(power.value.get(), 1);
        assert_eq!(power.notice.as_deref(), Some("Max power limited to 1."));
        assert_eq!(
            clamp_power(70_000.0).unwrap().notice.as_deref(),
            Some("Max power limited to 65,535.")
        );
        assert_eq!(
            clamp_baud(5e6).unwrap().notice.as_deref(),
            Some("Baud rate limited to 4,000,000.")
        );
        for notice in [
            "Max feed limited to 100,000 mm/min.",
            "Width limited to 10,000 mm.",
            "Address limited to 200 characters.",
        ] {
            assert!(notice.len() <= 70);
        }
    }

    #[test]
    fn work_sides_and_texts_are_limited() {
        let width = clamp_work_side("Width", Length::from_mm(20_000.0)).unwrap();
        assert_eq!(width.value, Length::from_mm(10_000.0));
        assert_eq!(width.notice.as_deref(), Some("Width limited to 10,000 mm."));
        assert_eq!(
            clamp_work_side("Height", Length::from_mm(0.2))
                .unwrap()
                .notice
                .as_deref(),
            Some("Height limited to 1 mm.")
        );
        let long = "x".repeat(61);
        let name = clamp_name(&long).unwrap();
        assert_eq!(name.value.chars().count(), 60);
        assert_eq!(
            name.notice.as_deref(),
            Some("Name limited to 60 characters.")
        );
        assert!(clamp_name("   ").is_none());
        assert_eq!(clamp_name(" K40 ").unwrap().value, "K40");
        assert_eq!(
            clamp_port(&"p".repeat(201)).notice.as_deref(),
            Some("Port limited to 200 characters.")
        );
    }

    #[test]
    fn copy_names_count_up_and_keep_the_suffix_within_60_characters() {
        let taken = ["Generic laser", "Generic laser copy"];
        let is_taken = |name: &str| taken.contains(&name);
        assert_eq!(copy_name("Generic laser", is_taken), "Generic laser copy 2");
        assert_eq!(save_as_name("K40", is_taken), "K40");
        assert_eq!(
            save_as_name("Generic laser", is_taken),
            "Generic laser copy 2"
        );
        let long = "y".repeat(60);
        let copied = copy_name(&long, |_| false);
        assert_eq!(copied.chars().count(), 60);
        assert!(copied.ends_with(" copy"));
    }

    #[test]
    fn a_new_id_is_a_lowercase_uuid_v4() {
        let id = new_profile_id([0xff; 16]);
        assert_eq!(id, "ffffffff-ffff-4fff-bfff-ffffffffffff");
        assert!(is_profile_id(&id));
        assert!(is_profile_id(&new_profile_id([0; 16])));
        assert!(!is_profile_id("generic-laser"));
        assert!(!is_profile_id("FFFFFFFF-FFFF-4FFF-BFFF-FFFFFFFFFFFF"));
        assert!(!is_profile_id("ffffffff-ffff-1fff-bfff-ffffffffffff"));
    }

    #[test]
    fn corners_have_stored_names() {
        for corner in OriginCorner::ALL {
            assert_eq!(OriginCorner::from_name(corner.name()), Some(corner));
        }
        assert_eq!(OriginCorner::from_name("middle"), None);
        assert!(OriginCorner::BottomRight.is_right() && OriginCorner::BottomRight.is_bottom());
    }
}
