//! The controller behind the job machine, as the hub drives it (`specs/0064-machine-connector/adrs.md`
//! C2): commands in, events, bytes for the port and terminal lines out. Pure: no port, clock or
//! thread; the caller passes the bytes read and the time since the server's start.

use std::time::Duration;

use curvyo_connector_core::{ControllerCommand, ControllerInfo, Event, Job};

/// What one turn of a controller produced.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Advance {
    /// Events for the job machine, in order.
    pub events: Vec<Event>,
    /// Bytes for the port, in order.
    pub write: Vec<u8>,
    /// Lines for the connector's terminal.
    pub notices: Vec<String>,
    /// The link is to be closed and opened again; the terminal line that says why.
    pub close: Option<String>,
}

/// A controller: the fake one or GRBL over serial.
pub trait Controller: Send {
    /// What it reports about itself; `None` until identified.
    fn info(&self) -> Option<ControllerInfo>;
    /// A command of the job machine (with the held job); events that follow at once. Bytes the
    /// command needs go out with the next [`Controller::advance`].
    fn command(
        &mut self,
        command: &ControllerCommand,
        job: Option<&Job>,
        at: Duration,
    ) -> Vec<Event>;
    /// Bytes read from the link since the last call, and the time since the server's start.
    fn advance(&mut self, read: &[u8], at: Duration) -> Advance;
}
