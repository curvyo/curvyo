//! The active embroidery page as JSON for the front end (`specs/0160-embroidery` criteria 1 to 6,
//! 9, 12, 13, `ux-notes.md` 3 to 7): machine, hoop, operations, checks, total, play and Export.

use curvyo_embroidery_core::{
    BUILTINS, EmbroiderySettings, OpKind, StitchKind, StitchPlan, summarise,
};
use serde_json::{Value, json};

use super::super::Session;
use super::EmbroideryPage;
use super::job::EmbroideryJob;

/// "1 item", "3 items".
fn items(n: usize) -> String {
    if n == 1 {
        "1 item".to_owned()
    } else {
        format!("{n} items")
    }
}

/// "About 14 min 56 s", "About 12 s".
fn about(seconds: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: a sewing time is finite and positive, far below u64::MAX seconds
    let s = seconds.max(0.0).ceil() as u64;
    match (s / 60, s % 60) {
        (0, s) => format!("About {s} s"),
        (m, 0) => format!("About {m} min"),
        (m, s) => format!("About {m} min {s} s"),
    }
}

/// `n` with a comma every three digits ("8,960").
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A length without trailing zeros ("2.5 mm", "3 mm").
fn mm(value: f64) -> String {
    format!("{} mm", (value * 100.0).round() / 100.0)
}

/// The row's third line (UX 5.2).
fn line3(kind: OpKind, s: EmbroiderySettings) -> String {
    match kind {
        OpKind::Running => format!("{}, {} ×", mm(s.stitch_length().get()), s.repeats()),
        OpKind::Tatami => format!(
            "{}°, {}, {}",
            s.angle_degrees(),
            mm(s.row_spacing().get()),
            mm(s.max_stitch().get())
        ),
        OpKind::Satin => mm(s.zigzag_spacing().get()),
    }
}

/// The play readout at `position` of `plan` (UX 6.2): the needle points sewn, of all, and the
/// colour block with its thread.
pub(in crate::session) fn play_readout(plan: &StitchPlan, position: usize) -> Value {
    let position = position.min(plan.stitches.len());
    let sewn = plan.stitches[..position]
        .iter()
        .filter(|s| s.kind == StitchKind::Normal)
        .count();
    let total = plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Normal)
        .count();
    let changes = |upto: usize| {
        plan.stitches[..upto]
            .iter()
            .filter(|s| s.kind == StitchKind::ColourChange)
            .count()
    };
    let block = plan
        .blocks
        .iter()
        .rev()
        .find(|b| b.stitches.start < position.max(1))
        .or_else(|| plan.blocks.first());
    json!({
        "position": position,
        "length": plan.stitches.len(),
        "text": format!("{} of {}", grouped(sewn), grouped(total)),
        "colour": block.map(|b| b.thread.hex()),
        "colourText": format!("{} of {}", changes(position) + 1, changes(plan.stitches.len()) + 1),
    })
}

impl Session {
    /// The active embroidery page's view; `null` on another page.
    #[must_use]
    pub fn embroidery_view_json(&self) -> String {
        self.embroidery_view()
            .map_or_else(|| "null".to_owned(), |v| v.to_string())
    }

    /// The machine list of Add page with Embroidery (criterion 2): every embroidery profile with
    /// its page size, the W6 EU7 chosen, as `[{"id", "name", "width", "height", "chosen"}]`.
    #[must_use]
    pub fn embroidery_profiles_json(&self) -> String {
        let profiles = self.embroidery_profiles();
        let chosen = self.preselected_machine(
            curvyo_embroidery_core::PROCESS_ID,
            profiles.iter().map(|p| p.id.as_str()),
            curvyo_embroidery_core::DEFAULT_BUILTIN.id,
        );
        let rows: Vec<Value> = profiles
            .iter()
            .map(|p| {
                let size = p.values.page_size();
                json!({
                    "id": p.id,
                    "name": p.values.name,
                    "width": (size.width.as_mm() * 10.0).round() / 10.0,
                    "height": (size.height.as_mm() * 10.0).round() / 10.0,
                    "chosen": p.id == chosen,
                })
            })
            .collect();
        Value::Array(rows).to_string()
    }

