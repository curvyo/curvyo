//! The words of the image import (`specs/0058-bitmap-import` criteria 5, 10, 17, 18, 21, 22, 24,
//! 26, 30, 37; UX notes "Notices"): the display name of a file and every notice. The user page
//! `docs/importing-images.md` quotes the same strings (criterion 47).

use curvyo_image_core::ImageError;

use crate::svg_import_text::notice_file_name;

/// The longest display name in characters (criterion 22).
const MAX_DISPLAY_NAME_CHARS: usize = 128;

/// The refusal of a file of any other type (criterion 26; the Import button also takes SVG).
pub const UNSUPPORTED_FILE_NOTICE: &str =
    "This file type isn't supported. Use SVG, PNG, JPEG or WebP.";
/// A supported type that cannot be read (criterion 26).
pub const DAMAGED_IMAGE_NOTICE: &str = "The image is damaged and could not be read.";
/// A buffer that could not be allocated (criterion 26).
pub const OUT_OF_MEMORY_NOTICE: &str = "Not enough memory to open this image.";
/// The working limit reduced the picture (criterion 18).
pub const REDUCED_NOTICE: &str =
    "Shown and traced at reduced resolution (16 MP). Original is kept.";
/// Only the first frame of an animation is used (criterion 6).
pub const ANIMATED_NOTICE: &str = "Animated image: only the first frame was used.";
/// An image of an opened project whose asset cannot be decoded (criterion 30).
pub const BAD_ASSET_NOTICE: &str = "An image in this project could not be read.";
/// The refusal of an outline command for a selection with an image (criterion 37).
pub const IMAGE_COMMAND_REFUSAL: &str =
    "Images can't be used with this command. Nothing was changed.";

const MIB: f64 = 1_048_576.0;

/// A file name as an image's name: the folder cut off (`/` and `\`), control characters and
/// `<`, `>`, `"` removed, at most 128 characters, "Image" when nothing is left (criterion 22).
#[must_use]
pub fn image_display_name(file_name: &str) -> String {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or_default();
    let kept: String = base
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | '"'))
        .take(MAX_DISPLAY_NAME_CHARS)
        .collect();
    let kept = kept.trim();
    if kept.is_empty() {
        "Image".to_owned()
    } else {
        kept.to_owned()
    }
}

/// `n` with thousands separators: "12,000".
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Whole megabytes of `bytes`, rounded ("140 MB").
fn whole_mb(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let mb = (bytes as f64 / MIB).round();
    format!("{mb:.0} MB")
}

/// The reason line of a refused file (criteria 17 and 26).
#[must_use]
pub fn failure_reason(error: ImageError) -> String {
    match error {
        ImageError::Unsupported => UNSUPPORTED_FILE_NOTICE.to_owned(),
        ImageError::Damaged => DAMAGED_IMAGE_NOTICE.to_owned(),
        ImageError::OutOfMemory => OUT_OF_MEMORY_NOTICE.to_owned(),
        ImageError::TooManyPixels { width, height } => format!(
            "Image too large: {} x {} px. The limit is 50 megapixels.",
            grouped(u64::from(width)),
            grouped(u64::from(height))
        ),
        ImageError::FileTooLarge { bytes } => {
            format!("File too large: {}. The limit is 100 MB.", whole_mb(bytes))
        }
    }
}

/// The refusal when the project would pass 512 MiB of pictures (criterion 17).
#[must_use]
pub fn project_full_reason(adds: u64) -> String {
    format!(
        "Project too large: this image adds {}. The limit is 512 MB.",
        whole_mb(adds)
    )
}

/// A refused file's notice: its name (cut to 24 characters), then the reason (criterion 26).
#[must_use]
pub fn failure_notice(file_name: &str, reason: &str) -> String {
    format!(
        "{}\n{reason}",
        notice_file_name(&image_display_name(file_name))
    )
}

