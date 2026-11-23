//! The words of the recovery block in the File tab (`specs/0136-autosave-and-restore` criteria
//! 4 to 7, 9 and 13, `ux-notes.md` sections 3 to 9): row lines, the damaged look, the help line,
//! accessible names and the failed-write sentences. The page passes the time as the local
//! broken-down parts, so this stays free of any clock or zone.

/// A local date and time, as the page's zone reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    /// The year.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u32,
    /// The day, 1 to 31.
    pub day: u32,
    /// The hour, 0 to 23.
    pub hour: u32,
    /// The minute, 0 to 59.
    pub minute: u32,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// "12:03" the same day, "2 Nov 12:03" the same year, "29 Sep 2025 12:03" earlier.
#[must_use]
pub fn time_text(at: LocalTime, now: LocalTime) -> String {
    let clock = format!("{:02}:{:02}", at.hour, at.minute);
    let month = MONTHS[(at.month.clamp(1, 12) - 1) as usize];
    if at.year == now.year && at.month == now.month && at.day == now.day {
        clock
    } else if at.year == now.year {
        format!("{} {month} {clock}", at.day)
    } else {
        format!("{} {month} {} {clock}", at.day, at.year)
    }
}

/// "1 page" or "3 pages".
#[must_use]
pub fn pages_text(pages: u32) -> String {
    if pages == 1 {
        "1 page".to_owned()
    } else {
        format!("{pages} pages")
    }
}

/// Why a copy cannot be restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unrestorable {
    /// Truncated or unreadable.
    Damaged,
    /// Made by a newer Curvyo.
    Newer,
}

/// Line 2 of a row: "12:03 · 3 pages", or the damaged sentence (the open errors of `0001`
/// criterion 7).
#[must_use]
pub fn row_line_two(
    at: LocalTime,
    now: LocalTime,
    pages: u32,
    damaged: Option<Unrestorable>,
) -> String {
    match damaged {
        Some(Unrestorable::Damaged) => "Cannot be restored: damaged.".to_owned(),
        Some(Unrestorable::Newer) => "Cannot be restored: made by a newer Curvyo.".to_owned(),
        None => format!("{} · {}", time_text(at, now), pages_text(pages)),
    }
}

/// Line 3 of a row, only when criterion 6 applies.
pub const FILE_CHANGED_LINE: &str = "File changed since";

/// The name of a row: the file's name or "Untitled".
#[must_use]
pub fn row_name(name: Option<&str>) -> &str {
    name.filter(|n| !n.is_empty()).unwrap_or("Untitled")
}

/// The name of a restored project: "Poster (recovered)" or "Untitled" (criterion 5 **(Proposal)**).
#[must_use]
pub fn recovered_name(name: Option<&str>) -> String {
    match name.filter(|n| !n.is_empty()) {
        Some(name) => {
            let stem = name.strip_suffix(".curvyo").unwrap_or(name);
            format!("{stem} (recovered)")
        }
        None => "Untitled".to_owned(),
    }
}

/// How a button names its row: the file's name, or "Untitled from 12:03".
#[must_use]
pub fn row_subject(name: Option<&str>, time: &str) -> String {
    match name.filter(|n| !n.is_empty()) {
        Some(name) => name.to_owned(),
        None => format!("Untitled from {time}"),
    }
}

/// The accessible name of a row.
#[must_use]
pub fn row_label(
    name: Option<&str>,
    time: &str,
    pages: u32,
    changed: bool,
    damaged: Option<Unrestorable>,
) -> String {
    let mut label = format!(
        "{}, last change {time}, {}",
        row_name(name),
        pages_text(pages)
    );
    if changed {
        label.push_str(", file changed since");
    }
    match damaged {
        Some(Unrestorable::Damaged) => label.push_str(", cannot be restored: damaged"),
        Some(Unrestorable::Newer) => label.push_str(", cannot be restored: made by a newer Curvyo"),
        None => {}
    }
    label
}

/// The Restore button's name.
#[must_use]
pub fn restore_label(subject: &str) -> String {
    format!("Restore {subject}")
}

/// The Discard button's name, unarmed.
#[must_use]
pub fn discard_label(subject: &str) -> String {
    format!("Discard {subject}")
}

