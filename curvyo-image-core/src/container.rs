//! The chunk and marker structure of a file, walked before any decoder sees the bytes
//! (`specs/0058-bitmap-import` criteria 17, 19, 20 and 21): a panic or an allocation by a declared
//! length aborts a wasm module, so lengths the file cannot hold are refused here.
//!
//! Upstream: png 0.18.1 panics on a `PLTE` whose length is not a multiple of 3 (`palette.rs:74`),
//! image-webp allocates a chunk by its declared size; both are caught by this walk, nothing is
//! filed upstream.

use crate::error::ImageError;
use crate::format::ImageFormat;

/// The largest `PLTE` chunk: 256 entries of 3 bytes.
const MAX_PLTE_BYTES: usize = 256 * 3;

/// What the walk learned: the size the file declares, read without the decoder (on a 32-bit
/// target the decoder fails on a size it cannot address before it reports it), and whether a
/// JPEG ends before its end-of-image marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Structure {
    /// Width and height as the header states them, when the walk found the header.
    pub declared: Option<(u32, u32)>,
    /// A JPEG that ends before the end marker of its main image.
    pub truncated: bool,
}

/// Walks `bytes` of `format`.
///
/// # Errors
/// [`ImageError::Damaged`] for a PNG or WebP chunk that does not fit in the file and for a `PLTE`
/// chunk of a length that is not a multiple of 3 or over 768 bytes.
pub(crate) fn walk(format: ImageFormat, bytes: &[u8]) -> Result<Structure, ImageError> {
    match format {
        ImageFormat::Png => walk_png(bytes),
        ImageFormat::WebP => walk_webp(bytes),
        ImageFormat::Jpeg => Ok(walk_jpeg(bytes)),
    }
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn be16(bytes: &[u8], at: usize) -> Option<usize> {
    Some(usize::from(u16::from_be_bytes(
        bytes.get(at..at + 2)?.try_into().ok()?,
    )))
}

fn le24(bytes: &[u8], at: usize) -> Option<u32> {
    let b = bytes.get(at..at + 3)?;
    Some(u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16)
}

/// The end of a chunk of `length` data bytes starting at `data` plus `tail`, when it fits.
fn chunk_end(bytes: &[u8], data: usize, length: u32, tail: usize) -> Result<usize, ImageError> {
    usize::try_from(length)
        .ok()
        .and_then(|length| data.checked_add(length)?.checked_add(tail))
        .filter(|&end| end <= bytes.len())
        .ok_or(ImageError::Damaged)
}

/// PNG: length, type, data, CRC per chunk, up to `IEND`.
fn walk_png(bytes: &[u8]) -> Result<Structure, ImageError> {
    let mut structure = Structure::default();
    let mut at = 8;
    while at < bytes.len() {
        let length = be32(bytes, at).ok_or(ImageError::Damaged)?;
        let kind = bytes.get(at + 4..at + 8).ok_or(ImageError::Damaged)?;
        let data = at + 8;
        let end = chunk_end(bytes, data, length, 4)?;
        let length = end - data - 4;
        match kind {
            b"IHDR" if length >= 8 => {
                structure.declared = be32(bytes, data).zip(be32(bytes, data + 4));
            }
            b"PLTE" if length % 3 != 0 || length > MAX_PLTE_BYTES => {
                return Err(ImageError::Damaged);
            }
            b"IEND" => break,
            _ => {}
        }
        at = end;
    }
    Ok(structure)
}

/// WebP: RIFF chunks of type, little-endian length and data padded to an even length.
fn walk_webp(bytes: &[u8]) -> Result<Structure, ImageError> {
    let mut structure = Structure::default();
    let riff_end = le32(bytes, 4)
        .and_then(|size| usize::try_from(size).ok())
        .map_or(bytes.len(), |size| size.saturating_add(8).min(bytes.len()));
    let mut at = 12;
    while at + 8 <= riff_end {
        let length = le32(bytes, at + 4).ok_or(ImageError::Damaged)?;
        let data = at + 8;
        let end = chunk_end(bytes, data, length, 0)?;
        if &bytes[at..at + 4] == b"VP8X" && end - data >= 10 {
            structure.declared = le24(bytes, data + 4)
                .zip(le24(bytes, data + 7))
                .map(|(w, h)| (w + 1, h + 1));
        }
        at = end + (end - data) % 2;
    }
    Ok(structure)
}

/// JPEG: segments by their length (so a thumbnail inside `APP1` is skipped whole), the entropy
/// data after a start of scan up to the next marker, until the end-of-image marker. Malformed
/// structure is left to the decoder; only running out of bytes counts as truncated.
fn walk_jpeg(bytes: &[u8]) -> Structure {
    let mut structure = Structure::default();
    let mut at = 2;
    loop {
        if bytes.get(at) != Some(&0xFF) {
            structure.truncated = at >= bytes.len();
            return structure;
        }
        while bytes.get(at + 1) == Some(&0xFF) {
            at += 1;
        }
        let Some(&marker) = bytes.get(at + 1) else {
            structure.truncated = true;
            return structure;
        };
        at += 2;
        match marker {
            0xD9 => return structure,
            0x01 | 0xD0..=0xD8 => continue,
            _ => {}
        }
        let Some(length) = be16(bytes, at) else {
            structure.truncated = true;
            return structure;
        };
        if length < 2 {
            return structure;
        }
        let end = at + length;
        if end > bytes.len() {
            structure.truncated = true;
            return structure;
        }
        let start_of_frame = matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if start_of_frame && structure.declared.is_none() && length >= 7 {
            structure.declared = be16(bytes, at + 5)
                .zip(be16(bytes, at + 3))
                .and_then(|(w, h)| Some((u32::try_from(w).ok()?, u32::try_from(h).ok()?)));
        }
        at = end;
        if marker == 0xDA {
            at = after_entropy_data(bytes, at);
        }
    }
}

/// The position of the next marker after entropy-coded data starting at `at` (`bytes.len()` when
/// there is none): `FF 00` is a stuffed byte and `FF D0` to `FF D7` are restart markers.
fn after_entropy_data(bytes: &[u8], mut at: usize) -> usize {
    while at < bytes.len() {
        if bytes[at] == 0xFF {
            match bytes.get(at + 1) {
                Some(0x00 | 0xD0..=0xD7) => at += 2,
                _ => return at,
            }
        } else {
            at += 1;
        }
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_chunk(kind: [u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
        out.extend_from_slice(&kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&[0; 4]);
        out
    }

    fn png(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        for chunk in chunks {
            out.extend_from_slice(chunk);
        }
        out
    }

    fn ihdr(width: u32, height: u32) -> Vec<u8> {
        let mut data = width.to_be_bytes().to_vec();
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&[8, 3, 0, 0, 0]);
        png_chunk(*b"IHDR", &data)
    }

    #[test]
    fn a_png_declares_its_ihdr_size_and_a_palette_must_be_whole_entries() {
        let iend = png_chunk(*b"IEND", &[]);
        for (plte_len, ok) in [(3, true), (768, true), (4, false), (8, false), (771, false)] {
            let file = png(&[
                ihdr(60_000, 7),
                png_chunk(*b"PLTE", &vec![0; plte_len]),
                iend.clone(),
            ]);
            let walked = walk(ImageFormat::Png, &file);
            assert_eq!(walked.is_ok(), ok, "PLTE of {plte_len}");
            if ok {
                assert_eq!(walked.unwrap().declared, Some((60_000, 7)));
            }
        }
    }

    #[test]
    fn a_png_chunk_longer_than_the_file_is_damaged() {
        let mut file = png(&[ihdr(2, 1), png_chunk(*b"IDAT", &[1, 2, 3])]);
        let at = file.len() - 4 - 3 - 8;
        file[at..at + 4].copy_from_slice(&0xFFFF_FFF0_u32.to_be_bytes());
        assert_eq!(walk(ImageFormat::Png, &file), Err(ImageError::Damaged));
        assert_eq!(
            walk(ImageFormat::Png, &file[..file.len() - 5]),
            Err(ImageError::Damaged)
        );
    }

    fn webp(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut body = b"WEBP".to_vec();
        for (kind, data) in chunks {
            body.extend_from_slice(*kind);
            body.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
            body.extend_from_slice(data);
            if data.len() % 2 == 1 {
                body.push(0);
            }
        }
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
        out.extend(body);
        out
    }

    #[test]
    fn a_webp_declares_its_canvas_and_every_chunk_must_fit() {
        let vp8x = vec![0, 0, 0, 0, 0x5F, 0xEA, 0x00, 0x5F, 0xEA, 0x00];
        let file = webp(&[
            (b"VP8X", vp8x),
            (b"EXIF", vec![1, 2, 3]),
            (b"VP8L", vec![0; 4]),
        ]);
        assert_eq!(
            walk(ImageFormat::WebP, &file).unwrap().declared,
            Some((60_000, 60_000))
        );
        let mut huge = file.clone();
        huge[16 + 18..16 + 22].copy_from_slice(&0xFF00_0000_u32.to_le_bytes());
        assert_eq!(walk(ImageFormat::WebP, &huge), Err(ImageError::Damaged));
        assert_eq!(
            walk(ImageFormat::WebP, &file[..file.len() - 1]),
            Err(ImageError::Damaged)
        );
    }

    fn segment(marker: u8, data: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, marker];
        out.extend_from_slice(&u16::try_from(data.len() + 2).unwrap().to_be_bytes());
        out.extend_from_slice(data);
        out
    }

    fn jpeg(width: u16, height: u16, scan: &[u8], end: bool) -> Vec<u8> {
        let mut out = vec![0xFF, 0xD8];
        let mut sof = vec![8];
        sof.extend_from_slice(&height.to_be_bytes());
        sof.extend_from_slice(&width.to_be_bytes());
        sof.extend_from_slice(&[1, 1, 0x11, 0]);
        out.extend(segment(0xC0, &sof));
        out.extend(segment(0xDA, &[1, 1, 0, 0, 63, 0]));
        out.extend_from_slice(scan);
        if end {
            out.extend_from_slice(&[0xFF, 0xD9]);
        }
        out
    }

    #[test]
    fn a_jpeg_without_its_end_marker_is_truncated_and_declares_its_frame_size() {
        let scan = [1, 0xFF, 0x00, 2, 0xFF, 0xD3, 3];
        let whole = walk_jpeg(&jpeg(65_535, 65_535, &scan, true));
        assert_eq!(whole.declared, Some((65_535, 65_535)));
        assert!(!whole.truncated);
        assert!(walk_jpeg(&jpeg(4, 3, &scan, false)).truncated);
        assert!(walk_jpeg(&[0xFF, 0xD8]).truncated);
    }

    #[test]
    fn a_thumbnail_inside_app1_does_not_end_the_main_image() {
        let thumbnail = jpeg(2, 2, &[5, 6], true);
        let mut file = vec![0xFF, 0xD8];
        file.extend(segment(0xE1, &thumbnail));
        file.extend_from_slice(&jpeg(64, 48, &[7, 8, 9], false)[2..]);
        let walked = walk_jpeg(&file);
        assert_eq!(walked.declared, Some((64, 48)));
        assert!(walked.truncated);
    }
}
