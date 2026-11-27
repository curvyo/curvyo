//! Export job file and Send to machine of a CNC page (`specs/0161-cnc-output` criteria 12 to 14,
//! 19): the page's G-code under the laser export's name rule, refused while a check blocks; the
//! same bytes go to the connector of the page's user profile.

use curvyo_cnc_core::OutputFormat;

use super::super::Session;
use super::super::output::SentJobFile;

/// A CNC job file for the save dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CncExport {
    /// The file's bytes.
    pub bytes: Vec<u8>,
    /// The page's name, for the file name.
    pub page: String,
    /// The save dialog's file type ("GRBL G-code").
    pub file_type: &'static str,
    /// The extension without its dot ("nc").
    pub extension: &'static str,
}

impl Session {
    /// The job file of the active CNC page; `None` on another page or while a check blocks
    /// Export. Nothing is written to the project, so an export is no history step.
    #[must_use]
    pub fn cnc_export(&self) -> Option<CncExport> {
        let page = self.active_cnc()?;
        let job = self.cnc_job(&page);
        if job.blocked() {
            return None;
        }
        let format: OutputFormat = page.copy.values.output_format;
        Some(CncExport {
            bytes: job.gcode.into_bytes(),
            page: page.page.name.clone(),
            file_type: format.file_type(),
            extension: format.extension(),
        })
    }

    /// The profile id and connector address of the active CNC page's user profile, `None`
    /// without an address (a built-in has none).
    pub(in crate::session) fn cnc_connector(&self) -> Option<(String, String)> {
        let record = self.cnc_record()?;
        let address = record.values.connection.connector_address.trim().to_owned();
        (!address.is_empty()).then_some((record.id, address))
    }

    /// Send to machine on a CNC page (criterion 19): the bytes Export would write, for the
    /// connector of the page's profile; `None` without one or while a check blocks. Nothing is
    /// written to the project.
    pub fn cnc_send_job(&mut self) -> Option<SentJobFile> {
        let (profile_id, address) = self.cnc_connector()?;
        let export = self.cnc_export()?;
        let state = &mut self.manufacturing.cnc;
        let token = state.next_send;
        state.next_send = state.next_send.wrapping_add(1);
        Some(SentJobFile {
            bytes: export.bytes,
            name: export.page,
            profile_id,
            address,
            token,
        })
    }
}
