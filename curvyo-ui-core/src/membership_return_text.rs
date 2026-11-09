//! The words of the Removed block in the Share tab (`specs/0097-membership-lifecycle` ux-notes
//! section 7): its heading; the way back is a new invite (customer 2026-11-09).

use crate::membership_text::{middle_cut, named};

/// A file's name in a sentence, "this document" when the file has none yet (a joined copy
/// never saved).
fn file_name(name: &str) -> String {
    if name.trim().is_empty() {
        "this document".to_owned()
    } else {
        middle_cut(name.trim(), 24)
    }
}

/// The Removed block's heading (criteria 13 and 20, section 7.1).
#[must_use]
pub fn ended_heading(name: &str, left: bool) -> String {
    let name = file_name(name);
    if left {
        format!("You left {name}")
    } else {
        format!("You were removed from {name}")
    }
}

/// The canvas notice after an add back (section 8.3).
#[must_use]
pub fn added_back_notice(label: &str) -> String {
    format!("{} was added back.", named(label))
}

/// Every text of this module with sample inputs, for the wording test of criterion 34.
#[cfg(test)]
fn every_text() -> Vec<String> {
    vec![
        crate::share_text::REMOVED_NOTICE.to_owned(),
        crate::share_text::LEFT_NOTICE.to_owned(),
        ended_heading("Poster", true),
        ended_heading("Poster", false),
        added_back_notice("Ben"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 34: no text of the share flow says "revoked" or "lost access", and none says
    /// "can no longer read" (or "cannot read") without saying what stays.
    #[test]
    fn no_text_of_the_share_flow_says_revoked_or_lost_access() {
        let mut texts = every_text();
        texts.extend(crate::membership_text::every_text());
        assert!(texts.len() > 30, "{}", texts.len());
        for text in &texts {
            let lower = text.to_lowercase();
            assert!(!lower.contains("revoked"), "{text}");
            assert!(!lower.contains("lost access"), "{text}");
            assert!(!lower.contains("access lost"), "{text}");
            for phrase in ["can no longer read", "cannot read"] {
                assert!(
                    !lower.contains(phrase)
                        || lower.contains("still holds")
                        || lower.contains("keeps"),
                    "{text}"
                );
            }
        }
    }

    /// A file without a name is "this document".
    #[test]
    fn a_file_without_a_name_is_this_document() {
        assert_eq!(
            ended_heading("", false),
            "You were removed from this document"
        );
        assert_eq!(ended_heading(" ", true), "You left this document");
        assert_eq!(
            ended_heading("Box job", false),
            "You were removed from Box job"
        );
    }
}
