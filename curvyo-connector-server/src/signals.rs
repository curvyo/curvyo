//! SIGINT and SIGTERM without a handler (`specs/0064-machine-connector/adrs.md` C11): blocked
//! before any thread starts, then awaited by the main thread with `sigwait`.

use nix::sys::signal::{SigSet, Signal};

/// The signals the connector stops a job for.
#[must_use]
pub fn set() -> SigSet {
    let mut set = SigSet::empty();
    set.add(Signal::SIGINT);
    set.add(Signal::SIGTERM);
    set
}

/// Blocks [`set`] for this thread and every thread it starts afterwards.
///
/// # Errors
/// The operating system's refusal.
pub fn block() -> nix::Result<SigSet> {
    let set = set();
    set.thread_block()?;
    Ok(set)
}

/// Waits for one of the blocked signals.
///
/// # Errors
/// The operating system's refusal.
pub fn wait(set: &SigSet) -> nix::Result<Signal> {
    set.wait()
}
