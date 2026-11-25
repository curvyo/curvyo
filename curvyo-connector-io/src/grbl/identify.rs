//! Identifies GRBL after the port opens (`specs/0127` G2, G17; addendum C4): soft reset first,
//! the banner within the limit, then `$I`, `$$` and one status report, one at a time; a stale
//! banner or `ok` from before the reset is not taken as an answer.

use std::time::Duration;

use crate::grbl::reply::{Reply, Report};
use crate::grbl::settings::Settings;

/// GRBL's soft reset, a real-time byte.
pub const RESET: u8 = 0x18;
/// A banner that comes again during identify restarts it, at most this often.
const MAX_RESTARTS: u8 = 2;
/// How much of another program's text the terminal shows.
const GOT_CHARS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Banner,
    Info,
    Settings,
    Report,
}

/// What identify found.
#[derive(Debug, Clone, PartialEq)]
pub struct Identified {
    /// The firmware version from the banner ("1.1h").
    pub version: String,
    /// `$$`.
    pub settings: Settings,
    /// The first status report.
    pub report: Report,
    /// The number of an `ALARM:<n>` seen during identify.
    pub alarm: Option<u16>,
    /// GRBL said `[MSG:'$H'|'$X' to unlock]`.
    pub locked: bool,
    /// The planner blocks `$I` named (`[OPT:V,15,128]`), for the restart line (`specs/0118` 39).
    pub planner: Option<u32>,
}

/// Why identify failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No banner within the limit; `got` is the start of other text, if any came.
    NoAnswer {
        /// Up to 40 printable characters.
        got: Option<String>,
    },
    /// A GRBL 0.9 banner.
    Unsupported,
}

/// How identify stands after a reply or a tick.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Still waiting.
    Waiting,
    /// Identified.
    Done(Box<Identified>),
    /// Failed.
    Failed(Failure),
}

/// The identify sequence.
#[derive(Debug, Clone)]
pub struct Identify {
    step: Step,
    limit: Duration,
    deadline: Duration,
    restarts: u8,
    got: Option<String>,
    version: String,
    settings: Settings,
    alarm: Option<u16>,
    locked: bool,
    planner: Option<u32>,
}

/// The planner blocks of an `[OPT:<codes>,<blocks>,<rx>]` line.
fn planner_blocks(text: &str) -> Option<u32> {
    let inner = text.strip_prefix("[OPT:")?.strip_suffix(']')?;
    inner
        .split(',')
        .nth(1)?
        .trim()
        .parse()
        .ok()
        .filter(|n| *n > 0)
}

impl Identify {
    /// Starts at `at`: the soft reset goes into `out`; the banner must come within `limit`.
    pub fn start(at: Duration, limit: Duration, out: &mut Vec<u8>) -> Self {
        out.push(RESET);
        Self {
            step: Step::Banner,
            limit,
            deadline: at + limit,
            restarts: 0,
            got: None,
            version: String::new(),
            settings: Settings::default(),
            alarm: None,
            locked: false,
            planner: None,
        }
    }

    /// Takes one line of GRBL output; the next command goes into `out`.
    pub fn on_reply(&mut self, reply: Reply, at: Duration, out: &mut Vec<u8>) -> Outcome {
        match reply {
            Reply::Banner(version) => return self.on_banner(version, at, out),
            Reply::Alarm(n) => self.alarm = Some(n),
            Reply::Message(text) if text.contains("to unlock") => self.locked = true,
            Reply::Setting(n, value) if self.step == Step::Settings => self.settings.set(n, &value),
            Reply::Other(text) if self.step == Step::Info && planner_blocks(&text).is_some() => {
                self.planner = planner_blocks(&text);
            }
            Reply::Ok | Reply::Error(_) if self.step == Step::Info => {
                out.extend_from_slice(b"$$\n");
                self.step = Step::Settings;
            }
            // An `ok` before any `$n=` line answers an earlier `$I` (two banners, two `$I`), not
            // `$$`: `$32` is known only from a `$$` that was really read (`specs/0142` HC7).
            Reply::Ok if self.step == Step::Settings && self.settings.is_empty() => {}
            Reply::Ok | Reply::Error(_) if self.step == Step::Settings => {
                out.push(b'?');
                self.step = Step::Report;
            }
            Reply::Report(report) if self.step == Step::Report => {
                return Outcome::Done(Box::new(Identified {
                    version: std::mem::take(&mut self.version),
                    settings: std::mem::take(&mut self.settings),
                    report,
                    alarm: self.alarm,
                    locked: self.locked,
                    planner: self.planner,
                }));
            }
            Reply::Other(text) if self.step == Step::Banner && self.got.is_none() => {
                let printable: String = text
                    .chars()
                    .filter(|c| c.is_ascii_graphic() || *c == ' ')
                    .take(GOT_CHARS)
                    .collect();
                if !printable.trim().is_empty() {
                    self.got = Some(printable);
                }
            }
            _ => {}
        }
        Outcome::Waiting
    }

