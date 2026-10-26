//! Every sentence a notice of the image import shows is the same string in the code and in the
//! user page `docs/importing-images.md` (`specs/0058-bitmap-import` criterion 47).

#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_image_core::ImageError;
use curvyo_ui_core::{
    ANIMATED_NOTICE, BAD_ASSET_NOTICE, DAMAGED_IMAGE_NOTICE, IMAGE_COMMAND_REFUSAL,
    OUT_OF_MEMORY_NOTICE, REDUCED_NOTICE, UNSUPPORTED_FILE_NOTICE, batch_failures_notice,
    failure_reason, fit_notice, project_full_reason, size_notice, skipped_notice, truncated_notice,
};

#[test]
fn every_notice_is_quoted_in_the_user_page() {
    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/importing-images.md"),
    )
    .unwrap();
    let texts = [
        fit_notice(0.2),
        REDUCED_NOTICE.to_owned(),
        size_notice(24 * 1_048_576).unwrap(),
        ANIMATED_NOTICE.to_owned(),
        truncated_notice("photo.jpg"),
        skipped_notice(20, 31),
        batch_failures_notice(2, "x")
            .lines()
            .next()
            .unwrap()
            .to_owned(),
        UNSUPPORTED_FILE_NOTICE.to_owned(),
        DAMAGED_IMAGE_NOTICE.to_owned(),
        failure_reason(ImageError::TooManyPixels {
            width: 12_000,
            height: 9_000,
        }),
        failure_reason(ImageError::FileTooLarge {
            bytes: 140 * 1_048_576,
        }),
        project_full_reason(140 * 1_048_576),
        OUT_OF_MEMORY_NOTICE.to_owned(),
        BAD_ASSET_NOTICE.to_owned(),
        IMAGE_COMMAND_REFUSAL.to_owned(),
    ];
    for text in texts {
        assert!(
            page.contains(&format!("- {text}\n")),
            "not in the page: {text}"
        );
    }
}
