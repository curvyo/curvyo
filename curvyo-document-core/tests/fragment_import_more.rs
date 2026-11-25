//! More of the copy import (`specs/0137` criteria 4, 5, 8 and 9, ADR 0084 decisions 6 and 7):
//! pictures and fonts travel, a font that is not embedded keeps its reference, a laser page
//! with its placed page comes in as copies, an unknown page kind comes in as a drawing page, the
//! remaps of text carriers, dimensions and tilings, the rule of Weld and a large import's
//! budget.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;

use curvyo_document_core::{
    Affine, AnchorId, Angle, AssetHash, CameFrom, CarrierRef, CellKey, CircleOptions,
    DimensionKind, DimensionPlace, DimensionSlots, Document, DocumentSize, FontKind, FontRef,
    Fragment, FragmentTarget, GridOptions, Length, NewAnchor, NewImage, NewOutputPage, NodeId,
    ObjectSnapshot, OffsetLines, OnPathValues, PageFallback, PageKind, Percent, Point, PointSlot,
    SpacingMode, Style, TextLayoutValues, TilingCell, TilingLayout, TilingRecipe, TilingRequest,
    TilingResult, UnixMillis, Variance, Vec2, pack, unpack,
};

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(10_000);

fn aid(peer: u64) -> AnchorId {
    AnchorId::new(peer, NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}

fn fresh(n: usize) -> Vec<AnchorId> {
    (0..n).map(|_| aid(2)).collect()
}

fn record() -> CameFrom {
    CameFrom {
        document: "Box job".into(),
        page: "Lid".into(),
        page_id: "first".into(),
        item: None,
        at: UnixMillis::new(0),
        online: None,
    }
}

fn square(d: &Document, x: f64, y: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(aid(1), Point::new(x, y)),
            NewAnchor::corner(aid(1), Point::new(x + 10.0, y)),
            NewAnchor::corner(aid(1), Point::new(x + 10.0, y + 10.0)),
            NewAnchor::corner(aid(1), Point::new(x, y + 10.0)),
        ],
        true,
    )
}

fn insert(target: &Document, fragment: &Fragment) -> Vec<NodeId> {
    target
        .insert_fragment(
            fragment,
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::ZERO,
            },
            &record(),
            &fresh(fragment.anchor_count()),
        )
        .unwrap()
        .roots
}

fn png() -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&[7; 32]);
    bytes
}

fn font(hash: &str) -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: hash.to_owned(),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

