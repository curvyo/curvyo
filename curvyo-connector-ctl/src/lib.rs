//! `curvyoctl` as a library (ADR 0071 decision 10, `specs/0128`): `run` parses one command line,
//! talks to one connector and writes the result, so the tests drive it in process.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod args;
mod audit;
mod connect;
mod context;
mod error;
mod jobs;
mod known;
mod login;
mod passphrase;
mod people;
mod reset;
mod secrets;
mod setup;
mod texts;

use std::io::Write;
use std::process::ExitCode;

use curvyo_connector_core::PROTOCOL_VERSION;

pub use args::{Command, Target};
pub use context::Context;
pub use error::CtlError;
pub use known::{KNOWN_FILE, Known};

use crate::args::parse;
use crate::texts::{HELP, command_help};

fn help(command: Option<&str>) -> String {
    match command.and_then(command_help) {
        None => HELP.to_owned(),
        Some(c) => {
            let mut text = format!("{}\n{}\n", c.usage, c.sentence);
            for option in c.options {
                text.push_str("  ");
                text.push_str(option);
                text.push('\n');
            }
            text
        }
    }
}

/// Runs one command; output and its result in the commands' words.
fn dispatch(command: &Command, ctx: &mut Context) -> Result<(String, Option<CtlError>), CtlError> {
    let done = |text: String| (text, None);
    Ok(match command {
        Command::Help(c) => done(help(*c)),
        Command::Version => done(format!(
            "curvyoctl {}, protocol {PROTOCOL_VERSION}\n",
            env!("CARGO_PKG_VERSION")
        )),
        Command::Identify(target) => setup::identify(ctx, target)?,
        Command::Ports(target) => done(setup::ports(ctx, target)?),
        Command::ApplyConfig {
            target,
            file,
            fingerprint,
        } => done(setup::apply_config(ctx, target, file, *fingerprint)?),
        Command::GetConfig(c) => done(setup::get_config(ctx, c.as_deref())?),
        Command::Status(c) => done(jobs::status(ctx, c.as_deref())?),
        Command::ConfirmStart(p) => done(jobs::confirm_start(ctx, p)?),
        Command::Stop(c) => done(jobs::stop(ctx, c.as_deref())?),
        Command::Unlock(c) => done(jobs::unlock(ctx, c.as_deref())?),
        Command::Jobs(c, last) => done(jobs::jobs(ctx, c.as_deref(), *last)?),
        Command::Audit(c, last) => done(audit::audit(ctx, c.as_deref(), *last)?),
        Command::Pair {
            connector,
            name,
            owner,
        } => done(people::pair(ctx, connector.as_deref(), name, *owner)?),
        Command::Clients(c) => done(people::clients(ctx, c.as_deref())?),
        Command::Revoke(c, name) => done(people::revoke(ctx, c.as_deref(), name)?),
        Command::Reset(c, confirm) => done(reset::reset(ctx, c.as_deref(), confirm)?),
        Command::Login { address, code } => done(login::login(ctx, address, code)?),
        Command::Logout(c) => done(login::logout(ctx, c.as_deref())?),
        Command::Whoami(c) => done(login::whoami(ctx, c.as_deref())?),
    })
}

/// Writes a refusal: `curvyoctl: <text>`, its detail lines, and for a usage error the usage line.
fn write_error(err: &mut dyn Write, error: &CtlError) {
    let _ = writeln!(err, "curvyoctl: {error}");
    match error {
        CtlError::Refused { details, .. } => {
            let width = details.iter().map(|(l, _)| l.len() + 1).max().unwrap_or(0);
            for (label, value) in details {
                let _ = writeln!(err, "{:<width$} {value}", format!("{label}:"));
            }
        }
        CtlError::Usage {
            usage: Some(usage), ..
        } => {
            let _ = writeln!(err, "{usage}");
        }
        CtlError::Usage { .. } => {}
    }
}

/// Runs the command line `args` (without the program name): results on `out`, every refusal
/// on `err`; exit 0 done, 1 refused or not reachable, 2 usage (A22).
pub fn run(
    args: &[String],
    ctx: &mut Context,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode {
    ExitCode::from(run_code(args, ctx, out, err))
}

/// [`run`] with the exit code as a number.
pub fn run_code(
    args: &[String],
    ctx: &mut Context,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let result = parse(args).and_then(|command| dispatch(&command, ctx));
    let code = match result {
        Ok((text, refusal)) => {
            let _ = out.write_all(text.as_bytes());
            refusal.map_or(0, |e| {
                write_error(err, &e);
                e.exit_code()
            })
        }
        Err(e) => {
            write_error(err, &e);
            e.exit_code()
        }
    };
    let _ = out.flush();
    code
}
