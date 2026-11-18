//! The command line, parsed by hand (no `clap`, no environment variables, no config file in this
//! slice).

use std::net::SocketAddr;
use std::path::PathBuf;

use curvyo_connector_io::client::DEFAULT_PORT;

/// The default pace of the fake controller.
pub const DEFAULT_LINES_PER_SECOND: u32 = 20;
/// The default baud rate of `--serial` (`specs/0127` G1).
pub const DEFAULT_BAUD: u32 = 115_200;
/// The highest baud rate (the profile field's range, `0026`).
pub const MAX_BAUD: u32 = 4_000_000;

/// What the program was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Serve.
    Run,
    /// Print the paired apps and exit.
    ListClients,
    /// Unpair the app of this name and exit.
    Revoke(String),
    /// Print the help and exit.
    Help,
}

/// The parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// What to do.
    pub command: Command,
    /// Where to listen.
    pub listen: SocketAddr,
    /// The state directory.
    pub state_dir: PathBuf,
    /// Attach the fake controller.
    pub fake_controller: bool,
    /// Its pace.
    pub fake_lines_per_second: u32,
    /// Its alarm line.
    pub fake_alarm_at_line: Option<u32>,
    /// The serial port of a GRBL controller.
    pub serial: Option<String>,
    /// Its baud rate.
    pub baud: u32,
    /// `Start` waits for Enter on this terminal.
    pub require_local_start: bool,
    /// Print a new pairing code at start.
    pub pair: bool,
}

/// The help text: every flag.
pub const HELP: &str = "\
curvyo-connector-server: the machine connector, run next to the machine.

Usage: curvyo-connector-server [options]