/// The Discard button's name, armed.
#[must_use]
pub fn discard_armed_label(subject: &str) -> String {
    format!("Press again to discard {subject} for good.")
}

/// The block's heading.
pub const BLOCK_HEADING: &str = "Unsaved work from a crash";

/// The help line: "Kept in `<path>`" on the desktop, "Kept in this browser" in the browser.
#[must_use]
pub fn help_line(path: Option<&str>) -> String {
    match path {
        Some(path) => format!("Kept in {path}"),
        None => "Kept in this browser".to_owned(),
    }
}

/// Why a recovery write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteFailure {
    /// No space left.
    DiskFull,
    /// The app data folder refuses the write.
    NoPermission,
    /// The browser's storage is full.
    BrowserFull,
    /// Anything else, with the system's reason.
    Other(String),
}

/// The failed-write sentence of criterion 13 (the File tab line and the one canvas notice).
#[must_use]
pub fn failure_sentence(failure: &WriteFailure) -> String {
    match failure {
        WriteFailure::DiskFull => "Recovery copy not written: the disk is full.".to_owned(),
        WriteFailure::NoPermission => {
            "Recovery copy not written: no permission for the app data folder.".to_owned()
        }
        WriteFailure::BrowserFull => {
            "Recovery copy not written: the browser's storage is full.".to_owned()
        }
        WriteFailure::Other(reason) => {
            let reason = reason.trim().trim_end_matches('.');
            format!("Recovery copy not written: {reason}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: LocalTime = LocalTime {
        year: 2026,
        month: 11,
        day: 23,
        hour: 14,
        minute: 0,
    };

    fn at(year: i32, month: u32, day: u32) -> LocalTime {
        LocalTime {
            year,
            month,
            day,
            hour: 12,
            minute: 3,
        }
    }

    #[test]
    fn the_time_follows_the_workspace_row_format() {
        assert_eq!(time_text(at(2026, 11, 23), NOW), "12:03");
        assert_eq!(time_text(at(2026, 11, 2), NOW), "2 Nov 12:03");
        assert_eq!(time_text(at(2025, 9, 29), NOW), "29 Sep 2025 12:03");
    }

    #[test]
    fn line_two_has_time_and_pages_or_the_damaged_sentence() {
        let t = at(2026, 11, 23);
        assert_eq!(row_line_two(t, NOW, 1, None), "12:03 · 1 page");
        assert_eq!(row_line_two(t, NOW, 3, None), "12:03 · 3 pages");
        assert_eq!(
            row_line_two(t, NOW, 3, Some(Unrestorable::Damaged)),
            "Cannot be restored: damaged."
        );
        assert_eq!(
            row_line_two(t, NOW, 3, Some(Unrestorable::Newer)),
            "Cannot be restored: made by a newer Curvyo."
        );
    }

    #[test]
    fn names_and_labels_follow_the_ux_notes() {
        assert_eq!(row_name(None), "Untitled");
        assert_eq!(recovered_name(Some("Poster.curvyo")), "Poster (recovered)");
        assert_eq!(recovered_name(None), "Untitled");
        assert_eq!(row_subject(None, "12:03"), "Untitled from 12:03");
        assert_eq!(
            row_label(Some("Poster.curvyo"), "12:03", 3, true, None),
            "Poster.curvyo, last change 12:03, 3 pages, file changed since"
        );
        assert_eq!(
            discard_armed_label("Poster.curvyo"),
            "Press again to discard Poster.curvyo for good."
        );
    }

    #[test]
    fn the_help_line_names_the_place() {
        assert_eq!(help_line(None), "Kept in this browser");
        assert_eq!(
            help_line(Some("~/.local/share/curvyo/recovery")),
            "Kept in ~/.local/share/curvyo/recovery"
        );
    }

    #[test]
    fn the_failure_sentences_name_the_cause() {
        assert_eq!(
            failure_sentence(&WriteFailure::DiskFull),
            "Recovery copy not written: the disk is full."
        );
        assert_eq!(
            failure_sentence(&WriteFailure::Other("broken pipe.".to_owned())),
            "Recovery copy not written: broken pipe."
        );
    }
}
