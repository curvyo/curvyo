//! Lists the font faces of the Curvyo font folder and of the operating system with `fontdb`
//! (`specs/0074-text-objects` criteria 16, 19 to 21; ADR 0020 §3). Names, the 12 OS/2 class bytes
//! and a file stamp only (`specs/0075-font-library/adrs.md` decisions 4 and 5): the bytes are read
//! by path when a face is needed, and shaping and classifying happen in the core crates. SVG fonts
//! in the Curvyo folder are named by [`crate::svg_fonts`] (`specs/0076-single-line-fonts`).

use std::path::{Path, PathBuf};

/// Where a face was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSource {
    /// The Curvyo font folder.
    Curvyo,
    /// The operating system's fonts.
    System,
}

/// One face of a font file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFace {
    /// Where it was found.
    pub source: FontSource,
    /// The family name ("Source Sans 3").
    pub family: String,
    /// The style name ("Regular", "Bold Italic", "Condensed Light").
    pub style: String,
    /// The file (host side only).
    pub path: PathBuf,
    /// The face's index in the file (0 unless it is a collection).
    pub index: u32,
    /// The OS/2 bytes 30 to 41 (`sFamilyClass` and PANOSE); `None` without an OS/2 table.
    pub os2: Option<[u8; 12]>,
    /// `<length>-<modified seconds>` of the file: a cached check result still holds while it
    /// matches.
    pub stamp: String,
    /// A single-line face (an SVG font, `specs/0076-single-line-fonts` criterion 1).
    pub single_line: bool,
}

/// The faces of the Curvyo font folder and the files in it that are not readable fonts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderFonts {
    /// Every face of every readable `.ttf`, `.otf`, `.ttc` and `.svg` font file, by family and
    /// style.
    pub faces: Vec<FontFace>,
    /// The names of the font files that could not be read (criterion 19, "2 skipped").
    pub skipped: Vec<String>,
}

pub(crate) const FONT_EXTENSIONS: [&str; 3] = ["ttf", "otf", "ttc"];

/// The extension of an SVG font file (only in the Curvyo folder; the system lists none).
pub(crate) const SVG_EXTENSION: &str = "svg";

/// Whether `path` ends in `.svg`, any case.
pub(crate) fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(SVG_EXTENSION))
}

/// The largest font file read (64 MiB): text-core's `MAX_FACE_BYTES` (ADR 0020 §2), checked
/// before the bytes are read, so a stray huge file never goes through memory or the IPC.
pub const MAX_FONT_FILE_BYTES: u64 = 64 * 1024 * 1024;

fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// What the listing reads of one face from its table directory.
pub(crate) struct FaceTables {
    /// Vector outlines (`glyf`, `CFF ` or `CFF2`) and no colour or bitmap tables (`COLR`,
    /// `CBDT`, `sbix`, `SVG `): colour and bitmap fonts are not listed (criterion 21).
    pub(crate) outline: bool,
    /// The OS/2 bytes 30 to 41.
    pub(crate) os2: Option<[u8; 12]>,
}

/// Reads the table directory of face `index` and, with one more seek, its OS/2 class bytes.
pub(crate) fn face_tables(
    file: &mut (impl std::io::Read + std::io::Seek),
    index: u32,
) -> Option<FaceTables> {
    use std::io::SeekFrom;
    let mut read_at = |at: u64, len: usize| -> Option<Vec<u8>> {
        file.seek(SeekFrom::Start(at)).ok()?;
        let mut buffer = vec![0; len];
        file.read_exact(&mut buffer).ok()?;
        Some(buffer)
    };
    {
        let head = read_at(0, 12)?;
        let directory = if &head[..4] == b"ttcf" {
            let at = 12 + 4 * u64::from(index);
            u64::from(be_u32(&read_at(at, 4)?, 0)?)
        } else {
            0
        };
        let header = read_at(directory, 12)?;
        let tables = usize::from(u16::from_be_bytes([header[4], header[5]]));
        let records = read_at(directory + 12, 16 * tables)?;
        let tags: Vec<&[u8]> = records
            .as_chunks::<16>()
            .0
            .iter()
            .map(|record| &record[..4])
            .collect();
        let has = |tag: &[u8; 4]| tags.contains(&tag.as_slice());
        let outlines = has(b"glyf") || has(b"CFF ") || has(b"CFF2");
        let colour = has(b"COLR") || has(b"CBDT") || has(b"sbix") || has(b"SVG ");
        let os2_offset = records
            .as_chunks::<16>()
            .0
            .iter()
            .find(|record| &record[..4] == b"OS/2")
            .and_then(|record| be_u32(record, 8));
        let os2 = os2_offset
            .and_then(|at| read_at(u64::from(at) + 30, 12))
            .and_then(|bytes| bytes.try_into().ok());
        Some(FaceTables {
            outline: outlines && !colour,
            os2,
        })
    }
}

