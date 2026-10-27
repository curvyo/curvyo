//! The CPU time of the calling thread, so a time bound holds on a busy machine (`CLAUDE.md` §5).

use std::time::Duration;

/// The calling thread's CPU time since it started (Linux `schedstat`; wall time since the first
/// call elsewhere).
pub fn thread_cpu() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse::<u64>().ok())
        .map_or_else(
            || {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                START.get_or_init(std::time::Instant::now).elapsed()
            },
            Duration::from_nanos,
        )
}
