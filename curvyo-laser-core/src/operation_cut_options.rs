//! The cut path settings of a Cut or Cut-out operation (`specs/0139` criteria 1 to 3, adrs L2):
//! their ranges, defaults, typed edits and stored `op_options` keys.

use serde_json::{Map, Value, json};

use crate::operation_kind::OperationKind;
use crate::operation_options::{ENTER_NUMBER, ENTER_WHOLE_NUMBER, OptionsDamage};
use crate::steps::{exact_steps, rounded_steps, typed_steps};

/// How many tabs the operation places on each path, 0 to 20.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TabCount(u8);

impl TabCount {
    /// The largest count.
    pub const MAX: u8 = 20;
    /// The chip text of a typed count out of range (UX section 10).
    pub const RANGE_TEXT: &'static str = "Count is 0 to 20.";

    /// `count`, `None` above 20.
    #[must_use]
    pub fn new(count: u64) -> Option<Self> {
        u8::try_from(count)
            .ok()
            .filter(|c| *c <= Self::MAX)
            .map(Self)
    }

    /// A typed count, a whole number.
    ///
    /// # Errors
    /// The chip text.
    pub fn typed(count: f64) -> Result<Self, &'static str> {
        if !count.is_finite() || count.fract() != 0.0 {
            return Err(ENTER_WHOLE_NUMBER);
        }
        rounded_steps(count, 1.0)
            .and_then(|c| u64::try_from(c).ok())
            .and_then(Self::new)
            .ok_or(Self::RANGE_TEXT)
    }

    /// The count.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A length of a cut path setting in hundredths of a millimetre: a tab width or a lead length
/// (both 0.10 to 20.00 mm, adrs L2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CutLength(u16);

impl CutLength {
    /// The smallest tab width, in hundredths of a millimetre.
    pub const TAB_MIN: u16 = 10;
    /// The largest tab width, in hundredths of a millimetre.
    pub const TAB_MAX: u16 = 2_000;
    /// The default tab width, 0.50 mm (Proposal).
    pub const DEFAULT_TAB: Self = Self(50);
    /// The chip text of a typed tab width out of range (UX section 10).
    pub const TAB_RANGE_TEXT: &'static str = "Width is 0.10 to 20.00 mm.";

    /// The default lead length, 1.00 mm (`specs/0139` criterion 13, Proposal).
    pub const DEFAULT_LEAD: Self = Self(100);
    /// The chip text of a typed lead-in length out of range (UX section 10).
    pub const LEAD_IN_RANGE_TEXT: &'static str = "Lead-in is 0.10 to 20.00 mm.";
    /// The chip text of a typed lead-out length out of range (UX section 10).
    pub const LEAD_OUT_RANGE_TEXT: &'static str = "Lead-out is 0.10 to 20.00 mm.";

    /// A tab width of `hundredths` hundredths of a millimetre, `None` outside 0.10 to 20.00 mm.
    #[must_use]
    pub fn tab(hundredths: i64) -> Option<Self> {
        u16::try_from(hundredths)
            .ok()
            .filter(|h| (Self::TAB_MIN..=Self::TAB_MAX).contains(h))
            .map(Self)
    }

    /// The exact stored tab width `mm`, `None` with more than two decimals or out of range.
    #[must_use]
    pub fn tab_of_mm(mm: f64) -> Option<Self> {
        Self::tab(exact_steps(mm, 100.0)?)
    }

