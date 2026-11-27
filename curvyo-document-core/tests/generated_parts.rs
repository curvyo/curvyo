//! Detach part and Attach part at the document model (`specs/0147-rounded-box` criterion 15,
//! ADR 0082 §10): part keys in the record, `member.<node>#<part>` memberships, one step each, and
//! the lifecycle of a key whose part is gone.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::assert_is_empty
)]

use curvyo_document_core::{
    ATTACH_PART_COMMIT_LABEL, Affine, AnchorId, DETACH_PART_COMMIT_LABEL, Document, DocumentSize,
    GeneratedRecord, NewAnchor, NewOperation, NewOutputPage, NodeId, OperationSnapshot, PartError,
    Point, pack, unpack,
};

fn square(at: f64) -> Vec<f64> {
    let (a, b) = (at, at + 4.0);
    vec![
        a, a, a, a, b, a, b, a, b, a, b, b, b, b, a, b, a, b, a, b, a, b, a, a, a, a,
    ]
}

fn record(keys: &[&str]) -> GeneratedRecord {
    GeneratedRecord {
        kind: "curvyo.generators/rounded-box".into(),
        seed: 1,
        params: serde_json::Map::new(),
        parts: keys
            .iter()
            .enumerate()
            .map(|(i, _)| vec![square(i as f64 * 10.0)])
            .collect(),
        part_keys: keys.iter().map(|k| (*k).to_owned()).collect(),
        part_paints: std::collections::BTreeMap::new(),
    }
}

struct World {
    d: Document,
    m: NodeId,
    boxed: NodeId,
    cut: NodeId,
}

fn world(keys: &[&str]) -> World {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    let boxed = d.create_generated(&record(keys), Affine::IDENTITY).unwrap();
    let cut = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 3 mm",
                settings: None,
            },
            0,
            &[vec![boxed]],
        )
        .unwrap();
    World { d, m, boxed, cut }
}

fn ops(w: &World) -> Vec<OperationSnapshot> {
    w.d.operations(w.m).unwrap()
}

#[test]
fn detach_writes_a_copy_after_the_original_with_the_part_key_in_one_step() {
    let w = world(&["strip", "base", "lid"]);
    let before = w.d.own_step_count();
    let copies =
        w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[])
            .unwrap();
    assert_eq!(w.d.own_step_count(), before + 1, "one step");
    assert_eq!(
        w.d.own_steps().last().unwrap().label,
        DETACH_PART_COMMIT_LABEL
    );
    let list = ops(&w);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, w.cut, "the original stays first");
    assert_eq!(list[1].id, copies[0], "the copy sits right after it");
    assert_eq!(list[1].name, "Cut 3 mm · Lid");
    assert_eq!(list[1].kind.as_deref(), Some("cut"));
    assert_eq!(
        list[0].members,
        vec![vec![w.boxed]],
        "the box keeps its operation"
    );
    let lid = w.boxed.with_part("lid").unwrap();
    assert_eq!(list[1].members, vec![vec![lid]]);
    assert_eq!(w.d.detached_parts(w.boxed), ["lid"]);
}

#[test]
fn detach_is_refused_for_an_unknown_part_a_detached_part_and_no_operation() {
    let w = world(&["strip", "lid"]);
    assert_eq!(
        w.d.detach_part(w.boxed, "ring", "Ring", &[w.cut], &[]),
        Err(PartError::NoSuchPart)
    );
    assert_eq!(
        w.d.detach_part(w.boxed, "lid", "Lid", &[], &[]),
        Err(PartError::NoOperation)
    );
    w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[])
        .unwrap();
    assert_eq!(
        w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[]),
        Err(PartError::AlreadyDetached)
    );
    assert_eq!(ops(&w).len(), 2, "the refusals wrote nothing");
}

#[test]
fn a_record_without_keys_gets_them_in_the_detach_step() {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    let mut old = record(&["strip", "lid"]);
    old.part_keys.clear();
    let boxed = d.create_generated(&old, Affine::IDENTITY).unwrap();
    let new = NewOperation {
        kind: "cut",
        name: "Cut 1",
        settings: None,
    };
    let cut = d.add_operation(m, &new, 0, &[vec![boxed]]).unwrap();
    assert_eq!(
        d.detach_part(boxed, "lid", "Lid", &[cut], &[]),
        Err(PartError::NoSuchPart),
        "no keys and none given"
    );
    let keys = ["strip".to_owned(), "lid".to_owned()];
    d.detach_part(boxed, "lid", "Lid", &[cut], &keys).unwrap();
    assert_eq!(d.generated(boxed).unwrap().record.part_keys, keys);
    assert_eq!(d.detached_parts(boxed), ["lid"]);
}

#[test]
fn attach_removes_every_key_of_the_part_in_one_step_and_undo_brings_it_back() {
    let w = world(&["strip", "lid"]);
    w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[])
        .unwrap();
    assert_eq!(w.d.attach_part(w.boxed, "strip"), Ok(false), "not detached");
    let before = w.d.own_step_count();
    assert_eq!(w.d.attach_part(w.boxed, "lid"), Ok(true));
    assert_eq!(w.d.own_step_count(), before + 1);
    assert_eq!(
        w.d.own_steps().last().unwrap().label,
        ATTACH_PART_COMMIT_LABEL
    );
    assert!(w.d.detached_parts(w.boxed).is_empty());
    assert!(ops(&w)[1].members.is_empty());
    let step = w.d.own_steps().last().unwrap().step;
    w.d.undo_step(step).unwrap();
    assert_eq!(w.d.detached_parts(w.boxed), ["lid"]);
}

