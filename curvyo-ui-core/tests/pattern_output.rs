//! A pattern fill in the outputs (`specs/0071-pattern-fill` criteria 24 to 26): the SVG export
//! writes the region without a fill and the pieces as plain paths, and Convert to paths makes the
//! same geometry as ordinary paths in a group above the shape.

#![allow(clippy::unwrap_used, clippy::float_cmp)]

use curvyo_document_core::{
    Document, FillPaint, GeneratorKind, Length, NodeId, ObjectSnapshot, PatternMode, Point,
    RectBounds, StyleEdit,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{
    AnchorIdMinter, ExportScope, ImportFaces, SvgExportOptions, object_pattern_pieces,
    pattern_convert_plan, svg_export,
};

fn patterned(kind: GeneratorKind, mode: PatternMode) -> (Document, NodeId) {
    let document = Document::new(1);
    let id = document.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(30.0),
    });
    for edit in [
        StyleEdit::FillPaint(FillPaint::Pattern),
        StyleEdit::PatternGenerator(kind),
        StyleEdit::PatternMode(mode),
    ] {
        document.edit_style(&[id], &edit).unwrap();
    }
    (document, id)
}

fn export(document: &Document) -> String {
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let outcome = svg_export(
        document,
        &document.read_objects(),
        &fonts,
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    );
    outcome.text()
}

/// Criterion 26: real paths, no `pattern` element, no generator name.
#[test]
fn ac26_the_svg_export_writes_the_pieces_as_paths() {
    for kind in GeneratorKind::ALL {
        let (document, id) = patterned(kind, PatternMode::Lines);
        let pieces = object_pattern_pieces(&document.object(id).unwrap())
            .unwrap()
            .unwrap();
        let svg = export(&document);
        assert!(!svg.contains("<pattern"), "{kind:?}");
        assert!(!svg.contains(kind.id()), "{kind:?} names no generator");
        let paths = svg.matches("<path").count();
        assert!(
            paths >= pieces.len(),
            "{kind:?}: {paths} paths for {} pieces",
            pieces.len()
        );
        assert!(svg.contains("fill=\"none\""), "the region has no fill");
    }
}

/// Criterion 17: the export of the same document is byte-equal run after run.
#[test]
fn ac17_the_export_is_byte_equal() {
    let (document, _) = patterned(GeneratorKind::LeafForest, PatternMode::Shapes);
    assert_eq!(export(&document), export(&document));
}

/// Criteria 24, 25: Convert gives one path per piece in a group above the shape, the shape's fill
/// off, the geometry equal to the live pattern's; one commit, undone by the history.
#[test]
fn ac24_ac25_convert_makes_the_same_pieces_as_paths() {
    for mode in [PatternMode::Lines, PatternMode::Shapes] {
        let (document, id) = patterned(GeneratorKind::Dots, mode);
        let object = document.object(id).unwrap();
        let live = object_pattern_pieces(&object).unwrap().unwrap();
        let objects = [object];
        let mut minter = AnchorIdMinter::new(77);
        let plan = pattern_convert_plan(&objects, &[id], &mut minter);
        assert_eq!(plan.len(), 1);
        let groups = document.convert_patterns(&plan).unwrap();
        assert_eq!(groups.len(), 1);
        let leaves = document.leaves_of(groups[0]);
        assert_eq!(leaves.len(), live.len());
        for (leaf, piece) in leaves.iter().zip(&live) {
            let ObjectSnapshot::Path(path) = document.object(*leaf).unwrap() else {
                panic!("a path");
            };
            let points: Vec<Point> = path.anchors.iter().map(|a| a.point).collect();
            let expected: Vec<Point> = piece.outlines[0].0.iter().map(|a| a.0).collect();
            assert_eq!(points, expected);
            assert_eq!(path.style.fill.enabled, mode == PatternMode::Shapes);
        }
        let shape = document.object(id).unwrap();
        assert!(!shape.style().fill.enabled, "the shape keeps no fill");
        assert!(object_pattern_pieces(&shape).is_none());
    }
}

/// Criterion 23: an open path with a pattern has nothing to convert.
#[test]
fn ac23_an_open_shape_has_nothing_to_convert() {
    let document = Document::new(1);
    let id = document.create_path(
        &[
            curvyo_document_core::NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 1),
                Point::new(0.0, 0.0),
            ),
            curvyo_document_core::NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 2),
                Point::new(20.0, 0.0),
            ),
            curvyo_document_core::NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 3),
                Point::new(20.0, 20.0),
            ),
        ],
        false,
    );
    document
        .edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let objects = [document.object(id).unwrap()];
    assert_eq!(object_pattern_pieces(&objects[0]), Some(Ok(Vec::new())));
    assert_eq!(
        pattern_convert_plan(&objects, &[id], &mut AnchorIdMinter::new(1)).len(),
        0
    );
}