Options:
  --listen <addr:port>             where to listen (default 0.0.0.0:7447)
  --state-dir <dir>                key, paired apps and job log
                                   (default: the user's data directory, curvyo-connector)
  --fake-controller                attach a fake controller: no machine, nothing moves
  --fake-lines-per-second <n>      its pace (default 20; needs --fake-controller)
  --fake-alarm-at-line <n>         raise an alarm at line n (needs --fake-controller)
  --serial <port>                  serial port of the GRBL controller, for example /dev/ttyUSB0
  --baud <n>                       baud rate, 1 to 4000000 (default 115200; needs --serial)
  --require-local-start            a job starts only after Enter on this terminal
  --pair                           print a new pairing code at start
  --list-clients                   print the paired apps and exit
  --revoke <name>                  unpair the app of this name and exit
  --help                           print this text and exit

While running: an empty line is the start press for a job that waits; the line `pair` prints a
new pairing code. A GRBL alarm lock is left with Unlock in the app.

Serial ports on Linux: the user must be in the group that owns the port (dialout on Debian, Ubuntu
and Fedora, uucp on Arch; log in again after adding it) or a udev rule must grant access.
ModemManager may probe ttyACM and ttyUSB devices and disturb the controller; the connector says so
at start when it runs. See docs/guides/connector-serial-linux.md.

SIGINT and SIGTERM stop a running job (feed hold, reset, M5, M9) before the connector exits.
";

/// A command line that cannot be used; the program exits with code 2.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct UsageError(pub String);

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, UsageError> {
    args.next()
        .ok_or_else(|| UsageError(format!("{flag} needs a value")))
}

fn number(text: &str, flag: &str) -> Result<u32, UsageError> {
    text.parse::<u32>()
        .ok()
        .filter(|n| *n >= 1)
        .ok_or_else(|| UsageError(format!("{flag} needs a whole number from 1")))
}

/// Parses the arguments after the program name. `default_state_dir` is the user's data directory
/// when there is one.
///
/// # Errors
/// [`UsageError`] for an unknown flag, a missing or bad value, or flags that do not fit together.
pub fn parse(
    mut args: impl Iterator<Item = String>,
    default_state_dir: Option<PathBuf>,
) -> Result<Args, UsageError> {
    let mut parsed = Args {
        command: Command::Run,
        listen: SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)),
        state_dir: PathBuf::new(),
        fake_controller: false,
        fake_lines_per_second: DEFAULT_LINES_PER_SECOND,
        fake_alarm_at_line: None,
        serial: None,
        baud: DEFAULT_BAUD,
        require_local_start: false,
        pair: false,
    };
    let mut state_dir = None;
    let mut fake_option = None;
    let mut baud_given = false;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--help" | "-h" => parsed.command = Command::Help,
            "--listen" => {
                let text = value(&mut args, &flag)?;
                parsed.listen = text
                    .parse()
                    .map_err(|_| UsageError(format!("--listen needs addr:port, not {text}")))?;
            }
            "--state-dir" => state_dir = Some(PathBuf::from(value(&mut args, &flag)?)),
            "--fake-controller" => parsed.fake_controller = true,
            "--fake-lines-per-second" => {
                parsed.fake_lines_per_second = number(&value(&mut args, &flag)?, &flag)?;
                fake_option = Some(flag);
            }
            "--fake-alarm-at-line" => {
                parsed.fake_alarm_at_line = Some(number(&value(&mut args, &flag)?, &flag)?);
                fake_option = Some(flag);
            }
            "--serial" => parsed.serial = Some(value(&mut args, &flag)?),
            "--baud" => {
                let text = value(&mut args, &flag)?;
                parsed.baud = text
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=MAX_BAUD).contains(n))
                    .ok_or_else(|| UsageError("--baud must be 1 to 4000000".to_owned()))?;
                baud_given = true;
            }
            "--require-local-start" => parsed.require_local_start = true,
            "--pair" => parsed.pair = true,
            "--list-clients" => parsed.command = Command::ListClients,
            "--revoke" => parsed.command = Command::Revoke(value(&mut args, &flag)?),
            other => return Err(UsageError(format!("unknown option {other}"))),
        }
    }
    if parsed.serial.is_some() && parsed.fake_controller {
        return Err(UsageError(
            "--serial and --fake-controller cannot be used together".to_owned(),
        ));
    }
    if baud_given && parsed.serial.is_none() {
        return Err(UsageError("--baud needs --serial".to_owned()));
    }
    if let Some(flag) = fake_option.filter(|_| !parsed.fake_controller) {
        return Err(UsageError(format!("{flag} needs --fake-controller")));
    }
    parsed.state_dir = state_dir
        .or(default_state_dir)
        .ok_or_else(|| UsageError("no data directory found; give --state-dir".to_owned()))?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Args, UsageError> {
        parse(
            args.iter().map(|a| (*a).to_owned()),
            Some(PathBuf::from("/data/curvyo-connector")),
        )
    }

    #[test]
    fn the_defaults() {
        let args = parse_args(&[]).unwrap();
        assert_eq!(args.command, Command::Run);
        assert_eq!(args.listen.port(), 7447);
        assert_eq!(args.state_dir, PathBuf::from("/data/curvyo-connector"));
        assert!(!args.fake_controller && !args.require_local_start && !args.pair);
        assert_eq!(args.fake_lines_per_second, 20);
    }

    #[test]
    fn every_flag_is_read() {
        let args = parse_args(&[
            "--listen",
            "127.0.0.1:0",
            "--state-dir",
            "/tmp/x",
            "--fake-controller",
            "--fake-lines-per-second",
            "500",
            "--fake-alarm-at-line",
            "7",
            "--require-local-start",
            "--pair",
        ])
        .unwrap();
        assert_eq!(args.listen.to_string(), "127.0.0.1:0");
        assert_eq!(args.state_dir, PathBuf::from("/tmp/x"));
        assert!(args.fake_controller && args.require_local_start && args.pair);
        assert_eq!(
            (args.fake_lines_per_second, args.fake_alarm_at_line),
            (500, Some(7))
        );
        assert_eq!(
            parse_args(&["--revoke", "Laptop"]).unwrap().command,
            Command::Revoke("Laptop".into())
        );
        assert_eq!(
            parse_args(&["--list-clients"]).unwrap().command,
            Command::ListClients
        );
        assert_eq!(parse_args(&["--help"]).unwrap().command, Command::Help);
    }

    #[test]
    fn a_bad_line_is_a_usage_error() {
        for bad in [
            &["--nope"][..],
            &["--listen"],
            &["--listen", "x"],
            &["--fake-alarm-at-line", "3"],
            &["--fake-controller", "--fake-lines-per-second", "0"],
            &["--revoke"],
        ] {
            assert!(parse_args(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn g1_serial_and_baud() {
        let args = parse_args(&["--serial", "/dev/ttyUSB0"]).unwrap();
        assert_eq!(
            (args.serial.as_deref(), args.baud),
            (Some("/dev/ttyUSB0"), 115_200)
        );
        let args = parse_args(&["--serial", "/dev/ttyACM0", "--baud", "4000000"]).unwrap();
        assert_eq!(args.baud, 4_000_000);
        assert_eq!(
            parse_args(&["--serial", "x", "--baud", "1"]).unwrap().baud,
            1
        );
        assert!(parse_args(&[]).unwrap().serial.is_none());
        let table = [
            (
                &["--serial", "x", "--fake-controller"][..],
                "--serial and --fake-controller cannot be used together",
            ),
            (&["--baud", "9600"], "--baud needs --serial"),
            (
                &["--serial", "x", "--baud", "0"],
                "--baud must be 1 to 4000000",
            ),
            (
                &["--serial", "x", "--baud", "4000001"],
                "--baud must be 1 to 4000000",
            ),
            (
                &["--serial", "x", "--baud", "fast"],
                "--baud must be 1 to 4000000",
            ),
            (&["--serial"], "--serial needs a value"),
        ];
        for (line, message) in table {
            assert_eq!(parse_args(line).unwrap_err().0, message, "{line:?}");
        }
    }

    #[test]
    fn the_help_names_every_flag() {
        for flag in [
            "--listen",
            "--state-dir",
            "--fake-controller",
            "--fake-lines-per-second",
            "--fake-alarm-at-line",
            "--require-local-start",
            "--pair",
            "--list-clients",
            "--revoke",
            "--help",
            "--serial",
            "--baud",
            "dialout",
            "uucp",
            "ModemManager",
        ] {
            assert!(HELP.contains(flag), "{flag}");
        }
    }
}
