//! The words and layouts of `curvyoctl`'s output (UX notes sections 1 to 6): help, usage lines,
//! labelled lines, columns, times and fingerprints.

use curvyo_connector_core::{Role, count};
use std::fmt::Write as _;

/// `curvyoctl --help` (UX notes section 6).
pub const HELP: &str = "\
curvyoctl: set up and manage Curvyo connectors from this computer.

Usage: curvyoctl <command> [<connector>] [options]

Setup of a connector in maintenance mode:
  identify --insecure <addr>         name, software, mode and key
  ports --insecure <addr>            serial ports of the connector's host
  apply-config --insecure <addr> -f <file> [--fingerprint <key>]
                                     configure it; this computer becomes owner

A configured connector (owner):
  identify                           name, software, mode and key
  ports                              serial ports of the connector's host
  apply-config -f <file>             change the configuration
  get-config                         print the configuration as TOML
  status                             connector, controller and job
  confirm-start [<job>]              run the job whose start a member requested
  stop                               stop or discard the job
  unlock                             clear the controller's alarm
  jobs [--last <n>]                  the job log (default 20 lines)
  audit [--last <n>]                 the admin log (default 20 lines)
  pair --name <name> [--owner]       pairing code for an app, member or owner
  clients                            the paired apps and their roles
  revoke <name>                      unpair a client
  reset --confirm <name>             delete all its state and stop it

A second computer:
  login <addr> --code <code>         become an owner with a pair --owner code
  logout                             remove the connector from this computer
  whoami                             this computer's name, role and the key

Other:
  version                            this program's version and protocol
  --help                             this text; after a command, its own text

<connector>  a name from this computer's list; optional while it holds one
<key>        a key fingerprint: 32 hex characters, spaces or hyphens allowed
Exit codes: 0 done, 1 refused or not reachable, 2 usage.
";

/// One command's usage line, sentence and options (UX notes section 6).
pub struct CommandHelp {
    /// The command.
    pub name: &'static str,
    /// `Usage: curvyoctl ...`.
    pub usage: &'static str,
    /// What it does, one sentence.
    pub sentence: &'static str,
    /// Its options, one line each.
    pub options: &'static [&'static str],
}

/// Every command's own help.
pub const COMMANDS: [CommandHelp; 18] = [
    CommandHelp {
        name: "identify",
        usage: "Usage: curvyoctl identify [<connector>]\n       curvyoctl identify --insecure <addr>",
        sentence: "Print the connector's name, software, mode and key.",
        options: &["--insecure <addr>   a connector this computer has no entry for"],
    },
    CommandHelp {
        name: "ports",
        usage: "Usage: curvyoctl ports [<connector>]\n       curvyoctl ports --insecure <addr>",
        sentence: "Print the serial ports of the connector's host.",
        options: &["--insecure <addr>   a connector in maintenance mode"],
    },
    CommandHelp {
        name: "apply-config",
        usage: "Usage: curvyoctl apply-config [<connector>] -f <file>\n       curvyoctl apply-config --insecure <addr> -f <file> [--fingerprint <key>]",
        sentence: "Configure a connector; in maintenance mode this computer becomes its owner.",
        options: &[
            "-f <file>             the configuration, TOML",
            "--insecure <addr>     a connector in maintenance mode",
            "--fingerprint <key>   the key the connector must present",
        ],
    },
    CommandHelp {
        name: "get-config",
        usage: "Usage: curvyoctl get-config [<connector>]",
        sentence: "Print the configuration as TOML.",
        options: &[],
    },
    CommandHelp {
        name: "status",
        usage: "Usage: curvyoctl status [<connector>]",
        sentence: "Print the connector, its controller and its job.",
        options: &[],
    },
    CommandHelp {
        name: "confirm-start",
        usage: "Usage: curvyoctl confirm-start [<connector>] [<job>]",
        sentence: "Run the job whose start or resume a member requested.",
        options: &["<job>   the job's name, checked before the start"],
    },
    CommandHelp {
        name: "stop",
        usage: "Usage: curvyoctl stop [<connector>]",
        sentence: "Stop the running job or discard the held one.",
        options: &[],
    },
    CommandHelp {
        name: "unlock",
        usage: "Usage: curvyoctl unlock [<connector>]",
        sentence: "Clear the controller's alarm.",
        options: &[],
    },
    CommandHelp {
        name: "jobs",
        usage: "Usage: curvyoctl jobs [<connector>] [--last <n>]",
        sentence: "Print the last lines of the job log.",
        options: &["--last <n>   how many lines, 1 to 1000 (default 20)"],
    },
    CommandHelp {
        name: "audit",
        usage: "Usage: curvyoctl audit [<connector>] [--last <n>]",
        sentence: "Print the last lines of the admin log.",
        options: &["--last <n>   how many lines, 1 to 1000 (default 20)"],
    },
    CommandHelp {
        name: "pair",
        usage: "Usage: curvyoctl pair [<connector>] --name <name> [--owner]",
        sentence: "Print a pairing code; the app becomes a member, or an owner with --owner.",
        options: &[
            "--name <name>   the app's name in clients and logs",
            "--owner         the app becomes an owner",
        ],
    },
    CommandHelp {
        name: "clients",
        usage: "Usage: curvyoctl clients [<connector>]",
        sentence: "Print the paired apps and their roles.",
        options: &[],
    },
    CommandHelp {
        name: "revoke",
        usage: "Usage: curvyoctl revoke [<connector>] <name>",
        sentence: "Unpair a client and close its connections.",
        options: &[],
    },
    CommandHelp {
        name: "reset",
        usage: "Usage: curvyoctl reset [<connector>] --confirm <connector name>",
        sentence: "Stop the connector's job, delete all its state and stop it.",
        options: &["--confirm <name>   the connector's name, typed in full"],
    },
    CommandHelp {
        name: "login",
        usage: "Usage: curvyoctl login <addr> --code <code>",
        sentence: "Pair this computer as an owner with a code from pair --owner.",
        options: &["--code <code>   the pairing code"],
    },
    CommandHelp {
        name: "logout",
        usage: "Usage: curvyoctl logout [<connector>]",
        sentence: "Remove the connector and its token from this computer.",
        options: &[],
    },
    CommandHelp {
        name: "whoami",
        usage: "Usage: curvyoctl whoami [<connector>]",
        sentence: "Print this computer's name and role on the connector, and its key.",
        options: &[],
    },
    CommandHelp {
        name: "version",
        usage: "Usage: curvyoctl version",
        sentence: "Print this program's version and protocol.",
        options: &[],
    },
];

