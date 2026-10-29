//! What the import dialog's preview pane shows for the highlighted path (`specs/0087-image-clipboard-and-drop`
//! criteria 26 and 28): the picture with its pixel size (an SVG: its page with the size in mm),
//! "Too large", the name only, or nothing.
//!
//! It only reads the highlighted file, through `curvyo-image-core`'s bounded decoder with the
//! import's ceilings or, for an `.svg`, through the SVG importer and its limits
//! ([`svg_thumbnail`]), and never writes. The GTK pane of `import_dialog` draws the answer.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use curvyo_image_core::{ImageError, MAX_FILE_BYTES, WorkingImage, probe, thumbnail};
use curvyo_svg_core::SvgError;

use crate::preview_fonts::PreviewFonts;
use crate::svg_preview::{mm_size_line, svg_thumbnail};

/// The longest side of the preview's picture, in pixels (criterion 26).
pub(crate) const PREVIEW_SIDE: u32 = 256;

/// The text under a file that is over the ceilings (criterion 28).
pub(crate) const TOO_LARGE: &str = "Too large";

/// What the pane shows.
#[derive(Debug, PartialEq)]
pub(crate) enum Preview {
    /// A folder or no path: an empty pane.
    Nothing,
    /// A file that cannot be read or is not a PNG, JPEG, WebP or SVG: its name only.
    NameOnly { name: String },
    /// A file over the import's ceilings, never decoded: its name and [`TOO_LARGE`].
    TooLarge { name: String },
    /// The picture, its longest side at most [`PREVIEW_SIDE`], and its size line (pixels, or
    /// millimetres for an SVG).
    Picture {
        picture: WorkingImage,
        size_line: String,
    },
}

/// The size line of a picture of `width` x `height` pixels (upright), "3000 × 2000 px".
pub(crate) fn size_line(width: u32, height: u32) -> String {
    format!("{width} × {height} px")
}

/// Whether the file called `name` is previewed as an SVG: its extension is `.svg`, as the
/// Import dialog's filter and Import SVG choose it.
fn is_svg(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
}

/// The file ceiling of `name`: the SVG importer's 50 MiB for an SVG, else the image import's.
fn ceiling_of(name: &str) -> u64 {
    if is_svg(name) {
        curvyo_svg_core::limits::MAX_FILE_BYTES as u64
    } else {
        MAX_FILE_BYTES
    }
}

/// The preview of the file at `path`: a folder shows nothing, a file over its ceiling is "Too
/// large" without a read, anything else is read (at most the ceiling) and judged by
/// [`preview_from`].
pub(crate) fn preview_of(path: &Path, fonts: &mut PreviewFonts) -> Preview {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let length = match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => return Preview::Nothing,
        Ok(metadata) => metadata.len(),
        Err(_) => return Preview::NameOnly { name },
    };
    let ceiling = ceiling_of(&name);
    preview_from(name, length, || read_bounded(path, ceiling), fonts)
}

/// The preview of a file called `name` of `length` bytes whose bytes `read` gives: "Too large"
/// over a ceiling (`read` is not called over the file ceiling), the size line from the header,
/// the picture from the bounded decode, the name only for anything unreadable or unsupported.
/// An `.svg` is judged by [`svg_preview`].
pub(crate) fn preview_from(
    name: String,
    length: u64,
    read: impl FnOnce() -> io::Result<Vec<u8>>,
    fonts: &mut PreviewFonts,
) -> Preview {
    if length > ceiling_of(&name) {
        return Preview::TooLarge { name };
    }
    let Ok(bytes) = read() else {
        return Preview::NameOnly { name };
    };
    if is_svg(&name) {
        return svg_preview(name, &bytes, fonts);
    }
    let refused = |error: ImageError| match error {
        ImageError::FileTooLarge { .. } | ImageError::TooManyPixels { .. } => {
            Preview::TooLarge { name: name.clone() }
        }
        ImageError::Unsupported | ImageError::Damaged | ImageError::OutOfMemory => {
            Preview::NameOnly { name: name.clone() }
        }
    };
    // The size line comes from the header alone (criterion 26).
    let (width, height) = match probe(&bytes) {
        Ok(info) => info.upright_size(),
        Err(error) => return refused(error),
    };
    match thumbnail(&bytes, PREVIEW_SIDE) {
        Ok(picture) => Preview::Picture {
            picture,
            size_line: size_line(width, height),
        },
        Err(error) => refused(error),
    }
}

/// The preview of the SVG `bytes`: its page and size in mm, "Too large" over the importer's
/// file limit, the name only for a file the importer refuses or that draws nothing.
fn svg_preview(name: String, bytes: &[u8], fonts: &mut PreviewFonts) -> Preview {
    match svg_thumbnail(bytes, PREVIEW_SIDE, fonts) {
        Ok(Some(svg)) => Preview::Picture {
            picture: svg.picture,
            size_line: mm_size_line(svg.page_mm),
        },
        Err(SvgError::TooLarge) => Preview::TooLarge { name },
        Ok(None) | Err(_) => Preview::NameOnly { name },
    }
}

