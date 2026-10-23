//! Tester cases for `specs/0076-single-line-fonts` criterion 23 on the desktop's font folder
//! that fail on the branch: hostile SVG fonts that the text engine refuses must be skipped in the
//! folder listing (criterion 1) and refused by Add font (criterion 3), so no face is listed that
//! can never load. Written from `specification.md` and the public names.
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::{Path, PathBuf};

use curvyo_storage_io::{add_font_files, list_font_folder};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-text-core/tests/fixtures/svg_fonts")
        .join(name)
}

/// The hostile fixtures of criterion 23 plus an internal subset behind a decoy `<!DOCTYPE` in a
/// comment and a nesting deeper than 256, written into `dir`.
fn hostile_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = [
        "entity.svg",
        "external-entity.svg",
        "not-well-formed.svg",
        "zero-upem.svg",
        "not-finite.svg",
    ]
    .iter()
    .map(|name| {
        let to = dir.join(name);
        std::fs::copy(fixture(name), &to).unwrap();
        to
    })
    .collect();
    let decoy = dir.join("decoy.svg");
    std::fs::write(
        &decoy,
        r#"<!-- <!DOCTYPE svg> --><!DOCTYPE svg [<!ENTITY a "Evil">]><svg><defs><font><font-face font-family="&a;" units-per-em="1000"/><glyph unicode="I" d="M0 0V700"/></font></defs></svg>"#,
    )
    .unwrap();
    files.push(decoy);
    let deep = dir.join("deep.svg");
    std::fs::write(
        &deep,
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\">{}<font><font-face font-family=\"Deep\" units-per-em=\"1000\"/><glyph unicode=\"I\" d=\"M0 0V700\"/></font>{}</svg>",
            "<g>".repeat(300),
            "</g>".repeat(300)
        ),
    )
    .unwrap();
    files.push(deep);
    files
}

/// Criteria 1 and 23: every hostile file in the Curvyo folder is skipped and counted; none is
/// listed as a face.
#[test]
fn ac23_hostile_svg_fonts_are_skipped_in_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let files = hostile_files(dir.path());
    let listed = list_font_folder(dir.path());
    let families: Vec<&str> = listed.faces.iter().map(|f| f.family.as_str()).collect();
    assert!(families.is_empty(), "listed: {families:?}");
    assert_eq!(listed.skipped.len(), files.len(), "{:?}", listed.skipped);
}

/// Criteria 3 and 23: Add font refuses every hostile file as "not a font" and copies nothing.
#[test]
fn ac23_hostile_svg_fonts_are_refused_on_add_font() {
    let source = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    let files = hostile_files(source.path());
    let added = add_font_files(folder.path(), &files);
    assert!(added.added.is_empty(), "{added:?}");
    assert_eq!(added.not_fonts.len(), files.len(), "{added:?}");
    assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 0);
}
