//! The words of the image clipboard and of a drop (`specs/0087-image-clipboard-and-drop` criteria
//! 5, 14, 15 and 21; UX notes §2 to §4). The user page `docs/importing-images.md` quotes the same strings
//! (criterion 18).

/// A paste with no image on the clipboard (criterion 5).
pub const NOTHING_TO_PASTE_NOTICE: &str = "Nothing to paste: the clipboard has no image.";
/// The busy line of a large paste (UX notes §2).
pub const PASTING_NOTICE: &str = "Pasting image...";
/// A copy that has not finished after 400 ms (criterion 14).
pub const COPYING_NOTICE: &str = "Copying...";
/// A browser without `ClipboardItem` (criterion 15).
pub const COPY_NEEDS_NEWER_BROWSER_NOTICE: &str = "Copying images needs a newer browser.";

/// A drop that carries only a link: nothing is fetched (criterion 21, customer Frage 68 A).
pub const LINK_DROP_NOTICE: &str = "That drop is a link, not an image. Nothing was downloaded.";

/// The longest notice of a failed copy in characters (criterion 15).
const MAX_FAILED_CHARS: usize = 70;

/// `"Could not copy the image: <reason>."` with the reason's trailing full stop dropped and the
/// reason cut with "..." so that the notice has at most 70 characters (criterion 15).
#[must_use]
pub fn copy_failed_notice(reason: &str) -> String {
    const HEAD: &str = "Could not copy the image: ";
    let reason = reason.trim().trim_end_matches('.').trim_end();
    let room = MAX_FAILED_CHARS - HEAD.len() - 1;
    let reason: String = if reason.chars().count() > room {
        let mut cut: String = reason.chars().take(room - 3).collect();
        cut.truncate(cut.trim_end().len());
        cut + "..."
    } else {
        reason.to_owned()
    };
    format!("{HEAD}{reason}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_notices_have_the_spec_lengths() {
        assert_eq!(NOTHING_TO_PASTE_NOTICE.chars().count(), 45);
    }

    #[test]
    fn a_failed_copy_names_a_short_reason_within_70_characters() {
        assert_eq!(
            copy_failed_notice("Write permission denied."),
            "Could not copy the image: Write permission denied."
        );
        let long = copy_failed_notice(&"the clipboard is held by another program ".repeat(3));
        assert_eq!(long.chars().count(), 70);
        assert!(long.ends_with("..."), "{long}");
        assert!(long.starts_with("Could not copy the image: the clipboard is held"));
    }
}
