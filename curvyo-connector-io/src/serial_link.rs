//! The serial port of the GRBL controller (addendum C9): one thread opens it, retries while it
//! is absent, reads and writes it and hands the bytes to the hub. The only module that touches a
//! port.

use std::io::{ErrorKind as IoKind, Read as _, Write as _};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serialport::{ErrorKind, SerialPort};

use crate::grbl::controller::SAFETY;
use crate::hub::{Hub, SerialSettings};

/// How long a read waits for bytes: how often the thread turns.
const READ_TIMEOUT: Duration = Duration::from_millis(10);

/// The app's words and the terminal's line for a port that would not open (UX notes 4, 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenProblem {
    /// For `Status.detail`.
    pub detail: String,
    /// For the terminal.
    pub notice: String,
}

/// The words for an open error of `port`; `group` explains a permission problem.
#[must_use]
pub fn open_problem(port: &str, error: &serialport::Error, group: Option<String>) -> OpenProblem {
    let (detail, why) = match error.kind() {
        ErrorKind::Io(IoKind::NotFound) => {
            (format!("Port {port} not found"), "not found".to_owned())
        }
        ErrorKind::Io(IoKind::PermissionDenied) => (
            format!("No permission for {port}"),
            match group {
                Some(group) => format!("permission denied ({group})"),
                None => "permission denied".to_owned(),
            },
        ),
        ErrorKind::NoDevice => (
            format!("Port {port} is in use"),
            "in use by another program".to_owned(),
        ),
        _ => (format!("Cannot open {port}"), error.description.clone()),
    };
    OpenProblem {
        detail,
        notice: format!("Cannot open {port}: {why}"),
    }
}

