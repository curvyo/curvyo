//! The path options of a Cut or Cut-out operation and of an item's own block (`specs/0139`
//! criteria 13, 21, 22, adrs L2, L3): lead-in, lead-out and overcut with their ranges, defaults,
//! typed edits, stored `op_options` keys and the `path_own.<key>` register.

use serde_json::{Map, Value, json};

use crate::cut_item_records::RecordDamage;
use crate::operation_cut_options::CutLength;
use crate::operation_options::{ENTER_NUMBER, OptionsDamage};
use crate::steps::{exact_steps, typed_steps};

/// The shape of a lead-in or lead-out (criterion 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LeadKind {
    /// No lead.
    #[default]
    None,
    /// A straight segment perpendicular to the path, on the waste side.
    Line,
    /// A quarter circle tangent to the path, bending to the waste side.
    Arc,
}

impl LeadKind {
    /// The stored text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Line => "line",
            Self::Arc => "arc",
        }
    }

    /// The kind of a stored text.
    #[must_use]
    pub fn of_str(text: &str) -> Option<Self> {
        Some(match text {
            "none" => Self::None,
            "line" => Self::Line,
            "arc" => Self::Arc,
            _ => return None,
        })
    }

    /// The kind of a pill segment index (0 None, 1 Line, 2 Arc).
    #[must_use]
    pub const fn of_index(index: u64) -> Option<Self> {
        match index {
            0 => Some(Self::None),
            1 => Some(Self::Line),
            2 => Some(Self::Arc),
            _ => None,
        }
    }

    /// The name the item part's summary line uses.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Line => "Line",
            Self::Arc => "Arc",
        }
    }
}

/// How far the beam runs on past the start of a closed path, 0.00 to 10.00 mm in hundredths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Overcut(u16);

impl Overcut {
    /// The largest overcut in hundredths of a millimetre.
    pub const MAX: u16 = 1_000;
    /// The chip text of a typed overcut out of range (UX section 10).
    pub const RANGE_TEXT: &'static str = "Overcut is 0.00 to 10.00 mm.";

    /// `hundredths`, `None` above 10.00 mm.
    #[must_use]
    pub fn new(hundredths: i64) -> Option<Self> {
        u16::try_from(hundredths)
            .ok()
            .filter(|h| *h <= Self::MAX)
            .map(Self)
    }

    /// The exact stored `mm`, `None` with more than two decimals or out of range.
    #[must_use]
    pub fn of_mm(mm: f64) -> Option<Self> {
        Self::new(exact_steps(mm, 100.0)?)
    }

    /// A typed overcut, rounded to 0.01 mm.
    ///
    /// # Errors
    /// The chip text.
    pub fn typed(mm: f64) -> Result<Self, &'static str> {
        if !mm.is_finite() {
            return Err(ENTER_NUMBER);
        }
        typed_steps(mm, 100.0, 0.0, f64::from(Self::MAX))
            .and_then(Self::new)
            .ok_or(Self::RANGE_TEXT)
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

/// Lead-in, lead-out and overcut of an operation or of one item's own block (criteria 13, 21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathOptions {
    /// The lead-in's shape.
    pub lead_in: LeadKind,
    /// The lead-in's length (a Line's length, an Arc's radius).
    pub lead_in_length: CutLength,
    /// The lead-out's shape.
    pub lead_out: LeadKind,
    /// The lead-out's length.
    pub lead_out_length: CutLength,
    /// The overcut.
    pub overcut: Overcut,
}

impl PathOptions {
    /// None, 1 mm, None, 1 mm, 0 mm.
    pub const DEFAULT: Self = Self {
        lead_in: LeadKind::None,
        lead_in_length: CutLength::DEFAULT_LEAD,
        lead_out: LeadKind::None,
        lead_out_length: CutLength::DEFAULT_LEAD,
        overcut: Overcut(0),
    };

    /// Whether they add nothing to a path (no lead, no overcut): the path runs as today.
    #[must_use]
    pub fn is_inert(self) -> bool {
        self.lead_in == LeadKind::None && self.lead_out == LeadKind::None && self.overcut.0 == 0
    }

    /// The options with `edit` applied.
    #[must_use]
    pub const fn with(self, edit: PathEdit) -> Self {
        let mut next = self;
        match edit {
            PathEdit::LeadIn(kind) => next.lead_in = kind,
            PathEdit::LeadInLength(length) => next.lead_in_length = length,
            PathEdit::LeadOut(kind) => next.lead_out = kind,
            PathEdit::LeadOutLength(length) => next.lead_out_length = length,
            PathEdit::Overcut(overcut) => next.overcut = overcut,
        }
        next
    }

