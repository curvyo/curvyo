//! `Session`'s answer to a return check after a removal or a leave (`specs/0097` ADR 0041
//! decision 6). Since the customer dropped add back (2026-11-09) no check starts from the page;
//! the way back is a new invite.

use curvyo_crypto_core::keyring::Keyring;
use curvyo_document_core::EndReason;
use curvyo_sync_core::CheckOutcome;

use super::Session;

impl Session {
    /// A return check answered: after Keep as it is, the ended record says "left" (a
    /// document's in its file, a workspace's for the marker).
    pub(super) fn check_answered(&mut self, outcome: CheckOutcome) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        link.check = Some(outcome);
        link.revision = link.revision.wrapping_add(1);
        if outcome != CheckOutcome::LeftAgain {
            return;
        }
        let heads = link.client.keyring().map(Keyring::heads);
        if let Some((record, workspace)) = link.returning.as_mut() {
            record.ended = EndReason::Left;
            // The heads now hold this leave: the earlier add back is no answer to a later check.
            if let Some(heads) = heads {
                record.heads = heads.into_iter().map(|h| h.0).collect();
            }
            if *workspace {
                // The marker's record, for the host to write.
                link.workspace_ended = Some(record.clone());
            } else {
                self.document.set_ended_share(Some(record.clone()));
            }
        }
    }
}
