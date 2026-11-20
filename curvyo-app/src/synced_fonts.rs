//! The desktop side of the font sync (`specs/0116-library-sync` criteria 11, 16 and 18): the
//! page hands the bytes of fonts that arrived from the maker's other devices, and they are
//! written into the Curvyo font folder under the rules of Add font; a font deleted on another
//! device goes to the operating system's trash. The commands decide nothing about the sync.

use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::fonts::{FaceToRemove, FontFiles, RemovedFonts, fonts_dir};

/// One font that arrived: its name, folder, bytes and licence files.
#[derive(serde::Deserialize)]
pub(crate) struct ArrivedFont {
    file_name: String,
    folder: Option<String>,
    bytes: Vec<u8>,
    documents: Vec<ArrivedDocument>,
}

/// A licence or readme file that arrived with a font.
#[derive(serde::Deserialize)]
pub(crate) struct ArrivedDocument {
    name: String,
    bytes: Vec<u8>,
}

/// What was done with one arrived font: `"written"`, `"present"` or `"refused"`.
fn outcome(write: &curvyo_storage_io::SyncedWrite) -> &'static str {
    match write {
        curvyo_storage_io::SyncedWrite::Written(_) => "written",
        curvyo_storage_io::SyncedWrite::Present => "present",
        curvyo_storage_io::SyncedWrite::NotAFont => "refused",
    }
}

fn write_all(dir: &std::path::Path, fonts: &[ArrivedFont]) -> Vec<&'static str> {
    fonts
        .iter()
        .map(|font| {
            let documents: Vec<(String, Vec<u8>)> = font
                .documents
                .iter()
                .map(|d| (d.name.clone(), d.bytes.clone()))
                .collect();
            outcome(&curvyo_storage_io::write_synced_font(
                dir,
                &curvyo_storage_io::SyncedFont {
                    file_name: &font.file_name,
                    folder: font.folder.as_deref(),
                    bytes: &font.bytes,
                    documents: &documents,
                },
            ))
        })
        .collect()
}

/// Writes fonts that arrived from other devices into the Curvyo folder; one outcome per font, in
/// order. Runs off the main thread.
#[tauri::command]
pub(crate) async fn add_synced_fonts(
    app: AppHandle,
    fonts: Vec<ArrivedFont>,
) -> Result<Vec<&'static str>, String> {
    let dir = fonts_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || write_all(&dir, &fonts))
        .await
        .map_err(|error| error.to_string())
}

/// Moves the files of the listed faces to the operating system's trash (a font deleted on
/// another device, criterion 11), under the rules of Remove: only files in the Curvyo folder
/// whose faces are all given.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn trash_font_files(
    app: AppHandle,
    faces: Vec<FaceToRemove>,
    files: State<'_, FontFiles>,
) -> Result<RemovedFonts, String> {
    let dir = fonts_dir(&app)?;
    let faces: Vec<(PathBuf, u32)> = faces
        .into_iter()
        .filter_map(|face| files.path_of(&face.id).map(|path| (path, face.index)))
        .collect();
    let report = curvyo_storage_io::trash_font_files(&dir, &faces);
    Ok(RemovedFonts::from_report(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bytes that are no font are refused and write nothing; the outcome words are the page's.
    #[test]
    fn a_non_font_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = [ArrivedFont {
            file_name: "junk.ttf".to_owned(),
            folder: None,
            bytes: b"nope".to_vec(),
            documents: Vec::new(),
        }];
        assert_eq!(write_all(dir.path(), &fonts), ["refused"]);
    }
}
