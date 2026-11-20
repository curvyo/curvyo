//! The connector's configuration (`connector.toml`, `ApplyConfig`) and its validation, pure.

use core::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::count::count;

/// The format this build reads and writes.
pub const CONFIG_FORMAT: u32 = 1;
/// The longest connector name and owner label, in characters (A16).
pub const MAX_LABEL: usize = 64;
/// The longest client name, in characters (A16).
pub const MAX_CLIENT_NAME: usize = 32;
/// The longest serial port path, in bytes.
pub const MAX_PORT_BYTES: usize = 255;
/// The highest baud rate (`specs/0127` G1).
pub const MAX_BAUD: u32 = 4_000_000;
/// The highest pace of the fake controller.
pub const MAX_LINES_PER_SECOND: u32 = 10_000;
/// The baud rate a `grbl-serial` controller without `baud` gets.
pub const DEFAULT_BAUD: u32 = 115_200;
/// The pace a fake controller without `lines_per_second` gets.
pub const DEFAULT_LINES_PER_SECOND: u32 = 20;

/// What one connector is set up as (ADR 0071 decision 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorConfig {
    /// Always [`CONFIG_FORMAT`].
    pub format: u32,
    /// The connector's name: what `Welcome` carries and `curvyoctl` calls it.
    pub name: String,
    /// The owner label ("Marc's workshop"), shown as "Belongs to".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Where to listen, `addr:port`; counts from the next start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen: Option<String>,
    /// The controller.
    pub controller: ControllerConfig,
}

/// The controller of a configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ControllerConfig {
    /// GRBL on a serial port.
    GrblSerial {
        /// The port path.
        port: String,
        /// Its baud rate.
        #[serde(default = "default_baud")]
        baud: u32,
    },
    /// The fake controller: nothing moves.
    Fake {
        /// Its pace.
        #[serde(default = "default_pace")]
        lines_per_second: u32,
        /// The line at which it raises an alarm.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alarm_at_line: Option<u32>,
    },
}

fn default_baud() -> u32 {
    DEFAULT_BAUD
}

fn default_pace() -> u32 {
    DEFAULT_LINES_PER_SECOND
}

/// A field of a configuration that breaks a rule; `field` is a dotted path (`controller.baud`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {reason}")]
pub struct ConfigError {
    /// The dotted path of the field.
    pub field: String,
    /// The rule, as a fact ("0 is not from 1 to 4,000,000").
    pub reason: String,
}

fn fault(field: &str, reason: String) -> ConfigError {
    ConfigError {
        field: field.to_owned(),
        reason,
    }
}

/// Checks a label or name: 1 to `max` characters, no control character.
///
/// # Errors
/// The broken rule, as a fact.
pub fn check_text(value: &str, max: usize) -> Result<(), String> {
    if value.is_empty() {
        return Err("is empty".to_owned());
    }
    if value.chars().count() > max {
        return Err(format!("has more than {max} characters"));
    }
    if value.chars().any(char::is_control) {
        return Err("has a control character".to_owned());
    }
    Ok(())
}

/// Checks a client name: 1 to [`MAX_CLIENT_NAME`] characters, no control character.
///
/// # Errors
/// The broken rule, as a fact.
pub fn check_client_name(name: &str) -> Result<(), String> {
    check_text(name, MAX_CLIENT_NAME)
}

fn check_range(field: &str, value: u32, low: u32, high: u32) -> Result<(), ConfigError> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(fault(
            field,
            format!(
                "{} is not from {} to {}",
                count(value),
                count(low),
                count(high)
            ),
        ))
    }
}

impl ConnectorConfig {
    /// Checks every rule of A16.
    ///
    /// # Errors
    /// The first field that breaks a rule.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.format != CONFIG_FORMAT {
            return Err(fault(
                "format",
                format!("{} is not {CONFIG_FORMAT}", self.format),
            ));
        }
        check_text(&self.name, MAX_LABEL).map_err(|r| fault("name", r))?;
        if let Some(owner) = &self.owner {
            check_text(owner, MAX_LABEL).map_err(|r| fault("owner", r))?;
        }
        if let Some(listen) = &self.listen
            && listen.parse::<SocketAddr>().is_err()
        {
            return Err(fault(
                "listen",
                format!("{listen} is not an address and port"),
            ));
        }
        self.controller.validate()
    }

    /// The listen address, when the configuration names a valid one.
    #[must_use]
    pub fn listen_addr(&self) -> Option<SocketAddr> {
        self.listen.as_deref().and_then(|l| l.parse().ok())
    }
}

