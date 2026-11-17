//! Export job file (`specs/0029` criteria 27 to 30; ADR 0064 §11; UX notes §7): the job file's
//! bytes, name and file type for the save dialog, and after a successful write the one step
//! "Export job file" with Lasered recorded beside it.

use std::collections::BTreeMap;

use curvyo_document_core::SettingsWrite;
use curvyo_laser_core::job::{ExportEffects, export_effects, extension, file_type, write};

use super::super::Session;
use super::super::materials::LibraryChange;

/// How long the export notice shows, in ms (criterion 27).
pub const OUTPUT_NOTICE_MS: u32 = 3_000;
/// How long it shows with its second line, in ms (criterion 27, UX P19).
pub const OUTPUT_NOTICE_LONG_MS: u32 = 5_000;
/// The longest line of the notice (UX notes §10).
const NOTICE_CHARS: usize = 70;

/// A job file handed to the frontend for saving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedJob {
    /// The file's bytes.
    pub bytes: Vec<u8>,
    /// The default file name ("Untitled - Laser 1.gcode").
    pub name: String,
    /// The save dialog's file type ("GRBL G-code").
    pub file_type: &'static str,
    /// The extension without its dot.
    pub extension: &'static str,
    /// The token [`Session::export_job_done`] takes after a successful write.
    pub token: u32,
}

/// The effects of the jobs handed out and not yet saved, by token: taken from the exported job,
/// so an edit while the dialog is open changes nothing of what is recorded (ADR 0064 §11).
#[derive(Debug, Default)]
pub(super) struct Exports {
    pending: BTreeMap<u32, ExportEffects>,
    next: u32,
}

/// `text` with every character a file name may not hold replaced by "-".
fn file_safe(text: &str) -> String {
    text.chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect()
}

/// The default name: "<project without extension> - <page>.<extension>" (criterion 27).
#[must_use]
pub fn job_file_name(project_file: &str, page: &str, extension: &str) -> String {
    let stem = project_file.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = stem.rsplit_once('.').map_or(stem, |(stem, _)| stem);
    let stem = if stem.trim().is_empty() {
        "Untitled"
    } else {
        stem
    };
    format!("{} - {}.{extension}", file_safe(stem), file_safe(page))
}

/// `line` cut with an ellipsis to [`NOTICE_CHARS`] characters.
fn fit(line: &str) -> String {
    if line.chars().count() <= NOTICE_CHARS {
        return line.to_owned();
    }
    let mut cut: String = line.chars().take(NOTICE_CHARS - 2).collect();
    cut.push('…');
    cut.push('.');
    cut
}

/// The notice after a write and how long it shows (criterion 27, UX P19).
#[must_use]
pub fn export_notice(name: &str, downloaded: bool, fixture_jobs_off: usize) -> (String, u32) {
    let verb = if downloaded { "Downloaded" } else { "Exported" };
    let first = fit(&format!("{verb} {name}."));
    match fixture_jobs_off {
        0 => (first, OUTPUT_NOTICE_MS),
        1 => (
            format!("{first}\nOutput turned off on 1 fixture job."),
            OUTPUT_NOTICE_LONG_MS,
        ),
        n => (
            format!("{first}\nOutput turned off on {n} fixture jobs."),
            OUTPUT_NOTICE_LONG_MS,
        ),
    }
}

impl Session {
    /// The job file of the active laser page for the save dialog (criterion 27), `None` while a
    /// block disables Export or on another page. Nothing is written to the project.
    pub fn export_job(&mut self, project_file: &str) -> Option<ExportedJob> {
        let job = self.laser_job()?;
        if !job.gate.export_allowed() {
            return None;
        }
        let format = job.input.machine.values.output_format;
        let effects = export_effects(&job.input, &job.built, self.materials().reads());
        let exports = &mut self.output.exports;
        let token = exports.next;
        exports.next = exports.next.wrapping_add(1);
        exports.pending.insert(token, effects);
        Some(ExportedJob {
            bytes: write(format, &job.built.job),
            name: job_file_name(project_file, &job.input.name, extension(format)),
            file_type: file_type(format),
            extension: extension(format),
            token,
        })
    }

    /// The job of `token` was written as `name` (criterion 28): one step "Export job file"
    /// switching its fixture jobs off and refreshing the stored numbers, then Lasered at
    /// `now_ms` on its test grids and their records, which an undo keeps. The notice and its
    /// milliseconds; `None` for an unknown token. A cancelled or failed write calls nothing.
    pub fn export_job_done(
        &mut self,
        token: u32,
        name: &str,
        downloaded: bool,
        now_ms: u64,
    ) -> Option<(String, u32)> {
        let effects = self.output.exports.pending.remove(&token)?;
        // Every older token is stale once one job is saved.
        self.output.exports.pending.retain(|&held, _| held > token);
        let writes: Vec<SettingsWrite> = effects
            .refreshed
            .iter()
            .map(|(operation, stored)| SettingsWrite {
                operation: *operation,
                value: match stored.to_value() {
                    serde_json::Value::Object(map) => Some(map),
                    _ => None,
                },
            })
            .collect();
        let jobs_on = self.fixture_jobs_with_output(&effects.fixture_jobs_off);
        self.recording_unguarded(|this| {
            this.document.export_job(&effects.fixture_jobs_off, &writes)
        });
        if !effects.lasered_grids.is_empty() {
            let mut change = LibraryChange::default();
            for &grid in &effects.lasered_grids {
                // A record that cannot be written leaves the document's Lasered standing.
                let _ = self.lasered_record(&mut change, grid, now_ms);
            }
            self.document
                .mark_test_grids_lasered(&effects.lasered_grids, now_ms);
            self.write_library_unjournalled(change);
        }
        Some(export_notice(name, downloaded, jobs_on))
    }

    /// How many of `jobs` have Output on now.
    fn fixture_jobs_with_output(&self, jobs: &[curvyo_document_core::NodeId]) -> usize {
        let Some(manufacturing) = self.active_manufacturing() else {
            return 0;
        };
        manufacturing
            .fixtures
            .iter()
            .flat_map(|f| f.jobs.iter())
            .filter(|job| job.output && jobs.contains(&job.id))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_name_takes_the_project_stem_and_the_page() {
        assert_eq!(
            job_file_name("", "Laser 1", "gcode"),
            "Untitled - Laser 1.gcode"
        );
        assert_eq!(
            job_file_name("/home/m/Coasters.curvyo", "Laser 1", "gcode"),
            "Coasters - Laser 1.gcode"
        );
        assert_eq!(
            job_file_name("a.b.curvyo", "L/1:2", "gcode"),
            "a.b - L-1-2.gcode"
        );
    }

    #[test]
    fn the_notice_has_a_second_line_for_fixture_jobs_and_shows_longer() {
        assert_eq!(
            export_notice("A - L.gcode", false, 0),
            ("Exported A - L.gcode.".to_owned(), OUTPUT_NOTICE_MS)
        );
        assert_eq!(
            export_notice("A - L.gcode", true, 2),
            (
                "Downloaded A - L.gcode.\nOutput turned off on 2 fixture jobs.".to_owned(),
                OUTPUT_NOTICE_LONG_MS
            )
        );
        let long = "x".repeat(100);
        let (text, _) = export_notice(&long, false, 0);
        assert_eq!(text.chars().count(), NOTICE_CHARS);
    }
}
