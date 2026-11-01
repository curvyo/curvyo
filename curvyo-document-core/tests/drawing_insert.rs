//! Writing an imported drawing (`specs/0024-svg-import-export` criteria 2, 3 and 77): one layer in
//! one commit that one undo removes, linked clones of paths, and a new document whose content
//! is in its setup change.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, Angle, CURRENT_FORMAT_VERSION, DisplayUnit, Document,
    DocumentSize, DrawingAnchor, DrawingNode, DrawingOutline, DrawingPage, DrawingRegisters,
    EllipseFrame, Length, ObjectEditError, ObjectSnapshot, Opacity, Point, Shape, Style, Vec2,
};

fn corner(x: f64, y: f64) -> DrawingAnchor {
    DrawingAnchor {
        point: Point::new(x, y),
        handle_in: Vec2::ZERO,
        handle_out: Vec2::ZERO,
        kind: AnchorKind::Corner,
    }
}

fn square(registers: DrawingRegisters) -> DrawingNode {
    DrawingNode::Path {
        registers,
        outlines: vec![
            DrawingOutline {
                closed: true,
                anchors: vec![corner(0.0, 0.0), corner(10.0, 0.0), corner(10.0, 10.0)],
            },
            DrawingOutline {
                closed: false,
                anchors: vec![corner(20.0, 0.0), corner(30.0, 0.0)],
            },
        ],
        style: Style::default(),
    }
}

fn drawing() -> Vec<DrawingNode> {
    vec![
        DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some("Cut".into()),
                locked: true,
                ..DrawingRegisters::default()
            },
            layer: false,
            children: vec![
                DrawingNode::Clone {
                    registers: DrawingRegisters::default(),
                    original: 1,
                    matrix: Affine::new(1.0, 0.0, 0.0, 1.0, 30.0, 0.0),
                },
                square(DrawingRegisters {
                    visible: false,
                    opacity: Opacity::new(0.5).unwrap(),
                    ..DrawingRegisters::default()
                }),
            ],
        },
        DrawingNode::Primitive {
            registers: DrawingRegisters::default(),
            shape: Shape::Ellipse {
                frame: EllipseFrame {
                    center: Point::new(5.0, 5.0),
                    rx: Length::from_mm(2.0),
                    ry: Length::from_mm(3.0),
                },
                arc: None,
                curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
            },
            rotation: Angle::from_radians(30_f64.to_radians()),
            style: Style::default(),
        },
    ]
}

fn ids(count: usize) -> Vec<AnchorId> {
    (0..count as u64).map(|n| AnchorId::new(9, n)).collect()
}

#[test]
fn ac3_one_layer_in_one_commit_and_one_undo_removes_it() {
    let document = Document::new(1);
    let nodes = drawing();
    let anchor_ids = ids(DrawingNode::anchor_count(&nodes));
    assert_eq!(anchor_ids.len(), 5);
    let steps = document.own_step_count();
    let (layer, top) = document
        .insert_drawing("poster", &nodes, &anchor_ids, &[])
        .unwrap();
    assert_eq!(document.own_step_count(), steps + 1);
    assert!(document.is_layer(layer));
    assert_eq!(
        document.node_registers(layer).unwrap().name.as_deref(),
        Some("poster")
    );
    assert_eq!(top.len(), 2);
    assert_eq!(document.children(Some(layer)), top);
    let group = document.node_registers(top[0]).unwrap();
    assert!(group.locked && group.name.as_deref() == Some("Cut"));
    let members = document.children(Some(top[0]));
    let read = document.read_objects();
    let clone = read.objects.iter().find(|o| o.id() == members[0]).unwrap();
    let ObjectSnapshot::Clone(clone) = clone else {
        panic!()
    };
    assert_eq!(clone.resolved.anchors[0].point, Point::new(30.0, 0.0));
    assert_eq!(clone.resolved.extra_subpaths.len(), 1);
    let path = document.node_registers(members[1]).unwrap();
    assert!(!path.visible && path.opacity.get() == 0.5);
    let ObjectSnapshot::Primitive(ellipse) = document.object(top[1]).unwrap() else {
        panic!()
    };
    assert!((ellipse.rotation.as_radians() - 30_f64.to_radians()).abs() < 1e-9);
    let step = document.own_steps().last().unwrap().clone();
    document.undo_step(step.step).unwrap();
    assert!(!document.contains_node(layer));
}

