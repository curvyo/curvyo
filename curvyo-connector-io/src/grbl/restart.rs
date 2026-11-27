//! A restart from a line for GRBL (`specs/0118` 2e, adrs R8, R9): the checks before the first
//! byte (`$32` read again, the travel), the entry lines, then the job from its line, the arrival.

use std::time::Duration;

use curvyo_connector_core::{EDGE, Event, Job, Millimetres};

use crate::grbl::controller::{BUSY_WORDS, GrblController};
use crate::grbl::frame::outside_travel;
use crate::grbl::home::Owed;
use crate::grbl::prepare::prepare;
use crate::grbl::settings::Settings;
use crate::grbl::stream::Stream;
use crate::grbl::texts;

/// GRBL's settings report: the restart reads `$32` again with it.
pub const SETTINGS_LINE: &[u8] = b"$$\n";

/// The head stands at the travel's target when the work position is this close (mm).
pub const ARRIVED: f64 = 0.05;

/// What `ControllerCommand::StartAt` carries.
pub(super) struct StartAt<'a> {
    /// The first job line.
    pub(super) from_line: u32,
    /// The lines before it.
    pub(super) entry: &'a [String],
    /// The work X and Y the travel ends at.
    pub(super) target: [Millimetres; 2],
    /// Homed since the controller was identified.
    pub(super) homed: bool,
}

/// A restart that waits for the answer to its `$$`: `$32` may have changed on the controller
/// since the connector identified it (its own panel, another program), so the laser mode is read
/// again right before the first byte of the entry (`specs/0118` 38).
#[derive(Debug, Clone)]
pub(super) struct Recheck {
    job: Job,
    from_line: u32,
    entry: Vec<String>,
    target: [Millimetres; 2],
    homed: bool,
    /// The `$<n>=` lines read so far.
    fresh: Settings,
}

impl Recheck {
    /// Takes one `$<n>=<value>` line of the answer.
    pub(super) fn set(&mut self, number: u16, value: &str) {
        self.fresh.set(number, value);
    }

    /// The line the restart begins at.
    pub(super) fn first_line(&self) -> u32 {
        self.from_line
    }
}

impl GrblController {
    fn refuse_start(&mut self, detail: String) {
        self.events.push(Event::JobFailedWith {
            elapsed_ms: 0,
            detail,
        });
    }

    /// The largest distance by which the travel's target misses the travel, when the machine is
    /// homed and its position known.
    fn target_misses(&self, start: &StartAt<'_>) -> Option<f64> {
        let within = self.travel_box()?;
        if !start.homed || !self.wco_seen {
            return None;
        }
        let worst = (0..2)
            .map(|i| {
                let machine = start.target[i].0 + self.wco[i];
                (within.min[i] - machine).max(machine - within.max[i])
            })
            .fold(0.0_f64, f64::max);
        (worst > EDGE).then_some(worst)
    }

    /// Why the restart cannot go out now, by the settings as last read; `None` when it can.
    fn start_refusal(&self, start: &StartAt<'_>) -> Option<String> {
        match (
            self.guarded_laser_mode(&self.settings),
            self.word.as_deref(),
        ) {
            (None, _) => Some(texts::LASER_MODE_UNKNOWN.to_owned()),
            (Some(false), _) => Some(texts::LASER_MODE_OFF.to_owned()),
            (_, Some(word)) if BUSY_WORDS.contains(&word) => {
                Some(format!("Controller state: {word}"))
            }
            _ => self.target_misses(start).map(outside_travel),
        }
    }

    /// The prepared entry lines and the job's lines from `from_line`, or the sentence of the
    /// first line `prepare` (G5) refuses.
    fn restart_lines(job: &Job, start: &StartAt<'_>) -> Result<(Vec<String>, Vec<String>), String> {
        let entry = prepare(&start.entry.join("\n"))
            .map_err(|refused| texts::failed_at(start.from_line, refused.cause.text()))?;
        let lines = prepare(&job.text)
            .map_err(|refused| texts::failed_at(refused.line, refused.cause.text()))?;
        let skip = usize::try_from(start.from_line.saturating_sub(1)).unwrap_or(0);
        Ok((entry, lines.into_iter().skip(skip).collect()))
    }

