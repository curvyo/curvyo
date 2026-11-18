//! Decides whether Send to machine is enabled and why not (`specs/0124-send-to-machine`
//! criteria S2 to S5, S8; architect addendum B2).

use curvyo_laser_core::OutputFormat;
use curvyo_laser_core::job::Gate;

/// How this app stands to the page's connector, set by the frontend from its link store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SendLink {
    /// The browser build: no direct route to a connector (S5).
    Browser,
    /// No pairing is stored for the profile (`0064` Q5).
    #[default]
    NotPaired,
    /// The connector revoked the token (`0064` Q9).
    NoLongerPaired,
    /// The pinned key does not match (`0064` Q8).
    KeyChanged,
    /// Paired; reachability is never guessed (S3).
    Paired,
}

impl SendLink {
    /// The link a spelling names (`browser`, `not_paired`, `no_longer_paired`, `key_changed`,
    /// `paired`); anything else reads as not paired.
    #[must_use]
    pub fn parse(spelling: &str) -> Self {
        match spelling {
            "browser" => Self::Browser,
            "no_longer_paired" => Self::NoLongerPaired,
            "key_changed" => Self::KeyChanged,
            "paired" => Self::Paired,
            _ => Self::NotPaired,
        }
    }
}

/// Why Send is disabled; the first applicable cause in the order of [`send_button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendCause {
    /// A check of the page blocks: Export is disabled with the same text.
    Blocked,
    /// The browser build.
    Browser,
    /// The connector does not speak this output format.
    FormatNotSpoken(OutputFormat),
    /// No pairing.
    NotPaired,
    /// The token was revoked.
    NoLongerPaired,
    /// The pinned key changed.
    KeyChanged,
    /// A send is running.
    Sending,
}

/// The Send button's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendButton {
    /// Whether a press sends.
    pub enabled: bool,
    /// Why not, `None` when enabled.
    pub cause: Option<SendCause>,
}

/// Whether the connector speaks `format`: one arm per format (ADR 0064 section 12).
#[must_use]
pub const fn connector_speaks(format: OutputFormat) -> bool {
    match format {
        OutputFormat::Grbl => true,
    }
}

/// The state of Send for a page whose checks are `gate`, writing `format`, with `link` to its
/// connector and `sending` while this session's send runs. Causes in this order, the first wins:
/// a blocking check (Export's own cause), the browser, a format the connector does not speak,
/// the pairing, a running send.
#[must_use]
pub fn send_button(gate: &Gate, format: OutputFormat, link: SendLink, sending: bool) -> SendButton {
    let cause = if !gate.export_allowed() {
        Some(SendCause::Blocked)
    } else if link == SendLink::Browser {
        Some(SendCause::Browser)
    } else if !connector_speaks(format) {
        Some(SendCause::FormatNotSpoken(format))
    } else {
        match link {
            SendLink::NotPaired => Some(SendCause::NotPaired),
            SendLink::NoLongerPaired => Some(SendCause::NoLongerPaired),
            SendLink::KeyChanged => Some(SendCause::KeyChanged),
            SendLink::Browser | SendLink::Paired => sending.then_some(SendCause::Sending),
        }
    };
    SendButton {
        enabled: cause.is_none(),
        cause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_laser_core::job::{Cause, Issue, MachineFacts, Severity, Targets};

    fn gate(causes: Vec<Cause>) -> Gate {
        Gate {
            machine: MachineFacts {
                name: "Diode".to_owned(),
                format: OutputFormat::Grbl,
                air_assist: false,
                damaged: false,
            },
            runs: Vec::new(),
            issues: causes
                .into_iter()
                .map(|cause| Issue {
                    cause,
                    count: 1,
                    targets: Targets::None,
                })
                .collect(),
        }
    }

    fn cause(gate: &Gate, link: SendLink, sending: bool) -> Option<SendCause> {
        send_button(gate, OutputFormat::Grbl, link, sending).cause
    }

    #[test]
    fn a_paired_page_without_blocks_sends() {
        let button = send_button(&gate(vec![]), OutputFormat::Grbl, SendLink::Paired, false);
        assert_eq!(
            button,
            SendButton {
                enabled: true,
                cause: None
            }
        );
    }

    #[test]
    fn a_warning_does_not_disable_send() {
        let warn = gate(vec![Cause::GenericLaser]);
        assert_eq!(warn.first_block(), None);
        assert_eq!(cause(&warn, SendLink::Paired, false), None);
    }

    #[test]
    fn each_connector_cause_names_itself() {
        let ok = gate(vec![]);
        assert_eq!(
            cause(&ok, SendLink::Browser, false),
            Some(SendCause::Browser)
        );
        assert_eq!(
            cause(&ok, SendLink::NotPaired, false),
            Some(SendCause::NotPaired)
        );
        assert_eq!(
            cause(&ok, SendLink::NoLongerPaired, false),
            Some(SendCause::NoLongerPaired)
        );
        assert_eq!(
            cause(&ok, SendLink::KeyChanged, false),
            Some(SendCause::KeyChanged)
        );
        assert_eq!(cause(&ok, SendLink::Paired, true), Some(SendCause::Sending));
    }

    #[test]
    fn the_causes_come_in_the_documented_order() {
        let blocked = gate(vec![Cause::NothingRuns]);
        assert_eq!(
            cause(&blocked, SendLink::Browser, true),
            Some(SendCause::Blocked)
        );
        let ok = gate(vec![]);
        assert_eq!(
            cause(&ok, SendLink::Browser, true),
            Some(SendCause::Browser)
        );
        assert_eq!(
            cause(&ok, SendLink::NotPaired, true),
            Some(SendCause::NotPaired)
        );
        assert_eq!(cause(&ok, SendLink::Paired, true), Some(SendCause::Sending));
    }

    #[test]
    fn the_connector_speaks_every_format_this_build_writes() {
        for &format in OutputFormat::ALL {
            assert!(connector_speaks(format));
        }
    }

    #[test]
    fn export_disabled_means_send_disabled_for_every_blocking_cause() {
        let blocks = [
            Cause::FixtureNoMaterial,
            Cause::OutsideBed {
                width_um: 1,
                height_um: 1,
            },
            Cause::MachineDamaged,
            Cause::StartDamaged,
            Cause::NothingRuns,
        ];
        for block in blocks {
            assert_eq!(block.severity(), Severity::Block, "{block:?}");
            let g = gate(vec![block.clone()]);
            assert!(!g.export_allowed());
            let button = send_button(&g, OutputFormat::Grbl, SendLink::Paired, false);
            assert!(!button.enabled, "{block:?}");
            assert_eq!(button.cause, Some(SendCause::Blocked));
        }
        let g = gate(vec![]);
        assert!(g.export_allowed());
        assert!(send_button(&g, OutputFormat::Grbl, SendLink::Paired, false).enabled);
    }

    #[test]
    fn the_link_spellings_parse() {
        assert_eq!(SendLink::parse("browser"), SendLink::Browser);
        assert_eq!(SendLink::parse("paired"), SendLink::Paired);
        assert_eq!(SendLink::parse("key_changed"), SendLink::KeyChanged);
        assert_eq!(
            SendLink::parse("no_longer_paired"),
            SendLink::NoLongerPaired
        );
        assert_eq!(SendLink::parse("junk"), SendLink::NotPaired);
    }
}
