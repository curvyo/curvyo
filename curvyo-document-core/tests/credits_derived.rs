//! The credits of a document are derived from its objects (`specs/0138-template-library`
//! criterion 4, ADR 0085 decision 7): an object that came with a credit lists it, deleting the
//! object takes it away, and nothing is stored.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    AnchorId, CameFrom, Document, FragmentTarget, NewAnchor, NodeId, OnlineOrigin, Point,
    UnixMillis,
};

fn square(d: &Document, x: f64) -> NodeId {
    let a = |n: u64| AnchorId::new(1, n + if x > 0.0 { 10 } else { 0 });
    d.create_path(
        &[
            NewAnchor::corner(a(1), Point::new(x, 0.0)),
            NewAnchor::corner(a(2), Point::new(x + 5.0, 0.0)),
            NewAnchor::corner(a(3), Point::new(x + 5.0, 5.0)),
        ],
        true,
    )
}

fn record(credit: Option<&str>) -> CameFrom {
    CameFrom {
        document: "Name plate".into(),
        page: "Page 1".into(),
        page_id: "first".into(),
        item: None,
        at: UnixMillis::new(1),
        online: Some(OnlineOrigin {
            address: "https://example.org/cat/signs/name-plate/template.curvyo".into(),
            spdx: "CC-BY-4.0".into(),
            licence: "CC BY 4.0".into(),
            licence_url: "https://creativecommons.org/licenses/by/4.0/".into(),
            author: "Max Mustermann".into(),
            credit: credit.map(str::to_owned),
        }),
    }
}

fn import(
    target: &Document,
    source: &Document,
    roots: &[NodeId],
    record: &CameFrom,
    peer: u64,
) -> Vec<NodeId> {
    let fragment = source.extract_fragment(roots, None).unwrap();
    let anchors: Vec<AnchorId> = (0..fragment.anchor_count())
        .map(|n| AnchorId::new(peer, 100 + n as u64))
        .collect();
    target
        .insert_fragment(
            &fragment,
            FragmentTarget::Elements {
                page: "first",
                offset: curvyo_document_core::Vec2::new(0.0, 0.0),
            },
            record,
            &anchors,
        )
        .unwrap()
        .roots
}

#[test]
fn a_credit_is_listed_while_its_object_is_in_the_document() {
    let source = Document::new(1);
    let a = square(&source, 0.0);
    let target = Document::new(2);
    assert_eq!(target.credits(), vec![]);

    let credited = import(
        &target,
        &source,
        &[a],
        &record(Some("Name plate by Max Mustermann, CC BY 4.0")),
        2,
    );
    let credits = target.credits();
    assert_eq!(credits.len(), 1);
    assert_eq!(credits[0].text, "Name plate by Max Mustermann, CC BY 4.0");
    assert_eq!(credits[0].licence, "CC BY 4.0");

    // The same credit twice is listed once.
    let again = import(
        &target,
        &source,
        &[a],
        &record(Some("Name plate by Max Mustermann, CC BY 4.0")),
        3,
    );
    assert_eq!(target.credits().len(), 1);

    // An item without a credit (CC0) lists nothing.
    import(&target, &source, &[a], &record(None), 4);
    assert_eq!(target.credits().len(), 1);

    // Deleting every object that carries it removes the credit; nothing is stored.
    target.delete_objects(&credited).unwrap();
    assert_eq!(
        target.credits().len(),
        1,
        "the second copy still carries it"
    );
    target.delete_objects(&again).unwrap();
    assert_eq!(target.credits(), vec![]);
}
