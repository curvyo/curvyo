//! The words of the File tab's Workspace section (`specs/0095-workspace` criteria 1, 4, 6, 8,
//! 13, 14, 16, 17, 23; `ux-notes.md` sections 2, 3 and 10), formatted here so the page shows
//! states and numbers without composing sentences.

use curvyo_document_core::{EntryKind, PlanRefusal};

/// The longest item name a sentence shows whole; longer ones are cut in the middle
/// **(Proposal)**.
const SENTENCE_NAME_MAX: usize = 40;

/// Why a folder was not opened as a workspace, as the host reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenRefusal {
    /// The folder cannot be written (criterion 4).
    NotWritable,
    /// The marker has a higher format number (criterion 23).
    TooNew,
    /// The marker cannot be read.
    Damaged,
    /// The path is no folder, or a recent workspace is gone.
    NotFound,
}

impl OpenRefusal {
    /// The refusal for the host's code; anything unknown counts as not found.
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code {
            "not-writable" => Self::NotWritable,
            "too-new" => Self::TooNew,
            "damaged" => Self::Damaged,
            _ => Self::NotFound,
        }
    }
}

/// The refusal line under Open workspace (`ux-notes.md` section 2.2).
#[must_use]
pub fn open_refusal_text(refusal: OpenRefusal) -> &'static str {
    match refusal {
        OpenRefusal::NotWritable => {
            "Cannot write to this folder. It was not opened as a workspace."
        }
        OpenRefusal::TooNew => "This workspace was made by a newer Curvyo. Nothing was changed.",
        OpenRefusal::Damaged => {
            "The workspace file in this folder is damaged. Nothing was changed."
        }
        OpenRefusal::NotFound => "This folder was not found.",
    }
}

/// The row text of a file that cannot be read (criterion 8): `code` is `"not-curvyo"` or
/// anything else for damage.
#[must_use]
pub fn unreadable_text(code: &str) -> &'static str {
    if code == "not-curvyo" {
        "Cannot be read: not a Curvyo file."
    } else {
        "Cannot be read: damaged."
    }
}

/// The alert of a file operation the host could not run, for its code; nothing was changed
/// **(Proposal)**.
#[must_use]
pub fn operation_failed_text(code: &str) -> &'static str {
    match code {
        "exists" => "An item with this name appeared meanwhile. Nothing was changed.",
        "not-found" => "This item no longer exists. Nothing was changed.",
        _ => "The workspace folder could not be changed. Nothing was changed.",
    }
}

/// The alert when the folder could not follow a change of the shared list: the rest of that
/// change waits for the next scan **(Proposal)**.
#[must_use]
pub fn follow_failed_text() -> &'static str {
    "The workspace folder could not follow a change. Trying again shortly."
}

/// The header line of an open workspace: "14 documents".
#[must_use]
pub fn document_count_text(count: usize) -> String {
    match count {
        1 => "1 document".to_owned(),
        n => format!("{n} documents"),
    }
}

/// The time of a document's last change (criterion 6): "12:03" today, "2 Nov 12:03" before,
/// "2 Nov 2025 12:03" in another year **(Proposal)**. Times are seconds since the Unix epoch,
/// shown in local time `offset_minutes` east of UTC.
#[must_use]
pub fn change_time_text(modified: u64, now: u64, offset_minutes: i32) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let local = |seconds: u64| {
        i64::try_from(seconds).unwrap_or(i64::MAX / 2) + i64::from(offset_minutes) * 60
    };
    let then = local(modified);
    let today = local(now).div_euclid(86_400);
    let day = then.div_euclid(86_400);
    let seconds = then.rem_euclid(86_400);
    let clock = format!("{:02}:{:02}", seconds / 3600, seconds % 3600 / 60);
    if day == today {
        return clock;
    }
    let (year, month, date) = civil_date(day);
    let (this_year, _, _) = civil_date(today);
    // invariant: `civil_date` gives a month in 1..=12.
    let month = MONTHS[usize::try_from(month - 1).unwrap_or(0)];
    if year == this_year {
        format!("{date} {month} {clock}")
    } else {
        format!("{date} {month} {year} {clock}")
    }
}

/// The year, month (1 to 12) and day of the month of `days` since 1970-01-01 (proleptic
/// Gregorian calendar; Howard Hinnant's `civil_from_days`).
fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// A row's accessible name (`ux-notes.md` section 11): "Lid, document, changed 12:03" or
/// "Sides, folder, 6 documents, expanded".
#[must_use]
pub fn row_label(name: &str, kind: EntryKind, count: usize, expanded: bool, time: &str) -> String {
    match kind {
        EntryKind::Document => format!("{name}, document, changed {time}"),
        EntryKind::Folder => format!(
            "{name}, folder, {}, {}",
            document_count_text(count),
            if expanded { "expanded" } else { "collapsed" }
        ),
    }
}

/// The inline confirm of Delete (criterion 17): "Move Lid to the trash?" or "Move Sides and its 6
/// documents to the trash?".
#[must_use]
pub fn delete_confirm_text(name: &str, kind: EntryKind, documents: usize) -> String {
    let name = shown(name);
    match (kind, documents) {
        (EntryKind::Folder, 0) | (EntryKind::Document, _) => format!("Move {name} to the trash?"),
        (EntryKind::Folder, 1) => format!("Move {name} and its 1 document to the trash?"),
        (EntryKind::Folder, n) => format!("Move {name} and its {n} documents to the trash?"),
    }
}

/// The alert when the OS has no trash for the item (criterion 17).
#[must_use]
pub fn no_trash_text(name: &str) -> String {
    format!("The system has no trash. {} was not deleted.", shown(name))
}

