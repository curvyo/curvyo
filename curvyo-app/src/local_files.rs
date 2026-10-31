//! Reads the local files the operating system's clipboard or the drag over the window names
//! (`specs/0087-image-clipboard-and-drop` criteria 6 and 21, lead decision 2026-10-29) for the
//! page's import: regular files only, symlinks resolved, the import's file limit checked on the
//! open file before a byte is read. The page never names a file: it says only whether it pastes
//! or drops, and the host reads the list itself. The page's import decides by the content what
//! each file is.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use curvyo_image_core::MAX_FILE_BYTES;
use tauri::AppHandle;

use crate::file_uris::local_paths;

/// The most files one import takes (`specs/0058-bitmap-import` criterion 5); the rest are named
/// but not read, so the import's skipped notice counts them.
const MAX_READ_FILES: usize = 20;

/// Where the file list comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Source {
    /// The clipboard, for Ctrl+V.
    Paste,
    /// The drag that was just dropped on the window.
    Drop,
}

/// One named file, as the page receives it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind")]
pub(crate) enum LocalFile {
    /// The file's bytes.
    #[serde(rename = "bytes")]
    Bytes { name: String, bytes: Vec<u8> },
    /// Over the import's file limit: its length, never read.
    #[serde(rename = "too_large")]
    TooLarge { name: String, bytes: u64 },
    /// Missing, unreadable, a folder or not a regular file.
    #[serde(rename = "refused")]
    Refused { name: String },
    /// Past the import's file count: not read.
    #[serde(rename = "skipped")]
    Skipped { name: String },
}

/// The local files the clipboard (`from` paste) or the dropped drag (`from` drop) names as
/// `file://` URIs or absolute paths, read in their order; empty when it names none (the page then
/// shows its link or nothing-to-paste notice). Remote links are never fetched.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) async fn read_local_files(app: AppHandle, from: Source) -> Vec<LocalFile> {
    tauri::async_runtime::spawn_blocking(move || {
        read_named(&named_paths(&listed_texts(&app, from)))
    })
    .await
    .unwrap_or_default()
}

/// Linux: the file lists `WebKitGTK` hides from the page, read on the GTK side.
#[cfg(target_os = "linux")]
fn listed_texts(app: &AppHandle, from: Source) -> Vec<String> {
    crate::native_file_lists::texts(app, from)
}

/// Windows and macOS: their webviews hand the page real files for a file manager's copy or drop;
/// file names given only as text are not read there.
#[cfg(not(target_os = "linux"))]
fn listed_texts(_app: &AppHandle, _from: Source) -> Vec<String> {
    Vec::new()
}

/// The paths the first of `texts` that names any local file names.
fn named_paths(texts: &[String]) -> Vec<PathBuf> {
    let windows = cfg!(windows);
    texts
        .iter()
        .map(|text| local_paths(text, windows))
        .find(|paths| !paths.is_empty())
        .unwrap_or_default()
}

/// Each of `paths` read, past the import's file count named only.
fn read_named(paths: &[PathBuf]) -> Vec<LocalFile> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            if index < MAX_READ_FILES {
                read_local_file(path, MAX_FILE_BYTES, read_bounded)
            } else {
                LocalFile::Skipped {
                    name: file_name(path),
                }
            }
        })
        .collect()
}

/// `path` read with `read` when it resolves (symlinks followed) to a regular file of at most
/// `ceiling` bytes; its length alone when it is larger; refused otherwise. The checks are made on
/// the opened file, so a path swapped after the first look is judged by what was opened; the
/// first look keeps a pipe or device from being opened at all. The name is the named path's
/// file name.
fn read_local_file(
    path: &Path,
    ceiling: u64,
    read: impl FnOnce(File, u64) -> io::Result<Vec<u8>>,
) -> LocalFile {
    let name = file_name(path);
    let Ok(target) = path.canonicalize() else {
        return LocalFile::Refused { name };
    };
    if !target.metadata().is_ok_and(|meta| meta.is_file()) {
        return LocalFile::Refused { name };
    }
    let Ok(file) = File::open(&target) else {
        return LocalFile::Refused { name };
    };
    let Ok(meta) = file.metadata() else {
        return LocalFile::Refused { name };
    };
    if !meta.is_file() {
        return LocalFile::Refused { name };
    }
    if meta.len() > ceiling {
        return LocalFile::TooLarge {
            name,
            bytes: meta.len(),
        };
    }
    match read(file, ceiling) {
        Ok(bytes) => LocalFile::Bytes { name, bytes },
        Err(_) => LocalFile::Refused { name },
    }
}

