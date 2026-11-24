//! The container members the library reads and a save joins (ADR 0084 §1, decision 10): the
//! manifest's format, `library.json` and the `pages/<id>.png` pictures read from any zip archive
//! without opening the document, and the join of `thumbnail.png` and the page pictures into the
//! packed bytes. Pure over bytes and readers, so the desktop host (a `File`) and the browser
//! build (the stored entry) share one reader.

use std::io::{Cursor, Read, Seek, Write as _};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::library_summary::{LIBRARY_MEMBER, MAX_LIBRARY_BYTES};

/// The folder of the page pictures inside a container.
pub const PAGES_PREFIX: &str = "pages/";
/// The member of the active page's picture (`specs/0095` criterion 6).
pub const THUMBNAIL_MEMBER: &str = "thumbnail.png";
/// The largest picture a row reads and a save writes.
pub const MAX_PICTURE_BYTES: u64 = 1024 * 1024;
/// The most page pictures a save writes (ADR 0084 §1, **Proposal**).
pub const MAX_PAGE_PICTURES: usize = 32;
const MANIFEST_MEMBER: &str = "manifest.json";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// What the library shows of one document file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryMembers {
    /// A container with a manifest.
    Readable {
        /// The manifest's `format_version` (0 when it names none).
        format_version: u32,
        /// The bytes of `library.json`, when present and at most 256 KiB.
        summary: Option<Vec<u8>>,
        /// The page ids that have a `pages/<id>.png` member.
        pictures: Vec<String>,
    },
    /// Not a zip archive, or one without a manifest.
    NotCurvyo,
    /// A zip archive that cannot be read through.
    Damaged,
}

/// A container member as read with a size limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZipMember {
    /// The archive has no such member.
    Missing,
    /// The member is larger than the limit.
    TooLarge,
    /// The member cannot be read through.
    Broken,
    /// The member's bytes.
    Read(Vec<u8>),
}

use ZipMember as Member;

/// The member `name` of `archive`, at most `limit` bytes: the one member reader of the library,
/// the workspace rows and the share members.
pub fn read_zip_member<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> ZipMember {
    let mut member = match archive.by_name(name) {
        Ok(member) => member,
        Err(zip::result::ZipError::FileNotFound) => return Member::Missing,
        Err(_) => return Member::Broken,
    };
    if member.size() > limit {
        return Member::TooLarge;
    }
    let mut bytes = Vec::new();
    match member.by_ref().take(limit + 1).read_to_end(&mut bytes) {
        Ok(_) if bytes.len() as u64 <= limit => Member::Read(bytes),
        Ok(_) => Member::TooLarge,
        Err(_) => Member::Broken,
    }
}

/// What the library shows of the container in `reader`; only the zip directory, the manifest
/// and `library.json` are read.
pub fn read_library_members<R: Read + Seek>(mut reader: R) -> LibraryMembers {
    let mut signature = [0_u8; 4];
    if reader.read_exact(&mut signature).is_err()
        || !(signature == *b"PK\x03\x04" || signature == *b"PK\x05\x06")
        || reader.rewind().is_err()
    {
        return LibraryMembers::NotCurvyo;
    }
    let Ok(mut archive) = ZipArchive::new(reader) else {
        return LibraryMembers::Damaged;
    };
    let format_version = match read_zip_member(&mut archive, MANIFEST_MEMBER, MAX_MANIFEST_BYTES) {
        Member::Missing => return LibraryMembers::NotCurvyo,
        Member::Broken | Member::TooLarge => return LibraryMembers::Damaged,
        Member::Read(bytes) => match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(json) => json
                .get("format_version")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(0),
            Err(_) => return LibraryMembers::Damaged,
        },
    };
    let summary = match read_zip_member(&mut archive, LIBRARY_MEMBER, MAX_LIBRARY_BYTES) {
        Member::Read(bytes) => Some(bytes),
        Member::Missing | Member::TooLarge | Member::Broken => None,
    };
    let pictures = archive
        .file_names()
        .filter_map(|name| {
            let name = name.ok()?;
            let id = name.strip_prefix(PAGES_PREFIX)?.strip_suffix(".png")?;
            (!id.is_empty() && !id.contains('/')).then(|| id.to_owned())
        })
        .collect();
    LibraryMembers::Readable {
        format_version,
        summary,
        pictures,
    }
}

