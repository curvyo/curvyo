//! Export job file of an embroidery page (`specs/0160-embroidery` criteria 4, 14 to 18): the DST
//! of the page's stitch plan under the laser export's name rule, refused while a stitch lies
//! outside the hoop or nothing is sewn.

use curvyo_embroidery_core::write_dst;

use super::super::Session;

/// An embroidery job file for the save dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbroideryExport {
    /// The file's bytes.
    pub bytes: Vec<u8>,
    /// The page's name, for the file name and the `LA:` label.
    pub page: String,
    /// The save dialog's file type ("Tajima DST").
    pub file_type: &'static str,
    /// The extension without its dot.
    pub extension: &'static str,
}

impl Session {
    /// The job file of the active embroidery page; `None` on another page, while a stitch lies
    /// outside the hoop or when nothing is sewn (criterion 4). Nothing is written to the project,
    /// so an export is no history step.
    #[must_use]
    pub fn embroidery_export(&self) -> Option<EmbroideryExport> {
        let page = self.active_embroidery()?;
        let job = self.embroidery_job(&page);
        if job.is_empty() || job.outside.iter().any(|o| !o.is_empty()) {
            return None;
        }
        let format = page.copy.values.output_format;
        let bytes = write_dst(&page.page.name, &job.plan.stitches).ok()?;
        Some(EmbroideryExport {
            bytes,
            page: page.page.name.clone(),
            file_type: format.file_type(),
            extension: format.extension(),
        })
    }
}
