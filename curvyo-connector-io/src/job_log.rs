//! `jobs.log`: one line per job outcome, append only, never leaving the connector (Q18).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::identity::make_dir;

/// The file name in the state directory.
pub const JOB_LOG_FILE: &str = "jobs.log";

/// How a job ended, in the words of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogResult {
    /// All lines ran.
    Completed,
    /// The maker stopped it.
    Stopped,
    /// The job failed, with the reason when the controller named one (`specs/0127` G7, G15).
    Failed(Option<String>),
    /// The controller raised an alarm with this text.
    Alarm(String),
    /// The submit was refused for this reason.
    Rejected(String),
    /// The app of this name sent `Unlock` in alarm (`specs/0127` G33); the line has no job name.
    Unlock(String),
}

impl LogResult {
    /// The result as the log and the terminal print it.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Completed => "Completed".to_owned(),
            Self::Stopped => "Stopped".to_owned(),
            Self::Failed(None) => "Failed".to_owned(),
            Self::Failed(Some(text)) => format!("Failed: {}", clean(text)),
            Self::Alarm(text) => format!("Alarm: {}", clean(text)),
            Self::Rejected(reason) => format!("Rejected: {}", clean(reason)),
            Self::Unlock(client) => format!("Unlock: {}", clean(client)),
        }
    }
}

/// Control characters (tab and line breaks too) become a space, so a line stays one line.
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// `<UTC time RFC 3339> TAB <job name> TAB <result>`, without the line end.
#[must_use]
pub fn format_line(at: OffsetDateTime, name: &str, result: &LogResult) -> String {
    let utc = at.to_offset(time::UtcOffset::UTC);
    let whole = utc.replace_nanosecond(0).unwrap_or(utc);
    let time = whole.format(&Rfc3339).unwrap_or_default();
    format!("{time}\t{}\t{}", clean(name), result.text())
}

/// The log of one state directory.
#[derive(Debug, Clone)]
pub struct JobLog {
    path: PathBuf,
}

impl JobLog {
    /// The log in `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(JOB_LOG_FILE),
        }
    }

    /// Appends one line.
    ///
    /// # Errors
    /// The I/O error, with the file left as it was.
    pub fn append(
        &self,
        at: OffsetDateTime,
        name: &str,
        result: &LogResult,
    ) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            make_dir(dir)?;
        }
        let mut options = OpenOptions::new();
        options.append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&self.path)?;
        writeln!(file, "{}", format_line(at, name, result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_matches_its_golden_file() {
        let dir = tempfile::tempdir().unwrap();
        let log = JobLog::new(dir.path());
        let t = |s: i64| OffsetDateTime::from_unix_timestamp(1_700_000_000 + s).unwrap();
        log.append(t(0), "coaster.gcode", &LogResult::Completed)
            .unwrap();
        log.append(t(60), "coaster.gcode", &LogResult::Stopped)
            .unwrap();
        log.append(t(120), "sign.nc", &LogResult::Failed(None))
            .unwrap();
        log.append(
            t(180),
            "sign.nc",
            &LogResult::Alarm("Fake alarm at line 3".into()),
        )
        .unwrap();
        log.append(t(240), "big.txt", &LogResult::Rejected("TooLarge".into()))
            .unwrap();
        log.append(
            t(300),
            "odd\tname\n.gcode",
            &LogResult::Alarm("a\tb".into()),
        )
        .unwrap();
        for (s, text) in [
            (360, "line 12, No feed rate (error 22)"),
            (420, "line 4, Contains !, ~ or ?"),
            (480, "controller lost at line 120"),
            (540, "could not confirm the machine stopped"),
        ] {
            log.append(
                t(s),
                "cut_line.gcode",
                &LogResult::Failed(Some(text.into())),
            )
            .unwrap();
        }
        log.append(
            t(600),
            "cut_line.gcode",
            &LogResult::Alarm("Hard limit hit, position may be lost (alarm 1)".into()),
        )
        .unwrap();
        log.append(t(660), "", &LogResult::Unlock("Laptop".into()))
            .unwrap();
        let text = std::fs::read_to_string(dir.path().join(JOB_LOG_FILE)).unwrap();
        assert_eq!(text, include_str!("../tests/fixtures/jobs.log"));
    }
}
