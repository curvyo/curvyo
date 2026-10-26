//! Independent tester cases for `specs/0075-font-library/specification.md` on the desktop's
//! folder I/O: criterion 16 (2,000 listed faces with their class bytes inside `0074`'s 2 s, a
//! release-mode benchmark), criterion 27 (copying into and deleting from the Curvyo folder with
//! the file manager still works next to Add font and Remove), and the edges of Add and Remove
//! (criteria 21, 25, 32: path safety, the folder only).
//!
//! Written from the specification and the public names before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::assert_is_empty,
    missing_docs
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use curvyo_storage_io::{FontSource, add_font_files, list_font_folder, remove_font_files};
use curvyo_text_core::{DeclaredClass, declared_class};

fn included(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts")
        .join(rel)
}

const SANS: &str = "source-sans-3/SourceSans3-Regular.otf";
const SERIF: &str = "source-serif-4/SourceSerif4-Regular.ttf";
const SCRIPT: &str = "great-vibes/GreatVibes-Regular.ttf";

fn families(dir: &Path) -> Vec<String> {
    let mut f: Vec<String> = list_font_folder(dir)
        .faces
        .into_iter()
        .map(|face| face.family)
        .collect();
    f.sort();
    f.dedup();
    f
}

/// Criterion 27: a file copied in with the file manager is listed at the next read, deleted
/// with the file manager it is gone; Add font then sees the manual copy as "already there", and
/// Remove of a family the maker already deleted by hand writes and deletes nothing else.
#[test]
fn ac27_file_manager_copy_and_delete_still_work() {
    let dir = tempfile::tempdir().unwrap();
    assert!(families(dir.path()).is_empty());

    // The maker copies a file in by hand, under a name of his own.
    let manual = dir.path().join("my script.ttf");
    std::fs::copy(included(SCRIPT), &manual).unwrap();
    let listed = list_font_folder(dir.path());
    assert_eq!(families(dir.path()), vec!["Great Vibes".to_owned()]);
    assert!(listed.faces.iter().all(|f| f.source == FontSource::Curvyo));

    // Add font of the same bytes: "already there", nothing copied.
    let added = add_font_files(dir.path(), &[included(SCRIPT)]);
    assert!(added.added.is_empty(), "{added:?}");
    assert_eq!(added.present.len(), 1, "{added:?}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);

    // Add font of another file works next to the manual copy.
    let added = add_font_files(dir.path(), &[included(SANS)]);
    assert_eq!(added.added.len(), 1, "{added:?}");
    assert_eq!(
        families(dir.path()),
        vec!["Great Vibes".to_owned(), "Source Sans 3".to_owned()]
    );

    // The maker deletes the manual copy by hand: gone at the next read.
    std::fs::remove_file(&manual).unwrap();
    assert_eq!(families(dir.path()), vec!["Source Sans 3".to_owned()]);

    // A Remove for the already deleted file deletes nothing else and does not panic.
    let removed = remove_font_files(dir.path(), &[(manual.clone(), 0)]);
    assert!(removed.removed.is_empty(), "{removed:?}");
    assert_eq!(families(dir.path()), vec!["Source Sans 3".to_owned()]);

    // Remove of the added family by its listed path deletes exactly that file.
    let face = list_font_folder(dir.path()).faces.remove(0);
    let removed = remove_font_files(dir.path(), &[(face.path.clone(), face.index)]);
    assert_eq!(removed.removed.len(), 1, "{removed:?}");
    assert!(families(dir.path()).is_empty());
    // The originals stay where they were (criterion 21).
    assert!(included(SANS).is_file() && included(SCRIPT).is_file());
}

/// Criterion 25 and the core rule "nothing outside the Curvyo folder": Remove refuses paths
/// outside the folder, `..` escapes and a file in a sub-folder, and never deletes them.
/// (`specs/0083-font-info-and-zip` criterion 14 widened Remove to subfolders one level deep; the
/// sub-folder case is now two levels deep, which stays refused.)
#[test]
fn ac25_remove_never_leaves_the_folder() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("fonts");
    std::fs::create_dir(&dir).unwrap();
    let outside = root.path().join("outside.ttf");
    std::fs::copy(included(SCRIPT), &outside).unwrap();
    let sub = dir.join("sub").join("deeper");
    std::fs::create_dir_all(&sub).unwrap();
    let nested = sub.join("nested.ttf");
    std::fs::copy(included(SCRIPT), &nested).unwrap();

    let escapes = [
        (outside.clone(), 0),
        (dir.join("..").join("outside.ttf"), 0),
        (nested.clone(), 0),
        (dir.clone(), 0),
    ];
    let removed = remove_font_files(&dir, &escapes);
    assert!(removed.removed.is_empty(), "{removed:?}");
    assert!(
        outside.is_file(),
        "a file outside the folder is never deleted"
    );
    assert!(nested.is_file(), "a file in a sub-folder is never deleted");
    assert!(dir.is_dir());
}

