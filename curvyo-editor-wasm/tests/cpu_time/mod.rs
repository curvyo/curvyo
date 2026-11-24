//! The CPU time a piece of work costs the calling thread, for budget tests that must not fail on
//! a busy machine: wall time counts every moment another process holds the core, thread CPU
//! time (`CLOCK_THREAD_CPUTIME_ID`) leaves that preemption out. Off Unix it falls back to wall
//! time; the budget jobs run on Linux.

#![allow(dead_code, clippy::unwrap_used)]

use std::time::Duration;

#[path = "../budget_factor/mod.rs"]
pub mod budget_factor;

/// The CPU time the calling thread has used so far.
#[cfg(unix)]
fn thread_time() -> Duration {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    Duration::new(
        u64::try_from(now.tv_sec).unwrap(),
        u32::try_from(now.tv_nsec).unwrap(),
    )
}

/// The result of `work` and the CPU time it cost this thread (wall time off Unix), divided by
/// `CURVYO_BUDGET_FACTOR` (see [`budget_factor`]).
pub fn cpu_time_of<T>(work: impl FnOnce() -> T) -> (T, Duration) {
    #[cfg(unix)]
    {
        let start = thread_time();
        let result = work();
        (
            result,
            budget_factor::scaled(thread_time().saturating_sub(start)),
        )
    }
    #[cfg(not(unix))]
    {
        let start = std::time::Instant::now();
        let result = work();
        (result, budget_factor::scaled(start.elapsed()))
    }
}