    /// `ControllerCommand::StartAt`: every check that needs no answer, then `$$` to read `$32`
    /// again; the entry and the job follow its answer. Every refusal ends the restart through
    /// `JobFailedWith` before the first byte of the entry.
    pub(super) fn start_at(&mut self, job: Option<&Job>, start: &StartAt<'_>) {
        let Some(job) = job else {
            return;
        };
        if self.stop.is_some() || self.recheck.is_some() {
            return;
        }
        let refusal = self
            .start_refusal(start)
            .or_else(|| Self::restart_lines(job, start).err());
        if let Some(detail) = refusal {
            return self.refuse_start(detail);
        }
        self.out.extend_from_slice(SETTINGS_LINE);
        self.owed.push_back(Owed::Settings);
        self.recheck = Some(Recheck {
            job: job.clone(),
            from_line: start.from_line,
            entry: start.entry.to_vec(),
            target: start.target,
            homed: start.homed,
            fresh: Settings::default(),
        });
    }

    /// GRBL answered the restart's `$$`: the settings read replace the cached ones, then the
    /// checks run again and the entry goes out, or the restart is refused.
    pub(super) fn on_settings_answer(&mut self, code: Option<u16>, at: Duration) {
        let Some(recheck) = self.recheck.take() else {
            return;
        };
        if code.is_some() || recheck.fresh.is_empty() {
            // `$32` was not read again: the restart does not go out on the old value.
            return self.refuse_start(texts::LASER_MODE_UNKNOWN.to_owned());
        }
        self.take_settings(recheck.fresh.clone());
        let start = StartAt {
            from_line: recheck.from_line,
            entry: &recheck.entry,
            target: recheck.target,
            homed: recheck.homed,
        };
        let lines = match self.start_refusal(&start) {
            Some(detail) => Err(detail),
            None => Self::restart_lines(&recheck.job, &start),
        };
        match lines {
            Err(detail) => self.refuse_start(detail),
            Ok((entry, rest)) => {
                self.entering = Some([start.target[0].0, start.target[1].0]);
                self.begin(Stream::new_at(entry, rest, start.from_line, at));
            }
        }
    }

    /// Keeps the settings of a `$$` read after identify, and tells the job machine when the
    /// laser mode changed (G34: no job starts while it is off or unknown).
    fn take_settings(&mut self, fresh: Settings) {
        let (before, now) = (
            self.guarded_laser_mode(&self.settings),
            self.guarded_laser_mode(&fresh),
        );
        self.settings = fresh;
        if let Some(info) = self.info.as_mut() {
            info.settings = Some(self.settings.to_wire());
        }
        if before != now {
            let notice = match now {
                Some(false) => Some(texts::LASER_MODE_OFF),
                None => Some(texts::LASER_MODE_UNKNOWN),
                Some(true) => None,
            };
            self.notices.extend(notice.map(str::to_owned));
            self.events.push(Event::LaserModeOff(now == Some(false)));
            self.events.push(Event::LaserModeUnknown(now.is_none()));
        }
    }

    /// The restart reached its line: every entry line answered and a report puts the head at
    /// the target, or every line was answered. GRBL's `ok` to a job line after the entry means
    /// "in the planner", not "the head is there", so it proves nothing (Proposal, amends R9).
    pub(super) fn note_arrival(&mut self) {
        let Some(target) = self.entering else {
            return;
        };
        let Some(stream) = self.stream.as_ref() else {
            self.entering = None;
            return;
        };
        let there = self.position.is_some_and(|p| {
            (p[0] - target[0]).abs() <= ARRIVED && (p[1] - target[1]).abs() <= ARRIVED
        });
        if stream.all_answered() || (stream.entry_answered() && there) {
            self.entering = None;
            self.events.push(Event::EntryReached);
        }
    }
}
