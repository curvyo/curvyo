//! The words and numbers of the Share tab, the collaboration pill and the canvas marks of other
//! participants (`specs/0092-cooperative-editing/ux-notes.md` §2 to §8), formatted here so both
//! builds say the same.

/// How many participant colours there are (`--peer-1` to `--peer-6`, UX notes §4).
pub const PEER_COLOURS: u8 = 6;

/// The participant colour token number (1 to 6) of a fingerprint: the same on every device
/// without being stored (adrs.md decision 5). FNV-1a over the fingerprint's text.
#[must_use]
pub fn participant_colour(fingerprint: &str) -> u8 {
    let hash = fingerprint
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    // invariant: the remainder is below 6.
    u8::try_from(hash % u64::from(PEER_COLOURS)).unwrap_or(0) + 1
}

/// A disc's initials (UX notes §4): the first letters of the first two words, else the first two
/// letters, uppercase.
#[must_use]
pub fn initials(label: &str) -> String {
    let words: Vec<&str> = label.split_whitespace().collect();
    let letters: String = if words.len() >= 2 {
        words
            .iter()
            .take(2)
            .filter_map(|w| w.chars().next())
            .collect()
    } else {
        label.trim().chars().take(2).collect()
    };
    letters.to_uppercase()
}

/// `text` cut to `max` characters, the last one an ellipsis when cut.
#[must_use]
pub fn cut_label(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// The label or "Someone" when the participant is unknown (UX notes §7).
fn who(label: Option<&str>) -> &str {
    label.filter(|l| !l.trim().is_empty()).unwrap_or("Someone")
}

/// The chip of a contested object, "Ben is editing this" (criterion 15, UX notes §5): at most
/// 36 characters, the label cut first.
#[must_use]
pub fn editing_marker(label: Option<&str>) -> String {
    const TAIL: &str = " is editing this";
    let room = 36 - TAIL.chars().count();
    format!("{}{TAIL}", cut_label(who(label), room))
}

/// The canvas notice of an undo step skipped because a participant deleted its object
/// (criterion 22, UX notes §8): at most 70 characters, the label cut to 16.
#[must_use]
pub fn skipped_deleted_notice(label: Option<&str>) -> String {
    format!(
        "Skipped: {} deleted that object.",
        cut_label(who(label), 16)
    )
}

/// The notice when a participant deleted the object being moved, typed in, or the selection
/// (UX notes §7.2).
#[must_use]
pub fn remote_delete_notice(label: Option<&str>, what: RemoteDeleteCase, count: usize) -> String {
    let who = cut_label(who(label), 16);
    match what {
        RemoteDeleteCase::Moving => format!("{who} deleted the object you were moving."),
        RemoteDeleteCase::Typing => format!("{who} deleted this text while you were editing."),
        RemoteDeleteCase::Selected if count > 1 => {
            format!("{who} deleted {count} selected objects.")
        }
        RemoteDeleteCase::Selected => format!("{who} deleted the selected object."),
    }
}

/// What a remote delete took from under the maker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDeleteCase {
    /// The object of the move drag in flight.
    Moving,
    /// The text in edit mode.
    Typing,
    /// The selection, which became empty.
    Selected,
}

/// A normalised address as sentences show it: `192.168.1.20:8787` for `ws://`, the whole
/// address for `wss://`.
#[must_use]
pub fn server_shown(address: &str) -> &str {
    address.strip_prefix("ws://").unwrap_or(address)
}

/// The connection of a shared document, as the pill and the Share tab name it (UX notes §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connected, caught up and holding the key.
    Connected,
    /// Between a lost connection and the server's answer.
    Reconnecting,
    /// The connection was lost; editing goes on.
    Offline,
    /// The server could not be reached when the document opened.
    NotConnected,
    /// Joining: connected, waiting for a member to hand over the key.
    Waiting,
    /// After a rejected update: reconnecting from the server's snapshot (criterion 25).
    Resyncing,
}

