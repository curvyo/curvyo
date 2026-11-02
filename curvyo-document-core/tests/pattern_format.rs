//! The stored pattern fill (`specs/0071-pattern-fill` criteria 1, 3, 17, 20, 21; ADR 0029 §1, §2,
//! §5, §12): the style edits, the fresh seed, copies and clones, format 32 and the golden
//! `pattern_v32.curvyo`. Regenerate the golden with `CURVYO_WRITE_FIXTURES=1`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::needless_pass_by_value,
    clippy::cast_precision_loss
)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CopySource, Document, FillPaint, GeneratorKind, Length,
    NewAnchor, NodeId, PatternFill, PatternGenerator, PatternMode, PatternParam, PatternSeed,
    Point, RectBounds, StyleEdit, StyleEditError, Vec2, pack, unpack,
};

const PATTERN_FORMAT_VERSION: u32 = 32;
const GOLDEN: &str = "pattern_v32.curvyo";

const _: () = assert!(CURRENT_FORMAT_VERSION >= PATTERN_FORMAT_VERSION);

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn manifest_version(container: &[u8]) -> u64 {
    let mut archive = zip::ZipArchive::new(Cursor::new(container)).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    manifest["format_version"].as_u64().unwrap()
}

fn square(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(100.0),
        height: Length::from_mm(100.0),
    })
}

/// A square path with a square hole: a compound region.
fn holed(document: &Document, first_anchor: u64) -> NodeId {
    let ring = |n: u64, pts: &[(f64, f64)]| -> (Vec<NewAnchor>, bool) {
        let anchors = pts
            .iter()
            .zip(n..)
            .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), Point::new(*x, *y)))
            .collect();
        (anchors, true)
    };
    let base = document.create_path(
        &ring(first_anchor, &[(0.0, 120.0), (5.0, 120.0), (5.0, 125.0)]).0,
        true,
    );
    let outlines = [
        ring(
            first_anchor + 10,
            &[(0.0, 120.0), (60.0, 120.0), (60.0, 180.0), (0.0, 180.0)],
        ),
        ring(
            first_anchor + 20,
            &[(20.0, 140.0), (20.0, 160.0), (40.0, 160.0), (40.0, 140.0)],
        ),
    ];
    document
        .replace_with_path(&[base], base, &outlines, "boolean_difference")
        .unwrap()
}

fn changes_of(document: &Document) -> usize {
    let loro = loro::LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro.len_changes()
}

fn fill_of(document: &Document, id: NodeId) -> curvyo_document_core::Fill {
    document.object(id).unwrap().style().fill.clone()
}

fn pattern_of(document: &Document, id: NodeId) -> PatternFill {
    *fill_of(document, id).pattern.unwrap()
}

fn edit(document: &Document, id: NodeId, edit: StyleEdit) {
    document.edit_style(&[id], &edit).unwrap();
}

