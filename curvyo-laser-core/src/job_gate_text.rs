//! The words of the gate (ADR 0064 §8, amended 2026-11-17; `ux-notes.md` section 10): a check's
//! three parts, the numbers line and the machine line. No other module builds a sentence of it.

use std::collections::HashMap;

use curvyo_document_core::NodeId;

use crate::job_gate::{Issue, MachineFacts, Targets};
use crate::job_gate_cause::Cause;
use crate::job_gate_rows::NumbersLine;
use crate::job_input::mm_of_um;
use crate::job_write;
use crate::machine_profile::grouped;
use crate::material::Interval;
use crate::operation_settings::Field;
use crate::output_settings::Start;

/// The operation names the checks name, as the session passes them (the Operations tab's).
#[derive(Debug, Clone, Default)]
pub struct Names(HashMap<NodeId, String>);

impl Names {
    /// The names of `operations`.
    pub fn of(operations: impl IntoIterator<Item = (NodeId, String)>) -> Self {
        Self(operations.into_iter().collect())
    }

    /// The name of operation `op`, "An operation" when it is not known here.
    #[must_use]
    pub fn operation(&self, op: NodeId) -> &str {
        self.0.get(&op).map_or("An operation", String::as_str)
    }
}

/// A check's text in three parts; `button` is the name or count the line's button shows, placed
/// by the cause, empty when the line has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckText {
    /// The words before the button.
    pub before: String,
    /// The button's words.
    pub button: String,
    /// The words after the button.
    pub after: String,
}

impl CheckText {
    /// The whole sentence (Export's tooltip, the screen reader's line).
    #[must_use]
    pub fn whole(&self) -> String {
        format!("{}{}{}", self.before, self.button, self.after)
    }

    fn plain(text: &str) -> Self {
        Self {
            before: text.to_owned(),
            button: String::new(),
            after: String::new(),
        }
    }

    fn led(button: impl Into<String>, after: impl Into<String>) -> Self {
        Self {
            before: String::new(),
            button: button.into(),
            after: after.into(),
        }
    }
}

/// "1 item", "3 items".
fn items(n: usize) -> String {
    if n == 1 {
        "1 item".to_owned()
    } else {
        format!("{n} items")
    }
}

