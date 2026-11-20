//! The setup commands (A22, A23; UX notes 4.1 to 4.4): `identify`, `ports`, `apply-config` and
//! `get-config`, with trust on first use and the `--fingerprint` check before anything is sent.

use std::fmt::Write as _;
use std::path::Path;

use curvyo_connector_core::{ConnectorConfig, ConnectorMode, ErrorCode, Message};
use curvyo_connector_io::client::Connection;
use curvyo_connector_io::config_file::{self, ConfigFileError};
use curvyo_connector_io::spki::SpkiHash;
use time::OffsetDateTime;

use crate::args::Target;
use crate::connect::{self, normal, refusal, request};
use crate::context::Context;
use crate::error::CtlError;
use crate::known::Known;
use crate::secrets;
use crate::texts::{labelled, minutes};

fn mode_lines(mode: Option<&ConnectorMode>) -> Vec<(&'static str, String)> {
    match mode {
        Some(ConnectorMode::Maintenance { closes_in_s: 0 }) => vec![
            ("Mode", "Maintenance mode, closed".to_owned()),
            ("Setup", "closed".to_owned()),
        ],
        Some(ConnectorMode::Maintenance { closes_in_s }) => {
            let at = OffsetDateTime::now_utc()
                + time::Duration::seconds(i64::try_from(*closes_in_s).unwrap_or(0));
            vec![
                (
                    "Mode",
                    format!(
                        "Maintenance mode, closes in {} ({:02}:{:02} UTC)",
                        minutes(*closes_in_s),
                        at.hour(),
                        at.minute()
                    ),
                ),
                ("Setup", "open to the local network".to_owned()),
            ]
        }
        Some(ConnectorMode::Configured { .. }) => vec![("Mode", "Configured".to_owned())],
        Some(ConnectorMode::Foreground) => vec![("Mode", "Foreground".to_owned())],
        None => Vec::new(),
    }
}

/// The `identify` lines of a greeted connection.
fn identity(connection: &Connection) -> String {
    let greeting = &connection.greeting;
    let mut rows = vec![("Name", greeting.name.clone())];
    if let Some(ConnectorMode::Configured { owner: Some(owner) }) = &greeting.mode {
        rows.push(("Belongs to", owner.clone()));
    }
    rows.push((
        "Software",
        format!(
            "Curvyo connector {}, protocol {}",
            greeting.version, greeting.protocol
        ),
    ));
    rows.extend(mode_lines(greeting.mode.as_ref()));
    rows.push(("Key", connection.spki.fingerprint()));
    labelled(&rows)
}

fn key_changed(entry: &Known, presented: SpkiHash) -> CtlError {
    CtlError::with_keys(
        format!("the key of {} changed", entry.name),
        vec![
            (
                "Stored key",
                entry.spki().map(|k| k.fingerprint()).unwrap_or_default(),
            ),
            ("Presented key", presented.fingerprint()),
        ],
    )
}

fn entry_at(ctx: &Context, address: &str) -> Result<Option<Known>, CtlError> {
    Ok(ctx
        .known()
        .list()?
        .into_iter()
        .find(|k| k.address == address))
}

/// `identify`: the lines, and a refusal after them when the stored key of that address differs.
///
/// # Errors
/// The failure in words.
pub fn identify(
    ctx: &mut Context,
    target: &Target,
) -> Result<(String, Option<CtlError>), CtlError> {
    match target {
        Target::Known(name) => {
            let entry = connect::resolve(ctx, name.as_deref())?;
            let connection = connect::pinned(&entry.address, entry.spki()?, &entry.name)?;
            Ok((identity(&connection), None))
        }
        Target::Insecure(address) => {
            let address = normal(address);
            let connection = connect::insecure(&address)?;
            let text = identity(&connection);
            match entry_at(ctx, &address)? {
                Some(entry) if entry.spki()? != connection.spki => {
                    Ok((text, Some(key_changed(&entry, connection.spki))))
                }
                _ => Ok((text, None)),
            }
        }
    }
}

