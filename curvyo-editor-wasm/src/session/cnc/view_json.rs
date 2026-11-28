//! The active CNC page as JSON for the front end (`specs/0161-cnc-output` criteria 1 to 12, 18,
//! 19, `ux-notes.md` 3 to 8): machine, stock, tools, operations, checks, total, play, Export and
//! Send.

use curvyo_cnc_core::{BUILTINS, Side, Tool, number};
use serde_json::{Value, json};

use super::super::Session;
use super::CncPage;
use super::draw::head_at;
use super::job::CncJobView;

fn items(n: usize) -> String {
    if n == 1 {
        "1 item".to_owned()
    } else {
        format!("{n} items")
    }
}

/// "7 min 40 s", "12 s".
pub(in crate::session) fn duration(seconds: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: a run time is finite and positive, far below u64::MAX seconds
    let s = seconds.max(0.0).ceil() as u64;
    match (s / 60, s % 60) {
        (0, s) => format!("{s} s"),
        (m, 0) => format!("{m} min"),
        (m, s) => format!("{m} min {s} s"),
    }
}

/// `n` with a comma every three digits.
fn grouped(n: u32) -> String {
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

fn tool_line(t: &Tool) -> String {
    format!(
        "{} flutes · {} mm/min · {} rpm · {} mm down",
        t.flutes.get(),
        number(t.feed.get()),
        grouped(t.rpm.get()),
        number(t.step_down.get())
    )
}

/// The play readout at `t` seconds (UX 7): "2 min 3 s of 7 min 40 s · Z −5.00 mm".
pub(in crate::session) fn play_readout(job: &CncJobView, t: Option<f64>) -> Value {
    let total = job.time.total.get();
    let t = t.unwrap_or(total).clamp(0.0, total);
    let z = head_at(job, t).map_or_else(
        || job.job.moves.last().map_or(0.0, |m| m.to.z()),
        |(_, w)| w.z(),
    );
    let z_text = format!("{z:.2}").replace('-', "−");
    json!({
        "position": t,
        "length": total,
        "text": format!("{} of {} · Z {z_text} mm", duration(t), duration(total)),
    })
}

impl Session {
    /// The active CNC page's view; `null` on another page.
    #[must_use]
    pub fn cnc_view_json(&self) -> String {
        self.cnc_view()
            .map_or_else(|| "null".to_owned(), |v| v.to_string())
    }

    /// The machine list of Add page with CNC (criterion 1): every CNC profile with its page
    /// size, the Generic GRBL router chosen.
    #[must_use]
    pub fn cnc_profiles_json(&self) -> String {
        let profiles = self.cnc_profiles();
        let chosen = self.preselected_machine(
            curvyo_cnc_core::PROCESS_ID,
            profiles.iter().map(|p| p.id.as_str()),
            curvyo_cnc_core::DEFAULT_BUILTIN.id,
        );
        let rows: Vec<Value> = profiles
            .iter()
            .map(|p| {
                let size = p.values.work_area;
                json!({
                    "id": p.id,
                    "name": p.values.name,
                    "width": size.width.as_mm(),
                    "height": size.height.as_mm(),
                    "chosen": p.id == chosen,
                })
            })
            .collect();
        Value::Array(rows).to_string()
    }

    /// Moves the play position to `seconds` of the planner's time (session state, no step); the
    /// readout there.
    pub fn set_cnc_play(&mut self, seconds: f64) -> String {
        let Some(page) = self.active_cnc() else {
            return "null".to_owned();
        };
        let job = self.cnc_job(&page);
        let t = seconds.clamp(0.0, job.time.total.get());
        self.manufacturing.cnc.play = Some(t);
        play_readout(&job, Some(t)).to_string()
    }

    /// The view as a value, `None` on another page.
    pub(in crate::session) fn cnc_view(&self) -> Option<Value> {
        let page = self.active_cnc()?;
        let job = self.cnc_job(&page);
        let v = &page.copy.values;
        let format = v.output_format;
        let builtin = page
            .copy
            .id
            .as_deref()
            .is_none_or(|id| BUILTINS.iter().any(|b| b.id == id));
        let record = self.cnc_record();
        let profiles: Vec<Value> = self
            .cnc_profiles()
            .iter()
            .map(|p| json!({"id": p.id, "name": p.values.name, "builtin": BUILTINS.iter().any(|b| b.id == p.id)}))
            .collect();
        let tools: Vec<Value> = page
            .tools
            .iter()
            .map(|t| {
                json!({
                    "id": t.id,
                    "name": t.name(),
                    "line2": tool_line(t),
                    "numbers": t.to_json(),
                })
            })
            .collect();
        let checks = checks(&page, &job);
        let (cut, rapid) = job.job.lengths();
        let passes: u32 = job
            .job
            .reports
            .iter()
            .map(|r| r.passes * u32::try_from(r.loops).unwrap_or(0))
            .sum();
        let runs = job.list_index.len();
        let total_items: usize = job.list_index.iter().map(|&l| job.counts[l]).sum();
        Some(json!({
            "id": page.id.full_id(),
            "page": page.page.id,
            "profile": {
                "id": page.copy.id,
                "name": v.name,
                "builtin": builtin,
                "output": format.label(),
                "workArea": format!("{} × {} mm", number(v.work_area.width.as_mm()), number(v.work_area.height.as_mm())),
                "zTravel": v.z_travel.as_mm(),
                "spindleMax": v.spindle_max.get(),
                "safeHeight": v.safe_height.as_mm(),
                "connector": record.map(|r| r.values.connection.connector_address),
            },
            "profiles": profiles,
            "stock": {
                "width": page.stock.width.as_mm(),
                "height": page.stock.height.as_mm(),
                "thickness": page.stock.thickness.as_mm(),
                "material": page.stock.material,
            },
            "tools": tools,
            "operations": rows(&page, &job),
            "kinds": Side::ALL.iter().map(|&side| json!({
                "side": side.key(),
                "label": format!("Profile {}", side.key()),
                "description": page.last_of(side).map_or_else(
                    || format!("Creates {}", page.next_name()),
                    |op| format!("Assigns to {}", op.name),
                ),
            })).collect::<Vec<_>>(),
            "checks": checks,
            "total": {
                "time": (!job.is_empty()).then(|| format!("About {}", duration(job.time.total.get()))),
                "runs": format!("Runs {runs} {}, {}.", if runs == 1 { "operation" } else { "operations" }, items(total_items)),
                "lengths": format!("Cut {:.2} m · rapid {:.2} m · {passes} {}", cut / 1000.0, rapid / 1000.0, if passes == 1 { "pass" } else { "passes" }),
            },
            "machineLine": format!("{} · {}", v.name, format.label()),
            "exportAllowed": !job.blocked(),
            "fileType": format.file_type(),
            "toolpath": self.toolpath_view_pressed(page.id),
            "play": play_readout(&job, self.manufacturing.cnc.play),
            "send": self.cnc_connector().map(|(id, address)| json!({"profileId": id, "address": address, "enabled": !job.blocked()})),
            // Criterion 20: damaged values read as their defaults, with a notice.
            "notice": (page.damaged + job.damaged > 0)
                .then_some("Some stored values could not be read and show their defaults."),
        }))
    }
}

fn rows(page: &CncPage, job: &CncJobView) -> Vec<Value> {
    page.operations
        .iter()
        .enumerate()
        .map(|(index, op)| {
            let profile = CncPage::is_profile(op);
            let (s, _) = page.settings_of(op);
            let tool = page.tool(&s.tool);
            let count = job.counts.get(index).copied().unwrap_or(0);
            let plan_op = job.list_index.iter().position(|&l| l == index);
            let mut seconds = 0.0;
            for (i, m) in job.job.moves.iter().enumerate() {
                if Some(m.operation) == plan_op && m.pass > 0 {
                    let start = if i == 0 { 0.0 } else { job.move_end[i - 1] };
                    seconds += job.move_end[i] - start;
                }
            }
            let passes = s.passes(tool);
            let feed = s.feed.unwrap_or(tool.feed);
            let typed = if s.feed.is_some() { " typed" } else { "" };
            json!({
                "id": op.id,
                "name": op.name,
                "number": index + 1,
                "side": profile.then(|| s.side.key()),
                "line2": if profile { format!("Profile · {} · {}", s.side.label(), items(count)) } else { "Unknown kind".to_owned() },
                "line3": format!(
                    "Ø {} mm, {} mm deep, {passes} {}, {} mm/min{typed}",
                    number(tool.diameter.get()),
                    number(s.depth.get()),
                    if passes == 1 { "pass" } else { "passes" },
                    number(feed.get()),
                ),
                "time": (seconds > 0.0).then(|| duration(seconds)),
                "passes": passes,
                "numbers": profile.then(|| json!({
                    "side": s.side.key(),
                    "direction": s.direction.key(),
                    "depth_mm": s.depth.get(),
                    "tool": s.tool,
                    "feed_mm_min": feed.get(),
                    "plunge_mm_min": s.plunge.unwrap_or(tool.plunge).get(),
                    "feedTyped": s.feed.is_some(),
                    "plungeTyped": s.plunge.is_some(),
                    "toolFeed": tool.feed.get(),
                    "toolPlunge": tool.plunge.get(),
                })),
                "output": op.output,
                "unfolds": profile,
            })
        })
        .collect()
}

fn checks(page: &CncPage, job: &CncJobView) -> Vec<Value> {
    let mut out = Vec::new();
    let block = |text: String, items: &[curvyo_document_core::NodeId], op: Option<String>| json!({"severity": "block", "text": text, "items": items, "operation": op});
    let n = job.outside_stock.len();
    if n > 0 {
        let verb = if n == 1 { "lies" } else { "lie" };
        out.push(block(
            format!("{} {verb} outside the stock.", items(n)),
            &job.outside_stock,
            None,
        ));
    }
    let v = &page.copy.values;
    let area = format!(
        "{} × {} mm",
        number(v.work_area.width.as_mm()),
        number(v.work_area.height.as_mm())
    );
    for &l in &job.outside_area {
        let op = &page.operations[l];
        out.push(block(
            format!("{} leaves the {area} work area.", op.name),
            &[],
            Some(op.id.full_id()),
        ));
    }
    for &l in &job.other_tools {
        let op = &page.operations[l];
        out.push(block(
            format!("{} uses another tool. One tool per job for now.", op.name),
            &[],
            Some(op.id.full_id()),
        ));
    }
    for (plan_op, report) in job.job.reports.iter().enumerate() {
        let op = &page.operations[job.list_index[plan_op]];
        if report.too_narrow > 0 {
            let verb = if report.too_narrow == 1 { "is" } else { "are" };
            out.push(block(
                format!(
                    "{}: {} {verb} too narrow for this tool.",
                    op.name,
                    items(report.too_narrow)
                ),
                &[],
                Some(op.id.full_id()),
            ));
        }
        if report.open > 0 {
            let noun = if report.open == 1 {
                "open path is"
            } else {
                "open paths are"
            };
            out.push(json!({
                "severity": "warn",
                "text": format!("{}: {} {noun} not milled.", op.name, report.open),
                "items": [],
                "operation": op.id.full_id(),
            }));
        }
    }
    if job.is_empty() {
        out.push(block(
            "Nothing runs. Assign items to an operation.".to_owned(),
            &[],
            None,
        ));
    }
    if page.copy.id.as_deref() == Some(curvyo_cnc_core::GENERIC_GRBL_ROUTER.id) {
        out.push(json!({
            "severity": "warn",
            "text": format!("The machine is {}; its limits may not match yours.", v.name),
            "items": [],
            "operation": null,
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_the_ux_notes_write_them() {
        assert_eq!(duration(460.0), "7 min 40 s");
        assert_eq!(duration(12.0), "12 s");
        assert_eq!(grouped(12_000), "12,000");
        assert_eq!(
            tool_line(&Tool::default_with_id("t")),
            "2 flutes · 800 mm/min · 12,000 rpm · 1 mm down"
        );
    }
}
