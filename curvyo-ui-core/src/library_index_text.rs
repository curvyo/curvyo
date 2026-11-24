//! The words of the document library (`specs/0137-document-library` UX notes sections 3, 4, 7,
//! 8 and 10): row lines, read errors, import notices and the Came from line, formatted here so
//! every build says the same.

use curvyo_document_core::{CameFrom, DocumentSize};

use crate::library_index::DocumentState;
use crate::workspace_text::change_time_text;

/// Why a whole source cannot be read, as the host reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceReason {
    /// The folder is not there.
    NotFound,
    /// The folder may not be read.
    NoAccess,
    /// Another read error (a drive that is not connected).
    Io,
}

impl SourceReason {
    /// The reason of the host's code (`not_found`, `no_access`, `io`).
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code {
            "not_found" => Self::NotFound,
            "no_access" => Self::NoAccess,
            _ => Self::Io,
        }
    }
}

/// A source row's second line: "Cannot be read: folder not found.".
#[must_use]
pub const fn source_error_text(reason: SourceReason) -> &'static str {
    match reason {
        SourceReason::NotFound => "Cannot be read: folder not found.",
        SourceReason::NoAccess => "Cannot be read: no access.",
        SourceReason::Io => "Cannot be read: drive not connected.",
    }
}

/// A document row's second line when it cannot be read; `None` for a readable one.
#[must_use]
pub const fn document_error_text(state: DocumentState) -> Option<&'static str> {
    match state {
        DocumentState::Readable => None,
        DocumentState::NotCurvyo => Some("Cannot be read: not a Curvyo file."),
        DocumentState::Damaged => Some("Cannot be read: damaged."),
        DocumentState::Newer => Some("Cannot be read: made by a newer Curvyo."),
    }
}

/// A document row's second line: "3 pages · 2 Nov 12:03", with "Saved version · " ahead while
/// this window's document has unsaved changes (criterion 11). Times are Unix milliseconds.
#[must_use]
pub fn document_line(
    pages: usize,
    modified: i64,
    now: i64,
    offset_minutes: i32,
    saved: bool,
) -> String {
    let seconds = |ms: i64| u64::try_from(ms.div_euclid(1000)).unwrap_or(0);
    let count = match pages {
        1 => "1 page".to_owned(),
        n => format!("{n} pages"),
    };
    let time = change_time_text(seconds(modified), seconds(now), offset_minutes);
    if saved {
        format!("Saved version · {count} · {time}")
    } else {
        format!("{count} · {time}")
    }
}

/// A page row's size line: "210 × 297 mm".
#[must_use]
pub fn page_size_text(size: DocumentSize) -> String {
    let mm = |v: f64| {
        let rounded = (v * 10.0).round() / 10.0;
        if (rounded - rounded.round()).abs() < 1e-9 {
            format!("{rounded:.0}")
        } else {
            format!("{rounded:.1}")
        }
    };
    format!(
        "{} × {} mm",
        mm(size.width.as_mm()),
        mm(size.height.as_mm())
    )
}

/// The search's empty result: 'No document or page matches "lid".'.
#[must_use]
pub fn no_match_text(query: &str) -> String {
    format!("No document or page matches \"{}\".", query.trim())
}

/// The empty library.
pub const NO_DOCUMENTS_TEXT: &str = "No documents yet.";

/// The refusal of Add folder for a folder already listed.
#[must_use]
pub fn already_in_text(name: &str) -> String {
    format!("{name} is already in the library.")
}

/// The refusal of Add folder for a workspace already listed.
#[must_use]
pub fn is_workspace_text(name: &str) -> String {
    format!("{name} is a workspace. It is already listed.")
}

/// The armed Remove's name: "Press again to remove Logos from the library".
#[must_use]
pub fn remove_armed_text(name: &str) -> String {
    format!("Press again to remove {name} from the library")
}

/// The notice after an import whose linked clones lost their originals (UX notes section 7.2).
#[must_use]
pub fn clones_became_paths_text(count: usize) -> Option<String> {
    match count {
        0 => None,
        1 => Some("1 linked clone became a path: its original was not picked.".to_owned()),
        n => Some(format!(
            "{n} linked clones became paths: their originals were not picked."
        )),
    }
}

/// The notice after an import whose texts on a path left their carriers behind (like the
/// unpicked clone of [`clones_became_paths_text`]).
#[must_use]
pub fn texts_released_text(count: usize) -> Option<String> {
    match count {
        0 => None,
        1 => Some("1 text left its path: its path was not picked.".to_owned()),
        n => Some(format!(
            "{n} texts left their paths: their paths were not picked."
        )),
    }
}

/// The notice of a page that came in as a drawing page (UX notes section 7.1).
#[must_use]
pub fn drawing_fallback_text(page: &str, reason: &str) -> String {
    format!("{page} came in as a drawing page: {reason}.")
}