/// [`face_tables`] of face `index` of the file at `path`.
fn file_face_tables(path: &Path, index: u32) -> Option<FaceTables> {
    face_tables(&mut std::fs::File::open(path).ok()?, index)
}

/// `<length>-<modified seconds>` of the file at `path`; empty when it cannot be read.
fn stamp_of(path: &Path) -> String {
    std::fs::metadata(path).map_or_else(
        |_| String::new(),
        |meta| {
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |since| since.as_secs());
            format!("{}-{modified}", meta.len())
        },
    )
}

fn small_enough(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.len() <= MAX_FONT_FILE_BYTES)
}

/// The style name of a face from its weight, stretch and slant, in the usual English words
/// (`plan.md`, dated notes: `fontdb` states no subfamily name).
fn style_name(face: &fontdb::FaceInfo) -> String {
    let stretch = match face.stretch {
        fontdb::Stretch::UltraCondensed => "Ultra Condensed",
        fontdb::Stretch::ExtraCondensed => "Extra Condensed",
        fontdb::Stretch::Condensed => "Condensed",
        fontdb::Stretch::SemiCondensed => "Semi Condensed",
        fontdb::Stretch::Normal => "",
        fontdb::Stretch::SemiExpanded => "Semi Expanded",
        fontdb::Stretch::Expanded => "Expanded",
        fontdb::Stretch::ExtraExpanded => "Extra Expanded",
        fontdb::Stretch::UltraExpanded => "Ultra Expanded",
    };
    let weight = match face.weight.0 {
        0..=149 => "Thin",
        150..=249 => "Extra Light",
        250..=349 => "Light",
        350..=449 => "",
        450..=549 => "Medium",
        550..=649 => "Semibold",
        650..=749 => "Bold",
        750..=849 => "Extra Bold",
        _ => "Black",
    };
    let slant = match face.style {
        fontdb::Style::Normal => "",
        fontdb::Style::Italic => "Italic",
        fontdb::Style::Oblique => "Oblique",
    };
    let words: Vec<&str> = [stretch, weight, slant]
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        "Regular".to_owned()
    } else {
        words.join(" ")
    }
}

fn faces_of(db: &fontdb::Database, source: FontSource) -> Vec<FontFace> {
    let mut faces: Vec<FontFace> = db
        .faces()
        .filter_map(|face| {
            let path = match &face.source {
                fontdb::Source::File(path) => path.clone(),
                fontdb::Source::Binary(_) => return None,
            };
            let tables = file_face_tables(&path, face.index).filter(|t| t.outline)?;
            let family = face.families.first()?.0.clone();
            Some(FontFace {
                source,
                family,
                style: style_name(face),
                stamp: stamp_of(&path),
                path,
                index: face.index,
                os2: tables.os2,
                single_line: false,
            })
        })
        .collect();
    sort_faces(&mut faces);
    faces
}

/// Family (case-insensitive), style, file, index.
fn sort_faces(faces: &mut [FontFace]) {
    faces.sort_by(|a, b| {
        (a.family.to_lowercase(), &a.style, &a.path, a.index).cmp(&(
            b.family.to_lowercase(),
            &b.style,
            &b.path,
            b.index,
        ))
    });
}

