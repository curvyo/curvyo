//! SVG export writes nothing of the Came from record (`specs/0137` criterion 9, ADR 0084 §7).

#![allow(clippy::unwrap_used)]

use curvyo_document_core::{
    AnchorId, CameFrom, Document, FragmentTarget, NewAnchor, Point, UnixMillis, Vec2,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{ExportScope, ImportFaces, SvgExportOptions, svg_export};

#[test]
fn the_svg_export_writes_nothing_of_came_from() {
    let source = Document::new(1);
    let square = source.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(10.0, 10.0)),
        ],
        true,
    );
    let target = Document::new(2);
    let fragment = source.extract_fragment(&[square], None).unwrap();
    target
        .insert_fragment(
            &fragment,
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::ZERO,
            },
            &CameFrom {
                document: "Unmistakable source".into(),
                page: "Lid page".into(),
                page_id: "first".into(),
                item: None,
                at: UnixMillis::new(1_764_000_000_000),
                online: None,
            },
            &[
                AnchorId::new(2, 1),
                AnchorId::new(2, 2),
                AnchorId::new(2, 3),
            ],
        )
        .unwrap();
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let text = svg_export(
        &target,
        &target.read_objects(),
        &fonts,
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap()
    .text();
    assert!(text.contains("<path"), "{text}");
    for word in [
        "came_from",
        "Unmistakable source",
        "Lid page",
        "1764000000000",
    ] {
        assert!(!text.contains(word), "{word} in {text}");
    }
}