#[test]
fn bad_drawings_write_nothing() {
    let document = Document::new(1);
    let nodes = drawing();
    let too_few = ids(4);
    assert_eq!(
        document.insert_drawing("x", &nodes, &too_few, &[]),
        Err(ObjectEditError::AnchorIds)
    );
    let clone_of_primitive = vec![
        nodes[1].clone(),
        DrawingNode::Clone {
            registers: DrawingRegisters::default(),
            original: 0,
            matrix: Affine::IDENTITY,
        },
    ];
    assert_eq!(
        document.insert_drawing("x", &clone_of_primitive, &[], &[]),
        Err(ObjectEditError::NoSuchObject)
    );
    let mut deep = square(DrawingRegisters::default());
    for _ in 0..32 {
        deep = DrawingNode::Group {
            registers: DrawingRegisters::default(),
            layer: false,
            children: vec![deep],
        };
    }
    assert_eq!(
        document.insert_drawing("x", &[deep], &ids(5), &[]),
        Err(ObjectEditError::TooDeep)
    );
    assert_eq!(document.own_step_count(), 0);
}

#[test]
fn ac2_ac77_a_document_from_a_drawing_has_no_step() {
    let page = DrawingPage {
        size: DocumentSize::from_mm(100.0, 50.0),
        display_unit: Some(DisplayUnit::In),
        background: None,
        pages: Vec::new(),
    };
    let mut nodes = drawing();
    nodes.insert(
        0,
        DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some("Layer 1".into()),
                ..DrawingRegisters::default()
            },
            layer: true,
            children: vec![],
        },
    );
    let document = Document::from_drawing(4, &page, &nodes, &ids(5), &[]).unwrap();
    assert_eq!(document.own_step_count(), 0);
    assert_eq!(document.size(), page.size);
    assert_eq!(document.display_unit(), DisplayUnit::In);
    let top = document.children(None);
    assert_eq!(top.len(), 3);
    assert!(document.is_layer(top[0]));
    let bytes = curvyo_document_core::pack(&document, "test").unwrap();
    let reopened = curvyo_document_core::unpack(5, &bytes).unwrap();
    assert_eq!(reopened.read_objects().objects.len(), 3);
    // No format change of its own (criterion 77); `0058` took 25 for images, `0059` took 26 for pages, `0060` 27 for snapping, `0066` 28 and 29 for sketches, `0090` 30 for grids.
    assert!((24..=30).contains(&CURRENT_FORMAT_VERSION));
}

fn drawing_text(string: &str, hash: &str) -> curvyo_document_core::DrawingText {
    curvyo_document_core::DrawingText {
        string: string.to_owned(),
        font: curvyo_document_core::FontRef {
            family: "Lobster".into(),
            style: "Bold".into(),
            face_hash: hash.to_owned(),
            face_index: 0,
            kind: curvyo_document_core::FontKind::Outline,
        },
        size: Length::from_mm(5.0),
        layout: curvyo_document_core::TextLayoutValues::DEFAULT,
        chars: curvyo_document_core::TextChars::new([(
            1,
            curvyo_document_core::CharValues {
                kern_x: 0.25,
                kern_y: 0.0,
                rotation: 90.0,
            },
        )]),
        placement: Affine::translation(Vec2::new(10.0, 20.0)),
        style: Style::default(),
    }
}

