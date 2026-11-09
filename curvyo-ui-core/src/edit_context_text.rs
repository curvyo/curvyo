//! The words of a move-only selection (`specs/0106-move-versus-edit` criteria 9, 11 and 14, UX
//! notes §3 and §6): the Style tab's muted line and the refusal notice, at most 70 characters,
//! with each page name cut at 24.

use crate::share_text::cut_label;

/// How long a page name may be inside a line.
const NAME_CHARS: usize = 24;

/// The notice of a double-click on a pasteboard object in a project without a drawing page
/// (criterion 14).
pub const NO_DRAWING_PAGE_NOTICE: &str = "There is no drawing page to edit this on.";

/// Where the out-of-context objects of a selection lie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideHomes {
    /// The selection holds in-context objects too.
    pub mixed: bool,
    /// The distinct homes of the out-of-context objects, by name ("Drawing 2", "Pasteboard"), in
    /// the order first met.
    pub names: Vec<String>,
}

impl OutsideHomes {
    /// "Drawing 2" (cut at 24) for one home, "3 pages" for several.
    fn place(&self) -> String {
        match self.names.as_slice() {
            [one] => cut_label(one, NAME_CHARS),
            names => format!("{} pages", names.len()),
        }
    }
}

/// The Style tab's line: "On Drawing 2. Double-click to edit.", "Includes objects on 3 pages.
/// Double-click to edit." (UX notes §3).
#[must_use]
pub fn out_of_context_line(homes: &OutsideHomes) -> String {
    let lead = if homes.mixed {
        "Includes objects on"
    } else {
        "On"
    };
    format!("{lead} {}. Double-click to edit.", homes.place())
}

/// The refusal of an edit chord: "Not editable here: on Drawing 2.", "Not editable here:
/// includes objects on 3 pages." (criterion 9, UX notes §6).
#[must_use]
pub fn not_editable_notice(homes: &OutsideHomes) -> String {
    let lead = if homes.mixed {
        "includes objects on"
    } else {
        "on"
    };
    format!("Not editable here: {lead} {}.", homes.place())
}

/// Where the out-of-context objects lie, as the announcer reads it: "Drawing 2", "pasteboard",
/// "3 pages".
fn place_in_sentence(homes: &OutsideHomes) -> String {
    match homes.names.as_slice() {
        [one] if one == "Pasteboard" => "pasteboard".to_owned(),
        _ => homes.place(),
    }
}

/// Every home by name, as the status bar lists them: "Page 2, Laser 1", "pasteboard".
fn every_home(homes: &OutsideHomes) -> String {
    homes
        .names
        .iter()
        .map(|name| {
            if name == "Pasteboard" {
                "pasteboard".to_owned()
            } else {
                cut_label(name, NAME_CHARS)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The status bar's left text for a move-only selection: "Rectangle · Drawing 2", "Circle ·
/// pasteboard", "2 objects · Page 2, Laser 1" (UX notes §2): every home is named.
#[must_use]
pub fn out_of_context_status(subject: &str, homes: &OutsideHomes) -> String {
    format!("{subject} · {}", every_home(homes))
}

/// What the selection announcer adds for a move-only selection: ", on Drawing 2", ", on the
/// pasteboard", ", on 3 pages" (UX notes §2).
#[must_use]
pub fn out_of_context_suffix(homes: &OutsideHomes) -> String {
    match place_in_sentence(homes).as_str() {
        "pasteboard" => ", on the pasteboard".to_owned(),
        place => format!(", on {place}"),
    }
}

/// The muted tooltip line of a Layers row whose object lies on another page: "On Laser 1"
/// (UX notes §5), the name cut at 24.
#[must_use]
pub fn home_row_line(page_name: &str) -> String {
    format!("On {}", cut_label(page_name, NAME_CHARS))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn homes(mixed: bool, names: &[&str]) -> OutsideHomes {
        OutsideHomes {
            mixed,
            names: names.iter().map(|n| (*n).to_owned()).collect(),
        }
    }

    #[test]
    fn the_line_reads_as_the_ux_table() {
        assert_eq!(
            out_of_context_line(&homes(false, &["Drawing 2"])),
            "On Drawing 2. Double-click to edit."
        );
        assert_eq!(
            out_of_context_line(&homes(false, &["A", "B", "C"])),
            "On 3 pages. Double-click to edit."
        );
        assert_eq!(
            out_of_context_line(&homes(true, &["Drawing 2"])),
            "Includes objects on Drawing 2. Double-click to edit."
        );
        assert_eq!(
            out_of_context_line(&homes(false, &["Pasteboard"])),
            "On Pasteboard. Double-click to edit."
        );
    }

    #[test]
    fn the_notice_reads_as_the_ux_table() {
        assert_eq!(
            not_editable_notice(&homes(false, &["Drawing 2"])),
            "Not editable here: on Drawing 2."
        );
        assert_eq!(
            not_editable_notice(&homes(true, &["A", "B", "C"])),
            "Not editable here: includes objects on 3 pages."
        );
    }

    #[test]
    fn the_status_bar_the_announcer_and_the_row_line_name_the_home() {
        assert_eq!(
            out_of_context_status("Rectangle", &homes(false, &["Drawing 2"])),
            "Rectangle · Drawing 2"
        );
        assert_eq!(
            out_of_context_status("Circle", &homes(false, &["Pasteboard"])),
            "Circle · pasteboard"
        );
        assert_eq!(
            out_of_context_status("2 objects", &homes(true, &["Page 2", "Laser 1"])),
            "2 objects · Page 2, Laser 1",
            "the status bar names every home"
        );
        assert_eq!(
            out_of_context_suffix(&homes(false, &["Drawing 2"])),
            ", on Drawing 2"
        );
        assert_eq!(
            out_of_context_suffix(&homes(false, &["Pasteboard"])),
            ", on the pasteboard"
        );
        assert_eq!(home_row_line("Laser 1"), "On Laser 1");
    }

    #[test]
    fn a_long_name_is_cut_at_24_and_every_line_fits_70() {
        let long = "A very long drawing page name indeed";
        let line = out_of_context_line(&homes(true, &[long]));
        assert!(line.contains("A very long drawing pag…"));
        assert!(line.chars().count() <= 70, "{line}");
        assert!(not_editable_notice(&homes(true, &[long])).chars().count() <= 70);
        assert!(NO_DRAWING_PAGE_NOTICE.chars().count() <= 70);
    }
}
