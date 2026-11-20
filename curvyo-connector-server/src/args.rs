//! The command line, parsed by hand (no `clap`, no environment variables): the service flags,
//! the run flags of foreground mode and the host-side commands (`specs/0128` A21, A28, A31).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use curvyo_connector_io::client::DEFAULT_PORT;
use curvyo_connector_io::grbl::HOMING_CAP;

/// The default pace of the fake controller.
pub const DEFAULT_LINES_PER_SECOND: u32 = 20;
/// The default baud rate of `--serial` (`specs/0127` G1).
pub const DEFAULT_BAUD: u32 = 115_200;
/// The highest baud rate (the profile field's range, `0026`).
pub const MAX_BAUD: u32 = 4_000_000;
/// The range of `--homing-timeout` in seconds (`specs/0142` HC2): 0 is refused, a cap that can
/// be switched off is no cap.
pub const HOMING_TIMEOUT_S: std::ops::RangeInclusive<u64> = 10..=3600;

/// What the program was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Serve.
    Run,
    /// Print the paired apps and exit.
    ListClients,
    /// Unpair the app of this name and exit.
    Revoke(String),
    /// Empty the state directory unless a connector holds it, and exit (A21).
    Reset,
    /// Print the key's fingerprint and exit (A21).
    Fingerprint,
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
    /// How long its `$H` may go unanswered.
    pub homing_timeout: Duration,
    /// `Start` waits for Enter on this terminal.
    pub require_local_start: bool,
    /// Print a new pairing code at start.
    pub pair: bool,
    /// The first run flag of foreground mode on the line, if any (A28).
    pub foreground_flag: Option<String>,
    /// The `--listen` value when one was given (A31: `connector.toml` wins over it).
    pub listen_flag: Option<SocketAddr>,
}

/// The run flags that make foreground mode (A28).
const FOREGROUND_FLAGS: [&str; 8] = [
    "--fake-controller",
    "--fake-lines-per-second",
    "--fake-alarm-at-line",
    "--serial",
    "--baud",
    "--homing-timeout",
    "--require-local-start",
    "--pair",
];

/// The help text: every flag.
pub const HELP: &str = "\
curvyo-connector-server: the machine connector, run next to the machine.

Usage: curvyo-connector-server [options]

Without run flags it serves as configured by <state dir>/connector.toml, or in maintenance mode
for 30 minutes after the start when there is none, set up with curvyoctl.

