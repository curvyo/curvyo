//! The events a job machine takes and the commands it gives the controller side.

use std::time::Duration;

use uuid::Uuid;

use super::Peer;
use crate::focus::{PulseLength, PulsePower};
use crate::jog::{Feed, JogAxis, JogDirection, TravelBox};
use crate::message::Message;
use crate::millimetres::Millimetres;

/// Something that happened: a message from the app or an event from the controller side.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // one value per event, never stored in bulk
pub enum Event {
    /// A message from a paired, greeted client, or from the host itself.
    Message(Message, Peer),
    /// A controller was attached.
    ControllerConnected,
    /// The controller was lost; a job in flight is dropped and named in the detail.
    ControllerLost,
    /// The controller was lost for the reason in `detail`, which the app shows in place of the
    /// line count; a job in flight is dropped (`specs/0142`, the homing cap).
    ControllerLostWith {
        /// The sentence ("Lost the controller: homing took longer than 120 s").
        detail: String,
    },
    /// The controller got through `line` lines; `elapsed_ms` is counted by the caller.
    Progress {
        /// Lines done.
        line: u32,
        /// Milliseconds since the start.
        elapsed_ms: u64,
    },
    /// The controller reports its position.
    Position([Millimetres; 3]),
    /// The controller ran every line.
    JobFinished {
        /// Milliseconds since the start.
        elapsed_ms: u64,
    },
    /// The controller reported an error while running.
    JobFailed {
        /// Milliseconds since the start.
        elapsed_ms: u64,
    },
    /// The controller failed the job, running or paused, for the reason in `detail`.
    JobFailedWith {
        /// Milliseconds since the start.
        elapsed_ms: u64,
        /// The sentence the app shows ("Failed at line 12: No feed rate (error 22)").
        detail: String,
    },
    /// The controller raised an alarm.
    Alarm {
        /// Its text.
        text: String,
    },
    /// The alarm was cleared.
    AlarmCleared,
    /// The button on the connector's host was pressed.
    LocalStartPressed,
    /// A client connection that sent a job ended; a job still being received is dropped
    /// (back to `Idle`), a job that is `Ready` or further is not affected.
    ClientDisconnected,
    /// The connection that sent the job `job_id` ended: that job's transfer is dropped if it is
    /// still arriving, and no other (`specs/0125` S19, `specs/0132` L2).
    TransferAbandoned {
        /// The job.
        job_id: Uuid,
    },
    /// The chain's decision when a job ended (made inside the machine, carried out by the
    /// caller as the next event): the first waiting job `job_id` starts.
    RunNext {
        /// The job.
        job_id: Uuid,
    },
    /// The controller holds the job (asked for or not): `Running` becomes `Paused`.
    Held,
    /// The controller runs the held job again.
    Resumed,
    /// The controller confirms that a stop ended: the job is `Done { Stopped }`.
    Stopped,
    /// The controller could not confirm a stop: the job is dropped, `NoController`.
    StopUnconfirmed,
    /// Why there is no controller ("Port /dev/ttyUSB0 not found"); only in `NoController`.
    NoControllerDetail {
        /// The sentence.
        detail: String,
    },
    /// The controller reports a state outside the job's states ("Door"); `None` once it left it.
    ControllerWord(Option<String>),
    /// Whether the controller's laser mode is off: jobs are refused while it is (G34).
    LaserModeOff(bool),
    /// Whether the controller's laser mode could not be read: jobs are refused while so (G34).
    LaserModeUnknown(bool),
    /// The controller ended its homing cycle (GRBL's `ok` to `$H`, `specs/0118` M2).
    Homed,
    /// The controller refused the homing cycle and stays where it was (GRBL `error:<n>` to `$H`).
    HomingFailed {
        /// The sentence the app shows ("Homing is not enabled (error 5)").
        detail: String,
    },
    /// The time since the connector's start, for the jog's dead-man; the machine has no clock.
    Clock(Duration),
    /// The controller finished the jog (or its cancel): the head stands, nothing is owed.
    JogEnded,
    /// The controller refused a jog (GRBL `error:15` and the like): the head stands.
    JogFailed {
        /// The sentence the app shows ("Jog beyond the travel (error 15)").
        detail: String,
    },
    /// The controller restarted without being asked (a banner outside a stream): the position
    /// is not trusted any more, and a jog ends.
    ControllerReset,
    /// The controller reports where the head is in machine coordinates and its work offset;
    /// `None` for what the report did not carry.
    MachinePosition {
        /// The machine position.
        machine: Option<[Millimetres; 3]>,
        /// The work coordinate offset.
        offset: Option<[Millimetres; 3]>,
    },
    /// The controller's planner holds this many blocks (GRBL `$I`, `[OPT:V,15,128]`).
    Planner {
        /// The blocks.
        blocks: u32,
    },
    /// A restart from a line: the head stands at the start of its line, the job's own lines
    /// run from now on (`specs/0118` 2e, adrs R9).
    EntryReached,
    /// The controller cannot hold now (a job's `$H` runs) and holds at its first safe moment;
    /// `Running` keeps the pause pending until `Held` (`specs/0133` P3).
    PauseDeferred,
    /// The box the controller's travel allows in machine coordinates, `None` while unknown
    /// (`$130`, `$131` not reported): a go to point is clamped into it before it moves
    /// (`specs/0118` adrs P4).
    Travel(Option<TravelBox>),
    /// The go to point's target as the controller clamped it to the travel, in job file
    /// coordinates (`specs/0118` adrs P4); the move ends with `JogEnded` or `JogFailed`.
    GoToTarget {
        /// The clamped target.
        target: [Millimetres; 2],
    },
    /// The pulse block ran to its own `M5` (`specs/0118` 35 b).
    PulseEnded,
    /// The controller refused a line of the pulse block and switched the beam off.
    PulseFailed {
        /// The sentence the app shows.
        detail: String,
    },
    /// The controller wrote no pulse (laser mode off or unknown, `$30` unknown, or `S` below 1).
    PulseSkipped,
}

