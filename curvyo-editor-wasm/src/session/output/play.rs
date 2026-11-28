//! The play bar of the Toolpath view (`specs/0029` criterion 37; ADR 0064 §10; UX notes §5, P9,
//! P10, P12): the slider's position as a fraction of the job's time, Play over 10 s, the step
//! keys and the readout. Session state, never a step.

use serde::Serialize;

use curvyo_laser_core::job::readout_text;

use super::super::Session;

/// Play runs the whole job in this many milliseconds, whatever its length (Proposal).
pub const PLAY_WHOLE_MS: f64 = 10_000.0;

/// The slider and Play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlayState {
    /// The position, 0 to 1 of the job's time; the end by default (UX P10).
    pub(super) fraction: f64,
    /// While playing: the clock of the last tick and the job generation it plays.
    pub(super) playing: Option<(f64, u64)>,
}

impl Default for PlayState {
    fn default() -> Self {
        Self {
            fraction: 1.0,
            playing: None,
        }
    }
}

/// The play bar as the frontend draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlayView {
    /// The slider's position, 0 to 1.
    pub fraction: f64,
    /// Whether Play is running (the button shows Pause).
    pub playing: bool,
    /// "6 min 6 s of 9 min 51 s" (time only, criterion 37); the slider's `aria-valuetext` too.
    pub readout: String,
    /// Whether the controls work: something runs (UX P20).
    pub enabled: bool,
    /// The fraction one arrow press moves: one stitch on an embroidery page (ADR 0098 decision
    /// 3); absent elsewhere, where the arrows step by percent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// The thread of the colour block at the head, "#RRGGBB" (embroidery); absent elsewhere.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
}

impl Session {
    /// The play bar of the active laser page while its Toolpath view is pressed.
    #[must_use]
    pub fn play_view(&self) -> Option<PlayView> {
        if !self.toolpath_view_on() {
            return None;
        }
        if let Some(family) = self.active_family() {
            return self.family_play_view(&family);
        }
        let job = self.laser_job()?;
        let fraction = self.play_fraction();
        let head = job.timeline.head_at(&job.built.job, fraction);
        Some(PlayView {
            fraction,
            playing: self.output.play.playing.is_some(),
            readout: readout_text(&head, &job.timeline),
            enabled: !job.built.job.moves.is_empty(),
            step: None,
            colour: None,
        })
    }

    /// The position the view draws: Play stops by itself when the job changed (UX P12).
    pub(super) fn play_fraction(&self) -> f64 {
        self.output.play.fraction
    }

    /// Moves the slider to `fraction` of the job's time (0 to 1); Play keeps playing from there.
    pub fn set_play_position(&mut self, fraction: f64) {
        let fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.output.play.fraction = fraction;
        if let Some(family) = self.active_family() {
            self.set_family_play(&family, fraction);
        }
    }

    /// The arrow and page keys: steps the slider by `percent` of the run (UX notes §5).
    pub fn step_play_position(&mut self, percent: f64) {
        self.set_play_position(self.output.play.fraction + percent / 100.0);
    }

    /// Play or Pause at the clock `now_ms`: Play starts from the slider, or from 0 at the end
    /// (criterion 37). Whether it is playing now.
    pub fn toggle_play(&mut self, now_ms: f64) -> bool {
        if self.output.play.playing.take().is_some() || self.frame_head_shown() {
            // One ring means one head: Play pauses while the frame's ring shows (W34).
            return false;
        }
        let (generation, empty) = match self.active_family() {
            Some(family) => {
                let view = self.family_play_view(&family);
                if let Some(view) = &view {
                    self.output.play.fraction = view.fraction;
                }
                (0, view.is_none_or(|v| !v.enabled))
            }
            None => match self.laser_job() {
                Some(job) => (job.generation, job.built.job.moves.is_empty()),
                None => return false,
            },
        };
        if empty {
            return false;
        }
        let play = &mut self.output.play;
        if play.fraction >= 1.0 {
            play.fraction = 0.0;
        }
        play.playing = Some((now_ms, generation));
        true
    }

    /// One animation frame at the clock `now_ms` (the frontend's `requestAnimationFrame`):
    /// advances the head, stops at the end and when the job changed (UX P12). Whether it is
    /// still playing.
    pub fn play_tick(&mut self, now_ms: f64) -> bool {
        let Some((last, generation)) = self.output.play.playing else {
            return false;
        };
        let family = self.active_family();
        let current = match &family {
            Some(_) => Some(0),
            None => self.laser_job().map(|job| job.generation),
        };
        let shown = self.toolpath_view_on();
        let play = &mut self.output.play;
        if current != Some(generation) || !shown {
            play.playing = None;
            return false;
        }
        let elapsed = (now_ms - last).max(0.0);
        play.fraction = (play.fraction + elapsed / PLAY_WHOLE_MS).min(1.0);
        let fraction = play.fraction;
        let done = fraction >= 1.0;
        play.playing = (!done).then_some((now_ms, generation));
        if let Some(family) = family {
            self.set_family_play(&family, fraction);
        }
        !done
    }
}
