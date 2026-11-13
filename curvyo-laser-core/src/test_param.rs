//! One parameter a test grid varies or shares, with its steps, ranges and texts (ADR 0057 §1,
//! `specs/0112` criteria 3 to 6, UX notes 4.3 and 10).

use std::fmt;

use crate::machine_profile::FeedRate;
use crate::material::{Interval, Passes, PowerPercent, SetKind, SetNumbers, Settings};

/// A parameter of a test grid: on one axis or shared by every cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Param {
    /// Power in percent, kept in tenths.
    Power,
    /// Speed in whole mm/min.
    Speed,
    /// Passes.
    Passes,
    /// The hatch interval in mm, kept in hundredths; Engrave only.
    Interval,
}

impl Param {
    /// Every parameter in the order of the bar (UX 4.1).
    pub const ALL: [Self; 4] = [Self::Power, Self::Speed, Self::Passes, Self::Interval];

    /// The stored text of an axis `param`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Speed => "speed",
            Self::Passes => "passes",
            Self::Interval => "interval",
        }
    }

    /// The parameter an axis text names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == text)
    }

    /// The key of the parameter in `fixed`.
    #[must_use]
    pub const fn fixed_key(self) -> &'static str {
        match self {
            Self::Power => "power_pct",
            Self::Speed => "speed_mm_min",
            Self::Passes => "passes",
            Self::Interval => "interval_mm",
        }
    }

    /// Whether a grid of `kind` offers the parameter (interval only for Engrave, criterion 4).
    #[must_use]
    pub fn offered(self, kind: SetKind) -> bool {
        self != Self::Interval || kind == SetKind::Engrave
    }

    /// The parameters a grid of `kind` offers, in bar order.
    pub fn of_kind(kind: SetKind) -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(move |p| p.offered(kind))
    }

    /// The axis title burned beside the values ("Power %", "mm/min", "Passes", "Interval mm").
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Power => "Power %",
            Self::Speed => "mm/min",
            Self::Passes => "Passes",
            Self::Interval => "Interval mm",
        }
    }

    /// Steps per unit: tenths of a percent, whole mm/min, whole passes, hundredths of a mm.
    pub(crate) const fn per_unit(self) -> f64 {
        match self {
            Self::Power => 10.0,
            Self::Speed | Self::Passes => 1.0,
            Self::Interval => 100.0,
        }
    }

    /// Whether a typed value must be whole.
    pub(crate) const fn whole(self) -> bool {
        matches!(self, Self::Speed | Self::Passes)
    }

    /// The steps a stored value may take; a speed is not checked against a machine.
    pub(crate) fn stored_steps(self) -> (u32, u32) {
        self.steps_up_to(FeedRate::new(u64::from(FeedRate::MAX)).unwrap_or_default())
    }

    /// The steps a typed value may take on a machine of maximum feed `max_feed`.
    pub(crate) fn steps_up_to(self, max_feed: FeedRate) -> (u32, u32) {
        match self {
            Self::Power => (
                u32::from(PowerPercent::MIN_TENTHS),
                u32::from(PowerPercent::MAX_TENTHS),
            ),
            Self::Speed => (1, max_feed.get()),
            Self::Passes => (u32::from(Passes::MIN), u32::from(Passes::MAX)),
            Self::Interval => (
                u32::from(Interval::MIN_HUNDREDTHS),
                u32::from(Interval::MAX_HUNDREDTHS),
            ),
        }
    }

    /// The validation text of a value out of range (UX section 10).
    #[must_use]
    pub fn range_text(self, max_feed: FeedRate) -> String {
        match self {
            Self::Power => "Power is 1 to 100 %.".to_owned(),
            Self::Speed => format!("Speed is 1 to {} mm/min.", max_feed.get()),
            Self::Passes => "Passes are 1 to 20.".to_owned(),
            Self::Interval => Interval::RANGE_TEXT.to_owned(),
        }
    }

    /// The default Min and Max of an axis (criterion 6): 20 to 100 %, 10 % to 100 % of the
    /// maximum feed, 1 to 5 passes, 0.05 to 0.25 mm.
    pub(crate) fn axis_default(self, max_feed: FeedRate) -> (u32, u32) {
        match self {
            Self::Power => (200, 1000),
            Self::Speed => (((max_feed.get() + 5) / 10).max(1), max_feed.get()),
            Self::Passes => (1, 5),
            Self::Interval => (5, 25),
        }
    }

    /// The default shared value (G7): the material set's when given, else 50 %, 3000 mm/min (at
    /// most the maximum feed), 1 pass, 0.10 mm.
    pub(crate) fn fixed_default(self, max_feed: FeedRate, set: Option<&SetNumbers>) -> u32 {
        let from_set = set.and_then(|set| match self {
            Self::Power => Some(u32::from(set.settings.power.tenths())),
            Self::Speed => Some(set.settings.speed.get()),
            Self::Passes => Some(u32::from(set.settings.passes.get())),
            Self::Interval => set.interval.map(|i| u32::from(i.hundredths())),
        });
        from_set.unwrap_or(match self {
            Self::Power => 500,
            Self::Speed => max_feed.get().min(3000),
            Self::Passes => 1,
            Self::Interval => u32::from(Interval::DEFAULT.hundredths()),
        })
    }
}

/// One value of a parameter, in its steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamValue {
    /// The parameter.
    pub param: Param,
    steps: u32,
}

impl ParamValue {
    /// `steps` of `param`; the caller keeps them inside the parameter's stored range.
    pub(crate) const fn new(param: Param, steps: u32) -> Self {
        Self { param, steps }
    }

    /// The value in steps of its parameter.
    #[must_use]
    pub const fn steps(self) -> u32 {
        self.steps
    }

    /// The value in its unit (`60.5`, `1950`, `1`, `0.08`).
    #[must_use]
    pub fn number(self) -> f64 {
        f64::from(self.steps) / self.param.per_unit()
    }
}

impl fmt::Display for ParamValue {
    /// "60" or "60.5", "1950", "1", "0.08": the burned value and the name of a cell row.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.param {
            Param::Power => write!(f, "{}", power(self.steps)),
            Param::Speed | Param::Passes => write!(f, "{}", self.steps),
            Param::Interval => write!(f, "{}.{:02}", self.steps / 100, self.steps % 100),
        }
    }
}

fn power(tenths: u32) -> PowerPercent {
    // invariant: power steps are kept within 10 to 1000 tenths.
    #[allow(clippy::expect_used)]
    PowerPercent::from_tenths(u16::try_from(tenths).expect("power steps fit u16"))
        .expect("power steps in range")
}

/// The numbers of a cell from the value of each parameter of a grid of `kind`.
///
/// # Panics
/// Does not panic: every value is kept within its parameter's range.
pub(crate) fn numbers_of(kind: SetKind, value: impl Fn(Param) -> u32) -> SetNumbers {
    // invariant: steps of each parameter lie within its stored range.
    #[allow(clippy::expect_used)]
    SetNumbers {
        settings: Settings {
            power: power(value(Param::Power)),
            speed: FeedRate::new(u64::from(value(Param::Speed))).expect("speed in range"),
            passes: Passes::new(u64::from(value(Param::Passes))).expect("passes in range"),
        },
        interval: (kind == SetKind::Engrave).then(|| {
            Interval::from_hundredths(i64::from(value(Param::Interval))).expect("interval in range")
        }),
    }
}
