//! GRBL 1.1 over serial (`specs/0127-connector-grbl-serial`; addendum C3): the pure parts of the
//! streamer. Nothing here touches a port, a clock or a thread; `serial_link` does the I/O.

pub mod controller;
pub mod flow;
pub mod focus;
mod frame;
mod hold;
pub mod home;
mod homing;
pub mod identify;
pub mod jog;
pub mod prepare;
mod replies;
pub mod reply;
pub mod settings;
pub mod stop;
pub mod stream;
pub mod texts;

use std::time::Duration;

pub use controller::GrblController;

/// The longest a `$H` may go unanswered before the controller counts as lost, unless
/// `--homing-timeout` sets another (`specs/0142` HC2).
pub const HOMING_CAP: Duration = Duration::from_secs(120);

/// The connector's GRBL limits; the tests shorten them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// Between two tries to open and identify (G3).
    pub retry: Duration,
    /// For the banner after the reset (G2).
    pub identify: Duration,
    /// No answer to `?` for this long is a lost controller (G15).
    pub loss: Duration,
    /// For `Hold:0` after `!` (G12, G29).
    pub hold: Duration,
    /// For the banner after the stop's reset (G12).
    pub banner: Duration,
    /// Between two `?` while running, paused or in alarm (G4).
    pub poll_fast: Duration,
    /// Between two `?` otherwise (G4).
    pub poll_slow: Duration,
    /// No answer to `$H` for this long is a lost controller (`specs/0142`).
    pub homing: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            retry: Duration::from_secs(2),
            identify: Duration::from_secs(5),
            loss: Duration::from_secs(3),
            hold: Duration::from_secs(2),
            banner: Duration::from_secs(3),
            poll_fast: Duration::from_millis(200),
            poll_slow: Duration::from_secs(1),
            homing: HOMING_CAP,
        }
    }
}
