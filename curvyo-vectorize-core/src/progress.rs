//! How a trace reports where it is and learns that it should stop (ADR 0023 §2): `poll` at fixed
//! work boundaries, never on a clock.

use std::ops::ControlFlow;

use crate::TraceError;

/// The stage a trace is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The passes over the mask: labelling, specks, holes (`done` and `total` count rows).
    Labelling,
    /// The outlines (`done` and `total` count regions).
    Tracing,
}

/// Where a trace is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// The stage.
    pub phase: Phase,
    /// Units of the stage done.
    pub done: u32,
    /// Units of the stage in all.
    pub total: u32,
}

/// Rows between two calls of `poll` in the passes over the mask.
pub(crate) const ROWS_PER_POLL: usize = 64;

/// The caller's `poll`, turned into `Err(Cancelled)` on `Break`.
pub(crate) struct Poller<'p> {
    poll: &'p mut dyn FnMut(Progress) -> ControlFlow<()>,
}

impl<'p> Poller<'p> {
    pub(crate) fn new(poll: &'p mut dyn FnMut(Progress) -> ControlFlow<()>) -> Self {
        Self { poll }
    }

    /// Calls `poll`; `Err(Cancelled)` when it asks to stop.
    pub(crate) fn check(
        &mut self,
        phase: Phase,
        done: usize,
        total: usize,
    ) -> Result<(), TraceError> {
        let progress = Progress {
            phase,
            done: u32::try_from(done).unwrap_or(u32::MAX),
            total: u32::try_from(total).unwrap_or(u32::MAX),
        };
        match (self.poll)(progress) {
            ControlFlow::Continue(()) => Ok(()),
            ControlFlow::Break(()) => Err(TraceError::Cancelled),
        }
    }

    /// Calls `poll` every [`ROWS_PER_POLL`] rows of a pass over `rows` rows.
    pub(crate) fn row(&mut self, row: usize, rows: usize) -> Result<(), TraceError> {
        if row.is_multiple_of(ROWS_PER_POLL) {
            self.check(Phase::Labelling, row, rows)
        } else {
            Ok(())
        }
    }
}
