//! The project's saved marker: whether the document differs from what the last Save, Save as, Open
//! or New left (`specs/0051-sidebar-and-no-menu/` criteria 22, 42 to 44, `adrs.md` decision 2).
//!
//! The flag is session state over [`curvyo_document_core::Document::version`], which changes
//! exactly when something is committed. An undo is a forward commit, so undoing back to the saved
//! content still differs: the project stays marked unsaved, the safe direction.

use curvyo_document_core::SaveError;

use super::Session;

/// The application's version, recorded in every saved file and shown on the File tab. One source
/// for the desktop and the browser build: the crate version.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

impl Session {
    /// Packs the document into `.curvyo` bytes for a save and remembers which version went into
    /// them. The caller reports [`Session::mark_saved`] once the bytes are written; a failed write
    /// reports nothing and changes nothing.
    ///
    /// # Errors
    /// See [`curvyo_document_core::pack`].
    pub fn pack_for_save(&mut self) -> Result<Vec<u8>, SaveError> {
        let version = self.document.version();
        let bytes = self.pack(APP_VERSION)?;
        self.packed = Some(version);
        Ok(bytes)
    }

    /// The bytes of the last [`Session::pack_for_save`] are written: the version that went into
    /// them is the saved one. A change made while the write ran stays unsaved.
    pub fn mark_saved(&mut self) {
        if let Some(version) = self.packed.take() {
            self.saved = version;
        }
    }

    /// Whether the document differs from the last save. Read it between events, never inside a
    /// batch of commits.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.document.version() != self.saved
    }
}
