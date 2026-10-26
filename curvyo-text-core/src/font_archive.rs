//! Reads the font and document entries of a zip archive from its bytes within the archive limits
//! (`specs/0083-font-info-and-zip` criteria 8, 9, 15 to 17 and 19; `adrs.md` decisions 1 to 3).
//!
//! Nothing here trusts a size the archive declares: the entry count is read from the end record
//! before the table is built, every entry's span is checked against the others, and each entry is
//! decompressed through a bounded reader. Font entries are only candidates: the caller checks
//! their bytes as it checks a loose font file.

use std::io::{Cursor, Read};

use zip::{CompressionMethod, ZipArchive};

use crate::archive_names::{
    base_name, extension, free_name, is_document_name, is_unsafe_path, parts, sanitised_name,
};
use crate::limits::{
    MAX_ARCHIVE_BYTES, MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_FONTS, MAX_ARCHIVE_KEPT_BYTES,
    MAX_DOCUMENT_BYTES, MAX_FACE_BYTES,
};

/// The font file kinds an archive may hold, in the order a face offered twice is taken in
/// (criterion 11: TTF, OTF, TTC, SVG).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontFileKind {
    /// `.ttf`
    Ttf,
    /// `.otf`
    Otf,
    /// `.ttc`
    Ttc,
    /// `.svg`
    Svg,
}

/// Why an entry is skipped as unsafe (counted in the notice, criterion 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsafeReason {
    /// An absolute, drive-letter, UNC or `..` name.
    Path,
    /// A symbolic link or another entry that is not a regular file.
    NotAFile,
}

/// What an archive entry is (criterion 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryClass {
    /// A font file candidate.
    Font(FontFileKind),
    /// A licence, readme or other text or PDF kept beside the fonts.
    Document,
    /// Not kept and not reported: other files, `__MACOSX`, dot files, nested archives, encrypted
    /// entries and other compression methods.
    LeftOut,
    /// Skipped and counted as unsafe.
    Unsafe(UnsafeReason),
}

/// One file entry of an archive (folders are not listed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    /// The full name for display: UTF-8, else CP437, `_` for bytes the UTF-8 flag claims but
    /// that are not UTF-8.
    pub name: String,
    /// The sanitised base name, unique among the archive's fonts and documents (criterion 9);
    /// empty for the other classes.
    pub file_name: String,
    /// What the entry is.
    pub class: EntryClass,
    /// The entry's index in the zip table.
    zip_index: usize,
}

/// Why an archive is refused as a whole (criterion 13's three reasons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ArchiveError {
    /// The bytes are not a zip archive.
    #[error("the file is not a zip archive")]
    NotZip,
    /// The archive is damaged or its entries overlap.
    #[error("the archive is damaged")]
    Damaged,
    /// Over the file, entry, font or total size limit.
    #[error("the archive is over the size limit")]
    OverLimit,
}

/// Why one entry's bytes are not handed out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EntrySkip {
    /// Damaged data, a wrong CRC-32, more bytes than declared or over the entry's own limit.
    #[error("the entry is unsafe")]
    Unsafe,
    /// The kept bytes of the archive passed 256 MiB: the whole archive is refused.
    #[error("the archive is over the size limit")]
    ArchiveOverLimit,
    /// The entry is not a font or document candidate.
    #[error("the entry is not kept")]
    NotKept,
}

/// An opened archive: its entry table and the bytes behind it.
pub struct FontArchive {
    zip: ZipArchive<Cursor<Vec<u8>>>,
    entries: Vec<ArchiveEntry>,
    kept_bytes: usize,
}

impl std::fmt::Debug for FontArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontArchive")
            .field("entries", &self.entries)
            .finish_non_exhaustive()
    }
}

/// Extensions of archives inside the archive: never opened (criterion 16).
const NESTED_ARCHIVES: [&str; 5] = ["zip", "7z", "rar", "tar", "gz"];