    /// The view as a value, `None` on another page.
    pub(in crate::session) fn embroidery_view(&self) -> Option<Value> {
        let page = self.active_embroidery()?;
        let job = self.embroidery_job(&page);
        let values = &page.copy.values;
        let format = values.output_format;
        let profiles: Vec<Value> = self
            .embroidery_profiles()
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "name": p.values.name,
                    "builtin": BUILTINS.iter().any(|b| b.id == p.id),
                })
            })
            .collect();
        let checks = checks(&page, &job);
        let blocked = checks.iter().any(|c| c["severity"] == "block");
        let total = total(&page, &job);
        let position = self
            .manufacturing
            .embroidery
            .play
            .unwrap_or(job.plan.stitches.len());
        Some(json!({
            "id": page.id.full_id(),
            "page": page.page.id,
            "profile": {
                "id": page.copy.id,
                "name": values.name,
                "output": format.label(),
                "speed": values.speed.get(),
                "maxStitch": mm(values.max_stitch.mm()),
                "hoop": page.hoop.size.label(),
            },
            "profiles": profiles,
            "hoop": {
                "key": page.hoop.size.key(),
                "sizes": values.hoops.iter().map(|h| json!({"key": h.key(), "label": h.label()})).collect::<Vec<_>>(),
            },
            "operations": rows(&page, &job, values.speed.get()),
            "kinds": OpKind::ALL.iter().map(|&kind| {
                let last = page.operations.iter().rev().find(|op| EmbroideryPage::kind_of(op) == Some(kind));
                json!({
                    "kind": kind.key(),
                    "label": kind.label(),
                    "description": last.map_or_else(
                        || format!("Creates {}", page.next_name(kind)),
                        |op| format!("Assigns to {}", op.name),
                    ),
                })
            }).collect::<Vec<_>>(),
            "checks": checks,
            "total": total,
            "machineLine": format!("{} · {}", values.name, format.label()),
            "exportAllowed": !blocked && !job.is_empty(),
            "fileType": format.file_type(),
            "toolpath": self.toolpath_view_pressed(page.id),
            "play": play_readout(&job.plan, position),
            "stitchesPerSecond": f64::from(values.speed.get()) / 60.0,
            // Criterion 20: damaged values read as their defaults, with a notice.
            "notice": (page.copy_damaged + usize::from(page.hoop.damaged) + job.damaged > 0)
                .then_some("Some stored values could not be read and show their defaults."),
        }))
    }

    /// Moves the simulation to `position` stitches of the list (criterion 13, session state, no
    /// step); the readout at the new position.
    pub fn set_embroidery_play(&mut self, position: usize) -> String {
        let Some(page) = self.active_embroidery() else {
            return "null".to_owned();
        };
        let job = self.embroidery_job(&page);
        let position = position.min(job.plan.stitches.len());
        self.manufacturing.embroidery.play = Some(position);
        play_readout(&job.plan, position).to_string()
    }
}

/// The output group's total (UX 7): the time, the runs and the stitch line.
fn total(page: &EmbroideryPage, job: &EmbroideryJob) -> Value {
    let summary = summarise(&job.plan.stitches, page.copy.values.speed);
    let runs = page
        .operations
        .iter()
        .filter(|op| op.output && EmbroideryPage::kind_of(op).is_some())
        .count();
    let running: usize = job.counts.iter().sum();
    let plural = |n: usize, one: &'static str, many: &'static str| if n == 1 { one } else { many };
    json!({
        "time": (!job.is_empty()).then(|| about(summary.seconds)),
        "runs": format!("Runs {runs} {}, {}.", plural(runs, "operation", "operations"), items(running)),
        "stitches": format!(
            "{} stitches · {} {} · {} {} · thread {:.1} m",
            grouped(summary.stitches),
            summary.colours,
            plural(summary.colours, "colour", "colours"),
            summary.trims,
            plural(summary.trims, "trim", "trims"),
            summary.thread_mm / 1000.0
        ),
    })
}

