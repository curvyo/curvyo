//! The job commands (A13, A14, A34; UX notes 4.5 to 4.8): `status`, `confirm-start`, `stop`,
//! `unlock` and `jobs`, in the owner app's words.

use curvyo_connector_core::{
    ConnectorMode, ControllerInfo, JobResult, LogName, Message, Progress, State,
};

use crate::connect::{self, Owner, client_error};
use crate::context::Context;
use crate::error::CtlError;
use crate::texts::{columns, duration, labelled, log_time, number};

/// The parts of a `Status` the commands read.
struct Seen<'a> {
    state: State,
    job: Option<&'a Progress>,
    alarm: Option<&'a str>,
    requested_by: Option<&'a str>,
    job_name: &'a str,
    sent_by: Option<&'a str>,
}

fn seen(status: &Message) -> Option<Seen<'_>> {
    match status {
        Message::Status {
            state,
            job,
            alarm,
            requested_by,
            job_name,
            sent_by,
            ..
        } => Some(Seen {
            state: *state,
            job: job.as_ref(),
            alarm: alarm.as_deref(),
            requested_by: requested_by.as_deref(),
            job_name: job_name.as_deref().unwrap_or("the job"),
            sent_by: sent_by.as_deref(),
        }),
        _ => None,
    }
}

/// The `State:` words (UX notes 4.5).
fn state_words(s: &Seen<'_>) -> String {
    let line = s.job.map_or(0, |j| j.line);
    let lines = s.job.map_or(0, |j| j.lines);
    let elapsed = s.job.map_or(0, |j| j.elapsed_ms);
    match s.state {
        State::NoController => "No controller".to_owned(),
        State::Idle => "Idle".to_owned(),
        State::Receiving => format!("Receiving {}", s.job_name),
        State::Ready => "Ready".to_owned(),
        State::AwaitingLocalStart => match s.requested_by {
            Some(member) if line > 0 => {
                format!("Resume requested by {member} at line {}", number(line))
            }
            Some(member) => format!("Start requested by {member}"),
            None => "Waiting for the start at the machine".to_owned(),
        },
        State::Homing => "Homing".to_owned(),
        State::Running => format!(
            "Running, line {} / {}, {}",
            number(line),
            number(lines),
            duration(elapsed)
        ),
        State::Paused => format!("Paused at line {}, {}", number(line), duration(elapsed)),
        State::Done {
            result: JobResult::Completed,
        } => format!("Completed, {} lines", number(lines)),
        State::Done {
            result: JobResult::Stopped,
        } => format!("Stopped at line {}", number(line)),
        State::Done {
            result: JobResult::Failed,
        } => format!("Failed at line {}", number(line)),
        State::Alarm => format!("Alarm: {}", s.alarm.unwrap_or_default()),
    }
}

fn controller_words(info: Option<&ControllerInfo>) -> String {
    match info {
        None => "No controller".to_owned(),
        Some(info) => match &info.port {
            Some(port) => format!("{} on {port}", info.name),
            None => info.name.clone(),
        },
    }
}

fn status_of(owner: &Owner) -> Result<Seen<'_>, CtlError> {
    seen(&owner.status)
        .ok_or_else(|| CtlError::refused(format!("{} sent no status", owner.entry.name)))
}

/// `status`.
///
/// # Errors
/// The failure in words.
pub fn status(ctx: &mut Context, name: Option<&str>) -> Result<String, CtlError> {
    let owner = connect::owner(ctx, name)?;
    let s = status_of(&owner)?;
    let greeting = &owner.connection.greeting;
    let connector = match &greeting.mode {
        Some(ConnectorMode::Configured { owner: Some(label) }) => {
            format!("{}, belongs to {label}", greeting.name)
        }
        _ => greeting.name.clone(),
    };
    let mut rows = vec![
        ("Connector", connector),
        ("Controller", controller_words(greeting.controller.as_ref())),
        ("State", state_words(&s)),
    ];
    if let Some(job) = s.job {
        rows.push((
            "Job",
            format!("{}, {} lines", s.job_name, number(job.lines)),
        ));
        if let Some(sender) = s.sent_by {
            rows.push(("Sent by", sender.to_owned()));
        }
    }
    Ok(labelled(&rows))
}

/// Sends `message` and waits for the state to leave `before`.
fn command(
    owner: &mut Owner,
    message: &Message,
    before: State,
) -> Result<Option<Message>, CtlError> {
    let who = owner.entry.name.clone();
    owner
        .connection
        .command(message, before)
        .map_err(|e| match e {
            curvyo_connector_io::client::ClientError::Protocol(code) => {
                CtlError::refused(format!("{who} refused: {code}"))
            }
            other => client_error(other, &who, &who),
        })
}

