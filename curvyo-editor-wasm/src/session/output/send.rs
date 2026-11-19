//! Send to machine (`specs/0125-send-to-machine`; architect addendum B2, B3): the job bytes for the
//! connector, the one pending send, and the commit after the connector accepted the job.

use curvyo_laser_core::job::{ExportEffects, name as format_name, write};
use curvyo_ui_core::{SendCause, SendLink, SendMachine, send_button};
use serde::Serialize;

use super::super::Session;
use super::export::{HandOut, OUTPUT_NOTICE_MS};

/// The Send button of the output group, present only for a profile that names a connector (S1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SendView {
    /// Whether a press sends.
    pub enabled: bool,
    /// Why not: `blocked`, `browser`, `format_not_spoken`, `not_paired`, `no_longer_paired`,
    /// `key_changed` or `sending`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<&'static str>,
    /// The words of `blocked` (Export's own text) and `format_not_spoken`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The machine profile the connector belongs to.
    pub profile_id: String,
    /// The connector's address as the profile stores it.
    pub address: String,
    /// Whether a press also starts the job, from the connector's last status (`0134` SS9).
    pub starts: bool,
    /// A number that changes whenever the job a press would send changes (`0134` UX P15).
    pub job: u64,
}

/// A job handed to the host for the connector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentJobFile {
    /// The job text's bytes: the file Export would write.
    pub bytes: Vec<u8>,
    /// The page's name, the job's name at the connector.
    pub name: String,
    /// The machine profile whose pairing is used.
    pub profile_id: String,
    /// The connector's address.
    pub address: String,
    /// The token [`Session::send_job_done`] and [`Session::send_job_failed`] take.
    pub token: u32,
}

/// The one send in flight, the next token and the link to the connector.
#[derive(Debug, Default)]
pub(super) struct SendState {
    pending: Option<(u32, ExportEffects)>,
    /// A send that timed out unresolved: Send is enabled again, a late acceptance still commits.
    timed_out: Option<(u32, ExportEffects)>,
    next: u32,
    link: SendLink,
    machine: SendMachine,
}

/// The notice after an accepted send: only the fixture jobs it switched off, `None` for none
/// (UX notes U11).
#[must_use]
pub fn send_notice(fixture_jobs_off: usize) -> Option<(String, u32)> {
    match fixture_jobs_off {
        0 => None,
        1 => Some((
            "Output turned off on 1 fixture job.".to_owned(),
            OUTPUT_NOTICE_MS,
        )),
        n => Some((
            format!("Output turned off on {n} fixture jobs."),
            OUTPUT_NOTICE_MS,
        )),
    }
}

impl Session {
    /// How the app stands to the connector of the active page (S3), set by the frontend from its
    /// link store: `browser`, `not_paired`, `no_longer_paired`, `key_changed` or `paired`.
    pub fn set_send_link(&mut self, spelling: &str) {
        self.output.send.link = SendLink::parse(spelling);
    }

    /// How the page's connector stands for a start (`0134` SS9), set by the frontend from its
    /// link store's last status: the state spelling and the controller's own word (`""` for
    /// none); an empty state for no status.
    pub fn set_send_machine(&mut self, state: &str, controller_word: &str) {
        self.output.send.machine = SendMachine::parse(state, controller_word);
    }

    /// Whether a press of Send to machine now also starts the job (`0134` SS9, UX P4): the
    /// check made at the arming and at the confirming press, after [`Self::set_send_machine`].
    #[must_use]
    pub fn send_will_start(&self) -> bool {
        self.output_group_view()
            .and_then(|view| view.send)
            .is_some_and(|send| send.starts)
    }

    /// The profile id and connector address of the active page, `None` without an address.
    fn connector_of_page(&self) -> Option<(String, String)> {
        let id = self.active_copy()?.read.copy.id?;
        let record = self.manufacturing.machine.profiles.find(&id)?;
        let address = record.connection.connector_address.trim().to_owned();
        (!address.is_empty() && !record.is_builtin()).then_some((id, address))
    }

