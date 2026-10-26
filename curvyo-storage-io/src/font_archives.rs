//! Adds a zip archive's fonts and documents to the Curvyo font folder as one font folder, whole
//! or not at all (`specs/0083-font-info-and-zip` criteria 9, 11, 12, 15 to 17; `adrs.md`
//! decision 5). The archive is read by `curvyo-text-core`; each font is checked as a loose file
//! is; the folder is built under a `.curvyo-add-*` name and renamed when complete.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use curvyo_text_core::limits::MAX_ARCHIVE_BYTES;
use curvyo_text_core::{
    ArchiveError, EntryClass, EntrySkip, FontArchive, archive_folder_name, free_folder_name,
    listable_faces,
};

use crate::font_files::{every_font_file, is_listable, same_bytes_in};
use crate::font_subfolders::TEMP_PREFIX;

/// Why an archive was refused as a whole (the notice's reasons, criterion 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRefusal {
    /// Not a zip archive, damaged, unreadable or not written.
    Unreadable,
    /// Over the file, entry, font or total size limit.
    OverLimit,
}

/// What adding one archive did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedArchive {
    /// The archive's file name.
    pub name: String,
    /// The folder created, `None` when nothing was added.
    pub folder: Option<String>,
    /// The font files added (their names in the folder).
    pub added: Vec<String>,
    /// Font entries already in the Curvyo folder, or the same face twice in the archive.
    pub present: usize,
    /// Documents kept.
    pub documents: usize,
    /// Entries skipped as unsafe.
    pub unsafe_entries: usize,
    /// Set when the archive was refused whole.
    pub refused: Option<ArchiveRefusal>,
}

/// Whether `path` names a `.zip` (any case).
#[must_use]
pub fn is_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

/// Tries a rename this many times when Windows reports the folder in use (a sync client).
const RENAME_TRIES: u32 = 3;
const RENAME_PAUSE: std::time::Duration = std::time::Duration::from_millis(100);

/// The archive's bytes, read through `take` so a file over 256 MiB is never read whole.
fn read_archive(path: &Path) -> Result<Vec<u8>, ArchiveRefusal> {
    let file = std::fs::File::open(path).map_err(|_| ArchiveRefusal::Unreadable)?;
    let mut bytes = Vec::new();
    file.take(MAX_ARCHIVE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ArchiveRefusal::Unreadable)?;
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(ArchiveRefusal::OverLimit);
    }
    Ok(bytes)
}

/// The files to write: (file name, bytes), fonts first.
#[derive(Default)]
struct Kept {
    fonts: Vec<(String, Vec<u8>)>,
    documents: Vec<(String, Vec<u8>)>,
    present: usize,
    unsafe_entries: usize,
}

/// Reads and checks the entries of `archive`: fonts in the order TTF, OTF, TTC, SVG (criterion
/// 11), each against the Curvyo folder and the faces already taken, then the documents.
fn kept_entries(archive: &mut FontArchive, existing: &[PathBuf]) -> Result<Kept, ArchiveRefusal> {
    let mut kept = Kept::default();
    let mut fonts: Vec<(curvyo_text_core::FontFileKind, usize)> = Vec::new();
    let mut documents = Vec::new();
    for (index, entry) in archive.entries().iter().enumerate() {
        match entry.class {
            EntryClass::Font(kind) => fonts.push((kind, index)),
            EntryClass::Document => documents.push(index),
            EntryClass::Unsafe(_) => kept.unsafe_entries += 1,
            EntryClass::LeftOut => {}
        }
    }
    fonts.sort_by_key(|(kind, index)| (*kind, *index));
    let mut faces_taken: BTreeSet<(String, String)> = BTreeSet::new();
    let read = |archive: &mut FontArchive, index: usize, kept: &mut Kept| match archive.read(index)
    {
        Ok(bytes) => Ok(Some(bytes)),
        Err(EntrySkip::ArchiveOverLimit) => Err(ArchiveRefusal::OverLimit),
        Err(_) => {
            kept.unsafe_entries += 1;
            Ok(None)
        }
    };
    for (_, index) in fonts {
        let Some(bytes) = read(archive, index, &mut kept)? else {
            continue;
        };
        if !is_listable(&bytes) {
            continue;
        }
        let faces: BTreeSet<(String, String)> = listable_faces(&bytes)
            .into_iter()
            .map(|face| (face.family, face.style))
            .collect();
        let twice = !faces.is_empty() && faces.is_subset(&faces_taken);
        if twice || same_bytes_in(existing, &bytes) {
            kept.present += 1;
            continue;
        }
        faces_taken.extend(faces);
        kept.fonts
            .push((archive.entries()[index].file_name.clone(), bytes));
    }
    if kept.fonts.is_empty() {
        return Ok(kept);
    }
    for index in documents {
        if let Some(bytes) = read(archive, index, &mut kept)? {
            kept.documents
                .push((archive.entries()[index].file_name.clone(), bytes));
        }
    }
    Ok(kept)
}

