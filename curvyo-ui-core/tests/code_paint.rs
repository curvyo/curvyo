//! Slice 4 of `specs/0144-qr-and-barcodes` (ADR 0091 "Per-part paint" P4, P6, P7): a code with a
//! Background and a Finder colour converts to a group of its job parts plus an unassigned
//! Background path below, the job stays what it was, and the SVG export writes one path per paint.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Color, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, ObjectSnapshot, Point,
};
use curvyo_extension_core::ParamValue;
use curvyo_ui_core::instance;
use curvyo_ui_core::{AnchorIdMinter, ExtensionSettings};

const QR: &str = "curvyo.codes/qr";

fn painted() -> (Document, NodeId, ExtensionSettings) {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let id = instance::place(&document, &settings, QR, Point::new(50.0, 40.0)).unwrap();
    for (key, value) in [
        ("background", ParamValue::Switch(true)),
        (
            "background_colour",
            ParamValue::Colour(Color {
                r: 250,
                g: 240,
                b: 200,
            }),
        ),
        ("finder_colour_on", ParamValue::Switch(true)),
        (
            "finder_colour",
            ParamValue::Colour(Color { r: 200, g: 0, b: 0 }),
        ),
    ] {
        instance::change_value(&document, &settings, id, key, &value).unwrap();
    }
    (document, id, settings)
}

#[test]
fn the_record_keeps_the_paints_and_a_value_change_keeps_them_with_the_geometry() {
    let (document, id, settings) = painted();
    let record = document.generated(id).unwrap().record;
    assert_eq!(record.part_keys, ["background", "code", "finder"]);
    assert_eq!(record.part_paints.len(), 2);
    assert!(!record.part_paints["background"].job && record.part_paints["finder"].job);
    // A change that does not move the paints leaves them; the content refused keeps them with the
    // last parts (parts and paints are kept together, ADR 0091 P9).
    instance::change_value(
        &document,
        &settings,
        id,
        "content",
        &ParamValue::Text(String::new()),
    )
    .unwrap();
    let kept = document.generated(id).unwrap().record;
    assert_eq!(kept.parts, record.parts);
    assert_eq!(kept.part_paints, record.part_paints);
    // Switching the Background off drops the part and its paint together.
    instance::change_value(
        &document,
        &settings,
        id,
        "content",
        &ParamValue::Text("Hi".into()),
    )
    .unwrap();
    instance::change_value(
        &document,
        &settings,
        id,
        "background",
        &ParamValue::Switch(false),
    )
    .unwrap();
    let record = document.generated(id).unwrap().record;
    assert_eq!(record.part_keys, ["code", "finder"]);
    assert_eq!(record.part_paints.keys().collect::<Vec<_>>(), ["finder"]);
}

#[test]
fn convert_gives_a_group_of_code_and_finders_and_an_unassigned_background_below() {
    let (document, id, _) = painted();
    let page = document
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let manufacturing = page.kind.manufacturing().unwrap();
    let operation = document
        .add_operation(
            manufacturing,
            &NewOperation {
                kind: "engrave",
                name: "Engrave 1",
                settings: None,
            },
            0,
            &[vec![id]],
        )
        .unwrap();
    let members = |document: &Document| {
        document
            .operations(manufacturing)
            .unwrap()
            .into_iter()
            .find(|o| o.id == operation)
            .unwrap()
            .members
    };
    let before = document.read_objects().objects.len();
    let mut minter = AnchorIdMinter::new(1);
    let ground = instance::convert(&document, &mut minter, id)
        .unwrap()
        .expect("the Background became a path");
    assert_eq!(members(&document), vec![vec![id]], "the job is what it was");
    assert_eq!(document.leaves_of(id).len(), 2, "the code and the finders");
    assert_eq!(document.read_objects().objects.len(), before + 2);
    // The Background is a plain filled path of the symbol and its quiet zone, no operation.
    let ObjectSnapshot::Path(path) = document.object(ground).unwrap() else {
        panic!("a path")
    };
    assert!(path.style.fill.enabled && !path.style.stroke.enabled);
    assert_eq!(
        path.style.fill.color,
        Color {
            r: 250,
            g: 240,
            b: 200
        }
    );
    assert_eq!(path.paint_runs.len(), 0);
    // It sits directly below the converted object.
    let order: Vec<NodeId> = document.read_nodes().iter().map(|n| n.id).collect();
    let at = |node: NodeId| order.iter().position(|n| *n == node).unwrap();
    assert!(at(ground) < at(id));
    assert!(!document.read_objects().objects.is_empty());
}

