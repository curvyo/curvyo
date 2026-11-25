//! `admin.log`: one line per owner action, append only, `0600`, never leaving the host (A18;
//! addendum D9). Five tab-separated fields: time, event, by, subject, detail.

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use curvyo_connector_core::Role;
use time::OffsetDateTime;

use crate::job_log::{append_private, log_time};

/// The file name in the state directory.
pub const ADMIN_LOG_FILE: &str = "admin.log";

/// One owner action; `by` (the acting client, empty for the host) is given beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminEvent {
    /// The first configuration, from this peer.
    Configured {
        /// The sender's address.
        peer: IpAddr,
    },
    /// A later configuration changed these top-level fields.
    ConfigChanged {
        /// `controller`, `listen`, `name`, `owner`, in that order.
        fields: Vec<&'static str>,
    },
    /// A pairing code was issued for this name and role; never the code.
    CodeIssued {
        /// The name the pairing will be recorded under.
        name: String,
        /// Its role.
        role: Role,
    },
    /// The acting client paired with this role.
    Paired {
        /// Its role.
        role: Role,
    },
    /// A client was unpaired.
    Revoked {
        /// Its name.
        name: String,
        /// Its role.
        role: Role,
    },
    /// The controller's alarm lock was left.
    Unlocked,
    /// A start or resume that waited was confirmed.
    StartConfirmed {
        /// Who asked for it.
        requester: String,
        /// The job's name.
        job_name: String,
        /// The job's id.
        job_id: String,
        /// A resume rather than a start.
        resume: bool,
    },
    /// A member's Home that waited was confirmed (`specs/0118` 37); the line names Home.
    HomeConfirmed {
        /// Who asked for it.
        requester: String,
    },
    /// An owner removed a job another client sent from the job list.
    JobRemoved {
        /// The job's name.
        job_name: String,
        /// The job's id.
        job_id: String,
        /// Who sent it.
        sent_by: String,
    },
    /// "Run next automatically" was set or cleared on a listed job.
    RunNextSet {
        /// The job's name.
        job_name: String,
        /// The job's id.
        job_id: String,
        /// The new value.
        on: bool,
    },
    /// An owner restarted an interrupted job from a line (`specs/0132` criterion 13).
    RestartFromLine {
        /// The job's name.
        job_name: String,
        /// The job's id.
        job_id: String,
        /// The line.
        line: u32,
    },
    /// A stored job did not come back after a restart: its name, then its id and the cause.
    JobDropped {
        /// The job's name.
        job_name: String,
        /// `<id> <cause>`.
        detail: String,
    },
    /// The stored job list could not be read: every stored job was dropped.
    JobListDropped {
        /// The cause.
        detail: String,
    },
}

/// `owner` or `member`, as the file and `curvyoctl` write a role.
#[must_use]
pub fn role_word(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Member => "member",
    }
}

fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

impl AdminEvent {
    fn fields(&self) -> (&'static str, String, String) {
        match self {
            Self::Configured { peer } => ("configured", String::new(), peer.to_string()),
            Self::ConfigChanged { fields } => ("config-changed", String::new(), fields.join(", ")),
            Self::CodeIssued { name, role } => {
                ("code-issued", name.clone(), role_word(*role).to_owned())
            }
            Self::Paired { role } => ("paired", String::new(), role_word(*role).to_owned()),
            Self::Revoked { name, role } => ("revoked", name.clone(), role_word(*role).to_owned()),
            Self::Unlocked => ("unlocked", String::new(), String::new()),
            Self::StartConfirmed {
                requester,
                job_name,
                job_id,
                resume,
            } => {
                let tail = if *resume { " resume" } else { "" };
                (
                    "start-confirmed",
                    requester.clone(),
                    format!("{job_name} {job_id}{tail}"),
                )
            }
            Self::HomeConfirmed { requester } => {
                ("start-confirmed", requester.clone(), "Home".to_owned())
            }
            Self::JobRemoved {
                job_name,
                job_id,
                sent_by,
            } => (
                "job-removed",
                job_name.clone(),
                format!("{job_id} sent by {sent_by}"),
            ),
            Self::RunNextSet {
                job_name,
                job_id,
                on,
            } => (
                "run-next-set",
                job_name.clone(),
                format!("{job_id} {}", if *on { "on" } else { "off" }),
            ),
            Self::RestartFromLine {
                job_name,
                job_id,
                line,
            } => (
                "restart-from-line",
                job_name.clone(),
                format!("{job_id} line {line}"),
            ),
            Self::JobDropped { job_name, detail } => {
                ("job-dropped", job_name.clone(), detail.clone())
            }
            Self::JobListDropped { detail } => ("job-list-dropped", String::new(), detail.clone()),
        }
    }
}

