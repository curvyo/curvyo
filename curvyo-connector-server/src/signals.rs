//! SIGINT and SIGTERM as a wake-up of the main thread (`specs/0064-machine-connector/adrs.md` C11).
//!
//! A handler is installed process-wide, so whichever thread the kernel picks, the signal only
//! reaches the main thread's `wait`. Blocking the signals and `sigwait` is not enough: `serialport`
//! waits in `ppoll` with an empty mask, which unblocks them in the serial thread, and the default
//! action then killed the process without the safe stop of G16.

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

/// Installs the handlers; call it before any thread starts.
///
/// # Errors
/// The operating system's refusal.
pub fn install() -> std::io::Result<Signals> {
    Signals::new([SIGINT, SIGTERM])
}

/// Waits for SIGINT or SIGTERM.
pub fn wait(signals: &mut Signals) {
    let _ = signals.forever().next();
}
