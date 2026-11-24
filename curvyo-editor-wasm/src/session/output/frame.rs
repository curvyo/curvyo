//! The frame of the active laser page (`specs/0118` slice 2b, adrs F2, F7): the Frame button's
//! view, the frame text for the connector, and the head ring on the canvas while it runs.

use curvyo_document_core::{Point, ViewTransform};
use curvyo_laser_core::job::{
    Cause, FrameShape, Job, PointUm, Severity, Start, build_frame_job, mm_of_um, outside_the_bed,
    um_of_mm, write_frame,
};
use curvyo_render_core::{DrawList, head_ring};
use serde::Serialize;

use super::super::Session;
use super::job_cache::PageJob;

/// The Frame button's state on the page: whether a press frames, and the page's cause if not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrameView {
    /// Whether the page has a job to frame.
    pub enabled: bool,
    /// "Nothing to frame" or "Outside the work area" (W31).
    pub cause: Option<&'static str>,
}

/// The job a page's frame traces when it is not the page's own job: a fixture block has no
/// material numbers, so its geometry runs at stand-in numbers (`specs/0118` Frage 335 B).
#[derive(Debug)]
pub(in crate::session) struct FrameJob {
    job: Job,
    outside: bool,
}

/// The causes the gate gives that a frame does not mind: the numbers a frame never burns with,
/// and the bed, measured on the frame's own job.
fn frame_ignores(cause: &Cause) -> bool {
    matches!(
        cause,
        Cause::FixtureNoMaterial | Cause::FixtureNoSet { .. } | Cause::OutsideBed { .. }
    )
}

/// The cause the page gives for a frame, `None` when it can be framed: the frame's job must have
/// a move to trace, no block of the gate may stand in its way but missing numbers, and a bed
/// overflow blocks except with Head position, which the gate does not measure (criterion 25).
fn frame_cause(job: &PageJob, frame: (&Job, bool)) -> Option<&'static str> {
    let (traced, outside) = frame;
    let blocked = job
        .gate
        .issues
        .iter()
        .any(|issue| issue.cause.severity() == Severity::Block && !frame_ignores(&issue.cause));
    if blocked || !traced.burns() {
        Some("Nothing to frame")
    } else if outside && job.input.start != Start::HeadPosition {
        Some("Outside the work area")
    } else {
        None
    }
}

/// Where the head ring stands: page millimetres of the document.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FrameHead(Point);

/// The head ring's place for the machine position `(x_mm, y_mm)` the controller reports: work
/// coordinates for an absolute frame, the offset from where the frame began for Head position.
fn head_place(traced: &Job, (x_mm, y_mm): (f64, f64), relative: bool) -> Point {
    let frame = traced.frame;
    let machine = if relative {
        let home = frame.machine_point(traced.home);
        PointUm::new(home.x + um_of_mm(x_mm), home.y + um_of_mm(y_mm))
    } else {
        PointUm::new(um_of_mm(x_mm), um_of_mm(y_mm))
    };
    let page = frame.page_point(machine);
    Point::new(mm_of_um(page.x), mm_of_um(page.y))
}

impl Session {
    /// The job the frame of `job` traces and whether it reaches outside the bed: the page's own
    /// job, or the one with stand-in numbers built once when a block has none.
    fn frame_of<'a>(&self, job: &'a PageJob) -> (&'a Job, bool) {
        if !job.built.unpriced() {
            let outside = job
                .gate
                .issues
                .iter()
                .any(|issue| matches!(issue.cause, Cause::OutsideBed { .. }));
            return (&job.built.job, outside);
        }
        let frame = job.frame.get_or_init(|| {
            let built = self
                .output
                .job
                .with_plans(|plans| build_frame_job(&job.input, self.materials().reads(), plans));
            FrameJob {
                outside: outside_the_bed(&job.input, &built),
                job: built.job,
            }
        });
        (&frame.job, frame.outside)
    }

    /// The Frame button of the output group for `job`.
    pub(super) fn frame_view(&self, job: &PageJob) -> FrameView {
        let cause = frame_cause(job, self.frame_of(job));
        FrameView {
            enabled: cause.is_none(),
            cause,
        }
    }

    /// The frame text of the active laser page for the connector, `shape` read now; `None` while
    /// the page cannot be framed. Nothing is written to the project and no history step is made.
    pub fn frame_bytes(&self, shape: FrameShape) -> Option<Vec<u8>> {
        let job = self.laser_job()?;
        let frame = self.frame_of(&job);
        frame_cause(&job, frame).is_none().then_some(())?;
        let format = job.input.machine.values.output_format;
        write_frame(format, frame.0, shape)
    }

    /// The head ring at the controller's position; whether the page has a job to place it on.
    /// `relative` for Head position: the position is the offset from where the frame began.
    pub fn set_frame_head(&mut self, x_mm: f64, y_mm: f64, relative: bool) -> bool {
        let Some(job) = self.laser_job() else {
            return false;
        };
        let place = head_place(self.frame_of(&job).0, (x_mm, y_mm), relative);
        let origin = self
            .document
            .manufacturing(job.manufacturing)
            .and_then(|m| m.page)
            .map_or(Point::new(0.0, 0.0), |page| page.origin);
        self.output.frame_head = Some(FrameHead(Point::new(
            place.x + origin.x,
            place.y + origin.y,
        )));
        true
    }

    /// The frame ended: the ring leaves. Whether one was shown.
    pub fn clear_frame_head(&mut self) -> bool {
        self.output.frame_head.take().is_some()
    }

    /// Whether the head ring shows (the play bar's own head hides, Play pauses; W34).
    pub(in crate::session) fn frame_head_shown(&self) -> bool {
        self.output.frame_head.is_some()
    }

    /// The head ring's draw list in `view`, `None` while no frame runs.
    pub(in crate::session) fn frame_head_overlay(&self, view: ViewTransform) -> Option<DrawList> {
        self.output
            .frame_head
            .map(|FrameHead(at)| head_ring(at, view))
    }
}

/// The two frame shapes' spellings of the host: `rectangle` and `rubber_band`.
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn shape_of(spelling: &str) -> FrameShape {
    if spelling == "rubber_band" {
        FrameShape::RubberBand
    } else {
        FrameShape::Rectangle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_spells_the_two_shapes() {
        assert_eq!(shape_of("rectangle"), FrameShape::Rectangle);
        assert_eq!(shape_of("rubber_band"), FrameShape::RubberBand);
        assert_eq!(shape_of("anything"), FrameShape::Rectangle);
    }
}
