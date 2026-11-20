//! The command line, parsed by hand: one command, its options and its positionals; every
//! fault is a usage error, exit 2 (UX notes sections 1, 5, 6).

use std::path::PathBuf;

use curvyo_connector_io::pairing_code::normalise;

use crate::error::CtlError;
use crate::texts::{command_help, usage_of};

/// The default of `--last`.
pub const DEFAULT_LAST: u32 = 20;

/// Where a command goes: a connector in maintenance mode by address, or a known one by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// `--insecure <addr>`: first contact, no stored key needed.
    Insecure(String),
    /// A name from this computer's list; `None` while it holds exactly one.
    Known(Option<String>),
}

/// One parsed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `--help`, or `<command> --help` with that command's name.
    Help(Option<&'static str>),
    /// `version`.
    Version,
    /// `identify`.
    Identify(Target),
    /// `ports`.
    Ports(Target),
    /// `apply-config`.
    ApplyConfig {
        /// Where.
        target: Target,
        /// The file.
        file: PathBuf,
        /// The 16 bytes of `--fingerprint`.
        fingerprint: Option<[u8; 16]>,
    },
    /// `get-config`.
    GetConfig(Option<String>),
    /// `status`.
    Status(Option<String>),
    /// `confirm-start`: up to two positionals, a connector name and a job name.
    ConfirmStart(Vec<String>),
    /// `stop`.
    Stop(Option<String>),
    /// `unlock`.
    Unlock(Option<String>),
    /// `jobs`.
    Jobs(Option<String>, u32),
    /// `audit`.
    Audit(Option<String>, u32),
    /// `pair`.
    Pair {
        /// Which connector.
        connector: Option<String>,
        /// The app's name.
        name: String,
        /// With `--owner`.
        owner: bool,
    },
    /// `clients`.
    Clients(Option<String>),
    /// `revoke`.
    Revoke(Option<String>, String),
    /// `reset`.
    Reset(Option<String>, String),
    /// `login`.
    Login {
        /// The connector's address.
        address: String,
        /// The code as typed.
        code: String,
    },
    /// `logout`.
    Logout(Option<String>),
    /// `whoami`.
    Whoami(Option<String>),
}

/// What one command takes: the options with a value, the flags without one, the most
/// positionals.
fn shape(command: &str) -> Option<(&'static [&'static str], &'static [&'static str], usize)> {
    Some(match command {
        "identify" | "ports" => (&["--insecure"], &[], 1),
        "apply-config" => (&["--insecure", "-f", "--fingerprint"], &[], 1),
        "get-config" | "status" | "stop" | "unlock" | "clients" | "logout" | "whoami" => {
            (&[], &[], 1)
        }
        "confirm-start" | "revoke" => (&[], &[], 2),
        "jobs" | "audit" => (&["--last"], &[], 1),
        "pair" => (&["--name"], &["--owner"], 1),
        "reset" => (&["--confirm"], &[], 1),
        "login" => (&["--code"], &[], 1),
        "version" => (&[], &[], 0),
        _ => return None,
    })
}

struct Line {
    values: Vec<(String, String)>,
    flags: Vec<String>,
    positionals: Vec<String>,
}

impl Line {
    fn value(&self, option: &str) -> Option<String> {
        self.values
            .iter()
            .find(|(o, _)| o == option)
            .map(|(_, v)| v.clone())
    }

    fn first(&self) -> Option<String> {
        self.positionals.first().cloned()
    }
}

fn usage(command: &str, text: impl Into<String>) -> CtlError {
    CtlError::Usage {
        text: text.into(),
        usage: usage_of(command),
    }
}

/// `--fingerprint`: 32 hex characters, either case, spaces or hyphens between; no prefix.
///
/// # Errors
/// A usage error for any other text.
pub fn fingerprint(text: &str) -> Result<[u8; 16], CtlError> {
    let hex: String = text.chars().filter(|c| *c != '-' && *c != ' ').collect();
    let bad = || usage("apply-config", "--fingerprint needs 32 hex characters");
    if hex.len() != 32 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let mut out = [0; 16];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
    }
    Ok(out)
}

fn split(command: &'static str, words: &[String]) -> Result<Line, CtlError> {
    let (with_value, flags, most) =
        shape(command).ok_or_else(|| CtlError::usage(format!("unknown command {command}")))?;
    let mut line = Line {
        values: Vec::new(),
        flags: Vec::new(),
        positionals: Vec::new(),
    };
    let mut words = words.iter();
    while let Some(word) = words.next() {
        if with_value.contains(&word.as_str()) {
            let value = words
                .next()
                .ok_or_else(|| usage(command, format!("{word} needs a value")))?;
            line.values.push((word.clone(), value.clone()));
        } else if flags.contains(&word.as_str()) {
            line.flags.push(word.clone());
        } else if word.starts_with('-') && word.len() > 1 {
            return Err(usage(command, format!("unknown option {word}")));
        } else if line.positionals.len() < most {
            line.positionals.push(word.clone());
        } else {
            return Err(usage(command, format!("unexpected argument {word}")));
        }
    }
    Ok(line)
}

fn target(line: &Line) -> Target {
    match line.value("--insecure") {
        Some(address) => Target::Insecure(address),
        None => Target::Known(line.first()),
    }
}

fn last(command: &'static str, line: &Line) -> Result<u32, CtlError> {
    match line.value("--last") {
        None => Ok(DEFAULT_LAST),
        Some(text) => text
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=1000).contains(n))
            .ok_or_else(|| usage(command, "--last needs a whole number from 1 to 1000")),
    }
}

