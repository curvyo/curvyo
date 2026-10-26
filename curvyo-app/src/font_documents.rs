//! The commands that show a font folder's documents in the info view
//! (`specs/0083-font-info-and-zip` criterion 4, `adrs.md` decision 7): the first 1 MiB of a text
//! document as plain text, and a `.pdf` handed to the system's viewer. The page names a document
//! only by its folder and file name; `curvyo-storage-io` refuses anything that is not a document
//! directly in a subfolder of the Curvyo folder.

use tauri::AppHandle;

use crate::fonts::fonts_dir;

/// A text document's first 1 MiB, decoded as UTF-8 with replacement, and whether it was cut.
#[derive(Clone, serde::Serialize)]
pub(crate) struct DocumentText {
    text: String,
    cut: bool,
}

/// The text of document `name` of the font folder `folder`.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_font_document(
    app: AppHandle,
    folder: String,
    name: String,
) -> Result<DocumentText, String> {
    let dir = fonts_dir(&app)?;
    let (bytes, cut) = curvyo_storage_io::read_font_document(&dir, &folder, &name)
        .ok_or_else(|| "this document could not be read".to_string())?;
    Ok(DocumentText {
        text: String::from_utf8_lossy(&bytes).into_owned(),
        cut,
    })
}

/// Opens the `.pdf` document `name` of the font folder `folder` in the system's viewer.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn open_font_document(
    app: AppHandle,
    folder: String,
    name: String,
) -> Result<(), String> {
    let dir = fonts_dir(&app)?;
    let path = curvyo_storage_io::font_document_pdf(&dir, &folder, &name)
        .ok_or_else(|| "this document could not be opened".to_string())?;
    open::that_detached(path).map_err(|error| error.to_string())
}
