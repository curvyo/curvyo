//! `Session`'s handling of what the relay session asks for: transport actions queued for the
//! host and session events applied to the link and the document.

use curvyo_crypto_core::MemberId;
use curvyo_document_core::EndReason;
use curvyo_sync_core::{ClientOutput, ConnectionId, RefusalReason, SessionEvent};
use curvyo_ui_core::presence::PresencePayload;
use curvyo_ui_core::share_text::{JoinRefusal, NOT_ADMITTED_NOTICE, NOT_ADMITTED_WORKSPACE_NOTICE};

use super::Session;
use super::sync::{BACK_ONLINE_AFTER_MS, JoinPhase, SyncEnd, SyncLink, TransportAction};

impl Session {
    /// Carries out everything the client session asked for, until it asks for nothing.
    pub(super) fn sync_pump(&mut self) {
        self.sync_drain();
        self.sync_pending();
    }

    fn sync_drain(&mut self) {
        loop {
            let Some(link) = self.sync.as_mut() else {
                return;
            };
            let outputs = link.client.drain();
            if outputs.is_empty() {
                return;
            }
            for output in outputs {
                match output {
                    ClientOutput::Connect => {
                        if let Some(link) = self.sync.as_mut() {
                            let address = format!("{}/", link.server);
                            link.actions.push_back(TransportAction::Connect(address));
                        }
                    }
                    ClientOutput::Send(bytes) => {
                        if let Some(link) = self.sync.as_mut() {
                            link.actions.push_back(TransportAction::Send(bytes));
                        }
                    }
                    ClientOutput::Close => {
                        if let Some(link) = self.sync.as_mut() {
                            link.actions.push_back(TransportAction::Close);
                        }
                    }
                    ClientOutput::Event(event) => self.sync_event(event),
                }
            }
        }
    }