/// Criterion 8: a picture's bytes and an embedded font's bytes come along by hash.
#[test]
fn pictures_and_embedded_fonts_travel_with_the_copy() {
    let source = Document::new(1);
    let image = source
        .add_images(&[NewImage {
            bytes: Arc::from(png().as_slice()),
            placement: Affine::scale(20.0, 10.0),
            name: "Logo".into(),
        }])
        .unwrap()[0];
    let face = source.insert_asset(b"font bytes of a face").unwrap();
    let text = source
        .create_text(
            "Hallo",
            &font(face.as_str()),
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let target = Document::new(2);
    let fragment = source.extract_fragment(&[image, text], None).unwrap();
    insert(&target, &fragment);
    assert!(
        target.asset(&AssetHash::of(&png())).is_some(),
        "the picture"
    );
    assert!(target.asset(&face).is_some(), "the embedded font");
}

/// Criterion 8: a font that is not embedded keeps its name and reference and draws with the
/// fallback.
#[test]
fn a_font_that_is_not_embedded_keeps_its_reference() {
    let source = Document::new(1);
    let missing = "d".repeat(64);
    let text = source
        .create_text(
            "Hallo",
            &font(&missing),
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let target = Document::new(2);
    let roots = insert(&target, &source.extract_fragment(&[text], None).unwrap());
    let copy = target.text(roots[0]).unwrap();
    assert_eq!(copy.font, font(&missing));
    assert!(target.asset(&AssetHash::parse(&missing).unwrap()).is_none());
}

fn laser_page(d: &Document) -> curvyo_document_core::Page {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
}

/// Criterion 4, ADR 0084 decision 6: a laser page comes in as a laser page; its placed page,
/// whose drawing page is not imported, comes in as copies of the members at the placement's
/// matrix, and no placement is left.
#[test]
fn a_laser_page_comes_in_and_its_placed_page_becomes_copies() {
    let source = Document::new(1);
    let member = square(&source, 10.0, 10.0);
    let laser = laser_page(&source);
    let placement = source
        .place_page(&laser.id, "first", Affine::translation(Vec2::new(5.0, 5.0)))
        .unwrap();
    let fragment = source
        .extract_fragment_placing(&[], Some(&laser.id), &[(placement, vec![member])])
        .unwrap();
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Page,
            &record(),
            &fresh(fragment.anchor_count()),
        )
        .unwrap();
    let page = report.page.unwrap();
    assert!(
        matches!(page.kind, PageKind::Output { .. }),
        "{:?}",
        page.kind
    );
    assert_eq!(report.fallback, None);
    assert!(target.placed_pages().is_empty(), "no placement is left");
    assert_eq!(report.roots.len(), 1, "one copy of the member");
    let Some(ObjectSnapshot::Path(copy)) = target.object(report.roots[0]) else {
        panic!("a path")
    };
    let first = copy.all_anchors().next().unwrap().point;
    // Source: output origin + matrix × (member − drawing origin); then the page's move.
    let expected = Point::new(
        laser.origin.x + 15.0 + (page.origin.x - laser.origin.x),
        laser.origin.y + 15.0 + (page.origin.y - laser.origin.y),
    );
    assert!((first.x - expected.x).abs() < 1e-6 && (first.y - expected.y).abs() < 1e-6);
    // The laser page and its manufacturing open again.
    let reopened = unpack(3, &pack(&target, "t").unwrap()).unwrap();
    assert!(
        reopened
            .pages()
            .iter()
            .any(|p| matches!(p.kind, PageKind::Output { .. }))
    );
}

/// Criterion 4: a page of a kind this build does not know comes in as a drawing page with the
/// reason.
#[test]
fn an_unknown_page_kind_comes_in_as_a_drawing_page() {
    let source = Document::new(1);
    let extra = source.add_page();
    let loro = loro::LoroDoc::new();
    loro.import(&source.export_loro_snapshot().unwrap())
        .unwrap();
    loro.set_peer_id(9).unwrap();
    loro.get_map("root")
        .insert(&format!("page.{}.kind", extra.id), "embroidery")
        .unwrap();
    loro.commit();
    let bytes = pack(&source, "t").unwrap();
    let mut files = {
        use std::io::Read;
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_slice())).unwrap();
        (0..archive.len())
            .map(|i| {
                let mut file = archive.by_index(i).unwrap();
                let mut content = Vec::new();
                file.read_to_end(&mut content).unwrap();
                (file.name().unwrap().to_string(), content)
            })
            .collect::<Vec<_>>()
    };
    for (name, content) in &mut files {
        if name == "document.loro" {
            *content = loro.export(loro::ExportMode::Snapshot).unwrap();
        }
    }
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, content) in &files {
        use std::io::Write;
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    let unknown = unpack(4, &writer.finish().unwrap().into_inner()).unwrap();
    assert_eq!(
        unknown.page_kind(&extra.id),
        Some(PageKind::Unknown("embroidery".into()))
    );
    let fragment = unknown.extract_fragment(&[], Some(&extra.id)).unwrap();
    let target = Document::new(2);
    let report = target
        .insert_fragment(&fragment, FragmentTarget::Page, &record(), &[])
        .unwrap();
    assert_eq!(report.page.unwrap().kind, PageKind::Drawing);
    assert_eq!(
        report.fallback,
        Some(PageFallback::UnknownKind("embroidery".into()))
    );
}

/// ADR 0084 decision 6: a text on a carrier copied with its carrier follows the copy; alone it is
/// released from the path and names no node of the source.
#[test]
fn a_text_on_a_carrier_follows_its_copy_or_keeps_the_outline() {
    let source = Document::new(1);
    let carrier = square(&source, 0.0, 0.0);
    let text = source
        .create_text(
            "On a path",
            &font(&"c".repeat(64)),
            Length::from_mm(5.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    source
        .put_text_on_path(
            text,
            CarrierRef {
                node: carrier,
                outline: 0,
            },
            &OnPathValues::at(Length::from_mm(0.0)),
            false,
        )
        .unwrap();
    let both = Document::new(2);
    let roots = insert(
        &both,
        &source.extract_fragment(&[carrier, text], None).unwrap(),
    );
    let on = both.text(roots[1]).unwrap().on_path.unwrap();
    assert_eq!(on.carrier.node, roots[0]);
    let alone = Document::new(3);
    let fragment = source.extract_fragment(&[text], None).unwrap();
    assert_eq!(fragment.texts_released(), 1, "counted for the notice");
    let roots = insert(&alone, &fragment);
    let copy = alone.text(roots[0]).unwrap();
    assert!(
        copy.on_path.is_none(),
        "no link into the source: {:?}",
        copy.on_path
    );
    assert_eq!(
        copy.placement,
        Affine::IDENTITY,
        "the straight placement is kept"
    );
}

/// ADR 0084 decision 6: a dimension copied with its path measures the copy; alone its points
/// become free.
#[test]
fn a_dimension_follows_its_copied_path_or_becomes_free() {
    let source = Document::new(1);
    let a = aid(1);
    let b = aid(1);
    let line = source.create_path(
        &[
            NewAnchor::corner(a, Point::new(0.0, 0.0)),
            NewAnchor::corner(b, Point::new(30.0, 40.0)),
        ],
        false,
    );
    let dimension = source
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Anchor {
                    node: line,
                    anchor: a,
                },
                PointSlot::Anchor {
                    node: line,
                    anchor: b,
                },
            ]),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap();
    let both = Document::new(2);
    let roots = insert(
        &both,
        &source.extract_fragment(&[line, dimension], None).unwrap(),
    );
    let copy = both.dimension(roots[1]).unwrap();
    let DimensionSlots::Points(points) = &copy.slots else {
        panic!("points")
    };
    assert!(
        points
            .iter()
            .all(|slot| matches!(slot, PointSlot::Anchor { node, .. } if *node == roots[0]))
    );
    assert_eq!(copy.shown.points.len(), 2, "{:?}", copy.shown);
    let alone = Document::new(3);
    let roots = insert(
        &alone,
        &source.extract_fragment(&[dimension], None).unwrap(),
    );
    let copy = alone.dimension(roots[0]).unwrap();
    let DimensionSlots::Points(points) = &copy.slots else {
        panic!("points")
    };
    assert!(
        points
            .iter()
            .all(|slot| !matches!(slot, PointSlot::Anchor { .. })),
        "free points"
    );
}