/// Parses the words after the program name.
///
/// # Errors
/// A usage error (exit 2).
pub fn parse(words: &[String]) -> Result<Command, CtlError> {
    let Some((first, rest)) = words.split_first() else {
        return Ok(Command::Help(None));
    };
    if matches!(first.as_str(), "--help" | "-h" | "help") {
        return Ok(Command::Help(None));
    }
    let Some(help) = command_help(first) else {
        return Err(if first.starts_with('-') {
            CtlError::usage(format!("unknown option {first}"))
        } else {
            CtlError::usage(format!("unknown command {first}"))
        });
    };
    let command = help.name;
    if rest.iter().any(|w| w == "--help" || w == "-h") {
        return Ok(Command::Help(Some(command)));
    }
    let line = split(command, rest)?;
    Ok(match command {
        "version" => Command::Version,
        "identify" => Command::Identify(target(&line)),
        "ports" => Command::Ports(target(&line)),
        "apply-config" => Command::ApplyConfig {
            target: target(&line),
            file: line
                .value("-f")
                .map(PathBuf::from)
                .ok_or_else(|| usage(command, "apply-config needs -f <file>"))?,
            fingerprint: line
                .value("--fingerprint")
                .map(|f| fingerprint(&f))
                .transpose()?,
        },
        "get-config" => Command::GetConfig(line.first()),
        "status" => Command::Status(line.first()),
        "confirm-start" => Command::ConfirmStart(line.positionals),
        "stop" => Command::Stop(line.first()),
        "unlock" => Command::Unlock(line.first()),
        "jobs" => Command::Jobs(line.first(), last(command, &line)?),
        "audit" => Command::Audit(line.first(), last(command, &line)?),
        "pair" => Command::Pair {
            connector: line.first(),
            name: line
                .value("--name")
                .ok_or_else(|| usage(command, "pair needs --name <name>"))?,
            owner: line.flags.iter().any(|f| f == "--owner"),
        },
        "clients" => Command::Clients(line.first()),
        "revoke" => {
            let mut p = line.positionals;
            let name = p
                .pop()
                .ok_or_else(|| usage(command, "revoke needs <name>"))?;
            Command::Revoke(p.pop(), name)
        }
        "reset" => Command::Reset(
            line.first(),
            line.value("--confirm")
                .ok_or_else(|| usage(command, "reset needs --confirm <connector name>"))?,
        ),
        "login" => {
            let address = line
                .first()
                .ok_or_else(|| usage(command, "login needs <addr>"))?;
            let code = line
                .value("--code")
                .ok_or_else(|| usage(command, "login needs --code <code>"))?;
            if normalise(&code).is_none() {
                return Err(usage(command, "--code is not a pairing code"));
            }
            Command::Login { address, code }
        }
        "logout" => Command::Logout(line.first()),
        _ => Command::Whoami(line.first()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    fn text(line: &str) -> String {
        parse(&words(line)).unwrap_err().to_string()
    }

    #[test]
    fn a22_commands_options_and_usage_errors() {
        assert_eq!(
            parse(&words("pair --name Phone --owner")).unwrap(),
            Command::Pair {
                connector: None,
                name: "Phone".into(),
                owner: true
            }
        );
        assert_eq!(
            parse(&words("revoke workshop-laser Phone")).unwrap(),
            Command::Revoke(Some("workshop-laser".into()), "Phone".into())
        );
        assert_eq!(
            parse(&words("jobs --last 5")).unwrap(),
            Command::Jobs(None, 5)
        );
        assert_eq!(
            parse(&words("identify --insecure 192.168.1.20")).unwrap(),
            Command::Identify(Target::Insecure("192.168.1.20".into()))
        );
        assert_eq!(text("frob"), "unknown command frob");
        assert_eq!(text("reset --forse"), "unknown option --forse");
        assert_eq!(text("pair --name"), "--name needs a value");
        assert_eq!(text("reset"), "reset needs --confirm <connector name>");
        assert_eq!(
            text("jobs --last 0"),
            "--last needs a whole number from 1 to 1000"
        );
        assert_eq!(
            text("login 10.0.0.2 --code K7QM-4XT"),
            "--code is not a pairing code"
        );
        assert!(parse(&words("login 10.0.0.2 --code k7qm4xtr")).is_ok());
        assert_eq!(
            parse(&words("pair --help")).unwrap(),
            Command::Help(Some("pair"))
        );
        assert_eq!(parse(&[]).unwrap(), Command::Help(None));
        assert_eq!(parse(&words("frob")).unwrap_err().exit_code(), 2);
    }

    #[test]
    fn a23_the_fingerprint_takes_all_32_hex_characters_in_any_grouping() {
        let want = fingerprint("3F2A9C1077B4E2D10A5C61F89B3E4D27").unwrap();
        assert_eq!(
            fingerprint("3f2a-9c10-77b4-e2d1-0a5c-61f8-9b3e-4d27").unwrap(),
            want
        );
        assert_eq!(
            fingerprint("3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27").unwrap(),
            want
        );
        for bad in [
            "3F2A-9C10",
            "3F2A9C1077B4E2D10A5C61F89B3E4D2",
            "3F2A9C1077B4E2D10A5C61F89B3E4D2X",
            "0x3F2A9C1077B4E2D10A5C61F89B3E4D",
        ] {
            assert_eq!(
                fingerprint(bad).unwrap_err().to_string(),
                "--fingerprint needs 32 hex characters",
                "{bad}"
            );
        }
    }
}