/// Slice 2, criteria 78, 82, 93, 94, 104: a text with kerns and a missing face, a text on a
/// carrier that becomes a guide, and two images of the same bytes come in with the drawing in its
/// one commit; one undo removes them all, the bytes are one asset.
#[test]
fn ac78_ac94_ac104_texts_and_images_in_the_one_commit() {
    use curvyo_document_core::{MISSING_FACE_HASH, OnPathValues};
    use std::sync::Arc;
    let document = Document::new(1);
    let png: Arc<[u8]> = Arc::from(&b"\x89PNG\r\n\x1a\nbytes"[..]);
    let nodes = vec![
        DrawingNode::Text {
            registers: DrawingRegisters::default(),
            text: drawing_text("AB", MISSING_FACE_HASH),
        },
        square(DrawingRegisters {
            guide: true,
            ..DrawingRegisters::default()
        }),
        DrawingNode::TextOnPath {
            registers: DrawingRegisters::default(),
            text: drawing_text("On path", MISSING_FACE_HASH),
            carrier: 0,
            values: OnPathValues::at(Length::from_mm(12.0)),
        },
        DrawingNode::Image {
            registers: DrawingRegisters {
                name: Some("Image".into()),
                ..DrawingRegisters::default()
            },
            asset: 0,
            placement: Affine::new(50.0, 0.0, 0.0, 25.0, 0.0, 12.5),
        },
        DrawingNode::Image {
            registers: DrawingRegisters::default(),
            asset: 1,
            placement: Affine::new(5.0, 0.0, 0.0, 5.0, 0.0, 0.0),
        },
    ];
    let anchors = ids(DrawingNode::anchor_count(&nodes));
    let steps = document.own_step_count();
    let (layer, top) = document
        .insert_drawing("poster", &nodes, &anchors, &[png.clone(), png.clone()])
        .unwrap();
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(top.len(), 5);
    let text = document.text(top[0]).unwrap();
    assert_eq!(text.string, "AB");
    assert_eq!(text.font.face_hash, MISSING_FACE_HASH);
    assert_eq!(text.chars.get(1).kern_x, 0.25);
    assert!(document.node_registers(top[1]).unwrap().guide);
    let on_path = document.text(top[2]).unwrap().on_path.unwrap();
    assert_eq!(on_path.carrier.node, top[1]);
    assert_eq!(on_path.values.position.as_mm(), 12.0);
    let read = document.read_objects();
    let images: Vec<_> = read
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].asset, images[1].asset);
    assert_eq!(document.asset_bytes(), png.len() as u64);
    assert_eq!(
        document.node_registers(top[3]).unwrap().name.as_deref(),
        Some("Image")
    );
    let step = document.own_steps().last().unwrap().clone();
    document.undo_step(step.step).unwrap();
    assert!(!document.contains_node(layer));
    let mut bad = drawing_text("x", "nothash");
    assert_eq!(
        document.insert_drawing(
            "x",
            &[DrawingNode::Text {
                registers: DrawingRegisters::default(),
                text: bad.clone(),
            }],
            &[],
            &[]
        ),
        Err(ObjectEditError::InvalidText)
    );
    bad.font.face_hash = MISSING_FACE_HASH.into();
    assert_eq!(
        document.insert_drawing(
            "x",
            &[DrawingNode::TextOnPath {
                registers: DrawingRegisters::default(),
                text: bad,
                carrier: 0,
                values: OnPathValues::at(Length::from_mm(0.0)),
            }],
            &[],
            &[]
        ),
        Err(ObjectEditError::NoSuchObject),
        "a carrier must be a drawn leaf"
    );
}

#[test]
fn ac129_a_document_from_a_drawing_has_its_pages_in_order() {
    let frame = |name: &str, x: f64, w: f64| curvyo_document_core::DrawingPageFrame {
        name: name.to_owned(),
        origin: curvyo_document_core::Point::new(x, 20.0),
        size: DocumentSize::from_mm(w, 50.0),
    };
    let page = DrawingPage {
        size: DocumentSize::from_mm(100.0, 50.0),
        display_unit: None,
        background: None,
        pages: vec![frame("Front", -50.0, 100.0), frame("Coaster", 70.0, 80.0)],
    };
    let document = Document::from_drawing(7, &page, &[], &[], &[]).unwrap();
    let pages = document.pages();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].name, "Front");
    assert_eq!(pages[0].origin.x, -50.0);
    assert_eq!(pages[1].name, "Coaster");
    assert_eq!(pages[1].origin.x, 70.0);
    assert_eq!(pages[1].size.width.as_mm(), 80.0);
    assert_eq!(document.own_step_count(), 0);
}