/// The PNG bytes of the picture of page `page_id` in the container in `reader`; `None` when it
/// has none, it is too large, or the bytes are no readable container.
pub fn read_page_picture<R: Read + Seek>(reader: R, page_id: &str) -> Option<Vec<u8>> {
    let mut archive = ZipArchive::new(reader).ok()?;
    let name = format!("{PAGES_PREFIX}{page_id}.png");
    match read_zip_member(&mut archive, &name, MAX_PICTURE_BYTES) {
        Member::Read(bytes) => Some(bytes),
        Member::Missing | Member::TooLarge | Member::Broken => None,
    }
}

/// Every page picture of the container in `reader`, `(page id, png)`, in one pass (the browser
/// reads a whole entry once, ADR 0084 decision 10).
pub fn read_page_pictures<R: Read + Seek>(reader: R) -> Vec<(String, Vec<u8>)> {
    let Ok(mut archive) = ZipArchive::new(reader) else {
        return Vec::new();
    };
    let ids: Vec<String> = archive
        .file_names()
        .filter_map(|name| {
            let name = name.ok()?;
            let id = name.strip_prefix(PAGES_PREFIX)?.strip_suffix(".png")?;
            (!id.is_empty() && !id.contains('/')).then(|| id.to_owned())
        })
        .collect();
    ids.into_iter()
        .filter_map(|id| {
            match read_zip_member(
                &mut archive,
                &format!("{PAGES_PREFIX}{id}.png"),
                MAX_PICTURE_BYTES,
            ) {
                Member::Read(bytes) => Some((id, bytes)),
                _ => None,
            }
        })
        .collect()
}

/// Whether `png` is a PNG a save writes: the signature and at most [`MAX_PICTURE_BYTES`].
#[must_use]
pub fn is_storable_png(png: &[u8]) -> bool {
    png.starts_with(PNG_SIGNATURE) && png.len() as u64 <= MAX_PICTURE_BYTES
}

/// The bytes are not a readable container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the document bytes are not a readable container")]
pub struct NotAContainer;