/// `confirm-start [<connector>] [<job>]`: runs a waiting request at once, asks nothing (V8).
///
/// # Errors
/// "no start is waiting on <name>", "<name> waits for <a>, not <b>", or the failure in words.
pub fn confirm_start(ctx: &mut Context, positionals: &[String]) -> Result<String, CtlError> {
    let known = ctx.known().list()?;
    let (connector, job) = match positionals {
        [connector, job] => (Some(connector.as_str()), Some(job.as_str())),
        [one] if known.iter().any(|k| &k.name == one) => (Some(one.as_str()), None),
        [one] => (None, Some(one.as_str())),
        _ => (None, None),
    };
    let mut owner = connect::owner(ctx, connector)?;
    let who = owner.entry.name.clone();
    let (job_id, line, requester, job_name) = {
        let s = status_of(&owner)?;
        match (s.state, s.requested_by, s.job) {
            (State::AwaitingLocalStart, Some(member), Some(progress)) => (
                progress.job_id,
                progress.line,
                member.to_owned(),
                s.job_name.to_owned(),
            ),
            _ => return Err(CtlError::refused(format!("no start is waiting on {who}"))),
        }
    };
    if let Some(wanted) = job
        && wanted != job_name
    {
        return Err(CtlError::refused(format!(
            "{who} waits for {job_name}, not {wanted}"
        )));
    }
    command(
        &mut owner,
        &Message::Start { job_id },
        State::AwaitingLocalStart,
    )?;
    Ok(if line > 0 {
        format!(
            "Resumed {job_name} at line {}, requested by {requester}\n",
            number(line)
        )
    } else {
        format!("Started {job_name}, requested by {requester}\n")
    })
}

/// `stop`: the safety command; "No job" is its goal state, exit 0 (V9).
///
/// # Errors
/// The failure in words.
pub fn stop(ctx: &mut Context, name: Option<&str>) -> Result<String, CtlError> {
    let mut owner = connect::owner(ctx, name)?;
    let (state, line, job_name) = {
        let s = status_of(&owner)?;
        (s.state, s.job.map_or(0, |j| j.line), s.job_name.to_owned())
    };
    let text = match state {
        State::Running | State::Paused => format!("Stopped {job_name} at line {}\n", number(line)),
        State::AwaitingLocalStart if line > 0 => {
            format!("Stopped {job_name} at line {}\n", number(line))
        }
        State::Receiving | State::Ready | State::AwaitingLocalStart | State::Homing => {
            format!("Discarded {job_name}\n")
        }
        _ => return Ok("No job\n".to_owned()),
    };
    command(&mut owner, &Message::Stop, state)?;
    Ok(text)
}

/// `unlock`.
///
/// # Errors
/// The failure in words.
pub fn unlock(ctx: &mut Context, name: Option<&str>) -> Result<String, CtlError> {
    let mut owner = connect::owner(ctx, name)?;
    if status_of(&owner)?.state != State::Alarm {
        return Ok("No alarm\n".to_owned());
    }
    let who = owner.entry.name.clone();
    connect::call(&mut owner.connection, &Message::Unlock, &who)?;
    Ok("Alarm cleared\n".to_owned())
}

/// The raw last lines of a log.
pub(crate) fn log_lines(
    ctx: &mut Context,
    name: Option<&str>,
    log: LogName,
    last: u32,
) -> Result<Vec<String>, CtlError> {
    let mut owner = connect::owner(ctx, name)?;
    let who = owner.entry.name.clone();
    match connect::call(&mut owner.connection, &Message::GetLog { log, last }, &who)? {
        Message::LogLines { lines } => Ok(lines),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}

/// `jobs`: time, job, sender, starter (the confirming owner when there is one), result.
///
/// # Errors
/// The failure in words.
pub fn jobs(ctx: &mut Context, name: Option<&str>, last: u32) -> Result<String, CtlError> {
    let lines = log_lines(ctx, name, LogName::Jobs, last)?;
    if lines.is_empty() {
        return Ok("No jobs\n".to_owned());
    }
    let rows: Vec<Vec<String>> = lines
        .iter()
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let field = |i: usize| f.get(i).copied().unwrap_or_default().to_owned();
            let started = if field(5).is_empty() {
                field(4)
            } else {
                field(5)
            };
            vec![log_time(&field(0)), field(1), field(3), started, field(2)]
        })
        .collect();
    Ok(columns(
        &["Time (UTC)", "Job", "Sent by", "Started by", "Result"],
        &rows,
    ))
}
