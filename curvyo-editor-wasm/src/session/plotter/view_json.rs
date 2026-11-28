//! The active plotter page as JSON for the front end (`specs/0159-cutting-plotter` criteria 1 to
//! 10, `ux-notes.md` 4, 6, 7, 9): machine, mat, operations, checks, total and Export.

use curvyo_cutter_core::{BUILTINS, Grip, OpKind, OutputFormat};
use serde_json::{Value, json};

use super::super::Session;
use super::PlotterPage;
use super::job::PlotterJob;

/// "1 item", "3 items".
fn items(n: usize) -> String {
    if n == 1 {
        "1 item".to_owned()
    } else {
        format!("{n} items")
    }
}

/// "About 3 min 40 s", "About 12 s".
fn about(seconds: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: a job time is finite and positive, far below u64::MAX seconds
    let s = seconds.max(0.0).ceil() as u64;
    match (s / 60, s % 60) {
        (0, s) => format!("About {s} s"),
        (m, 0) => format!("About {m} min"),
        (m, s) => format!("About {m} min {s} s"),
    }
}

/// `mm` in metres with two decimals.
fn metres(mm: f64) -> String {
    format!("{:.2} m", mm / 1000.0)
}

/// "#RRGGBB" of a grip.
fn hex(grip: Grip) -> String {
    format!("#{:06X}", grip.colour())
}

impl Session {
    /// The active plotter page's view; `null` on another page.
    #[must_use]
    pub fn plotter_view_json(&self) -> String {
        self.plotter_view()
            .map_or_else(|| "null".to_owned(), |v| v.to_string())
    }

    /// The machine list of Add page with Plotter (criterion 2): every plotter profile with its
    /// page size, the Cricut Maker 3 chosen, as `[{"id", "name", "width", "height", "chosen"}]`.
    #[must_use]
    pub fn plotter_profiles_json(&self) -> String {
        let rows: Vec<Value> = self
            .plotter_profiles()
            .iter()
            .map(|p| {
                let size = p.values.page_size();
                json!({
                    "id": p.id,
                    "name": p.values.name,
                    "width": (size.width.as_mm() * 10.0).round() / 10.0,
                    "height": (size.height.as_mm() * 10.0).round() / 10.0,
                    "chosen": p.id == curvyo_cutter_core::DEFAULT_BUILTIN.id,
                })
            })
            .collect();
        Value::Array(rows).to_string()
    }

    /// The view as a value, `None` on another page.
    pub(in crate::session) fn plotter_view(&self) -> Option<Value> {
        let page = self.active_plotter()?;
        let built = self.plotter_job(&page, false);
        let values = &page.copy.values;
        let format = values.output_format;
        let hidden_numbers = format.sets_numbers();
        let area = page.cut_area();
        let profiles: Vec<Value> = self
            .plotter_profiles()
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "name": p.values.name,
                    "builtin": BUILTINS.iter().any(|b| b.id == p.id),
                })
            })
            .collect();
        let mat = page.active_mat().map(|m| {
            json!({
                "size": m.size.key(),
                "sizes": values.mats.iter().map(|s| json!({"key": s.key(), "label": s.label()})).collect::<Vec<_>>(),
                "grip": m.grip.key(),
                "gripWord": m.grip.word(),
                "gripColour": hex(m.grip),
                "grips": Grip::ALL.iter().map(|g| json!({"key": g.key(), "word": g.word(), "colour": hex(*g)})).collect::<Vec<_>>(),
                "grid": m.grid,
            })
        });
        let operations = rows(&page, &built, hidden_numbers);
        let lengths = built.job.lengths_mm();
        let running: usize = built.counts.iter().sum();
        let runs = page
            .operations
            .iter()
            .filter(|op| op.output && PlotterPage::kind_of(op).is_some())
            .count();
        let checks = checks(&page, &built);
        let blocked = checks.iter().any(|c| c["severity"] == "block");
        let total = json!({
            "time": (!hidden_numbers && !built.job.is_empty()).then(|| about(built.job.seconds())),
            "runs": format!("Runs {} {}, {}.", runs, if runs == 1 { "operation" } else { "operations" }, items(running)),
            "lengths": format!("Cut {} · draw {} · travel {}", metres(lengths.cut), metres(lengths.draw), metres(lengths.travel)),
        });
        Some(json!({
            "id": page.id.full_id(),
            "page": page.page.id,
            "profile": {
                "id": page.copy.id,
                "name": values.name,
                "outputFormat": format.key(),
                "output": format.label(),
                "setsNumbers": hidden_numbers,
                "maxSpeed": values.max_speed.get(),
                "defaultPressure": values.default_pressure.get(),
                "defaultSpeed": values.default_speed.get(),
                "matless": values.mats.is_empty(),
                "area": format!("{:.1} × {:.1} mm", area.w, area.h),
            },
            "profiles": profiles,
            "mat": mat,
            "operations": operations,
            "kinds": OpKind::ALL.iter().map(|&kind| {
                let last = page.operations.iter().rev().find(|op| PlotterPage::kind_of(op) == Some(kind));
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
            "exportAllowed": !blocked && !built.job.is_empty(),
            "fileType": format.label(),
            "toolpath": self.toolpath_view_pressed(page.id),
            // Criterion 17: damaged values read as their defaults, with a notice.
            "notice": (page.copy_damaged + page.mat.map_or(0, |m| m.1.damaged) + built.damaged > 0)
                .then_some("Some stored values could not be read and show their defaults."),
        }))
    }
}

