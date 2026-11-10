//! The operation rules over a real document (`specs/0101-laser-operations` criteria 5, 6, 12,
//! 26, 27): the chain read from the document, the Cuts last register and the summary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    AnchorId, Document, DocumentSize, NewAnchor, NewOperation, NewOutputPage, NodeId, Point,
};
use curvyo_laser_core::operations::{
    Hidden, KindRead, Leaf, LeafShape, OperationKind, OperationList, Summary, cuts_last,
    insert_index, run_sequence, summarize,
};

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn add(d: &Document, mfg: NodeId, kind: OperationKind, members: &[NodeId]) -> NodeId {
    let kinds: Vec<KindRead> = d
        .operations(mfg)
        .unwrap()
        .iter()
        .map(|o| KindRead::of(o.kind.as_deref()))
        .collect();
    let on = cuts_last(&d.manufacturing(mfg).unwrap());
    let at = insert_index(&kinds, kind.section(), on);
    let new = NewOperation {
        kind: kind.as_str(),
        name: kind.label(),
    };
    let paths: Vec<Vec<NodeId>> = members.iter().map(|&member| vec![member]).collect();
    d.add_operation(mfg, &new, at, &paths).unwrap()
}

#[test]
fn cuts_last_is_on_until_switched_off_and_on_again_moves_cutting_below() {
    let d = Document::new(1);
    let mfg = laser(&d);
    assert!(
        cuts_last(&d.manufacturing(mfg).unwrap()),
        "absent reads as on"
    );
    d.set_cuts_last(mfg, false, &[]).unwrap();
    assert!(!cuts_last(&d.manufacturing(mfg).unwrap()));
    let cut = add(&d, mfg, OperationKind::Cut, &[]);
    let engrave = add(&d, mfg, OperationKind::Engrave, &[]);
    let ops = d.operations(mfg).unwrap();
    let kinds: Vec<KindRead> = ops
        .iter()
        .map(|o| KindRead::of(o.kind.as_deref()))
        .collect();
    assert_eq!(ops.iter().map(|o| o.id).collect::<Vec<_>>(), [cut, engrave]);
    let order: Vec<NodeId> = run_sequence(&kinds, true)
        .into_iter()
        .map(|at| ops[at].id)
        .collect();
    d.set_cuts_last(mfg, true, &order).unwrap();
    assert!(cuts_last(&d.manufacturing(mfg).unwrap()));
    let stored: Vec<NodeId> = d.operations(mfg).unwrap().iter().map(|o| o.id).collect();
    assert_eq!(stored, [engrave, cut]);
}

#[test]
fn a_deleted_group_member_neither_runs_nor_stops_the_rule() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let c = |k: u64, x: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, 0.0));
    let a = d.create_path(&[c(1, 0.0), c(2, 5.0), c(3, 5.0)], true);
    let b = d.create_path(&[c(4, 0.0), c(5, 5.0)], false);
    let group = d.group_objects(&[a, b]).unwrap();
    add(&d, mfg, OperationKind::CutOut, &[group]);
    add(&d, mfg, OperationKind::Engrave, &[a]);
    d.delete_objects(&[a]).unwrap();
    let ops = d.operations(mfg).unwrap();
    let engrave = ops
        .iter()
        .find(|o| o.kind.as_deref() == Some("engrave"))
        .unwrap();
    assert!(
        engrave.members.is_empty(),
        "Frage 169 A: the key goes with the object"
    );
    let list = OperationList::new(&ops, true);
    let chain = d.key_chain(&[b]);
    let summary = summarize(
        &list,
        [Leaf {
            chain: &chain,
            hidden: Hidden::No,
            shape: LeafShape::Outlines { closed: 0, open: 1 },
        }],
    );
    assert_eq!(
        summary,
        Summary {
            operations: 1,
            running: 1,
            ..Summary::default()
        }
    );
    assert_eq!(
        list.nearest(&chain, curvyo_laser_core::operations::ChainScope::Whole)
            .unwrap()
            .target,
        vec![group]
    );
}