/// Why a page came in as a drawing page, the part after the colon of
/// [`drawing_fallback_text`].
#[must_use]
pub fn fallback_reason_text(fallback: &curvyo_document_core::PageFallback) -> String {
    match fallback {
        curvyo_document_core::PageFallback::UnknownKind(kind) => {
            format!("page kind {kind} is unknown")
        }
        curvyo_document_core::PageFallback::NoMachineData => {
            "its machine data cannot be read".to_owned()
        }
    }
}

/// The alert of an import that failed (UX notes section 10).
#[must_use]
pub fn import_failed_text(name: &str, reason: &str) -> String {
    format!("{name} could not be imported: {reason}. Nothing was changed.")
}

/// The refusal of elements on a sketch page (UX notes section 7.2).
pub const SKETCH_PAGE_REFUSAL_TEXT: &str = "Elements cannot go onto a sketch page.";

/// The preview pane's announcement of the ringed object (UX notes 6.4): "Rectangle 3f9a1c, 2 of
/// 42, picked"; `at` counts from 1.
#[must_use]
pub fn pick_announcement(name: &str, at: usize, of: usize, picked: bool) -> String {
    let tail = if picked { ", picked" } else { "" };
    format!("{name}, {at} of {of}{tail}")
}

/// The preview pane footer's count (UX notes 6.2): "Nothing picked", "1 of 1 element picked",
/// "3 of 42 elements picked".
#[must_use]
pub fn picked_text(picked: usize, of: usize) -> String {
    match (picked, of) {
        (0, _) => "Nothing picked".to_owned(),
        (p, 1) => format!("{p} of 1 element picked"),
        (p, n) => format!("{p} of {n} elements picked"),
    }
}

/// "Imported 24 Nov 2026 12:03" (UX notes 8.2): the import time with its year, in local time
/// `offset_minutes` east of UTC.
#[must_use]
pub fn imported_text(at: i64, offset_minutes: i32) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let local = at.div_euclid(1000) + i64::from(offset_minutes) * 60;
    let (year, month, day) = crate::workspace_text::civil_date(local.div_euclid(86_400));
    let seconds = local.rem_euclid(86_400);
    let month = MONTHS[usize::try_from(month - 1).unwrap_or(0).min(11)];
    format!(
        "Imported {day} {month} {year} {:02}:{:02}",
        seconds / 3600,
        seconds % 3600 / 60
    )
}

/// The accessible name of a source row (UX notes 2.1, 4.2): "Box job, open workspace, 14
/// documents", "Box job, workspace in this browser, 14 documents", "Logos, folder, 31
/// documents", or with `cannot be read: <reason>` in place of the count.
#[must_use]
pub fn source_label(
    name: &str,
    kind: &str,
    documents: usize,
    error: Option<SourceReason>,
) -> String {
    match error {
        Some(reason) => {
            let why = source_error_text(reason)
                .trim_start_matches("Cannot be read: ")
                .trim_end_matches('.');
            format!("{name}, {kind}, cannot be read: {why}")
        }
        None => format!(
            "{name}, {kind}, {}",
            crate::workspace_text::document_count_text(documents)
        ),
    }
}

/// The accessible name of a document row (UX notes 4.2): "Lid, document, 3 pages, changed 2 Nov
/// 12:03", or "Lid, document, cannot be read: damaged".
#[must_use]
pub fn document_label(name: &str, pages: usize, time: &str, error: Option<&str>) -> String {
    if let Some(error) = error {
        let why = error
            .trim_start_matches("Cannot be read: ")
            .trim_end_matches('.');
        return format!("{name}, document, cannot be read: {why}");
    }
    let count = if pages == 1 {
        "1 page".to_owned()
    } else {
        format!("{pages} pages")
    };
    format!("{name}, document, {count}, changed {time}")
}

/// The accessible name of a page row (UX notes 4.2): "Front, drawing page, 210 by 297
/// millimetres, in Lid".
#[must_use]
pub fn page_label(name: &str, kind: &str, size: DocumentSize, document: &str) -> String {
    let mm = |v: f64| {
        let rounded = (v * 10.0).round() / 10.0;
        if (rounded - rounded.round()).abs() < 1e-9 {
            format!("{rounded:.0}")
        } else {
            format!("{rounded:.1}")
        }
    };
    format!(
        "{name}, {kind} page, {} by {} millimetres, in {document}",
        mm(size.width.as_mm()),
        mm(size.height.as_mm())
    )
}

/// The Came from line: "From Box job › Lid".
#[must_use]
pub fn came_from_text(record: &CameFrom) -> String {
    format!("From {} › {}", record.document, record.page)
}