Options:
  --listen <addr:port>             where to listen (default 0.0.0.0:7447; connector.toml wins)
  --state-dir <dir>                configuration, key, paired apps and logs
                                   (default: the user's data directory, curvyo-connector)
  --fingerprint                    print the key fingerprint and exit
  --reset                          delete everything in the state directory and exit
  --list-clients                   print the paired apps and their roles and exit
  --revoke <name>                  unpair the app of this name and exit
  --help                           print this text and exit

Run flags (foreground mode, refused when connector.toml exists):
  --fake-controller                attach a fake controller: no machine, nothing moves
  --fake-lines-per-second <n>      its pace (default 20; needs --fake-controller)
  --fake-alarm-at-line <n>         raise an alarm at line n (needs --fake-controller)
  --serial <port>                  serial port of the GRBL controller, for example /dev/ttyUSB0
  --baud <n>                       baud rate, 1 to 4000000 (default 115200; needs --serial)
  --homing-timeout <seconds>       the controller counts as lost when homing ($H) gets no answer
                                   for this long; 10 to 3600 (default 120; needs --serial)
  --require-local-start            a job starts only after Enter on this terminal
  --pair                           print a new pairing code at start

In foreground mode: an empty line is the start press for a job that waits; the line `pair` prints a
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

fn homing_timeout(text: &str) -> Result<Duration, UsageError> {
    text.parse::<u64>()
        .ok()
        .filter(|s| HOMING_TIMEOUT_S.contains(s))
        .map(Duration::from_secs)
        .ok_or_else(|| {
            UsageError(format!(
                "--homing-timeout must be 10 to 3600 seconds, got {text}"
            ))
        })
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
        homing_timeout: HOMING_CAP,
        require_local_start: false,
        pair: false,
        foreground_flag: None,
        listen_flag: None,
    };
    let mut state_dir = None;
    let mut fake_option = None;
    let mut baud_given = false;
    let mut homing_given = false;
    while let Some(flag) = args.next() {
        if parsed.foreground_flag.is_none() && FOREGROUND_FLAGS.contains(&flag.as_str()) {
            parsed.foreground_flag = Some(flag.clone());
        }
        match flag.as_str() {
            "--help" | "-h" => parsed.command = Command::Help,
            "--listen" => {
                let text = value(&mut args, &flag)?;
                parsed.listen = text
                    .parse()
                    .map_err(|_| UsageError(format!("--listen needs addr:port, not {text}")))?;
                parsed.listen_flag = Some(parsed.listen);
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
            "--homing-timeout" => {
                parsed.homing_timeout = homing_timeout(&value(&mut args, &flag)?)?;
                homing_given = true;
            }
            "--require-local-start" => parsed.require_local_start = true,
            "--pair" => parsed.pair = true,
            "--list-clients" => parsed.command = Command::ListClients,
            "--reset" => parsed.command = Command::Reset,
            "--fingerprint" => parsed.command = Command::Fingerprint,
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
    if homing_given && parsed.serial.is_none() {
        return Err(UsageError("--homing-timeout needs --serial".to_owned()));
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
    fn hc2_homing_timeout() {
        assert_eq!(
            parse_args(&["--serial", "x"]).unwrap().homing_timeout,
            Duration::from_secs(120)
        );
        for (value, secs) in [("10", 10), ("3600", 3600), ("120", 120)] {
            let args = parse_args(&["--serial", "x", "--homing-timeout", value]).unwrap();
            assert_eq!(args.homing_timeout, Duration::from_secs(secs), "{value}");
        }
        for value in ["0", "9", "3601", "abc", "-5", "10.5", ""] {
            assert_eq!(
                parse_args(&["--serial", "x", "--homing-timeout", value])
                    .unwrap_err()
                    .0,
                format!("--homing-timeout must be 10 to 3600 seconds, got {value}"),
                "{value}"
            );
        }
        assert_eq!(
            parse_args(&["--homing-timeout", "60"]).unwrap_err().0,
            "--homing-timeout needs --serial"
        );
        assert_eq!(
            parse_args(&["--serial", "x", "--homing-timeout"])
                .unwrap_err()
                .0,
            "--homing-timeout needs a value"
        );
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
            "--homing-timeout <seconds>",
            "default 120",
            "10 to 3600",
            "dialout",
            "uucp",
            "ModemManager",
        ] {
            assert!(HELP.contains(flag), "{flag}");
        }
    }

    #[test]
    fn a21_a28_host_commands_and_the_first_run_flag() {
        assert_eq!(parse_args(&["--reset"]).unwrap().command, Command::Reset);
        assert_eq!(
            parse_args(&["--fingerprint"]).unwrap().command,
            Command::Fingerprint
        );
        let args = parse_args(&["--listen", "127.0.0.1:0"]).unwrap();
        assert_eq!(args.foreground_flag, None);
        assert_eq!(args.listen_flag, Some(args.listen));
        let args = parse_args(&["--require-local-start", "--fake-controller"]).unwrap();
        assert_eq!(
            args.foreground_flag.as_deref(),
            Some("--require-local-start")
        );
        assert_eq!(args.listen_flag, None);
        for flag in [
            "--reset",
            "--fingerprint",
            "connector.toml",
            "maintenance mode",
        ] {
            assert!(HELP.contains(flag), "{flag}");
        }
        for line in HELP.lines() {
            assert!(line.chars().count() <= 100, "{line}");
        }
    }
}
