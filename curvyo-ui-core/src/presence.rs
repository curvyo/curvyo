//! The presence payload one participant sends the others (ADR 0031 decision 7, `specs/0092`
//! criteria 7, 14 to 16): never document content, never saved, never undoable.
//!
//! It travels JSON-encoded inside a sealed presence envelope; this module holds its shape and
//! the checks a received one passes before anything draws it.

use curvyo_document_core::NodeId;
use serde::{Deserialize, Serialize};

/// The payload version this build writes and reads.
pub const PRESENCE_VERSION: u8 = 1;
/// A participant's label is cut to this many characters (UX notes §11).
pub const MAX_LABEL_CHARS: usize = 24;
/// More selected objects than this are not outlined one by one; the box covers them anyway.
const MAX_SELECTION: usize = 10_000;
/// Coordinates beyond this are not drawn (millimetres).
const MAX_COORDINATE_MM: f64 = 1e9;

/// One participant's current state, sent whole each time it changes and as a heartbeat. Every
/// message carries the hello (label and Loro peer id), so a late receiver needs no other
/// message to name its pointer (Proposal).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PresencePayload {
    /// [`PRESENCE_VERSION`].
    pub v: u8,
    /// The "Name shown to others".
    pub label: String,
    /// The session's Loro peer id as decimal text (a JSON number would lose precision), so the
    /// History tab can name the participant's steps (ADR 0031 decision 4).
    pub peer: String,
    /// The pointer in document millimetres, `None` while it is off the canvas.
    pub pointer: Option<[f64; 2]>,
    /// The selected objects.
    pub selection: Vec<NodeId>,
    /// The active tool's name (`select`, `pen`, ...), for the small tool icon (criterion 16).
    pub tool: String,
    /// The offset of the selection's move drag in flight, in millimetres (criterion 7).
    pub drag: Option<[f64; 2]>,
    /// In a shared workspace, the open document's id as hex, so that marks are drawn only in
    /// the same document (`specs/0095-workspace`, ADR 0037 decision 5: it travels inside the
    /// seal); empty for a document shared on its own.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub document: String,
    /// In a shared workspace, the Loro peer id of this device's list replica as decimal text,
    /// so that the notice of a rename or move names its author (`specs/0095-workspace`
    /// `ux-notes.md` section 7); empty for a document shared on its own.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub list_peer: String,
}

impl PresencePayload {
    /// The session's peer id, if the text is one.
    #[must_use]
    pub fn peer_id(&self) -> Option<u64> {
        self.peer.parse().ok()
    }

    /// The list replica's peer id, if the text is one.
    #[must_use]
    pub fn list_peer_id(&self) -> Option<u64> {
        self.list_peer.parse().ok()
    }

    /// The payload as it may be drawn: `None` for another version; the label cut to
    /// [`MAX_LABEL_CHARS`] characters, a non-finite or absurd pointer or drag dropped, an
    /// overlong selection cut.
    #[must_use]
    pub fn checked(mut self) -> Option<Self> {
        if self.v != PRESENCE_VERSION {
            return None;
        }
        self.label = self.label.trim().chars().take(MAX_LABEL_CHARS).collect();
        let sane = |p: &[f64; 2]| {
            p.iter()
                .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE_MM)
        };
        self.pointer = self.pointer.filter(sane);
        self.drag = self.drag.filter(sane);
        self.selection.truncate(MAX_SELECTION);
        self.tool.truncate(16);
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> PresencePayload {
        PresencePayload {
            v: PRESENCE_VERSION,
            label: "  Ben  ".to_owned(),
            peer: "18446744073709551615".to_owned(),
            pointer: Some([10.0, 20.0]),
            selection: Vec::new(),
            tool: "select".to_owned(),
            drag: Some([f64::NAN, 1.0]),
            document: String::new(),
            list_peer: String::new(),
        }
    }

    #[test]
    fn a_checked_payload_keeps_what_can_be_drawn() {
        let checked = payload().checked().unwrap();
        assert_eq!(checked.label, "Ben");
        assert_eq!(checked.pointer, Some([10.0, 20.0]));
        assert_eq!(checked.drag, None, "a non-finite drag is dropped");
        assert_eq!(checked.peer_id(), Some(u64::MAX));
    }

    #[test]
    fn another_version_is_not_drawn_and_a_long_label_is_cut() {
        let mut other = payload();
        other.v = 2;
        assert!(other.checked().is_none());
        let mut long = payload();
        long.label = "x".repeat(40);
        assert_eq!(
            long.checked().unwrap().label.chars().count(),
            MAX_LABEL_CHARS
        );
    }
}
