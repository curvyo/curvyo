//! SVG export of sketch instances (`specs/0099-sketch-pages` criterion 29, `0024` criterion 59): a
//! clone that holds own values is a path, never a `<use>`; a clone without own values may stay a
//! `<use>`. Golden `fixtures/export_sketch_instance.svg`.
#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{
    Affine, Document, ExposedKey, InstanceValue, InstanceWrite, NodeId, Style, ValueSet,
};
use curvyo_sketch_core as sk;
use curvyo_svg_core::write_svg;
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{ExportScope, ImportFaces, SvgExportOptions, svg_export};

fn graph() -> sk::Sketch {
    let mut s = sk::Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        sk::Entity::Point {
            position: sk::Position::mm(60.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        sk::Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    s.parameters.insert(
        "width".into(),
        sk::Parameter::literal("width", sk::Quantity::length(60.0).unwrap()),
    );
    s.constraints.insert(
        "horizontal".into(),
        sk::Constraint::driving(sk::Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        sk::Constraint::driving(sk::Relation::Length {
            curve: "line".into(),
            value: sk::Expression::Reference("width".into()),
        }),
    );
    s
}

fn svg(d: &Document) -> String {
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let laid = d.read_objects();
    let outcome = svg_export(
        d,
        &laid,
        &fonts,
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap();
    write_svg(&outcome.svg)
}

fn document() -> (Document, [NodeId; 3]) {
    let d = Document::new(1);
    let sketch = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let key = ExposedKey::parameter(&"width".into());
    d.expose_parameter(sketch, &key, None).unwrap();
    let plain = d.linked_clone(&[sketch], &[]).unwrap()[0];
    let own = d.linked_clone(&[sketch], &[]).unwrap()[0];
    let mut set = ValueSet::default();
    set.insert(key, InstanceValue::from_mm(90.0));
    d.write_instance_values(
        &[InstanceWrite {
            clone: own,
            own: set,
            last_solvable: None,
            unlinked: curvyo_document_core::KeySet::default(),
        }],
        "change_parameter",
        None,
    )
    .unwrap();
    let below = d.linked_clone(&[own], &[]).unwrap()[0];
    (d, [plain, own, below])
}

#[test]
fn a_clone_with_own_values_is_a_path_and_the_others_may_be_uses() {
    let (d, _) = document();
    let text = svg(&d);
    assert!(
        text.contains("<path id=\"path2\" d=\"M 0 0 L 90 0\""),
        "the clone with own values is a path:\n{text}"
    );
    assert_eq!(
        text.matches("<use").count(),
        2,
        "the two clones without own values stay uses:\n{text}"
    );
    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/export_sketch_instance.svg");
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}
