//! The hub's mode (ADR 0071 decisions 1 to 4): maintenance with its window, configured, or
//! foreground; the `Welcome` of each, the first configuration and its race, later configuration
//! changes and the reset's exit request.

use std::net::IpAddr;
use std::sync::mpsc::Sender;
use std::time::Instant;

use curvyo_connector_core::{
    ConnectorConfig, ConnectorMode, ErrorCode, Message, Mode, Peer, Role, StartRule, State,
    check_client_name,
};
use time::OffsetDateTime;

use super::{Hub, Inner, PoisonError, Refused};
use crate::admin_log::AdminEvent;
use crate::config_file;
use crate::log_tail::last_lines;
use crate::maintenance::MaintenanceWindow;
use crate::token::Token;

/// The connector's mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HubMode {
    /// Run with the command-line flags: no owners, nothing written but pairings and logs.
    Foreground,
    /// Not configured; `ApplyConfig` is taken while the window is open.
    Maintenance(MaintenanceWindow),
    /// Configured from `connector.toml`.
    Configured(Box<ConnectorConfig>),
}

/// Why the program is asked to end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitReason {
    /// SIGINT or SIGTERM.
    Signal,
    /// An owner's `Reset`: the state directory is to be emptied after the shutdown (A19).
    Reset {
        /// The owner who sent it.
        by: String,
    },
}

fn error(code: ErrorCode, text: String) -> Refused {
    Refused { code, text }
}

/// Whether a job is held or running: a configuration change waits (A16).
fn job_in_hand(state: State) -> bool {
    matches!(
        state,
        State::Receiving
            | State::Ready
            | State::AwaitingLocalStart
            | State::Homing
            | State::Running
            | State::Paused
    )
}

/// The top-level fields that differ between two configurations, for `admin.log`.
fn changed_fields(old: &ConnectorConfig, new: &ConnectorConfig) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if old.controller != new.controller {
        fields.push("controller");
    }
    if old.listen != new.listen {
        fields.push("listen");
    }
    if old.name != new.name {
        fields.push("name");
    }
    if old.owner != new.owner {
        fields.push("owner");
    }
    fields
}

impl Hub {
    /// Sets the mode at start, before any connection: the machine follows the mode's start rule.
    pub fn set_mode(&self, mode: HubMode) {
        let rule = match &mode {
            HubMode::Configured(_) => StartRule::OwnerConfirms,
            _ if self.settings.require_local_start => StartRule::LocalPress,
            _ => StartRule::Immediate,
        };
        let configured = matches!(mode, HubMode::Configured(_));
        let mut inner = self.lock();
        inner.mode = mode;
        if configured {
            self.rebuild(&mut inner, &self.settings.controller.clone(), rule);
        }
    }

    /// The channel `Reset` and the signal thread send the exit request on.
    pub fn set_exit(&self, exit: Sender<super::ExitReason>) {
        *self.exit.lock().unwrap_or_else(PoisonError::into_inner) = Some(exit);
    }

    /// Asks the main thread to end the program.
    pub fn request_exit(&self, reason: ExitReason) {
        if let Some(exit) = &*self.exit.lock().unwrap_or_else(PoisonError::into_inner) {
            let _ = exit.send(reason);
        }
    }

    /// The mode as the access table needs it.
    #[must_use]
    pub fn access_mode(&self, now: Instant) -> Mode {
        match &self.lock().mode {
            HubMode::Foreground => Mode::Foreground,
            HubMode::Configured(_) => Mode::Configured,
            HubMode::Maintenance(window) => Mode::Maintenance {
                window_open: window.is_open(now),
            },
        }
    }

    /// The mode itself.
    #[must_use]
    pub fn mode(&self) -> HubMode {
        self.lock().mode.clone()
    }