#[test]
fn choosing_pattern_gives_hatch_defaults_and_the_shapes_fresh_seed() {
    let document = Document::new(1);
    let a = square(&document, 0.0);
    let b = square(&document, 200.0);
    document
        .edit_style(&[a, b], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let fill = fill_of(&document, a);
    assert!(fill.enabled);
    assert_eq!(fill.paint, FillPaint::Pattern);
    let pattern = *fill.pattern.unwrap();
    assert_eq!(
        pattern.generator,
        PatternGenerator::defaults(GeneratorKind::Hatch)
    );
    assert_eq!(pattern.seed, PatternSeed::fresh(a));
    assert_ne!(
        pattern_of(&document, b).seed,
        pattern.seed,
        "each shape its own seed"
    );
}

#[test]
fn switching_to_solid_or_none_keeps_the_pattern_for_later() {
    let document = Document::new(1);
    let a = square(&document, 0.0);
    edit(&document, a, StyleEdit::FillPaint(FillPaint::Pattern));
    edit(
        &document,
        a,
        StyleEdit::PatternGenerator(GeneratorKind::Dots),
    );
    edit(
        &document,
        a,
        StyleEdit::PatternParam(PatternParam::Diameter, 1.5),
    );
    let stored = pattern_of(&document, a);
    edit(&document, a, StyleEdit::FillPaint(FillPaint::Colour));
    assert!(fill_of(&document, a).painted_pattern().is_none());
    edit(&document, a, StyleEdit::FillEnabled(false));
    edit(&document, a, StyleEdit::FillPaint(FillPaint::Pattern));
    assert_eq!(fill_of(&document, a).painted_pattern(), Some(stored));
}

#[test]
fn each_pattern_edit_is_one_commit_and_a_bad_value_writes_nothing() {
    let document = Document::new(1);
    let a = square(&document, 0.0);
    edit(&document, a, StyleEdit::FillPaint(FillPaint::Pattern));
    let changes = changes_of(&document);
    edit(
        &document,
        a,
        StyleEdit::PatternParam(PatternParam::Spacing, 3.0),
    );
    assert_eq!(changes_of(&document), changes + 1);
    edit(
        &document,
        a,
        StyleEdit::PatternParam(PatternParam::Spacing, 3.0),
    );
    assert_eq!(
        changes_of(&document),
        changes + 1,
        "unchanged writes nothing"
    );
    assert!(matches!(
        document.edit_style(&[a], &StyleEdit::PatternParam(PatternParam::Spacing, 0.0)),
        Err(StyleEditError::InvalidPattern(_))
    ));
    assert!(matches!(
        document.edit_style(&[a], &StyleEdit::PatternMargin(Length::from_mm(-1.0))),
        Err(StyleEditError::InvalidMargin)
    ));
    assert_eq!(changes_of(&document), changes + 1);
    let seed = pattern_of(&document, a).seed;
    edit(&document, a, StyleEdit::PatternReroll);
    assert_eq!(pattern_of(&document, a).seed, seed.next());
    edit(&document, a, StyleEdit::PatternMode(PatternMode::Shapes));
    edit(&document, a, StyleEdit::PatternMargin(Length::from_mm(2.0)));
    let pattern = pattern_of(&document, a);
    assert_eq!(pattern.mode, PatternMode::Shapes);
    assert_eq!(pattern.margin, Length::from_mm(2.0));
    assert!(!pattern.whole_pieces, "off by default");
}

/// Criterion 29: Whole pieces only is one commit of its own name, kept across generators.
#[test]
fn whole_pieces_only_is_one_named_commit_and_stays_with_the_pattern() {
    let document = Document::new(1);
    let a = square(&document, 0.0);
    edit(&document, a, StyleEdit::FillPaint(FillPaint::Pattern));
    edit(
        &document,
        a,
        StyleEdit::PatternGenerator(GeneratorKind::Honeycomb),
    );
    let changes = changes_of(&document);
    let on = StyleEdit::PatternWholePieces(true);
    assert_eq!(on.commit_label(), "pattern_whole_pieces");
    assert!(on.is_pattern_edit());
    edit(&document, a, on.clone());
    assert_eq!(changes_of(&document), changes + 1);
    edit(&document, a, on);
    assert_eq!(
        changes_of(&document),
        changes + 1,
        "unchanged writes nothing"
    );
    assert!(pattern_of(&document, a).whole_pieces);
    edit(
        &document,
        a,
        StyleEdit::PatternGenerator(GeneratorKind::Hatch),
    );
    edit(
        &document,
        a,
        StyleEdit::PatternGenerator(GeneratorKind::Dots),
    );
    assert!(pattern_of(&document, a).whole_pieces, "kept while hidden");
    let reopened = unpack(3, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert!(pattern_of(&reopened, a).whole_pieces);
}

/// Criterion 20: a copy carries the same pattern and seed; editing it leaves the original.
#[test]
fn a_duplicate_keeps_the_pattern_and_the_seed() {
    let document = Document::new(1);
    let a = square(&document, 0.0);
    edit(&document, a, StyleEdit::FillPaint(FillPaint::Pattern));
    edit(
        &document,
        a,
        StyleEdit::PatternGenerator(GeneratorKind::LeafForest),
    );
    let copy = document
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap()[0];
    assert_eq!(fill_of(&document, copy), fill_of(&document, a));
    edit(&document, copy, StyleEdit::PatternReroll);
    assert_ne!(
        pattern_of(&document, copy).seed,
        pattern_of(&document, a).seed
    );
}

/// Criterion 21: a linked clone reads its source's pattern with the rest of its style.
#[test]
fn a_linked_clone_shows_its_sources_pattern() {
    let document = Document::new(1);
    let a = holed(&document, 1);
    edit(&document, a, StyleEdit::FillPaint(FillPaint::Pattern));
    let clone = document.linked_clone(&[a], &[]).unwrap()[0];
    assert_eq!(fill_of(&document, clone), fill_of(&document, a));
    edit(
        &document,
        a,
        StyleEdit::PatternGenerator(GeneratorKind::Grid),
    );
    assert_eq!(
        fill_of(&document, clone).pattern.unwrap().generator.kind(),
        GeneratorKind::Grid
    );
}

/// Every pattern of the golden in one step, so the file's history has no two adjacent rows with
/// the same label (`specs/0020-undo-redo` criterion 36's check over the fixtures).
fn golden_document() -> Document {
    let document = Document::new(1);
    let squares: Vec<NodeId> = document.batch("create_rect", |document| {
        (0..GeneratorKind::ALL.len())
            .map(|i| square(document, 120.0 * i as f64))
            .collect()
    });
    let compound = holed(&document, 1);
    document.batch("edit_style", |document| {
        for (i, (kind, id)) in GeneratorKind::ALL.into_iter().zip(&squares).enumerate() {
            edit(document, *id, StyleEdit::FillPaint(FillPaint::Pattern));
            edit(document, *id, StyleEdit::PatternGenerator(kind));
            if kind.has_whole_pieces() && i % 3 != 2 {
                edit(document, *id, StyleEdit::PatternWholePieces(true));
            }
            if i % 2 == 1 {
                edit(document, *id, StyleEdit::PatternMode(PatternMode::Shapes));
                edit(
                    document,
                    *id,
                    StyleEdit::PatternMargin(Length::from_mm(1.5)),
                );
            }
        }
        edit(document, compound, StyleEdit::FillPaint(FillPaint::Pattern));
        edit(
            document,
            compound,
            StyleEdit::PatternMargin(Length::from_mm(2.0)),
        );
    });
    document
}

fn patterns(document: &Document) -> Vec<Option<PatternFill>> {
    document
        .object_ids()
        .into_iter()
        .map(|id| fill_of(document, id).painted_pattern())
        .collect()
}

#[test]
fn the_golden_is_version_32_and_reads_back_every_generator() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
    }
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), u64::from(PATTERN_FORMAT_VERSION));
    let document = unpack(2, &bytes).unwrap();
    let read = patterns(&document);
    assert_eq!(read, patterns(&golden_document()));
    let kinds: Vec<GeneratorKind> = read
        .iter()
        .take(10)
        .map(|p| p.unwrap().generator.kind())
        .collect();
    assert_eq!(kinds, GeneratorKind::ALL);
    assert!(read.iter().flatten().any(|p| p.whole_pieces));
    assert!(read.iter().flatten().any(|p| !p.whole_pieces));
}

#[test]
fn save_and_reopen_keeps_every_pattern_and_seed() {
    let document = golden_document();
    let reopened = unpack(3, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert_eq!(patterns(&reopened), patterns(&document));
}

#[test]
fn a_file_without_patterns_opens_as_before() {
    let document = Document::new(1);
    let a = square(&document, 0.0);
    edit(&document, a, StyleEdit::FillEnabled(true));
    let reopened = unpack(3, &pack(&document, "0.1.0").unwrap()).unwrap();
    let fill = fill_of(&reopened, reopened.object_ids()[0]);
    assert!(fill.paints_solid());
    assert_eq!(fill.pattern, None);
}