/// Lists the font files directly in `dir` (not in sub-folders), creating the folder when it is
/// missing (criterion 19). A file with a font extension (`.svg` included) that holds no readable
/// face is counted as skipped; other files are ignored.
#[must_use]
pub fn list_font_folder(dir: &Path) -> FolderFonts {
    let _ = std::fs::create_dir_all(dir);
    let mut db = fontdb::Database::new();
    let mut skipped = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return FolderFonts::default();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            is_svg(path)
                || path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| FONT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    files.sort();
    let name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let mut svg_faces = Vec::new();
    for path in files {
        if is_svg(&path) {
            let faces = svg_faces_of(&path);
            if faces.is_empty() {
                skipped.push(name(&path));
            }
            svg_faces.extend(faces);
            continue;
        }
        let before = db.len();
        if !small_enough(&path) || db.load_font_file(&path).is_err() || db.len() == before {
            skipped.push(name(&path));
        }
    }
    let mut faces = faces_of(&db, FontSource::Curvyo);
    faces.extend(svg_faces);
    sort_faces(&mut faces);
    FolderFonts { faces, skipped }
}

/// The faces of the SVG font at `path` (empty when it is larger than 16 MiB or not a font).
fn svg_faces_of(path: &Path) -> Vec<FontFace> {
    let small = std::fs::metadata(path)
        .is_ok_and(|meta| meta.len() <= crate::svg_fonts::MAX_SVG_FONT_BYTES as u64);
    let Some(bytes) = small.then(|| std::fs::read(path).ok()).flatten() else {
        return Vec::new();
    };
    crate::svg_fonts::svg_font_faces(&bytes)
        .into_iter()
        .map(|face| FontFace {
            source: FontSource::Curvyo,
            family: face.family,
            style: face.style,
            path: path.to_path_buf(),
            index: face.index,
            os2: None,
            stamp: stamp_of(path),
            single_line: true,
        })
        .collect()
}