/// The inline refusal of a name in the name field (criteria 13, 14).
#[must_use]
pub fn name_refusal_text(refusal: &PlanRefusal, kind: EntryKind) -> String {
    match refusal {
        PlanRefusal::Name(reason) => reason.to_string(),
        PlanRefusal::Exists => match kind {
            EntryKind::Document => "A file with this name exists.".to_owned(),
            EntryKind::Folder => "A folder with this name exists.".to_owned(),
        },
        PlanRefusal::NotFound => "This item no longer exists.".to_owned(),
        PlanRefusal::NotAFolder | PlanRefusal::IntoItself | PlanRefusal::AlreadyThere => {
            "This name is not allowed on this system.".to_owned()
        }
    }
}

/// The alert of a refused move (criterion 16): "Cannot move Sides into itself." or "Sides
/// already has a Lid.". `folder` is the target folder's name (the workspace's for the root).
#[must_use]
pub fn move_refusal_text(refusal: &PlanRefusal, name: &str, folder: &str) -> String {
    let (name, folder) = (shown(name), shown(folder));
    match refusal {
        PlanRefusal::IntoItself => format!("Cannot move {name} into itself."),
        PlanRefusal::Exists => format!("{folder} already has a {name}."),
        PlanRefusal::AlreadyThere => format!("{name} is already in {folder}."),
        PlanRefusal::NotAFolder | PlanRefusal::NotFound | PlanRefusal::Name(_) => {
            "This item no longer exists.".to_owned()
        }
    }
}

/// The status after a creation (criterion 13): "Created Lid 2.".
#[must_use]
pub fn created_text(name: &str) -> String {
    format!("Created {}.", shown(name))
}

/// The status after a move (criterion 16): "Moved Lid to Sides.".
#[must_use]
pub fn moved_text(name: &str, folder: &str) -> String {
    format!("Moved {} to {}.", shown(name), shown(folder))
}

/// The announcement after a delete: "Deleted Lid.".
#[must_use]
pub fn deleted_text(name: &str) -> String {
    format!("Deleted {}.", shown(name))
}

/// `name`, cut in the middle with an ellipsis past [`SENTENCE_NAME_MAX`] characters.
fn shown(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= SENTENCE_NAME_MAX {
        return name.to_owned();
    }
    let head = SENTENCE_NAME_MAX / 2;
    let tail = SENTENCE_NAME_MAX - head - 1;
    let mut cut: String = chars[..head].iter().collect();
    cut.push('…');
    cut.extend(&chars[chars.len() - tail..]);
    cut
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::NameRefusal;

    use super::*;

    /// 2026-11-03 12:03 UTC.
    const NOW: u64 = 1_793_707_380;

    #[test]
    fn change_times_show_the_clock_today_and_the_date_before() {
        assert_eq!(change_time_text(NOW, NOW, 0), "12:03");
        assert_eq!(change_time_text(NOW - 86_400, NOW, 0), "2 Nov 12:03");
        assert_eq!(change_time_text(NOW, NOW, 60), "13:03", "local time");
        assert_eq!(
            change_time_text(NOW - 400 * 86_400, NOW, 0),
            "29 Sep 2025 12:03"
        );
    }

    #[test]
    fn delete_confirms_name_the_item_and_a_folders_documents() {
        assert_eq!(
            delete_confirm_text("Lid", EntryKind::Document, 0),
            "Move Lid to the trash?"
        );
        assert_eq!(
            delete_confirm_text("Sides", EntryKind::Folder, 6),
            "Move Sides and its 6 documents to the trash?"
        );
        assert_eq!(
            delete_confirm_text("Sides", EntryKind::Folder, 1),
            "Move Sides and its 1 document to the trash?"
        );
        assert_eq!(
            no_trash_text("Lid"),
            "The system has no trash. Lid was not deleted."
        );
    }

    #[test]
    fn refusals_say_why() {
        assert_eq!(
            name_refusal_text(
                &PlanRefusal::Name(NameRefusal::Separator),
                EntryKind::Document
            ),
            "A name cannot contain / or \\."
        );
        assert_eq!(
            name_refusal_text(&PlanRefusal::Exists, EntryKind::Document),
            "A file with this name exists."
        );
        assert_eq!(
            move_refusal_text(&PlanRefusal::IntoItself, "Sides", "Sides"),
            "Cannot move Sides into itself."
        );
        assert_eq!(
            move_refusal_text(&PlanRefusal::Exists, "Lid", "Sides"),
            "Sides already has a Lid."
        );
        assert_eq!(
            open_refusal_text(OpenRefusal::from_code("not-writable")),
            "Cannot write to this folder. It was not opened as a workspace."
        );
        assert_eq!(
            unreadable_text("not-curvyo"),
            "Cannot be read: not a Curvyo file."
        );
        assert_eq!(unreadable_text("damaged"), "Cannot be read: damaged.");
        assert_eq!(
            operation_failed_text("exists"),
            "An item with this name appeared meanwhile. Nothing was changed."
        );
    }

    #[test]
    fn labels_and_statuses() {
        assert_eq!(
            row_label("Lid", EntryKind::Document, 0, false, "12:03"),
            "Lid, document, changed 12:03"
        );
        assert_eq!(
            row_label("Sides", EntryKind::Folder, 6, true, ""),
            "Sides, folder, 6 documents, expanded"
        );
        assert_eq!(document_count_text(14), "14 documents");
        assert_eq!(created_text("Lid 2"), "Created Lid 2.");
        assert_eq!(moved_text("Lid", "Sides"), "Moved Lid to Sides.");
        let long = "a".repeat(60);
        assert_eq!(created_text(&long).chars().count(), "Created .".len() + 40);
    }
}