/// "group uucp, this user is not a member" from the port's group id, `/etc/group` and
/// `/proc/self/status` (G27). The name falls back to the number.
#[must_use]
pub fn group_note(gid: u32, group_file: &str, proc_status: &str) -> String {
    let name = group_file
        .lines()
        .find_map(|l| {
            let mut parts = l.split(':');
            let name = parts.next()?;
            let id: u32 = parts.nth(1)?.parse().ok()?;
            (id == gid).then(|| name.to_owned())
        })
        .unwrap_or_else(|| gid.to_string());
    let ids = |key: &str| -> Vec<u32> {
        proc_status
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .map(|rest| {
                rest.split_whitespace()
                    .filter_map(|n| n.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    };
    let member = ids("Groups:").contains(&gid) || ids("Gid:").contains(&gid);
    if member {
        format!("group {name}")
    } else {
        format!("group {name}, this user is not a member")
    }
}

#[cfg(unix)]
fn port_group(port: &str) -> Option<String> {
    use std::os::unix::fs::MetadataExt as _;
    let gid = std::fs::metadata(Path::new(port)).ok()?.gid();
    let groups = std::fs::read_to_string("/etc/group").unwrap_or_default();
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    Some(group_note(gid, &groups, &status))
}

#[cfg(not(unix))]
fn port_group(_port: &str) -> Option<String> {
    None
}

fn open(settings: &SerialSettings) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(&settings.port, settings.baud)
        .timeout(READ_TIMEOUT)
        .open()
}

struct Link {
    settings: SerialSettings,
    hub: Arc<Hub>,
    port: Option<Box<dyn SerialPort>>,
    next_try: Instant,
    last_problem: Option<String>,
    said_retry: bool,
}

impl Link {
    fn try_open(&mut self, now: Instant) {
        if now < self.next_try {
            std::thread::sleep(READ_TIMEOUT.min(self.next_try - now));
            return;
        }
        self.next_try = now + self.settings.timing.retry;
        match open(&self.settings) {
            Ok(port) => {
                self.last_problem = None;
                self.said_retry = false;
                self.hub.attach_serial(&self.settings, now);
                self.port = Some(port);
            }
            Err(e) => {
                let group = (e.kind() == ErrorKind::Io(IoKind::PermissionDenied))
                    .then(|| port_group(&self.settings.port))
                    .flatten();
                let problem = open_problem(&self.settings.port, &e, group);
                self.hub.open_failed(problem.detail, now);
                if self.last_problem.as_ref() != Some(&problem.notice) {
                    self.hub.notice(problem.notice.clone());
                    self.last_problem = Some(problem.notice);
                }
                if !self.said_retry {
                    let secs = self.settings.timing.retry.as_secs_f64();
                    self.hub.notice(format!(
                        "Trying {} again every {secs} s",
                        self.settings.port
                    ));
                    self.said_retry = true;
                }
            }
        }
    }

    /// The port failed: the hub loses the controller, the safety bytes are tried once.
    fn fail(&mut self, error: &std::io::Error, now: Instant) {
        self.hub.lose(format!("Lost the controller: {error}"), now);
        if let Some(mut port) = self.port.take() {
            let _ = port.write_all(SAFETY);
        }
        self.next_try = now + self.settings.timing.retry;
    }

    fn turn(&mut self, buf: &mut [u8]) {
        let Some(port) = self.port.as_mut() else {
            self.try_open(Instant::now());
            return;
        };
        let read = match port.read(buf) {
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    IoKind::TimedOut | IoKind::WouldBlock | IoKind::Interrupted
                ) =>
            {
                0
            }
            Err(e) => return self.fail(&e, Instant::now()),
        };
        let now = Instant::now();
        let exchange = self.hub.exchange(&buf[..read], now);
        if !exchange.write.is_empty()
            && let Err(e) = port.write_all(&exchange.write).and_then(|()| port.flush())
        {
            return self.fail(&e, now);
        }
        if exchange.close {
            self.port = None;
            self.next_try = now + self.settings.timing.retry;
        }
    }
}

/// Starts the thread that owns the port of `settings` until `shutdown` is set.
#[must_use]
pub fn spawn(settings: SerialSettings, hub: Arc<Hub>, shutdown: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut link = Link {
            settings,
            hub,
            port: None,
            next_try: Instant::now(),
            last_problem: None,
            said_retry: false,
        };
        let mut buf = [0_u8; 1024];
        while !shutdown.load(Ordering::Relaxed) {
            link.turn(&mut buf);
        }
        if let Some(mut port) = link.port.take() {
            let _ = port.flush();
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_errors_name_the_port_and_the_cause() {
        let port = "/dev/ttyUSB0";
        let err = |kind| serialport::Error::new(kind, "x");
        let p = open_problem(port, &err(ErrorKind::Io(IoKind::NotFound)), None);
        assert_eq!(p.detail, "Port /dev/ttyUSB0 not found");
        assert_eq!(p.notice, "Cannot open /dev/ttyUSB0: not found");
        let p = open_problem(
            port,
            &err(ErrorKind::Io(IoKind::PermissionDenied)),
            Some("group uucp, this user is not a member".into()),
        );
        assert_eq!(p.detail, "No permission for /dev/ttyUSB0");
        assert_eq!(
            p.notice,
            "Cannot open /dev/ttyUSB0: permission denied (group uucp, this user is not a member)"
        );
        let p = open_problem(port, &err(ErrorKind::NoDevice), None);
        assert_eq!(p.detail, "Port /dev/ttyUSB0 is in use");
        assert_eq!(
            p.notice,
            "Cannot open /dev/ttyUSB0: in use by another program"
        );
        let p = open_problem(port, &err(ErrorKind::Unknown), None);
        assert_eq!(p.detail, "Cannot open /dev/ttyUSB0");
    }

    #[test]
    fn the_group_of_the_port_and_whether_the_user_is_in_it() {
        let groups = "root:x:0:\nuucp:x:14:\ndialout:x:20:marc\n";
        let status = "Name:\tx\nGid:\t1000\t1000\t1000\t1000\nGroups:\t10 998 1000\n";
        assert_eq!(
            group_note(14, groups, status),
            "group uucp, this user is not a member"
        );
        assert_eq!(
            group_note(20, groups, "Gid:\t20\t20\t20\t20\nGroups:\t\n"),
            "group dialout"
        );
        assert_eq!(
            group_note(77, groups, status),
            "group 77, this user is not a member"
        );
    }
}