    /// The muted summary line of the item part (UX 3.2): "Lead-in Arc 1 mm · Lead-out None ·
    /// Overcut 0.5 mm", each unit tied to its number by a no-break space so a wrap never leaves
    /// "mm" alone.
    #[must_use]
    pub fn summary(self) -> String {
        let lead = |name: &str, kind: LeadKind, length: CutLength| match kind {
            LeadKind::None => format!("{name} None"),
            _ => format!("{name} {} {}\u{a0}mm", kind.label(), short(length.mm())),
        };
        format!(
            "{} · {} · Overcut {}\u{a0}mm",
            lead("Lead-in", self.lead_in, self.lead_in_length),
            lead("Lead-out", self.lead_out, self.lead_out_length),
            short(self.overcut.mm())
        )
    }

    /// The `path_own.<key>` register: all five values.
    #[must_use]
    pub fn to_record(self) -> Value {
        let mut map = Map::new();
        write_path_keys(self, &mut map);
        Value::Object(map)
    }
}

impl Default for PathOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A millimetre value with at most two decimals and no trailing zeros ("1", "0.5", "1.25").
fn short(mm: f64) -> String {
    let text = format!("{mm:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// One edit of the Path options group of an unfolded Cut or Cut-out row or an item's own block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathEdit {
    /// The lead-in's shape.
    LeadIn(LeadKind),
    /// The lead-in's length.
    LeadInLength(CutLength),
    /// The lead-out's shape.
    LeadOut(LeadKind),
    /// The lead-out's length.
    LeadOutLength(CutLength),
    /// The overcut.
    Overcut(Overcut),
}

/// One field of the Path options group, by its host name (`lead_in`, `lead_in_mm`, `lead_out`,
/// `lead_out_mm`, `overcut`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathField {
    /// The lead-in pill: 0 None, 1 Line, 2 Arc.
    LeadIn,
    /// The lead-in length field.
    LeadInLength,
    /// The lead-out pill.
    LeadOut,
    /// The lead-out length field.
    LeadOutLength,
    /// The overcut field.
    Overcut,
}

impl PathField {
    /// The field of its host name.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        Some(match name {
            "lead_in" => Self::LeadIn,
            "lead_in_mm" => Self::LeadInLength,
            "lead_out" => Self::LeadOut,
            "lead_out_mm" => Self::LeadOutLength,
            "overcut" => Self::Overcut,
            _ => return None,
        })
    }

    /// The edit of `value` typed into (or, for a pill, the segment index `value` pressed in)
    /// this field.
    ///
    /// # Errors
    /// The chip text of UX section 10.
    pub fn typed(self, value: f64) -> Result<PathEdit, &'static str> {
        let kind = || {
            if value.fract() != 0.0 || value < 0.0 {
                return Err(ENTER_NUMBER);
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // invariant: a whole number at least 0, checked above
            let index = value as u64;
            LeadKind::of_index(index).ok_or(ENTER_NUMBER)
        };
        match self {
            Self::LeadIn => kind().map(PathEdit::LeadIn),
            Self::LeadOut => kind().map(PathEdit::LeadOut),
            Self::LeadInLength => {
                CutLength::typed(value, CutLength::LEAD_IN_RANGE_TEXT).map(PathEdit::LeadInLength)
            }
            Self::LeadOutLength => {
                CutLength::typed(value, CutLength::LEAD_OUT_RANGE_TEXT).map(PathEdit::LeadOutLength)
            }
            Self::Overcut => Overcut::typed(value).map(PathEdit::Overcut),
        }
    }
}

const KEY_LEAD_IN: &str = "lead_in";
const KEY_LEAD_IN_MM: &str = "lead_in_mm";
const KEY_LEAD_OUT: &str = "lead_out";
const KEY_LEAD_OUT_MM: &str = "lead_out_mm";
const KEY_OVERCUT: &str = "overcut_mm";
/// The stored keys of the path options; only Cut and Cut-out take them.
pub(crate) const PATH_KEYS: [&str; 5] = [
    KEY_LEAD_IN,
    KEY_LEAD_IN_MM,
    KEY_LEAD_OUT,
    KEY_LEAD_OUT_MM,
    KEY_OVERCUT,
];