/// The singular or plural verb of a count.
const fn verb(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

/// The check text of `issue` with the operation names `names` (`ux-notes.md` section 10).
#[must_use]
pub fn check_text(issue: &Issue, names: &Names) -> CheckText {
    let count = issue.count;
    let op = |id: &NodeId| names.operation(*id).to_owned();
    let text = match &issue.cause {
        Cause::NoSettings { op: id } => CheckText::led(op(id), " has no power and speed."),
        Cause::FixtureNoMaterial => {
            CheckText::led("Fixture jobs", " have no material. Pick one above.")
        }
        Cause::FixtureNoSet { material, kind } => CheckText::led(
            "Fixture jobs",
            format!(": {material} has no {} set.", kind.label()),
        ),
        Cause::OutsideBed {
            width_um,
            height_um,
        } => CheckText::led(
            items(count),
            format!(
                " {} outside the {} × {} mm bed.",
                verb(count, "lies", "lie"),
                mm_text(mm_of_um(*width_um)),
                mm_text(mm_of_um(*height_um))
            ),
        ),
        Cause::MachineDamaged => CheckText::plain("The machine profile of this page is damaged."),
        Cause::StartDamaged => CheckText::led("Start", " is damaged. Pick one above."),
        Cause::NothingRuns => CheckText::plain("Nothing runs. Assign items to an operation."),
        Cause::GenericLaser => {
            CheckText::plain("The machine is Generic laser; its limits may not match yours.")
        }
        Cause::MaterialNotFound { op: id, material } => CheckText::led(
            op(id),
            format!(": {material} is not found, using stored numbers."),
        ),
        Cause::NoSetForLaser {
            op: id,
            material,
            kind,
        } => CheckText::led(
            op(id),
            format!(
                ": {material} has no {} set, using stored numbers.",
                kind.label()
            ),
        ),
        Cause::TypedOverride {
            op: id,
            material,
            fields,
        } => CheckText::led(
            op(id),
            format!(" runs a typed {}, not {material}’s.", fields_text(fields)),
        ),
        Cause::OverFeed { op: id, feed } => CheckText::led(
            op(id),
            format!(
                " runs at {} mm/min, the machine's maximum.",
                grouped(u64::from(*feed))
            ),
        ),
        Cause::ZWithoutAxis { op: id } => CheckText::led(
            op(id),
            " has a Z offset; the machine has no motorised Z axis.",
        ),
        Cause::NotAssigned => CheckText::led(
            items(count),
            format!(" {} not assigned.", verb(count, "is", "are")),
        ),
        Cause::HiddenRuns => CheckText::led(
            items(count),
            format!(" {} while hidden.", verb(count, "runs", "run")),
        ),
        Cause::OutputOff { op: Some(id) } if count == 1 => {
            CheckText::led(op(id), " has Output off.")
        }
        Cause::OutputOff { .. } => {
            CheckText::led(format!("{count} operations"), " have Output off.")
        }
        Cause::CutOutNoContour { op: id } => CheckText {
            before: format!("{}: ", op(id)),
            button: items(count),
            after: format!(" {} no closed contour.", verb(count, "has", "have")),
        },
        Cause::DamagedSettings => CheckText::plain(&if count == 1 {
            "1 operation has damaged settings and does not run.".to_owned()
        } else {
            format!("{count} operations have damaged settings and do not run.")
        }),
    };
    if issue.targets == Targets::None {
        return CheckText::plain(&text.whole());
    }
    text
}

/// The machine line of the output group (P30): "Diode 10 W · GRBL", " · air assist" when on;
/// "Machine profile damaged" for a damaged copy.
#[must_use]
pub fn machine_line(machine: &MachineFacts) -> String {
    if machine.damaged {
        return "Machine profile damaged".to_owned();
    }
    let mut line = format!("{} · {}", machine.name, job_write::name(machine.format));
    if machine.air_assist {
        line.push_str(" · air assist");
    }
    line
}

/// A numbers line: "35 %, 6,000 mm/min, 1 ×, 0.25 mm", "typed" after each typed number.
#[must_use]
pub fn numbers_text(line: &NumbersLine) -> String {
    let mark = |field| {
        if line.typed.contains(&field) {
            " typed"
        } else {
            ""
        }
    };
    let mut text = format!(
        "{} %{}, {} mm/min{}, {} ×{}",
        line.numbers.power,
        mark(Field::Power),
        grouped(u64::from(line.numbers.speed.get())),
        mark(Field::Speed),
        line.numbers.passes.get(),
        mark(Field::Passes)
    );
    if let Some(interval) = line.interval {
        text.push_str(", ");
        text.push_str(&interval_text(interval));
    }
    text
}

/// The Start pick's words of `start` (`ux-notes.md` section 10).
#[must_use]
pub const fn start_text(start: Start) -> &'static str {
    match start {
        Start::MachineOrigin => "Machine origin",
        Start::HeadPosition => "Head position",
        Start::HomeThenOrigin => "Home, then machine origin",
    }
}

/// "power", "power and speed", "power, speed and passes".
fn fields_text(fields: &[Field]) -> String {
    let words: Vec<&str> = fields
        .iter()
        .map(|field| match field {
            Field::Power => "power",
            Field::Speed => "speed",
            Field::Passes => "passes",
        })
        .collect();
    match words.as_slice() {
        [] => "number".to_owned(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A length in mm with at most two decimals and no trailing zeros ("400", "297.5").
#[must_use]
pub fn mm_text(mm: f64) -> String {
    let text = format!("{mm:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    text.to_owned()
}

/// The interval as a numbers line writes it ("0.25 mm", "0.1 mm").
#[must_use]
pub fn interval_text(interval: Interval) -> String {
    format!("{} mm", mm_text(interval.mm()))
}

#[cfg(test)]
#[path = "job_gate_text_tests.rs"]
mod tests;
