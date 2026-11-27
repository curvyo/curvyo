//! Tester acceptance case for `specs/0139-laser-cut-path-options` criterion 11 (format): the
//! document format of this build is 68, the number the story writes. A build of 65 or lower refusing
//! the file needs that build and is not testable here.

#![allow(missing_docs)]

#[test]
fn c11_format_version_is_68() {
    assert_eq!(curvyo_document_core::CURRENT_FORMAT_VERSION, 68);
}
