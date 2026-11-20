//! `reset --confirm <name>` (A19; UX notes 4.11): the typed name is the confirmation, checked
//! here and again by the connector; afterwards the connector is removed from this computer.

use curvyo_connector_core::{ErrorCode, Message, State};
use std::fmt::Write as _;

use crate::connect::{self, refusal, request};
use crate::context::Context;
use crate::error::CtlError;
use crate::login::forget;
use crate::texts::{number, usage_of};

/// `reset`.
///
/// # Errors
/// A usage error for a name that does not match, or the failure in words.
pub fn reset(
    ctx: &mut Context,
    connector: Option<&str>,
    confirm: &str,
) -> Result<String, CtlError> {
    let entry = connect::resolve(ctx, connector)?;
    if confirm != entry.name {
        return Err(CtlError::Usage {
            text: format!("--confirm {confirm} does not match {}", entry.name),
            usage: usage_of("reset"),
        });
    }
    let mut owner = connect::owner(ctx, Some(&entry.name))?;
    let who = owner.entry.name.clone();
    let mut text = String::new();
    if let Message::Status {
        state: State::Running | State::Paused,
        job,
        job_name,
        ..
    } = &owner.status
    {
        let line = job.as_ref().map_or(0, |j| j.line);
        let name = job_name.as_deref().unwrap_or("the job");
        let _ = writeln!(text, "Stopped {name} at line {}", number(line));
    }
    let message = Message::Reset {
        connector_name: confirm.to_owned(),
    };
    match request(&mut owner.connection, &message, &who)? {
        Message::ResetDone => {
            forget(ctx, &owner.entry)?;
            let _ = writeln!(
                text,
                "Reset {who}: stopped, configuration, key, clients and logs deleted"
            );
            let _ = writeln!(text, "Removed {who} from this computer");
            Ok(text)
        }
        Message::Error {
            code: ErrorCode::UnknownName,
            ..
        } => Err(CtlError::refused(format!("{who} is not called {confirm}"))),
        Message::Error { code, .. } => Err(refusal(code, &who, &owner.connection.greeting.version)),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}