fn rows(page: &PlotterPage, built: &PlotterJob, hidden: bool) -> Vec<Value> {
    page.operations
            .iter()
            .enumerate()
            .map(|(index, op)| {
                let kind = PlotterPage::kind_of(op);
                let count = built.counts.get(index).copied().unwrap_or(0);
                let (numbers, line3) = match kind {
                    Some(kind) => {
                        let (s, _) = page.settings_of(kind, op);
                        let line = if hidden {
                            "Set in Design Space".to_owned()
                        } else if kind == OpKind::Cut {
                            format!("{} %, {} mm/s, {} ×", s.pressure.get(), s.speed.get(), s.passes.get())
                        } else {
                            format!("{} %, {} mm/s", s.pressure.get(), s.speed.get())
                        };
                        (
                            json!({"pressure": s.pressure.get(), "speed": s.speed.get(), "passes": s.passes.get()}),
                            line,
                        )
                    }
                    None => (Value::Null, "Unknown kind".to_owned()),
                };
                json!({
                    "id": op.id,
                    "name": op.name,
                    "kind": kind.map(OpKind::key),
                    "number": index + 1,
                    "line2": format!("{} · {}", kind.map_or("Unknown", OpKind::label), items(count)),
                    "line3": line3,
                    "numbers": numbers,
                    "output": op.output,
                    "unfolds": !hidden && kind.is_some(),
                })
            })
            .collect()
}

fn checks(page: &PlotterPage, built: &PlotterJob) -> Vec<Value> {
    let mut out = Vec::new();
    if !built.outside.is_empty() {
        let n = built.outside.len();
        let verb = if n == 1 { "lies" } else { "lie" };
        let text = if page.active_mat().is_some() {
            format!("{} {verb} outside the mat's cut area.", items(n))
        } else {
            let area = page.cut_area();
            format!(
                "{} {verb} outside the {:.0} × {:.0} mm work area.",
                items(n),
                area.w,
                area.h
            )
        };
        out.push(json!({
            "severity": "block",
            "text": text,
            "items": built.outside,
        }));
    }
    if built.job.is_empty() {
        out.push(json!({"severity": "block", "text": "Nothing runs. Assign items to an operation.", "items": []}));
    }
    if page.copy.id.as_deref() == Some(curvyo_cutter_core::GENERIC_HPGL_CUTTER.id)
        && page.copy.values.output_format == OutputFormat::Hpgl
    {
        out.push(json!({
            "severity": "warn",
            "text": "The machine is Generic HPGL cutter; its limits may not match yours.",
            "items": [],
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_and_lengths_read_as_the_ux_notes_write_them() {
        assert_eq!(about(220.0), "About 3 min 40 s");
        assert_eq!(about(11.2), "About 12 s");
        assert_eq!(about(120.0), "About 2 min");
        assert_eq!(metres(1240.0), "1.24 m");
        assert_eq!(hex(Grip::Standard), "#9CB87D");
    }
}
