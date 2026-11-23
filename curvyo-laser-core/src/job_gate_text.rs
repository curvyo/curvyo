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
use crate::material::{Interval, PowerPercent};
use crate::operation_image_options::Dither;
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

    /// The name of item `item` (the session passes the names of the raster items it checks
    /// beside the operations'), "An item" when it is not known here.
    #[must_use]
    pub fn item(&self, item: NodeId) -> &str {
        self.0.get(&item).map_or("An item", String::as_str)
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

/// "1 image", "3 images".
fn images(n: usize) -> String {
    if n == 1 {
        "1 image".to_owned()
    } else {
        format!("{n} images")
    }
}

/// The singular or plural verb of a count.
const fn verb(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

/// The check text of a raster cause (`specs/0123` criteria 31 to 36, UX section 4).
fn raster_text(cause: &Cause, count: usize, names: &Names) -> CheckText {
    let op = |id: &NodeId| names.operation(*id).to_owned();
    match cause {
        Cause::GreyNeedsM4 { op: id } => CheckText::led(op(id), " needs laser mode M4 for Grey."),
        Cause::TooManyDots { item, count: dots } => CheckText::led(
            names.item(*item),
            format!(
                " would need {} dots; the limit is {}.",
                grouped(*dots),
                grouped(crate::job_raster_plan::MAX_DOTS)
            ),
        ),
        Cause::CoarseImages => CheckText::led(
            images(count),
            verb(
                count,
                " is coarser than its Interval.",
                " are coarser than their Interval.",
            ),
        ),
        Cause::OverlappingRasterItems => CheckText::led(items(count), " overlap and burn twice."),
        Cause::ReducedImages => CheckText::led(
            images(count),
            verb(
                count,
                " was reduced and loses detail.",
                " were reduced and lose detail.",
            ),
        ),
        Cause::UnreadableImages => CheckText::led(
            images(count),
            verb(
                count,
                " cannot be read and is skipped.",
                " cannot be read and are skipped.",
            ),
        ),
        Cause::NotPictures { op: id } => CheckText::led(
            items(count),
            format!(
                " in {}{}",
                op(id),
                verb(
                    count,
                    " is not a picture and does not burn.",
                    " are not pictures and do not burn.",
                )
            ),
        ),
        Cause::LowestPowerAbovePower { op: id } => {
            CheckText::led(op(id), ": Lowest power is above Power.")
        }
        _ => CheckText::plain(""),
    }
}

/// The check text of an operation's numbers: material, set, typed values, feed, Z offset.
fn operation_numbers_text(cause: &Cause, names: &Names) -> CheckText {
    let op = |id: &NodeId| names.operation(*id).to_owned();
    match cause {
        Cause::NoSettings { op: id } => CheckText::led(op(id), " has no power and speed."),
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
        _ => CheckText::plain(""),
    }
}

/// The check text of a cause that counts items or operations: outside the bed, not assigned,
/// hidden, Output off, no contour.
fn count_text(cause: &Cause, count: usize, names: &Names) -> CheckText {
    let op = |id: &NodeId| names.operation(*id).to_owned();
    match cause {
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
        _ => CheckText::plain(""),
    }
}

/// The check text of `issue` with the operation names `names` (`ux-notes.md` section 10).
#[must_use]
pub fn check_text(issue: &Issue, names: &Names) -> CheckText {
    let count = issue.count;
    let cause = &issue.cause;
    let text = match cause {
        Cause::FixtureNoMaterial => {
            CheckText::led("Fixture jobs", " have no material. Pick one above.")
        }
        Cause::FixtureNoSet { material, kind } => CheckText::led(
            "Fixture jobs",
            format!(": {material} has no {} set.", kind.label()),
        ),
        Cause::MachineDamaged => CheckText::plain("The machine profile of this page is damaged."),
        Cause::StartDamaged => CheckText::led("Start", " is damaged. Pick one above."),
        Cause::NothingRuns => CheckText::plain("Nothing runs. Assign items to an operation."),
        Cause::GenericLaser => {
            CheckText::plain("The machine is Generic laser; its limits may not match yours.")
        }
        Cause::NoSettings { .. }
        | Cause::MaterialNotFound { .. }
        | Cause::NoSetForLaser { .. }
        | Cause::TypedOverride { .. }
        | Cause::OverFeed { .. }
        | Cause::ZWithoutAxis { .. } => operation_numbers_text(cause, names),
        Cause::OutsideBed { .. }
        | Cause::NotAssigned
        | Cause::HiddenRuns
        | Cause::OutputOff { .. }
        | Cause::CutOutNoContour { .. }
        | Cause::DamagedSettings => count_text(cause, count, names),
        Cause::GreyNeedsM4 { .. }
        | Cause::TooManyDots { .. }
        | Cause::CoarseImages
        | Cause::OverlappingRasterItems
        | Cause::ReducedImages
        | Cause::UnreadableImages
        | Cause::NotPictures { .. }
        | Cause::LowestPowerAbovePower { .. } => raster_text(cause, count, names),
        Cause::BrokenAdjustments { item } => CheckText {
            before: format!(
                "{count} {} in the Adjustments of ",
                verb(count, "step", "steps")
            ),
            button: names.item(*item).to_owned(),
            after: " could not run.".to_owned(),
        },
        Cause::StaleResults { item } => CheckText {
            before: format!("{count} stored AI {} of ", verb(count, "result", "results")),
            button: names.item(*item).to_owned(),
            after: verb(count, " is out of date.", " are out of date.").to_owned(),
        },
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
    if let Some((dither, lowest)) = line.dither {
        text.push_str(" · ");
        text.push_str(&dither_text(dither, lowest, line.numbers.power));
    }
    text
}

/// The dither of an Image row's numbers line (`specs/0123` criterion 47 d, UX notes 2.3): the
/// strip's short name ("Floyd"), Grey with its range from Lowest power to Power.
fn dither_text(dither: Dither, lowest_tenths: u16, power: PowerPercent) -> String {
    match dither {
        Dither::Threshold => "Threshold".to_owned(),
        Dither::Ordered => "Ordered".to_owned(),
        Dither::FloydSteinberg => "Floyd".to_owned(),
        Dither::Jarvis => "Jarvis".to_owned(),
        Dither::Atkinson => "Atkinson".to_owned(),
        Dither::Stucki => "Stucki".to_owned(),
        Dither::Halftone => "Halftone".to_owned(),
        Dither::LineScreen => "Lines".to_owned(),
        Dither::Grey => {
            let (whole, tenth) = (lowest_tenths / 10, lowest_tenths % 10);
            let lowest = if tenth == 0 {
                whole.to_string()
            } else {
                format!("{whole}.{tenth}")
            };
            format!("Grey {lowest}–{power} %")
        }
    }
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