fn le_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn le_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le_u64(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// The end-of-central-directory record: the declared entry count and where the central
/// directory starts (zip64 when the classic fields are saturated); `None` without one.
fn end_record(bytes: &[u8]) -> Option<(u64, u64)> {
    const EOCD: u32 = 0x0605_4b50;
    const LOCATOR: u32 = 0x0706_4b50;
    const EOCD64: u32 = 0x0606_4b50;
    let last = bytes.len().checked_sub(22)?;
    let first = last.saturating_sub(usize::from(u16::MAX));
    let at = first
        + bytes[first..last + 4]
            .windows(4)
            .rposition(|window| window == EOCD.to_le_bytes())?;
    let count = u64::from(le_u16(bytes, at + 10)?);
    let start = u64::from(le_u32(bytes, at + 16)?);
    if count != u64::from(u16::MAX) && start != u64::from(u32::MAX) {
        return Some((count, start));
    }
    let locator = at.checked_sub(20)?;
    if le_u32(bytes, locator) != Some(LOCATOR) {
        return Some((count, start));
    }
    let record = usize::try_from(le_u64(bytes, locator + 8)?).ok()?;
    if le_u32(bytes, record) != Some(EOCD64) {
        return None;
    }
    Some((le_u64(bytes, record + 32)?, le_u64(bytes, record + 48)?))
}

/// The raw names whose central header sets the UTF-8 flag (general purpose bit 11). A walk that
/// meets damaged headers stops; the names read so far are kept.
fn utf8_flagged(bytes: &[u8], start: u64, count: u64) -> Vec<Vec<u8>> {
    const CENTRAL: u32 = 0x0201_4b50;
    let mut flagged = Vec::new();
    let Ok(mut at) = usize::try_from(start) else {
        return flagged;
    };
    for _ in 0..count {
        if le_u32(bytes, at) != Some(CENTRAL) {
            break;
        }
        let (Some(flags), Some(name), Some(extra), Some(comment)) = (
            le_u16(bytes, at + 8),
            le_u16(bytes, at + 28),
            le_u16(bytes, at + 30),
            le_u16(bytes, at + 32),
        ) else {
            break;
        };
        let name_at = at + 46;
        let Some(raw) = bytes.get(name_at..name_at + usize::from(name)) else {
            break;
        };
        if flags & 0x0800 != 0 && std::str::from_utf8(raw).is_err() {
            flagged.push(raw.to_vec());
        }
        at = name_at + usize::from(name) + usize::from(extra) + usize::from(comment);
    }
    flagged
}

/// The display name of an entry: UTF-8 when valid; `_` for each bad byte when the UTF-8 flag is
/// set; else CP437 as `zip` decodes it.
fn display_name(raw: &[u8], decoded: Option<String>, flagged: &[Vec<u8>]) -> String {
    if let Ok(text) = std::str::from_utf8(raw) {
        return text.to_owned();
    }
    if flagged.iter().any(|f| f.as_slice() == raw) {
        return String::from_utf8_lossy(raw).replace('\u{FFFD}', "_");
    }
    decoded.unwrap_or_else(|| String::from_utf8_lossy(raw).replace('\u{FFFD}', "_"))
}

/// What a file entry named `name` is, before its name is made unique.
fn class_of(name: &str, not_a_file: bool, readable: bool) -> EntryClass {
    if is_unsafe_path(name) {
        return EntryClass::Unsafe(UnsafeReason::Path);
    }
    if not_a_file {
        return EntryClass::Unsafe(UnsafeReason::NotAFile);
    }
    let base = base_name(name);
    let hidden =
        base.starts_with('.') || parts(name).any(|part| part.eq_ignore_ascii_case("__MACOSX"));
    let ext = extension(base).unwrap_or_default();
    if hidden || !readable || NESTED_ARCHIVES.contains(&ext.as_str()) {
        return EntryClass::LeftOut;
    }
    match ext.as_str() {
        "ttf" => EntryClass::Font(FontFileKind::Ttf),
        "otf" => EntryClass::Font(FontFileKind::Otf),
        "ttc" => EntryClass::Font(FontFileKind::Ttc),
        "svg" => EntryClass::Font(FontFileKind::Svg),
        _ if is_document_name(name) => EntryClass::Document,
        _ => EntryClass::LeftOut,
    }
}

/// Whether a Unix mode names something other than a regular file or a folder.
const fn is_special_mode(mode: u32) -> bool {
    let kind = mode & 0o170_000;
    kind != 0 && kind != 0o100_000 && kind != 0o040_000
}

impl FontArchive {
    /// Opens the archive in `bytes` and classes its entries (criteria 8, 9, 15, 16).
    ///
    /// # Errors
    /// [`ArchiveError::OverLimit`] for an archive over 256 MiB, more than 10,000 entries or more
    /// than 500 fonts; [`ArchiveError::NotZip`] for bytes that are not a zip archive;
    /// [`ArchiveError::Damaged`] for a damaged table or entries whose data overlap.
    pub fn open(bytes: Vec<u8>) -> Result<Self, ArchiveError> {
        if bytes.len() > MAX_ARCHIVE_BYTES {
            return Err(ArchiveError::OverLimit);
        }
        let (declared, start) = end_record(&bytes).ok_or(ArchiveError::NotZip)?;
        if declared > MAX_ARCHIVE_ENTRIES as u64 {
            return Err(ArchiveError::OverLimit);
        }
        let flagged = utf8_flagged(&bytes, start, declared);
        let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| ArchiveError::Damaged)?;
        if zip.len() > MAX_ARCHIVE_ENTRIES {
            return Err(ArchiveError::OverLimit);
        }
        let directory = zip.central_directory_start();
        let mut spans = Vec::with_capacity(zip.len());
        let mut entries = Vec::new();
        for zip_index in 0..zip.len() {
            let file = zip
                .by_index_raw(zip_index)
                .map_err(|_| ArchiveError::Damaged)?;
            let data_start = file.data_start().ok_or(ArchiveError::Damaged)?;
            let end = data_start
                .checked_add(file.compressed_size())
                .ok_or(ArchiveError::Damaged)?;
            spans.push((file.header_start(), end));
            let raw = file.name_raw().to_vec();
            let name = display_name(
                &raw,
                file.name().ok().map(std::borrow::Cow::into_owned),
                &flagged,
            );
            if file.is_dir() || name.ends_with(['/', '\\']) {
                continue;
            }
            let not_a_file = file.is_symlink() || file.unix_mode().is_some_and(is_special_mode);
            let readable = !file.encrypted()
                && matches!(
                    file.compression(),
                    CompressionMethod::Stored | CompressionMethod::Deflated
                );
            entries.push(ArchiveEntry {
                class: class_of(&name, not_a_file, readable),
                name,
                file_name: String::new(),
                zip_index,
            });
        }
        spans.sort_unstable();
        let overlap = spans.windows(2).any(|pair| pair[0].1 > pair[1].0);
        if overlap || spans.last().is_some_and(|last| last.1 > directory) {
            return Err(ArchiveError::Damaged);
        }
        let fonts = entries
            .iter()
            .filter(|e| matches!(e.class, EntryClass::Font(_)))
            .count();
        if fonts > MAX_ARCHIVE_FONTS {
            return Err(ArchiveError::OverLimit);
        }
        let mut taken: Vec<String> = Vec::new();
        for entry in &mut entries {
            if matches!(entry.class, EntryClass::Font(_) | EntryClass::Document) {
                let name = free_name(&sanitised_name(base_name(&entry.name)), &taken);
                taken.push(name.clone());
                entry.file_name = name;
            }
        }
        Ok(Self {
            zip,
            entries,
            kept_bytes: 0,
        })
    }

    /// The archive's file entries in archive order.
    #[must_use]
    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    /// Decompresses the font or document entry `index` of [`Self::entries`] under its limit (64
    /// MiB for a font, 16 MiB for a document), its CRC-32 and declared size checked; the caller
    /// takes one entry at a time.
    ///
    /// # Errors
    /// [`EntrySkip::NotKept`] for an entry of another class, [`EntrySkip::Unsafe`] for damaged
    /// or oversized data, [`EntrySkip::ArchiveOverLimit`] once the bytes read for the archive
    /// pass 256 MiB.
    pub fn read(&mut self, index: usize) -> Result<Vec<u8>, EntrySkip> {
        let entry = self.entries.get(index).ok_or(EntrySkip::NotKept)?;
        let limit = match entry.class {
            EntryClass::Font(_) => MAX_FACE_BYTES,
            EntryClass::Document => MAX_DOCUMENT_BYTES,
            _ => return Err(EntrySkip::NotKept),
        };
        let file = self
            .zip
            .by_index(entry.zip_index)
            .map_err(|_| EntrySkip::Unsafe)?;
        let declared = file.size();
        let bound = declared.min(limit as u64).saturating_add(1);
        let mut bytes = Vec::new();
        file.take(bound)
            .read_to_end(&mut bytes)
            .map_err(|_| EntrySkip::Unsafe)?;
        if bytes.len() > limit || bytes.len() as u64 > declared {
            return Err(EntrySkip::Unsafe);
        }
        self.kept_bytes = self.kept_bytes.saturating_add(bytes.len());
        if self.kept_bytes > MAX_ARCHIVE_KEPT_BYTES {
            return Err(EntrySkip::ArchiveOverLimit);
        }
        Ok(bytes)
    }
}