/// The bytes of `file`, at most one byte over `ceiling`, so a file that grew after its length
/// was read is still refused by the import's own check and never read whole.
fn read_bounded(file: File, ceiling: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(ceiling + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The last part of `path`, as text.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The page can only say paste or drop: the command has no parameter that names a file, and
    /// a path given as the source is refused before the command runs.
    #[test]
    fn the_page_cannot_name_a_file_to_read() {
        let source = include_str!("local_files.rs");
        let start = source
            .find("pub(crate) async fn read_local_files(")
            .expect("the command");
        let signature: String = source[start..]
            .split('{')
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            signature,
            "pub(crate) async fn read_local_files(app: AppHandle, from: Source) -> Vec<LocalFile>"
        );
        let source_of = |json: &str| serde_json::from_str::<Source>(json).ok();
        assert_eq!(source_of(r#""paste""#), Some(Source::Paste));
        assert_eq!(source_of(r#""drop""#), Some(Source::Drop));
        for path in [
            r#""/home/me/.ssh/id_rsa""#,
            r#""file:///etc/passwd""#,
            r#"["/etc/passwd"]"#,
        ] {
            assert_eq!(source_of(path), None, "{path}");
        }
    }

    /// What the command reads for `texts` when they name files.
    fn read_texts(texts: &[String]) -> Vec<LocalFile> {
        read_named(&named_paths(texts))
    }

    /// A fresh folder for one test.
    fn folder(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("curvyo-local-files-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp folder");
        dir
    }

    /// The `file://` URI of `path`: `file:///C:/dir/file` on Windows, `file:///dir/file` elsewhere.
    fn uri(path: &Path) -> String {
        let text = path
            .to_string_lossy()
            .replace(' ', "%20")
            .replace('\\', "/");
        if text.starts_with('/') {
            format!("file://{text}")
        } else {
            format!("file:///{text}")
        }
    }

    #[test]
    fn a_regular_file_is_read_with_its_name() {
        let dir = folder("regular");
        let file = dir.join("my photo.png");
        std::fs::write(&file, b"\x89PNG bytes").expect("write");
        let read = read_texts(&[String::new(), format!("copy\n{}", uri(&file))]);
        assert_eq!(
            read,
            [LocalFile::Bytes {
                name: "my photo.png".into(),
                bytes: b"\x89PNG bytes".to_vec()
            }]
        );
    }

    #[test]
    fn a_folder_a_missing_file_and_a_device_are_refused() {
        let dir = folder("refused");
        let missing = dir.join("gone.png");
        // `/dev/null` is a device on Unix; a Windows path has a drive, so it names no file there.
        let device = if cfg!(unix) {
            "file:///dev/null\r\n"
        } else {
            ""
        };
        let read = read_texts(&[format!("{}\r\n{}\r\n{device}", uri(&dir), uri(&missing))]);
        let names: Vec<_> = read
            .iter()
            .map(|file| match file {
                LocalFile::Refused { name } => name.as_str(),
                other => panic!("refused, got {other:?}"),
            })
            .collect();
        assert_eq!(names.len(), if cfg!(unix) { 3 } else { 2 });
        assert_eq!(names[1], "gone.png");
        if cfg!(unix) {
            assert_eq!(names[2], "null");
        }
    }

    #[test]
    fn over_the_limit_is_too_large_and_never_read() {
        let dir = folder("large");
        let file = dir.join("huge.png");
        // A sparse file: its length is over the limit, no disk is used.
        File::create(&file)
            .and_then(|f| f.set_len(MAX_FILE_BYTES + 1))
            .expect("sparse file");
        let read = read_local_file(&file, MAX_FILE_BYTES, |_: File, _| {
            panic!("a file over the limit is never read")
        });
        assert_eq!(
            read,
            LocalFile::TooLarge {
                name: "huge.png".into(),
                bytes: MAX_FILE_BYTES + 1
            }
        );
        let at_limit = read_local_file(&file, MAX_FILE_BYTES + 1, |_: File, ceiling| {
            Ok(vec![0; usize::try_from(ceiling).unwrap_or(0).min(4)])
        });
        assert!(matches!(at_limit, LocalFile::Bytes { .. }), "{at_limit:?}");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_resolved_to_their_target() {
        let dir = folder("links");
        let file = dir.join("real.png");
        std::fs::write(&file, b"pixels").expect("write");
        let to_file = dir.join("link.png");
        let to_dir = dir.join("link-dir");
        std::os::unix::fs::symlink(&file, &to_file).expect("symlink");
        std::os::unix::fs::symlink(&dir, &to_dir).expect("symlink");
        assert_eq!(
            read_local_file(&to_file, 100, read_bounded),
            LocalFile::Bytes {
                name: "link.png".into(),
                bytes: b"pixels".to_vec()
            }
        );
        assert_eq!(
            read_local_file(&to_dir, 100, read_bounded),
            LocalFile::Refused {
                name: "link-dir".into()
            }
        );
    }

    #[test]
    fn remote_links_and_prose_read_nothing() {
        let read = read_texts(&[
            "https://example.com/a.png".into(),
            "some words".into(),
            "ftp://example.com/b.png\nfile://server/c.png".into(),
        ]);
        assert_eq!(read, []);
    }

    #[test]
    fn past_twenty_files_the_rest_is_named_not_read() {
        let dir = folder("many");
        let list: Vec<String> = (0..22)
            .map(|i| {
                let file = dir.join(format!("{i}.png"));
                std::fs::write(&file, b"x").expect("write");
                uri(&file)
            })
            .collect();
        let read = read_texts(&[list.join("\n")]);
        assert_eq!(read.len(), 22);
        assert!(
            read[..20]
                .iter()
                .all(|file| matches!(file, LocalFile::Bytes { .. }))
        );
        assert_eq!(
            read[20..],
            [
                LocalFile::Skipped {
                    name: "20.png".into()
                },
                LocalFile::Skipped {
                    name: "21.png".into()
                }
            ]
        );
    }
}