/// `container` with `thumbnail` as its `thumbnail.png` and each `(page id, png)` of `pages` as
/// `pages/<page id>.png` (`specs/0137` criterion 2), in one pass; earlier pictures are replaced
/// and those of pages no longer given are dropped. A picture that is not a storable PNG, past
/// the first [`MAX_PAGE_PICTURES`] or of a page id that is no plain name is left out. Every
/// other member is copied raw.
///
/// # Errors
/// [`NotAContainer`].
pub fn with_pictures(
    container: &[u8],
    thumbnail: Option<&[u8]>,
    pages: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, NotAContainer> {
    join(container, thumbnail, Some(pages))
}

/// `container` with `png` as its `thumbnail.png`, every other member (the page pictures too)
/// copied raw (a workspace row's picture of a file written without one, `specs/0095`).
///
/// # Errors
/// [`NotAContainer`].
pub fn with_thumbnail(container: &[u8], png: &[u8]) -> Result<Vec<u8>, NotAContainer> {
    join(container, Some(png), None)
}

/// The one join: `pages` `None` keeps the page pictures as they are.
fn join(
    container: &[u8],
    thumbnail: Option<&[u8]>,
    pages: Option<&[(String, Vec<u8>)]>,
) -> Result<Vec<u8>, NotAContainer> {
    let mut archive = ZipArchive::new(Cursor::new(container)).map_err(|_| NotAContainer)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let member = archive.by_index_raw(index).map_err(|_| NotAContainer)?;
        let name = member.name().map_err(|_| NotAContainer)?;
        if name == THUMBNAIL_MEMBER || (pages.is_some() && name.starts_with(PAGES_PREFIX)) {
            continue;
        }
        writer.raw_copy_file(member).map_err(|_| NotAContainer)?;
    }
    // A PNG is compressed already.
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut write = |name: &str, png: &[u8]| {
        writer
            .start_file(name, options)
            .and_then(|()| writer.write_all(png).map_err(Into::into))
            .map_err(|_| NotAContainer)
    };
    if let Some(png) = thumbnail.filter(|png| is_storable_png(png)) {
        write(THUMBNAIL_MEMBER, png)?;
    }
    let plain =
        |id: &str| !id.is_empty() && !id.starts_with('.') && !id.contains(['/', '\\', '\0']);
    for (id, png) in pages
        .unwrap_or_default()
        .iter()
        .filter(|(id, png)| plain(id) && is_storable_png(png))
        .take(MAX_PAGE_PICTURES)
    {
        write(&format!("{PAGES_PREFIX}{id}.png"), png)?;
    }
    Ok(writer.finish().map_err(|_| NotAContainer)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(bytes: &[u8]) -> Vec<String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut names: Vec<_> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().unwrap().to_string())
            .collect();
        names.sort();
        names
    }

    fn container(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in members {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn the_pictures_are_joined_in_one_pass_and_replaced_by_the_next_save() {
        let png = b"\x89PNG\r\n\x1a\npicture".to_vec();
        let pages = vec![
            ("first".to_owned(), png.clone()),
            ("7-3".to_owned(), png.clone()),
            ("../x".to_owned(), png.clone()),
            ("bad".to_owned(), b"not a png".to_vec()),
        ];
        let base = container(&[("manifest.json", b"{}")]);
        let once = with_pictures(&base, Some(&png), &pages).unwrap();
        assert_eq!(
            names(&once),
            [
                "manifest.json",
                "pages/7-3.png",
                "pages/first.png",
                "thumbnail.png"
            ]
        );
        let again = with_pictures(&once, None, &pages[..1]).unwrap();
        assert_eq!(names(&again), ["manifest.json", "pages/first.png"]);
        let many: Vec<_> = (0..40).map(|i| (format!("p{i}"), png.clone())).collect();
        let capped = with_pictures(&base, None, &many).unwrap();
        assert_eq!(names(&capped).len(), 1 + MAX_PAGE_PICTURES);
        assert_eq!(with_pictures(b"no zip", None, &[]), Err(NotAContainer));
    }

    #[test]
    fn the_members_are_read_without_the_document() {
        let bytes = container(&[
            ("manifest.json", br#"{"format_version": 57}"#),
            ("library.json", br#"{"format":1,"pages":[]}"#),
            ("pages/first.png", b"\x89PNG\r\n\x1a\nfirst"),
            ("pages/7-3.png", b"\x89PNG\r\n\x1a\nsecond"),
        ]);
        let LibraryMembers::Readable {
            format_version,
            summary,
            mut pictures,
        } = read_library_members(Cursor::new(bytes.as_slice()))
        else {
            panic!("readable")
        };
        pictures.sort();
        assert_eq!(format_version, 57);
        assert_eq!(summary.as_deref(), Some(&br#"{"format":1,"pages":[]}"#[..]));
        assert_eq!(pictures, ["7-3", "first"]);
        assert_eq!(
            read_page_picture(Cursor::new(bytes.as_slice()), "7-3"),
            Some(b"\x89PNG\r\n\x1a\nsecond".to_vec())
        );
        assert_eq!(
            read_page_picture(Cursor::new(bytes.as_slice()), "none"),
            None
        );
        assert_eq!(read_page_pictures(Cursor::new(bytes.as_slice())).len(), 2);
    }

    #[test]
    fn an_unreadable_container_says_why() {
        let read = |bytes: &[u8]| read_library_members(Cursor::new(bytes));
        assert_eq!(read(b"plain text"), LibraryMembers::NotCurvyo);
        assert_eq!(read(b"PK\x03\x04broken"), LibraryMembers::Damaged);
        assert_eq!(
            read(&container(&[("other.txt", b"x")])),
            LibraryMembers::NotCurvyo
        );
    }
}
