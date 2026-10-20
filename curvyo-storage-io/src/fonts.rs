//! Lists the font faces of the Curvyo font folder and of the operating system with `fontdb`
//! (`specs/0074-text-objects` criteria 16, 19 to 21; ADR 0020 §3). Names only: the bytes are read
//! by path when a face is needed, and shaping happens in `curvyo-text-core`.

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
}

/// The faces of the Curvyo font folder and the files in it that are not readable fonts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderFonts {
    /// Every face of every readable `.ttf`, `.otf` and `.ttc` file, by family and style.
    pub faces: Vec<FontFace>,
    /// The names of the font files that could not be read (criterion 19, "2 skipped").
    pub skipped: Vec<String>,
}

const FONT_EXTENSIONS: [&str; 3] = ["ttf", "otf", "ttc"];

/// The largest font file read (64 MiB): text-core's `MAX_FACE_BYTES` (ADR 0020 §2), checked
/// before the bytes are read, so a stray huge file never goes through memory or the IPC.
pub const MAX_FONT_FILE_BYTES: u64 = 64 * 1024 * 1024;

fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Whether face `index` of the file at `path` has vector outlines (`glyf`, `CFF ` or `CFF2`) and
/// no colour or bitmap tables (`COLR`, `CBDT`, `sbix`, `SVG `): colour and bitmap fonts are not
/// listed (criterion 21). Reads the table directory only.
fn is_outline_face(path: &Path, index: u32) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let read_at = |file: &mut std::fs::File, at: u64, len: usize| -> Option<Vec<u8>> {
        file.seek(SeekFrom::Start(at)).ok()?;
        let mut buffer = vec![0; len];
        file.read_exact(&mut buffer).ok()?;
        Some(buffer)
    };
    let check = || -> Option<bool> {
        let mut file = std::fs::File::open(path).ok()?;
        let head = read_at(&mut file, 0, 12)?;
        let directory = if &head[..4] == b"ttcf" {
            let at = 12 + 4 * u64::from(index);
            u64::from(be_u32(&read_at(&mut file, at, 4)?, 0)?)
        } else {
            0
        };
        let header = read_at(&mut file, directory, 12)?;
        let tables = usize::from(u16::from_be_bytes([header[4], header[5]]));
        let records = read_at(&mut file, directory + 12, 16 * tables)?;
        let tags: Vec<&[u8]> = records
            .as_chunks::<16>()
            .0
            .iter()
            .map(|record| &record[..4])
            .collect();
        let has = |tag: &[u8; 4]| tags.contains(&tag.as_slice());
        let outlines = has(b"glyf") || has(b"CFF ") || has(b"CFF2");
        let colour = has(b"COLR") || has(b"CBDT") || has(b"sbix") || has(b"SVG ");
        Some(outlines && !colour)
    };
    check().unwrap_or(false)
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
            if !is_outline_face(&path, face.index) {
                return None;
            }
            let family = face.families.first()?.0.clone();
            Some(FontFace {
                source,
                family,
                style: style_name(face),
                path,
                index: face.index,
            })
        })
        .collect();
    faces.sort_by(|a, b| {
        (a.family.to_lowercase(), &a.style, &a.path, a.index).cmp(&(
            b.family.to_lowercase(),
            &b.style,
            &b.path,
            b.index,
        ))
    });
    faces
}

/// Lists the font files directly in `dir` (not in sub-folders), creating the folder when it is
/// missing (criterion 19). A file with a font extension that holds no readable face is counted
/// as skipped; other files are ignored.
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
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| FONT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    files.sort();
    for path in files {
        let before = db.len();
        if !small_enough(&path) || db.load_font_file(&path).is_err() || db.len() == before {
            skipped.push(
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
        }
    }
    FolderFonts {
        faces: faces_of(&db, FontSource::Curvyo),
        skipped,
    }
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
}
