//! The words of a browser's passkey key backup (ADR 0040 decision 13, Frage 145 A,
//! `specs/0096-own-devices-sync/ux-notes.md` 9.1).

/// The own row's line while a passkey backup exists (replaces "Clearing site data removes this
/// device.").
pub const BACKED_UP_LINE: &str = "Keys backed up with a passkey. Clearing site data does not lose \
                                  this device.";

/// The button that starts a backup.
pub const BACK_UP_LABEL: &str = "Back up with passkey";
/// The button that removes the backup.
pub const REMOVE_LABEL: &str = "Remove backup";
/// The button on a browser without a group that restores this device.
pub const RESTORE_LABEL: &str = "Restore with passkey";

/// While the browser waits for the passkey.
pub const WAITING: &str = "Waiting for the passkey...";
/// While the record goes to the server or comes back.
pub const WORKING: &str = "Talking to the server...";

/// The honest answer when the browser or its passkey has no PRF: the keys stay in `IndexedDB` only.
pub const NO_PRF: &str = "This browser or passkey cannot lock keys. The keys stay in this \
                          browser only.";
/// The maker closed the passkey prompt, or the browser refused it.
pub const CANCELLED: &str = "No passkey was used. Nothing changed.";
/// The server has no backup for this passkey.
pub const NO_BACKUP: &str = "This passkey has no backup on this server.";
/// The server could not be reached or ended the connection.
pub const UNREACHABLE: &str = "Could not reach the server. Nothing changed.";
/// The server's record does not open with this passkey.
pub const DAMAGED: &str = "The backup on the server does not open with this passkey.";
/// Restore on a browser that is in a group already: its identity is kept.
pub const IN_GROUP: &str = "This device is paired already. Nothing changed.";
/// The server refused the write (another write key, too large, or a protocol error).
pub const REFUSED: &str = "The server refused the backup.";

/// The notice after a backup was stored.
pub const STORED_NOTICE: &str = "Keys backed up with a passkey.";
/// The notice after the backup was removed.
pub const REMOVED_NOTICE: &str = "Backup removed from the server.";
/// The notice after a restore.
pub const RESTORED_NOTICE: &str = "This device was restored from its passkey backup.";

/// The answer to an ended exchange by the outcome's kind (`refused-unknown` and so on); empty
/// for a kind without words.
#[must_use]
pub fn outcome_text(kind: &str) -> &'static str {
    match kind {
        "no-prf" => NO_PRF,
        "cancelled" => CANCELLED,
        "refused-unknown" => NO_BACKUP,
        "failed" => UNREACHABLE,
        "damaged" => DAMAGED,
        "refused-forbidden" | "refused-too-large" | "refused-protocol" => REFUSED,
        "in-group" => IN_GROUP,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_outcome_of_the_page_has_words() {
        for kind in [
            "no-prf",
            "cancelled",
            "refused-unknown",
            "failed",
            "damaged",
            "refused-forbidden",
            "in-group",
        ] {
            assert!(!outcome_text(kind).is_empty(), "{kind}");
        }
        assert_eq!(outcome_text("done"), "");
    }
}