/// A new, empty `.curvyo-add-<n>` folder in `dir`.
fn temporary_folder(dir: &Path) -> std::io::Result<PathBuf> {
    let pid = std::process::id();
    for n in 0_u32..1000 {
        let path = dir.join(format!("{TEMP_PREFIX}{pid}-{n}"));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::other("no free temporary folder name"))
}

fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::File::create_new(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// The names directly in `dir`.
fn names_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Renames the complete `temp` folder to a free name for `stem` in `dir`: a name taken
/// meanwhile gets the next `-N`; a Windows sharing error is tried again 3 times.
fn publish(dir: &Path, temp: &Path, stem: &str) -> std::io::Result<String> {
    let mut taken = names_in(dir);
    let mut tries = 0;
    loop {
        let name = free_folder_name(stem, &taken);
        match std::fs::rename(temp, dir.join(&name)) {
            Ok(()) => return Ok(name),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::DirectoryNotEmpty
                ) =>
            {
                taken.push(name);
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied && tries < RENAME_TRIES =>
            {
                tries += 1;
                std::thread::sleep(RENAME_PAUSE);
            }
            Err(error) => return Err(error),
        }
    }
}

/// Writes `kept` into a temporary folder and renames it; the temporary folder is removed on
/// any failure.
fn write_folder(dir: &Path, stem: &str, kept: &Kept) -> std::io::Result<String> {
    let temp = temporary_folder(dir)?;
    let written = kept
        .fonts
        .iter()
        .chain(&kept.documents)
        .try_for_each(|(name, bytes)| write_synced(&temp.join(name), bytes))
        .and_then(|()| publish(dir, &temp, stem));
    if written.is_err() {
        let _ = std::fs::remove_dir_all(&temp);
    }
    written
}

/// Adds the archive at `path` to the Curvyo folder `dir` (created when missing): its new fonts
/// and its documents go into one new font folder named after the archive, or nothing is written.
/// The archive is only read.
#[must_use]
pub fn add_font_archive(dir: &Path, path: &Path) -> AddedArchive {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut report = AddedArchive {
        name,
        folder: None,
        added: Vec::new(),
        present: 0,
        documents: 0,
        unsafe_entries: 0,
        refused: None,
    };
    let _ = std::fs::create_dir_all(dir);
    let opened = read_archive(path).and_then(|bytes| {
        FontArchive::open(bytes).map_err(|error| match error {
            ArchiveError::OverLimit => ArchiveRefusal::OverLimit,
            ArchiveError::NotZip | ArchiveError::Damaged => ArchiveRefusal::Unreadable,
        })
    });
    let kept = opened.and_then(|mut archive| kept_entries(&mut archive, &every_font_file(dir)));
    let kept = match kept {
        Ok(kept) => kept,
        Err(refusal) => {
            report.refused = Some(refusal);
            return report;
        }
    };
    report.present = kept.present;
    report.unsafe_entries = kept.unsafe_entries;
    if kept.fonts.is_empty() {
        return report;
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    match write_folder(dir, &archive_folder_name(&stem), &kept) {
        Ok(folder) => {
            report.folder = Some(folder);
            report.added = kept.fonts.iter().map(|(name, _)| name.clone()).collect();
            report.documents = kept.documents.len();
        }
        Err(_) => report.refused = Some(ArchiveRefusal::Unreadable),
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-text-core/tests/fixtures/fonts/zip")
            .join(name)
    }

    fn included(path: &str) -> Vec<u8> {
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../frontend/public/fonts")
                .join(path),
        )
        .unwrap()
    }

    fn zip_of(path: &Path, entries: &[(&str, Vec<u8>)]) {
        let mut out = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, data) in entries {
            out.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            out.write_all(data).unwrap();
        }
        out.finish().unwrap();
    }

    fn sorted_names(dir: &Path) -> Vec<String> {
        let mut names = names_in(dir);
        names.sort();
        names
    }

    /// Criteria 9 and 12: the fonts and documents of an archive land flat in one folder named
    /// after it; a taken name gets `-2`; no temporary folder is left; the archive stays.
    #[test]
    fn ac09_ac12_one_folder_per_archive() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        let archive = fixture("nested-folders.zip");
        let first = add_font_archive(&fonts, &archive);
        assert_eq!(first.folder.as_deref(), Some("nested-folders"));
        assert_eq!((first.added.len(), first.documents), (1, 1));
        assert_eq!(
            sorted_names(&fonts.join("nested-folders")),
            ["HersheySans1.svg", "NOTICE.txt"]
        );
        assert_eq!(sorted_names(&fonts), ["nested-folders"]);
        assert!(archive.is_file());
        // The same fonts again: already there, nothing written.
        let again = add_font_archive(&fonts, &fixture("plain.zip"));
        assert_eq!((again.present, again.folder.clone()), (1, None));
        std::fs::create_dir(fonts.join("Pack")).unwrap();
        let pack = dir.path().join("pack.zip");
        zip_of(
            &pack,
            &[(
                "GreatVibes.ttf",
                included("great-vibes/GreatVibes-Regular.ttf"),
            )],
        );
        assert_eq!(
            add_font_archive(&fonts, &pack).folder.as_deref(),
            Some("pack-2")
        );
    }

    /// Criterion 11: the same face as `.otf` and `.ttf` is added once (the TTF); an archive that
    /// adds no font keeps no documents.
    #[test]
    fn ac11_same_face_twice_and_no_new_font() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        let vibes = included("great-vibes/GreatVibes-Regular.ttf");
        let mut other = vibes.clone();
        // Other bytes, the same names: a changed byte at the end of the file.
        let last = other.len() - 1;
        other[last] ^= 0x01;
        let pack = dir.path().join("Vibes.zip");
        zip_of(
            &pack,
            &[
                ("a/Vibes.otf", other),
                ("b/Vibes.ttf", vibes),
                ("OFL.txt", b"OFL".to_vec()),
            ],
        );
        let report = add_font_archive(&fonts, &pack);
        assert_eq!(report.added, ["Vibes.ttf"]);
        assert_eq!((report.present, report.documents), (1, 1));
        let empty = add_font_archive(&fonts, &fixture("no-font.zip"));
        assert_eq!(
            (empty.folder, empty.documents, empty.refused),
            (None, 0, None)
        );
        assert_eq!(sorted_names(&fonts), ["Vibes"]);
    }

    /// Criteria 13, 15 and 16: refusals and unsafe counts; nothing is written for them.
    #[test]
    fn ac15_ac16_refused_archives_write_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        let cases = [
            ("not-a-zip.zip", Some(ArchiveRefusal::Unreadable)),
            ("overlap-bomb.zip", Some(ArchiveRefusal::Unreadable)),
            ("many-entries.zip", Some(ArchiveRefusal::OverLimit)),
            ("total-bomb.zip", Some(ArchiveRefusal::OverLimit)),
        ];
        for (name, refusal) in cases {
            assert_eq!(
                add_font_archive(&fonts, &fixture(name)).refused,
                refusal,
                "{name}"
            );
        }
        assert_eq!(names_in(&fonts), Vec::<String>::new());
        let dotdot = add_font_archive(&fonts, &fixture("dotdot.zip"));
        assert_eq!((dotdot.unsafe_entries, dotdot.added.len()), (2, 1));
        assert_eq!(sorted_names(&fonts), ["dotdot"]);
    }

    /// Criterion 12: a failed write leaves no folder and no temporary folder.
    #[cfg(unix)]
    #[test]
    fn ac12_failed_write_leaves_nothing() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        std::fs::create_dir(&fonts).unwrap();
        std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o500)).unwrap();
        let report = add_font_archive(&fonts, &fixture("plain.zip"));
        std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(report.refused, Some(ArchiveRefusal::Unreadable));
        assert_eq!(names_in(&fonts), Vec::<String>::new());
    }
}