#[test]
fn the_convert_of_a_code_without_a_background_is_the_one_path_it_was() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let id = instance::place(&document, &settings, QR, Point::new(0.0, 0.0)).unwrap();
    let mut minter = AnchorIdMinter::new(1);
    assert_eq!(instance::convert(&document, &mut minter, id).unwrap(), None);
    assert_eq!(document.leaves_of(id).len(), 1);
}

#[test]
fn the_svg_export_writes_one_path_per_paint_in_part_order() {
    use curvyo_text_core::FaceStore;
    use curvyo_ui_core::{ExportScope, ImportFaces, SvgExportOptions, svg_export};
    let (document, _, _) = painted();
    let store = FaceStore::new();
    let outcome = svg_export(
        &document,
        &document.read_objects(),
        &ImportFaces {
            faces: &[],
            store: &store,
            substitute: None,
        },
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap();
    let svg = curvyo_svg_core::write_svg(&outcome.svg);
    // The Background first (cream), the modules in the object's fill, the finders in red.
    println!("{svg}");
    let fills: Vec<&str> = svg
        .split("<path ")
        .skip(1)
        .map(|p| p.split("fill").nth(1).unwrap_or(""))
        .collect();
    assert_eq!(fills.len(), 3, "{svg}");
    assert!(fills[0].contains("#faf0c8"), "{fills:?}");
    assert!(fills[2].contains("#c80000"), "{fills:?}");
}

#[test]
fn switching_the_finder_colour_on_starts_it_as_the_fills_colour_in_the_same_step() {
    use curvyo_document_core::StyleEdit;
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let id = instance::place(&document, &settings, QR, Point::new(0.0, 0.0)).unwrap();
    let blue = Color {
        r: 10,
        g: 20,
        b: 200,
    };
    document
        .edit_style(&[id], &StyleEdit::FillColor(blue))
        .unwrap();
    let steps = document.own_step_count();
    instance::change_value(
        &document,
        &settings,
        id,
        "finder_colour_on",
        &ParamValue::Switch(true),
    )
    .unwrap();
    assert_eq!(document.own_step_count(), steps + 1, "one undo step");
    let record = document.generated(id).unwrap().record;
    assert_eq!(record.part_paints["finder"].fill, blue);
    // Once the maker chose another colour it is kept when the switch comes on again.
    let red = Color { r: 200, g: 0, b: 0 };
    instance::change_value(
        &document,
        &settings,
        id,
        "finder_colour",
        &ParamValue::Colour(red),
    )
    .unwrap();
    instance::change_value(
        &document,
        &settings,
        id,
        "finder_colour_on",
        &ParamValue::Switch(false),
    )
    .unwrap();
    instance::change_value(
        &document,
        &settings,
        id,
        "finder_colour_on",
        &ParamValue::Switch(true),
    )
    .unwrap();
    assert_eq!(
        document.generated(id).unwrap().record.part_paints["finder"].fill,
        red
    );
}

#[test]
fn the_parts_group_never_lists_the_background() {
    // The Parts group is built for the rounded box only; a code has none yet, so the Background (a
    // part that cannot be detached, ADR 0091 P6) is never offered. Detach part for the Finder is the
    // core command; its Parts group row is left open (spec 0144 slice 4 notes).
    let (document, id, settings) = painted();
    let view = instance::view(&settings, &document.generated(id).unwrap());
    assert!(view.parts.iter().all(|part| part.key != "background"));
}
