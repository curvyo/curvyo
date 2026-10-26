//! The file type by its first bytes, never by a name or MIME type (`specs/0058-bitmap-import`
//! criterion 2), and the two container facts the decoder does not report: an animated WebP and
//! a JPEG cut off before its end marker (criteria 6 and 21).

/// A supported bitmap format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// PNG.
    Png,
    /// JPEG (baseline, progressive, grey, CMYK).
    Jpeg,
    /// WebP (lossy, lossless, alpha, the first frame of an animation).
    WebP,
}

impl ImageFormat {
    /// The format of `bytes` by their signature, or `None`.
    #[must_use]
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            Some(Self::WebP)
        } else {
            None
        }
    }

    /// The file extension of an asset of this format (`specs/0058-bitmap-import` criterion 23).
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
        }
    }

    /// The format's name as the maker reads it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::WebP => "WebP",
        }
    }

    pub(crate) const fn decoder_format(self) -> image::ImageFormat {
        match self {
            Self::Png => image::ImageFormat::Png,
            Self::Jpeg => image::ImageFormat::Jpeg,
            Self::WebP => image::ImageFormat::WebP,
        }
    }
}

/// Whether `bytes` (a WebP) is an extended file with the animation flag set.
pub(crate) fn is_animated_webp(bytes: &[u8]) -> bool {
    bytes.len() > 20 && &bytes[12..16] == b"VP8X" && bytes[20] & 0x02 != 0
}

/// Whether `bytes` (a JPEG) end before their end-of-image marker: no `FF D9` follows the first
/// start-of-scan marker. Inside the scan data a `FF` is always followed by `00` or a restart
/// marker, so the pair cannot occur by chance.
pub(crate) fn is_truncated_jpeg(bytes: &[u8]) -> bool {
    let Some(scan) = bytes.windows(2).position(|pair| pair == [0xFF, 0xDA]) else {
        return true;
    };
    !bytes[scan..].windows(2).any(|pair| pair == [0xFF, 0xD9])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_decide_and_other_formats_are_none() {
        assert_eq!(
            ImageFormat::detect(b"\x89PNG\r\n\x1a\nrest"),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            ImageFormat::detect(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::detect(b"RIFF\0\0\0\0WEBPVP8 "),
            Some(ImageFormat::WebP)
        );
        for other in [
            &b"BM6\0\0\0"[..],
            b"GIF89a",
            b"II*\0",
            b"RIFF\0\0\0\0WAVE",
            b"",
            b"<svg",
        ] {
            assert_eq!(ImageFormat::detect(other), None);
        }
    }

    #[test]
    fn a_jpeg_without_its_end_marker_is_truncated() {
        assert!(is_truncated_jpeg(&[0xFF, 0xD8, 0xFF, 0xDA, 1, 2, 3]));
        assert!(!is_truncated_jpeg(&[0xFF, 0xD8, 0xFF, 0xDA, 1, 0xFF, 0xD9]));
        assert!(is_truncated_jpeg(&[0xFF, 0xD8]));
    }
}
