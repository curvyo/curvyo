//! A second computer (A35 to A38; UX notes 4.13): `login` pairs it as an owner with a code from
//! `pair --owner`, `logout` forgets the connector locally, `whoami` shows its name and role.

use std::time::Instant;

use curvyo_connector_core::{ConnectorMode, Role};
use curvyo_connector_io::frames::RefusalReason;
use curvyo_connector_io::pairing::PairError;

use crate::connect::{self, ANSWER, client_error, normal};
use crate::context::Context;
use crate::error::CtlError;
use crate::known::Known;
use crate::secrets;
use crate::texts::{labelled, role_word};

/// Removes `entry` and its token from this computer; the connector is not asked.
///
/// # Errors
/// The list's or the store's failure.
pub fn forget(ctx: &mut Context, entry: &Known) -> Result<(), CtlError> {
    if let Ok(spki) = entry.spki() {
        secrets::remove(ctx.store()?, &spki)?;
    }
    ctx.known().remove(&entry.address)
}

fn pair_error(error: PairError, who: &str, address: &str) -> CtlError {
    match error {
        PairError::Refused {
            reason: RefusalReason::WrongCode,
            attempts_left: 0,
        } => CtlError::refused("code spent"),
        PairError::Refused {
            reason: RefusalReason::WrongCode,
            attempts_left: 1,
        } => CtlError::refused("wrong code, 1 attempt left"),
        PairError::Refused {
            reason: RefusalReason::WrongCode,
            attempts_left,
        } => CtlError::refused(format!("wrong code, {attempts_left} attempts left")),
        PairError::Refused {
            reason: RefusalReason::CodeExpired,
            ..
        } => CtlError::refused("code expired"),
        PairError::Refused {
            reason: RefusalReason::NoCode,
            ..
        }
        | PairError::BadCode => CtlError::refused("code not valid"),
        PairError::Unverified => CtlError::refused("wrong code"),
        PairError::Client(e) => client_error(e, who, address),
    }
}

/// `login <addr> --code <code>`.
///
/// # Errors
/// One line each (A37); nothing is stored then.
pub fn login(ctx: &mut Context, address: &str, code: &str) -> Result<String, CtlError> {
    let address = normal(address);
    // The token is stored after the code is spent: the store must exist before it is sent.
    ctx.store()?;
    let list = ctx.known().list()?;
    let stored = list.iter().find(|k| k.address == address).cloned();
    let mut connection = match &stored {
        // The stored key is authoritative: a changed key is refused before the code is sent.
        Some(entry) => connect::pinned(&address, entry.spki()?, &entry.name)?,
        None => connect::insecure(&address)?,
    };
    let who = connection.greeting.name.clone();
    if matches!(
        connection.greeting.mode,
        Some(ConnectorMode::Maintenance { .. })
    ) {
        return Err(CtlError::refused(format!("{who} is in maintenance mode")));
    }
    if let Some(other) = list.iter().find(|k| k.name == who && k.address != address) {
        return Err(CtlError::refused(format!(
            "a connector called {who} is already known at {}",
            other.address
        )));
    }
    let record = connection
        .pair(code, &ctx.client_name(), Instant::now() + ANSWER)
        .map_err(|e| pair_error(e, &who, &address))?;
    let Some((name, role)) = connection.session.clone() else {
        return Err(CtlError::refused(format!(
            "connector too old (version {})",
            connection.greeting.version
        )));
    };
    if role == Role::Member {
        return Err(CtlError::refused("the code was issued for a member"));
    }
    secrets::put(ctx.store()?, &record.spki, &record.token)?;
    ctx.known().put(Known {
        name: who.clone(),
        address: address.clone(),
        spki_sha256: record.spki.to_hex(),
    })?;
    Ok(format!(
        "Logged in to {who}. This computer is an owner.\n{}",
        labelled(&[
            ("Connector", who.clone()),
            ("Address", address),
            ("Name", name),
            ("Stored key", record.spki.fingerprint()),
        ])
    ))
}

/// `logout [<connector>]`: local only, so it works offline and with a lost connector.
///
/// # Errors
/// The list's or the store's failure.
pub fn logout(ctx: &mut Context, connector: Option<&str>) -> Result<String, CtlError> {
    let entry = connect::resolve(ctx, connector)?;
    forget(ctx, &entry)?;
    Ok(format!("Removed {} from this computer\n", entry.name))
}

/// `whoami [<connector>]`.
///
/// # Errors
/// The refusals of every owner command.
pub fn whoami(ctx: &mut Context, connector: Option<&str>) -> Result<String, CtlError> {
    let owner = connect::owner(ctx, connector)?;
    let (name, role) = owner.connection.session.clone().ok_or_else(|| {
        CtlError::refused(format!(
            "connector too old (version {})",
            owner.connection.greeting.version
        ))
    })?;
    Ok(labelled(&[
        ("Connector", owner.entry.name.clone()),
        ("Address", owner.entry.address.clone()),
        ("Name", name),
        ("Role", role_word(role).to_owned()),
        ("Key", owner.connection.spki.fingerprint()),
    ]))
}
