//! The image clipboard's values across the wasm boundary (`specs/0087-image-clipboard-and-drop`
//! criteria 4, 5, 7, 11 to 15): the worker's PNG, the paste rank as a number and the notices by
//! kind. Host-independent, so ordinary `cargo test` covers it.

use curvyo_ui_core::{
    COPY_NEEDS_NEWER_BROWSER_NOTICE, COPYING_NOTICE, LINK_DROP_NOTICE, NOTHING_TO_PASTE_NOTICE,
    PASTING_NOTICE, PasteRank, copy_failed_notice,
};

/// The PNG of `rgba` (`width` x `height`, straight alpha) for the clipboard (criteria 12 and 14).
///
/// # Errors
/// The reason in words when the pixels are over the working limits or do not match the size.
pub fn encode_rgba_png(rgba: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let image = curvyo_image_core::WorkingImage {
        width,
        height,
        pixels: rgba,
        original_size: (width, height),
        truncated: false,
        animated: false,
    };
    curvyo_image_core::encode_png(&image).map_err(|error| error.to_string())
}

/// How a clipboard file of `bytes` labelled `mime` serves a paste: 2 an image by its content, 1 a
/// raster type the import refuses with its notice, 0 nothing to paste (criteria 4, 5 and 7).
#[must_use]
pub fn paste_rank_code(bytes: &[u8], mime: &str) -> u8 {
    match curvyo_ui_core::paste_rank(bytes, mime) {
        PasteRank::Image => 2,
        PasteRank::Refused => 1,
        PasteRank::None => 0,
    }
}

/// A notice of the image clipboard by `kind`: `"nothing"` (criterion 5), `"pasting"`, `"copying"` (criterion 14),
/// `"failed"` with its `reason`, `"newer-browser"` (criterion 15) and `"link"` (a dropped link,
/// criterion 21); empty for another kind.
#[must_use]
pub fn clipboard_notice(kind: &str, reason: &str) -> String {
    match kind {
        "nothing" => NOTHING_TO_PASTE_NOTICE.to_owned(),
        "pasting" => PASTING_NOTICE.to_owned(),
        "copying" => COPYING_NOTICE.to_owned(),
        "failed" => copy_failed_notice(reason),
        "newer-browser" => COPY_NEEDS_NEWER_BROWSER_NOTICE.to_owned(),
        "link" => LINK_DROP_NOTICE.to_owned(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_notice() {
        // No success notice after a copy: the customer rule of 2026-10-31.
        assert_eq!(clipboard_notice("copied", ""), "");
        assert_eq!(
            clipboard_notice("failed", "Write permission denied."),
            "Could not copy the image: Write permission denied."
        );
        assert_eq!(clipboard_notice("other", ""), "");
        for kind in ["nothing", "pasting", "copying", "newer-browser", "link"] {
            assert!(!clipboard_notice(kind, "").is_empty(), "{kind}");
        }
    }

    #[test]
    fn the_worker_png_decodes_to_its_pixels_and_ranks_as_an_image() {
        let rgba = [1, 2, 3, 4].repeat(6);
        let png = encode_rgba_png(rgba.clone(), 3, 2).unwrap();
        assert_eq!(
            curvyo_image_core::decode_working(&png).unwrap().pixels,
            rgba
        );
        assert_eq!(paste_rank_code(&png, ""), 2);
        assert_eq!(paste_rank_code(b"GIF89a", "image/gif"), 1);
        assert_eq!(paste_rank_code(b"hello", "text/plain"), 0);
        assert!(encode_rgba_png(vec![0; 3], 1, 1).is_err());
    }
}