/// ADR 0029, Consequences: the SVG export of every generator, compared with
/// `fixtures/pattern/pattern_export.svg` (regenerate with `CURVYO_UPDATE_GOLDEN=1`).
#[test]
fn ac26_the_export_of_every_generator_matches_its_golden() {
    let document = Document::new(1);
    for (i, kind) in GeneratorKind::ALL.into_iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let x = 10.0 + 45.0 * (i % 5) as f64;
        let y = if i < 5 { 10.0 } else { 60.0 };
        let id = document.create_rect(RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(40.0),
            height: Length::from_mm(40.0),
        });
        for edit in [
            StyleEdit::FillPaint(FillPaint::Pattern),
            StyleEdit::PatternGenerator(kind),
            StyleEdit::PatternMode(if i % 2 == 0 {
                PatternMode::Lines
            } else {
                PatternMode::Shapes
            }),
        ] {
            document.edit_style(&[id], &edit).unwrap();
        }
    }
    let svg = export(&document);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pattern/pattern_export.svg");
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &svg).unwrap();
    }
    let golden = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(golden == svg, "the export differs from {}", path.display());
}

/// UX notes §6: every pattern edit commits under a label the History tab names.
#[test]
fn every_pattern_edit_has_a_history_name() {
    use curvyo_document_core::{PatternMode, PatternParam, PatternSeed, pattern_params};
    let mut edits = vec![
        StyleEdit::FillPaint(FillPaint::Pattern),
        StyleEdit::PatternMargin(Length::from_mm(1.0)),
        StyleEdit::PatternMode(PatternMode::Lines),
        StyleEdit::PatternMode(PatternMode::Shapes),
        StyleEdit::PatternSeed(PatternSeed::new(1)),
        StyleEdit::PatternReroll,
        StyleEdit::PatternWholePieces(true),
    ];
    for kind in GeneratorKind::ALL {
        edits.push(StyleEdit::PatternGenerator(kind));
        edits.extend(
            pattern_params(kind)
                .iter()
                .map(|spec| StyleEdit::PatternParam(spec.param, spec.default)),
        );
    }
    for edit in &edits {
        let name = curvyo_ui_core::known_operation_name(edit.commit_label());
        assert!(name.is_some(), "{edit:?} has no history name");
        assert_ne!(name, Some("Change style"), "{edit:?}");
    }
    assert_eq!(
        curvyo_ui_core::known_operation_name(
            StyleEdit::PatternParam(PatternParam::Spacing, 2.0).commit_label()
        ),
        Some("Pattern spacing")
    );
    assert_eq!(
        curvyo_ui_core::known_operation_name(
            StyleEdit::PatternGenerator(GeneratorKind::Honeycomb).commit_label()
        ),
        Some("Pattern: Honeycomb")
    );
    assert_eq!(
        curvyo_ui_core::known_operation_name(StyleEdit::PatternWholePieces(false).commit_label()),
        Some("Pattern: whole pieces")
    );
}

/// Criterion 29: with whole pieces only, the export and Convert to paths carry exactly the whole
/// pieces the canvas draws: every honeycomb cell whole (six corners), fewer than when cut.
#[test]
fn ac29_whole_pieces_reach_the_export_and_convert_unchanged() {
    let (document, id) = patterned(GeneratorKind::Honeycomb, PatternMode::Shapes);
    let cut = object_pattern_pieces(&document.object(id).unwrap())
        .unwrap()
        .unwrap();
    let cut_svg = export(&document);
    document
        .edit_style(&[id], &StyleEdit::PatternWholePieces(true))
        .unwrap();
    let object = document.object(id).unwrap();
    let live = object_pattern_pieces(&object).unwrap().unwrap();
    assert!(!live.is_empty() && live.len() < cut.len());
    assert!(
        live.iter()
            .all(|p| p.outlines.len() == 1 && p.outlines[0].0.len() == 6)
    );
    let svg = export(&document);
    assert_eq!(svg, export(&document), "byte-equal");
    assert_eq!(
        cut_svg.matches("<path").count() - svg.matches("<path").count(),
        cut.len() - live.len(),
        "the export leaves out the same pieces"
    );
    let objects = [object];
    let mut minter = AnchorIdMinter::new(78);
    let plan = pattern_convert_plan(&objects, &[id], &mut minter);
    let groups = document.convert_patterns(&plan).unwrap();
    let leaves = document.leaves_of(groups[0]);
    assert_eq!(leaves.len(), live.len());
    for (leaf, piece) in leaves.iter().zip(&live) {
        let ObjectSnapshot::Path(path) = document.object(*leaf).unwrap() else {
            panic!("a path");
        };
        let points: Vec<Point> = path.anchors.iter().map(|a| a.point).collect();
        let expected: Vec<Point> = piece.outlines[0].0.iter().map(|a| a.0).collect();
        assert_eq!(points, expected);
    }
}

/// Architecture review change 3 (ADR 0029 §7): the pieces are written before the outline, as the
/// canvas draws them below its stroke; on a laser or plotter document order is job order, so the
/// engraving comes before the outline is cut.
#[test]
fn ac25_the_export_writes_the_pieces_before_the_outline() {
    let document = Document::new(1);
    let id = document.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(40.0),
    });
    document
        .edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let svg = export(&document);
    let pieces = svg.find("<path id=\"path1\"").unwrap();
    let outline = svg.find("<rect id=\"rect1\"").unwrap();
    assert!(pieces < outline, "{svg}");
}
