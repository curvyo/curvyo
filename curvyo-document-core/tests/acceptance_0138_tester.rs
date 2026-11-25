//! Acceptance tests of `specs/0138-template-library` at the document model: the online keys of the
//! Came from record (criterion 3), the credits derived from the objects (criterion 4) and the
//! one-step Load of a template (criterion 3). Written from the specification before the
//! implementation was read; black box through the public API only.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{
    AnchorId, CameFrom, Document, FIRST_PAGE_ID, FragmentTarget, OnlineOrigin, Point, RectBounds,
    UnixMillis, Vec2,
};

fn online(credit: Option<&str>) -> CameFrom {
    CameFrom {
        document: "Curvyo catalogue".to_owned(),
        page: "Name plate".to_owned(),
        page_id: FIRST_PAGE_ID.to_owned(),
        item: None,
        at: UnixMillis::new(1_700_000_000_000),
        online: Some(OnlineOrigin {
            address:
                "https://raw.githubusercontent.com/curvyo/templates/main/signs/name-plate.curvyo"
                    .to_owned(),
            spdx: "CC-BY-4.0".to_owned(),
            licence: "CC BY 4.0".to_owned(),
            licence_url: "https://creativecommons.org/licenses/by/4.0/".to_owned(),
            author: "Max Mustermann".to_owned(),
            credit: credit.map(str::to_owned),
        }),
    }
}

/// A source with one rectangle, copied into a fresh target through the element import.
fn import_one(target: &Document, credit: Option<&str>) -> Vec<curvyo_document_core::NodeId> {
    let src = Document::new(1);
    let a = src.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
    ));
    let frag = src.extract_fragment(&[a], Some(FIRST_PAGE_ID)).unwrap();
    let anchors: Vec<AnchorId> = (0..frag.anchor_count())
        .map(|i| AnchorId::new(9138, 1000 + i as u64))
        .collect();
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &online(credit),
            &anchors,
        )
        .expect("the copy is inserted");
    report.roots
}

// Criterion 4: while an object with a credit is in the document, its credit is listed in the
// Credits list, derived from the object (nothing stored separately).
#[test]
fn ac4_credit_of_an_imported_object_is_listed() {
    let target = Document::new(2);
    let roots = import_one(&target, Some("Name plate by Max Mustermann, CC BY 4.0"));
    assert_ne!(roots.len(), 0);
    let credits = target.credits();
    assert_eq!(
        credits.len(),
        1,
        "one object with a credit, one line: {credits:?}"
    );
    assert_eq!(credits[0].text, "Name plate by Max Mustermann, CC BY 4.0");
    assert_eq!(credits[0].licence, "CC BY 4.0");
    assert_eq!(
        credits[0].licence_url,
        "https://creativecommons.org/licenses/by/4.0/"
    );
}

// Criterion 4: an object without a credit text adds no line.
#[test]
fn ac4_object_without_credit_text_lists_nothing() {
    let target = Document::new(2);
    let _ = import_one(&target, None);
    assert_eq!(target.credits().len(), 0);
}

// Criterion 4: "the app never removes a credit from an object" and "deleting the object removes its
// credit": undoing the import step removes the objects, so the derived list has no line left.
#[test]
fn ac4_undoing_the_import_removes_the_credit_with_its_object() {
    let target = Document::new(2);
    let _ = import_one(&target, Some("Name plate by Max Mustermann, CC BY 4.0"));
    assert_eq!(target.credits().len(), 1);
    let step = target.own_steps().last().unwrap().step;
    target.undo_step(step).unwrap();
    assert_eq!(
        target.credits().len(),
        0,
        "the credit leaves with its object: {:?}",
        target.credits()
    );
}

// Criterion 3: the Came from record carries the address and the licence of the online item.
#[test]
fn ac3_came_from_record_carries_address_licence_and_author() {
    let target = Document::new(2);
    let roots = import_one(&target, None);
    let record = target.came_from(roots[0]).expect("the copy has a record");
    let origin = record.online.expect("the online keys are present");
    assert!(
        origin
            .address
            .starts_with("https://raw.githubusercontent.com/")
    );
    assert_eq!(origin.spdx, "CC-BY-4.0");
    assert_eq!(origin.author, "Max Mustermann");
}

// Criterion 3: a template loads in one undo step, every page as a new page.
#[test]
fn ac3_load_of_a_template_is_one_undo_step() {
    let src = Document::new(1);
    let a = src.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
    ));
    let frag = src.extract_fragment(&[a], Some(FIRST_PAGE_ID)).unwrap();
    let target = Document::new(3);
    let before_pages = target.pages().len();
    let steps_before = target.own_steps().len();
    let ids_a: Vec<AnchorId> = (0..frag.anchor_count())
        .map(|i| AnchorId::new(9138, 2000 + i as u64))
        .collect();
    let ids_b: Vec<AnchorId> = (0..frag.anchor_count())
        .map(|i| AnchorId::new(9138, 3000 + i as u64))
        .collect();
    let reports = target
        .insert_fragment_pages(&[(&frag, &ids_a), (&frag, &ids_b)], &online(None))
        .expect("both pages load");
    assert_eq!(reports.len(), 2);
    assert_eq!(
        target.pages().len(),
        before_pages + 2,
        "every page becomes a new page"
    );
    assert_eq!(
        target.own_steps().len(),
        steps_before + 1,
        "one step for the whole template"
    );
    let last = target.own_steps().last().unwrap().label.clone();
    assert_eq!(last, "load_online_template");
}
