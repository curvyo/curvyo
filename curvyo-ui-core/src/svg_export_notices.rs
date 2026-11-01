//! The words around Export SVG… (`specs/0024-svg-import-export` criteria 4, 131 and 148,
//! `ux-notes.md` "SVG export: UX notes"): the default file name, the busy notice and the
//! notices after an export. The sentences live here so both builds say the same.

use std::fmt::Write as _;

use crate::svg_import_text::notice_file_name;

/// From this many objects the export shows the busy state (`ux-notes.md`, "Busy state").
pub const SVG_EXPORT_BUSY_OBJECTS: usize = 5000;

/// The notice when Selection only is on and nothing is selected (criterion 131).
pub const NOTHING_SELECTED_NOTICE: &str = "Nothing is selected.";

/// The name the save dialog (or the download) offers (criterion 4): the source's name when the
/// project was opened from an SVG, else the project's file name with `.svg`, else
/// "Untitled.svg".
#[must_use]
pub fn svg_export_name(source_svg: Option<&str>, project_file: Option<&str>) -> String {
    if let Some(source) = source_svg.filter(|s| !s.is_empty()) {
        return source.to_owned();
    }
    match project_file.filter(|s| !s.is_empty()) {
        Some(name) => {
            let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
            format!("{stem}.svg")
        }
        None => "Untitled.svg".to_owned(),
    }
}

/// "Exporting poster.svg..." (`ux-notes.md`, "Busy state").
#[must_use]
pub fn svg_export_busy_notice(file_name: &str) -> String {
    format!("Exporting {}...", notice_file_name(file_name))
}

/// The notice after a written export (criteria 4 and 148): "Exported poster.svg." (browser:
/// "Downloaded poster.svg."), with "The drawing is empty." when nothing was in scope and the
/// count of texts kept as text when their fonts are missing.
#[must_use]
pub fn svg_export_notice(file_name: &str, downloaded: bool, objects: usize, kept: usize) -> String {
    let verb = if downloaded { "Downloaded" } else { "Exported" };
    let mut notice = format!("{verb} {}.", notice_file_name(file_name));
    if objects == 0 {
        notice.push_str(" The drawing is empty.");
    } else if kept == 1 {
        notice.push_str(" 1 text kept as text because its font is missing.");
    } else if kept > 1 {
        let _ = write!(
            notice,
            " {kept} texts kept as text because their fonts are missing."
        );
    }
    notice
}

/// How long the notice stays: 6 s with the count of texts, else 3 s (criterion 148).
#[must_use]
pub const fn svg_export_notice_ms(kept: usize) -> u32 {
    if kept > 0 { 6000 } else { 3000 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac4_the_default_name() {
        assert_eq!(svg_export_name(Some("poster.svg"), None), "poster.svg");
        assert_eq!(svg_export_name(None, Some("poster.curvyo")), "poster.svg");
        assert_eq!(svg_export_name(None, None), "Untitled.svg");
    }

    #[test]
    fn ac4_ac148_the_notices() {
        assert_eq!(
            svg_export_notice("poster.svg", false, 3, 0),
            "Exported poster.svg."
        );
        assert_eq!(
            svg_export_notice("poster.svg", true, 3, 0),
            "Downloaded poster.svg."
        );
        assert_eq!(
            svg_export_notice("Untitled.svg", false, 0, 0),
            "Exported Untitled.svg. The drawing is empty."
        );
        assert_eq!(
            svg_export_notice("poster.svg", false, 5, 2),
            "Exported poster.svg. 2 texts kept as text because their fonts are missing."
        );
        assert_eq!(
            svg_export_notice("poster.svg", false, 5, 1),
            "Exported poster.svg. 1 text kept as text because its font is missing."
        );
        assert_eq!(svg_export_notice_ms(1), 6000);
        assert_eq!(
            svg_export_busy_notice("poster.svg"),
            "Exporting poster.svg..."
        );
    }
}