/// "Scaled to 20 % to fit the document." (criterion 10).
#[must_use]
pub fn fit_notice(factor: f64) -> String {
    format!(
        "Scaled to {:.0} % to fit the document.",
        (factor * 100.0).round()
    )
}

/// "This image adds 24.0 MB to the project file." for files of 10 MB or more (criterion 24).
#[must_use]
pub fn size_notice(bytes: u64) -> Option<String> {
    #[allow(clippy::cast_precision_loss)]
    let mb = bytes as f64 / MIB;
    (bytes >= 10 * 1_048_576).then(|| format!("This image adds {mb:.1} MB to the project file."))
}

/// "photo.jpg is incomplete; the part that could be read was imported." (criterion 21).
#[must_use]
pub fn truncated_notice(file_name: &str) -> String {
    format!(
        "{} is incomplete; the part that could be read was imported.",
        notice_file_name(&image_display_name(file_name))
    )
}

/// "Imported 20 of 31 images. The rest were skipped." (criterion 5).
#[must_use]
pub fn skipped_notice(imported: usize, chosen: usize) -> String {
    format!("Imported {imported} of {chosen} images. The rest were skipped.")
}

/// "2 files could not be imported." then the first reason (criterion 5).
#[must_use]
pub fn batch_failures_notice(failed: usize, first_reason: &str) -> String {
    if failed == 1 {
        format!("1 file could not be imported.\n{first_reason}")
    } else {
        format!("{failed} files could not be imported.\n{first_reason}")
    }
}

/// "Importing photo.jpg..." or "Importing 3 of 12 images..." (UX notes, "Busy state").
#[must_use]
pub fn importing_notice(file_name: &str, at: usize, of: usize) -> String {
    if of > 1 {
        format!("Importing {at} of {of} images...")
    } else {
        format!(
            "Importing {}...",
            notice_file_name(&image_display_name(file_name))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac22_names_lose_their_folder_and_unsafe_characters() {
        assert_eq!(image_display_name("/home/a/photo.jpg"), "photo.jpg");
        assert_eq!(image_display_name("C:\\Users\\a\\scan<1>.png"), "scan1.png");
        assert_eq!(image_display_name("a\u{7}\"b\".webp"), "ab.webp");
        assert_eq!(image_display_name("/folder/"), "Image");
        assert_eq!(image_display_name(&"x".repeat(300)).chars().count(), 128);
    }

    #[test]
    fn ac17_ac26_the_refusal_lines_name_the_numbers() {
        assert_eq!(
            failure_reason(ImageError::TooManyPixels {
                width: 12_000,
                height: 9_000
            }),
            "Image too large: 12,000 x 9,000 px. The limit is 50 megapixels."
        );
        assert_eq!(
            failure_reason(ImageError::FileTooLarge {
                bytes: 140 * 1_048_576
            }),
            "File too large: 140 MB. The limit is 100 MB."
        );
        assert_eq!(
            project_full_reason(140 * 1_048_576),
            "Project too large: this image adds 140 MB. The limit is 512 MB."
        );
        assert_eq!(
            failure_notice("/x/photo-of-the-long-name-here.jpg", DAMAGED_IMAGE_NOTICE),
            "photo-of-the-lo…here.jpg\nThe image is damaged and could not be read."
        );
    }

    #[test]
    fn ac10_ac24_success_lines() {
        assert_eq!(fit_notice(0.198_425), "Scaled to 20 % to fit the document.");
        assert_eq!(size_notice(10 * 1_048_576 - 1), None);
        assert_eq!(
            size_notice(24 * 1_048_576).as_deref(),
            Some("This image adds 24.0 MB to the project file.")
        );
        assert_eq!(
            truncated_notice("photo.jpg"),
            "photo.jpg is incomplete; the part that could be read was imported."
        );
        assert!(
            [
                REDUCED_NOTICE,
                ANIMATED_NOTICE,
                IMAGE_COMMAND_REFUSAL,
                UNSUPPORTED_FILE_NOTICE
            ]
            .iter()
            .all(|text| text.chars().count() <= 70)
        );
    }
}