    /// Counts the switches out of maintenance mode.
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.lock().epoch
    }

    /// The configuration, in configured mode.
    #[must_use]
    pub fn config(&self) -> Option<ConnectorConfig> {
        match &self.lock().mode {
            HubMode::Configured(config) => Some((**config).clone()),
            _ => None,
        }
    }

    /// The connector's name: the configuration's, else the host's.
    pub(super) fn name_of(&self, inner: &Inner) -> String {
        match &inner.mode {
            HubMode::Configured(config) => config.name.clone(),
            _ => self.settings.connector_name.clone(),
        }
    }

    pub(super) fn welcome_of(&self, inner: &Inner, now: Instant) -> Message {
        let mode = match &inner.mode {
            HubMode::Foreground => ConnectorMode::Foreground,
            HubMode::Maintenance(window) => ConnectorMode::Maintenance {
                closes_in_s: window.closes_in(now).as_secs(),
            },
            HubMode::Configured(config) => ConnectorMode::Configured {
                owner: config.owner.clone(),
            },
        };
        Message::Welcome {
            connector_name: self.name_of(inner),
            controller: inner.info.clone(),
            mode: Some(mode),
        }
    }

    /// The `Welcome` for a greeted client.
    #[must_use]
    pub fn welcome(&self) -> Message {
        let inner = self.lock();
        self.welcome_of(&inner, Instant::now())
    }

    /// `ApplyConfig` (A6, A7, A16): `owner` is `None` from a session that is not logged in. In
    /// maintenance mode the sender becomes the owner and the answer carries its token; in
    /// configured mode an owner replaces the configuration. Both writes happen under the hub's
    /// lock, so of two first configurations exactly one wins (addendum D6).
    ///
    /// # Errors
    /// The `Error` to answer: window closed, invalid, already configured, or a job in hand.
    pub fn apply_config(
        &self,
        config: ConnectorConfig,
        client_name: &str,
        owner: Option<&Peer>,
        peer: IpAddr,
        now: Instant,
    ) -> Result<Option<String>, Refused> {
        let mut inner = self.lock();
        match (&inner.mode, owner) {
            (HubMode::Foreground, _) => Err(Refused::fixed(ErrorCode::OwnerRequired)),
            (HubMode::Maintenance(window), _) if !window.is_open(now) => {
                Err(Refused::fixed(ErrorCode::MaintenanceClosed))
            }
            (HubMode::Maintenance(_), _) => {
                self.first_config(&mut inner, config, client_name, peer)
            }
            (HubMode::Configured(_), None) => {
                self.note_refused_setup(&inner, peer);
                Err(error(
                    ErrorCode::OwnerRequired,
                    "The connector is configured already.".to_owned(),
                ))
            }
            (HubMode::Configured(old), Some(by)) => {
                let old = (**old).clone();
                self.change_config(&mut inner, &old, config, &by.name)
                    .map(|()| None)
            }
        }
    }

    fn first_config(
        &self,
        inner: &mut Inner,
        config: ConnectorConfig,
        client_name: &str,
        peer: IpAddr,
    ) -> Result<Option<String>, Refused> {
        let invalid = |text: String| error(ErrorCode::InvalidConfig, text);
        config.validate().map_err(|e| invalid(e.to_string()))?;
        check_client_name(client_name).map_err(|r| invalid(format!("client_name: {r}")))?;
        let failed = |e: &dyn std::fmt::Display| {
            let _ = inner.notices.send(format!("Cannot configure: {e}"));
            error(
                ErrorCode::WrongState,
                "The configuration was not written.".to_owned(),
            )
        };
        let token = Token::generate().ok_or_else(|| failed(&"no random bytes are available"))?;
        let at = OffsetDateTime::now_utc();
        // `clients.json` first, then `connector.toml`, the commit point (addendum D6).
        self.clients.drop_owners().map_err(|e| failed(&e))?;
        self.clients
            .add(client_name, Role::Owner, &token.sha256_hex(), at)
            .map_err(|e| failed(&e))?;
        config_file::store(&self.settings.state_dir, &config).map_err(|e| failed(&e))?;
        let controller = config.controller.clone();
        let _ = inner
            .notices
            .send(format!("Configured by {client_name} from {peer}"));
        let _ = inner
            .notices
            .send(format!("Controller: {}", controller.describe()));
        inner.mode = HubMode::Configured(Box::new(config));
        inner.epoch += 1;
        inner.configured_by = Some((client_name.to_owned(), peer));
        self.append_admin(inner, client_name, &AdminEvent::Configured { peer });
        self.install(inner, controller);
        Ok(Some(token.to_hex().to_string()))
    }

    fn change_config(
        &self,
        inner: &mut Inner,
        old: &ConnectorConfig,
        config: ConnectorConfig,
        by: &str,
    ) -> Result<(), Refused> {
        if job_in_hand(inner.machine.state()) {
            return Err(error(
                ErrorCode::WrongState,
                "A job is held or running; the configuration is not changed.".to_owned(),
            ));
        }
        config
            .validate()
            .map_err(|e| error(ErrorCode::InvalidConfig, e.to_string()))?;
        if let Err(e) = config_file::store(&self.settings.state_dir, &config) {
            let _ = inner
                .notices
                .send(format!("Cannot change the configuration: {e}"));
            return Err(error(
                ErrorCode::WrongState,
                "The configuration was not written.".to_owned(),
            ));
        }
        let fields = changed_fields(old, &config);
        let controller = (old.controller != config.controller).then(|| config.controller.clone());
        inner.mode = HubMode::Configured(Box::new(config));
        let _ = inner.notices.send(format!("Configuration changed by {by}"));
        self.append_admin(inner, by, &AdminEvent::ConfigChanged { fields });
        match controller {
            Some(controller) => {
                let _ = inner
                    .notices
                    .send(format!("Controller: {}", controller.describe()));
                self.install(inner, controller);
            }
            None => self.publish_welcome(inner),
        }
        Ok(())
    }

    /// The terminal line of a setup refused because the connector is configured (A7).
    fn note_refused_setup(&self, inner: &Inner, peer: IpAddr) {
        let by = inner.configured_by.clone().map_or_else(
            || {
                let lines = last_lines(self.admin_log.path(), 1000).unwrap_or_default();
                let found = lines.iter().rev().find_map(|l| {
                    let mut f = l.split('\t');
                    let (_, event, by, _, detail) =
                        (f.next()?, f.next()?, f.next()?, f.next()?, f.next()?);
                    (event == "configured").then(|| format!("{by} from {detail}"))
                });
                found.unwrap_or_else(|| "an earlier setup".to_owned())
            },
            |(name, address)| format!("{name} from {address}"),
        );
        let _ = inner
            .notices
            .send(format!("Setup from {peer} refused: configured by {by}"));
    }

    /// A refused setup from a session that is not logged in, seen by the gate (A7).
    pub fn setup_refused(&self, peer: IpAddr) {
        let inner = self.lock();
        if matches!(inner.mode, HubMode::Configured(_)) {
            self.note_refused_setup(&inner, peer);
        }
    }

    /// Prints "Maintenance window closed at HH:MM UTC" once, when the window has closed (A5).
    pub fn check_window(&self, now: Instant, closed_at: &str) {
        let mut inner = self.lock();
        let closed = matches!(&inner.mode, HubMode::Maintenance(w) if !w.is_open(now));
        if closed && !inner.window_told {
            inner.window_told = true;
            let _ = inner
                .notices
                .send(format!("Maintenance window closed at {closed_at}"));
        }
    }

    /// `Reset { connector_name }` (A19): the name must be the configuration's.
    ///
    /// # Errors
    /// `UnknownName` for another name; nothing changes.
    pub fn check_reset(&self, connector_name: &str) -> Result<(), Refused> {
        let inner = self.lock();
        match &inner.mode {
            HubMode::Configured(config) if config.name == connector_name => Ok(()),
            _ => Err(error(
                ErrorCode::UnknownName,
                format!("This connector is not called {connector_name}."),
            )),
        }
    }
}
