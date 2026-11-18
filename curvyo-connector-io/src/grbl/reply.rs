//! Reads one line of GRBL 1.1 output: answers, alarms, the banner, status reports, settings and
//! messages (`specs/0127` G2, G4).

/// One line GRBL printed.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// `ok`.
    Ok,
    /// `error:<n>`.
    Error(u16),
    /// `ALARM:<n>`.
    Alarm(u16),
    /// `Grbl 1.1h ['$' for help]`: the version.
    Banner(String),
    /// `<Idle|MPos:...>`.
    Report(Report),
    /// `$<n>=<value>`.
    Setting(u16, String),
    /// `[MSG:...]`: the text inside.
    Message(String),
    /// Anything else (`[VER:...]`, `[OPT:...]`, text of another program).
    Other(String),
}

/// A status report.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// The state word: `Idle`, `Run`, `Hold`, `Jog`, `Alarm`, `Door`, `Check`, `Home`, `Sleep`.
    pub state: String,
    /// The number after the state (`Hold:0`).
    pub sub: Option<u8>,
    /// Machine position.
    pub mpos: Option<[f64; 3]>,
    /// Work position.
    pub wpos: Option<[f64; 3]>,
    /// Work coordinate offset (sent now and then).
    pub wco: Option<[f64; 3]>,
    /// Accessory state (`A:SFM`): spindle and coolant; empty when absent.
    pub accessories: String,
    /// Free bytes of GRBL's receive buffer, the second number of `Bf:` (only when `$10` asks).
    pub rx_free: Option<usize>,
}

impl Report {
    /// Whether this is a completed hold (`Hold:0`).
    #[must_use]
    pub fn hold_complete(&self) -> bool {
        self.state == "Hold" && self.sub == Some(0)
    }

    /// Whether the accessory field shows the spindle (the laser) on, `S` or `C`.
    #[must_use]
    pub fn spindle_on(&self) -> bool {
        self.accessories.contains(['S', 'C'])
    }
}

fn axes(text: &str) -> Option<[f64; 3]> {
    let mut out = [0.0; 3];
    for (i, part) in text.split(',').enumerate() {
        let value: f64 = part.parse().ok()?;
        if let Some(slot) = out.get_mut(i) {
            *slot = value;
        }
    }
    Some(out)
}

fn report(inner: &str) -> Option<Report> {
    let mut fields = inner.split('|');
    let head = fields.next()?;
    let (state, sub) = match head.split_once(':') {
        Some((state, sub)) => (state, sub.parse().ok()),
        None => (head, None),
    };
    if state.is_empty() || !state.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut out = Report {
        state: state.to_owned(),
        sub,
        ..Report::default()
    };
    for field in fields {
        let Some((key, value)) = field.split_once(':') else {
            continue;
        };
        match key {
            "MPos" => out.mpos = axes(value),
            "WPos" => out.wpos = axes(value),
            "WCO" => out.wco = axes(value),
            "A" => value.clone_into(&mut out.accessories),
            "Bf" => out.rx_free = value.split(',').nth(1).and_then(|n| n.parse().ok()),
            _ => {}
        }
    }
    Some(out)
}

fn banner(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("GrblHAL ")
        .or_else(|| line.strip_prefix("Grbl "))?;
    let version = rest.split_whitespace().next()?;
    version
        .starts_with(|c: char| c.is_ascii_digit())
        .then(|| version.to_owned())
}

/// Reads one line, without its line end.
#[must_use]
pub fn parse(line: &str) -> Reply {
    let line = line.trim_end_matches(['\r', '\n']);
    if line == "ok" {
        return Reply::Ok;
    }
    if let Some(n) = line.strip_prefix("error:").and_then(|n| n.parse().ok()) {
        return Reply::Error(n);
    }
    if let Some(n) = line.strip_prefix("ALARM:").and_then(|n| n.parse().ok()) {
        return Reply::Alarm(n);
    }
    if let Some(r) = line
        .strip_prefix('<')
        .and_then(|l| l.strip_suffix('>'))
        .and_then(report)
    {
        return Reply::Report(r);
    }
    if let Some(text) = line.strip_prefix("[MSG:").and_then(|l| l.strip_suffix(']')) {
        return Reply::Message(text.to_owned());
    }
    if let Some((n, value)) = line
        .strip_prefix('$')
        .and_then(|l| l.split_once('='))
        .and_then(|(n, v)| Some((n.parse().ok()?, v)))
    {
        return Reply::Setting(n, value.to_owned());
    }
    if let Some(version) = banner(line) {
        return Reply::Banner(version);
    }
    Reply::Other(line.to_owned())
}

/// Splits bytes read from the port into lines; a partial line waits for its end.
#[derive(Debug, Clone, Default)]
pub struct Lines {
    partial: Vec<u8>,
}

/// The longest line kept; GRBL never prints more, so a longer one is cut (another program).
const MAX_REPLY: usize = 256;

impl Lines {
    /// The complete lines in `bytes`, without their line ends; empty lines are dropped.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        for &b in bytes {
            if b == b'\n' {
                let text = String::from_utf8_lossy(&self.partial)
                    .trim_end_matches('\r')
                    .to_owned();
                self.partial.clear();
                if !text.is_empty() {
                    out.push(text);
                }
            } else if self.partial.len() < MAX_REPLY {
                self.partial.push(b);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_alarms_messages_and_settings() {
        assert_eq!(parse("ok"), Reply::Ok);
        assert_eq!(parse("ok\r"), Reply::Ok);
        assert_eq!(parse("error:22"), Reply::Error(22));
        assert_eq!(parse("ALARM:1"), Reply::Alarm(1));
        assert_eq!(
            parse("[MSG:'$H'|'$X' to unlock]"),
            Reply::Message("'$H'|'$X' to unlock".into())
        );
        assert_eq!(parse("$32=1"), Reply::Setting(32, "1".into()));
        assert_eq!(parse("$130=400.000"), Reply::Setting(130, "400.000".into()));
        assert_eq!(
            parse("[VER:1.1h.20190825:]"),
            Reply::Other("[VER:1.1h.20190825:]".into())
        );
        assert_eq!(parse("error:x"), Reply::Other("error:x".into()));
    }

    #[test]
    fn lines_wait_for_their_end() {
        let mut lines = Lines::default();
        assert!(lines.push(b"o").is_empty());
        assert_eq!(lines.push(b"k\r\n\r\nerr"), ["ok"]);
        assert_eq!(lines.push(b"or:2\n"), ["error:2"]);
    }
}