/// Lists the installed fonts the operating system's font configuration names (criterion 20).
/// Slow on a cold disk: the caller runs it off the UI thread, on first use only.
#[must_use]
pub fn list_system_fonts() -> Vec<FontFace> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    faces_of(&db, FontSource::System)
        .into_iter()
        .filter(|face| {
            face.path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| FONT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
                && small_enough(&face.path)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn included(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(path)
    }

    /// Criteria 19 and 21: the folder is created, every font file directly in it is listed by
    /// family and style, a damaged one is counted as skipped, sub-folders and other files are
    /// not read.
    #[test]
    fn ac19_ac21_folder_listing() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        assert_eq!(list_font_folder(&fonts), FolderFonts::default());
        assert!(fonts.is_dir(), "created on first listing");
        std::fs::copy(
            included("source-serif-4/SourceSerif4-Bold.ttf"),
            fonts.join("Serif-Bold.ttf"),
        )
        .unwrap();
        std::fs::copy(
            included("source-sans-3/SourceSans3-It.otf"),
            fonts.join("Sans-It.OTF"),
        )
        .unwrap();
        std::fs::write(fonts.join("broken.ttf"), b"not a font").unwrap();
        // A bitmap or colour font (its `GPOS` renamed `sbix` here) is not listed (criterion 21).
        let mut colour = std::fs::read(included("source-serif-4/SourceSerif4-It.ttf")).unwrap();
        let tables = usize::from(u16::from_be_bytes([colour[4], colour[5]]));
        for record in 0..tables {
            let at = 12 + 16 * record;
            if &colour[at..at + 4] == b"GPOS" {
                colour[at..at + 4].copy_from_slice(b"sbix");
            }
        }
        std::fs::write(fonts.join("Colour.ttf"), colour).unwrap();
        std::fs::write(fonts.join("notes.txt"), b"hello").unwrap();
        std::fs::create_dir(fonts.join("sub")).unwrap();
        std::fs::copy(
            included("great-vibes/GreatVibes-Regular.ttf"),
            fonts.join("sub/GreatVibes.ttf"),
        )
        .unwrap();
        let listed = list_font_folder(&fonts);
        let names: Vec<(&str, &str)> = listed
            .faces
            .iter()
            .map(|f| (f.family.as_str(), f.style.as_str()))
            .collect();
        assert_eq!(
            names,
            [("Source Sans 3", "Italic"), ("Source Serif 4", "Bold")]
        );
        assert_eq!(listed.skipped, ["broken.ttf"]);
        assert!(listed.faces.iter().all(|f| f.source == FontSource::Curvyo));
    }

    /// `specs/0076-single-line-fonts` criterion 1: an SVG font in the folder is listed by family
    /// and style as single-line, with the names text-core reads; an `.svg` that is not a font is
    /// skipped.
    #[test]
    fn ac01_svg_fonts_in_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        std::fs::create_dir(&fonts).unwrap();
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-text-core/tests/fixtures/svg_fonts");
        std::fs::copy(
            included("hershey-sans-1/HersheySans1.svg"),
            fonts.join("a.svg"),
        )
        .unwrap();
        std::fs::copy(fixtures.join("two-fonts.svg"), fonts.join("b.SVG")).unwrap();
        std::fs::copy(fixtures.join("kerning.svg"), fonts.join("c.svg")).unwrap();
        std::fs::copy(fixtures.join("not-a-font.svg"), fonts.join("logo.svg")).unwrap();
        std::fs::copy(fixtures.join("entity.svg"), fonts.join("entity.svg")).unwrap();
        let listed = list_font_folder(&fonts);
        assert_eq!(listed.skipped, ["entity.svg", "logo.svg"]);
        let names: Vec<(&str, &str, u32)> = listed
            .faces
            .iter()
            .map(|f| (f.family.as_str(), f.style.as_str(), f.index))
            .collect();
        assert_eq!(
            names,
            [
                ("Curvyo Kerning Test", "Bold Italic", 0),
                ("Hershey Sans 1-stroke", "Regular", 0),
                ("Two One", "Regular", 1),
                ("Two Two", "Bold", 2)
            ]
        );
        for face in &listed.faces {
            assert!(face.single_line && face.os2.is_none());
            let bytes = std::fs::read(&face.path).unwrap();
            let read: Vec<(u32, String, String)> = curvyo_text_core::listable_faces(&bytes)
                .into_iter()
                .map(|f| (f.index, f.family, f.style))
                .collect();
            assert!(read.contains(&(face.index, face.family.clone(), face.style.clone())));
        }
    }

    /// `specs/0075-font-library` decisions 3 and 4: the browser and Add font name a face as the
    /// listing does, and the listing carries the OS/2 class bytes text-core reads.
    #[test]
    fn listing_and_text_core_agree_on_names_and_class_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        std::fs::create_dir(&fonts).unwrap();
        for (from, to) in [
            ("source-serif-4/SourceSerif4-BoldIt.ttf", "a.ttf"),
            ("source-sans-3/SourceSans3-It.otf", "b.otf"),
            ("source-sans-3/SourceSans3-Bold.otf", "c.otf"),
            ("great-vibes/GreatVibes-Regular.ttf", "d.ttf"),
        ] {
            std::fs::copy(included(from), fonts.join(to)).unwrap();
        }
        let listed = list_font_folder(&fonts);
        assert_eq!(listed.faces.len(), 4);
        for face in &listed.faces {
            let bytes = std::fs::read(&face.path).unwrap();
            let read = curvyo_text_core::listable_faces(&bytes);
            assert_eq!(
                (read[0].family.as_str(), read[0].style.as_str()),
                (face.family.as_str(), face.style.as_str())
            );
            assert_eq!(face.os2, curvyo_text_core::os2_class_bytes(&bytes, 0));
            assert!(face.stamp.starts_with(&format!("{}-", bytes.len())));
        }
    }
}