impl ConnectionState {
    /// The state's name for the host (`connected`, `reconnecting`, ...).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Reconnecting => "reconnecting",
            Self::Offline => "offline",
            Self::NotConnected => "not-connected",
            Self::Waiting => "waiting",
            Self::Resyncing => "resyncing",
        }
    }

    /// The pill's short text; empty while connected (quiet: avatars only).
    #[must_use]
    pub const fn pill_text(self) -> &'static str {
        match self {
            Self::Connected => "",
            Self::Reconnecting | Self::Waiting => "Reconnecting...",
            Self::Offline => "Offline, changes are kept",
            Self::NotConnected => "Not connected",
            Self::Resyncing => "Resyncing...",
        }
    }

    /// The Share tab's line under "Shared on ...".
    #[must_use]
    pub fn tab_line(self, server: &str) -> String {
        match self {
            Self::Connected => "Connected".to_owned(),
            Self::Reconnecting | Self::Waiting => "Reconnecting...".to_owned(),
            Self::Offline => {
                "Offline. Changes are kept and merge when the connection returns.".to_owned()
            }
            Self::NotConnected => format!("Not connected to {}.", server_shown(server)),
            Self::Resyncing => "Resyncing from the server's snapshot.".to_owned(),
        }
    }
}

/// The Share tab's subject line (UX notes §2): "Not shared", "Shared, 2 people", "Offline".
#[must_use]
pub fn share_subject(shared: bool, people: usize, state: ConnectionState) -> String {
    if !shared {
        return "Not shared".to_owned();
    }
    if matches!(
        state,
        ConnectionState::Offline | ConnectionState::NotConnected
    ) {
        return "Offline".to_owned();
    }
    match people {
        1 => "Shared, 1 person".to_owned(),
        n => format!("Shared, {n} people"),
    }
}

/// The pill's accessible name, "Collaboration, 2 people, connected".
#[must_use]
pub fn pill_name(online_people: usize, state: ConnectionState) -> String {
    let people = match online_people {
        1 => "1 person".to_owned(),
        n => format!("{n} people"),
    };
    let state = match state {
        ConnectionState::Connected => "connected",
        ConnectionState::Reconnecting | ConnectionState::Waiting => "reconnecting",
        ConnectionState::Offline => "offline, changes are kept",
        ConnectionState::NotConnected => "not connected",
        ConnectionState::Resyncing => "resyncing",
    };
    format!("Collaboration, {people}, {state}")
}

/// The inline confirm of Share (UX notes §2.2), the one place the privacy fact is said.
#[must_use]
pub fn share_confirm_text(server: &str) -> String {
    format!(
        "Upload this document to {}? It leaves this computer encrypted; the server cannot read it.",
        server_shown(server)
    )
}

/// The sentence of criterion 2, verbatim.
pub const INVITE_SENTENCE: &str =
    "Anyone with this link can edit this document once, until it expires.";

/// "Expires Tue 14:32." for an expiry in seconds since the Unix epoch, shown in local time
/// `offset_minutes` east of UTC.
#[must_use]
pub fn expiry_text(expires_unix: u64, offset_minutes: i32) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    let local = i64::try_from(expires_unix).unwrap_or(i64::MAX) + i64::from(offset_minutes) * 60;
    let days = local.div_euclid(86_400);
    let seconds = local.rem_euclid(86_400);
    // invariant: `rem_euclid(7)` is in 0..7.
    let day = DAYS[usize::try_from(days.rem_euclid(7)).unwrap_or(0)];
    format!(
        "Expires {day} {:02}:{:02}.",
        seconds / 3600,
        seconds % 3600 / 60
    )
}

/// The removal dialog's sentence (criterion 32, ADR 0010 §8): removal does not reach back.
/// (Proposal: the words; the label is cut to 24 characters.)
#[must_use]
pub fn removal_confirm_text(label: &str) -> String {
    let label = if label.is_empty() {
        "This participant".to_owned()
    } else {
        cut_label(label, 24)
    };
    format!(
        "Remove {label}? Removing does not reach back: {label} keeps what is already on that computer and gets no later changes."
    )
}

/// What a removed device shows (criterion 33).
pub const REMOVED_NOTICE: &str =
    "You were removed from this document. Your copy stays on this computer.";

/// What a shared document shows when the relay refuses this device as no participant
/// (criterion 27; Proposal, the UX notes name no text for it).
pub const NOT_ADMITTED_NOTICE: &str =
    "The server did not let this device in: it is not a participant of this document.";

/// The refusal of a join or a link (UX notes §2.5).
#[must_use]
pub fn join_refusal_text(reason: JoinRefusal, server: &str) -> String {
    match reason {
        JoinRefusal::Used => "This link was already used.".to_owned(),
        JoinRefusal::Expired => "This link has expired. Ask for a new one.".to_owned(),
        JoinRefusal::Invalid => "This link is not valid.".to_owned(),
        JoinRefusal::Unreachable => {
            format!("Cannot reach the server at {}.", server_shown(server))
        }
    }
}