fn rows(page: &EmbroideryPage, job: &EmbroideryJob, speed: u16) -> Vec<Value> {
    let mut previous = None;
    page.operations
        .iter()
        .enumerate()
        .map(|(index, op)| {
            let kind = EmbroideryPage::kind_of(op);
            let count = job.counts.get(index).copied().unwrap_or(0);
            let read = kind.map(|k| EmbroideryPage::settings_of(k, op));
            let thread = read.map(|r| r.1.hex());
            let starts_colour = op.output && thread.is_some() && thread != previous;
            if op.output && thread.is_some() {
                previous.clone_from(&thread);
            }
            let plan_op = job.list_index.iter().position(|&l| l == index);
            let stitches = job
                .plan
                .blocks
                .iter()
                .filter(|b| Some(b.operation) == plan_op)
                .map(|b| {
                    job.plan.stitches[b.stitches.clone()]
                        .iter()
                        .filter(|s| s.kind == StitchKind::Normal)
                        .count()
                })
                .sum::<usize>();
            #[allow(clippy::cast_precision_loss)] // far below 2^52 stitches
            let seconds = stitches as f64 * 60.0 / f64::from(speed);
            json!({
                "id": op.id,
                "name": op.name,
                "kind": kind.map(OpKind::key),
                "number": index + 1,
                "thread": thread,
                "startsColour": starts_colour,
                "line2": format!("{} · {}", kind.map_or("Unknown", OpKind::label), items(count)),
                "time": (stitches > 0).then(|| about(seconds).trim_start_matches("About ").to_owned()),
                "line3": match (kind, read) {
                    (Some(k), Some((s, _, _))) => line3(k, s),
                    _ => "Unknown kind".to_owned(),
                },
                "numbers": read.map(|(s, _, _)| json!({
                    "stitch_length_mm": s.stitch_length().get(),
                    "repeats": s.repeats(),
                    "angle_deg": s.angle_degrees(),
                    "row_spacing_mm": s.row_spacing().get(),
                    "max_stitch_mm": s.max_stitch().get(),
                    "zigzag_spacing_mm": s.zigzag_spacing().get(),
                })),
                "output": op.output,
                "unfolds": kind.is_some(),
            })
        })
        .collect()
}

fn checks(page: &EmbroideryPage, job: &EmbroideryJob) -> Vec<Value> {
    let mut out = Vec::new();
    for (index, op) in page.operations.iter().enumerate() {
        let outside = &job.outside[index];
        if !outside.is_empty() {
            let verb = if outside.len() == 1 { "lies" } else { "lie" };
            out.push(json!({
                "severity": "block",
                "text": format!("{}: {} {verb} outside the hoop.", op.name, items(outside.len())),
                "items": outside,
            }));
        }
    }
    if job.is_empty() {
        out.push(json!({"severity": "block", "text": "Nothing runs. Assign items to an operation.", "items": []}));
    }
    for (index, op) in page.operations.iter().enumerate() {
        let narrow = job.narrow[index];
        if narrow > 0 {
            let verb = if narrow == 1 { "is" } else { "are" };
            out.push(json!({
                "severity": "warn",
                "text": format!("{}: {} {verb} too narrow and runs as running stitch.", op.name, items(narrow)),
                "items": [],
            }));
        }
        let empty = job.no_area[index];
        if empty > 0 {
            let verb = if empty == 1 { "has" } else { "have" };
            out.push(json!({
                "severity": "warn",
                "text": format!("{}: {} {verb} no closed area.", op.name, items(empty)),
                "items": [],
            }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_the_ux_notes_write_them() {
        assert_eq!(grouped(8_960), "8,960");
        assert_eq!(grouped(1_204_000), "1,204,000");
        assert_eq!(grouped(12), "12");
        assert_eq!(about(896.0), "About 14 min 56 s");
        let s = EmbroiderySettings::default();
        assert_eq!(line3(OpKind::Running, s), "2.5 mm, 1 ×");
        assert_eq!(line3(OpKind::Tatami, s), "0°, 0.25 mm, 3 mm");
        assert_eq!(line3(OpKind::Satin, s), "0.4 mm");
    }
}