    /// A typed tab width, rounded to 0.01 mm.
    ///
    /// # Errors
    /// The chip text.
    pub fn typed_tab(mm: f64) -> Result<Self, &'static str> {
        Self::typed(mm, Self::TAB_RANGE_TEXT)
    }

    /// A typed lead length (same range as a tab width), rounded to 0.01 mm; `range` is the chip
    /// text of a value out of range ([`Self::LEAD_IN_RANGE_TEXT`] or
    /// [`Self::LEAD_OUT_RANGE_TEXT`]).
    ///
    /// # Errors
    /// The chip text.
    pub fn typed(mm: f64, range: &'static str) -> Result<Self, &'static str> {
        if !mm.is_finite() {
            return Err(ENTER_NUMBER);
        }
        typed_steps(
            mm,
            100.0,
            f64::from(Self::TAB_MIN),
            f64::from(Self::TAB_MAX),
        )
        .and_then(Self::tab)
        .ok_or(range)
    }

    /// Hundredths of a millimetre.
    #[must_use]
    pub const fn hundredths(self) -> u16 {
        self.0
    }

    /// The length in whole micrometres.
    #[must_use]
    pub fn um(self) -> i64 {
        i64::from(self.0) * 10
    }

    /// The length in mm.
    #[must_use]
    pub fn mm(self) -> f64 {
        f64::from(self.0) / 100.0
    }
}

/// The cut path settings of one Cut or Cut-out operation; every other kind keeps the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CutSettings {
    /// Tabs per path.
    pub tab_count: TabCount,
    /// The width of each tab.
    pub tab_width: CutLength,
    /// Whether a path inside another closed path of the operation gets no automatic tabs.
    pub tab_skip_inner: bool,
}

impl CutSettings {
    /// Count 0, 0.50 mm, Skip inner shapes off.
    pub const DEFAULT: Self = Self {
        tab_count: TabCount(0),
        tab_width: CutLength::DEFAULT_TAB,
        tab_skip_inner: false,
    };

    /// The settings with `edit` applied.
    #[must_use]
    pub const fn with(self, edit: CutEdit) -> Self {
        let mut next = self;
        match edit {
            CutEdit::TabCount(count) => next.tab_count = count,
            CutEdit::TabWidth(width) => next.tab_width = width,
            CutEdit::SkipInner(on) => next.tab_skip_inner = on,
        }
        next
    }
}

impl Default for CutSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One edit of the Tabs group of an unfolded Cut or Cut-out row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutEdit {
    /// A tab count.
    TabCount(TabCount),
    /// A tab width.
    TabWidth(CutLength),
    /// Skip inner shapes on or off.
    SkipInner(bool),
}

const KEY_TAB_COUNT: &str = "tab_count";
const KEY_TAB_WIDTH: &str = "tab_width_mm";
const KEY_TAB_SKIP_INNER: &str = "tab_skip_inner";
/// The stored keys of the cut path settings; only Cut and Cut-out take them.
pub(crate) const CUT_KEYS: [&str; 3] = [KEY_TAB_COUNT, KEY_TAB_WIDTH, KEY_TAB_SKIP_INNER];

/// Whether operations of `kind` cut paths through, so the cut path settings apply.
#[must_use]
pub const fn cuts_through(kind: OperationKind) -> bool {
    matches!(kind, OperationKind::Cut | OperationKind::CutOut)
}