/// The help of `command`.
#[must_use]
pub fn command_help(command: &str) -> Option<&'static CommandHelp> {
    COMMANDS.iter().find(|c| c.name == command)
}

/// The usage line of `command`.
#[must_use]
pub fn usage_of(command: &str) -> Option<&'static str> {
    command_help(command).map(|c| c.usage)
}

/// Labelled lines: values start one space after the longest label.
#[must_use]
pub fn labelled(rows: &[(&str, String)]) -> String {
    let width = rows.iter().map(|(l, _)| l.len() + 1).max().unwrap_or(0);
    rows.iter().fold(String::new(), |mut text, (label, value)| {
        let _ = writeln!(text, "{:<width$} {value}", format!("{label}:"));
        text
    })
}

/// Columns: a header, one row per entry, at least two spaces between columns, the last free.
#[must_use]
pub fn columns(header: &[&str], rows: &[Vec<String>]) -> String {
    let n = header.len();
    let mut widths: Vec<usize> = header.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(n.saturating_sub(1)) {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let line = |cells: Vec<&str>| {
        let mut text = String::new();
        for (i, cell) in cells.iter().enumerate() {
            if i + 1 == cells.len() {
                text.push_str(cell);
            } else {
                let pad = widths[i] + 2 - cell.chars().count();
                text.push_str(cell);
                text.push_str(&" ".repeat(pad));
            }
        }
        format!("{}\n", text.trim_end())
    };
    let mut text = line(header.to_vec());
    for row in rows {
        text.push_str(&line(row.iter().map(String::as_str).collect()));
    }
    text
}

/// `2026-11-18T14:06:11Z` as `2026-11-18 14:06:11`.
#[must_use]
pub fn log_time(rfc3339: &str) -> String {
    rfc3339.trim_end_matches('Z').replacen('T', " ", 1)
}

/// `owner` or `member`.
#[must_use]
pub fn role_word(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Member => "member",
    }
}

/// "28 min", or "under 1 min" below a minute.
#[must_use]
pub fn minutes(seconds: u64) -> String {
    if seconds < 60 {
        "under 1 min".to_owned()
    } else {
        format!("{} min", seconds / 60)
    }
}

/// "m:ss" under an hour, "h:mm:ss" from an hour.
#[must_use]
pub fn duration(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// A count with a thousands comma.
#[must_use]
pub fn number(n: u32) -> String {
    count(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fixed_text_fits_eighty_columns() {
        for line in HELP.lines() {
            assert!(line.len() <= 80, "{line}");
            assert!(line.is_ascii(), "{line}");
        }
        for command in &COMMANDS {
            let lines = command
                .usage
                .lines()
                .chain(std::iter::once(command.sentence));
            for line in lines.chain(command.options.iter().copied()) {
                assert!(line.len() <= 80, "{line}");
            }
        }
    }

    #[test]
    fn labels_and_columns_line_up() {
        let text = labelled(&[("Name", "a".into()), ("Software", "b".into())]);
        assert_eq!(text, "Name:     a\nSoftware: b\n");
        let table = columns(
            &["Name", "Role", "Paired (UTC)"],
            &[vec![
                "Laptop".into(),
                "owner".into(),
                "2026-11-18 14:09".into(),
            ]],
        );
        assert_eq!(
            table,
            "Name    Role   Paired (UTC)\nLaptop  owner  2026-11-18 14:09\n"
        );
        assert_eq!(log_time("2026-11-18T14:06:11Z"), "2026-11-18 14:06:11");
        assert_eq!(
            (minutes(1680), minutes(59)),
            ("28 min".into(), "under 1 min".into())
        );
        assert_eq!(
            (duration(42_000), duration(3_723_000)),
            ("0:42".into(), "1:02:03".into())
        );
    }
}