#[test]
fn the_part_keys_survive_save_and_reopen() {
    let w = world(&["strip", "lid"]);
    w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[])
        .unwrap();
    let back = unpack(2, &pack(&w.d, "0.1.0").unwrap()).unwrap();
    assert_eq!(back.detached_parts(w.boxed), ["lid"]);
    assert_eq!(
        back.generated(w.boxed).unwrap().record.part_keys,
        ["strip", "lid"]
    );
    let list = back.operations(w.m).unwrap();
    assert_eq!(
        list[1].members,
        vec![vec![w.boxed.with_part("lid").unwrap()]]
    );
}

#[test]
fn deleting_the_object_sweeps_its_part_keys() {
    let w = world(&["strip", "lid"]);
    w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[])
        .unwrap();
    w.d.delete_objects(&[w.boxed]).unwrap();
    assert!(ops(&w).iter().all(|op| op.members.is_empty()));
}

#[test]
fn a_part_key_whose_part_is_gone_is_kept_and_reads_again_when_it_returns() {
    let w = world(&["strip", "lid"]);
    w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[])
        .unwrap();
    // Lid switched off: the record has no `lid` part. The key stays on the operation.
    let without = record(&["strip"]);
    w.d.change_generated(w.boxed, &without, Affine::IDENTITY, "change_parameter", &[])
        .unwrap();
    assert_eq!(
        w.d.detached_parts(w.boxed),
        ["lid"],
        "kept, ignored by the record"
    );
    w.d.change_generated(
        w.boxed,
        &record(&["strip", "lid"]),
        Affine::IDENTITY,
        "change_parameter",
        &[],
    )
    .unwrap();
    assert_eq!(w.d.detached_parts(w.boxed), ["lid"], "applies again");
}

#[test]
fn a_record_with_a_duplicate_or_misplaced_key_is_refused() {
    let d = Document::new(1);
    let mut bad = record(&["strip", "lid"]);
    bad.part_keys = vec!["lid".into(), "lid".into()];
    assert!(d.create_generated(&bad, Affine::IDENTITY).is_err());
    bad.part_keys = vec!["lid".into()];
    assert!(
        d.create_generated(&bad, Affine::IDENTITY).is_err(),
        "not as long as the parts"
    );
    bad.part_keys = vec!["Lid".into(), "x".into()];
    assert!(
        d.create_generated(&bad, Affine::IDENTITY).is_err(),
        "not a valid key"
    );
}

#[test]
fn convert_to_paths_moves_a_part_key_to_the_part_s_new_path_in_the_same_step() {
    let w = world(&["strip", "lid"]);
    w.d.detach_part(w.boxed, "lid", "Lid", &[w.cut], &[])
        .unwrap();
    let corners = |base: u64, side: f64| {
        [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)]
            .iter()
            .zip(0_u64..)
            .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(9, base + k), Point::new(*x, *y)))
            .collect::<Vec<_>>()
    };
    let before = w.d.own_step_count();
    w.d.convert_generated(
        w.boxed,
        &[
            vec![(corners(10, 4.0), true)],
            vec![(corners(20, 4.0), true)],
        ],
    )
    .unwrap();
    assert_eq!(w.d.own_step_count(), before + 1);
    let list = ops(&w);
    let [lid_path] = list[1].members.as_slice() else {
        panic!("one member expected: {:?}", list[1].members);
    };
    assert_eq!(lid_path.len(), 1);
    assert_ne!(
        lid_path[0].node(),
        w.boxed,
        "the key names the lid's own path"
    );
    assert!(lid_path[0].part().is_none());
    assert_eq!(
        list[0].members,
        vec![vec![w.boxed]],
        "the box keeps its key"
    );
}

#[test]
fn the_order_of_part_memberships_follows_the_key_text_and_not_what_was_decoded_before() {
    // Two documents write the same keys in opposite orders; a third reads other keys first. The
    // order the operation reports is the text order every time.
    let order = |keys: &[&str], other_first: bool| {
        let w = world(&["alpha", "mid", "zeta", "wall-9", "wall-10"]);
        if other_first {
            let other = world(&["zz", "aa"]);
            other
                .d
                .detach_part(other.boxed, "zz", "Z", &[other.cut], &[])
                .unwrap();
        }
        for key in keys {
            w.d.set_operation_members(w.cut, &[vec![w.boxed.with_part(key).unwrap()]], true)
                .unwrap();
        }
        let back = unpack(2, &pack(&w.d, "0.1.0").unwrap()).unwrap();
        back.operations(w.m).unwrap()[0]
            .members
            .iter()
            .filter_map(|m| m[0].part().map(|p| p.to_string()))
            .collect::<Vec<_>>()
    };
    let expected = ["alpha", "mid", "wall-10", "wall-9", "zeta"];
    assert_eq!(
        order(&["zeta", "alpha", "wall-9", "mid", "wall-10"], false),
        expected
    );
    assert_eq!(
        order(&["wall-10", "mid", "wall-9", "alpha", "zeta"], true),
        expected
    );
}