    /// The relay confirmed own keyring acts (a `Pong`, no event): the Share tab reads again, and
    /// an add back confirmed now says "was added back" (ADR 0041 decision 11).
    fn sync_pending(&mut self) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        let pending = link.client.pending_acts();
        if pending == link.pending {
            return;
        }
        link.revision = link.revision.wrapping_add(1);
        let confirmed = link.client.confirmed_keyring();
        let added: Vec<MemberId> = link
            .adding_back
            .iter()
            .filter(|m| {
                !pending.iter().any(|a| a.member == **m)
                    && confirmed.as_ref().is_some_and(|k| k.role(m).is_some())
            })
            .copied()
            .collect();
        for member in added {
            link.adding_back.remove(&member);
            let label = link.labels.get(&member).cloned().unwrap_or_default();
            link.notice(
                curvyo_ui_core::membership_return_text::added_back_notice(&label),
                false,
            );
        }
        link.pending = pending;
    }

    #[allow(clippy::too_many_lines)] // One arm per session event; split, the table scatters.
    pub(super) fn sync_event(&mut self, event: SessionEvent) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        link.revision = link.revision.wrapping_add(1);
        match event {
            SessionEvent::Online => {
                link.online = true;
                link.resyncing = false;
                link.ever_online = true;
                if let Some(at) = link.lost_at.take()
                    && link.now.since(at) > BACK_ONLINE_AFTER_MS
                {
                    link.notice("Back online.".to_owned(), false);
                }
            }
            SessionEvent::Offline => {
                link.online = false;
                // The link was quiet since its last message: a loss found by missing pongs is
                // already 4 s old, a closed socket is fresh (criterion 17's 5 s).
                link.lost_at = Some(link.last_heard.min(link.now));
            }
            SessionEvent::Remote { seq, kind, bytes } => {
                if link.workspace.is_some() {
                    self.route_workspace_record(seq, kind, &bytes);
                } else {
                    link.remote.push_back((seq, kind, bytes));
                }
            }
            SessionEvent::Acked { client_id, .. } => self.sync_acked(client_id),
            SessionEvent::KeyringChanged => link.keys_changed = true,
            SessionEvent::WaitingForMember => {
                if link.join.is_some() {
                    link.join = Some(JoinPhase::Waiting);
                }
            }
            SessionEvent::Admitted => {
                if link.join.is_some() {
                    link.join = Some(JoinPhase::Downloading);
                }
            }
            SessionEvent::InviteExpired => {
                link.join = Some(JoinPhase::Refused(JoinRefusal::Expired));
                link.ended = Some(SyncEnd::Refused);
            }
            SessionEvent::Refused(reason) => refused(link, reason),
            SessionEvent::Removed => self.sync_removed(),
            SessionEvent::Peer { connection, member } => {
                link.peers.entry(connection).or_default().member = Some(member);
            }
            SessionEvent::Presence {
                connection,
                member,
                payload,
            } => presence(link, connection, member, &payload),
            SessionEvent::PresenceGone { connection } => {
                link.peers.remove(&connection);
            }
            SessionEvent::SnapshotDue { covers_seq } => self.sync_snapshot_due(covers_seq),
            SessionEvent::UpdateRejected => self.reject_once(),
            SessionEvent::Resyncing => {
                link.online = false;
                link.resyncing = true;
                link.peers.clear();
            }
        }
    }

    /// The relay stored a push: a document update, or the list's.
    fn sync_acked(&mut self, client_id: u64) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        if let Some(version) = link.pushed.remove(&client_id) {
            link.synced = version;
        } else {
            self.workspace_list_acked(client_id);
        }
    }

    /// A snapshot is due: every record before it is applied first, since the snapshot holds
    /// them; a workspace seals its list and documents, a document itself.
    fn sync_snapshot_due(&mut self, covers_seq: u64) {
        self.sync_apply_remote();
        if self.has_workspace_link() {
            self.workspace_snapshot_asked(covers_seq);
        } else {
            self.upload_snapshot(covers_seq);
        }
    }

    /// This device was removed from the room: the workspace's marker keeps the ended record (ADR
    /// 0041 decision 6) and its open document stays a local file (`specs/0095-workspace`
    /// criterion 44). After its own leave the record says "left".
    fn sync_removed(&mut self) {
        let Some(link) = self.sync.as_ref() else {
            return;
        };
        if link.ended == Some(SyncEnd::Left) {
            // Its own leave already ended the share.
            return;
        }
        let reason = if link.leaving {
            EndReason::Left
        } else {
            EndReason::Removed
        };
        self.end_share(reason);
    }

    pub(super) fn upload_snapshot(&mut self, covers_seq: u64) {
        if self.sync.is_none() {
            return;
        }
        // The share record belongs to this replica's file, never to the room's snapshot.
        let Ok(bytes) = curvyo_document_core::pack_snapshot(&self.document, super::APP_VERSION)
        else {
            return;
        };
        if let Some(link) = self.sync.as_mut() {
            let _ = link.client.upload_snapshot(&bytes, covers_seq);
        }
    }

    /// Criterion 25: the record `seq` did not import. It is not merged; the client session
    /// shows it once and resyncs from the room's snapshot.
    pub(super) fn record_rejected(&mut self, seq: u64) {
        if let Some(link) = self.sync.as_mut() {
            link.client.record_rejected(seq);
        }
        self.sync_pump();
    }

    /// Criterion 25: a refused update is shown once, never merged.
    pub(super) fn reject_once(&mut self) {
        if let Some(link) = self.sync.as_mut()
            && !link.rejected_shown
        {
            link.rejected_shown = true;
            link.notice("A received update was rejected.".to_owned(), true);
        }
    }
}

/// The relay refused the link: a join shows why, a member's link ends; a device that left is
/// refused as removed and stays "left".
fn refused(link: &mut SyncLink, reason: RefusalReason) {
    if link.ended == Some(SyncEnd::Left) {
        return;
    }
    if reason == RefusalReason::NotAMember {
        let text = if link.workspace.is_some() {
            NOT_ADMITTED_WORKSPACE_NOTICE
        } else {
            NOT_ADMITTED_NOTICE
        };
        link.notice(text.to_owned(), true);
    } else if link.join.is_some_and(|j| j != JoinPhase::Done) {
        let refusal = match reason {
            RefusalReason::Used => JoinRefusal::Used,
            RefusalReason::Expired => JoinRefusal::Expired,
            _ => JoinRefusal::Invalid,
        };
        link.join = Some(JoinPhase::Refused(refusal));
    }
    link.online = false;
    link.ended = Some(if reason == RefusalReason::Removed {
        SyncEnd::Removed
    } else {
        SyncEnd::Refused
    });
}

/// A peer's presence: kept when it reads, its label remembered for its member.
fn presence(
    link: &mut SyncLink,
    connection: ConnectionId,
    member: Option<MemberId>,
    payload: &[u8],
) {
    let payload = serde_json::from_slice::<PresencePayload>(payload)
        .ok()
        .and_then(PresencePayload::checked);
    let Some(payload) = payload else {
        return;
    };
    let peer = link.peers.entry(connection).or_default();
    if member.is_some() {
        peer.member = member;
    }
    if let Some(member) = peer.member
        && !payload.label.is_empty()
    {
        link.labels.insert(member, payload.label.clone());
    }
    peer.presence = Some(payload);
}
