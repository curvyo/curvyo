//! Checks a photo's bytes and names its file (ADR 0058 §1, §5; `specs/0112` criteria 27, 31).

use crate::material_record::{PhotoKind, photo_file_name};
use crate::test_id::TestId;

/// The longest side of a photo in pixels.
pub const MAX_PHOTO_SIDE: u32 = 600;
/// The most bytes of a photo after encoding (1 MiB).
pub const MAX_PHOTO_BYTES: usize = 1024 * 1024;

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Why photo bytes were refused; the text is the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PhotoRefusal {
    /// Neither a JPEG nor a PNG with a readable size.
    #[error("Use a JPEG or PNG image.")]
    NotAnImage,
    /// More than 1 MiB.
    #[error("The photo is over 1 MiB.")]
    TooLarge,
    /// A side longer than 600 px.
    #[error("The photo is over 600 px.")]
    TooWide,
}

/// A checked photo: its kind and size from its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhotoHeader {
    /// JPEG or PNG.
    pub kind: PhotoKind,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// What the host must do to keep a photo beside its record (desktop file or browser storage).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoWrite {
    /// Write `bytes` as `name`.
    Write {
        /// `<material id>.<test id>.jpg|png`.
        name: String,
        /// The image bytes as the frontend encoded them.
        bytes: Vec<u8>,
    },
    /// Remove `name`.
    Remove {
        /// `<material id>.<test id>.jpg|png`.
        name: String,
    },
}

/// The kind and size of `bytes` from the PNG `IHDR` or the JPEG `SOF` header, without decoding
/// a pixel; refused above 1 MiB or a side above 600 px.
///
/// # Errors
/// The refusal.
pub fn check_photo(bytes: &[u8]) -> Result<PhotoHeader, PhotoRefusal> {
    if bytes.len() > MAX_PHOTO_BYTES {
        return Err(PhotoRefusal::TooLarge);
    }
    let header = png_header(bytes)
        .or_else(|| jpeg_header(bytes))
        .ok_or(PhotoRefusal::NotAnImage)?;
    if header.width == 0 || header.height == 0 {
        return Err(PhotoRefusal::NotAnImage);
    }
    if header.width.max(header.height) > MAX_PHOTO_SIDE {
        return Err(PhotoRefusal::TooWide);
    }
    Ok(header)
}

/// The host write of a new photo of test `test` in material `material`, once checked.
///
/// # Errors
/// The refusal of [`check_photo`].
pub fn write_photo(
    material: &str,
    test: TestId,
    bytes: Vec<u8>,
) -> Result<(PhotoKind, PhotoWrite), PhotoRefusal> {
    let kind = check_photo(&bytes)?.kind;
    let name = photo_file_name(material, test, kind);
    Ok((kind, PhotoWrite::Write { name, bytes }))
}

/// The host write that removes the photo of `kind` of test `test` in material `material`.
#[must_use]
pub fn remove_photo(material: &str, test: TestId, kind: PhotoKind) -> PhotoWrite {
    PhotoWrite::Remove {
        name: photo_file_name(material, test, kind),
    }
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn be16(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from(u16::from_be_bytes(
        bytes.get(at..at + 2)?.try_into().ok()?,
    )))
}

fn png_header(bytes: &[u8]) -> Option<PhotoHeader> {
    if !bytes.starts_with(PNG_SIGNATURE) || bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    Some(PhotoHeader {
        kind: PhotoKind::Png,
        width: be32(bytes, 16)?,
        height: be32(bytes, 20)?,
    })
}

/// Walks the JPEG segments to the first frame header (`SOF0` to `SOF15` without `DHT`, `JPG`
/// and `DAC`); `None` when the scan starts first or the bytes end.
fn jpeg_header(bytes: &[u8]) -> Option<PhotoHeader> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut at = 2;
    loop {
        if *bytes.get(at)? != 0xff {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            0xff => at += 1,
            0x01 | 0xd0..=0xd7 => at += 2,
            0xda | 0xd9 => return None,
            0xc0..=0xcf if !matches!(marker, 0xc4 | 0xc8 | 0xcc) => {
                return Some(PhotoHeader {
                    kind: PhotoKind::Jpg,
                    height: be16(bytes, at + 5)?,
                    width: be16(bytes, at + 7)?,
                });
            }
            _ => {
                let length = usize::try_from(be16(bytes, at + 2)?).ok()?;
                if length < 2 {
                    return None;
                }
                at += 2 + length;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JPEG: &[u8] = include_bytes!("../tests/fixtures/materials/photos/7k2qx_600.jpg");
    const WIDE: &[u8] = include_bytes!("../tests/fixtures/materials/photos/wide_601.png");
    const NOT: &[u8] = include_bytes!("../tests/fixtures/materials/photos/not_an_image.jpg");
    const MATERIAL: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

    fn test() -> TestId {
        TestId::parse("7K2QX").unwrap()
    }

    #[test]
    fn a_600_px_jpeg_reads_its_size_from_the_frame_header() {
        let header = check_photo(JPEG).unwrap();
        assert_eq!(
            header,
            PhotoHeader {
                kind: PhotoKind::Jpg,
                width: 600,
                height: 400
            }
        );
        let (kind, write) = write_photo(MATERIAL, test(), JPEG.to_vec()).unwrap();
        assert_eq!(kind, PhotoKind::Jpg);
        assert_eq!(
            write,
            PhotoWrite::Write {
                name: format!("{MATERIAL}.7K2QX.jpg"),
                bytes: JPEG.to_vec()
            }
        );
    }

    #[test]
    fn a_601_px_png_is_too_wide_and_other_bytes_are_no_image() {
        assert_eq!(check_photo(WIDE), Err(PhotoRefusal::TooWide));
        assert_eq!(check_photo(NOT), Err(PhotoRefusal::NotAnImage));
        assert_eq!(check_photo(&JPEG[..20]), Err(PhotoRefusal::NotAnImage));
        assert_eq!(check_photo(&[]), Err(PhotoRefusal::NotAnImage));
        let mut png = WIDE.to_vec();
        png[16..20].copy_from_slice(&600_u32.to_be_bytes());
        assert_eq!(check_photo(&png).unwrap().kind, PhotoKind::Png);
        png[16..20].copy_from_slice(&0_u32.to_be_bytes());
        assert_eq!(check_photo(&png), Err(PhotoRefusal::NotAnImage));
    }

    #[test]
    fn more_than_one_mib_is_refused_before_the_header_is_read() {
        let mut big = JPEG.to_vec();
        big.resize(MAX_PHOTO_BYTES + 1, 0);
        assert_eq!(check_photo(&big), Err(PhotoRefusal::TooLarge));
        big.truncate(MAX_PHOTO_BYTES);
        assert!(check_photo(&big).is_ok());
    }

    #[test]
    fn the_removal_names_the_file_of_its_kind() {
        assert_eq!(
            remove_photo(MATERIAL, test(), PhotoKind::Png),
            PhotoWrite::Remove {
                name: format!("{MATERIAL}.7K2QX.png")
            }
        );
        assert!(PhotoRefusal::NotAnImage.to_string().chars().count() <= 70);
    }
}
