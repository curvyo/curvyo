//! The client commands (A10, A11, A35; UX notes 4.9, 4.10): `pair`, `clients` and `revoke`.

use curvyo_connector_core::{ErrorCode, Message, Role, check_client_name};
use std::fmt::Write as _;

use crate::connect::{self, refusal, request};
use crate::context::Context;
use crate::error::CtlError;
use crate::login::forget;
use crate::texts::{columns, labelled, log_time, role_word};

/// `pair --name <name> [--owner]`: the code, printed only here, never stored (A10).
///
/// # Errors
/// The failure in words.
pub fn pair(
    ctx: &mut Context,
    connector: Option<&str>,
    name: &str,
    owner_role: bool,
) -> Result<String, CtlError> {
    check_client_name(name).map_err(|reason| CtlError::Usage {
        text: format!("--name {reason}"),
        usage: crate::texts::usage_of("pair"),
    })?;
    let role = if owner_role {
        Role::Owner
    } else {
        Role::Member
    };
    let mut owner = connect::owner(ctx, connector)?;
    let who = owner.entry.name.clone();
    let message = Message::IssueCode {
        client_name: name.to_owned(),
        role,
    };
    match request(&mut owner.connection, &message, &who)? {
        Message::CodeIssued { code, expires_in_s } => Ok(labelled(&[
            (
                "Pairing code",
                format!("{}, valid for {} minutes", code.expose(), expires_in_s / 60),
            ),
            ("For", format!("{name}, {}", role_word(role))),
        ])),
        Message::Error {
            code: ErrorCode::WrongState,
            ..
        } => Err(CtlError::refused(format!(
            "{name} is the last owner of {who}"
        ))),
        Message::Error { code, .. } => Err(refusal(code, &who, &owner.connection.greeting.version)),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}

/// `clients`: "(this computer)" marks the row whose token this computer uses.
///
/// # Errors
/// The failure in words.
pub fn clients(ctx: &mut Context, connector: Option<&str>) -> Result<String, CtlError> {
    let mut owner = connect::owner(ctx, connector)?;
    let who = owner.entry.name.clone();
    let me = owner
        .connection
        .session
        .as_ref()
        .map(|(name, _)| name.clone());
    match connect::call(&mut owner.connection, &Message::ListClients, &who)? {
        Message::Clients { clients } => {
            let rows: Vec<Vec<String>> = clients
                .iter()
                .map(|c| {
                    let mut paired = log_time(&c.paired_at).chars().take(16).collect::<String>();
                    if me.as_deref() == Some(c.name.as_str()) {
                        paired.push_str("  (this computer)");
                    }
                    vec![c.name.clone(), role_word(c.role).to_owned(), paired]
                })
                .collect();
            Ok(columns(&["Name", "Role", "Paired (UTC)"], &rows))
        }
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}

/// `revoke <name>`; revoking this computer also removes the connector from it.
///
/// # Errors
/// "<name> is the last owner of <connector>", "<connector> has no client called <name>".
pub fn revoke(ctx: &mut Context, connector: Option<&str>, name: &str) -> Result<String, CtlError> {
    let mut owner = connect::owner(ctx, connector)?;
    let who = owner.entry.name.clone();
    let me = owner.connection.session.as_ref().map(|(n, _)| n.clone());
    let message = Message::RevokeClient {
        name: name.to_owned(),
    };
    match request(&mut owner.connection, &message, &who)? {
        Message::ClientRevoked { .. } => {
            let mut text = format!("Revoked {name}\n");
            if me.as_deref() == Some(name) {
                forget(ctx, &owner.entry)?;
                let _ = writeln!(text, "Removed {who} from this computer");
            }
            Ok(text)
        }
        Message::Error {
            code: ErrorCode::WrongState,
            ..
        } => Err(CtlError::refused(format!(
            "{name} is the last owner of {who}"
        ))),
        Message::Error {
            code: ErrorCode::UnknownName,
            ..
        } => Err(CtlError::refused(format!(
            "{who} has no client called {name}"
        ))),
        Message::Error { code, .. } => Err(refusal(code, &who, &owner.connection.greeting.version)),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}
