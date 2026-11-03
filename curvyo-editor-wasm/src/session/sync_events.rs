//! `Session`'s handling of what the relay session asks for: transport actions queued for the
//! host and session events applied to the link and the document.

use curvyo_sync_core::{ClientOutput, RefusalReason, SessionEvent};
use curvyo_ui_core::presence::PresencePayload;
use curvyo_ui_core::share_text::{JoinRefusal, NOT_ADMITTED_NOTICE, REMOVED_NOTICE};

use super::Session;
use super::sync::{BACK_ONLINE_AFTER_MS, JoinPhase, SyncEnd, TransportAction};

impl Session {
    /// Carries out everything the client session asked for, until it asks for nothing.
    pub(super) fn sync_pump(&mut self) {
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
            SessionEvent::Remote { seq, kind, bytes } => link.remote.push_back((seq, kind, bytes)),
            SessionEvent::Acked { client_id, .. } => {
                if let Some(version) = link.pushed.remove(&client_id) {
                    link.synced = version;
                }
            }
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
            SessionEvent::Refused(reason) => {
                if link.awaiting_document {
                    let refusal = match reason {
                        RefusalReason::Used => JoinRefusal::Used,
                        RefusalReason::Expired => JoinRefusal::Expired,
                        _ => JoinRefusal::Invalid,
                    };
                    link.join = Some(JoinPhase::Refused(refusal));
                } else if reason == RefusalReason::NotAMember {
                    link.notice(NOT_ADMITTED_NOTICE.to_owned(), true);
                }
                link.online = false;
                link.ended = Some(if reason == RefusalReason::Removed {
                    SyncEnd::Removed
                } else {
                    SyncEnd::Refused
                });
            }
            SessionEvent::Removed => {
                link.online = false;
                link.ended = Some(SyncEnd::Removed);
                link.peers.clear();
                link.notice(REMOVED_NOTICE.to_owned(), true);
                self.document.set_share(None);
            }
            SessionEvent::Peer { connection, member } => {
                link.peers.entry(connection).or_default().member = Some(member);
            }
            SessionEvent::Presence {
                connection,
                member,
                payload,
            } => {
                let payload = serde_json::from_slice::<PresencePayload>(&payload)
                    .ok()
                    .and_then(PresencePayload::checked);
                if let Some(payload) = payload {
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
            }
            SessionEvent::PresenceGone { connection } => {
                link.peers.remove(&connection);
            }
            SessionEvent::SnapshotDue { covers_seq } => {
                // Every record before it is applied first: the snapshot holds them.
                self.sync_apply_remote();
                self.upload_snapshot(covers_seq);
            }
            SessionEvent::UpdateRejected => self.reject_once(),
            SessionEvent::Resyncing => {
                link.online = false;
                link.resyncing = true;
                link.peers.clear();
            }
        }
    }

    pub(super) fn upload_snapshot(&mut self, covers_seq: u64) {
        if self.sync.as_ref().is_none_or(|s| s.awaiting_document) {
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
