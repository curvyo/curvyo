//! `jobs.log`: one line per job outcome with the clients behind it, append only, never leaving
//! the connector (Q18, `specs/0128` A15).

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
    /// The app of this name sent `Home` and the connector took it (`specs/0118` M8); no job name.
    Home(String),
    /// The app of this name sent a frame the connector ran (`specs/0118` 2b); no job name.
    Frame(String),
    /// The app of this name started a jog pad session (`specs/0118` criterion 12); no job name.
    Jog(String),
    /// The app of this name sent a go to point the connector took (`specs/0118` 2d); no job name.
    FocusPoint(String),
    /// The connector wrote the pulse block of this app's go to point (`specs/0118` 35 c).
    FocusPulse(String),
    /// A start that waited for an owner was withdrawn or discarded (`specs/0128` A14).
    Discarded,
}

/// The clients behind a line: who sent the job, who pressed Start, which owner confirmed it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogPeople {
    /// Who sent the job.
    pub sent_by: String,
    /// Who pressed Start; empty when it never started.
    pub started_by: String,
    /// The owner who confirmed the start; empty for a start that ran at once and for Enter.
    pub confirmed_by: String,
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
            Self::Home(client) => format!("Home: {}", clean(client)),
            Self::Jog(client) => format!("Jog: {}", clean(client)),
            Self::Frame(client) => format!("Frame: {}", clean(client)),
            Self::FocusPoint(client) => format!("Focus point: {}", clean(client)),
            Self::FocusPulse(client) => format!("Focus pulse: {}", clean(client)),
            Self::Discarded => "Discarded".to_owned(),
        }
    }
}

/// Control characters (tab and line breaks too) become a space, so a line stays one line.
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// RFC 3339 UTC in whole seconds, as both logs write it.
#[must_use]
pub fn log_time(at: OffsetDateTime) -> String {
    let utc = at.to_offset(time::UtcOffset::UTC);
    let whole = utc.replace_nanosecond(0).unwrap_or(utc);
    whole.format(&Rfc3339).unwrap_or_default()
}

/// `<time> TAB <job name> TAB <result> TAB <sent by> TAB <started by> TAB <confirmed by>`,
/// without the line end (addendum D9).
#[must_use]
pub fn format_line(
    at: OffsetDateTime,
    name: &str,
    result: &LogResult,
    people: &LogPeople,
) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        log_time(at),
        clean(name),
        result.text(),
        clean(&people.sent_by),
        clean(&people.started_by),
        clean(&people.confirmed_by)
    )
}

/// Appends one line to the private log at `path`, made `0600` at creation.
///
/// # Errors
/// The I/O error, with the file left as it was.
pub(crate) fn append_private(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        make_dir(dir)?;
    }
    let mut options = OpenOptions::new();
    options.append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    writeln!(file, "{line}")
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
        people: &LogPeople,
    ) -> std::io::Result<()> {
        append_private(&self.path, &format_line(at, name, result, people))
    }

    /// Where the log is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_matches_its_golden_file() {
        let dir = tempfile::tempdir().unwrap();
        let log = JobLog::new(dir.path());
        let none = LogPeople::default();
        let t = |s: i64| OffsetDateTime::from_unix_timestamp(1_700_000_000 + s).unwrap();
        log.append(t(0), "coaster.gcode", &LogResult::Completed, &none)
            .unwrap();
        log.append(t(60), "coaster.gcode", &LogResult::Stopped, &none)
            .unwrap();
        log.append(t(120), "sign.nc", &LogResult::Failed(None), &none)
            .unwrap();
        log.append(
            t(180),
            "sign.nc",
            &LogResult::Alarm("Fake alarm at line 3".into()),
            &none,
        )
        .unwrap();
        log.append(
            t(240),
            "big.txt",
            &LogResult::Rejected("TooLarge".into()),
            &none,
        )
        .unwrap();
        log.append(
            t(300),
            "odd\tname\n.gcode",
            &LogResult::Alarm("a\tb".into()),
            &none,
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
                &none,
            )
            .unwrap();
        }
        log.append(
            t(600),
            "cut_line.gcode",
            &LogResult::Alarm("Hard limit hit, position may be lost (alarm 1)".into()),
            &none,
        )
        .unwrap();
        log.append(t(660), "", &LogResult::Unlock("Laptop".into()), &none)
            .unwrap();
        log.append(t(720), "", &LogResult::Home("Laptop".into()), &none)
            .unwrap();
        let people = LogPeople {
            sent_by: "Phone".into(),
            started_by: "Phone".into(),
            confirmed_by: "Laptop".into(),
        };
        log.append(t(780), "sign.gcode", &LogResult::Completed, &people)
            .unwrap();
        let discarded = LogPeople {
            sent_by: "Phone".into(),
            ..LogPeople::default()
        };
        log.append(t(840), "cut.gcode", &LogResult::Discarded, &discarded)
            .unwrap();
        let text = std::fs::read_to_string(dir.path().join(JOB_LOG_FILE)).unwrap();
        assert_eq!(text, include_str!("../tests/fixtures/jobs_v2.log"));
    }

    #[test]
    fn the_go_to_point_lines_match_their_golden_file() {
        let dir = tempfile::tempdir().unwrap();
        let log = JobLog::new(dir.path());
        let none = LogPeople::default();
        let t = |s: i64| OffsetDateTime::from_unix_timestamp(1_700_000_000 + s).unwrap();
        log.append(t(900), "", &LogResult::FocusPoint("Laptop".into()), &none)
            .unwrap();
        log.append(t(960), "", &LogResult::FocusPulse("Laptop".into()), &none)
            .unwrap();
        let text = std::fs::read_to_string(dir.path().join(JOB_LOG_FILE)).unwrap();
        assert_eq!(text, include_str!("../tests/fixtures/jobs_focus.log"));
    }
}