impl ControllerConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        match self {
            Self::GrblSerial { port, baud } => {
                if port.is_empty() {
                    return Err(fault("controller.port", "is empty".to_owned()));
                }
                if port.len() > MAX_PORT_BYTES {
                    return Err(fault(
                        "controller.port",
                        format!("has more than {MAX_PORT_BYTES} bytes"),
                    ));
                }
                if port.chars().any(char::is_control) {
                    return Err(fault(
                        "controller.port",
                        "has a control character".to_owned(),
                    ));
                }
                check_range("controller.baud", *baud, 1, MAX_BAUD)
            }
            Self::Fake {
                lines_per_second,
                alarm_at_line,
            } => {
                check_range(
                    "controller.lines_per_second",
                    *lines_per_second,
                    1,
                    MAX_LINES_PER_SECOND,
                )?;
                match alarm_at_line {
                    Some(0) => Err(fault(
                        "controller.alarm_at_line",
                        "0 is not at least 1".to_owned(),
                    )),
                    _ => Ok(()),
                }
            }
        }
    }

    /// The controller in words: "GRBL on /dev/ttyUSB0, 115200 baud" or "Fake controller:
    /// nothing moves" (UX notes 4.3).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::GrblSerial { port, baud } => format!("GRBL on {port}, {baud} baud"),
            Self::Fake { .. } => "Fake controller: nothing moves".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake() -> ConnectorConfig {
        ConnectorConfig {
            format: 1,
            name: "workshop-laser".to_owned(),
            owner: Some("Marc".to_owned()),
            listen: None,
            controller: ControllerConfig::Fake {
                lines_per_second: 20,
                alarm_at_line: None,
            },
        }
    }

    #[test]
    fn serde_refuses_an_unknown_field_inside_a_tagged_variant() {
        let text = r#"{"format":1,"name":"a","controller":{"kind":"fake","pace":3}}"#;
        assert!(serde_json::from_str::<ConnectorConfig>(text).is_err());
        let text = r#"{"format":1,"name":"a","colour":"red","controller":{"kind":"fake"}}"#;
        assert!(serde_json::from_str::<ConnectorConfig>(text).is_err());
        let text = r#"{"format":1,"name":"a","controller":{"kind":"fake"}}"#;
        let config: ConnectorConfig = serde_json::from_str(text).unwrap();
        assert_eq!(
            config.controller,
            ControllerConfig::Fake {
                lines_per_second: 20,
                alarm_at_line: None
            }
        );
    }

    #[test]
    fn the_rules_name_the_field_and_the_value() {
        assert_eq!(fake().validate(), Ok(()));
        let mut c = fake();
        c.controller = ControllerConfig::GrblSerial {
            port: "/dev/ttyUSB0".into(),
            baud: 0,
        };
        assert_eq!(
            c.validate().unwrap_err().to_string(),
            "controller.baud: 0 is not from 1 to 4,000,000"
        );
        let mut c = fake();
        c.name = "x".repeat(65);
        assert_eq!(
            c.validate().unwrap_err().to_string(),
            "name: has more than 64 characters"
        );
        c.name = "x".repeat(64);
        assert_eq!(c.validate(), Ok(()));
        let mut c = fake();
        c.listen = Some("nowhere".into());
        assert_eq!(c.validate().unwrap_err().field, "listen");
        c.listen = Some("[::]:7447".into());
        assert_eq!(c.validate(), Ok(()));
        assert_eq!(c.listen_addr().unwrap().port(), 7447);
    }

    #[test]
    fn client_names_are_one_to_thirty_two_characters_without_control_characters() {
        assert!(check_client_name("Phone").is_ok());
        assert!(check_client_name(&"ä".repeat(32)).is_ok());
        assert!(check_client_name(&"a".repeat(33)).is_err());
        assert!(check_client_name("").is_err());
        assert!(check_client_name("a\tb").is_err());
    }

    #[test]
    fn the_controller_in_words() {
        let grbl = ControllerConfig::GrblSerial {
            port: "/dev/ttyUSB0".into(),
            baud: 115_200,
        };
        assert_eq!(grbl.describe(), "GRBL on /dev/ttyUSB0, 115200 baud");
        assert_eq!(
            fake().controller.describe(),
            "Fake controller: nothing moves"
        );
    }
}
