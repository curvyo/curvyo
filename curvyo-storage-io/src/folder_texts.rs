//! The small text files of one folder, one record each (ADR 0004 §6, ADR 0038 §1): list and read
//! them in name order, or remove one. What a text means is decided by the caller.

use std::fs;
use std::io::{ErrorKind, Read};
use std::path::Path;

use crate::filesystem::FsError;

/// The files of `dir` whose name `accept` takes, in name order, at most `max_files`, each as
/// `(file name, text)`. Names are filtered before the limit, so other files cannot hide records.
/// A missing folder lists nothing. Of each file at most `max_bytes + 1` bytes are read, so the
/// caller can tell an over-long one; a file that is not UTF-8 text or cannot be read gives an
/// empty text, which the caller reads as damage. Nothing is written.
///
/// # Errors
/// [`FsError::Read`] when the folder exists but cannot be listed.
pub fn read_dir_texts(
    dir: &Path,
    accept: impl Fn(&str) -> bool,
    max_files: usize,
    max_bytes: usize,
) -> Result<Vec<(String, String)>, FsError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(FsError::Read {
                path: dir.display().to_string(),
                source,
            });
        }
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| accept(name))
        .collect();
    names.sort();
    names.truncate(max_files);
    Ok(names
        .into_iter()
        .map(|name| {
            let text = read_limited(&dir.join(&name), max_bytes).unwrap_or_default();
            (name, text)
        })
        .collect())
}

fn read_limited(path: &Path, max_bytes: usize) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(u64::try_from(max_bytes).ok()?.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    String::from_utf8(bytes).ok()
}

/// Removes the file at `path`; a file that is already gone is not an error.
///
/// # Errors
/// [`FsError::Write`] when the file exists and cannot be removed.
pub fn remove_file(path: &Path) -> Result<(), FsError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FsError::Write {
            path: path.display().to_string(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(name: &str) -> bool {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    }

    #[test]
    fn texts_are_listed_in_name_order_and_cut_after_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.json"), "bee").unwrap();
        fs::write(dir.path().join("a.json"), "a".repeat(20)).unwrap();
        fs::write(dir.path().join("c.json"), [0xff, 0xfe]).unwrap();
        fs::write(dir.path().join("notes.txt"), "x").unwrap();
        fs::create_dir(dir.path().join("d.json")).unwrap();
        let texts = read_dir_texts(dir.path(), json, 10, 8).unwrap();
        assert_eq!(
            texts,
            vec![
                ("a.json".to_owned(), "a".repeat(9)),
                ("b.json".to_owned(), "bee".to_owned()),
                ("c.json".to_owned(), String::new()),
            ]
        );
        assert_eq!(read_dir_texts(dir.path(), json, 1, 8).unwrap().len(), 1);
        assert_eq!(
            read_dir_texts(&dir.path().join("none"), json, 1, 8).unwrap(),
            vec![]
        );
    }

    #[test]
    fn rejected_names_do_not_count_towards_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        for junk in ["0.txt", "1.txt", "2.txt"] {
            fs::write(dir.path().join(junk), "junk").unwrap();
        }
        fs::write(dir.path().join("9.json"), "record").unwrap();
        let texts = read_dir_texts(dir.path(), json, 1, 8).unwrap();
        assert_eq!(texts, vec![("9.json".to_owned(), "record".to_owned())]);
    }

    #[test]
    fn removing_a_missing_file_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.json");
        fs::write(&path, "x").unwrap();
        remove_file(&path).unwrap();
        assert!(!path.exists());
        remove_file(&path).unwrap();
    }
}