/// Reads `key` into `settings`; `Ok(false)` for a key that is none of [`CUT_KEYS`].
pub(crate) fn read_cut_key(
    settings: &mut CutSettings,
    key: &str,
    value: &Value,
) -> Result<bool, OptionsDamage> {
    match key {
        KEY_TAB_COUNT => {
            let count = value.as_u64().ok_or(OptionsDamage::WrongType)?;
            settings.tab_count = TabCount::new(count).ok_or(OptionsDamage::BadValue)?;
        }
        KEY_TAB_WIDTH => {
            let mm = value.as_f64().ok_or(OptionsDamage::WrongType)?;
            settings.tab_width = CutLength::tab_of_mm(mm).ok_or(OptionsDamage::BadValue)?;
        }
        KEY_TAB_SKIP_INNER => {
            settings.tab_skip_inner = value.as_bool().ok_or(OptionsDamage::WrongType)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Writes every cut path key of `settings` into `map`.
pub(crate) fn write_cut_keys(settings: CutSettings, map: &mut Map<String, Value>) {
    map.insert(KEY_TAB_COUNT.to_owned(), json!(settings.tab_count.get()));
    map.insert(KEY_TAB_WIDTH.to_owned(), json!(settings.tab_width.mm()));
    map.insert(
        KEY_TAB_SKIP_INNER.to_owned(),
        json!(settings.tab_skip_inner),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation_options::{OptionEdit, read_options};

    #[test]
    fn counts_and_widths_are_refused_outside_their_ranges_with_the_chip_texts() {
        assert_eq!(TabCount::typed(20.0).map(TabCount::get), Ok(20));
        assert_eq!(TabCount::typed(21.0), Err("Count is 0 to 20."));
        assert_eq!(TabCount::typed(-1.0), Err("Count is 0 to 20."));
        assert_eq!(TabCount::typed(2.5), Err("Enter a whole number."));
        assert_eq!(
            CutLength::typed_tab(0.09),
            Err("Width is 0.10 to 20.00 mm.")
        );
        assert_eq!(
            CutLength::typed_tab(20.01),
            Err("Width is 0.10 to 20.00 mm.")
        );
        assert_eq!(CutLength::typed_tab(f64::NAN), Err("Enter a number."));
        assert_eq!(
            CutLength::typed_tab(2.004).map(CutLength::hundredths),
            Ok(200)
        );
        assert_eq!(CutLength::typed_tab(0.1).map(CutLength::um), Ok(100));
    }

    #[test]
    fn the_keys_read_back_only_on_cut_and_cut_out() {
        let mut options = crate::operation_options::OperationOptions::default();
        assert_eq!(options.cut, CutSettings::DEFAULT);
        options = options
            .with(
                OperationKind::Cut,
                OptionEdit::Cut(CutEdit::TabCount(TabCount::new(4).unwrap())),
            )
            .unwrap();
        options = options
            .with(
                OperationKind::Cut,
                OptionEdit::Cut(CutEdit::TabWidth(CutLength::tab(200).unwrap())),
            )
            .unwrap();
        for kind in [OperationKind::Cut, OperationKind::CutOut] {
            let value = options.to_value(kind);
            assert_eq!(value["tab_count"], json!(4));
            assert_eq!(value["tab_width_mm"], json!(2.0));
            assert_eq!(value["tab_skip_inner"], json!(false));
            assert_eq!(read_options(Some(&value), kind), Ok(options));
        }
        for kind in [
            OperationKind::Score,
            OperationKind::Engrave,
            OperationKind::EngraveOutline,
            OperationKind::Image,
        ] {
            assert!(options.to_value(kind).get("tab_count").is_none());
            assert_eq!(
                read_options(Some(&json!({ "tab_count": 1 })), kind),
                Err(OptionsDamage::KeyOnKind)
            );
            assert_eq!(
                options.with(kind, OptionEdit::Cut(CutEdit::SkipInner(true))),
                None
            );
        }
    }

    #[test]
    fn a_stored_value_out_of_range_or_of_the_wrong_type_is_damage() {
        let read = |v: Value| read_options(Some(&v), OperationKind::Cut);
        assert_eq!(
            read(json!({ "tab_count": 21 })),
            Err(OptionsDamage::BadValue)
        );
        assert_eq!(
            read(json!({ "tab_count": 1.5 })),
            Err(OptionsDamage::WrongType)
        );
        assert_eq!(
            read(json!({ "tab_width_mm": 0.05 })),
            Err(OptionsDamage::BadValue)
        );
        assert_eq!(
            read(json!({ "tab_width_mm": 0.555 })),
            Err(OptionsDamage::BadValue)
        );
        assert_eq!(
            read(json!({ "tab_skip_inner": 1 })),
            Err(OptionsDamage::WrongType)
        );
        assert_eq!(
            read(json!({ "tab_lead": "line" })),
            Err(OptionsDamage::UnknownKey),
            "a key this build does not know"
        );
        let absent = read_options(None, OperationKind::Cut).unwrap();
        assert_eq!(absent.cut, CutSettings::DEFAULT);
    }
}