    fn on_banner(&mut self, version: String, at: Duration, out: &mut Vec<u8>) -> Outcome {
        if version.starts_with("0.") {
            return Outcome::Failed(Failure::Unsupported);
        }
        if self.step != Step::Banner {
            if self.restarts >= MAX_RESTARTS {
                return Outcome::Waiting;
            }
            self.restarts += 1;
            self.settings = Settings::default();
        }
        self.version = version;
        self.deadline = at + self.limit;
        out.extend_from_slice(b"$I\n");
        self.step = Step::Info;
        Outcome::Waiting
    }

    /// Whether the limit has passed at `at`.
    #[must_use]
    pub fn on_time(&self, at: Duration) -> Outcome {
        if at >= self.deadline {
            Outcome::Failed(Failure::NoAnswer {
                got: self.got.clone(),
            })
        } else {
            Outcome::Waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grbl::reply::parse;

    const LIMIT: Duration = Duration::from_secs(5);

    fn feed(id: &mut Identify, lines: &[&str], at: Duration, out: &mut Vec<u8>) -> Outcome {
        let mut last = Outcome::Waiting;
        for line in lines {
            last = id.on_reply(parse(line), at, out);
        }
        last
    }

    #[test]
    fn reset_banner_info_settings_report() {
        let mut out = Vec::new();
        let mut id = Identify::start(Duration::ZERO, LIMIT, &mut out);
        assert_eq!(out, [RESET]);
        out.clear();
        let t = Duration::from_millis(100);
        feed(&mut id, &["Grbl 1.1h ['$' for help]"], t, &mut out);
        assert_eq!(out, b"$I\n");
        out.clear();
        feed(
            &mut id,
            &["[VER:1.1h.20190825:]", "[OPT:V,15,128]", "ok"],
            t,
            &mut out,
        );
        assert_eq!(out, b"$$\n");
        out.clear();
        feed(&mut id, &["$30=1000", "$32=1", "ok"], t, &mut out);
        assert_eq!(out, b"?");
        let done = feed(
            &mut id,
            &["<Idle|MPos:0.000,0.000,0.000|FS:0,0>"],
            t,
            &mut out,
        );
        let Outcome::Done(found) = done else {
            panic!("{done:?}")
        };
        assert_eq!(found.version, "1.1h");
        assert_eq!(found.settings.s_max(), Some(1000));
        assert_eq!(found.report.state, "Idle");
        assert!(!found.locked);
        assert_eq!(found.planner, Some(15));
    }

    #[test]
    fn the_planner_size_is_read_from_opt() {
        assert_eq!(planner_blocks("[OPT:V,15,128]"), Some(15));
        assert_eq!(planner_blocks("[OPT:VL,35,254]"), Some(35));
        assert_eq!(planner_blocks("[OPT:V,0,128]"), None);
        assert_eq!(planner_blocks("[OPT:V]"), None);
        assert_eq!(planner_blocks("[VER:1.1h.20190825:]"), None);
    }

    #[test]
    fn silence_or_other_text_fails_after_the_limit() {
        let mut out = Vec::new();
        let mut id = Identify::start(Duration::ZERO, LIMIT, &mut out);
        feed(
            &mut id,
            &["AT+CGMI", "\u{1}\u{2}"],
            Duration::from_secs(1),
            &mut out,
        );
        assert_eq!(id.on_time(Duration::from_millis(4999)), Outcome::Waiting);
        assert_eq!(
            id.on_time(LIMIT),
            Outcome::Failed(Failure::NoAnswer {
                got: Some("AT+CGMI".into())
            })
        );
        let silent = Identify::start(Duration::ZERO, LIMIT, &mut out);
        assert_eq!(
            silent.on_time(LIMIT),
            Outcome::Failed(Failure::NoAnswer { got: None })
        );
    }

    #[test]
    fn grbl_0_9_is_refused() {
        let mut out = Vec::new();
        let mut id = Identify::start(Duration::ZERO, LIMIT, &mut out);
        assert_eq!(
            feed(
                &mut id,
                &["Grbl 0.9j ['$' for help]"],
                Duration::ZERO,
                &mut out
            ),
            Outcome::Failed(Failure::Unsupported)
        );
    }

    #[test]
    fn a_second_banner_restarts_and_the_lock_and_alarm_are_kept() {
        let mut out = Vec::new();
        let mut id = Identify::start(Duration::ZERO, LIMIT, &mut out);
        let t = Duration::ZERO;
        feed(
            &mut id,
            &[
                "ALARM:3",
                "Grbl 1.1h ['$' for help]",
                "Grbl 1.1h ['$' for help]",
            ],
            t,
            &mut out,
        );
        out.clear();
        // Both `$I` are answered: the second `ok` is not the answer to `$$`.
        feed(
            &mut id,
            &["[MSG:'$H'|'$X' to unlock]", "ok", "ok"],
            t,
            &mut out,
        );
        assert_eq!(out, b"$$\n");
        out.clear();
        feed(&mut id, &["$32=1", "ok"], t, &mut out);
        assert_eq!(out, b"?");
        let done = feed(&mut id, &["<Alarm|MPos:0.000,0.000,0.000>"], t, &mut out);
        let Outcome::Done(found) = done else {
            panic!("{done:?}")
        };
        assert!(found.locked);
        assert_eq!(found.alarm, Some(3));
        assert_eq!(found.settings.laser_mode(), Some(true));
    }

    /// After a loss the banner of the reset the connector gave up with can still be in the port:
    /// a stale banner, then the identify reset's banner, then both `$I` answers (`specs/0142`).
    #[test]
    fn a_stale_banner_before_the_reset_banner_still_reads_the_settings() {
        let mut out = Vec::new();
        let mut id = Identify::start(Duration::ZERO, LIMIT, &mut out);
        let t = Duration::ZERO;
        let info = ["[VER:1.1h.20190825:]", "[OPT:V,15,128]", "ok"];
        feed(
            &mut id,
            &["Grbl 1.1h ['$' for help]", "ALARM:6"],
            t,
            &mut out,
        );
        feed(&mut id, &["Grbl 1.1h ['$' for help]"], t, &mut out);
        feed(&mut id, &info, t, &mut out);
        feed(&mut id, &info, t, &mut out);
        feed(
            &mut id,
            &["$30=1000", "$32=1", "$130=400.000", "ok"],
            t,
            &mut out,
        );
        assert_eq!(out, [&[RESET][..], b"$I\n$I\n$$\n?"].concat());
        let done = feed(&mut id, &["<Alarm|MPos:0.000,0.000,0.000>"], t, &mut out);
        let Outcome::Done(found) = done else {
            panic!("{done:?}")
        };
        assert_eq!(found.settings.laser_mode(), Some(true));
        assert_eq!(found.settings.s_max(), Some(1000));
    }

    /// A banner while `$$` is read restarts; the `ok` of the old `$$` answers the new `$I`, the
    /// `ok` of the new `$I` (no setting yet) answers nothing.
    #[test]
    fn a_banner_while_the_settings_are_read_reads_them_again() {
        let mut out = Vec::new();
        let mut id = Identify::start(Duration::ZERO, LIMIT, &mut out);
        let t = Duration::ZERO;
        feed(
            &mut id,
            &["Grbl 1.1h ['$' for help]", "ok", "$30=1000"],
            t,
            &mut out,
        );
        feed(&mut id, &["Grbl 1.1h ['$' for help]", "ok"], t, &mut out);
        feed(&mut id, &["[VER:1.1h.20190825:]", "ok"], t, &mut out);
        feed(&mut id, &["$30=1000", "$32=0", "ok"], t, &mut out);
        let done = feed(&mut id, &["<Idle|MPos:0.000,0.000,0.000>"], t, &mut out);
        let Outcome::Done(found) = done else {
            panic!("{done:?}")
        };
        assert_eq!(found.settings.laser_mode(), Some(false));
    }

    /// An `ok` with no setting before it never ends the settings step: `$32` stays unknown only
    /// when `$$` really answered without it.
    #[test]
    fn an_ok_without_settings_does_not_end_the_settings_step() {
        let mut out = Vec::new();
        let mut id = Identify::start(Duration::ZERO, LIMIT, &mut out);
        let t = Duration::ZERO;
        feed(&mut id, &["Grbl 1.1h ['$' for help]", "ok"], t, &mut out);
        out.clear();
        feed(&mut id, &["ok", "ok"], t, &mut out);
        assert!(out.is_empty(), "{out:?}");
        assert_eq!(
            feed(&mut id, &["<Idle|MPos:0.000,0.000,0.000>"], t, &mut out),
            Outcome::Waiting
        );
        assert!(matches!(id.on_time(LIMIT), Outcome::Failed(_)));
        feed(&mut id, &["$30=1000", "ok"], t, &mut out);
        let done = feed(&mut id, &["<Idle|MPos:0.000,0.000,0.000>"], t, &mut out);
        let Outcome::Done(found) = done else {
            panic!("{done:?}")
        };
        assert_eq!(found.settings.laser_mode(), None);
    }
}
