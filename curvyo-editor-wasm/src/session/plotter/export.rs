//! Export job file of a plotter page (`specs/0159-cutting-plotter` criteria 6, 11, 13, 16): the
//! Design Space SVG or HPGL of the page's job under the laser export's name rule, refused while
//! an item lies outside the cut area or nothing runs.

use curvyo_cutter_core::{OutputFormat, write_hpgl, write_svg};

use super::super::Session;

/// A plotter job file for the save dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlotterExport {
    /// The file's text.
    pub text: String,
    /// The page's name, for the file name.
    pub page: String,
    /// The save dialog's file type ("Design Space SVG").
    pub file_type: &'static str,
    /// The extension without its dot.
    pub extension: &'static str,
}

impl Session {
    /// The job file of the active plotter page; `None` on another page, while an item lies
    /// outside the cut area or when nothing runs (criterion 6). Nothing is written to the
    /// project, so an export is no history step (UX C16).
    #[must_use]
    pub fn plotter_export(&self) -> Option<PlotterExport> {
        let page = self.active_plotter()?;
        let built = self.plotter_job(&page, true);
        if !built.outside.is_empty() || built.job.is_empty() {
            return None;
        }
        let values = &page.copy.values;
        let format = values.output_format;
        // A mat job counts from the mat origin over the printed field (ADR 0098 decision 1).
        let mat = page.active_mat().map(|m| m.size);
        let size = mat.map_or(page.page.size, curvyo_cutter_core::MatSize::field_size);
        let text = match format {
            OutputFormat::DesignSpaceSvg => write_svg(&built.job, size, mat),
            OutputFormat::Hpgl => write_hpgl(&built.job, size, values.origin, values.force),
        };
        Some(PlotterExport {
            text,
            page: page.page.name.clone(),
            file_type: format.label(),
            extension: format.extension(),
        })
    }
}
