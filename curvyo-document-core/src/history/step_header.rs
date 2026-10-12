//! The grammar of a commit message: `<label>;s=<seq>;t=<seconds>[;<key>=<value>]...` (ADR 0014 §2).

use crate::units::Timestamp;

/// The reserved label of the commit that holds a new document's root setup (ADR 0014 context).
pub const LABEL_NEW_DOCUMENT: &str = "new_document";
/// The reserved label of an undo (`u` names the step it takes back).
pub const LABEL_UNDO: &str = "undo";
/// The reserved label of a redo (`d` names the step it applies again).
pub const LABEL_REDO: &str = "redo";
/// The reserved label of a history wipe (milestone 4).
pub const LABEL_WIPE: &str = "wipe_history";

/// The reserved label of an object undo (`ou` names the step it takes back, `o` the object).
pub const LABEL_OBJECT_UNDO: &str = "object_undo";
/// The reserved label of an object redo (`od` names the step it applies again, `o` the object).
pub const LABEL_OBJECT_REDO: &str = "object_redo";
/// The reserved label of the wipe of one object's history (`o` names the object).
pub const LABEL_WIPE_OBJECT: &str = "wipe_object_history";

/// The label of a go to version (`specs/0042-history-branches` criterion 10; `o` the object, `v`
/// the step whose version it went to).
pub const LABEL_GO_TO_VERSION: &str = "go_to_version";
/// The label of a clone of a version (criterion 22; `o` the source, `v` the step).
pub const LABEL_CLONE_VERSION: &str = "clone_version";
/// The key of the step whose version a go to or a clone names.
pub(crate) const KEY_VERSION: &str = "v";

/// The key of the step counter.
const KEY_SEQ: &str = "s";
/// The key of the step time.
const KEY_TIME: &str = "t";
/// The key of the step an undo takes back.
pub(crate) const KEY_UNDONE: &str = "u";
/// The key of the step a redo applies again.
pub(crate) const KEY_REDONE: &str = "d";
/// The key of the step an object undo takes back.
pub(crate) const KEY_OBJECT_UNDONE: &str = "ou";
/// The key of the step an object redo applies again.
pub(crate) const KEY_OBJECT_REDONE: &str = "od";
/// The key of the object an object undo, redo or wipe is about.
pub(crate) const KEY_OBJECT: &str = "o";

/// A parsed commit message that carries a header. A message without one is a legacy label, see
/// [`StepHeader::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepHeader {
    /// The commit label (`translate_objects`, `undo`, ...); contains no `;`.
    pub label: String,
    /// The session's step counter.
    pub seq: u64,
    /// The step's wall-clock time; unknown when the header has none.
    pub time: Timestamp,
    /// Further `key=value` pairs, in order. Unknown keys are kept and ignored.
    pub extra: Vec<(String, String)>,
}

impl StepHeader {
    /// A header without extra keys.
    pub(crate) fn new(label: &str, seq: u64, time: Timestamp) -> Self {
        Self {
            label: label.to_owned(),
            seq,
            time,
            extra: Vec::new(),
        }
    }

    /// The same header with one more `key=value` pair.
    #[cfg(test)]
    pub(crate) fn with(mut self, key: &str, value: impl Into<String>) -> Self {
        self.extra.push((key.to_owned(), value.into()));
        self
    }

    /// The same header with `key` set to `value`, in place when it is there.
    pub(crate) fn set(&mut self, key: &str, value: String) {
        match self.extra.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => self.extra.push((key.to_owned(), value)),
        }
    }

    /// The value of the extra key `key`.
    pub(crate) fn extra_value(&self, key: &str) -> Option<&str> {
        self.extra
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The commit message form of the header.
    pub(crate) fn format(&self) -> String {
        let mut text = format!(
            "{};{KEY_SEQ}={};{KEY_TIME}={}",
            self.label,
            self.seq,
            self.time.as_secs()
        );
        for (key, value) in &self.extra {
            text.push(';');
            text.push_str(key);
            text.push('=');
            text.push_str(value);
        }
        text
    }

    /// Parses a commit message. `None` is a legacy message: it has no `;`, or its first field after
    /// the label is not a step counter. A header without a time reads as the unknown time.
    pub(crate) fn parse(message: &str) -> Option<Self> {
        let mut fields = message.split(';');
        let label = fields.next()?;
        let mut seq = None;
        let mut time = Timestamp::UNKNOWN;
        let mut extra = Vec::new();
        for (index, field) in fields.enumerate() {
            let (key, value) = field.split_once('=')?;
            match (index, key) {
                (0, KEY_SEQ) => seq = Some(value.parse().ok()?),
                (0, _) => return None,
                (_, KEY_TIME) => time = Timestamp::from_secs(value.parse().ok()?),
                _ => extra.push((key.to_owned(), value.to_owned())),
            }
        }
        Some(Self {
            label: label.to_owned(),
            seq: seq?,
            time,
            extra,
        })
    }
}

/// Whether a commit with the message `next` continues the step whose last commit had `prev`: the
/// same message, or a further press of one go-to exploration, which repeats the header with a new
/// `v` (same label, step counter and object; `specs/0042-history-branches` criterion 11).
pub(crate) fn continues(prev: &str, next: &str) -> bool {
    if prev == next {
        return true;
    }
    let (Some(a), Some(b)) = (StepHeader::parse(prev), StepHeader::parse(next)) else {
        return false;
    };
    a.label == LABEL_GO_TO_VERSION
        && b.label == LABEL_GO_TO_VERSION
        && a.seq == b.seq
        && a.extra_value(KEY_OBJECT) == b.extra_value(KEY_OBJECT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_further_go_to_press_on_the_same_object_continues_the_step() {
        let first = "go_to_version;s=4;t=9;o=1:2;v=1@5";
        assert!(continues(first, first));
        assert!(continues(first, "go_to_version;s=4;t=9;o=1:2;v=1@9"));
        assert!(
            !continues(first, "go_to_version;s=5;t=9;o=1:2;v=1@9"),
            "a new step"
        );
        assert!(
            !continues(first, "go_to_version;s=4;t=9;o=1:3;v=1@9"),
            "another object"
        );
        assert!(!continues("move;s=4;t=9", "move;s=4;t=9;x=1"));
    }

    #[test]
    fn a_header_round_trips_with_extra_keys() {
        let header = StepHeader::new("undo", 12, Timestamp::from_secs(1_760_000_000))
            .with("u", "7@3")
            .with("x", "y");
        let text = header.format();
        assert_eq!(text, "undo;s=12;t=1760000000;u=7@3;x=y");
        assert_eq!(StepHeader::parse(&text), Some(header));
    }

    #[test]
    fn a_message_without_a_header_is_a_legacy_label() {
        assert_eq!(StepHeader::parse("translate_objects"), None);
        assert_eq!(StepHeader::parse("test: legacy node"), None);
        assert_eq!(StepHeader::parse("a;b"), None);
        assert_eq!(StepHeader::parse("a;t=5;s=1"), None);
        assert_eq!(StepHeader::parse("a;s=x"), None);
    }

    #[test]
    fn a_header_without_a_time_has_the_unknown_time() {
        let header = StepHeader::parse("move;s=3").unwrap();
        assert_eq!(header.seq, 3);
        assert_eq!(header.time, Timestamp::UNKNOWN);
    }
}