/// Why one stored path value does not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueDamage {
    WrongType,
    BadValue,
}

/// Reads `key` into `options`; `Ok(false)` for a key that is none of [`PATH_KEYS`].
fn read_key(options: &mut PathOptions, key: &str, value: &Value) -> Result<bool, ValueDamage> {
    let kind = |value: &Value| {
        let text = value.as_str().ok_or(ValueDamage::WrongType)?;
        LeadKind::of_str(text).ok_or(ValueDamage::BadValue)
    };
    let length = |value: &Value| {
        let mm = value.as_f64().ok_or(ValueDamage::WrongType)?;
        CutLength::tab_of_mm(mm).ok_or(ValueDamage::BadValue)
    };
    match key {
        KEY_LEAD_IN => options.lead_in = kind(value)?,
        KEY_LEAD_IN_MM => options.lead_in_length = length(value)?,
        KEY_LEAD_OUT => options.lead_out = kind(value)?,
        KEY_LEAD_OUT_MM => options.lead_out_length = length(value)?,
        KEY_OVERCUT => {
            let mm = value.as_f64().ok_or(ValueDamage::WrongType)?;
            options.overcut = Overcut::of_mm(mm).ok_or(ValueDamage::BadValue)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Reads the `op_options` key `key` into `options`; `Ok(false)` for a key that is none of
/// [`PATH_KEYS`].
pub(crate) fn read_path_key(
    options: &mut PathOptions,
    key: &str,
    value: &Value,
) -> Result<bool, OptionsDamage> {
    read_key(options, key, value).map_err(|damage| match damage {
        ValueDamage::WrongType => OptionsDamage::WrongType,
        ValueDamage::BadValue => OptionsDamage::BadValue,
    })
}

/// Writes every path option key of `options` into `map`.
pub(crate) fn write_path_keys(options: PathOptions, map: &mut Map<String, Value>) {
    map.insert(KEY_LEAD_IN.to_owned(), json!(options.lead_in.as_str()));
    map.insert(
        KEY_LEAD_IN_MM.to_owned(),
        json!(options.lead_in_length.mm()),
    );
    map.insert(KEY_LEAD_OUT.to_owned(), json!(options.lead_out.as_str()));
    map.insert(
        KEY_LEAD_OUT_MM.to_owned(),
        json!(options.lead_out_length.mm()),
    );
    map.insert(KEY_OVERCUT.to_owned(), json!(options.overcut.mm()));
}

/// Reads a stored `path_own.<key>` register: a map with exactly the five values (adrs L3).
///
/// # Errors
/// The damage of a register that does not read: the item runs nothing in that operation.
pub fn read_own_path(value: &Value) -> Result<PathOptions, RecordDamage> {
    let map = value.as_object().ok_or(RecordDamage::NotAMap)?;
    let mut options = PathOptions::DEFAULT;
    for (key, value) in map {
        match read_key(&mut options, key, value) {
            Ok(true) => {}
            Ok(false) => return Err(RecordDamage::UnknownField),
            Err(ValueDamage::WrongType) => return Err(RecordDamage::WrongType),
            Err(ValueDamage::BadValue) => return Err(RecordDamage::BadValue),
        }
    }
    if PATH_KEYS.iter().any(|key| !map.contains_key(*key)) {
        return Err(RecordDamage::WrongType);
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation_kind::OperationKind;
    use crate::operation_options::{OperationOptions, OptionEdit, read_options};

    #[test]
    fn lengths_and_overcut_are_refused_outside_their_ranges_with_the_chip_texts() {
        let lead_in = |mm| CutLength::typed(mm, CutLength::LEAD_IN_RANGE_TEXT);
        assert_eq!(lead_in(0.09), Err("Lead-in is 0.10 to 20.00 mm."));
        assert_eq!(
            CutLength::typed(20.01, CutLength::LEAD_OUT_RANGE_TEXT),
            Err("Lead-out is 0.10 to 20.00 mm.")
        );
        assert_eq!(lead_in(2.0).map(CutLength::um), Ok(2_000));
        assert_eq!(lead_in(f64::NAN), Err("Enter a number."));
        assert_eq!(Overcut::typed(10.01), Err("Overcut is 0.00 to 10.00 mm."));
        assert_eq!(Overcut::typed(-0.01), Err("Overcut is 0.00 to 10.00 mm."));
        assert_eq!(Overcut::typed(0.0).map(Overcut::um), Ok(0));
        assert_eq!(Overcut::typed(5.004).map(Overcut::um), Ok(5_000));
        assert_eq!(LeadKind::of_index(2), Some(LeadKind::Arc));
        assert_eq!(LeadKind::of_index(3), None);
        let field = |name| PathField::of(name).unwrap();
        assert_eq!(
            field("lead_in").typed(1.0),
            Ok(PathEdit::LeadIn(LeadKind::Line))
        );
        assert_eq!(field("lead_out").typed(3.0), Err("Enter a number."));
        assert_eq!(
            field("lead_out_mm").typed(0.0),
            Err("Lead-out is 0.10 to 20.00 mm.")
        );
        assert_eq!(
            field("overcut").typed(11.0),
            Err("Overcut is 0.00 to 10.00 mm.")
        );
        assert_eq!(PathField::of("lead"), None);
    }

    #[test]
    fn the_keys_read_back_only_on_cut_and_cut_out_and_the_defaults_write_nothing() {
        let plain = OperationOptions::default();
        assert!(plain.to_value(OperationKind::Cut).get("lead_in").is_none());
        let options = plain
            .with(
                OperationKind::Cut,
                OptionEdit::Path(PathEdit::LeadIn(LeadKind::Arc)),
            )
            .unwrap()
            .with(
                OperationKind::Cut,
                OptionEdit::Path(PathEdit::Overcut(Overcut::new(50).unwrap())),
            )
            .unwrap();
        for kind in [OperationKind::Cut, OperationKind::CutOut] {
            let value = options.to_value(kind);
            assert_eq!(value["lead_in"], json!("arc"));
            assert_eq!(value["lead_in_mm"], json!(1.0));
            assert_eq!(value["lead_out"], json!("none"));
            assert_eq!(value["overcut_mm"], json!(0.5));
            assert_eq!(read_options(Some(&value), kind), Ok(options));
        }
        for kind in [
            OperationKind::Score,
            OperationKind::Engrave,
            OperationKind::EngraveOutline,
            OperationKind::Image,
        ] {
            assert!(options.to_value(kind).get("lead_in").is_none());
            assert_eq!(
                read_options(Some(&json!({ "overcut_mm": 1.0 })), kind),
                Err(OptionsDamage::KeyOnKind)
            );
            assert_eq!(
                options.with(kind, OptionEdit::Path(PathEdit::LeadOut(LeadKind::Line))),
                None
            );
        }
        let read = |v: Value| read_options(Some(&v), OperationKind::Cut);
        assert_eq!(
            read(json!({ "lead_in": "spiral" })),
            Err(OptionsDamage::BadValue)
        );
        assert_eq!(read(json!({ "lead_in": 1 })), Err(OptionsDamage::WrongType));
        assert_eq!(
            read(json!({ "lead_out_mm": 0.05 })),
            Err(OptionsDamage::BadValue)
        );
        assert_eq!(
            read(json!({ "overcut_mm": 10.5 })),
            Err(OptionsDamage::BadValue)
        );
        assert_eq!(
            read(json!({ "overcut_mm": 0.555 })),
            Err(OptionsDamage::BadValue)
        );
    }

    #[test]
    fn an_own_block_reads_back_and_names_its_damage() {
        let own = PathOptions::DEFAULT
            .with(PathEdit::LeadOut(LeadKind::Line))
            .with(PathEdit::LeadOutLength(CutLength::tab(250).unwrap()));
        assert_eq!(read_own_path(&own.to_record()), Ok(own));
        assert_eq!(read_own_path(&json!([1])), Err(RecordDamage::NotAMap));
        let mut record = own.to_record();
        record["x"] = json!(1);
        assert_eq!(read_own_path(&record), Err(RecordDamage::UnknownField));
        let mut record = own.to_record();
        record["overcut_mm"] = json!(11.0);
        assert_eq!(read_own_path(&record), Err(RecordDamage::BadValue));
        let mut record = own.to_record();
        record.as_object_mut().unwrap().remove("lead_in");
        assert_eq!(read_own_path(&record), Err(RecordDamage::WrongType));
        assert_eq!(
            own.summary(),
            "Lead-in None · Lead-out Line 2.5\u{a0}mm · Overcut 0\u{a0}mm"
        );
        assert!(PathOptions::DEFAULT.is_inert() && !own.is_inert());
    }
}
