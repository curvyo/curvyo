//! The GRBL bytes of a frame (`specs/0118` criteria 24, 25; adrs F1): the laser is switched off,
//! then `G1 ... S0` moves trace the outline and return to its first point. Never `G0` (it
//! ignores the feed), never `$H`, never an `S` above 0.

use std::fmt::Write as _;

use crate::job_frame::{FrameShape, outline};
use crate::job_input::PointUm;
use crate::job_moves::Job;
use crate::job_write_grbl::coordinate;

/// The frame feed in mm/min before the machine's maximum caps it (criterion 25, Proposal).
pub const FRAME_FEED: u32 = 3_000;

/// The GRBL G-code of the frame of `job`, ASCII with LF line ends; `None` when no move burns.
/// Head position moves relative from the job's home and returns to it.
#[must_use]
pub fn write_frame_grbl(job: &Job, shape: FrameShape) -> Option<Vec<u8>> {
    let ring = outline(job, shape)?;
    let relative = job.start.relative();
    let feed = FRAME_FEED.min(job.machine.max_feed.get());
    let home = job.machine_point(job.home);
    let mut text = String::with_capacity(64 * ring.len() + 256);
    let _ = writeln!(text, "; Curvyo frame: {}", ascii(&job.name));
    let _ = writeln!(text, "; Machine: {}", ascii(&job.machine.name));
    for line in [
        "G21",
        if relative { "G91" } else { "G90" },
        "G17",
        "M5",
        "M9",
    ] {
        text.push_str(line);
        text.push('\n');
    }
    let mut at = home;
    let first = ring[0];
    let path = ring.iter().copied().chain([first]);
    let back = relative.then_some(home);
    for to in path.chain(back) {
        let shown = if relative {
            PointUm::new(to.x - at.x, to.y - at.y)
        } else {
            to
        };
        at = to;
        let _ = writeln!(
            text,
            "G1 X{} Y{} F{feed} S0",
            coordinate(shown.x),
            coordinate(shown.y)
        );
    }
    Some(text.into_bytes())
}

/// `text` as a comment: printable ASCII only, at most 60 characters.
fn ascii(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect()
}