/// Why a join did not go through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinRefusal {
    /// The link was already used (criterion 4).
    Used,
    /// The link expired (criteria 3, 4).
    Expired,
    /// Not an invite link, or refused for another reason.
    Invalid,
    /// The server did not answer.
    Unreachable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_are_stable_and_in_range() {
        let a = participant_colour("ABCD EFGH IJKL MNOP");
        assert_eq!(a, participant_colour("ABCD EFGH IJKL MNOP"));
        assert!((1..=6).contains(&a));
        let used: std::collections::BTreeSet<u8> = (0..60)
            .map(|n| participant_colour(&format!("FP{n:03}")))
            .collect();
        assert_eq!(used.len(), 6, "every colour is reached");
    }

    #[test]
    fn initials_take_two_words_or_two_letters() {
        assert_eq!(initials("Ben Ortiz"), "BO");
        assert_eq!(initials("ben"), "BE");
        assert_eq!(initials("Anna Maria Lena"), "AM");
        assert_eq!(initials("é"), "É");
    }

    #[test]
    fn the_marker_and_the_skip_notice_stay_within_their_lengths() {
        assert_eq!(editing_marker(Some("Ben")), "Ben is editing this");
        assert_eq!(editing_marker(None), "Someone is editing this");
        let long = editing_marker(Some(&"x".repeat(24)));
        assert_eq!(long.chars().count(), 36);
        assert!(long.contains('…'));
        assert_eq!(
            skipped_deleted_notice(Some("Ben")),
            "Skipped: Ben deleted that object."
        );
        assert!(
            skipped_deleted_notice(Some(&"y".repeat(24)))
                .chars()
                .count()
                <= 70
        );
        assert_eq!(
            remote_delete_notice(Some("Ben"), RemoteDeleteCase::Selected, 3),
            "Ben deleted 3 selected objects."
        );
    }

    #[test]
    fn server_addresses_are_shown_without_ws() {
        assert_eq!(server_shown("ws://192.168.1.20:8787"), "192.168.1.20:8787");
        assert_eq!(
            server_shown("wss://sync.example.org:443"),
            "wss://sync.example.org:443"
        );
    }

    #[test]
    fn subjects_lines_and_names() {
        assert_eq!(
            share_subject(false, 0, ConnectionState::Connected),
            "Not shared"
        );
        assert_eq!(
            share_subject(true, 2, ConnectionState::Connected),
            "Shared, 2 people"
        );
        assert_eq!(share_subject(true, 2, ConnectionState::Offline), "Offline");
        assert_eq!(
            pill_name(2, ConnectionState::Connected),
            "Collaboration, 2 people, connected"
        );
        assert_eq!(
            ConnectionState::NotConnected.tab_line("ws://192.168.1.20:8080"),
            "Not connected to 192.168.1.20:8080."
        );
        assert_eq!(
            share_confirm_text("ws://192.168.1.20:8080"),
            "Upload this document to 192.168.1.20:8080? It leaves this computer encrypted; the server cannot read it."
        );
    }

    #[test]
    fn resyncing_removal_and_expired_texts() {
        let state = ConnectionState::Resyncing;
        assert_eq!(state.name(), "resyncing");
        assert_eq!(state.pill_text(), "Resyncing...");
        assert_eq!(
            state.tab_line("ws://h:1"),
            "Resyncing from the server's snapshot."
        );
        let text = removal_confirm_text("Ben");
        assert!(text.starts_with("Remove Ben? Removing does not reach back"));
        assert!(removal_confirm_text("").starts_with("Remove This participant?"));
        assert_eq!(
            join_refusal_text(JoinRefusal::Expired, ""),
            "This link has expired. Ask for a new one."
        );
    }

    #[test]
    fn the_expiry_names_the_local_weekday_and_time() {
        // 2026-11-03 13:32 UTC is a Tuesday.
        let at = 1_793_712_720;
        assert_eq!(expiry_text(at, 0), "Expires Tue 13:32.");
        assert_eq!(expiry_text(at, 60), "Expires Tue 14:32.");
        assert_eq!(expiry_text(at, 11 * 60), "Expires Wed 00:32.");
    }
}
