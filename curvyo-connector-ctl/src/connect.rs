//! Reaching a connector (addendum D12): first contact without a stored key, a known one by its
//! pinned key and its token, and the words of every failure (UX notes section 5).

use std::time::{Duration, Instant};

use curvyo_connector_core::{ErrorCode, Message};
use curvyo_connector_io::client::{
    CONNECT_TIMEOUT, ClientError, Connection, DEFAULT_PORT, connect,
};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::spki::SpkiHash;

use crate::context::Context;
use crate::error::CtlError;
use crate::known::Known;
use crate::secrets;

/// How long a request may wait for its answer.
pub const ANSWER: Duration = Duration::from_secs(5);

/// `host:port` with the default port when none is given.
#[must_use]
pub fn normal(address: &str) -> String {
    let text = address.trim();
    if let Ok(socket) = text.parse::<std::net::SocketAddr>() {
        return socket.to_string();
    }
    if let Ok(ip) = text.parse::<std::net::IpAddr>() {
        return std::net::SocketAddr::new(ip, DEFAULT_PORT).to_string();
    }
    if text.contains(':') {
        text.to_owned()
    } else {
        format!("{text}:{DEFAULT_PORT}")
    }
}

/// The words of a failed connection to `address` (`who` names it once it is known).
#[must_use]
pub fn client_error(error: ClientError, who: &str, address: &str) -> CtlError {
    match error {
        ClientError::BadAddress(_) => CtlError::usage(format!("not an address: {address}")),
        ClientError::Unreachable(dialled) => {
            CtlError::refused(format!("nothing answers at {dialled}"))
        }
        ClientError::TimedOut => CtlError::refused(format!("no answer from {who}")),
        ClientError::KeyChanged { stored, presented } => CtlError::with_keys(
            format!("the key of {who} changed"),
            vec![
                ("Stored key", stored.fingerprint()),
                ("Presented key", presented.fingerprint()),
            ],
        ),
        ClientError::NoLongerPaired => {
            CtlError::refused(format!("{who} does not know this computer"))
        }
        ClientError::InMaintenance => CtlError::refused(format!("{who} is in maintenance mode")),
        ClientError::ProtocolMismatch { connector } => {
            CtlError::refused(format!("{who} speaks protocol {connector}"))
        }
        ClientError::Tls(_) | ClientError::Io(_) | ClientError::Protocol(_) => {
            CtlError::refused(format!("{address} closed the connection"))
        }
    }
}

/// A connection to `address` trusting the key it presents (`--insecure`).
///
/// # Errors
/// The failure in words.
pub fn insecure(address: &str) -> Result<Connection, CtlError> {
    let address = normal(address);
    connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .map_err(|e| client_error(e, &address, &address))
}

/// A connection to `address` that sends nothing unless the key is `spki`.
///
/// # Errors
/// The failure in words; a changed key names `who`.
pub fn pinned(address: &str, spki: SpkiHash, who: &str) -> Result<Connection, CtlError> {
    connect(address, PinMode::Expect(spki), CONNECT_TIMEOUT)
        .map_err(|e| client_error(e, who, address))
}

/// The entry `name` names; without a name the only one (UX notes V5; usage errors otherwise).
///
/// # Errors
/// "no connector is known", "more than one connector is known: ...", "no connector is called ...".
pub fn resolve(ctx: &Context, name: Option<&str>) -> Result<Known, CtlError> {
    let list = ctx.known().list()?;
    match name {
        Some(name) => list
            .into_iter()
            .find(|k| k.name == name)
            .ok_or_else(|| CtlError::usage(format!("no connector is called {name}"))),
        None => match list.len() {
            0 => Err(CtlError::usage("no connector is known")),
            1 => Ok(list
                .into_iter()
                .next()
                .ok_or_else(|| CtlError::usage("no connector is known"))?),
            _ => {
                let mut names: Vec<String> = list.into_iter().map(|k| k.name).collect();
                names.sort();
                Err(CtlError::usage(format!(
                    "more than one connector is known: {}",
                    names.join(", ")
                )))
            }
        },
    }
}

/// An owner's connection to the known connector `name`: pinned key, then the stored token.
///
/// # Errors
/// The failure in words.
pub fn owner(ctx: &mut Context, name: Option<&str>) -> Result<Owner, CtlError> {
    let entry = resolve(ctx, name)?;
    let spki = entry.spki()?;
    let token = secrets::get(ctx.store()?, &spki, &entry.name)?;
    let mut connection = pinned(&entry.address, spki, &entry.name)?;
    let status = connection
        .auth(&token, Instant::now() + ANSWER)
        .map_err(|e| client_error(e, &entry.name, &entry.address))?;
    Ok(Owner {
        entry,
        connection,
        status,
    })
}

/// A logged-in connection to a known connector and the status its login answered with.
pub struct Owner {
    /// The entry of this computer's list.
    pub entry: Known,
    /// The connection.
    pub connection: Connection,
    /// The status after the login.
    pub status: Message,
}

/// The answer to `message`; an `Error` answer is returned as an `Error` message.
///
/// # Errors
/// The connection's failure in words.
pub fn request(
    connection: &mut Connection,
    message: &Message,
    who: &str,
) -> Result<Message, CtlError> {
    connection
        .request(message, Instant::now() + ANSWER)
        .map_err(|e| match e {
            ClientError::TimedOut => CtlError::refused(format!("no answer from {who}")),
            other => client_error(other, who, who),
        })
}

/// The words of an `Error` answer that no command has its own words for (UX notes section 5).
#[must_use]
pub fn refusal(code: ErrorCode, who: &str, version: &str) -> CtlError {
    match code {
        ErrorCode::UnknownType => {
            CtlError::refused(format!("connector too old (version {version})"))
        }
        ErrorCode::OwnerRequired => {
            CtlError::refused(format!("this computer is not an owner of {who}"))
        }
        ErrorCode::Maintenance => CtlError::refused(format!("{who} is in maintenance mode")),
        ErrorCode::PairingRequired => {
            CtlError::refused(format!("{who} does not know this computer"))
        }
        other => CtlError::refused(format!("{who} refused: {other:?}")),
    }
}

/// `request`, with every `Error` answer turned into its words.
///
/// # Errors
/// The connection's failure or the refusal in words.
pub fn call(
    connection: &mut Connection,
    message: &Message,
    who: &str,
) -> Result<Message, CtlError> {
    match request(connection, message, who)? {
        Message::Error { code, .. } => Err(refusal(code, who, &connection.greeting.version)),
        answer => Ok(answer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a25_an_older_connector_and_the_owner_refusals_in_words() {
        let words = |code| refusal(code, "workshop-laser", "0.1.0").to_string();
        assert_eq!(
            words(ErrorCode::UnknownType),
            "connector too old (version 0.1.0)"
        );
        assert_eq!(
            words(ErrorCode::OwnerRequired),
            "this computer is not an owner of workshop-laser"
        );
        assert_eq!(
            words(ErrorCode::Maintenance),
            "workshop-laser is in maintenance mode"
        );
        assert_eq!(
            words(ErrorCode::Malformed),
            "workshop-laser refused: Malformed"
        );
        assert_eq!(normal("192.168.1.20"), "192.168.1.20:7447");
        assert_eq!(normal("workshop"), "workshop:7447");
        assert_eq!(normal("[::1]:9000"), "[::1]:9000");
    }
}
