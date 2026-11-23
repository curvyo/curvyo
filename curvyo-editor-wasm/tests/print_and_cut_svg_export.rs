//! `specs/0121-print-and-cut` criterion 4: the SVG export of a page writes its registration marks
//! at their position, each as `<g data-curvyo-kind="registration-mark" data-curvyo-mark="n">`
//! with its circle and one path, black, no fill (ADR 0081 §3). Golden
//! `curvyo-svg-core/tests/fixtures/export/registration_marks.svg`; `CURVYO_UPDATE_GOLDEN=1`
//! rewrites it.
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_editor_wasm::{Session, SvgExportResult};
use curvyo_ui_core::SvgExportOptions;

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-svg-core/tests/fixtures/export/registration_marks.svg")
}

#[test]
fn the_svg_export_writes_every_mark_at_its_position() {
    let mut s = Session::new(121_201);
    assert!(s.add_registration_marks());
    let SvgExportResult::Written { text, .. } = s.export_svg(SvgExportOptions {
        text_as_paths: false,
        strokes_as_paths: false,
        selection_only: false,
        active_page_only: false,
    }) else {
        panic!("written");
    };
    for n in 1..=4 {
        assert!(
            text.contains(&format!(
                "data-curvyo-kind=\"registration-mark\" data-curvyo-mark=\"{n}\""
            )),
            "mark {n}: {text}"
        );
    }
    assert_eq!(text.matches("<circle").count(), 4);
    assert!(
        text.contains("cx=\"10\" cy=\"10\""),
        "mark 1 at its centre: {text}"
    );
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(golden_path(), &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden_path()).unwrap());
}
