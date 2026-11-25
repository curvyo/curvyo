//! Second acceptance round of `specs/0137-document-library` at the document model: a placed page
//! whose drawing page is not imported comes in as copies (criterion 4), an unpicked linked
//! clone of a rectangle stays a rectangle (criterion 5), a font not installed keeps its name
//! (criterion 8), and Weld keeps Came from only for equal records (criterion 9). Written from the
//! spec; the helpers follow the first round's file.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{
    Affine, AnchorId, CameFrom, Document, DocumentSize, FontKind, FontRef, FragmentTarget, Length,
    NewAnchor, NewOutputPage, NodeId, ObjectSnapshot, PageKind, Point, RectBounds, Style,
    TextLayoutValues, UnixMillis, Vec2, pack, unpack,
};

fn rec(document: &str, item: Option<&str>) -> CameFrom {
    CameFrom {
        document: document.to_owned(),
        page: "Page 1".to_owned(),
        page_id: "first".to_owned(),
        item: item.map(str::to_owned),
        at: UnixMillis::new(1_700_000_000_000),
        online: None,
    }
}

fn square(d: &Document, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + 10.0, y + 10.0),
    ))
}

fn ids(n: usize, seed: u64) -> Vec<AnchorId> {
    (0..n)
        .map(|i| AnchorId::new(4137, seed * 100 + i as u64))
        .collect()
}

fn font(family: &str) -> FontRef {
    FontRef {
        family: family.to_owned(),
        style: "Regular".to_owned(),
        face_hash: "d".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

/// Criterion 4 (placed page): a laser page whose drawing page is not imported comes in as an
/// output page with copies of the placed members, and no placement is left in the target.
#[test]
fn ac4_placed_page_of_a_laser_page_imports_as_copies_with_came_from() {
    let source = Document::new(1);
    let member = square(&source, 10.0, 10.0);
    let laser = source
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let placement = source
        .place_page(&laser.id, "first", Affine::translation(Vec2::new(5.0, 5.0)))
        .unwrap();
    let fragment = source
        .extract_fragment_placing(&[], Some(&laser.id), &[(placement, vec![member])])
        .unwrap();
    let target = Document::new(2);
    let came = rec("Box job", None);
    let report = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Page,
            &came,
            &ids(fragment.anchor_count(), 1),
        )
        .unwrap();
    let page = report.page.clone().unwrap();
    assert!(
        matches!(page.kind, PageKind::Output { .. }),
        "the laser page keeps its kind: {:?}",
        page.kind
    );
    assert!(
        target.placed_pages().is_empty(),
        "no placement is left behind"
    );
    assert_eq!(report.roots.len(), 1, "one copy of the placed member");
    assert!(
        target.object(report.roots[0]).is_some(),
        "the copy is an ordinary object"
    );
    assert_eq!(
        target.came_from(report.roots[0]).map(|c| c.document),
        Some("Box job".to_owned()),
        "the copy carries Came from"
    );
    assert_eq!(
        target.page_came_from(&page.id).map(|c| c.page_id),
        Some("first".to_owned())
    );
}

/// Criterion 5: a linked clone whose original was not picked comes in as an unlinked object of
/// its own kind: a rectangle stays a rectangle, and the count is kept for the notice.
#[test]
fn ac5_an_unpicked_rectangle_clone_comes_in_as_a_rectangle() {
    let source = Document::new(1);
    let original = square(&source, 0.0, 0.0);
    let clone = source
        .place_linked_clones(&[original], Vec2::new(30.0, 0.0))
        .unwrap()[0];
    let fragment = source.extract_fragment(&[clone], None).unwrap();
    assert_eq!(fragment.clones_became_paths(), 1);
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::new(0.0, 0.0),
            },
            &rec("Box job", None),
            &ids(fragment.anchor_count(), 2),
        )
        .unwrap();
    match target.object(report.roots[0]) {
        Some(ObjectSnapshot::Primitive(_)) => {}
        other => panic!("a rectangle clone must stay a rectangle, got {other:?}"),
    }
}

/// Criterion 8: a font that is not installed on this machine keeps its name in the copy.
#[test]
fn ac8_a_text_in_a_font_that_is_not_installed_keeps_its_name() {
    let source = Document::new(1);
    let text = source
        .create_text(
            "Hallo",
            &font("Nonexistent Sans 137"),
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let fragment = source.extract_fragment(&[text], None).unwrap();
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::new(0.0, 0.0),
            },
            &rec("Box job", None),
            &ids(fragment.anchor_count(), 3),
        )
        .unwrap();
    let copy = target.text(report.roots[0]).unwrap();
    assert_eq!(copy.font.family, "Nonexistent Sans 137");
    assert_eq!(copy.string, "Hallo");
}

/// Criterion 9: Weld keeps Came from only when every source carried the same record.
#[test]
fn ac9_weld_of_sources_with_different_records_drops_came_from() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let b = square(&source, 5.0, 5.0);
    let target = Document::new(2);
    let ra = target
        .insert_fragment(
            &source.extract_fragment(&[a], None).unwrap(),
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::new(0.0, 0.0),
            },
            &rec("Box job", None),
            &ids(
                source.extract_fragment(&[a], None).unwrap().anchor_count(),
                4,
            ),
        )
        .unwrap()
        .roots;
    let rb = target
        .insert_fragment(
            &source.extract_fragment(&[b], None).unwrap(),
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::new(0.0, 0.0),
            },
            &rec("Logos", None),
            &ids(
                source.extract_fragment(&[b], None).unwrap().anchor_count(),
                5,
            ),
        )
        .unwrap()
        .roots;
    let roots = [ra[0], rb[0]];
    let outline = vec![(
        vec![
            NewAnchor::corner(AnchorId::new(4137, 900), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(4137, 901), Point::new(15.0, 0.0)),
            NewAnchor::corner(AnchorId::new(4137, 902), Point::new(15.0, 15.0)),
        ],
        true,
    )];
    let welded = target
        .replace_with_path_at(&roots, &Style::default(), roots[0], &outline, "weld")
        .unwrap();
    assert_eq!(
        target.came_from(welded),
        None,
        "sources with different records keep none"
    );
}

/// Criterion 6 and 9: a copy's Came from is plain data that survives save and reopen; the copy
/// takes its own name as the item (an unnamed rectangle has none), whatever record was passed.
#[test]
fn ac9_a_record_survives_save_and_reopen_on_a_copied_object() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let target = Document::new(2);
    let roots = target
        .insert_fragment(
            &source.extract_fragment(&[a], None).unwrap(),
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::new(0.0, 0.0),
            },
            &rec("Box job", Some("passed")),
            &ids(
                source.extract_fragment(&[a], None).unwrap().anchor_count(),
                6,
            ),
        )
        .unwrap()
        .roots;
    let back = unpack(3, &pack(&target, "t").unwrap()).unwrap();
    let came = back.came_from(roots[0]).expect("the record survives");
    assert_eq!(came.document, "Box job");
    assert_eq!(came.item, None, "an unnamed rectangle has no item name");
}