/// Criteria 21 and 32: a file that is not a font, an empty file, a missing path and a font
/// whose extension is not accepted are "not fonts"; nothing is written; the taken-name rule
/// gives `-2` for other bytes under the same name, compared without case.
#[test]
fn ac21_refusals_and_name_collision() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let text = src.path().join("notes.ttf");
    std::fs::write(&text, b"not a font at all").unwrap();
    let empty = src.path().join("empty.otf");
    std::fs::write(&empty, b"").unwrap();
    let missing = src.path().join("gone.ttf");
    let woff = src.path().join("font.woff");
    std::fs::copy(included(SERIF), &woff).unwrap();

    let added = add_font_files(dir.path(), &[text, empty, missing, woff]);
    assert!(added.added.is_empty(), "{added:?}");
    assert_eq!(added.not_fonts.len(), 4, "{added:?}");
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "nothing written"
    );

    // Same name, other bytes, different case: copied as `-2`.
    let a = src.path().join("Font.ttf");
    std::fs::copy(included(SERIF), &a).unwrap();
    let sub = src.path().join("b");
    std::fs::create_dir(&sub).unwrap();
    let b = sub.join("font.TTF");
    std::fs::copy(included(SCRIPT), &b).unwrap();
    let added = add_font_files(dir.path(), &[a, b]);
    assert_eq!(added.added.len(), 2, "{added:?}");
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(
        names.iter().any(|n| n.to_lowercase().contains("-2")),
        "{names:?}"
    );
    assert!(
        names.iter().all(|n| !n.starts_with('.')),
        "no temp file left: {names:?}"
    );
}

/// Criteria 21 and 32, white box (the page now hands any path to `add_font_files`): a device
/// behind a font name, a directory with a font name, a path ending in `..` and a relative path
/// are refused without hanging; the original is never touched and nothing is written outside
/// the folder; the report names files only, never contents.
#[cfg(unix)]
#[test]
fn ac21_page_supplied_paths_are_refused_unless_readable_fonts() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let device = src.path().join("zero.ttf");
    std::os::unix::fs::symlink("/dev/zero", &device).unwrap();
    let folder = src.path().join("folder.otf");
    std::fs::create_dir(&folder).unwrap();
    let up = src.path().join("..");
    let relative = PathBuf::from("../../etc/passwd.ttf");
    let font = src.path().join("Serif.ttf");
    std::fs::copy(included(SERIF), &font).unwrap();
    let before = std::fs::read(&font).unwrap();

    let started = Instant::now();
    let added = add_font_files(dir.path(), &[device, folder, up, relative, font.clone()]);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(added.not_fonts.len(), 4, "{added:?}");
    assert_eq!(added.added, ["Serif.ttf"], "{added:?}");
    assert_eq!(std::fs::read(&font).unwrap(), before, "original unchanged");
    let mut outside: Vec<_> = std::fs::read_dir(src.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    outside.sort();
    assert_eq!(outside, ["Serif.ttf", "folder.otf", "zero.ttf"]);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

/// Criterion 16: 2,000 system-like faces are listed with their class bytes and classified within
/// `0074` criterion 20's 2 s. Run with `cargo nextest run --release -p curvyo-storage-io --test
/// acceptance_0075_tester --run-ignored only`.
#[test]
#[ignore = "release-mode benchmark"]
fn ac16_two_thousand_faces_listed_and_classified_in_two_seconds() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let sources = [SANS, SERIF, SCRIPT, "source-serif-4/SourceSerif4-Bold.ttf"];
    for i in 0..2000 {
        let src = included(sources[i % sources.len()]);
        let ext = src.extension().unwrap().to_string_lossy().into_owned();
        let to = dir.path().join(format!("face-{i:04}.{ext}"));
        // Hard links keep the disk small; every name is its own listed file.
        if std::fs::hard_link(&src, &to).is_err() {
            std::fs::copy(&src, &to).unwrap();
        }
    }
    // Warm the OS cache once, as a second start would.
    let _ = list_font_folder(dir.path());

    let t = Instant::now();
    let listed = list_font_folder(dir.path());
    let classes: Vec<_> = listed
        .faces
        .iter()
        .map(|f| f.os2.and_then(declared_class))
        .collect();
    let took = t.elapsed();
    assert_eq!(listed.faces.len(), 2000);
    // Of the four sources only Source Sans 3 declares a class (PANOSE Latin Text, sans style);
    // Source Serif 4 and Great Vibes declare none (sFamilyClass 0, criterion 15's rules).
    assert_eq!(
        classes
            .iter()
            .filter(|c| **c == Some(DeclaredClass::Sans))
            .count(),
        500
    );
    assert_eq!(classes.iter().filter(|c| c.is_some()).count(), 500);
    eprintln!("ac16: 2000 faces listed and classified in {took:?}");
    assert!(took < Duration::from_secs(2), "took {took:?}");
}
