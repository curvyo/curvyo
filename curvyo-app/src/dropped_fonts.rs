//! The desktop's font drop (`specs/0087-image-clipboard-and-drop` criterion 19, `adrs.md` decision
//! 4): the page hands the bytes of files dropped on the Fonts tab, and they are staged as files in
//! a fresh temporary folder so that Add font's own checks and archive handling run on them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::AppHandle;

use crate::fonts::{AddedFonts, add_paths, fonts_dir};

/// One file dropped on the Fonts tab: its name as the page saw it and its bytes.
#[derive(serde::Deserialize)]
pub(crate) struct DroppedFont {
    name: String,
    bytes: Vec<u8>,
}

/// A dropped file's name as a file name: the part after the last `/` or `\`, control characters
/// removed, "font" when nothing usable is left.
fn clean_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let kept: String = base.chars().filter(|c| !c.is_control()).collect();
    let kept = kept.trim();
    if kept.is_empty() || kept == "." || kept == ".." {
        "font".to_owned()
    } else {
        kept.to_owned()
    }
}

/// Writes each of `files` as `root/<n>/<its clean name>` (a folder per file, so equal names do not
/// collide) and returns the paths in order.
fn stage(root: &Path, files: &[DroppedFont]) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::with_capacity(files.len());
    for (n, file) in files.iter().enumerate() {
        let folder = root.join(n.to_string());
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(clean_name(&file.name));
        std::fs::write(&path, &file.bytes)?;
        paths.push(path);
    }
    Ok(paths)
}

/// A temporary folder of this process that no other call uses.
fn staging_root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("curvyo-dropped-fonts-{}-{n}", std::process::id()))
}

/// Add font for files dropped on the Fonts tab (`0075` criteria 21 to 24 through
/// `specs/0087-image-clipboard-and-drop` criterion 19): the bytes are staged, checked and copied
/// into the Curvyo folder as the dialog's files are, then the staging folder is removed. Runs off
/// the main thread.
#[tauri::command]
pub(crate) async fn add_dropped_fonts(
    app: AppHandle,
    files: Vec<DroppedFont>,
) -> Result<AddedFonts, String> {
    let dir = fonts_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let root = staging_root();
        let staged = stage(&root, &files);
        let report = staged.map(|paths| add_paths(&dir, paths));
        let _ = std::fs::remove_dir_all(&root);
        report.map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dropped(name: &str, bytes: &[u8]) -> DroppedFont {
        DroppedFont {
            name: name.to_owned(),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn a_dropped_name_is_only_its_base_name() {
        assert_eq!(clean_name("Lobster.ttf"), "Lobster.ttf");
        assert_eq!(clean_name("../../etc/passwd"), "passwd");
        assert_eq!(clean_name(r"C:\fonts\a.otf"), "a.otf");
        assert_eq!(clean_name("a\u{0}b.ttf"), "ab.ttf");
        assert_eq!(clean_name(".."), "font");
        assert_eq!(clean_name(""), "font");
    }

    #[test]
    fn equal_names_are_staged_apart_with_their_bytes_and_the_staging_goes() {
        let root = staging_root();
        let paths = stage(
            &root,
            &[dropped("a.ttf", b"one"), dropped("dir/a.ttf", b"two")],
        )
        .unwrap();
        assert_eq!(paths.len(), 2);
        assert_ne!(paths[0], paths[1]);
        assert!(
            paths
                .iter()
                .all(|p| p.starts_with(&root) && p.ends_with("a.ttf"))
        );
        assert_eq!(std::fs::read(&paths[1]).unwrap(), b"two");
        std::fs::remove_dir_all(&root).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn staged_files_go_through_add_fonts_checks() {
        let root = staging_root();
        let target = staging_root();
        let paths = stage(&root, &[dropped("notes.ttf", b"not a font")]).unwrap();
        let report = add_paths(&target, paths);
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["notFonts"], serde_json::json!(["notes.ttf"]));
        assert_eq!(json["added"], serde_json::json!([]));
        std::fs::remove_dir_all(&root).unwrap();
        let _ = std::fs::remove_dir_all(&target);
    }
}
