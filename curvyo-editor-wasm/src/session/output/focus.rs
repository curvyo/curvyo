//! The Focus point of the active laser page (`specs/0118` slice 2d, adrs P1, P4, P7; ux-notes W55,
//! W59): a canvas press as a target in job file coordinates, the target ring at the connector's
//! clamped target, and the pulse the page's profile copy asks for.

use curvyo_document_core::Point;
use curvyo_laser_core::job::{Job, PointUm, Start, mm_of_um, um_of_mm};
use serde::Serialize;

use super::super::Session;
use super::frame::FrameHead;
use super::job_cache::PageJob;

/// The page's cause of a greyed Focus point toggle: Head position has no fixed point.
pub const HEAD_POSITION: &str = "Start is Head position";
/// The page's cause with User origin and nothing to place.
pub const NOTHING_TO_PLACE: &str = "Nothing to place";

/// Where a press sends the head, or why it cannot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FocusTarget {
    /// The target in job file coordinates, mm, rounded to µm.
    Point {
        /// X.
        x: f64,
        /// Y.
        y: f64,
    },
    /// The page's cause (W55).
    Cause {
        /// "Start is Head position" or "Nothing to place".
        cause: &'static str,
    },
}

/// The pulse a press asks for: only with Focus pulse and Laser mode on in the copy (adrs P7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FocusPulseView {
    /// Per mille of the max power.
    pub power_permille: u16,
    /// Milliseconds.
    pub length_ms: u16,
}

impl Session {
    /// The job whose mapping a press uses: the page's job for Machine origin and Home, then
    /// machine origin (the bed mirror alone), its frame job for User origin (placed by its
    /// shift); the page's cause for Head position or User origin with nothing to place.
    fn focus_job<'a>(&self, job: &'a PageJob) -> Result<&'a Job, &'static str> {
        match job.input.start {
            Start::HeadPosition => Err(HEAD_POSITION),
            Start::UserOrigin => {
                let traced = self.frame_of(job).0;
                if traced.burns() {
                    Ok(traced)
                } else {
                    Err(NOTHING_TO_PLACE)
                }
            }
            Start::MachineOrigin | Start::HomeThenOrigin => Ok(&job.built.job),
        }
    }

    /// The page origin of the active laser page in document mm.
    fn focus_page_origin(&self, job: &PageJob) -> Point {
        self.document
            .manufacturing(job.manufacturing)
            .and_then(|m| m.page)
            .map_or(Point::new(0.0, 0.0), |page| page.origin)
    }

    /// The go to point target of the document point `(x_mm, y_mm)` on the active laser page: the
    /// job file coordinates a `G90` line writes for that page point as the Start pick places it.
    /// `None` off a laser page. No clamp: the connector clamps to the travel (adrs P4).
    #[must_use]
    pub fn focus_target(&self, x_mm: f64, y_mm: f64) -> Option<FocusTarget> {
        let job = self.laser_job()?;
        let mapped = match self.focus_job(&job) {
            Ok(mapped) => mapped,
            Err(cause) => return Some(FocusTarget::Cause { cause }),
        };
        let origin = self.focus_page_origin(&job);
        let page = PointUm::new(um_of_mm(x_mm - origin.x), um_of_mm(y_mm - origin.y));
        let machine = mapped.machine_point(page);
        Some(FocusTarget::Point {
            x: mm_of_um(machine.x),
            y: mm_of_um(machine.y),
        })
    }

    /// The target ring at the job file point `(x_mm, y_mm)` (the connector's `Status.target`),
    /// placed where the head goes on the bed (the bed mirror's inverse, so with User origin at the
    /// shifted place, as the frame's head ring, W49, W59); whether the page has a mapping.
    pub fn set_focus_ring(&mut self, x_mm: f64, y_mm: f64) -> bool {
        let Some(job) = self.laser_job() else {
            return false;
        };
        let Ok(mapped) = self.focus_job(&job) else {
            return false;
        };
        let page = mapped
            .frame
            .page_point(PointUm::new(um_of_mm(x_mm), um_of_mm(y_mm)));
        let origin = self.focus_page_origin(&job);
        self.output.focus_ring = Some(FrameHead(Point::new(
            mm_of_um(page.x) + origin.x,
            mm_of_um(page.y) + origin.y,
        )));
        true
    }

    /// The target ring leaves (the next press, the mode's end). Whether one was shown.
    pub fn clear_focus_ring(&mut self) -> bool {
        self.output.focus_ring.take().is_some()
    }

    /// The pulse of the page's profile copy, `None` while Focus pulse or Laser mode is off.
    #[must_use]
    pub fn focus_pulse(&self) -> Option<FocusPulseView> {
        let copy = self.active_copy()?.read.copy;
        let pulse = copy.values.focus_pulse;
        (pulse.on && copy.values.laser_mode).then_some(FocusPulseView {
            power_permille: pulse.power.get(),
            length_ms: pulse.length.get(),
        })
    }
}
