//! The text-free run facts of each running block (criterion 2; `ux-notes.md` 6.1, 6.2): its
//! ring, its time, its numbers lines, as the rows' line 2, line 3 or a fixture run line show them.

use curvyo_document_core::NodeId;

use crate::job_input::{BlockInput, Ring};
use crate::job_numbers::{BlockNumbers, NumbersRead, NumbersSource, RunNumbers};
use crate::material::Interval;
use crate::operation_image_options::Dither;
use crate::operation_kind::OperationKind;
use crate::operation_settings::{Field, Role};

/// The numbers of one set, with the typed fields (a row's line 3, or line 4 of Engrave +
/// Outline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumbersLine {
    /// The numbers as set (the speed as typed, criterion 3).
    pub numbers: RunNumbers,
    /// The interval of the main set of a kind with an interval (Engrave kinds, Image).
    pub interval: Option<Interval>,
    /// The typed fields.
    pub typed: Vec<Field>,
    /// An Image operation's dither mode and Lowest power in tenths of a percent, after the
    /// numbers (`specs/0123` criterion 47 d).
    pub dither: Option<(Dither, u16)>,
}

/// One run of the job: an operation that runs, or the fixture jobs of one kind.
#[derive(Debug, Clone, PartialEq)]
pub struct RunFacts {
    /// The operation, or the first fixture job.
    pub block: NodeId,
    /// Its ring.
    pub ring: Ring,
    /// Its kind.
    pub kind: Option<OperationKind>,
    /// Its time in seconds, `None` with No settings ("–").
    pub time_s: Option<f64>,
    /// Its numbers lines: the main set, then the outline set; empty with No settings.
    pub numbers: Vec<NumbersLine>,
    /// Whether it has no numbers (No settings, or a fixture run without a material or set).
    pub no_settings: bool,
}

/// The numbers lines of a block's read numbers: the main set (with the interval for Engrave
/// kinds), then the outline set, the typed fields of each; empty when it has no numbers. The
/// operations view calls this for every row, running or not (criterion 3).
#[must_use]
pub fn numbers_lines(read: &NumbersRead) -> Vec<NumbersLine> {
    let NumbersRead::Runs { numbers, source } = read else {
        return Vec::new();
    };
    let typed: &[(Role, Field)] = match source {
        NumbersSource::Live { typed, .. } | NumbersSource::Stored { typed, .. } => typed,
        NumbersSource::Own | NumbersSource::Fixture { .. } => &[],
    };
    lines_of(numbers, typed)
}

fn lines_of(numbers: &BlockNumbers, typed: &[(Role, Field)]) -> Vec<NumbersLine> {
    let line = |role: Role, set: RunNumbers, interval: Option<Interval>| NumbersLine {
        numbers: set,
        interval,
        typed: typed
            .iter()
            .filter(|(r, _)| *r == role)
            .map(|(_, f)| *f)
            .collect(),
        dither: None,
    };
    let mut main = line(
        Role::Main,
        numbers.main,
        numbers.kind.has_interval().then_some(numbers.interval),
    );
    if numbers.kind == OperationKind::Image {
        let image = &numbers.options.image;
        main.dither = Some((image.dither, image.lowest_power_tenths));
    }
    let mut lines = vec![main];
    if let Some(outline) = numbers.outline {
        lines.push(line(Role::Outline, outline, None));
    }
    lines
}

/// The facts of the running `block` with its read numbers and time.
pub(crate) fn run_facts(block: &BlockInput, read: &NumbersRead, time_s: Option<f64>) -> RunFacts {
    let numbers = numbers_lines(read);
    RunFacts {
        block: block.id,
        ring: block.ring,
        kind: block.kind.known(),
        time_s: if numbers.is_empty() { None } else { time_s },
        no_settings: numbers.is_empty(),
        numbers,
    }
}