/// The Came from line's tooltip, and the Layers tooltip line: "From Box job › Lid, 24 Nov
/// 12:03".
#[must_use]
pub fn came_from_tooltip(record: &CameFrom, now: i64, offset_minutes: i32) -> String {
    let seconds = |ms: i64| u64::try_from(ms.div_euclid(1000)).unwrap_or(0);
    format!(
        "{}, {}",
        came_from_text(record),
        change_time_text(seconds(record.at.get()), seconds(now), offset_minutes)
    )
}

/// The Came from line of several selected objects (UX notes section 8.2): the line of one
/// source, "2 of 5 from Box job › Lid" when some have none, "From 3 documents" for several;
/// `None` when none has a record. The bool says whether Open source is offered.
#[must_use]
pub fn came_from_selection_text(records: &[Option<CameFrom>]) -> Option<(String, bool)> {
    let with: Vec<&CameFrom> = records.iter().flatten().collect();
    let first = with.first()?;
    let same = with
        .iter()
        .all(|r| r.page_id == first.page_id && r.document == first.document);
    if !same {
        let mut documents: Vec<&str> = with.iter().map(|r| r.document.as_str()).collect();
        documents.sort_unstable();
        documents.dedup();
        return Some((format!("From {} documents", documents.len().max(2)), false));
    }
    if with.len() == records.len() {
        Some((came_from_text(first), true))
    } else {
        Some((
            format!(
                "{} of {} from {} › {}",
                with.len(),
                records.len(),
                first.document,
                first.page
            ),
            true,
        ))
    }
}

/// The notice of Open source when the page is gone from the source (UX notes section 8.3).
#[must_use]
pub fn page_gone_text(document: &str, page: &str) -> String {
    format!("{document} has no page {page} any more.")
}

/// The alert of Open source when the file went (UX notes section 8.3).
#[must_use]
pub fn source_gone_text(document: &str) -> String {
    format!("{document} cannot be found any more.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::UnixMillis;

    fn record(document: &str, page_id: &str) -> CameFrom {
        CameFrom {
            document: document.into(),
            page: "Lid".into(),
            page_id: page_id.into(),
            item: None,
            at: UnixMillis::new(0),
        }
    }

    #[test]
    fn rows_say_pages_time_and_saved_version() {
        let day = 86_400_000;
        assert_eq!(
            document_line(3, 12 * 3_600_000, day / 2, 0, false),
            "3 pages · 12:00"
        );
        assert_eq!(
            document_line(1, 0, 0, 0, true),
            "Saved version · 1 page · 00:00"
        );
        assert_eq!(
            page_size_text(DocumentSize::from_mm(210.0, 297.0)),
            "210 × 297 mm"
        );
        assert_eq!(
            page_size_text(DocumentSize::from_mm(80.25, 40.0)),
            "80.3 × 40 mm"
        );
    }

    #[test]
    fn the_pane_and_came_from_words() {
        assert_eq!(
            pick_announcement("Rectangle 3f9a1c", 2, 42, true),
            "Rectangle 3f9a1c, 2 of 42, picked"
        );
        assert_eq!(picked_text(0, 4), "Nothing picked");
        assert_eq!(picked_text(1, 1), "1 of 1 element picked");
        assert_eq!(picked_text(3, 42), "3 of 42 elements picked");
        assert_eq!(
            imported_text(1_764_000_000_000, 0),
            "Imported 24 Nov 2025 16:00"
        );
        assert_eq!(
            source_label("Old jobs", "folder", 0, Some(SourceReason::Io)),
            "Old jobs, folder, cannot be read: drive not connected"
        );
        assert_eq!(
            document_label("Lid", 0, "", Some("Cannot be read: damaged.")),
            "Lid, document, cannot be read: damaged"
        );
    }

    #[test]
    fn errors_name_their_reason() {
        assert_eq!(
            source_error_text(SourceReason::from_code("not_found")),
            "Cannot be read: folder not found."
        );
        assert_eq!(document_error_text(DocumentState::Readable), None);
        assert_eq!(
            document_error_text(DocumentState::Newer),
            Some("Cannot be read: made by a newer Curvyo.")
        );
    }

    #[test]
    fn the_came_from_line_of_a_selection() {
        let a = record("Box job", "1-1");
        assert_eq!(
            came_from_selection_text(&[Some(a.clone())]),
            Some(("From Box job › Lid".into(), true))
        );
        assert_eq!(
            came_from_selection_text(&[Some(a.clone()), None]),
            Some(("1 of 2 from Box job › Lid".into(), true))
        );
        assert_eq!(
            came_from_selection_text(&[Some(a), Some(record("Logos", "2-2"))]),
            Some(("From 2 documents".into(), false))
        );
        assert_eq!(came_from_selection_text(&[None]), None);
        assert_eq!(texts_released_text(0), None);
        assert_eq!(
            texts_released_text(1).as_deref(),
            Some("1 text left its path: its path was not picked.")
        );
        assert_eq!(
            clones_became_paths_text(2).as_deref(),
            Some("2 linked clones became paths: their originals were not picked.")
        );
    }
}