fn paths(answer: Message, who: &str, version: &str) -> Result<String, CtlError> {
    match answer {
        Message::Ports { paths } if paths.is_empty() => Ok("No serial ports\n".to_owned()),
        Message::Ports { paths } => Ok(paths.iter().fold(String::new(), |mut text, p| {
            let _ = writeln!(text, "{p}");
            text
        })),
        Message::Error { code, .. } => Err(refusal(code, who, version)),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}

/// `ports`.
///
/// # Errors
/// The failure in words.
pub fn ports(ctx: &mut Context, target: &Target) -> Result<String, CtlError> {
    match target {
        Target::Insecure(address) => {
            let mut connection = connect::insecure(address)?;
            let who = connection.greeting.name.clone();
            let answer = request(&mut connection, &Message::ListPorts, &who)?;
            paths(answer, &who, &connection.greeting.version)
        }
        Target::Known(name) => {
            let mut owner = connect::owner(ctx, name.as_deref())?;
            let who = owner.entry.name.clone();
            let answer = request(&mut owner.connection, &Message::ListPorts, &who)?;
            paths(answer, &who, &owner.connection.greeting.version)
        }
    }
}

/// Reads and checks the maker's file before anything is sent (UX notes 4.3).
///
/// # Errors
/// "cannot read <file>: ...", "<file>, line n: ...", "<file>: <field>: <rule>".
pub fn read_config(file: &Path) -> Result<ConnectorConfig, CtlError> {
    let shown = file.display();
    let text = std::fs::read_to_string(file)
        .map_err(|e| CtlError::refused(format!("cannot read {shown}: {e}")))?;
    config_file::parse(&text).map_err(|e| match e {
        ConfigFileError::Syntax { line, message } => {
            CtlError::refused(format!("{shown}, line {line}: {message}"))
        }
        other => CtlError::refused(format!("{shown}: {other}")),
    })
}

/// `apply-config`.
///
/// # Errors
/// The refusal in words; nothing is sent after a key mismatch.
pub fn apply_config(
    ctx: &mut Context,
    target: &Target,
    file: &Path,
    fingerprint: Option<[u8; 16]>,
) -> Result<String, CtlError> {
    let config = read_config(file)?;
    match target {
        Target::Insecure(address) => {
            first_config(ctx, &normal(address), &config, file, fingerprint)
        }
        Target::Known(name) => change_config(ctx, name.as_deref(), &config, file),
    }
}

fn first_config(
    ctx: &mut Context,
    address: &str,
    config: &ConnectorConfig,
    file: &Path,
    fingerprint: Option<[u8; 16]>,
) -> Result<String, CtlError> {
    let mut connection = connect::insecure(address)?;
    let presented = connection.spki;
    let stored = entry_at(ctx, address)?;
    if let Some(expected) = fingerprint {
        if presented.as_bytes()[..16] != expected {
            let expected_hex = expected.iter().fold(String::new(), |mut hex, b| {
                let _ = write!(hex, "{b:02X}");
                hex
            });
            let grouped: Vec<&str> = (0..8).map(|i| &expected_hex[4 * i..4 * i + 4]).collect();
            return Err(CtlError::with_keys(
                format!("the key at {address} differs from --fingerprint"),
                vec![
                    ("Expected key", grouped.join(" ")),
                    ("Presented key", presented.fingerprint()),
                ],
            ));
        }
    } else if let Some(entry) = &stored
        && entry.spki()? != presented
    {
        return Err(key_changed(entry, presented));
    }
    // The token comes back once: the store must exist before the configuration is sent.
    ctx.store()?;
    let who = connection.greeting.name.clone();
    let message = Message::ApplyConfig {
        config: config.clone(),
        client_name: ctx.client_name(),
    };
    match request(&mut connection, &message, &who)? {
        Message::ConfigApplied { token: Some(token) } => {
            let token =
                curvyo_connector_io::token::Token::from_hex(token.expose()).ok_or_else(|| {
                    CtlError::refused(format!("{who} sent a token that is not valid"))
                })?;
            if let Some(old) = stored
                .as_ref()
                .and_then(|e| e.spki().ok())
                .filter(|k| *k != presented)
            {
                secrets::remove(ctx.store()?, &old)?;
            }
            secrets::put(ctx.store()?, &presented, &token)?;
            ctx.known().put(Known {
                name: config.name.clone(),
                address: address.to_owned(),
                spki_sha256: presented.to_hex(),
            })?;
            let key_label = if fingerprint.is_some() {
                "Key"
            } else {
                "Stored key"
            };
            Ok(format!(
                "Configured. You are the owner.\n{}",
                labelled(&[
                    ("Connector", config.name.clone()),
                    ("Controller", config.controller.describe()),
                    (key_label, presented.fingerprint()),
                ])
            ))
        }
        Message::Error { code, message } => Err(match code {
            ErrorCode::OwnerRequired => CtlError::refused(format!("{who} is already configured")),
            ErrorCode::MaintenanceClosed => {
                CtlError::refused(format!("the setup window of {who} is closed"))
            }
            ErrorCode::InvalidConfig => CtlError::refused(format!("{}: {message}", file.display())),
            other => refusal(other, &who, &connection.greeting.version),
        }),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}

fn change_config(
    ctx: &mut Context,
    name: Option<&str>,
    config: &ConnectorConfig,
    file: &Path,
) -> Result<String, CtlError> {
    let mut owner = connect::owner(ctx, name)?;
    let who = owner.entry.name.clone();
    let before = match connect::call(&mut owner.connection, &Message::GetConfig, &who)? {
        Message::Config { config } => Some(config),
        _ => None,
    };
    let message = Message::ApplyConfig {
        config: config.clone(),
        client_name: ctx.client_name(),
    };
    match request(&mut owner.connection, &message, &who)? {
        Message::ConfigApplied { .. } => {
            if config.name != owner.entry.name {
                let mut entry = owner.entry.clone();
                entry.name.clone_from(&config.name);
                ctx.known().put(entry)?;
            }
            let mut text = "Configuration changed.\n".to_owned();
            if before.map(|b| b.controller) != Some(config.controller.clone()) {
                text.push_str(&labelled(&[("Controller", config.controller.describe())]));
            }
            Ok(text)
        }
        Message::Error { code, message } => Err(match code {
            ErrorCode::WrongState => {
                CtlError::refused(format!("{who} is running a job; configuration not changed"))
            }
            ErrorCode::InvalidConfig => CtlError::refused(format!("{}: {message}", file.display())),
            other => refusal(other, &who, &owner.connection.greeting.version),
        }),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}

/// `get-config`: the configuration as `connector.toml` holds it.
///
/// # Errors
/// The failure in words.
pub fn get_config(ctx: &mut Context, name: Option<&str>) -> Result<String, CtlError> {
    let mut owner = connect::owner(ctx, name)?;
    let who = owner.entry.name.clone();
    match connect::call(&mut owner.connection, &Message::GetConfig, &who)? {
        Message::Config { config } => Ok(config_file::to_text(&config)),
        other => Err(CtlError::refused(format!("{who} refused: {other:?}"))),
    }
}