fn recipe() -> TilingRecipe {
    TilingRecipe {
        layout: TilingLayout::Grid,
        grid: GridOptions {
            columns: 2,
            rows: 1,
            spacing_x: Length::from_mm(2.0),
            spacing_y: Length::from_mm(2.0),
            spacing_mode: SpacingMode::Gap,
            mirror_rows: false,
            mirror_columns: false,
            offset: OffsetLines::Rows,
            shift: Percent::new(0.0),
        },
        circle: CircleOptions {
            count: 2,
            centre: Vec2::new(0.0, 0.0),
            radius: Length::from_mm(10.0),
            start: Angle::from_radians(0.0),
            sweep: Angle::from_radians(std::f64::consts::TAU),
            face_centre: true,
        },
        variance: Variance::default(),
        seed: 1,
        result: TilingResult::Clones,
        keep_source: true,
    }
}

/// ADR 0084 decision 6: a tiling comes in as a tiling of its own copied sources.
#[test]
fn a_tiling_comes_in_with_its_sources_remapped() {
    let source = Document::new(1);
    let tile = square(&source, 0.0, 0.0);
    let next = Cell::new(50_000_u64);
    let mut mint = || {
        next.set(next.get() + 1);
        AnchorId::new(1, next.get())
    };
    let tiling = source
        .apply_tiling(
            &TilingRequest {
                tiling: None,
                sources: vec![tile],
                recipe: recipe(),
                cells: vec![TilingCell {
                    key: CellKey { row: 0, column: 1 },
                    affine: Affine::translation(Vec2::new(12.0, 0.0)),
                    opacity: Percent::new(0.0),
                    hue: Angle::from_radians(0.0),
                }],
                conversions: Vec::new(),
            },
            &mut mint,
        )
        .unwrap()
        .tiling;
    let target = Document::new(2);
    let roots = insert(&target, &source.extract_fragment(&[tiling], None).unwrap());
    let stored = target.tiling(roots[0]).expect("still a tiling");
    let children = target.children(Some(roots[0]));
    assert!(
        stored.sources.iter().all(|s| children.contains(s)),
        "{stored:?}"
    );
    let _ = Style::default();
}

/// ADR 0084 §7: Weld keeps Came from only when every operand carries an equal one.
#[test]
fn weld_keeps_an_equal_record_only() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let b = square(&source, 5.0, 5.0);
    let target = Document::new(2);
    let roots = insert(&target, &source.extract_fragment(&[a, b], None).unwrap());
    let outline = vec![(
        vec![
            NewAnchor::corner(aid(3), Point::new(0.0, 0.0)),
            NewAnchor::corner(aid(3), Point::new(15.0, 0.0)),
            NewAnchor::corner(aid(3), Point::new(15.0, 15.0)),
        ],
        true,
    )];
    let welded = target
        .replace_with_path_at(&roots, &Style::default(), roots[0], &outline, "weld")
        .unwrap();
    assert_eq!(target.came_from(welded), Some(record()));
}

fn thread_cpu() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
        .map_or_else(
            || {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                START.get_or_init(std::time::Instant::now).elapsed()
            },
            Duration::from_nanos,
        )
}

/// A large import (**Proposal**, plan task 6): 500 objects out of a source of 2,000 are taken
/// and inserted within 3 s of thread CPU time in a test build.
#[test]
fn a_large_import_stays_within_its_budget() {
    let source = Document::new(1);
    let ids: Vec<NodeId> = (0..2000)
        .map(|i| square(&source, f64::from(i % 50) * 12.0, f64::from(i / 50) * 12.0))
        .collect();
    let target = Document::new(2);
    let start = thread_cpu();
    let fragment = source.extract_fragment(&ids[..500], None).unwrap();
    let roots = insert(&target, &fragment);
    let took = thread_cpu().checked_sub(start).unwrap();
    assert_eq!(roots.len(), 500);
    assert!(took <= Duration::from_secs(3), "{took:?}");
}