/// `<time> TAB <event> TAB <by> TAB <subject> TAB <detail>`, control characters as a space.
#[must_use]
pub fn format_line(at: OffsetDateTime, by: &str, event: &AdminEvent) -> String {
    let (name, subject, detail) = event.fields();
    format!(
        "{}\t{name}\t{}\t{}\t{}",
        log_time(at),
        clean(by),
        clean(&subject),
        clean(&detail)
    )
}

/// The admin log of one state directory.
#[derive(Debug, Clone)]
pub struct AdminLog {
    path: PathBuf,
}

impl AdminLog {
    /// The log in `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(ADMIN_LOG_FILE),
        }
    }

    /// Where the log is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one line.
    ///
    /// # Errors
    /// The I/O error.
    pub fn append(&self, at: OffsetDateTime, by: &str, event: &AdminEvent) -> std::io::Result<()> {
        append_private(&self.path, &format_line(at, by, event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a18_the_log_matches_its_golden_file() {
        let dir = tempfile::tempdir().unwrap();
        let log = AdminLog::new(dir.path());
        let t = |s: i64| OffsetDateTime::from_unix_timestamp(1_795_010_771 + s).unwrap();
        let ctl = "curvyoctl@marc-laptop";
        let events = [
            (
                0,
                ctl,
                AdminEvent::Configured {
                    peer: "192.168.1.30".parse().unwrap(),
                },
            ),
            (
                149,
                ctl,
                AdminEvent::CodeIssued {
                    name: "Laptop".into(),
                    role: Role::Owner,
                },
            ),
            (171, "Laptop", AdminEvent::Paired { role: Role::Owner }),
            (
                369,
                ctl,
                AdminEvent::CodeIssued {
                    name: "Phone".into(),
                    role: Role::Member,
                },
            ),
            (414, "Phone", AdminEvent::Paired { role: Role::Member }),
            (
                830,
                "Laptop",
                AdminEvent::StartConfirmed {
                    requester: "Phone".into(),
                    job_name: "sign.gcode".into(),
                    job_id: "67e55044-10b1-426f-9247-bb680e5fe0c8".into(),
                    resume: false,
                },
            ),
            (
                1493,
                ctl,
                AdminEvent::Revoked {
                    name: "Phone".into(),
                    role: Role::Member,
                },
            ),
            (1729, "Laptop", AdminEvent::Unlocked),
            (
                3229,
                ctl,
                AdminEvent::ConfigChanged {
                    fields: vec!["controller", "name"],
                },
            ),
            (
                3300,
                "",
                AdminEvent::StartConfirmed {
                    requester: "odd\tname".into(),
                    job_name: "cut.gcode".into(),
                    job_id: "67e55044-10b1-426f-9247-bb680e5fe0c8".into(),
                    resume: true,
                },
            ),
        ];
        for (s, by, event) in events {
            log.append(t(s), by, &event).unwrap();
        }
        let text = std::fs::read_to_string(dir.path().join(ADMIN_LOG_FILE)).unwrap();
        assert_eq!(text, include_str!("../tests/fixtures/admin.log"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(log.path()).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    /// `specs/0118` slice 2a: a confirmed Home is the same event with the detail `Home`; its
    /// golden is a file of its own, the first one stays as it was.
    #[test]
    fn a_confirmed_home_is_a_start_confirmed_line_that_names_home() {
        let dir = tempfile::tempdir().unwrap();
        let log = AdminLog::new(dir.path());
        let t = |s: i64| OffsetDateTime::from_unix_timestamp(1_795_010_771 + s).unwrap();
        log.append(
            t(0),
            "Laptop",
            &AdminEvent::HomeConfirmed {
                requester: "Phone".into(),
            },
        )
        .unwrap();
        log.append(
            t(60),
            "",
            &AdminEvent::HomeConfirmed {
                requester: "odd\tname".into(),
            },
        )
        .unwrap();
        let text = std::fs::read_to_string(dir.path().join(ADMIN_LOG_FILE)).unwrap();
        assert_eq!(text, include_str!("../tests/fixtures/admin_home.log"));
    }

    /// `specs/0132` criterion 13 (`specs/0118` adrs R10): a restart from a line names the owner,
    /// the job and the line; a golden of its own.
    #[test]
    fn a_restart_from_a_line_names_owner_job_and_line() {
        let dir = tempfile::tempdir().unwrap();
        let log = AdminLog::new(dir.path());
        let t = OffsetDateTime::from_unix_timestamp(1_795_014_131).unwrap();
        let event = AdminEvent::RestartFromLine {
            job_name: "sign.gcode".into(),
            job_id: "67e55044-10b1-426f-9247-bb680e5fe0c8".into(),
            line: 1234,
        };
        log.append(t, "Laptop", &event).unwrap();
        let text = std::fs::read_to_string(dir.path().join(ADMIN_LOG_FILE)).unwrap();
        assert_eq!(text, include_str!("../tests/fixtures/admin_restart.log"));
    }
}