/// The bytes of `path`, at most one byte over `ceiling`, so a file that grew after its length
/// was read is still refused by the decoder's own check and never read whole.
fn read_bounded(path: &Path, ceiling: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(ceiling + 1)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use image::ImageEncoder;

    use super::*;

    fn png_of(width: u32, height: u32) -> Vec<u8> {
        let pixels = [200_u8, 30, 30].repeat((width * height) as usize);
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new_with_quality(
            &mut out,
            image::codecs::png::CompressionType::Fast,
            image::codecs::png::FilterType::NoFilter,
        )
        .write_image(&pixels, width, height, image::ExtendedColorType::Rgb8)
        .unwrap();
        out
    }

    fn image_fixture(folder: &str, name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures")
            .join(folder)
            .join(name);
        std::fs::read(path).unwrap()
    }

    fn of(name: &str, bytes: Vec<u8>) -> Preview {
        let length = bytes.len() as u64;
        preview_from(
            name.to_string(),
            length,
            move || Ok(bytes),
            &mut PreviewFonts::none(),
        )
    }

    #[test]
    fn the_size_line_reads_like_gimps() {
        assert_eq!(size_line(3000, 2000), "3000 × 2000 px");
        assert_eq!(size_line(1069, 1471), "1069 × 1471 px");
    }

    #[test]
    fn a_png_shows_a_256_px_picture_and_its_pixel_size() {
        let Preview::Picture { picture, size_line } = of("photo.png", png_of(3000, 2000)) else {
            panic!("a picture");
        };
        assert_eq!(size_line, "3000 × 2000 px");
        assert_eq!(picture.width, 256);
        assert!(picture.height.abs_diff(171) <= 1);
    }

    #[test]
    fn a_jpeg_and_a_webp_show_their_upright_size() {
        for name in ["jpeg_exif_orientation_6.jpg", "webp_lossy.webp"] {
            let bytes = image_fixture("images", name);
            let (width, height) = probe(&bytes).unwrap().upright_size();
            let Preview::Picture {
                size_line: line, ..
            } = of(name, bytes)
            else {
                panic!("{name}: a picture");
            };
            assert_eq!(line, size_line(width, height), "{name}");
        }
    }

    #[test]
    fn a_file_over_the_ceiling_is_too_large_and_never_read() {
        let preview = preview_from(
            "huge.png".into(),
            MAX_FILE_BYTES + 1,
            || panic!("a file over the ceiling is never read"),
            &mut PreviewFonts::none(),
        );
        assert_eq!(
            preview,
            Preview::TooLarge {
                name: "huge.png".into()
            }
        );
        assert_eq!(
            of(
                "bomb.png",
                image_fixture("images", "png_bomb_60000x60000.png")
            ),
            Preview::TooLarge {
                name: "bomb.png".into()
            }
        );
    }

    #[test]
    fn unreadable_damaged_and_other_files_show_their_name_only() {
        let unreadable = preview_from(
            "locked.png".into(),
            10,
            || Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            &mut PreviewFonts::none(),
        );
        assert_eq!(
            unreadable,
            Preview::NameOnly {
                name: "locked.png".into()
            }
        );
        for name in ["png_truncated.png", "text_renamed.png", "svg_renamed.png"] {
            assert_eq!(
                of(name, image_fixture("images", name)),
                Preview::NameOnly { name: name.into() },
                "{name}"
            );
        }
    }

    #[test]
    fn an_svg_shows_its_page_and_its_size_in_mm() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm"><rect width="10" height="10"/></svg>"#;
        let Preview::Picture { picture, size_line } = of("Poster.SVG", svg.to_vec()) else {
            panic!("a picture");
        };
        assert_eq!(size_line, "210 × 297 mm");
        assert_eq!((picture.width, picture.height), (181, 256));
    }

    #[test]
    fn an_svg_over_the_importers_limit_is_too_large_and_never_read() {
        let bytes = curvyo_svg_core::limits::MAX_FILE_BYTES;
        let limit = bytes as u64;
        let preview = preview_from(
            "huge.svg".into(),
            limit + 1,
            || panic!("an SVG over the limit is never read"),
            &mut PreviewFonts::none(),
        );
        assert_eq!(
            preview,
            Preview::TooLarge {
                name: "huge.svg".into()
            }
        );
        // The importer's own check catches a file that grew after its length was read.
        assert_eq!(
            preview_from(
                "grew.svg".into(),
                10,
                || Ok(vec![b' '; bytes + 1]),
                &mut PreviewFonts::none()
            ),
            Preview::TooLarge {
                name: "grew.svg".into()
            }
        );
    }

    #[test]
    fn a_refused_or_empty_svg_shows_its_name_only() {
        for (name, bytes) in [
            ("photo.svg", png_of(4, 4)),
            ("drawing.svg", vec![0x1f, 0x8b, 8, 0]),
            (
                "empty.svg",
                br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#.to_vec(),
            ),
        ] {
            assert_eq!(
                of(name, bytes),
                Preview::NameOnly { name: name.into() },
                "{name}"
            );
        }
    }

    #[test]
    fn a_folder_shows_nothing_and_a_missing_file_its_name() {
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            preview_of(&here, &mut PreviewFonts::none()),
            Preview::Nothing
        );
        assert_eq!(
            preview_of(&here.join("no-such-file.png"), &mut PreviewFonts::none()),
            Preview::NameOnly {
                name: "no-such-file.png".into()
            }
        );
    }

    #[test]
    fn a_file_on_disk_is_read_and_previewed() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png");
        assert!(matches!(
            preview_of(&path, &mut PreviewFonts::none()),
            Preview::Picture { .. }
        ));
    }
}