    /// The Send button for the output group view, `None` for a profile without a connector.
    pub(super) fn send_view(
        &self,
        hand: &super::job_cache::PageJob,
        blocked: Option<String>,
    ) -> Option<SendView> {
        let (profile_id, address) = self.connector_of_page()?;
        let format = hand.input.machine.values.output_format;
        let sending = self.output.send.pending.is_some();
        let state = &self.output.send;
        let button = send_button(&hand.gate, format, state.link, sending, state.machine);
        let (cause, text) = match button.cause {
            None => (None, None),
            Some(SendCause::Blocked) => (Some("blocked"), blocked),
            Some(SendCause::Browser) => (Some("browser"), None),
            Some(SendCause::FormatNotSpoken(format)) => (
                Some("format_not_spoken"),
                Some(format!(
                    "The connector does not speak {}",
                    format_name(format)
                )),
            ),
            Some(SendCause::NotPaired) => (Some("not_paired"), None),
            Some(SendCause::NoLongerPaired) => (Some("no_longer_paired"), None),
            Some(SendCause::KeyChanged) => (Some("key_changed"), None),
            Some(SendCause::Sending) => (Some("sending"), None),
        };
        Some(SendView {
            enabled: button.enabled,
            cause,
            text,
            profile_id,
            address,
            starts: button.starts,
            job: hand.generation,
        })
    }

    /// The job of the active laser page for the connector (S6): the gate is checked again at
    /// this moment, and `None` unless Send is enabled. Nothing is written to the project.
    pub fn send_job(&mut self) -> Option<SentJobFile> {
        let (profile_id, address) = self.connector_of_page()?;
        let HandOut { job, effects } = self.hand_out()?;
        let format = job.input.machine.values.output_format;
        let state = &mut self.output.send;
        let sending = state.pending.is_some();
        if !send_button(&job.gate, format, state.link, sending, state.machine).enabled {
            return None;
        }
        let token = state.next;
        state.timed_out = None;
        state.next = state.next.wrapping_add(1);
        state.pending = Some((token, effects));
        Some(SentJobFile {
            bytes: write(format, &job.built.job),
            name: job.input.name.clone(),
            profile_id,
            address,
            token,
        })
    }

    /// The connector accepted the job of `token` (S10): one step "Send job to machine", then
    /// Lasered at `now_ms`. The notice when fixture jobs were switched off; `None` otherwise and
    /// for an unknown or used token, which commits nothing.
    pub fn send_job_done(&mut self, token: u32, now_ms: u64) -> Option<(String, u32)> {
        let state = &mut self.output.send;
        let slot = if state
            .pending
            .as_ref()
            .is_some_and(|(held, _)| *held == token)
        {
            &mut state.pending
        } else {
            &mut state.timed_out
        };
        let (held, effects) = slot.take()?;
        if held != token {
            *slot = Some((held, effects));
            return None;
        }
        let jobs_on = self.commit_job_effects(
            &effects,
            curvyo_document_core::SEND_JOB_COMMIT_LABEL,
            now_ms,
        );
        send_notice(jobs_on)
    }

    /// The send of `token` did not end with an accepted job (S11): nothing is committed.
    pub fn send_job_failed(&mut self, token: u32) {
        if self
            .output
            .send
            .pending
            .as_ref()
            .is_some_and(|(held, _)| *held == token)
        {
            self.output.send.pending = None;
        }
        if self
            .output
            .send
            .timed_out
            .as_ref()
            .is_some_and(|(held, _)| *held == token)
        {
            self.output.send.timed_out = None;
        }
    }

    /// The send of `token` got no answer within the host's bound (S7, S8): Send is enabled
    /// again, nothing is committed now, and an acceptance that still arrives for `token` commits
    /// once until the next send is handed out.
    pub fn send_job_timed_out(&mut self, token: u32) {
        let state = &mut self.output.send;
        if state
            .pending
            .as_ref()
            .is_some_and(|(held, _)| *held == token)
        {
            state.timed_out = state.pending.take();
        }
    }
}