/// What the transport has to tell the controller.
#[derive(Debug, Clone, PartialEq)]
pub enum ControllerCommand {
    /// Stream the held job.
    Start {
        /// The job.
        job_id: Uuid,
    },
    /// Stream the held job from `from_line`: the `entry` lines first (they take the head there
    /// with the laser off and set the state of the line), then the job's lines from `from_line`;
    /// `target` is the work position the travel ends at, for the travel check.
    StartAt {
        /// The job.
        job_id: Uuid,
        /// The first job line.
        from_line: u32,
        /// The lines before it.
        entry: Vec<String>,
        /// The work X and Y the travel moves to.
        target: [Millimetres; 2],
        /// The controller has been homed since it was identified.
        homed: bool,
    },
    /// Hold the stream.
    Pause,
    /// Continue the stream.
    Resume,
    /// End the stream.
    Stop,
    /// Leave the controller's alarm lock.
    Unlock,
    /// Run the controller's homing cycle.
    Home,
    /// Trace the frame of `frame_job()` with the laser off; `homed` lets the controller check it
    /// against the travel.
    Frame {
        /// The frame.
        job_id: Uuid,
        /// The controller has been homed since it was identified.
        homed: bool,
    },
    /// Move the head one step with the laser off; `homed` lets the controller clamp it.
    Jog {
        /// The machine axis.
        axis: JogAxis,
        /// The way along it.
        direction: JogDirection,
        /// The distance.
        distance: Millimetres,
        /// The feed before the controller's own cap.
        feed: Feed,
        /// The controller has been homed since it was identified.
        homed: bool,
    },
    /// Move the head continuously until `JogCancel`, with the laser off.
    JogHold {
        /// The machine axis.
        axis: JogAxis,
        /// The way along it.
        direction: JogDirection,
        /// The feed before the controller's own cap.
        feed: Feed,
        /// The controller has been homed since it was identified.
        homed: bool,
    },
    /// End the jog now; the position stays.
    JogCancel,
    /// Move the head with the laser off to `target` (job file coordinates), already clamped to
    /// the travel by core; the controller clamps it again with the same inputs before it writes
    /// the jog and answers with `GoToTarget`, the target it moves to (`specs/0118` 2d, P4).
    GoTo {
        /// The target, clamped by core.
        target: [Millimetres; 2],
        /// The feed before the controller's own cap.
        feed: Feed,
    },
    /// Fire one low-power pulse where the head stands, the beam off again after `length`.
    Pulse {
        /// The power in per mille of the controller's full power.
        power: PulsePower,
        /// The length.
        length: PulseLength,
    },
}
