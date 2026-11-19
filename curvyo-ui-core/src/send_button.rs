//! Decides what a press of Send to machine does: whether it is enabled, why not, and whether it
//! also starts the job (`specs/0125-send-to-machine` S2 to S5, S8, addendum B2;
//! `specs/0134-send-and-start` SS9, addendum S1).

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

/// How the page's connector stands for a start, from its last status (`0134` addendum S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SendMachine {
    /// No status was read yet.
    #[default]
    Unknown,
    /// `Idle` or `Done`, and the controller reports no state word of its own: a sent job would
    /// be accepted and nothing else is held.
    Free,
    /// Anything else: a job is held or waits, the machine homes, is in alarm, has no controller,
    /// or the controller reports a word such as `Door` (`0127` G4).
    Holds,
}

impl SendMachine {
    /// The machine a status names: `state` is the connector's state spelling (`Idle`, `Ready`,
    /// ...), `controller_word` the controller's own state word (`Door`), `""` for none. An empty
    /// state reads as no status; an unknown spelling holds.
    #[must_use]
    pub fn parse(state: &str, controller_word: &str) -> Self {
        match state {
            "" => Self::Unknown,
            "Idle" | "Done" if controller_word.is_empty() => Self::Free,
            _ => Self::Holds,
        }
    }
}

/// Whether a press of Send to machine on `machine` also starts the job (`0134` SS1, SS2, SS5):
/// only on a free machine. The connector's own rules decide whether the start then waits
/// (`--require-local-start`, a member's start; SS3).
#[must_use]
pub const fn send_starts(machine: SendMachine) -> bool {
    matches!(machine, SendMachine::Free)
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
    /// Whether a press also starts the job: enabled and [`send_starts`] (`0134` SS9).
    pub starts: bool,
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
/// the pairing, a running send. `machine` decides only whether an enabled press also starts.
#[must_use]
pub fn send_button(
    gate: &Gate,
    format: OutputFormat,
    link: SendLink,
    sending: bool,
    machine: SendMachine,
) -> SendButton {
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
        starts: cause.is_none() && send_starts(machine),
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
        send_button(
            gate,
            OutputFormat::Grbl,
            link,
            sending,
            SendMachine::Unknown,
        )
        .cause
    }

    #[test]
    fn a_paired_page_without_blocks_sends() {
        let button = send_button(
            &gate(vec![]),
            OutputFormat::Grbl,
            SendLink::Paired,
            false,
            SendMachine::Unknown,
        );
        assert_eq!(
            button,
            SendButton {
                enabled: true,
                cause: None,
                starts: false,
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
            let button = send_button(
                &g,
                OutputFormat::Grbl,
                SendLink::Paired,
                false,
                SendMachine::Free,
            );
            assert!(!button.enabled, "{block:?}");
            assert_eq!(button.cause, Some(SendCause::Blocked));
        }
        let g = gate(vec![]);
        assert!(g.export_allowed());
        assert!(
            send_button(
                &g,
                OutputFormat::Grbl,
                SendLink::Paired,
                false,
                SendMachine::Free
            )
            .enabled
        );
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

    /// Every state spelling the connector sends (`0134` addendum S7), with and without a
    /// controller word: only `Idle` and `Done` without a word are free.
    const STATES: [(&str, bool); 11] = [
        ("NoController", false),
        ("Idle", true),
        ("Receiving", false),
        ("Ready", false),
        ("AwaitingLocalStart", false),
        ("Homing", false),
        ("Running", false),
        ("Paused", false),
        ("Done", true),
        ("Alarm", false),
        ("Junk", false),
    ];

    #[test]
    fn ss11_a_press_starts_only_on_a_free_machine() {
        for (state, free) in STATES {
            for word in ["", "Door", "Check", "Sleep"] {
                let machine = SendMachine::parse(state, word);
                let starts = free && word.is_empty();
                assert_eq!(send_starts(machine), starts, "{state} {word}");
                assert_eq!(machine == SendMachine::Free, starts, "{state} {word}");
            }
        }
        assert_eq!(SendMachine::parse("", ""), SendMachine::Unknown);
        assert!(!send_starts(SendMachine::Unknown));
        assert!(!send_starts(SendMachine::default()));
    }

    #[test]
    fn ss9_starts_needs_an_enabled_button_and_a_free_machine() {
        let ok = gate(vec![]);
        let blocked = gate(vec![Cause::NothingRuns]);
        for (state, free) in STATES {
            let machine = SendMachine::parse(state, "");
            for link in [SendLink::Paired, SendLink::NotPaired, SendLink::Browser] {
                for sending in [false, true] {
                    for g in [&ok, &blocked] {
                        let button = send_button(g, OutputFormat::Grbl, link, sending, machine);
                        assert_eq!(
                            button.starts,
                            button.enabled && free,
                            "{state} {link:?} {sending}"
                        );
                        assert!(!button.starts || button.cause.is_none());
                    }
                }
            }
        }
    }

    #[test]
    fn ss8_the_machine_never_changes_whether_send_is_enabled() {
        let ok = gate(vec![]);
        for (state, _) in STATES {
            let button = send_button(
                &ok,
                OutputFormat::Grbl,
                SendLink::Paired,
                false,
                SendMachine::parse(state, ""),
            );
            assert!(button.enabled, "{state}");
        }
    }
}
